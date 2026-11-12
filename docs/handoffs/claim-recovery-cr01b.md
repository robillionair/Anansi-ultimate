# Claim recovery CR-01b — handoff

Branch: `muse/claim-recovery-cr01b`
Base: reviewer tip `c54c67b545a19d1d651bb51a5235afd50afb3829`
on `codex/review-claim-recovery-cr01-cr02` (one commit on
delivered tip `b5dc7fcb4`: the in-flight probe + review docs,
no production change)
Tip: the handoff commit on this branch (exact SHA in the
delivery message); last code commit `47e37ae66`
Date: 2026-11-12 (Linux evidence only — **WINDOWS RERUN
OUTSTANDING**; no full Windows acceptance is claimed from
Linux results)
Design (committed before implementation):
`docs/handoffs/claim-recovery-cr01b-design.md`
(`95dd59879`, AC-O1..O9)

## 1. Reviewer finding (preserved)

Windows review package
`claim-recovery-cr01b-windows-review-2026-11-12.zip`
(SHA-256 `0ae6f5aaf297ea4bd87d7fe0eae6be1982370b7f9fa9bdbed988f456e2cbb3a5`,
8,555 bytes; manifest verified, all 5 files match; incoming
ZIP hash matched `f2d6dd84…`; reviewer bundle head exactly
`c54c67b5`; runtime tip exactly `b5dc7fcb4`).
Verdict: changes required.

**CR-01b — passive polling resets an active root's failure
budget.** The deterministic queue probe takes an active entry,
models two failures, performs a passive wake while that entry
is out of the queue, then requeues the failed entry. Expected
failure count 2, actual 0. Mechanism: while the worker owns
the active entry, the pending queue has no matching entry, so
`wake(Passive)` inserts a fresh zero-count entry, and
`requeue(active)` sees that entry and drops the active
counter. Documents' 4 s polling can land in this window when
storage reads/locks take time. The delivered tests covered
pending and exhausted roots but not active ones.

Confirmed good in the same review: the original
exhausted-root probe passes unchanged, and its public
response carries `recoveryDiagnostic` (attempts 3, reason
`storageUnavailable`, fixed sanitized message). The 11
delivered queue tests pass; only the new probe fails. The
full 395-test Windows suite, builds, UI, and browser
acceptance were NOT rerun after this finding. The reviewer
also notes the new diagnostic and Retry recovery UI need
meaningful interaction coverage; the green browser battery
alone does not exercise the new failure UI.

## 2. What was changed

Commits on the branch (base `c54c67b5`):

- `95dd59879` — design + acceptance criteria (AC-O1..O9)
  before code.
- `47e37ae66` — the fix, tests, shared recovery notice, and
  interaction coverage.

Files changed vs base (8): `jobs.rs`, the design doc,
`JobRecovery.tsx` (new), `Jobs.tsx`, `Documents.tsx`,
`job-recovery.test.tsx` (new), `package.json` (test list),
`job-recovery.spec.ts` (new). `docs/BUILD_STATUS.md` and
`docs/reviews/` are untouched. Both reviewer probes are
byte-identical (verified by diff).

### CR-01b: active-root tracking

- `WorkerQueue` gains an `active` set: roots whose entry the
  worker loop holds outside the pending queue. `take_next`
  marks a root active; `finish` (drained), `requeue`, and
  parking clear the mark — every exit from serving a root.
  All mutations share the wake-up mutex.
- Wake matrix: a passive wake of an **active** root is now a
  no-op returning false (no fresh entry), so the in-flight
  entry's requeue carries its true failure count. A
  deliberate wake of an active root adds a fresh entry,
  served after the current attempt. Pending and exhausted
  behavior is unchanged (CR-01).
- Stale-failure guards: `requeue` drops the stale in-flight
  entry when a fresh deliberate entry is pending (existing
  rule, now meaningful for the active case), and
  `mark_exhausted` now returns false — marking nothing and
  recording no parked signal — when a pending entry exists
  for the root. A pending entry for an active root can only
  come from a deliberate wake, so a stale in-flight failure
  can never override a newer deliberate recovery decision;
  the fresh entry is served with its own budget.
- The worker loop's drain arm (`Ok(None)`) now clears the
  active mark under the queue lock. Taking work and deciding
  to stop still share one lock hold (shutdown wake safety
  preserved). Merging counts was rejected: it would let a
  stale in-flight count override a deliberate fresh budget.

### UI: shared recovery notice + interaction coverage

- The recovery notice (message + attempt count + Retry
  recovery button) is extracted from `Jobs.tsx` and
  `Documents.tsx` into `JobRecovery.tsx`, used identically
  by both views. The button appears only while the job is
  Queued; without a diagnostic nothing renders. The control
  wiring is unchanged (`control(job, 'recover')` /
  `recover(job)` → `controlJob(..., 'recover')`).
- Static tests (`job-recovery.test.tsx`, added to `test:ui`):
  diagnosed Queued shows the exact message, attempts, and
  button; diagnosed Paused shows the message without the
  button; the corrupt-storage message and attempt count;
  no diagnostic renders nothing; busy disables the button.
