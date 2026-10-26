# M7 Slice 3 design: a complete, reviewable export deliverable

Baseline: the Windows-reviewed Slice 2 checkpoint
`0f586f865a8c4d227729c53449ce171b03b18fe7`
(branch `codex/review-m7-slice2`), whose parent is the
Slice 2 delivery `9928bf5c`. The reviewed branch's
keyed revision panel and its browser regression are
preserved untouched; this slice adds a new panel keyed
the same way. No merge or rewrite of the owner's preview
branch `codex/local-preview-m7` @ `3335248b`.

## Inventory: what already exists, and the actual gap

- `forge-export` already ships a deterministic USTAR of
  one saved source package plus an `ANANSI-PACKAGE.json`
  manifest (ids, versions, per-file hashes, limitations).
  It carries no documentation, no saved deliverable
  record, and no statement of review status beyond the
  package's own limitations.
- The artifact registry already gives immutable versions,
  exact-version reads, project identity, backup/restore
  with reference remapping, and the shared package path
  validator (`validate_package_files`).
- MASTER_SPEC §33 lists the deliverables of a built
  product (source, README, installation guide,
  architecture document, environment template, migrations,
  tests, build instructions, security notes, user guide,
  deployment guide, license). For an *unexecuted*
  prototype most of that list would be fabrication:
  there is no build, no test run, no deployment and no
  license to report. The remaining gap, and this slice,
  is the honest subset the owner assigned: architecture
  overview, usage instructions, known limitations,
  security notes and a change summary, saved as a
  version-pinned deliverable record and exported as one
  deterministic archive with the exact source bytes.

## Requirement mapping

| Owner requirement | Slice 3 answer |
|---|---|
| Open an exact source-package version | Deliverable pins `packageArtifactId` + `packageVersion` + recomputed `packageContentHash`; any read uses the exact version. |
| Prepare deliverable documentation | Five fixed documents: `architecture.md`, `usage.md`, `limitations.md`, `security.md`, `changes.md`. |
| Distinguish authorship | Every document carries `origin`: `user`, `model` or `verified`. Editing a model suggestion in the UI flips its origin to `user` (last author wins, honestly). |
| Never invent builds/tests/certification/deployment/license | No license, deployment or build document exists. The deliverable body, its view and the export manifest all carry `status: {executed:false, built:false, tested:false, published:false, productionReady:false}` and the banner "Unexecuted prototype — prepared for review." `usage.md` describes opening `index.html` from the filesystem for review only. |
| Inspect and edit drafts before saving | The panel shows every draft section as an editable textarea with its origin badge; nothing is saved until the user presses Save. |
| Save an immutable deliverable record | New artifact type `deliverable` (ARTIFACT_TYPES 17→18), format `anansi-deliverable` v1, saved through the existing registry. Title stays stable across versions. |
| Deterministic archive | `forge-deliverable-export` reuses `build_tar` (fixed mtime 0, no timestamps in the manifest): source files byte-identical at their exact paths, documents under `deliverable-docs/`, one `ANANSI-DELIVERABLE.json` manifest with ids, versions and per-file/per-document SHA-256. Exporting the same deliverable twice yields identical bytes. |
| Unexecuted status shown, export ≠ published/built/tested | Banner on the panel and the deliverable view; status block in the manifest; export filename is `<deliverableId>-v<N>.tar`, never "release" or "build". |
| Reopen after restart; backup/restore preserves relationships and exportability | Deliverable bodies remap `packageArtifactId`/`specArtifactId` fail-closed at restore; a fix-up pass (mirroring `fix_approval_hashes`) rewrites the pinned hashes to the restored rows' recomputed hashes, so the restored deliverable still exports. |
| Collision handling, decided before code | Documents live only under the `deliverable-docs/` namespace in the archive. If any source file path starts with `deliverable-docs/` (case-insensitive), deliverable save is refused and export re-checks and refuses; the package itself is never rewritten and its bytes are never moved. Document names are fixed, so documents cannot collide with each other. |
| Editing creates a new version; saved deliverables never follow latest source | Save accepts optional `deliverableArtifactId` + `expectedVersion` and revises via `artifact_revise_with_generator` (optimistic concurrency, as in Slice 2). A deliverable always names its pinned package version; a newer package version never changes it. |
| Missing/mismatched references fail closed | Save and export re-read the pinned package version and its pinned specification approval and recompute the package hash; any absence or mismatch is an error, never a silent fallback. |

## Model path and manual path

- Manual path (always available, no provider needed):
  the user writes all five sections in the panel and
  saves. Generator `{kind: "manual"}`.
- Model path (optional suggestion only): new run ops
  `forge-deliverable-draft-start` / `-poll` / `-cancel`
  on the existing M5 substrate, reusing the run registry
  (TTL, cap, eviction), Zero-Cost Lock, permits, budgets,
  cancellation and disclosure exactly as spec/build/
  revise. The draft run's segments are the specification
  body, a package inventory (plan, file list with sizes,
  limitations, parent diff when one exists — not the
  full file contents, to bound the prompt), and, last,
  the instruction segment: suggest the five documents as
  strict JSON, with the honesty rules stated (no build,
  test, security-certification, deployment or license
  claims; unknowns recorded as unknowns). The pinned
  specification version must hold backend approval
  *before any provider traffic*, as for build/revise;
  a manually registered package with an unapproved spec
  is refused. Draft output is parsed strictly (exactly
  the five names, each 1–12000 characters); anything
  else is status `invalid` and nothing is saved.
