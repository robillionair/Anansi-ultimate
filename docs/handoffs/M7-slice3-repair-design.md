# M7 Slice 3 acceptance repair — design

Baseline: `codex/review-m7-slice3` at
`bd72a4ffe89b3a73275200e499716077d0f45051`, one commit over the delivered
Slice 3 tip `cbb95a6960f0bb1379741bdc6a6a34e900611e18`. The review adds
only tests and evidence; production code is unchanged. This repair changes
no feature, no scope and no preview promotion. Linux reproduction on this
machine matches the Windows review exactly:

- `forge_deliverable review_`: 0 passed / 2 failed — export accepted
  `["status", "specification", "duplicate-document", "unknown-format"]`;
  restore returned success for an all-zeroes pinned package hash.
- Browser `forge-deliverable-review.spec.ts`: 1 failed, only the soft
  version-switch assertion (v1 editor showed the v2 architecture draft);
  restart and restore assertions completed.

## Root causes

1. **Export trusts stored fields.** `export_deliverable` re-reads the pinned
   package and specification and checks the two pinned hashes and the
   verified-facts text, but copies the rest of the saved body — status,
   specification identity, document list — into the manifest unchecked.
   A registry hash only proves the body bytes are unchanged since they
   were written; it says nothing about whether the body is a valid
   domain record. The generic artifact revise path can therefore mint
   an exportable record with a false production-ready claim, a
   specification id that is not the package's specification, a
   duplicated document, or an unsupported format version.
2. **Restore repairs before it validates.** `fix_deliverable_hashes`
   overwrites a deliverable's pinned hashes with the imported rows'
   hashes unconditionally, after the generic id remap. An invalid
   original binding (all-zeroes package hash) therefore becomes valid
   in the destination. Nothing validates the original package/spec
   relationships, workspace/project ownership, versions or original
   pinned hashes before ids and hashes are changed.
3. **Saved-document editor is not identity-keyed.** `DeliverableView`
   initialises its edit buffer from props once (`useState` initializer)
   and is rendered in `Projects.tsx` without a key, so switching the
   viewed deliverable version (or record) re-renders the same component
   instance with the previous version's buffer. The prepare panel was
   already keyed this way after the Slice 2 review; the saved view
   was missed. Cancel editing also leaves the edited buffer in place,
   so a cancelled draft can resurface.

## Repair

### 1. Full stored-record validation at export (`crates/core/src/forge.rs`)

New shared validator `validate_deliverable_record(doc, package_doc,
package_body_hash, spec_content_hash)`, called by `export_deliverable`
after `load_pinned_package` and before any manifest or tar bytes are
built. It refuses, without overriding or relabelling anything, when:

- format is not `anansi-deliverable` or `format_version` is not 1;
- `package_version` or `spec_version` is below 1;
- `spec_artifact_id` / `spec_version` differ from the pinned package's
  own specification identity (the package, not the deliverable, is the
  authority for which specification it was built from);
- `concept_name` differs from the pinned package's concept name;
- `package_content_hash` differs from the hash of the stored package
  body, or `spec_content_hash` differs from the stored specification
  row's content hash;
- any of the five status flags (`executed`, `built`, `tested`,
  `published`, `production_ready`) is true;
- the document set is not exactly the five fixed names plus
  `verified-facts.md`, each appearing once (no missing, extra or
  duplicate entries);
- a fixed document has an origin other than `user` or `model`, or
  content that is empty/whitespace or longer than 12000 characters
  (the same bounds as save validation);
- the verified-facts document has an origin other than `verified`
  or is empty. Its exact text is additionally checked against the
  backend recomputation, as before (verified facts carry no ids or
  hashes, so that check survives restore remapping).

The existing verified-facts equality check, package file validation,
collision check and approval gate stay in place; the validator adds
the missing domain checks around them. Manifest metadata is then
derived only from a validated record, so manifest and archive entries
agree: the archive is built from the same fixed document order the
validator enforced.

The validator is deliberately not a save-time gate on the generic
editor: the generic editor is a supported path, and the fix is to
refuse invalid records at the consumption boundary (export), not to
hide the editor or silently repair the record.

