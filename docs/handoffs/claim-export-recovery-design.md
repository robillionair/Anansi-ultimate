# Claim-export recovery — design

Branch: `muse/claim-export-recovery`
Base: Windows-verified tip `3e86af6b40883a357ed6a66ff5231e72c2365913`
Date: 2026-11-11
Status: design + acceptance criteria, committed BEFORE implementation.

## 1. The disclosed issue

In `crates/core/src/jobs.rs`, the process-global export worker
serves one storage root at a time:

```rust
while let Ok(Some(lease)) = storage.claim_export() {
    let result = match &lease.job.selection { ... execute ... };
    if result.is_err() {
        let _ = storage.fail_export(&lease);
    }
}
```

`claim_export()` returns `Err` when any step of the claim fails:
`recover_jobs()`, opening the catalog connection, beginning the
Immediate transaction, selecting the oldest Queued job, minting
the owner id, the claim UPDATE, reading the job back, or
committing. On `Err`, the `while let` exits silently. The root
was already popped from the pending queue, so it is dropped
with no retry: its Queued jobs stay stranded until some later
user action happens to wake the worker for that root again. No
record of the failure exists anywhere — the jobs simply never
move.

This was disclosed as a known limit in the ingestion-reliability
slice (handoff §6) and its Windows review found no evidence of
the path firing. It is not closed until this slice lands.

## 2. What was read

- **Worker loop** (`jobs.rs`, `Core::wake_export_worker`): one
  thread per process; `PROCESS_WORKER: Mutex<WorkerQueue>`;
  `WorkerQueue` serves storages FIFO (`VecDeque`, from the
  ingestion-reliability slice); taking work and deciding to
  stop share the wake-up mutex, so a wake committed during
  shutdown cannot be lost. Wake callers: `job_command`
  (non-artifact), `ingest-part` finish, backup/restore paths,
  readiness — all user-action-driven.
- **`claim_export`** (`jobs.rs`): calls `recover_jobs()` first
  (Running jobs with an expired lease become Paused — resume
  stays explicit), then claims the oldest Queued job across all
  workspaces in one Immediate transaction, setting Running +
  owner + 120 s lease. Ingestion jobs keep checkpoint/progress.
- **Cancellation** (`control_job`): Pause/Resume/Cancel/Retry
  are version-fenced state transitions on the API thread; they
  never touch the worker. Cancelling clears owner/lease, so an
  in-flight execution cannot publish afterwards:
  `complete_ingestion`/`fail_ingestion` require
  `state='Running' AND owner AND lease valid`, and
  `memory_write`'s `JobPublishGuard` refuses with
  `RevisionConflict` when the job was cancelled or the lease
  expired. A cancelled job is not Queued, so a claim retry can
  never resurrect it.
- **Execution guards** (`ingestion.rs`): `execute_ingestion`
  re-validates the staged file (length + hash) as the
  authoritative gate, extracts per kind under bounded budgets,
  and publishes through `memory_write` with the
  `JobPublishGuard` (job id + owner + dedup title/text-hash)
  for exactly-once publication. Privacy and project come from
  the durable selection. Recovery must reuse this path
  unchanged — there must be no separate recovery publish path.
- **Storage errors** (`storage.rs`, `readiness.rs`):
  `connection()` returns two distinct codes that `claim_export`
  propagates via `?`:
  - `StorageUnavailable` (`unavailable()`): locks/busy (5 s
    busy timeout), permission checks, I/O, missing marker —
    transient or fixable without data loss.
  - `CorruptStorage` (`corrupt()`): the catalog cannot be read
    (probe open, identity check, `quick_check`, migration) —
    the message says to keep the original files and restore a
    verified backup; no reset is performed. This requires user
    action and must not be retried by the worker.
  Every other failure inside `claim_export` itself is mapped to
  `unavailable()`.
- **Test helpers** (`ingestion.rs` tests): `setup()` temp
  storages, `begin`/`part` via the real ops, `wait_for_terminal`
  (bounded condition wait with transition diagnostics, observes
  only), `stage_for_test`, `read_job`, `list_documents`. The
  queue unit tests in `jobs.rs` construct `Storage` by root
  path directly.

## 3. Error classification

The worker distinguishes errors by the `ErrorCode` already
carried on `CommandError` — no new error plumbing:

