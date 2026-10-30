# M8 overnight hardening — WP4 integrated acceptance record

Branch `muse/m8-overnight-hardening`, from Slice 4 tip
`0b6ba0e8723edb9a223db63a00a69565d7d77b7c`. WP1
(criteria `da9a10ea`, reproductions `50fd2c2f`, core
repair `97898f22`, test/UI completions `ecc14b14`,
record `158cc241`), WP2 (criteria `f60480da`, UI
`ca932ae1`, tests `d334d9f2`, record `5ee51c84`),
WP3 (criteria `ee359c5c`, component `50fcf11a`,
tests `9eddbd4d`, record `3b91ff3c`). This file is
the WP4 record; the per-package criteria files carry
the before/after evidence for WP1–WP3.

## What the integrated browser spec proves

`tests/e2e/collider-integrated-acceptance.spec.ts`
(real API, synthetic provider, one test) chains the
whole workflow in the browser:

project selection (Source Harbor) -> exact item/version
review (Tide notes pinned to revision 1 with its v1
body and privacy label, Harbor summary artifact v1) ->
collision (one fake hit) -> save experiment ->
approved Forge transfer (user-edited brief) ->
restart/reopen (concept provenance intact) -> backup ->
fresh restore -> reopen (snapshot still v1, concept
provenance intact) -> Forge spec from the restored
concept (via idempotent re-approval) -> second
backup/fresh restore -> reopen -> Forge spec again.

Negatives inside the same test, not only the happy
path:

- A source edit after save (member revised to v3)
  never rewrites the saved snapshot: the reopened
  experiment still shows revision 1 with its v1 body,
  and v3 text appears nowhere.
- An edited brief after approval is not approval: the
  approve button returns and "Open in Product Forge"
  disappears until the approved brief is approved again
  (which is idempotent, same concept, no duplicate).

Two findings from building the spec, both test-side,
both recorded rather than worked around in production:

- Restore `name` must differ from the landing
  workspace name (`WORKSPACE_EXISTS` otherwise); the
  spec names the landing "<name> Landing" and the
  restore "<name>".
- The transfer panel is session-scoped: it shows the
  Forge button only after an approval in the current
  session, so a restored experiment reaches Forge by
  re-approving the same brief (alreadyTransferred).
  That is the product's idempotency design, not a gap.

## Verification order (WP4, sequential, Linux)

1. Focused unit/integration tests: boundary 7/7,
   double-restore 3/3, restore lib binding tests 3/3,
   picker helper/loader node tests, overview state
   tests — all pass (see WP1–WP3 records).
2. Regressions: collider 34/34, transfer 19/19,
   forge_restore 4/4, backup_restore 7/7, node suite
   pass; no assertion weakened anywhere in the round.
3. TypeScript, contract drift, strict Clippy
   (`--workspace --all-targets -D warnings`),
   formatting: all clean.
4. API build and web build pass; docs validation
   passes (170 Markdown files).
5. Canonical affected browser specs, run sequentially:
   collider (1), collider-memory-context (1),
   collider-memory-acceptance (2),
   collider-project-context (1),
   collider-picker-reliability (5),
   collider-pipeline-overview (1),
   collider-integrated-acceptance (1), transfer (1),
   transfer-identity (1), transfer-concurrency (1),
   forge (1), forge-review (1), forge-revision (1),
   forge-deliverable (1), backups (1), export (1) —
   21 tests, all passing.
6. Broader core run with `--no-fail-fast` (single run,
   `--test-threads=1`): 345 passed / 4 failed. The 4
   failures are exactly the known live-Ollama
   `semantic_workflow` environment failures
   (`concurrent_edit_and_privacy_change_abort_atomic_embedding_publication`,
   `partial_chunk_insert_failure_rolls_back_complete_chunk_set`,
   `real_semantic_embedding_indexing_retrieval_and_sealed_refusal`,
   `reranking_temporal_validity_and_contradiction_lifecycle_evaluation`),
   identical to the Slice 4 round. Count reconciliation:
   Slice 4 total 334 + 11 new round tests (7 boundary +
   3 double-restore + 1 restore unit test) = 345.
   Providers+API: 144/0.

