# SOMNIA compaction authorization — handoff

## §0 Baseline, branch, scope

- **Baseline (verified):** the published Windows preview,
  `codex/windows-preview-integration` at
  `cf69e37e667a7767f6ce1b7c0f5dc81901e0a187`. Verified by
  `git ls-remote` (origin's branch tip was exactly `cf69e37e`),
  fetched read-only into `preview/windows-preview-integration`.
  It descends from `4e4e970a` and contains the Checkpoint 2 tip
  `726aa8a07` (merged by `ad29bb5ae`).
- **Branch:** `muse/somnia-compaction-authorization`, created
  from `cf69e37e`. Commits (in order):
  `31402e19d` design + acceptance criteria (AC-A1..A10, committed
  before implementation), `664bfbdce` implementation,
  `d14bb9351` e2e approval-hash fixes for the older starts, plus
  this handoff commit. The delivery MANIFEST is authoritative for
  the final tip hash (tip-proof pattern).
- **Scope:** fix SOMNIA automatic-compaction model selection and
  authorization without weakening the provider gates. Three
  defects (design doc
  `docs/handoffs/somnia-compaction-authorization-design.md` §1):
  blank Model ID refused although the panel promises it selects
  the first configured model (F1); the cloud control offers a
  comparison the server can never run, and a permit could never
  cover it (F2); approval binds only the candidate hash
  server-side (F3). Stop at this slice. No real credentials, paid
  inference, pushes, preview promotion, or remote CI.

## §1 What changed (implementation, commit `664bfbdce`)

9 files. Server (`crates/core/src/somnia_compaction_run.rs`):

- `StartRequest` gains `approvalHash: Option<String>`.
  `StartResponse`, `PollResponse`, and the `CompactionRun`
  record gain `cost: String` (`"free"` / `"paid"` /
  `"unknown"`).
- `RunConfig.permit_id` is REPLACED by `cloud_authorized: bool`.
  The run never sends a permit; dispatch sets
  `explicitCloudAuthorization` from the approved cloud flag.
- `compute_approval_hash` (+ `ApprovalHashInput` /
  `ApprovalHashCase`): hex SHA-256 over canonical JSON
  `{cases: [{caseId, candidateHash, questions}] (request order),
  provider, model, cloud, maxTotalTokens}` with the RAW approved
  provider/model (trimmed, blank allowed). The server verifies
  each candidate hash first, then recomputes the approval hash
  from the verified hashes + the request fields, and refuses on
  mismatch — before model resolution and before any dispatch.
- Blank-model resolution in `resolve_model`: `(Some(p), None)`
  or a blank model string → the first configured chat model for
  provider `p`, in configuration order
  (`configured_chat_models`); `(None, None)` → first overall;
  `(Some, Some)` must be configured; `(None, Some)` still
  refused. Blank strings are treated as absent.
- Free-only gate (`check_run_target`, pure): cost class from the
  trusted configuration via the new `configured_model_cost`
  (`crates/core/src/provider_dispatch.rs`, + `CostClass`
  import). Free → allowed, local or cloud. Paid/Unknown →
  refused up front (message contains "free"). A supplied
  `permit_id` → refused up front. Cloud unchecked + cloud
  locality → refused. Zero-Cost Lock untouched (still runs per
  dispatch inside `authorize_cost`).
- 2 new unit tests: `approval_hash_binds_every_reviewed_field`,
  `free_only_gate_matrix`.

Client (`packages/ui/src/SomniaCompaction.tsx`):

- `computeCompactionApprovalHash` export (same key order as the
  server; the e2e passing proves byte agreement).
  `ApprovedSnapshot` extended with provider/model/cloud/
  maxTotalTokens/approvalHash. `approve()` is async and hashes
  the trimmed values; `start()` sends the SNAPSHOT values +
  `approvalHash`. The cloud checkbox gained a free-only note.
  Approved and results lines show the target + cost
  (`locality · cost`).
- Saved-experiment format stays v1, unchanged.

Contracts regenerated (`generated.ts` has
`approvalHash?: string | null` and `cost` on the responses;
`validators.mjs`; `readiness.json`); `--check` exits 0.

Tests (`tests/e2e/somnia-compaction.spec.ts`,
`somnia-compaction-benchmark.spec.ts`,
`packages/ui/tests/somnia-compaction.test.tsx`):

- E2E helpers: `computeApprovalHash` mirror (Node crypto),
  `startCloudFake` (OpenAI-compatible, echoes the model, usage
  12+4), session options `paidModel` + `cloudModel {id, cost}`,
  session exposes `fakeModels` / `cloudFakeHits` /
  `cloudFakeModels`.
- 4 NEW e2e tests: blank-model UI run with `aaa-decoy` (AC-A1);
  free-cloud UI run (AC-A3); billable/permit refusal API test
  with a valid-hash control (AC-A4, AC-A7); approval-tamper API
  test — no-hash + questions/target/budget/candidate tampers,
  each 400 + 0 new hits (AC-A5).
- UI tests +1 (`compaction approval hash binds every reviewed
  field`).
- Commit `d14bb9351`: the 3 older workflow tests and the
  benchmark spec now send a valid `approvalHash` on their
  legitimate starts (computed over the exact reviewed values,
  blank provider/model for the no-target calls).

## §2 Before / after evidence

Before (unfixed code, same 4 new tests): all 4 failed — the
blank-model and free-cloud runs never completed; the billable
test's control start returned 400 (unknown field); the no-hash
start returned **200** (F3 reproduced exactly).

After (Linux, this session, tip `d14bb9351`):

- Rust `somnia_compaction` filter 31/31; workspace lib
  **341/341** (api 4 + core 275 + desktop 1 + providers 61).
- tsc 0; contracts 0 drift; docs **327 PASS** (326 before this
  handoff was added); UI **336/336**.
- Compaction workflow + benchmark e2e **11/11** (10 workflow +
  1 benchmark).
- Benchmark synthetic numbers IDENTICAL to Checkpoint 2:
  **21/50 original, 21/50 compacted, 0 failed pairs, 1600
  tokens**, compaction 7258 µs this run (timing noise). Report
  in the delivery evidence, labelled synthetic (40 cases, 50
  pairs, 100 calls, abstentions 29/29). The frozen benchmark
  fixtures and the scorer are unchanged.
- Full regression record in the delivery evidence
  (`evidence/regressions.md`): SOMNIA slices 1–3 + model
  comparison 8/8; documents/PDF + SwarmTank AI 22/22;
  SwarmTank market/scenario/arena/slice1 10/10; backups 1/1;
  LARP scene 09 replay 1/1.

## §3 Acceptance criteria — status

- **AC-A1** Blank Model ID resolves — PASS (UI run, blank model,
  `llama-x` first of two; every dispatch body carries
  `llama-x`; poll echoes `ollama` / `llama-x`).
- **AC-A2** Refusals preserved — PASS (blank provider + explicit
  model refused; none configured refused with the existing
  message; explicit unconfigured model refused; all before
  traffic).
- **AC-A3** Free cloud runs — PASS (openrouter free model, cloud
  checked, OpenAI-compatible loopback fake; both arms dispatch
  with the exact provider/model; no permit; poll reports
  locality `cloud`, cost `free`).
- **AC-A4** Billable targets refused before traffic — PASS (Paid
  and Unknown, local and cloud, 400 with the free-only message,
  0 fake hits; supplied `permit_id` refused, 0 hits).
- **AC-A5** Approval invalidation is server-side — PASS
  (questions / target / budget / candidate tampers, each 400 +
  0 new hits; no-hash start 400).
- **AC-A6** Pinning holds — PASS (fake records every dispatch
  body; all carry the exact selected provider/model; replayed
  approval with a changed target refused by AC-A5).
- **AC-A7** Zero-traffic proofs — PASS (missing authorization,
  stale approval, disallowed target: 0 hits each, asserted on
  the fake's hit counter).
- **AC-A8** Frozen benchmark and scorer preserved — PASS (same
  synthetic numbers; no change to `somnia_compaction.rs`, the
  fixtures, or the scorer).
- **AC-A9** Regressions green (Linux) — MET EXCEPT two
  pre-existing failures, neither introduced by this slice:
  `providers.spec.ts` (single workflow test, 240 s timeout —
  reproduced IDENTICALLY on the pristine baseline `cf69e37e`
  in a worktree with the baseline's own binary and web dist;
  no change in this slice touches the provider dispatch path)
  and the `larp-somnia.spec.ts` instruments test (reproduced
  identically on pristine `4e4e970a` in the Checkpoint 2
  session). Both are reported as-is, not fixed.
- **AC-A10** No new cloud calls, no real credentials, no paid
  inference — PASS (all provider traffic in tests goes to
  loopback fakes; the free-cloud fake is loopback too).

## §4 Known limits

- **D6 (design §2):** approval binds the selection as
  specified; a blank model is a reference to "first configured
  at start time" — if the configuration changes between approval
  and start, the run uses the then-first model and reports it.
  The run's recorded target is the authority for what ran. A
  future `providers-configured-models` read op would let the UI
  resolve at approve time and bind the concrete model; that is
  a follow-up, not this slice.
- **Billable comparisons are not possible yet** (by design):
  one permit authorizes one dispatch chain, and a comparison
  needs up to 100 dispatches. The design doc §5 records the
  missing run-scoped permit contract (run id + provider/model
  scope, call cap ≤100, token cap ≤ approved `maxTotalTokens`,
  TTL covering the run, consumed once per run, owner-issued,
  caps shown in review). Not implemented in this slice.
- **Live rerun still blocked** on this VM (no local model, no
  provider credential), as in Checkpoint 2. The rerun path is
  unchanged: `ANANSI_SOMNIA_LIVE_ENDPOINT` /
  `ANANSI_SOMNIA_LIVE_MODEL` (see the Checkpoint 2 handoff §4).
  AC-I10 remains open.
- **Build hygiene incident:** while reproducing the providers
  failure on the baseline worktree, the baseline API was built
  with a shared `CARGO_TARGET_DIR`; cargo fingerprints collide
  across worktrees, so the baseline binary silently replaced
  the branch binary while the build still reported fresh. The
  first compaction re-run failed 2 tests in exactly the
  baseline pattern; the poisoning was proven with `strings`
  and fixed (worktree removed, `cargo clean -p anansi-core
  -p anansi-api`, rebuild, literal verified, specs re-run
  11/11). Lesson recorded in the workspace `AGENTS.md`.
  The worktree is gone; no baseline files remain in the repo.

## §5 Proposed global-status updates

NONE. AN-015 stays `verified`; no new requirement row until
Windows review. (Antigravity owns `BUILD_STATUS.md`, the
roadmap, the requirements matrix, and release documents; they
were not edited.)

## §6 Windows review notes

- Windows review outstanding for this slice (Linux evidence
  only, as above).
- The Windows reviewer should re-run: the compaction workflow
  + benchmark specs (11 tests), `providers.spec.ts` (note the
  Linux pre-existing timeout — a Windows pass/fail there is
  independent evidence, not a regression of this slice), and
  `larp-somnia.spec.ts` (instruments test known-failing on
  Linux; a Windows result for it is new information).
- The live rerun (AC-I10) needs a local model or a provider
  credential on the Windows side; it cannot be done on this VM.