| Code | Meaning | Worker behavior |
|---|---|---|
| `StorageUnavailable` | Possibly transient (lock, busy, I/O, permissions) | Bounded retry: re-queue at the back, up to `MAX_CLAIM_FAILURES` consecutive failures per arming, then park. |
| `CorruptStorage` and any other code | Requires user action (restore a backup, fix the catalog) | Park immediately: no retry budget consumed, no spin. |

Parking means: the root leaves the pending queue; its jobs
remain Queued (observable via `list_jobs` with their existing
log lines); the worker goes on serving other roots and exits
when the queue drains. Any later user wake (any job command, a
new enqueue, Retry) re-arms the root with a fresh budget —
recovery after the user fixes the underlying problem needs no
new mechanism. No job row is ever mutated without a lease, and
recovery never auto-resumes a Paused job.

## 4. The fix (smallest solution)

State lives in the existing queue entry; no new thread, timer,
API, DTO, schema change, or status surface.

```rust
const MAX_CLAIM_FAILURES: u32 = 3;
const CLAIM_RETRY_DELAY: Duration = Duration::from_millis(100);

struct PendingRoot { storage: Storage, claim_failures: u32 }
```

- `WorkerQueue::wake` deduplicates by root as before; a re-wake
  of a pending root resets its failure count to 0 (a fresh user
  action earns a fresh budget). Returns whether the caller must
  start the worker thread, unchanged.
- `WorkerQueue::take_next` returns the `PendingRoot`; the idle
  decision (`running = false` on empty) is unchanged and still
  shares the wake-up mutex with taking work.
- New `WorkerQueue::requeue`: pushes a failed root to the BACK
  of the queue (FIFO preserved; other roots are served before
  the retry). If the root is already pending — a fresh wake
  arrived while it was being served — the requeue is dropped
  and the pending entry (fresh budget) wins.
- Worker loop: on `Ok(Some(lease))` the failure count resets
  to 0 and execution proceeds exactly as today. On `Ok(None)`
  the root is drained. On `Err`:
  - non-retryable code → park (drop the root), no requeue;
  - `StorageUnavailable` → increment; at `MAX_CLAIM_FAILURES`
    park, otherwise requeue at the back and serve the next
    root.
- Before claiming a root that carries prior failures, the
  worker sleeps `CLAIM_RETRY_DELAY` (100 ms, fixed). The delay
  is a kindness to a sick store, not a correctness mechanism:
  the bound on attempts is what prevents a busy loop, and the
  requeue-at-back is what keeps other roots progressing. The
  sleep never blocks cancellation (API thread) and delays a
  newly woken root by at most 100 ms.
- A successful claim resets the budget, so a root that
  alternates success and failure is never parked for stale
  failures.

Why 3 attempts: each attempt already runs through SQLite with
a 5 s busy timeout, so three attempts span real contention;
more would only delay parking a persistently sick root.

## 5. Deterministic fault injection (test-only seam)

Production code cannot be made to fail `claim_export` on
demand without depending on filesystem races, so the worker
loop calls a one-line wrapper, `claim_for_worker`, which in
test builds consults a fault table before delegating to
`claim_export`:

- `#[cfg(test)]` static table keyed by storage-root path:
  remaining forced failures + the `ErrorCode` to return.
  Scoped by root, so parallel tests using separate temp
  directories cannot interfere.
- `#[cfg(test)]` helpers: arm faults, read remaining faults,
  clear faults. Consuming a fault returns
  `Err(CommandError::new(code, ...))` without touching the
  store — the worker's classification and budget logic is
  exercised exactly as in production.
- `#[cfg(test)]` parked set: the worker records a root when it
  parks it, so tests can wait on the parking condition
  (bounded wait, no timing assertions) instead of polling
  global worker state, which parallel tests would make flaky.
- The seam is `#[cfg(test)]` throughout and is compiled out of
  shipped builds: zero production behavior change beyond the
  retry logic itself. Direct `claim_export` callers (tests
  that drive the queue manually) are unaffected — only the
  worker loop goes through the wrapper.

## 6. Tests

Queue unit tests (`jobs.rs`, updated + new, deterministic):

- Existing three (FIFO order, dedup keeps first position,
  restart after idle) updated to the new entry type.
- Requeue goes to the back: A fails, B pending → B served
  before A's retry.
