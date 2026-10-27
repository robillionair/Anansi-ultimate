# M7 Slice 4 design: review a generated source package, record findings, revise from findings

Baseline: the owner's local integration
`codex/m7-preview-discovery` @
`e6d22f711c0c35312d1d3c43d53f2195409d48ff`
(verified baseline `cc25155f` ancestor; preview
materials and discovery changes preserved; this
branch is `muse/m7-source-review` from that exact
tip, not the older `bed0dc2` integration). No
merge or rewrite of any preview branch.

## Inventory: what already exists, and the actual gap

- The artifact registry already gives immutable
  versions, exact-version reads, project identity,
  backup/restore with reference remapping, and
  optimistic-concurrency revise
  (`artifact_revise_with_generator`).
- Forge already has: exact-version specification
  approval (before any provider traffic), the
  package validator, the Slice 2 revision engine
  (`forge-revise-start` / `-poll` / `-cancel` /
  `-save`, server-computed diff, reject-saves-
  nothing, stale-parent conflict), and the Slice 3
  deliverable pattern (a direct, model-free save op
  pinning exact package/specification versions and
  hashes, plus restore pre-validation and hash
  fix-up). The evaluation engine (Slice 3 of M6)
  is a different domain — rubric scoring of ideas
  — and is deliberately NOT reused: a source
  review assesses specification criteria against
  inert source, it does not score concepts.
- The specification body already carries
  `acceptanceCriteria`: 2–8 items, each at most
  300 characters (`parse_product_spec`).
- The gap: nothing records a structured review of
  a saved source package against those criteria.
  The welcome copy mentions "manual source
  review" but no review artifact, panel or
  findings-to-revision path exists. This slice
  adds exactly that, model-free: no model
  reviewer, no draft/start/poll ops, no provider
  traffic anywhere in the review path.

## Requirement mapping

| Owner requirement | Slice 4 answer |
|---|---|
| Open an exact saved package version + pinned specification | The review panel fetches the package's pinned specification at its exact version (`readProjectArtifact` with the spec id/version from the package doc) and shows its acceptance criteria beside the inert package view. |
| Present criteria alongside inert inspection | Criteria are listed verbatim from the pinned spec version; the package files are shown as inert text by the existing `SourcePackageView`. Nothing is executed, built, previewed or deployed. |
| Mark each criterion Not assessed / Needs changes / Satisfied by inspection, with bounded notes and optional file references | One finding per criterion, in spec order. Status enum `not-assessed` / `needs-changes` / `satisfied`. Note 0–2000 chars; a `needs-changes` finding must carry a non-empty note (a change request with nothing to change is not a finding). `fileRefs`: 0–12 paths, each an exact path in the pinned package's file list, unique. Default status is `not-assessed` for every criterion. |
| Limitations explicit | The panel, the saved view and the record itself carry the fixed backend limitations: inspection does not establish runtime behavior, passing tests, security certification or production readiness; the source was not executed, built, previewed or deployed. |
| Save an immutable, versioned review pinned to exact workspace/project/package/spec versions and hashes; backend resolves and validates references | New artifact type `source-review` (ARTIFACT_TYPES 18→19), format `anansi-source-review` v1. Body pins `packageArtifactId`, `packageVersion`, `packageContentHash`, `specArtifactId`, `specVersion`, `specContentHash`, `conceptName`, `findings[]`, `limitations[]`. Title `Source review: {concept}` (200 chars). Privacy `private`; refs are the package ref + spec ref. |
| Reopen after restart without following newer versions | A review always names its pinned package version; a newer package version never changes it. Reopening reads the exact review version and its exact pinned package/spec versions. |
| Select findings → inspect editable instructions → submit through existing revision workflow | The panel (and the saved review view) offers per-finding selection checkboxes; "Prepare revision instructions" composes editable text from the selected findings (criterion, note, file refs) into a textarea the user inspects and edits, then "Use in revision panel" prefills the existing `ForgeRevisePanel` instructions (new optional `prefill` prop, applied on key change). Submission then uses the unchanged revise flow: approval gate before provider traffic, reject saves nothing, accept saves a new package version, stale parents conflict. |
| Saving/selecting a review never initiates inference or approves a specification | The save op touches no provider machinery and no approval machinery; it requires no provider configuration. It also does NOT require specification approval — deliberately: review is inspection, and inspecting an unapproved package must be possible. Approval gates stay exactly where they are: at revision submission (and build/deliverable paths, unchanged). |
| Backup/fresh restore: validate originals before remapping, remap correctly, preserve history, refuse invalid refs before publication | Mirror the Slice 3 repair: `validate_original_review_bindings` runs over the source catalog before any id/hash change and before any destination write; `remap_review_references` rewrites both ids fail-closed; `fix_review_hashes` rewrites pinned hashes to the restored rows' recomputed hashes. A review whose pinned package/spec is absent, wrong-typed, cross-workspace/project, hash-mismatched or record-invalid refuses the whole restore with nothing published. |