- Mock-API e2e spec (`job-recovery.spec.ts`): an in-process
  typed HTTP server (mocking the boot surface, the
  dashboard's reads, and the jobs ops, with the real API's
  CORS preflight behavior) serves the real app a diagnosed
  Queued job. The spec connects through the real UI, creates
  a workspace, opens Jobs, asserts the message and button,
  clicks Retry recovery, asserts exactly one control call
  with action `recover` and the observed version, and
  asserts the UI updates to the recovered job. The mock
  responses are typed with the generated contract
  interfaces and validated again by the client's Ajv
  validators at runtime.

### What did not change

Budgets (3 / 1), FIFO order, requeue-at-back, the 100 ms
retry delay, `claim_export`, `recover_jobs`, execution,
publication, and cancellation semantics, the exhaustion
record, the diagnostic DTO, and the Recover action. No
contract regeneration was needed (no DTO change). No
dependency upgrades, cloud calls, or preview promotion.
Real sign-in stays dev-gated.

## 3. Before / after evidence

In-flight probe, unchanged, on the reviewer tip (Linux):

```
test jobs::tests::reviewer_worker_queue_passive_wake_preserves_inflight_budget ... FAILED
assertion `left == right` failed: passive polling discarded the in-flight failure budget
  left: 0
 right: 2
test result: FAILED. 1 passed; 1 failed; 327 filtered out
```

(Identical on Windows: 1 passed, 1 failed, 326 filtered;
the original exhausted-root probe passes in both.)

In-flight probe, unchanged, after the fix (Linux):

```
test jobs::tests::reviewer_worker_queue_passive_wake_preserves_inflight_budget ... ok
test ingestion::tests::reviewer_status_poll_does_not_rearm_exhausted_claim_budget ... ok
```

Queue tests: 19/19 (12 prior + 7 new). Ingestion module:
38/38 unchanged.

## 4. Evidence (Linux, this VM)

- Rust lib, parallel broad run: **404 passed**, 0 failed
  (396 + 1 probe + 7 new queue tests).
- Rust lib, sequential broad run (`--test-threads=1`):
  **404 passed**, 0 failed.
- Full workspace `--tests` run: **642 passed, 4 failed** —
  the 4 failures are the pre-existing `semantic_workflow`
  live-Ollama tests (no local model on this VM), the same
  four names as before; no other failure in any test binary.
- Integration `chatgpt_signin`: **32/32 passed** (regression).
- Integration `provider_dispatch`: **39/39 passed** (regression).
- Focused queue unit tests: **19/19 passed** (12 + 7 new).
- Focused ingestion module: **38/38 passed** (unchanged).
- TypeScript: `tsc --noEmit` 0 errors (covers the new spec).
- Contracts: `--check` exit 0, 0 drift (no DTO change).
- Docs validation: **21/21 passed**.
- UI suite (`pnpm test:ui`): **347/347 passed** (342 + 5 new).
- Node suite (`pnpm test`): **39/40** — the one failure is the
  pre-existing MSVC installer test. Not a regression.
- e2e regressions (lease-run, one spec per invocation):
  connect-ai **1/1**, DOCX **7/7**, PDF **5/5**, TXT/MD **9/9**,
  Ask **1/1**, job-recovery **1/1** (new mock-API spec).
- Warnings: one, pre-existing — `crates/core/src/mcp.rs:715`
  (`preserved_env_entries` never used). No warning from this slice.
- `rustfmt` applied to `jobs.rs` only; the diff contains no
  stray reflows (all hunks are in the queue/worker/test
  regions).

Linux evidence is Linux evidence: it does not close the
Windows rerun. The Windows verdict comes from §7.

## 5. Acceptance criteria — how each was met

- **AC-O1 (CR-01b probe):** the in-flight probe passes
  unchanged (§3). Verified byte-identical to the reviewer
  version.
- **AC-O2 (original probe):** the exhausted-root probe still
  passes unchanged (§3).
- **AC-O3 (wake matrix):** new queue tests cover passive and
  deliberate wakes across pending (existing), active (new),
  and exhausted (existing) states: repeated passive wakes of
  an active root add nothing; deliberate wakes re-arm in all
  three states.
- **AC-O4 (stale safety):** `worker_queue_deliberate_wake_...`
  (stale requeue drops, fresh budget wins) and
  `worker_queue_stale_park_suppressed_...` (stale park
  suppressed, no exhaustion record, fresh entry served);
  `worker_queue_mark_exhausted_without_pending_entry_marks`
  proves parking still proceeds without a re-arm.
- **AC-O5 (preservation):**
  `worker_queue_deliberate_fresh_entry_keeps_fifo_position`
  (fresh entry queues behind pending roots),
  `worker_queue_duplicate_wakes_never_duplicate_active_entries`,
  `worker_queue_finish_clears_active_mark`, and all 12 prior
  queue tests green; shutdown wake safety preserved by the
  unchanged lock discipline (take + stop decision share one
  hold; active marking happens inside `take_next`).
- **AC-O6 (diagnostics + Recover):** ingestion module 38/38
  unchanged, including all diagnostic/Recover tests; the
  diagnostic DTO and Recover action untouched.