- Saving is deliberately *not* a run-save: a deliverable
  save must accept user edits to a draft, which the
  claim-and-wait run-save cannot express. Drafts only
  populate the panel; the single save op below persists
  whatever the panel holds. This is the one deliberate
  deviation from the spec/build/revise pattern, and it
  is what makes "inspect and edit before saving" real.

## Backend ops (all on existing routes families)

- `forge-deliverable-draft-start` (ai_command):
  `ForgeDeliverableDraftStartRequest` — workspace,
  project, `packageArtifactId`, `packageVersion`,
  model/provider/mode, `zeroCostLock`, `permitId`.
  Denies unknown fields.
- `forge-deliverable-draft-poll` / `-cancel`:
  `ForgeRunRequest`; poll gains optional
  `deliverableDraft` (documents with origin `model`).
  A new `Stage::Deliverable` in the run registry.
- `forge-deliverable-save` (ai_command, direct op, not
  a run op): `ForgeDeliverableSaveRequest` — workspace,
  project, `packageArtifactId`, `packageVersion`,
  `documents[]` (name + content; origin is recomputed
  by the backend: `verified` is never accepted from the
  client), optional `deliverableArtifactId` +
  `expectedVersion` for a documentation revision.
  Backend appends a sixth document,
  `verified-facts.md` (origin `verified`), computed
  from the stored package and specification: file
  inventory with hashes, versions, template, concept,
  parent diff when one exists. Client-supplied
  `verified-facts.md` is refused. Validation: fixed
  names only, unique, each 1–12000 characters, all
  five user/model sections present; package and spec
  checks as above; `deliverable-docs/` collision
  refusal; archived projects refuse.
- `forge-deliverable-export` (artifacts route family,
  beside `forge-export`): `ForgeExportRequest` shape
  (artifact id + exact version). Re-validates the
  deliverable, the pinned package version (hash
  recomputed and compared), the pinned specification
  approval, the package files and the collision rule,
  then builds the deterministic archive. Recomputed
  verified facts must match the saved ones, else fail
  closed (a stored deliverable whose facts no longer
  match its package is corrupt, not exportable).

## Restore

- `remap_deliverable_references` (forge.rs) rewrites
  `packageArtifactId` and `specArtifactId` through the
  restore identity map, fail-closed when either is not
  in the package. Called from `restore.rs` for type
  `deliverable`, like the package/approval branches.
- Fix-up pass `fix_deliverable_hashes` (restore.rs,
  beside `fix_approval_hashes`): after remapping, each
  deliverable's `packageContentHash` is rewritten to
  the restored package version's recomputed body hash
  and its `specContentHash` to the restored spec
  version's hash (approval rows already carry the
  restored spec hash); the deliverable row's own hash
  is recomputed. A deliverable whose pinned package or
  spec version is absent from the package fails the
  restore closed.

## UI

- `DeliverablePanel` (Forge.tsx), keyed in Projects.tsx
  by workspace + project + package id + package version
  (the reviewed Slice 2 keying lesson, applied
  pre-emptively): banner stating the unexecuted status;
  per-section textarea + origin badge; "Suggest
  documentation with model" (Zero-Cost Lock checkbox,
  cancel while running); Save deliverable; after save,
  Export deliverable (downloads the tar via the client,
  as the package export does) and the saved version.
- `DeliverableView` for a saved deliverable artifact in
  the Artifacts view: documents rendered as inert text
  with origins, the verified-facts document, the pinned
  package/spec versions and hashes, the status banner,
  Export, and "Revise documentation" (edits load into
  the panel and save as a new version).
- The panel is offered only for saved source packages;
  it never appears for concepts, specifications or
  other artifact types.

## Acceptance criteria (testable)

1. Manual save: five user documents + backend verified
   facts → deliverable v1; pinned package hash equals
   the stored package body's hash; export bytes are
   deterministic (two exports identical); manifest
   hashes match the tar entries; source entries are
   byte-identical to the package files.
2. Model draft: fake-provider draft fills all five
   sections with origin `model`; editing one flips it
   to `user`; save persists the edited set.
3. Approval gate: a manually registered package whose
   pinned spec was never approved is refused at draft
   start and at save, with no provider traffic.
4. Collisions: a package containing
   `deliverable-docs/notes.md` refuses save and export;
   the package itself is unchanged.
5. Stale/missing: save against a nonexistent package
   version, export after the pinned spec approval is
   absent, and a documentation revision against a stale
   `expectedVersion` all fail closed; no new version
   is created by any of them.
6. Pinned, not following: after the package gains v2,
   the v1 deliverable still names v1 and still exports
   v1's bytes.
7. Restart + restore: reopen after restart; backup →
   fresh restore → restored deliverable shows restored
   package/spec ids, still exports, and its manifest
   names the restored ids.
8. Browser: full workflow in the app (spec → approve →
   build → save package → prepare deliverable →
   save → export via the API → restart → reopen →
   backup/fresh restore → reopen restored deliverable
   and export again) with a synthetic provider; no
   exported content is executed anywhere in tests.
9. No regressions: existing Forge/recovery suites,
   UI/contracts, TypeScript, contracts drift, strict
   Clippy, formatting, docs validation, and the
   canonical e2e subset including the reviewed
   version-switch regression.

## Boundaries (unchanged)

No generated-code execution, iframe preview, shell
commands, dependency installs, builds/tests of generated
products, deployment or external fetching. No real
credentials, paid/cloud inference, model downloads,
system/security changes, pushes, master merges or
remote CI. Antigravity review files are not edited.
The §32 sandbox, native desktop acceptance, preview
promotion and M5 production gates remain explicitly
open. This slice is Linux-verified only until Windows
verification says otherwise.