## What changed in the actual app (plain English)

- Starting a collision now re-checks that each pinned
  memory is still allowed to be sent: if its source was
  archived, sealed, expired or moved to another project
  since you picked it, the start refuses before any
  provider traffic, instead of sending an older revision
  past a current restriction. Your saved experiments
  never change.
- Experiments that reference other artifacts (for
  example, an earlier experiment used as project
  context) now survive backup and restore repeatedly:
  restore records the restored copy's fingerprint
  alongside the original, so the next restore accepts
  the restored copy instead of rejecting a previously
  valid result.
- The project picker no longer silently drops items
  past 50, hides load failures as empty lists, or
  leaves you without a retry. It shows privacy labels
  in the pre-send review, blocks a ninth item and
  overlong content with the limit named, prevents
  picking the same memory twice across the two
  pickers, and explains which items it hides and why.
- The Collider now has a pipeline overview at the top
  of the panel and the saved-experiment view: the real
  run state in plain words (not started, generating,
  validating, complete, failed, invalid, cancelling,
  cancelled), the six sections with how they relate,
  links into each section once complete, and a link to
  what was sent. No invented per-section progress and
  no animation. A 3D presentation remains unfinished.

## Remaining defects and deferred requirements

- Windows verification of Slice 4 is still in progress
  by the reviewer; Windows verification of this
  hardening checkpoint has not started. Linux evidence
  in this package must not be presented as closing
  either.
- M7 native desktop acceptance + preview promotion,
  M5 production gates, G1 audit and the §32 execution
  sandbox remain open (unchanged by this round).
- URL fetching, document ingestion and 3D
  visualization remain deferred in M8; SwarmTank 3D is
  the M9 item.
- Project-head expiry (a Temporary project record with
  a past expiry) is not a dispatch refusal; project
  archival and sealing already refuse. Noted in the
  WP1 record for the reviewer.
- The 4 known live-Ollama `semantic_workflow`
  environment failures are pre-existing on this VM
  (missing embedding model) and are the only full-core
  failures this round (345/4). The real-semantic
  `search.spec.ts` e2e failure is the same family,
  proven pre-existing on pristine `c7327bb7` in the
  Slice 4 round; it was not rerun in WP4's affected-spec
  set (see below).
- The full canonical e2e battery (all 29 specs) was
  not rerun end to end in WP4; the 16 affected specs
  above were run sequentially instead, per the WP4
  verification order. Unrelated specs are untouched by
  this round's diff (UI panel, collider/restore core,
  tests, docs).

## Windows reviewer checklist

1. Extract the ZIP, verify the SHA-256 and the bundle
   (`git bundle verify`), fetch into a scratch repo,
   confirm the exact tip and the Slice 4 ancestry
   (`0b6ba0e8`, `c7327bb7`, `532afccf`, `4ef90406d`).
2. Run the focused suites on Windows: collider,
   transfer, forge_restore, backup_restore, node.
3. Run the browser specs: collider-project-context,
   collider-picker-reliability,
   collider-pipeline-overview,
   collider-integrated-acceptance, transfer.
4. Confirm the WP1 boundary behavior through the real
   API with a synthetic provider (archived/sealed/
   expired/reassigned sources refuse before traffic;
   eligible older revisions still dispatch).
5. Confirm double restore of an experiment-context
   experiment and a transferred concept on Windows.
6. Confirm the picker behaviors (pagination past 50,
   privacy labels, bounds, retry) and the overview
   states in the Windows browser build.
7. Report any finding against the WP1–WP3 criteria
   files; do not edit reviewer-owned files, push,
   merge to master, or promote the preview.
