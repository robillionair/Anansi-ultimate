# WR-REC-TEST-01 follow-up: pre-admission stall clock — design

Branch: `muse/claim-recovery-wr-rec-test-01-pre-admission`
Base: reviewer tip `7cf1068b1fae2ce1410184ad5a3447f6e765cb64`
(one docs-only commit on delivered tip `e32456574`:
BUILD_STATUS +4 lines, review doc
`docs/reviews/windows-wr-rec-test-01-review.md`)
Date: 2026-11-12. Design + acceptance criteria committed
BEFORE implementation, per the standing rule.

## 1. The Windows evidence

The Windows review of the rebased WR-REC-TEST-01 repair
(`e32456574`, ZIP `7eaf0756…`) reports one remaining
default-concurrency failure in the broad library command
(core 336 passed + 1 failed, 204.70 s; Cargo stopped, so
desktop/provider binaries never ran):

```text
thread 'ingestion::tests::deliberate_wake_rearms_exhausted_root' panicked
at crates\core\src\ingestion.rs:888:9:
root "E:\\Anansi\\tmp\\.tmpjV3v4o\\storage" was never parked:
no admission/attempt progress within the 10s stall bound after 10.0073093s;
job state Queued v2 log="Upload complete; queued for import";
admitted 0 -> 0, attempted 0 -> 0, remaining faults 1000000;
progress history (elapsed_ms, admitted, attempted): [(0, 0, 0)]
```

Everything else in the review passed: the other formerly
failing test, the twelve-root contention test, the
never-admitted test (expected panic), the failing test in
isolation (1/1, 7.89 s), both reviewer probes (2/2), queue
tests (19/19). No production recovery defect was
established. The new instrumentation did its job: it
narrowed the finding to exactly one case — a root that was
never admitted before the wait expired.

## 2. Root cause: the stall clock starts before admission

`wait_for_parked` (ingestion.rs) keys its 10 s stall bound
on admission/attempt counter progress only. The counters
cannot distinguish three different situations that all
read as "0 admissions, 0 attempts":

1. **Registered pending.** The root has a queue entry and
   is waiting its turn behind other fixtures' roots in the
   single process-global FIFO worker. Legitimate queueing;
   the wait must not expire.
2. **Never registered.** No wake ever queued the root (a
   lost wake / never-scheduled root). A genuine stall; the
   wait must fail fast.
3. **Admitted, then stuck.** The worker took the root but
   no further admission/attempt progress follows. A
   genuine stall; the wait must fail fast.

In the failing test, the finished upload part wakes the
root (case 1: registered pending), but under broad-run
contention the worker does not admit it within 10 s, so
the stall bound — which starts at wait entry — expires on
a healthy queued root. The first `wait_for_parked` call in
`deliberate_wake_rearms_exhausted_root` is the failure
site: admitted 0→0, attempted 0→0, all 1,000,000 injected
faults remaining proves no claim attempt ever ran. This is
a test-synchronization defect, not a backend defect: no
claim was performed badly, the park path is untouched, and
the test passes in isolation. No probe proves a production
defect, so production stays unchanged.

## 3. Design: explicit test-side worker state + a phase machine

### 3.1 Worker-state mirror (jobs.rs, `#[cfg(test)]` only)

A per-root mirror of the queue state the worker itself
maintains, following the existing `CLAIM_ADMISSIONS` /
`CLAIM_ATTEMPTS` pattern (process-global table keyed by
storage-root path, compiled out of shipped builds):

```rust
struct WorkerTestState { pending: bool, active: bool, ever_registered: bool }
```

Hooks inside the `WorkerQueue` methods, so the mirror
tracks exactly the entries the worker serves:

- `wake`: a pushed or reset entry sets `pending` and
  `ever_registered`. A passive wake that returns early
  (exhausted or active root) changes nothing. A
  deliberate wake of an active root sets `pending` while
  `active` stays set — pending AND active at once, the
  CR-01b fresh-entry case.
- `take_next`: taking an entry clears `pending`, sets
  `active`. An empty queue changes nothing.
- `finish`: clears `active` (drain, park, requeue paths).
- `requeue`: after the push check, sets `pending` (the
  entry is pending again whether freshly pushed or already
  present from a newer wake) with `active` cleared by the
  `finish` call inside `requeue`.

Lock ordering: the mirror mutex is only ever taken while
holding the queue mutex (inside the methods) or alone (by
test reads). It never takes the queue mutex, so no lock
order can invert. Accessor: `worker_test_state(root)`,
defaulting to all-false for an unrecorded root.

### 3.2 Phase machine (ingestion.rs test module)

The wait is factored into a pure phase machine over
sampled observations and elapsed time, driven by the real
polling loop. An observation is one sample: the mirror
state plus the admission/attempt counters. Progress is ANY
change in `(pending, active, ever_registered, admitted,
attempted)` — state transitions count, so admission,
requeue, and registration each reset the stall clock.

Phases, resolved per poll:

- **Pending** (`pending` set, including pending+active):
  no stall expiry. The root is registered work waiting
  its turn. Only the overall bound applies. Pending is
  resolved before active: a fresh entry behind an
  in-flight attempt is legitimate queueing, and the
  overall bound remains the backstop if the in-flight
  attempt hung.
- **Never registered** (not pending, never registered):
  10 s stall expiry with the never-registered reason.
- **Active** (not pending, active set): 10 s stall expiry
  with the active-stall reason.
- **Left queue** (not pending, not active, previously
  registered): 10 s stall expiry with the left-queue
  reason — the root left the queue without parking.