## Backend ops (existing route families)

- `forge-review-save` (ai_command family, direct op
  like `forge-deliverable-save`, NOT a run op):
  `ForgeReviewSaveRequest` — workspace, project,
  `packageArtifactId`, `packageVersion`,
  `findings[]`, optional `reviewArtifactId` +
  `expectedVersion` for revising a saved review.
  Denies unknown fields; contract version 1;
  `packageVersion` ≥ 1; archived projects refuse
  via `require_project_writable`.
- Resolution (new `load_review_package`, reusing
  `read_deliverable_package` /
  `read_specification`, WITHOUT the approval gate
  in `load_pinned_package`): read the package at
  the exact version (workspace/project/type
  checked by the readers), parse
  `SourcePackageDoc`, read its pinned
  specification at its exact version, recompute
  the package body hash and take the spec row's
  content hash. Any absence or mismatch fails
  closed; there is no "latest" fallback.
- Finding validation (backend, strict): findings
  count equals the spec's acceptance-criteria
  count; each finding's `criterionIndex` equals
  its position and its `criterion` text equals
  the spec criterion verbatim (criterion
  substitution is refused); status is one of the
  three values; note ≤ 2000 chars and non-empty
  when status is `needs-changes`; fileRefs are
  unique, ≤ 12, each an exact package file path.
- Save: register via
  `artifact_register_generated` with generator
  `{kind: "manual"}`, or revise via
  `artifact_revise_with_generator` when
  `reviewArtifactId` + `expectedVersion` are
  given. A review revision may not switch
  packages, package versions, specifications or
  concept: the existing review's pins must equal
  the request's pins, and the existing title is
  kept. Stale `expectedVersion` conflicts, as in
  Slice 2.
- `validate_review_record` (forge.rs, beside
  `validate_deliverable_record`): format
  `anansi-source-review` v1; versions ≥ 1; spec
  identity equals the pinned package's own spec
  identity; concept equals the package concept;
  pinned hashes equal the stored package body
  hash / spec row hash; findings validate exactly
  as at save (count, order, verbatim criteria,
  statuses, note bounds, fileRef membership
  against the pinned package files); limitations
  equal the fixed backend set. Invalid records
  are refused, never repaired or relabelled.
  There is no review export op: a review is read
  in the app, and package export already exists.

## Restore

- `remap_review_references` (forge.rs): rewrites
  `packageArtifactId` and `specArtifactId`
  through the restore identity map, fail-closed
  when either is not in the package. Called from
  `restore.rs` for type `source-review`, beside
  the deliverable branch.
- `validate_original_review_bindings`
  (restore.rs, beside
  `validate_original_deliverable_bindings`):
  over the source catalog, before any id/hash
  change: the review row's stored hash equals
  the hash of its body; the pinned package/spec
  rows exist, have types `source-package` /
  `specification`, belong to the review's
  workspace/project, and their row hashes equal
  the hash of their bodies; then
  `validate_review_record` against the original
  hashes. Only valid originals are remapped.
- `fix_review_hashes` (restore.rs, beside
  `fix_deliverable_hashes`): after remapping,
  each review's `packageContentHash` /
  `specContentHash` are rewritten to the restored
  rows' recomputed hashes and the review row's
  own hash is recomputed.