- Re-wake resets the failure count of a pending root.
- Requeue drops when a fresh wake already re-pended the root.
- Park drains: after the budget is exhausted without requeue,
  the queue reports idle and a later wake restarts the worker.

Worker-level fault tests (`ingestion.rs` tests, real ops +
fault seam, no timing assertions):

- **Transient recovery:** arm 2 `StorageUnavailable` faults,
  finish the upload, assert the job Completes with no further
  user action, exactly one document, declared privacy kept.
- **Other roots progress:** root A armed with persistent
  faults, root B healthy, both woken; B Completes while A is
  still Queued; A is then observed parked with exactly 3
  attempts consumed.
- **Bounded persistent failure:** arm 1,000,000 faults; assert
  the parked signal fires and exactly 3 were consumed; the job
  stays Queued; no spin (the test finishes promptly under a
  bounded wait).
- **Non-retryable parks immediately:** arm `CorruptStorage`
  faults; assert the parked signal fires with exactly 1
  consumed; then clear the fault, wake via a plain job command,
  and assert the job Completes (user-action recovery).
- **Cancellation responsive:** arm persistent faults, wake,
  Cancel via `control_job` (asserts promptly), assert the job
  is Cancelled and — after clearing the fault and waking —
  nothing is published and the job stays Cancelled.
- **Exactly-once + permissions:** transient faults on a
  project-scoped private import; assert exactly one document
  carrying the declared privacy and project, and the job's
  artifact. Execution is the unchanged claim→execute→guard
  path, so no permission check can be bypassed by a retry.
- **Re-arm after parking:** a parked root whose fault clears
  completes on the next user wake (covered by the
  non-retryable test's second half and asserted for the
  retryable case too).

## 7. Acceptance criteria

- **AC-M1 (transient recovery):** a claim failure that clears
  recovers with no further user action — the job completes and
  exactly one document is published.
- **AC-M2 (isolation):** a persistently failing root never
  blocks other roots; a healthy root completes while the
  failing root is being retried and parked.
- **AC-M3 (bounded retry):** persistent `StorageUnavailable`
  failures consume exactly `MAX_CLAIM_FAILURES` attempts and
  then park; the worker never busy-loops.
- **AC-M4 (non-retryable):** a `CorruptStorage` claim failure
  parks immediately (one attempt); a later user wake after the
  fault clears recovers the job.
- **AC-M5 (cancellation/shutdown):** cancellation stays
  responsive during claim retries and a cancelled job is never
  published by recovery; the worker thread terminates when the
  queue drains (queue unit test + re-service after parking).
- **AC-M6 (exactly-once/permissions):** recovery never
  publishes twice and never bypasses a permission check: the
  published document carries the durable selection's privacy
  and project, through the unchanged execution path.
- **AC-M7 (FIFO/wake-up):** FIFO ordering and wake-up behavior
  are preserved: the three existing queue tests pass on the
  new entry type, plus requeue-to-back, re-wake reset, and
  requeue-dedup tests.
- **AC-M8 (contracts/scope):** no job-row mutation without a
  lease; Paused jobs are never auto-resumed; no DTO, contract,
  UI, or schema change; no new features, authentication
  changes, dependency upgrades, cloud calls, or preview
  promotion; real sign-in stays dev-gated.
- **AC-M9 (verification):** focused worker/ingestion tests,
  the broad library suite (parallel and sequential), the
  provider-dispatch and chatgpt_signin regressions,
  TypeScript/contracts/docs/UI/node suites, and the browser
  regressions (connect-ai, DOCX, PDF, TXT/MD, Ask). Linux
  evidence is kept distinct from Windows acceptance, and the
  accepted Windows evidence at the base tip is preserved.
- **AC-M10 (delivery):** a checksummed bundle ZIP containing
  this design, the tests, before/after evidence, reproduction
  commands, and an exact Windows rerun checklist. All existing
  assertions stay intact.

## 8. Out of scope

- Any change to `claim_export`, `recover_jobs`, execution,
  publication, or cancellation semantics.
- Any new worker-status API, job state, log line mutation, or
  observability surface (the Queued state + idle worker + next
  wake is the observable state, within existing contracts).
- Retrying execution failures (unchanged: `fail_export`).
- The backup/restore/ask/provider worker threads (different
  machinery, untouched).
- Any change to the API-key flow, the ChatGPT sign-in
  prototype, the frozen Windows pilot, or any global status
  document (Antigravity-owned files untouched).
