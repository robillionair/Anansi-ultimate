# M8 Slice 3 — explicitly selected memory context for LEXI-COLLIDER — record

Baseline: Windows-reviewed branch `codex/review-m8-acceptance-repair` at
exactly `4d3f8bfdaf904f452fe65c404a80bc2ef9aa83e2`, including the local
receipt-integrity correction `5b3cf958f9c35e45c9226d3988ae82b414b72d13`
(never restarted from `1ccda62d` alone). Incoming review ZIP SHA-256
`9d29cd1ec149f5b843ec0d83943f9ebe4ea0df949a3e1ea263f3e0e6dbc29685`,
enclosed bundle SHA-256
`94fdbfd987a14e2d83366de6578bbf19adb754c38cc617aa7e7dc65bffc4bf44`,
`git bundle verify` passed, ancestry of `1ccda62d`, `c50de23b` and
`5b3cf958f` confirmed. Slice branch: `muse/m8-slice3-memory-context`.
Design: [M8 Slice 3 design](M8-slice3-memory-context-design.md), committed
before coding. Owner authorization: one complete vertical slice, 2026-10-28.

## What changed

- Client submits references only. `ColliderStartRequest` gained an
  optional `memoryContext: [{ memoryId, revisionId }]`; bodies, privacy
  labels, workspace ownership and revision provenance are never trusted
  from the client. Bounds: at most 8 memories, each resolved body at
  most 8000 chars, total context at most 32000 chars; duplicates
  refused; no truncation. Resolution happens in `collider.start`
  through the existing workspace-scoped, revision-pinned,
  ownership-checked, hash-verified `Storage::memory_read`, which
  refuses Sealed content via `allow_plaintext` — before any provider
  traffic. Failures name only the 1-based selection position.
- Resolved entries are sent as additional `private` provider segments
  in selection order, framed as `Memory context (data) — {title}
  (revision {version}):\n{content}`. The instruction segment now states
  memory-context segments are likewise data, not instructions. Memory
  content is treated as untrusted source data throughout, on the
  existing M5 dispatch, authorization, disclosure, cancellation and
  accounting machinery. No new provider client, no schema change.
- Saved experiments keep format `anansi-collider-experiment` v1.
  `memoryContext` is optional/defaulted, so older experiments without
  context remain readable. Each saved entry pins `memoryId`,
  `revisionId`, `version`, `title`, verbatim `content`, `contentHash`
  and resolved privacy (`private`/`standard`/`public`); later memory
  edits cannot change historical experiments. Validation now also
  checks memory context, including content-hash consistency.
- Restore pre-validation (`validate_original_experiment_bindings`)
  checks each context entry against source-package `memory_revisions`
  rows: the revision exists, its owner memory matches, the owner
  workspace matches the artifact workspace, the row `body_hash`
  matches the pinned hash, the snapshot content hashes to the pinned
  hash, and metadata privacy is not Sealed. Any failure refuses the
  whole restore before destination publication. Remap rewrites
  memory/revision ids through the existing identity map, validating
  revision ownership via `revision_owners`, fail-closed; snapshot
  content and pinned hashes stay verbatim; the artifact row hash is
  recomputed. A new `fix_transfer_experiment_hashes` fix-up rewrites
  concept and receipt `sourceExperimentContentHash` values to the
  restored experiment row's recomputed hash, following the existing
  approval/receipt fix-up pattern.
- Picker: `memory_list` now excludes Sealed rows in SQL. The previous
  per-row `allow_plaintext()?` made the entire workspace list fail if
  any Sealed row existed; the per-row check remains as a backstop.
  Nothing is selected automatically; selecting, reviewing or changing
  context starts no inference.
- UI: the collider panel (already keyed by workspace/project) gained
  an optional memory picker (checkboxes `{title} · Revision {version}
  · {privacy}`), a pre-send review section showing exactly which
  revisions will be sent with verbatim bodies, and a generation-time
  snapshot view on saved experiments (`Memory context (as sent at
  generation time)`, per-entry details with verbatim `<pre>` content).
  Legacy experiments render without the section. Review loads carry a
  selection epoch so late responses for superseded selections are
  dropped; submit re-reads the current selection so the request
  carries exactly the reviewed revisions. The panel help text no
  longer lists memory selection among deferred slices.

## A defect this slice's tests caught and fixed