### 2. Original-binding validation before restore remapping (`crates/core/src/restore.rs`)

New pre-validation pass `validate_original_deliverable_bindings`,
run over the source catalog's original `artifact_versions` rows
before the prepared-row loop changes any id or hash, and therefore
before any destination state is written. For every row of type
`deliverable` with format `anansi-deliverable` it refuses when:

- the row's stored hash does not equal the hash of its own body;
- the pinned package row is absent, is not a `source-package`, or
  lives in a different workspace or project than the deliverable;
- the pinned specification row is absent, is not a `specification`,
  or lives in a different workspace or project than the deliverable;
- either pinned row's stored hash does not equal the hash of its body;
- the shared deliverable-record validator (above) fails against the
  original package body hash and specification row hash — this covers
  format/version, specification identity against the package, concept
  provenance, status, document set, origins and bounds on the
  original record.

Only a record passing all of these is then remapped and gets its
pinned hashes recomputed to the restored rows' hashes, exactly as
before. `fix_deliverable_hashes` and `fix_approval_hashes` are
unchanged in behaviour for valid records; approval transfer is
untouched. Because the pass runs before the transaction and before
any body files are written, a refusal publishes nothing: source and
existing destination workspaces are intact.

### 3. Saved-document editor identity discipline (UI)

- Key `DeliverableView` in `Projects.tsx` by
  `deliverable:{workspaceId}:{projectId}:{artifactId}:{version}`,
  the same full-identity keying the Slice 2 review required for the
  revision panel. Switching versions or records remounts the editor,
  discarding the previous buffer and editing state.
- Defence in depth inside `DeliverableView` itself, so the component
  is safe wherever it is rendered: an effect resets the edit buffer
  (and leaves editing mode) whenever the viewed artifact id or
  version changes, and Cancel editing resets the buffer to the
  viewed document before leaving editing mode. A cancelled or
  superseded draft therefore never carries into another version or
  record.
- The prepare panel (`DeliverablePanel`) is already keyed by package
  identity and is unchanged.

### 4. Browser consolidation

Delete `tests/e2e/forge-deliverable-review.spec.ts` and move its
version-switch probe into the canonical `forge-deliverable.spec.ts`
as a hard assertion (the review's soft assertion was diagnostic, to
let restart/restore continue past the failure; the canonical spec
asserts the fixed behaviour directly). Restart and fresh-restore
coverage in the canonical spec is unchanged.

## Tests

- Keep both review tests unchanged as the acceptance core, except
  the restore test gains no-mutation controls: after the failed
  restore, the destination must contain no workspace named
  `Invalid pin review`, a second restore attempt of the same backup
  must also fail with no such workspace, and the source deliverable
  must still have exactly its two versions with export still
  refusing it.
- New integration test `export_rejects_other_inconsistent_records`:
  each defect isolated from the original saved body (not chained):
  `executed`, `built`, `tested`, `published` true; specification
  version mismatch against the package; a missing document; an
  extra document; an invalid origin (`system`, and `verified` on an
  editable document). All must refuse export.
- Existing valid round trips are the controls: manual save +
  deterministic export, model draft edit + save, approval gate,
  collision refusal, package-version pinning, restart + fresh
  restore with continued exportability and approval transfer, and
  the full canonical browser workflow including the consolidated
  version-switch assertion.

## Acceptance criteria

1. `forge_deliverable` suite passes in full, including both review
   tests and the new inconsistent-record test, with no assertion
   removed and no deadline changed.
2. The canonical browser deliverable workflow passes, including
   v2 → v1 editor loading v1's exact architecture text, cancel
   leaving no draft behind, restart/reopen and backup/fresh restore
   with a re-export naming the restored package.
3. Full core suite shows only the 4 known environmental live-Ollama
   failures; providers+API, node UI/contracts, TypeScript, contracts
   drift, strict Clippy, formatting and docs checks all pass.
4. No new features, no preview promotion, no generated-code
   execution, no pushes, master merges or remote CI. Production
   gates (native credential, live-provider, G1 audit, remote CI)
   remain open, and Windows verification of the repair remains
   pending independent confirmation.