The stall clock therefore starts at the first transition
out of the pending phase (admission), never before it. The
60 s overall safety bound is checked in every phase,
after the stall check, with its existing distinct reason.
The 25 ms poll is unchanged. The parked check still comes
first each poll, and the wait still never re-wakes.

### 3.3 Diagnostic

The expiry panic keeps the existing fields (bound, job
state/version/log, admitted/attempted start→end,
remaining faults) and adds the worker state
(`pending`/`active`/`ever_registered`) plus a history
whose entries now carry the state columns:
`(elapsed_ms, pending, active, admitted, attempted)`.
The Windows 0→0 case would now read as registered-pending
with a full state history instead of an undifferentiated
stall. On success the wait stashes its transition history
in a test-only table keyed by root, so tests can assert
what the wait observed (see §4).

### 3.4 Reason taxonomy

- `never registered with the worker` — no pending entry,
  no admission, 10 s without progress.
- `worker holds the root but made no progress` — active,
  10 s without progress.
- `root left the worker queue without parking` —
  previously registered, neither pending nor active, 10 s
  without progress.
- `the 60s overall safety bound` — unchanged wording,
  any phase.

## 4. Coverage (all deterministic; no wall-clock assertions)

1. **Mirror unit tests** (jobs.rs `worker_queue` tests,
   unique root names per test — the existing tests share
   fake roots `"a"`/`"b"`/`"c"` across parallel tests, so
   mirror assertions must use keys no other test touches):
   wake→pending+ever; take→active, not pending; requeue→
   pending, not active; finish→not active; deliberate wake
   of an active root→pending+active; passive wake of an
   exhausted root→no state change.
2. **Scripted phase-machine tests** (ingestion.rs): the
   machine is driven with scripted observations and
   synthetic elapsed times — no threads, no sleeps:
   - pending-before-admission: 15 s of pending with 0/0
     counters → no expiry; then admission progress → parked.
   - never registered: 10 s unregistered, 0/0 → stall
     expiry, never-registered reason.
   - active stall: admitted, then 10 s frozen active →
     stall expiry, active-stall reason.
   - retry requeue: pending→active→pending→active cycles
     with advancing counters and gaps under 10 s → no
     expiry; then parked.
   - overall expiry: pending forever, 60 s elapsed →
     overall expiry, distinct reason.
3. **Integration test** (ingestion.rs): thirty fault roots
   queue ahead of the observed root (each root ahead costs
   the worker ≥200 ms of forced kindness delay across its
   3 admissions, so the observed root provably waits
   pending before its first admission). Asserts the
   observed root parks with exact 3 admissions / 3
   attempts / exact fault consumption, every root parks,
   AND the wait's stashed history contains a pending
   observation with 0 admissions — the end-to-end proof
   that the wait saw pending-before-admission. The
   deterministic >10 s-pending proof is the scripted
   test; this test proves the wiring (real mirror, real
   queue, real wait).
4. **Updated never-admitted test**: the existing
   `parking_wait_fails_fast_when_root_is_never_admitted`
   keeps its body (never-queued setup) and its fast-fail
   behavior; only the `should_panic` expected string
   changes to the new never-registered reason.

Preserved: both reviewer probes byte-identical; all
recovery assertions (exact attempt counts, diagnostics,
fairness, cancellation); the twelve-root contention test
untouched; `wait_for_terminal` untouched.

## 5. What does not change

- Production behavior: zero. The jobs.rs diff is
  `#[cfg(test)]` hooks and test-only tables/accessors;
  the ingestion.rs diff is confined to the test module.
  No queue, worker, budget, FIFO, fault-seam, or
  publication logic changes. Verified by diff (AC-Q7).
- Bounds: 10 s stall, 60 s overall, 25 ms poll — the
  same values, with the stall bound now applied per
  phase instead of from wait entry. No timeout increase,
  no added sleeps, no weakened assertions, no serial
  execution.
- `docs/BUILD_STATUS.md`, `docs/reviews/`, README, and
  the published evidence are untouched (Antigravity-owned).
  Nothing merged, pushed, or promoted.

## 6. Acceptance criteria

- **AC-Q1 (diagnosis):** an expiry names the worker
  state and carries the state history; the Windows 0→0
  case is distinguishable as registered-pending.
- **AC-Q2 (pending phase):** a registered pending root
  with zero admissions does not expire the stall bound;
  the stall clock starts at the first transition out of
  pending (scripted + integration proof).
- **AC-Q3 (never registered):** a never-registered root
  fails fast at the 10 s stall bound with the
  never-registered reason (scripted + real test).
- **AC-Q4 (active stall):** an admitted root with no
  further progress expires at the 10 s stall bound with
  the active-stall reason (scripted proof; no hook
  exists to freeze the real worker mid-claim, and none
  is added).
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
  unchanged (diff shows only `#[cfg(test)]` additions).
- **AC-Q8 (delivery):** handoff + checksummed bundle
  from reviewer tip `7cf1068b`, Windows rerun
  instructions, no full-Windows-acceptance claim from
  Linux results.

## 7. Known limits

- Pending-dominant phase resolution: a root that is both
  pending and active suspends stall detection until the
  overall bound. The overall bound is the backstop; no
  deeper in-flight instrumentation is added.
- Active-stall and overall-expiry coverage is scripted
  (fake clock), not real-time: freezing the real worker
  mid-claim would need a new hook, and a real 60 s test
  would slow the suite for no additional decision
  coverage. The real-time stall path is proven by the
  never-registered test (10 s, real clock).
- The thirty-root integration test adds roughly 7–10 s
  to the Linux core suite (sleep-dominated; similar on
  Windows).
