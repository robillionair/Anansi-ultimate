# WR-REC-TEST-01 — design

Branch: `muse/claim-recovery-wr-rec-test-01`
Base: reviewer tip `4ffdd431664a6e9258d1066cae97280ef61ad4b7`
on `codex/review-claim-recovery-cr01b` (docs-only commit on
delivered tip `8df1fcf3d`: BUILD_STATUS entry + review doc)
Date: 2026-11-12
Status: design + acceptance criteria, committed BEFORE any
code change. Characterization first; the fix follows the
evidence.

## 1. Windows evidence (preserved)

Windows review package
`claim-recovery-8df1fcf3-windows-review-2026-11-12.zip`
(SHA-256 `b54943d58b3c1d4f2d0154922ed4fc779b2dbd33d38f1d3088cf88032cbba54e`,
46,390 bytes; manifest verified, all 27 files match;
delivery ZIP hash matched `0ae9a2b0…`; reviewer bundle head
exactly `4ffdd431`; runtime tip exactly `8df1fcf3d`).
Verdict: focused fixes pass; broad Windows test reliability
needs repair. No new production recovery defect is
established. The CR-01b implementation is verified in
focused scope and must be preserved: probes 2/2, queue
19/19, sign-in 32/32, provider dispatch 38/38, tsc 0,
contracts 0 drift, UI 347/347, docs 21/21, browser 24/24
across 6 specs including job-recovery 1/1.

**WR-REC-TEST-01.** Exact command:
`node scripts/cargo.mjs test --locked --workspace --lib`.

- First run (overlapped compilation, web/dependency setup,
  other verification): API 4/4; core 331 passed, 4 failed
  (332.15s): `corrupt_claim_failure_parks_immediately_...`,
  `deliberate_wake_rearms_exhausted_root`,
  `diagnostic_absent_for_other_states_and_roots`,
  `diagnostic_shown_for_exhausted_queued_job_...`.
- Second quieter rerun (default Rust concurrency intact,
  after all other work finished): API 4/4; core 333 passed,
  2 failed (170.79s): `deliberate_wake_rearms_exhausted_root`
  and `diagnostic_absent_for_other_states_and_roots`.
- Every failure is the helper at ingestion.rs:818: the root
  was not observed parked within 10s; job still Queued v2,
  log `Upload complete; queued for import`. All four pass
  individually (7.48s / 7.98s for the two in the quiet
  rerun). The Cargo command stops at the core failure, so
  the desktop/provider library binaries never ran in either
  broad invocation — their inventory must not be counted
  as passing.

## 2. Scheduling mechanism (read before changing anything)

One process-global worker thread serves all fixtures:

- `take_next` pops the FRONT of the FIFO pending queue and
  marks the root active. If the entry carries failures, the
  worker sleeps 100 ms (kindness delay) while holding the
  entry — serving nothing else during the delay.
- The inner loop claims until drained: each `Ok(Some)` lease
  executes fully (SQLite claim + hash verify + chunking +
  indexing) before the next claim for the same root.
- A `StorageUnavailable` claim failure increments the count;
  below the budget of 3 the entry is requeued at the BACK,
  so the root waits for every other pending root to drain
  before its next attempt. At 3, the root parks.
- A non-retryable failure parks immediately (1 attempt).

Time-to-park for a fault root is therefore 3 admissions
plus 2 requeue cycles, each cycle lasting as long as every
other pending root's full drain. In the broad run, dozens
of fixtures share the worker (default test concurrency),
so each cycle can take seconds; the 10s TOTAL bound in
`wait_for_parked` starts when the test calls it, with no
knowledge of how much queueing stands ahead of the root.
The failure set differs between the two runs (4 vs 2) and
vanishes in isolation — the signature of contention-driven
queueing delay, not a test-specific defect. No starvation
mechanism exists in principle: FIFO + requeue-at-back
guarantees eventual service, and the take/stop lock
discipline prevents lost wake-ups.

What is missing is observability: at expiry the panic
reports only the job's state/version/log. It cannot
distinguish "never admitted" (pure queueing delay) from
"admitted 3 times, attempted 3 times, never parked" (a
backend defect). The characterization below closes that
gap before any wait logic changes.

## 3. Characterization plan (before the fix)

