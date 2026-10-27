# M7 Slice 4 addendum: review a generated source package, record findings, revise from findings

Baseline: the owner's local integration
`codex/m7-preview-discovery` @
`e6d22f711c0c35312d1d3c43d53f2195409d48ff`
(ZIP SHA-256
`1c404ce4cc5ef1011bc615f24b2f6e70cf3b7f45983e5c181e4448ffe914c20e`,
bundle SHA-256
`d6b6fa90cf1da8f62d5153a254536679bdfc6bfdf4e8702761376450a9be0a85`,
verified in a scratch repo before import:
bundle verifies, scratch fetch lands exactly on
`e6d22f71`, `cc25155f` confirmed ancestor).
Branch `muse/m7-source-review` from that exact
tip; the older `bed0dc2` integration was not
substituted. Preview materials and discovery
changes are preserved untouched; no preview
branch was merged, rewritten or promoted.

## What was built

A complete, model-free source-review workflow
(design: `docs/handoffs/M7-source-review-design.md`,
committed before coding):

- New artifact type `source-review`
  (ARTIFACT_TYPES 18→19), format
  `anansi-source-review` v1. A review pins the
  exact package version and its specification
  version with recomputed content hashes, the
  concept name, one finding per acceptance
  criterion of the pinned specification (in spec
  order, criterion text verbatim), and the fixed
  backend limitations. Finding statuses are
  `not-assessed` (the default for every
  criterion), `needs-changes` (requires a
  non-empty note) and `satisfied`; notes are
  bounded to 2000 characters; file references
  are 0–12 exact paths in the pinned package's
  file list, unique.
- Direct op `forge-review-save` (route
  `/v1/ai/forge-review-save`, ai_command family,
  not a run op): resolves the package and its
  pinned specification through a new
  `load_review_package` helper that reuses the
  existing package/specification readers but
  deliberately does NOT apply the approval gate
  — review is inspection, and inspecting an
  unapproved package must be possible. The op
  touches no provider machinery: saving a review
  starts no inference, requires no provider
  configuration, and neither requires nor creates
  specification approval. Approval gates stay
  exactly where they were: at revision
  submission (and build/deliverable paths).
  Revising a review uses
  `artifact_revise_with_generator` with
  optimistic concurrency; a revision may not
  switch packages, package versions or
  specifications, and a stale expected version
  conflicts.
- `validate_review_record` (forge.rs, beside
  `validate_deliverable_record`): format/version,
  spec identity equals the pinned package's own
  spec identity, concept equals the package
  concept, pinned hashes equal the stored rows'
  hashes, findings validate exactly as at save,
  limitations equal the fixed set. Invalid
  records are refused, never repaired or
  relabelled. There is no review export op: a
  review is read in the app, and package export
  already exists.
- Restore (restore.rs, mirroring the Slice 3
  repair): `validate_original_review_bindings`
  runs over the source catalog before any id/hash
  change and before any destination write (row
  hash, pinned rows exist with correct types in
  the review's own workspace/project, original
  pinned hashes, full record validation);
  `remap_review_references` rewrites both ids
  fail-closed; `fix_review_hashes` rewrites
  pinned hashes to the restored rows' recomputed
  hashes. A review whose pins are absent,
  wrong-typed, cross-workspace/project,
  hash-mismatched or record-invalid refuses the
  whole restore with nothing published.
- UI (Forge.tsx, Projects.tsx):
  `SourceReviewPanel` under the source-package
  view, keyed
  `review:{workspace}:{project}:{artifact}:{version}`
  (the reviewed Slice 2 keying lesson, applied
  pre-emptively): it fetches the pinned
  specification at its exact version, shows its
  criteria verbatim beside the inert package
  view, and carries the inspection-only banner.
  `SourceReviewView` for saved reviews shows
  pins, findings, limitations, "Revise this
  review" (edit buffer reset on identity change
  and on Cancel, as in the Slice 3 repair) and
  findings selection. Selection composes
  editable revision instructions
  (`composeRevisionInstructions`); "Use in
  revision panel" prefills the existing
  `ForgeRevisePanel` instructions through a new
  optional `prefill` prop, applied only when it
  targets the currently viewed package version —
  switching packages never inherits stale
  instructions. Submission then uses the
  unchanged revision flow: approval gate before
  provider traffic, reject saves nothing, accept
  saves a new package version, stale parents
  conflict.
