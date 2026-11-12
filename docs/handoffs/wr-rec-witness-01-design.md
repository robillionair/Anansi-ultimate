# WR-REC-WITNESS-01: admission gate for the witness test — design

Branch: `muse/claim-recovery-wr-rec-witness-01-gate`
Base: reviewer tip `399bb0af76fe46838b69248c6d3f222afc168cfb`
(one docs-only commit on delivered tip `606450202`:
BUILD_STATUS +4 lines, review doc
`docs/reviews/windows-wr-rec-pre-admission-review.md`)
Date: 2026-11-12. Design + acceptance criteria committed
BEFORE implementation, per the standing rule.

## 1. The Windows evidence

The Windows review of the pre-admission repair
(`606450202`, ZIP `6b0adf62…`) reports the original
timeout cases FIXED: `deliberate_wake_rearms_exhausted_root`
and `diagnostic_absent_for_other_states_and_roots` both
passed in the default-concurrency broad run, both
reviewer probes passed, the scripted phase-machine tests
and the never-registered expected-panic case passed. Core
345 passed + 1 failed (271.42 s); desktop 1/1 and
providers 63/63 were then run separately (not a clean
aggregate invocation); queue/mirror 22/22. The sole
failure is the new integration fixture:

```text
thread 'ingestion::tests::parking_wait_survives_pending_before_first_admission'
panicked at crates\core\src\ingestion.rs:2471:9:
wait observed pending before first admission:
[(0, false, true, 2, 1), (77, false, true, 3, 2)]
```

Tuple fields: elapsed ms, pending, active, admissions,
attempts. The observed root was already active with 2
admissions when sampling began. It ultimately parked with
the asserted exact admissions, attempts, and fault
consumption; only the later witness assertion failed.
No bound expired; no production failure was demonstrated.

## 2. Root cause: a relative-speed assumption, not a bound

The witness test created and woke thirty roots
sequentially, then began observing the last one. Its
comment assumed fixture setup takes milliseconds, so the
other roots' forced kindness delays would guarantee a
pending-before-first-admission observation. That
assumption does not hold on Windows: thirty begin+part
sequences with real SQLite/filesystem I/O are slow
enough that the worker serves the observed root — twice —
before the wait's first poll. The wait itself behaved
correctly (pending phase, exact counts, parked). Only
the test's observation guarantee was unsound. More roots
or a delay cannot provide the required synchronization;
the review prescribes an explicit gate/handshake, and
this design follows it exactly.

## 3. Design: root-scoped admission gate + witness handshake

### 3.1 The gate (jobs.rs, `#[cfg(test)]` only)

A test-only table of gated roots plus three helpers:

- `gate_admissions(root)` — hold the root pending
  (idempotent).
- `release_admissions(root)` — release the hold
  (idempotent).
- `admissions_gated(root)` — read the hold.

`take_next` gains a `#[cfg(test)]` selection branch:
serve the first UNGATED entry (`position` + `VecDeque::
remove`), leaving gated entries in place and in order.
When every entry is gated (or the queue is empty) it
returns `None` and the worker goes idle, exactly as for
an empty queue; the gate release re-wakes it (see §3.2).
With no gates armed, the first entry is ungated, so
selection is `remove(0)` — identical to `pop_front`.
In shipped builds the branch is compiled out and
`take_next` is exactly the previous code: zero
production scheduling change.

Never blocks under the queue mutex: the gate check is a
pure scan under the existing lock hold. The gates table
is a plain mutex, locked only for the scan; no condvar,
no sleep, no wait of any kind happens while the queue
mutex is held. Lock order is queue → gates; the gate
helpers take only the gates mutex, so no order can
invert. Fairness: ungated roots keep FIFO order among
themselves and gated entries keep their positions; after
release a root is served in its original position
relative to the remaining entries. Other-root
concurrency is preserved: the worker keeps serving
ungated roots while a root is gated — the gate never
stops the worker unless every entry is gated.

### 3.2 The guard (ingestion.rs test module)

`AdmissionGate<'a>` holds a borrow of the watched
`Core` and the root path. `arm` gates the root. `Drop`
releases the gate AND deliberately wakes the root, so a
worker that went idle restarts (a deliberate wake of a
pending root keeps its entry and restarts a stopped
worker). Cleanup runs on every exit path: normal drop,
assertion failure, panic, and witness timeout — `Drop`
runs during unwinding, and `thread::scope` joins the
observer only after the closure unwinds, by which time
the guard has already dropped, released, and woken.
`Drop` cannot panic: the wake returns early on a storage
error and all locks use the `unwrap_or_else` pattern.
A leaked gate cannot stall the suite: the guard is
scoped to the test, and gates are keyed by the test's
unique temp root.

### 3.3 The handshake (ingestion.rs test module)

The wait loop publishes each poll's history to the
stashed-history table and notifies a static condvar
(`PARKING_WITNESS`) — a live observation feed with no
sleeps. The test synchronizes on it:

