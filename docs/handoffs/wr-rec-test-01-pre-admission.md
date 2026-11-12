# WR-REC-TEST-01 follow-up: pre-admission stall clock — handoff

Branch: `muse/claim-recovery-wr-rec-test-01-pre-admission`
Base: reviewer tip `7cf1068b1fae2ce1410184ad5a3447f6e765cb64`
(one docs-only commit on delivered tip `e32456574`:
BUILD_STATUS +4 lines, review doc
`docs/reviews/windows-wr-rec-test-01-review.md`)
Tip: the handoff commit on this branch (exact SHA in the
delivery message); last code commit `6b033fd0e`
Date: 2026-11-12 (Linux evidence only — **WINDOWS RERUN
OUTSTANDING**; no full Windows acceptance is claimed from
Linux results)
Design (committed before implementation):
`docs/handoffs/wr-rec-test-01-pre-admission-design.md`
(AC-Q1..Q8)
Review package: `wr-rec-test-01-e324-windows-review-2026-11-12.zip`
(SHA-256 `e66316b5…`, 15,825 bytes; all 7 manifest hashes
verified; review bundle head exactly `7cf1068b`).

## 1. What the Windows review established

The review of the rebased repair (`e32456574`) ran the
default broad command with no other reviewer build or test
alongside: API 4/4, core 336 passed + 1 failed (204.70 s),
Cargo stopped at the core failure so desktop/provider
binaries never ran. The one failure:

```text
thread 'ingestion::tests::deliberate_wake_rearms_exhausted_root' panicked
at crates\core\src\ingestion.rs:888:9:
root "E:\\Anansi\\tmp\\.tmpjV3v4o\\storage" was never parked:
no admission/attempt progress within the 10s stall bound after 10.0073093s;
job state Queued v2 log="Upload complete; queued for import";
admitted 0 -> 0, attempted 0 -> 0, remaining faults 1000000;
progress history (elapsed_ms, admitted, attempted): [(0, 0, 0)]
```

The review's verdict: changes required, not a clean
405-test Windows acceptance, no production recovery
defect established. The new instrumentation narrowed the
finding to the pre-admission case: calling the same
10-second limit a stall bound still starts that clock
before the root's first admission, and the counters do
not distinguish a registered pending root from a root
that was never scheduled. Everything else passed: the
other formerly failing test, the twelve-root contention
test, the never-admitted test (expected panic), the
failing test in isolation (1/1, 7.89 s), both reviewer
probes (2/2), queue tests (19/19).

## 2. Root cause

`wait_for_parked` keyed its 10 s stall bound on
admission/attempt counter progress only. Three different
situations all read as "0 admissions, 0 attempts":
registered pending (legitimate FIFO queueing behind other
fixtures), never registered (a lost wake), and admitted
then stuck. In the failing test the finished upload part
wakes the root — registered pending — but under broad-run
contention the single process-global worker does not
admit it within 10 s, so the stall bound, starting at
wait entry, expires on a healthy queued root. Admitted
0→0, attempted 0→0, all 1,000,000 injected faults
remaining proves no claim attempt ever ran: a
test-synchronization defect, not a backend defect. No
probe proves a production defect, so production is
unchanged (see §4).

## 3. The fix (test-only)

**Worker-state mirror** (jobs.rs, `#[cfg(test)]` only):
a per-root mirror of the queue state the worker itself
maintains — `pending`, `active`, `ever_registered` —
updated by hooks inside the `WorkerQueue` methods
(`wake`, `take_next`, `finish`, `requeue`), following the
existing `CLAIM_ADMISSIONS`/`CLAIM_ATTEMPTS` pattern.
The mirror tracks exactly the entries the worker serves,
including a deliberate wake of an active root (pending
AND active) and requeues (active back to pending).