- Contracts: `ForgeReviewSaveRequest`,
  `SourceReviewDoc`, `ReviewFinding` added to
  the contract generator; client method
  `saveForgeReview` added beside
  `saveForgeDeliverable`. Generated contracts
  regenerated (only the benign pre-existing
  "unknown format uint ignored" warnings).
- Deliberately NOT built: a model reviewer
  (no draft/start/poll ops), any execution of
  generated code (no iframe preview, shell,
  builds/tests of generated products,
  deployment), any change to privacy or
  authorization boundaries, and any reuse of the
  concept-evaluation engine (a different domain:
  rubric scoring of ideas, not criterion
  assessment of source).

## Changed files

- `crates/core/src/forge.rs` — review structs,
  `load_review_package`, finding/record
  validation, `save_review`, remapping,
  `forge-review-save` dispatch.
- `crates/core/src/artifacts.rs` —
  `ARTIFACT_TYPES` 18→19, `source-review`.
- `crates/core/src/restore.rs` — original
  bindings validation, remapping branch,
  hash fix-up.
- `crates/core/src/ai_discovery.rs`,
  `crates/api/src/lib.rs` — op and route
  registration.
- `crates/contracts/src/main.rs`,
  `packages/contracts/schemas/readiness.json`,
  `packages/contracts/src/generated.ts`,
  `validators.mjs`, `validators.d.mts`,
  `client.ts` — new schemas and client method.
- `packages/ui/src/Forge.tsx`,
  `packages/ui/src/Projects.tsx` — review
  panel/view, findings selection, revision
  prefill.
- `crates/core/tests/forge_review.rs` (new,
  10 tests), `packages/ui/tests/components.test.tsx`
  (+4 tests), `tests/e2e/forge-review.spec.ts`
  (new).
- `docs/handoffs/M7-source-review-design.md`
  (new), this addendum, `docs/BUILD_STATUS.md`,
  `plans/05-product-forge.md`,
  `docs/REQUIREMENTS_MATRIX.md` (AN-012 row).

## Verification (Linux only; Windows pending)

All runs sequential on an idle machine, 2026-10-27.
No timeout or assertion was increased or
weakened; no failures occurred in this slice's
own suites (the baseline's one-off deliverable
approval-screen delay did not recur in the
canonical run and remains recorded as
intermittent, not root-caused).

- Backend review suite: forge_review 10/10.
- Full core: 254/258; the only 4 failures are
  the known environmental live-Ollama
  `semantic_workflow` tests (this VM has no
  live Ollama model), identical to prior slices.
- Providers+API: 144/0.
- Node UI/contracts: 64/64 (60 baseline + 4
  new review component/composition/parser
  tests).
- TypeScript, contracts drift, strict Clippy
  (`--all-targets -D warnings`), formatting:
  clean. API+web builds passed.
- Browser review spec: 1/1 (spec → approve →
  build → save package → review → save →
  reopen → review revision + Cancel regression
  → findings → prefill → revision reject →
  revision accept → historical v1 check →
  restart → backup/fresh restore → reopen
  restored review with remapped pins).
- Browser canonical battery: 13/13 (the 12
  prior specs plus forge-review), including
  the revision reject/accept and deliverable
  version-switch/Cancel regressions.

## Limitations and still open

- Linux verification only; Windows
  verification of this slice is pending and
  not claimed.
- A review assesses inert source against
  written criteria only: it establishes
  nothing about runtime behavior, passing
  tests, security certification or production
  readiness, and the UI and record say so.
- Review revision history is linear through
  the registry's optimistic concurrency, as in
  Slice 2: a review revision saves only while
  its parent version is still latest.
- The §32 execution sandbox, native desktop
  acceptance, preview promotion and M5
  production gates remain explicitly open.
  M7 as a whole is NOT complete.