1. Arm the gate (before any wake).
2. Finish the upload part: the wake registers the root
   synchronously (entry added inline); the gate holds
   admission. Assert synchronously, before the observer
   starts: mirror `pending`, admissions 0.
3. Spawn the observer (`wait_for_parked`) on a scoped
   thread (`std::thread::scope`, borrowing the fixture).
4. Condvar-wait (30 s bound, predicate on the stashed
   history) until the observer has recorded pending=true
   with zero admissions. On timeout the test fails
   loudly; the guard releases during unwinding and the
   scope joins the observer.
5. Drop the gate (release + deliberate wake), join the
   observer, assert exact 3 admissions / 3 attempts /
   exact fault consumption, the stashed-history witness
   assertion (identical in form to the original), and
   parked; clear faults/parked.

The 30-root fixture loop is REMOVED from this test — it
was the unsound mechanism. The twelve-root contention
test remains the multi-cycle contention coverage,
untouched.

### 3.4 What the wait loop gains

One hook per poll: publish the current history + notify.
The stashed history is therefore current after every
poll (the existing success-path write stays, now
idempotent). The phase machine, bounds (10 s stall,
60 s overall), 25 ms poll, parked-first check, and
never-re-wake behavior are all unchanged.

## 4. Coverage

- **Gate unit tests** (jobs.rs, unique root keys — the
  process-global-table lesson from the mirror tests):
  `take_next` skips a gated front entry and serves the
  next root, the gated entry keeps its place and is
  served after release (with a wake restarting the idle
  worker); gating is root-scoped (other roots unaffected
  and served while a sibling is gated).
- **Rewritten witness test** (same name, same witness
  assertion, same exact counts): single gated root,
  scoped observer thread, condvar-synchronized witness
  confirmation before release. No filler fixtures, no
  timing assumption.
- **Preserved:** both reviewer probes byte-identical;
  scripted phase-machine tests, twelve-root test,
  never-admitted test, fairness and cancellation tests
  untouched; timeout boundaries unchanged.

## 5. What does not change

- Production behavior: zero. The jobs.rs diff is the
  gate table/helpers, the `#[cfg(test)]` selection
  branch in `take_next`, and unit tests; the
  ingestion.rs diff is confined to the test module
  (guard, witness condvar + hook, rewritten witness
  test). No queue, worker, budget, FIFO, fault-seam,
  publication, or diagnostic logic changes in shipped
  builds. Verified by diff and by a non-test build
  (AC-R7).
- No extra sleeps, no weakened assertions (the witness
  assertion is identical in form, now synchronized),
  no serial execution, no timeout increase.
- `docs/BUILD_STATUS.md`, `docs/reviews/`, README, and
  the published evidence are untouched
  (Antigravity-owned). Nothing merged, pushed, or
  promoted.

## 6. Acceptance criteria

- **AC-R1 (gate):** a gated root stays pending with
  zero admissions while the gate is armed, regardless
  of queue depth or setup speed; ungated roots are
  still served (unit + witness proof).
- **AC-R2 (handshake):** the observer's recorded
  history contains pending=true with zero admissions
  BEFORE the gate releases (condvar-synchronized
  confirmation, no timing assumption).
- **AC-R3 (completion):** after release, the root
  parks with exact 3 admissions / 3 attempts / exact
  fault consumption, and the witness assertion holds.
- **AC-R4 (no blocking under lock):** the gate check
  is a non-blocking scan; no condvar, sleep, or wait
  of any kind happens while the queue mutex is held
  (verified by code inspection).
- **AC-R5 (cleanup):** the gate releases on normal
  completion, assertion failure, panic, and witness
  timeout (Drop guard + `thread::scope` join ordering);
  no residue can stall other tests.
- **AC-R6 (preservation):** reviewer probes
  byte-identical; scripted tests, twelve-root test,
  never-admitted test, fairness/cancellation tests
  untouched and green; bounds unchanged.
- **AC-R7 (production unchanged):** shipped-build
  behavior identical — the gate branch and all helpers
  are `#[cfg(test)]`; `take_next` compiles to exactly
  the previous code in non-test builds.
- **AC-R8 (delivery):** handoff + checksummed bundle
  from reviewer tip `399bb0af`, Windows rerun
  instructions, no full-Windows-acceptance claim from
  Linux results.

## 7. Known limits

- The witness confirmation waits up to 30 s on the
  condvar; the bound exists only to fail loudly if the
  observer never records (a test-harness failure, not
  a product bound). In the passing case the witness
  appears on the observer's first poll.
- The gate is test-only and root-scoped by design; it
  cannot hold a root in a shipped build, and it is
  armed only by the witness test.
- If every queue entry is gated, the worker goes idle
  until a release re-wakes it; in the witness test
  other tests' roots keep the worker busy, and the
  guard's Drop always re-wakes.