## UI

- `SourceReviewPanel` (Forge.tsx), under the
  source-package view, keyed
  `review:{workspaceId}:{projectId}:{artifactId}:{version}`
  (the reviewed Slice 2 keying lesson, applied
  pre-emptively): limitations banner; the pinned
  spec's criteria with per-criterion status
  select (default Not assessed), note textarea
  and file-reference input (comma-separated;
  the package file list is shown for reference);
  Save review; after save, the saved version.
  Findings selection checkboxes feed "Prepare
  revision instructions" (editable textarea) and
  "Use in revision panel" (prefill, see below).
  The panel is offered only for saved source
  packages.
- `SourceReviewView` for a saved `source-review`
  artifact: pins, findings with statuses/notes/
  file refs, limitations banner, "Revise this
  review" (findings load into the panel and save
  as a new version of the same review), and
  "Prepare revision from findings" (same
  selection → instructions → prefill path, which
  opens the pinned package view with the
  revision panel prefilled).
- `ForgeRevisePanel` gains an optional
  `prefill?: {key: string, text: string}` prop:
  an effect sets the instructions textarea when
  the prefill key changes. No other revise
  behavior changes; the approval gate, reject/
  accept and stale-parent handling are
  untouched. Projects.tsx holds the prefill
  state and passes the handler to both the
  review panel and the review view.

## Acceptance criteria (testable)

1. Manual save: findings for every criterion of
   the pinned spec (default Not assessed, one
   Needs changes with note + file ref, one
   Satisfied) → review v1; pinned hashes equal
   the stored package body hash / spec row hash;
   reopening shows the same findings.
2. Strict findings: wrong criterion text,
   missing/extra criterion, unknown status, note
   over 2000 chars, Needs changes without a
   note, a file ref not in the package, and a
   duplicate file ref all fail closed; nothing
   is saved by any of them.
3. References: save against a nonexistent
   package version, a package from another
   project, and a package whose pinned spec
   belongs to another project all fail closed.
4. No approval gate on review, no inference:
   saving a review for a package whose spec was
   never approved succeeds with no provider
   configured; saving or selecting a review
   creates no approval artifact and starts no
   run.
5. Pinned, not following: after the package
   gains v2, the v1 review still names v1, still
   reopens, and its findings still match v1's
   spec criteria.
6. Review revision: editing findings saves a
   new version of the same review; switching
   package/spec in a revision and a stale
   `expectedVersion` both fail closed.
7. Findings → revision: selected findings
   compose editable instructions; submitting
   them through the existing revise flow
   respects the approval gate (refused before
   provider traffic when the pinned spec is
   unapproved), reject saves nothing, accept
   saves a new package version, and a stale
   parent conflicts.
8. Restart + restore: reopen after restart;
   backup → fresh restore → restored review
   shows restored package/spec ids, still
   reopens, and still pins the original package
   version. Invalid originals (tampered pinned
   hash, cross-workspace pinned spec, tampered
   row hash) refuse the restore with no
   destination workspace published, and a second
   attempt also refuses.
9. Browser: full workflow (spec → approve →
   build → save package → review → save →
   reopen → findings → prefill → revision
   reject → revision accept → restart →
   backup/fresh restore → reopen restored
   review) with a synthetic provider; no
   generated content is executed anywhere.
10. No regressions: existing Forge/recovery
    suites, UI/contracts, TypeScript, contracts
    drift, strict Clippy, formatting, docs
    validation, and the canonical e2e subset
    including the reviewed revision
    rejection/acceptance and deliverable
    version-switch/Cancel regressions.

## Boundaries (unchanged)

No generated-code execution, iframe preview,
shell commands, dependency installs, builds/
tests of generated products, deployment or
external fetching. No model reviewer in this
slice. No real credentials, paid/cloud
inference, model downloads, system/security
changes, pushes, master merges, remote CI or
preview promotion. Antigravity review files are
not edited. The §32 sandbox, native desktop
acceptance, preview promotion and M5 production
gates remain explicitly open. This slice is
Linux-verified only until Windows verification
says otherwise.