1. **Instrumentation (test-only).** Two statics beside
   `CLAIM_FAULTS`/`CLAIM_PARKED`, compiled out of shipped
   builds: per-root admission counts (incremented at the
   worker loop's take site) and per-root attempt counts
   (incremented in `claim_for_worker` on every call), with
   read accessors. Roots are unique tempdirs per test, so
   no cross-test collision; the wait helper snapshots
   values at entry and reports deltas, so no clearing is
   needed.
2. **Expiry diagnostics.** `wait_for_parked` reports, on
   expiry: admissions (start → end), attempts (start →
   end), remaining faults, job state/version/log, and the
   progression timeline. This diagnostic is permanent.
3. **Experiments.** Stressed Linux runs (elevated test
   threads, repeated broad + ingestion-module runs) to
   observe admission/attempt progression under contention,
   plus a contention probe (many healthy roots ahead of
   one fault root) measuring time-to-park against queue
   depth. The verdict rule is fixed in advance: expiries
   with admissions < 3 or attempts < 3 establish queueing
   delay (test-harness issue, no backend defect); any
   expiry with admissions = 3 and attempts = 3 without
   parking establishes a backend defect and stops this
   slice for a production repair instead.
4. **Fix only after the verdict.** If (as the evidence so
   far suggests) the verdict is queueing delay, the fix is
   the progress-aware wait in §4. Production code stays
   untouched either way unless a deterministic probe
   proves a backend defect.

## 4. Fix: progress-aware parking wait (test helper only)

`wait_for_parked` synchronizes on the admission → attempt →
parking progression instead of a blind total bound:

- **Stall bound: 10 s, unchanged value, new semantics.**
  The bound applies to time without observable progress
  (an admission, an attempt, or parking), not to total
  elapsed time. A genuinely stuck root still fails within
  10 s of its last progress.
- **Overall safety bound: 60 s**, mirroring the existing
  `wait_for_terminal` bound in the same file. The suite
  can never hang on this helper.
- **Poll interval unchanged (25 ms).** No new sleeps, no
  serial execution, no weakened assertions.
- **Progress history** recorded like `wait_for_terminal`'s
  transition history; the panic on either bound names the
  bound type, elapsed time, admissions/attempts
  (start → end), remaining faults, job state/version/log,
  and the history.

All 14 `wait_for_parked` call sites (all in ingestion.rs:
fault tests + reviewer probe) gain the new behavior
without source changes to the tests themselves. Exact
attempt-count assertions, cancellation tests, fairness
tests, and both reviewer probes are untouched and must
all still pass.

## 5. Regression coverage

A contention test: many healthy roots with small jobs are
queued ahead of one fault root, and the test asserts the
fault root parks with exactly 3 attempts consumed — via
the progress-aware wait, asserting behavior (admissions
reach 3, attempts reach 3, parked) rather than any
wall-clock bound. It fails under the old total-bound wait
when the queue ahead is deep enough, and passes under the
new wait. It is run repeatedly on Linux to confirm
stability before delivery.

## 6. What does not change

- Production code: the worker loop, queue, park logic,
  budgets, FIFO order, retry delay, and the fault seam's
  behavior are untouched. The only jobs.rs additions are
  `#[cfg(test)]` counter increments, compiled out of
  shipped builds.
- `wait_for_terminal`: untouched (not failing).
- No timeout increases (the 10 s value is preserved as the
  stall bound), no added sleeps, no weakened assertions,
  no serial execution: the default broad command remains
  the acceptance command.
- Both reviewer probes pass unchanged (never edited).
- No features, dependency changes, cloud calls, or preview
  promotion. `docs/BUILD_STATUS.md` and `docs/reviews/`
  untouched. Real sign-in stays dev-gated.

## 7. Acceptance criteria

- **AC-P1 (characterization):** the scheduling analysis and
  the instrumentation data are recorded; the verdict
  (queueing delay vs backend defect) is stated with the
  admission/attempt evidence behind it.
- **AC-P2 (diagnostics):** on expiry, the wait reports
  admissions and attempts (start → end), remaining faults,
  job state/version/log, and the progress timeline.
- **AC-P3 (progress-aware wait):** the 10 s stall bound and
  60 s overall bound are implemented as in §4; poll
  interval unchanged; no new sleeps.
- **AC-P4 (failing tests):** both named tests pass in the
  default broad command (repeated Linux runs) and
  individually; all 14 `wait_for_parked` callers pass.
- **AC-P5 (preservation):** both reviewer probes pass
  unchanged; exact attempt counts, cancellation, and
  cross-root fairness tests all green; queue + ingestion
  suites green.
- **AC-P6 (test-focused):** no production behavior change;
  only `#[cfg(test)]` instrumentation in jobs.rs.
- **AC-P7 (regression coverage):** the contention test
  proves parking completes despite queued roots ahead,
  via progress synchronization, and is stable on repeat.
- **AC-P8 (delivery):** checksummed bundle ZIP from
  reviewer tip `4ffdd431` with this design, tests,
  before/after evidence, reproduction commands, and
  Windows instructions for the default broad command
  (expected Windows inventory 403). Reviewer docs
  untouched. No full-Windows-acceptance claim from Linux
  results.

## 8. Out of scope

- Any change to production scheduling (worker count,
  queue discipline, budgets, retry delay) — none is
  indicated; FIFO + requeue-at-back is the accepted
  fairness design.
- Serializing the test suite or the fault tests.
- The desktop/provider library binaries' Windows runs
  (they never executed in the review's broad invocations;
  the Windows rerun must execute the full command and
  record the observed inventory).
- Any change to the API-key flow, the ChatGPT sign-in
  prototype, the frozen Windows pilot, or any provider
  op/contract.
- No global status document edited (Antigravity-owned
  files untouched, including the reviewer's BUILD_STATUS
  entry, which stays as the reviewer wrote it).
