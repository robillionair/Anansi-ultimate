# M7 Slice 1 repair addendum — specification approval + portable export paths

Date: 2026-10-26 · Branch `muse/m7-slice1-repair` over
M7 Slice 1 tip `a439bf05add27eba9619fd7218191528663caddf`.
Design + acceptance criteria:
`docs/handoffs/M7-slice1-repair-design.md`. This is a
bounded repair: no new features, no sandbox, no
generated-code execution, no iframe preview, no dependency
installation, no deployment, no pushes, no master merges,
no remote CI.

## Gap 1 — approval was a UI gesture, not backend state

Owner diagnostic: save specification v1, revise it to v2
through the ordinary artifact revise operation, call
`forge-build-start` with `specVersion: 2` — generation was
accepted with no new approval action.

### Fix

- New operations `forge-spec-approve` and
  `forge-spec-approval-status` (ai surface, API routes
  `/v1/ai/forge-spec-approve` and
  `/v1/ai/forge-spec-approval-status`, generated
  contracts, client methods).
- Approval is a `specification-approval` artifact created
  **only** by `forge-spec-approve`. The type is
  deliberately absent from `ARTIFACT_TYPES`, so generic
  manual registration refuses it; a generic save or
  revision can never mint one. Its body
  (`anansi-specification-approval` v1) records
  `specArtifactId`, `specVersion` and `specContentHash`,
  and the row carries the workspace, project and
  `artifact://<spec>@v<n>` reference — binding approval
  to the exact workspace, project, specification identity,
  version and content hash. The hash is the specification
  version's stored `contentHash`, which `artifact_read`
  re-verifies against the stored body, never a
  client-supplied hash.
- `forge-build-start` requires a matching approval record
  **before any provider traffic**: no match means
  `InvalidRequest`, no run, no stream. `ForgeBuildStart-
  Request` has no approval field and denies unknown
  fields, so an `approved: true` payload is refused at
  parse time.
- Only the version-1 record of an approval artifact
  confers approval. Revising an approval artifact is
  ignored by the gate: v1 evidence is preserved and
  editing an approval record cannot transfer approval to
  another version. Re-approving an already-approved
  version is idempotent (`alreadyApproved: true`, same
  approval artifact id). Archived projects refuse
  approval like any other artifact change.
- UI: the spec panel now says "Save specification draft"
  (saving no longer claims approval); the build panel,
  rendered for the viewed specification version, loads
  that version's approval status, shows it in words, offers
  "Approve this specification version", and disables
  Generate while the version is unapproved. The backend
  gate remains the authority if the UI state is stale.
- Restart: approvals live in `artifact_versions`, so they
  persist across a fresh `Core` over the same root.
- Backup/restore: approval rows travel with the backup.
  Restore remaps the approval's specification reference
  (fail closed when the specification is not in the
  package) and a fix-up pass over the prepared
  `artifact_versions` rows rewrites the approval's
  `specContentHash` to the restored specification
  version's recomputed hash. Approval therefore transfers
  only to the restored equivalent of the same version —
  never to a different version, and never outside the
  package. A restored version that was never approved
  stays unapproved.
- Approval is permission to **generate source** from that
  specification version. It is not permission to execute
  generated software: build/test/preview/deployment stay
  Unavailable under §32, and source is still only ever
  inspected as inert text and exported for review.

## Gap 2 — export paths were not fully portable

Owner diagnostics: `validate_package_path("CON.txt")`
succeeded, and `validate_package_files` accepted both
`assets.js` and `assets.js/app.js`.

### Fix (shared validator, both paths)

`parse_source_package` and `forge-export` both call
`validate_package_files`, and export validates before
any tar bytes are built — so hardening the validator
once hardens generated-output parsing and stored-package
export identically. Additive rules, all prior rules kept:

- Windows device/reserved stems, case-insensitively, in
  any segment, with or without an extension: `CON`, `PRN`,
  `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`.
- Segments must not start or end with a space or a dot
  (Windows normalization collisions).
- File-versus-directory prefix conflicts, case-
  insensitively and regardless of order (`assets.js` +
  `assets.js/app.js`), and directory prefixes differing
  only by case (`CSS/…` + `css/…`), are refused.
- Unchanged: traversal, absolute/drive/UNC paths,
  backslashes, ADS colons, empty/dot segments, hidden
  files, the segment alphabet, the extension allowlist,
  exact and case-folded duplicates, size caps, NUL bytes,
  root `index.html` + `README.md`.

## Verification (sequential, idle machine)

- Forge units 8/8 (was 6; + reserved/prefix unit tests
  and the approval-remap unit test).
- `forge_generation` 5/5 (was 3): full workflow updated
  to approve v1 and v2 explicitly; invalid-outputs test
  updated likewise and now also accounts for the approval
  artifact type; new approval lifecycle test (unapproved
  refusal with wire capture unchanged, boolean-field
  refusal, idempotent approval, approved v1 build, stale
  approval-artifact revision preserving v1 while not
  approving v2, generic revise to v2 refused, explicit v2
  approval then v2 build, restart persistence, exactly
  two approval records); new export-refusal test
  (reserved name, prefix conflict, trailing dot — all
  refused by `forge-export` with no tar returned).
- `forge_restore` 3/3 (was 2): chain and absent-spec
  tests unchanged; new approval-transfer test (restored
  approval points at the restored specification version
  with its recomputed hash; status approved in the
  restored workspace without re-approval).
- Full anansi-core: 226/230 — only the 4 known
  environmental live-Ollama `semantic_workflow` failures
  (total up 5 from 225, exactly the new tests above).
- Providers + API 144/0; node contracts/UI 55/55 (was
  54; + Forge build-panel approval-action test); `tsc`
  clean; contracts check PASS; docs PASS (106 files);
  Clippy `--all-targets -D warnings` + fmt clean.
- Browser e2e `forge.spec.ts` 1/1, updated to exercise
  the approval step (save draft → not-approved status →
  explicit approval → generate → package → export →
  restart → reopen).
- Supplied tests were updated only to drive the new
  approval operation (and to account for the new approval
  artifact type in one list assertion); none were
  weakened, and no warnings were suppressed.

## Reproduction

From the bundle, on branch `muse/m7-slice1-repair`:

- Approval gap (before): on `a439bf0`, save a spec v1,
  revise to v2, `forge-build-start` with `specVersion: 2`
  succeeds. (After): the same call is refused with
  `InvalidRequest` until `forge-spec-approve` for v2.
- Path gap (before): `validate_package_path("CON.txt")`
  and a file set containing `assets.js` +
  `assets.js/app.js` validate. (After): both are refused,
  and `forge-export` of a stored package containing them
  returns the refusal with no tar.
- Commands: `node scripts/cargo.mjs test -p anansi-core
  --locked --lib forge`, `--test forge_generation`,
  `--test forge_restore`; node UI suite; `./node_modules/
  .bin/playwright test --config playwright.local.config.ts
  tests/e2e/forge.spec.ts` after building the web bundle
  and `anansi-api`.

## Still open

Windows preview promotion of the repair (owner review
pending); Windows verification of M7 Slice 1; the §32
execution sandbox as a later separately approved slice;
M5 production gates (real CredUI, live-provider
verification, G1 audit, remote CI). None are waived by
this repair or by its Linux evidence.