The transfer-preservation test (below) exposed a restore fix-up
ordering bug that predates this slice but only bites when an
experiment's hash changes on restore. `fix_transfer_receipt_hashes`
ran before `fix_transfer_experiment_hashes`, so the receipt pinned
the concept row hash from before the concept body was rewritten with
its restored experiment hash. For context-free experiments the
experiment hash never changes on restore, so the rewrite was a no-op
and the stale receipt never surfaced — which is why earlier transfer
restore tests passed without attempting Forge admission on the
restored workspace. With Slice 3's remapped memory references the
experiment hash changes, the concept hash changes, and Forge
admission refused a valid transfer. The fix reorders the two fix-ups
(experiment first, receipt second) with the dependency documented in
code. No assertion was weakened to accommodate it.

## Verification (Linux only; sequential on an idle machine)

- Backend: collider suite 19/19 (13 baseline + 6 new: wire capture
  with selected content present and unselected/cross-workspace
  content absent; Sealed/inaccessible/duplicate/overlong refusals
  with zero fake-provider hits; revision-B pinning surviving edits
  and restore with remapped ids; tampered/misbound restore refusal
  with no destination workspace; cancellation + malformed-output
  saving nothing; `memory_list` skipping Sealed rows). Restore unit
  tests cover the validation function directly, including the Sealed
  original branch — the supported backup pipeline itself refuses
  Sealed packages, so a Sealed original cannot reach restore through
  normal backup creation; the restore check is defense-in-depth for
  foreign or hand-crafted packages. Transfer suite 18/18, including
  the new context-transfer test: a context-bearing experiment's
  concept and receipt follow the restored experiment row, and the
  restored concept is Forge-admissible (a spec run completes against
  the synthetic provider on the restored workspace).
- Full core: 294 passed / 4 failed of 298 total (289 at the repair
  tip + 1 Windows-review receipt probe + 6 collider + 1 restore unit
  + 1 transfer). The 4 failures are the known environmental
  live-Ollama `semantic_workflow` tests on this Linux VM, unchanged
  from prior rounds.
- Providers + API: 144/0. Node UI/contracts: 81/81 (78 + 3 new
  collider tests). TypeScript clean, contracts check clean, strict
  Clippy (`--all-targets -- -D warnings`) clean, formatting clean,
  API + web builds pass.
- Browser canonical battery: 17/17, including the new
  `collider-memory-context.spec.ts` (selection → pre-send review →
  collision → save → restart → fresh restore, with project switching
  discarding selection and a post-revision reopen still showing
  revision B). No timeout or assertion was increased or weakened.

## Failed attempts (preserved, not hidden)

- First new collider run 17/19: the pinning test read artifact field
  `artifactType` (the record field is `type`), and the Sealed-restore
  integration subcase could not create backup bytes because export
  refuses Sealed content. Fixed with `type`, a wrong-owner
  integration refusal, and the restore unit test above.
- Next run 18/19: the cancellation test asserted the immediate cancel
  response was `cancelled`; it is `cancelling` — fixed to poll to the
  terminal state, matching the existing cancellation test.
- Browser first run: the `Projects` nav click matched `Refresh
  projects` too (substring match) — fixed with exact role names.
  Second run: the restored workspace holds two projects, so taking
  list index 0 picked the wrong project — fixed by filtering on the
  project title.
- Transfer test first run: Forge admission on the restored workspace
  refused the valid transfer — the fix-up ordering bug above, fixed
  by reordering, not by weakening the admission check.
- Strict Clippy flagged an unused binding in the new transfer test;
  removed. Formatting drift in the new test modules was fixed with
  `cargo fmt` and the affected suites re-run (collider 19/19,
  transfer 18/18).

## Windows owner test guide (short)

From the slice branch: run the focused backend suites
(`collider`, `transfer`, `forge_restore`), node UI/contracts,
TypeScript, contracts check, strict Clippy and formatting, then the
browser spec `tests/e2e/collider-memory-context.spec.ts` under the
lease wrapper. Check: the picker starts with nothing selected and
no review section; checking a memory shows exactly its revision and
verbatim body with zero provider hits until Collide; Sealed memories
do not appear in the picker; after revising a memory, the saved
experiment still shows the generation-time revision; after fresh
restore, the experiment, its transferred concept and its receipt all
resolve to the restored equivalents and the restored concept admits
a Forge spec run.

## Deferred and still open

AN-013 stays `in progress`. URL fetching, document ingestion,
saved-project selection and 3D visualization remain deferred. M8 is
not complete. Windows verification of Slice 3 is pending; this
delivery is Linux evidence only. No preview promotion, no native
desktop acceptance, no §32 sandbox, no M5 production gates — all
remain open and unclaimed by this slice.