**Phase machine** (ingestion.rs test module): the wait is
factored into a pure machine over sampled observations
(mirror state + counters) and elapsed time. Progress is
any change in any observed field, so registration,
admission, and requeue each reset the stall clock.
Phases: **pending** (no stall expiry — registered work
waiting its turn; only the overall bound applies),
**never registered** (10 s stall, never-registered
reason), **active** (10 s stall, active-stall reason),
**left queue** (10 s stall, left-queue reason). The stall
clock therefore starts at the first transition out of
the pending phase — admission — never before it. The
60 s overall safety bound applies in every phase,
including pending-forever. The 25 ms poll is unchanged;
the parked check still comes first each poll; the wait
still never re-wakes.

**Diagnostic:** the expiry panic keeps the existing
fields and adds the worker state plus a state history
`(elapsed_ms, pending, active, admitted, attempted)`.
The Windows 0→0 case would now read as
registered-pending with a full state history. On success
the wait stashes its transition history in a test-only
table so tests can assert what the wait observed.

**Bounds unchanged:** 10 s stall, 60 s overall, 25 ms
poll — the same values, with the stall bound now
applied per phase instead of from wait entry. No timeout
increase, no added sleeps, no weakened assertions, no
serial execution.

## 4. Production unchanged

The jobs.rs diff is additions-only (zero removed or
changed lines): five `#[cfg(test)]` hook statements, the
mirror table/struct/accessor, and unit tests. The
ingestion.rs diff is confined to the test module
(`wait_for_parked`, `parked_timeout`, new tests, and the
never-admitted test's expected panic string + comment).
No queue, worker, budget, FIFO, fault-seam, publication,
or diagnostic logic changes. Verified by diff: both
reviewer probes, both formerly failing tests, and the
twelve-root contention test are byte-identical to the
reviewer tip.

## 5. Coverage (all deterministic; no wall-clock assertions)

- **Mirror unit tests** (3): wake→take→requeue→finish
  cycle; deliberate wake of an active root is
  pending+active; passive wake of an exhausted root
  records nothing (plus a passive wake of an idle root
  registers it). Unique root names per test — the
  existing queue tests share fake roots across parallel
  tests, so mirror assertions use keys no other test
  touches.
- **Scripted phase-machine tests** (5): scripted
  observations + synthetic elapsed times, no threads,
  no sleeps — pending 15 s with 0/0 never expires;
  never-registered expires at 10 s with the
  never-registered reason; active stall expires 10 s
  after admission with the active-stall reason; requeue
  cycles with 6 s gaps spanning 30 s never expire;
  pending-forever expires at 60 s with the distinct
  overall reason.
- **Integration test** (1): thirty fault roots queue
  ahead of the observed root (each costs the worker
  ≥200 ms of forced kindness delay, so the observed
  root provably waits pending before its first
  admission). Asserts exact 3 admissions / 3 attempts /
  exact fault consumption, every root parks, AND the
  wait's stashed history contains a pending observation
  with 0 admissions — the end-to-end proof that the
  wait saw pending-before-admission.
- **Updated never-admitted test**: body unchanged
  (never-queued setup); only the `should_panic`
  expected string changed to the new never-registered
  reason. Still fails fast in 10.03 s on the real
  clock — the real-time stall path proof.

Known limits (design §7): pending-dominant phase
resolution (pending+active suspends stall detection
until the overall bound — the overall bound is the
backstop); active-stall and overall-expiry coverage is
scripted, not real-time (freezing the real worker
mid-claim would need a new hook; a real 60 s test
would slow the suite for no additional decision
coverage); the thirty-root test adds roughly 7–10 s to
the Linux core suite.

## 6. Before / after evidence

Before — at the reviewer tip, from the review package's
broad log (`wr-rec-test-01-windows-broad-1.log`): the
panic quoted in §1, core 336 + 1 failed in 204.70 s.
The old diagnostic could not say whether the root was
queued, lost, or stuck.

After — on this branch (Linux, §7): the failing test
passes in the default broad command (3 consecutive
runs) and the phase machine proves by construction
that a registered pending root cannot expire the stall
bound. The true before/after verdict for the Windows
broad command comes from the rerun in §8: this slice
does not claim it from Linux results.

## 7. Evidence (Linux, this VM, on this branch)

- Rust lib, default broad command, 3 consecutive runs:
  **415 passed** (4 + 347 + 1 + 63), 0 failed each run.
- Rust lib, sequential broad run (`--test-threads=1`):
  **415 passed**, 0 failed (regression check only — the
  default parallel command is the acceptance command).
- Full workspace `--tests` run: **653 passed, 4 failed** —
  the 4 failures are the pre-existing `semantic_workflow`
  live-Ollama tests (no local model on this VM), the same
  four names as before; no other failure in any test binary.
- Integration `chatgpt_signin`: **32/32 passed** (regression).
- Integration `provider_dispatch`: **39/39 passed** (regression).
- Focused queue tests (`worker_queue` filter): **19/19**
  (unchanged tests, unchanged count).
- Mirror state tests (`worker_test_state` filter):
  **3/3**.
- Focused ingestion module: **46/46** (40 + 5 scripted
  phase tests + 1 integration test).
- Parking-wait tests (`parking_wait_` filter): **7/7**
  (5 scripted + integration + never-admitted).
- Reviewer probes (`reviewer_` filter): **2/2**.
- The two formerly failing tests individually: **1/1 each**
  (in the ingestion module run).
- TypeScript: `tsc --noEmit` 0 errors.
- Contracts: `--check` exit 0, 0 drift (no DTO change).
- Docs validation: **21/21 passed**.
- UI suite (`pnpm test:ui`): **347/347 passed**.
- Node suite (`pnpm test`): **39/40** — the one failure is
  the pre-existing MSVC installer test. Not a regression.
- e2e regressions (lease-run, one spec per invocation):
  connect-ai **1/1**, DOCX **7/7**, PDF **5/5**, TXT/MD **9/9**,
  Ask **1/1**, job-recovery **1/1**.
- Warnings: one, pre-existing — `crates/core/src/mcp.rs:715`
  (`preserved_env_entries` never used). No warning from
  this slice.
- `rustfmt` state: the two touched files formatted
  (non-root files only); the diff contains no stray
  reflows — hunks are confined to the hook sites, the
  mirror, the wait rewrite, and the new tests.

Linux evidence is Linux evidence: it does not close the
Windows rerun. The Windows verdict comes from §8.

## 8. Windows rerun instructions

1. Apply the bundle ZIP on Windows in a separate review
   worktree on top of reviewer tip `7cf1068b`
   (docs-only commit on `e32456574`); confirm the tip
   stated in the delivery message (handoff commit; last
   code commit `6b033fd0e`) and a clean tree. Preserve
   the running pilot and existing data.
2. Run the exact acceptance command, unchanged, with
   default Rust concurrency (do NOT substitute
   `--test-threads=1`):
   `node scripts/cargo.mjs test --locked --workspace --lib`
   Expect: all green. Windows library inventory is 405
   at the reviewer tip + 9 new tests = 414 expected
   (one lib test is platform-gated — record the
   observed inventory, do not assume the Linux 415).
   The command must execute ALL FOUR library binaries
   (API, core, desktop, providers): the review's broad
   invocation stopped at the core failure, so a
   full-inventory green run is still the open claim. If
   any test fails, preserve the failure output in full:
   the new diagnostic names the worker state, which
   decides whether the cause is queueing, a lost wake,
   a stalled worker, or a parking defect. Do not report
   a clean suite on an isolated rerun alone.
3. Run the focused checks: both reviewer probes
   (`reviewer_` filter, expect 2/2), queue tests
   (`worker_queue` filter, expect 19/19), mirror tests
   (`worker_test_state` filter, expect 3/3), ingestion
   module (expect 46/46), parking-wait tests
   (`parking_wait_` filter, expect 7/7), the two
   formerly failing tests individually,
   `--test chatgpt_signin` (expect 32/32),
   `--test provider_dispatch` (expect 38/38, the
   Unix-only symlink test excluded), `tsc --noEmit` (0),
   contracts `--check` (0 drift), docs validation
   (21/21), UI suite (347/347).
4. Run the e2e regressions: connect-ai 1/1, DOCX 7/7,
   PDF 5/5, TXT/MD 9/9, Ask 1/1, job-recovery 1/1.
5. **Do not** enable the sign-in button or attempt a live
   Responses turn: eligibility is still unverified (spike
   handoff §5) — this slice changes no eligibility fact.

## 9. Reproduction commands (Linux, this VM)

- The formerly failing test:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib deliberate_wake_rearms_exhausted_root`
  `node scripts/cargo.mjs test --locked -p anansi-core --lib diagnostic_absent_for_other_states_and_roots`
- Reviewer probes:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib reviewer_ -- --nocapture`
- Queue discipline + mirror:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib worker_queue`
  `node scripts/cargo.mjs test --locked -p anansi-core --lib worker_test_state`
- Phase machine + integration:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib parking_wait_`
- Fault + diagnostic tests:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::`
- Broad library suite, default concurrency (acceptance):
  `node scripts/cargo.mjs test --locked --workspace --lib`
- Full test binaries:
  `node scripts/cargo.mjs test --locked --workspace --tests`
  (expect the 4 pre-existing live-Ollama failures only)

## 10. Acceptance criteria (AC-Q1..Q8, from the design doc)

- **AC-Q1 (diagnosis):** expiry names the worker state
  and carries the state history; the Windows 0→0 case
  is distinguishable as registered-pending.
- **AC-Q2 (pending phase):** a registered pending root
  with zero admissions does not expire the stall bound;
  the stall clock starts at the first transition out of
  pending (scripted + integration proof).
- **AC-Q3 (never registered):** a never-registered root
  fails fast at the 10 s stall bound with the
  never-registered reason (scripted + real test).
- **AC-Q4 (active stall):** an admitted root with no
  further progress expires at the 10 s stall bound with
  the active-stall reason (scripted proof).
- **AC-Q5 (requeue):** retry requeue cycles count as
  progress; multi-cycle waits complete with exact 3/3
  counts and exact fault consumption.
- **AC-Q6 (overall bound):** the 60 s overall bound
  fires for a root that never parks, including
  pending-forever, with the distinct overall reason
  (scripted proof).
- **AC-Q7 (preservation):** reviewer probes
  byte-identical; all recovery assertions green; the
  twelve-root test untouched; jobs.rs production paths
  unchanged (additions-only diff, all `#[cfg(test)]`).
- **AC-Q8 (delivery):** this handoff + the checksummed
  bundle ZIP from reviewer tip `7cf1068b`. Reviewer
  docs untouched. No full-Windows-acceptance claim.

## 11. Out of scope (unchanged)

- Any change to production scheduling (worker count,
  queue discipline, budgets, retry delay) — none is
  indicated; FIFO + requeue-at-back is the accepted
  fairness design.
- Serializing the test suite or the fault tests.
- `claim_export`, `recover_jobs`, execution, publication,
  and cancellation semantics — all untouched.
- Persisting exhaustion or active state across restarts;
  any new polling, auto-retry, or worker-status API.
- The backup/restore/ask/provider worker threads
  (different machinery, untouched).
- Any change to the API-key flow, the ChatGPT sign-in
  prototype, the frozen Windows pilot, or any provider
  op/contract.
- `providers.spec.ts` 240 s timeout and the other
  pre-existing failures (larp-somnia instruments test,
  backups.spec.ts 'Workspace' button, 4 semantic_workflow
  live-Ollama failures, node pwsh failure, MSVC installer
  test — all pre-existing on this VM).
- No global status document was edited (Antigravity-owned
  files untouched, including BUILD_STATUS.md, README.md,
  REQUIREMENTS_MATRIX.md, ROADMAP.md, the review doc,
  and the published evidence); proposed updates, if any,
  belong in this handoff only. Nothing was merged, pushed,
  or promoted: this branch ships as a verified bundle
  inside a ZIP.