- **AC-O7 (UI interaction):** the shared notice has static
  rendering coverage across states (5 tests), and the
  mock-API e2e spec proves the real click → exact control
  call → UI update path on the Jobs view (§6 for the
  Documents limit).
- **AC-O8 (verification):** §4. Linux and Windows evidence
  kept distinct throughout; no full-Windows-acceptance claim.
- **AC-O9 (delivery):** this handoff + the checksummed bundle
  ZIP (§8). Reviewer docs untouched; no unrelated changes.

## 6. Known limits

- The mock-API e2e spec covers the Jobs view's button. The
  Documents view's button uses the same shared component and
  the same `controlJob(..., 'recover')` call, but its click
  is not browser-exercised: Documents shows the notice only
  for an active upload job, and reaching that state requires
  driving the upload flow. The shared component's rendering
  (message, attempts, button presence) is covered statically
  for both views.
- The exhaustion record remains in-memory only (a restart
  recovers), as before.
- The active set is cleared by every production exit from
  serving a root (drain, park, requeue). A test that takes an
  entry and drops it without any of those calls leaves a
  stale mark in that test's local queue only; no production
  path drops an entry.
- No change to `chatgpt-signin-status`, the API-key flow, or
  any global status document.

## 7. Windows instructions

1. Apply the bundle ZIP on Windows in a separate review
   worktree on top of reviewer tip `c54c67b5`
   (`codex/review-claim-recovery-cr01-cr02`); confirm the
   tip stated in the delivery message (handoff commit; last
   code commit `47e37ae66`) and a clean tree. Preserve the
   running pilot and existing data.
2. Run both reviewer probes unchanged, exactly:
   `node scripts/cargo.mjs test --locked -p anansi-core --lib reviewer_ -- --nocapture`
   Expect: 2 passed. The exhausted-root probe's printed
   status response should still carry `recoveryDiagnostic`.
3. Run the focused probes:
   `node scripts/cargo.mjs test --locked -p anansi-core --lib jobs::tests::worker_queue`
   (expect 19/19) and
   `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::`
   (expect 38/38).
4. Run the broad command:
   `node scripts/cargo.mjs test --locked --workspace --lib`
   Expect: all green. Windows library inventory is 396 at the
   reviewer tip + 7 new = 403 expected (one lib test is
   platform-gated — record the observed inventory, do not
   assume the Linux 404).
5. Run the focused suites: `--test chatgpt_signin` (expect
   32/32), `--test provider_dispatch` (expect 38/38, the
   Unix-only symlink test excluded), `tsc --noEmit` (0),
   contracts `--check` (0 drift), docs validation (21/21), UI
   suite (347/347).
6. Run the e2e regressions: connect-ai 1/1, DOCX 7/7, PDF 5/5,
   TXT/MD 9/9, Ask 1/1, job-recovery 1/1 (the new spec needs
   no real API; it starts its own mock on 127.0.0.1).
7. If a queue test fails, preserve the failure output in full:
   the count assertion names the budget that was discarded.
   Do not report a clean suite on an isolated rerun alone.
8. **Do not** enable the sign-in button or attempt a live
   Responses turn: eligibility is still unverified (spike
   handoff §5) — this slice changes no eligibility fact.

## 8. Reproduction commands (Linux, this VM)

- Reviewer probes (in-flight fails on the reviewer tip, both
  pass after):
  `node scripts/cargo.mjs test --locked -p anansi-core --lib reviewer_ -- --nocapture`
- Queue discipline:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib worker_queue`
- Fault + diagnostic tests:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::`
- Broad library suite, parallel and sequential:
  `node scripts/cargo.mjs test --locked --workspace --lib`
  `node scripts/cargo.mjs test --locked --workspace --lib -- --test-threads=1`
- Full test binaries:
  `node scripts/cargo.mjs test --locked --workspace --tests`
  (expect the 4 pre-existing live-Ollama failures only)
- UI + new e2e spec:
  `pnpm test:ui`
  `node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/job-recovery.spec.ts`

## 9. Out of scope (unchanged)

- `claim_export`, `recover_jobs`, execution, publication, and
  cancellation semantics — all untouched.
- Persisting exhaustion or active state across restarts; any
  new polling, auto-retry, or worker-status API beyond the
  per-job diagnostic.
- Retrying execution failures (unchanged: `fail_export`).
- The backup/restore/ask/provider worker threads (different
  machinery, untouched).
- Any change to the API-key flow, the ChatGPT sign-in
  prototype, the frozen Windows pilot, or any provider
  op/contract.
- `providers.spec.ts` 240 s timeout and the other pre-existing
  failures (larp-somnia instruments test, backups.spec.ts
  'Workspace' button, 4 semantic_workflow live-Ollama failures,
  node pwsh failure, MSVC installer test — all pre-existing on
  this VM).
- No global status document was edited (Antigravity-owned files
  untouched, including the reviewer's BUILD_STATUS entry,
  which stays as the reviewer wrote it); proposed updates, if
  any, belong in this handoff only.
