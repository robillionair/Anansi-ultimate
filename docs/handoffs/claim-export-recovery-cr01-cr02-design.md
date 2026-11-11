# Claim-export recovery CR-01/CR-02 — design

Branch: `muse/claim-export-recovery-cr01-cr02`
Base: reviewer tip `acf2d7f97689fac61723d85b37ad17896f1833b8`
on `codex/review-claim-export-recovery` (one commit on delivered
tip `d287f6767`: the reviewer probe + review docs, no production
change)
Date: 2026-11-12
Status: design + acceptance criteria, committed BEFORE implementation.

## 1. Reviewer findings (preserved)

Windows review package
`claim-export-recovery-windows-review-2026-11-12.zip`
(SHA-256 `fb3ff20f…`, manifest verified, all 5 files match;
delivery ZIP hash matched `da84e845…`; reviewer bundle head
exactly `acf2d7f9`; required base exactly `d287f6767`).
Verdict: changes required. The wider 380-test suite and browser
acceptance were NOT run on the delivery; the focused findings
need repair first. The preceding 370-test accepted Windows
checkpoint is unchanged.

**CR-01 — status polling re-arms exhausted recovery.** The
probe injects persistent claim errors, waits for three attempts
and parking, issues one ordinary `job_command("list")`, and
checks the remaining fault budget. Windows result: remaining
faults change from 999997 (three attempts used) to 999994 (six
used). Documents polls that request automatically every 4 s
(`Documents.tsx` `refreshJobs`, `Jobs.tsx` refresh, both on a
4000 ms interval), so automatic observation grants fresh
budgets repeatedly, including after non-retryable parking.
Per-arming boundedness is insufficient for the actual UI
polling path. Root cause: `job_command("list")` calls
`wake_export_worker`, whose `WorkerQueue::wake` resets claim
failures — and a parked root has no record left, so any wake
re-adds it fresh.

**CR-02 — parked state has no actionable production
diagnostic.** After exhaustion the public response still
reports `Queued` with log `Upload complete; queued for import`,
and no failure reason, attempt-exhaustion indication, or
recovery instruction. Production `record_claim_parked` is a
no-op; the test-only parked set cannot support the production
observability claim. Normal waiting and intervention-required
parking are indistinguishable to the user.

The reviewer probe
(`reviewer_status_poll_does_not_rearm_exhausted_claim_budget`)
must pass UNCHANGED. It uses a healthy second root as a
worker-progress fence: if the read re-armed the failed root,
FIFO attempts that root before completing the fence; corrected
code keeps the root parked and still completes the fence, with
no second parking event required. Note the probe clears the
test-only parked signal before polling — so the production
exhaustion record MUST live separately from that signal (§4).

## 2. What was read

- **Wake call sites:** `job_command` (shared wake after every
  non-artifact command), `ingest-part` finish, `backup_operation`
  (after backup/preview/restore/schedule ops), the backup
  scheduler tick (only when new backup jobs were enqueued),
  `Core::for_host` (startup). Upload ops do not wake.
- **Polling paths:** `Documents.tsx` `refreshJobs` (4 s),
  `Jobs.tsx` refresh (4 s), `documents.ts` upload-outcome
  polling — all via `client.listJobs` → `job_command("list")`.
  `Backups.tsx` reads the backup schedule once on mount (not
  a polling path).
- **Control surface:** `Jobs.tsx` Pause/Resume/Cancel/Retry
  buttons via `client.controlJob(workspaceId, id, version,
  action)`; `Documents.tsx` shows the active job's state,
  checkpoint, and log with Cancel/Refresh buttons, plus
  "Resume upload" for interrupted uploads (a different flow).
- **Contracts:** Rust DTOs → `anansi-contracts` generator →
  `scripts/contracts.mjs` → `packages/contracts` schemas,
  `generated.ts`, validators. `--check` fails on drift.
  `JobAction` is a generated union; `Job` is generated from
  the Rust struct. The only `Job` struct literal is `read()`
  in `jobs.rs`. `read_job` serves internal paths only (backup
  upload, artifact reads, snapshots); the API-facing job reads
  are `list_jobs` and `control_job`.
- **Worker/claim/cancel semantics:** unchanged from the prior
  slice's design (lease fencing, `JobPublishGuard` exactly-once,
  `recover_jobs` Paused semantics, `StorageUnavailable` vs
  `CorruptStorage` from `connection()`).

## 3. CR-01 fix: wake kinds + exhaustion record

### Wake kinds

```rust
pub enum WakeKind { Deliberate, Passive }
```

`wake_export_worker(kind)` and `WorkerQueue::wake(storage,
kind)` take the kind explicitly at every call site:

| Call site | Kind | Why |
|---|---|---|
| `job_command` "export" (enqueue) | Deliberate | Genuinely new work. |
| `job_command` "list" | Passive | Status observation (the polling path). |
| `job_command` "control" Retry/Resume/Recover | Deliberate | Deliberate user action queueing work. |
| `job_command` "control" Pause/Cancel | Passive | No new work; preserves healthy-root wake behavior without re-arming. |
| `job_command` "artifact" | none | Unchanged. |
| `ingest-part` finish | Deliberate | Genuinely new work. |
| `backup_operation`, scheduler tick | Deliberate | User-initiated ops / newly enqueued backup jobs. |
| `Core::for_host` | Deliberate | Startup recovery. |

### Exhaustion record (production state)

`WorkerQueue` gains `exhausted: Vec<ExhaustedRoot>`, sharing
the wake-up mutex with the pending queue so wake/park
decisions are atomic:

```rust
struct ExhaustedRoot { root: PathBuf, reason: JobRecoveryReason, attempts: u32 }
```

- The worker loop records exhaustion under the queue lock
  when it parks a root: reason + attempts consumed in this
  arming (3 for a `StorageUnavailable` budget exhaustion;
  prior failures + 1 for an immediate non-retryable park).
- `wake(Deliberate)` clears the root's exhaustion record and
  adds/resets the pending entry (fresh budget).
- `wake(Passive)` on an exhausted root is a no-op (no add, no
  reset, no worker start for that root). On a pending root it
  is a no-op (never resets the failure count). Otherwise it
  adds the root fresh, exactly as today.
- A successful `control_job` Retry/Resume/Recover also clears
  the root's exhaustion (after commit, before returning), so
  the control response is accurate; the following deliberate
  wake is the backstop. Failed controls (version conflict,
  invalid transition) clear nothing.
- The record is in-memory only: a process restart starts with
  no exhaustion, and startup recovery proceeds (preserved).
  One entry per parked root; roots are storage roots, bounded
  in practice.

### What does NOT change

Budgets (3 / 1), FIFO order, requeue-at-back, the 100 ms retry
delay, claim/execute/guard/cancel semantics, and the
fault-injection seam. The test-only parked set stays separate
(§4).

## 4. The test-only parked signal stays separate

The reviewer probe calls `clear_claim_parked` before polling
and asserts no fresh attempts are consumed. If clearing the
test signal also cleared the production exhaustion record, the
probe would fail on correct code. Therefore:

- `CLAIM_PARKED` + `claim_parked` / `clear_claim_parked` keep
  their current meaning (test-only parking observation for
  `wait_for_parked`).
- The production exhaustion record lives in `WorkerQueue` and
  is never touched by the test helpers.
- The worker loop records BOTH on parking (production record
  under the queue lock; test signal via `record_claim_parked`
  as today).

## 5. CR-02 fix: diagnostic + explicit recovery path

### Diagnostic DTO (production boundary: list/control responses)

```rust
pub struct JobRecoveryDiagnostic {
    pub reason: JobRecoveryReason,  // StorageUnavailable | CorruptStorage
    pub attempts: u32,              // attempts consumed in this arming
    pub message: String,            // fixed sanitized text, no paths/errors
}
pub enum JobRecoveryReason { StorageUnavailable, CorruptStorage }
```

Added to `Job` as:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub recovery_diagnostic: Option<JobRecoveryDiagnostic>,
```

- Attached at read time in `list_jobs` and `control_job` ONLY,
  for `Queued` jobs whose storage root has an exhaustion
  record. Never in `read()` (transactional), never in enqueue
  responses, never for other states, never for another root
  (the check is keyed by the storage's own root path).
- The durable job row is NEVER written by parking or by
  diagnostic reads: version, log, checkpoint, and progress
  are identical before and after exhaustion. The diagnostic is
  computed from in-memory state at read time.
- Messages are FIXED strings per reason (equality-asserted in
  tests — the strongest sanitization proof; no paths, no
  SQLite text, no record contents):
  - Unavailable: "Recovery stopped because storage was
    unavailable. Check the storage folder and permissions,
    then choose Retry recovery."
  - Corrupt: "Recovery stopped because storage could not be
    read. Keep the original files and restore a verified
    backup, then choose Retry recovery."
- Reason mapping at park time: `StorageUnavailable` →
  Unavailable; anything else → Corrupt (the only other code
  `claim_export` can produce is `CorruptStorage` from
  `connection()`; the mapping is documented).
- Contracts regenerated (`pnpm contracts:generate`): the Job
  schema gains the optional field, and the new diagnostic
  types appear in `generated.ts` + validators. Old responses
  without a diagnostic serialize byte-identically
  (`skip_serializing_if`).

### Explicit recovery: `JobAction::Recover`

- New variant, valid from `Queued` ONLY (all other states
  refused with the existing invalid-transition error; wrong
  version conflicts as today). The UI offers it when the
  diagnostic is present; the API accepts it on any Queued job
  (idempotent, harmless on a healthy root).
- Goes through the existing fenced `control_job` path: version
  check + Immediate transaction + version bump + log line
  "Export recovery requested explicitly". State stays Queued;
  checkpoint/progress untouched (the upload state is
  preserved). This is a fenced control mutation, NOT an
  unleased job mutation: no row is written outside the
  version-fenced control transaction, and execution still
  requires a fresh claim + the staged-hash gate + the publish
  guard, so no permission check can be bypassed.
- On success it clears the root's exhaustion and the following
  deliberate wake re-arms the worker. The diagnostic disappears
  on the next read.
- Contract: `JobAction` gains `"recover"`; regenerated.

### UI (minimal, reusing existing patterns)

- `Jobs.tsx`: render the diagnostic message (role="alert")
  and a "Retry recovery" button (`control(job, 'recover')`
  inside the existing `run()` wrapper) when
  `job.recoveryDiagnostic` is present.
- `Documents.tsx`: in the Import progress section, show the
  diagnostic message and a "Retry recovery" button for the
  active job when present, mirroring the existing Cancel
  button (control + refresh).
- Polling behavior unchanged; no new polling, no auto-retry.

## 6. Tests

The reviewer probe passes UNCHANGED (never edited).

Queue unit tests (`jobs.rs`, updated for `WakeKind` + new):

- Existing seven updated to pass an explicit kind (Deliberate
  for their new-work scenarios).
- Passive wake on an exhausted root: no-op (not pending,
  exhaustion kept, worker-start flag false).
- Deliberate wake on an exhausted root: clears exhaustion,
  adds fresh with count 0.
- Passive wake on a pending root: failure count unchanged.
- Deliberate wake on a pending root: count reset.
- `mark_exhausted` / `exhaustion_for` round-trip; re-mark
  replaces the record.

Worker-level fault tests (`ingestion.rs`, real ops + seam):

- Repeated polls do not re-arm persistent failure: park
  (Unavailable) → three list polls → zero fresh attempts; a
  fence root completes.
- Polls do not re-arm non-retryable failure: park (Corrupt, 1
  attempt) → three list polls → still 1 consumed; fence
  completes.
- New work re-arms: park job 1 → clear faults → begin+finish
  job 2 on the same root → BOTH jobs complete.
- Deliberate wake re-arms (startup-equivalent): park → clear
  faults → `wake_export_worker(Deliberate)` → completes.
- Diagnostic shown for an exhausted Queued job: list response
  carries reason/attempts/exact message; the durable row is
  unchanged (version, log, checkpoint asserted).
- Diagnostic for corrupt exhaustion: reason Corrupt, 1
  attempt, exact message.
- Diagnostic absent for other states and roots: a Paused job
  on the exhausted root and jobs on a healthy root carry none.
- Recover action recovers and clears the diagnostic: park →
  diagnostic present → control Recover → diagnostic gone on
  next list → job completes.
- Recover is fenced: wrong version conflicts; Recover on
  Paused/Failed refused; Recover on a healthy Queued job
  succeeds and the job completes normally.
- Cancel of a diagnosed job: Cancelled, nothing published,
  diagnostic gone after the terminal transition.

Existing fault tests: the persistent/corrupt tests' recovery
halves switch from the list-based `wake()` helper to the
explicit Recover control — the old re-arm via polling IS the
CR-01 bug, so keeping it would assert the defect. All other
assertions stay intact. The cancel test keeps the polling
wake (proving polling never resurrects a cancelled job).

## 7. Acceptance criteria

- **AC-N1 (CR-01 probe):** the reviewer probe passes
  unchanged: one list poll after exhaustion consumes zero
  fresh attempts and the fence root completes.
- **AC-N2 (CR-01 polling):** repeated list polls consume zero
  fresh attempts for both persistent `StorageUnavailable`
  (3 total) and `CorruptStorage` (1 total) exhaustion; fence
  roots complete.
- **AC-N3 (CR-01 deliberate paths):** deliberate retry
  (Recover), genuinely new work (new enqueue), and deliberate
  wake (startup-equivalent) each re-arm an exhausted root and
  complete stranded jobs; FIFO fairness preserved.
- **AC-N4 (CR-01 queue discipline):** passive wakes never
  reset a pending root's count and never add an exhausted
  root; deliberate wakes reset/clear; parking records
  exhaustion with reason + attempts.
- **AC-N5 (CR-02 diagnostic):** after exhaustion, list/control
  responses carry the diagnostic (reason, attempts, exact
  fixed message) on Queued jobs of the exhausted root only;
  no paths or filesystem text; no other root's diagnostics;
  no diagnostic on other states.
- **AC-N6 (CR-02 no unleased mutation):** parking and
  diagnostic reads leave the durable job row byte-identical
  (version, log, checkpoint, progress); the diagnostic is
  computed at read time.
- **AC-N7 (CR-02 explicit recovery):** Recover is
  version-fenced, Queued-only, clears exhaustion and the
  diagnostic, and completes the job; harmless on a healthy
  Queued job.
- **AC-N8 (preservation):** cancellation stays responsive
  (diagnosed job cancels cleanly, nothing published);
  exactly-once publication, privacy, and project preserved;
  FIFO/wake-up behavior preserved; all existing assertions
  intact and the reviewer probe unedited.
- **AC-N9 (UI):** Jobs and Documents show the diagnostic and
  offer Retry recovery when present; polling unchanged; UI
  suite green; contracts regenerated with zero drift.
- **AC-N10 (verification):** focused worker/ingestion tests,
  broad suite parallel + sequential, full `--tests`, named
  regressions, browser regressions. Linux evidence kept
  distinct from Windows acceptance; no full-Windows-acceptance
  claim from Linux results.
- **AC-N11 (delivery):** checksummed bundle ZIP from reviewer
  tip `acf2d7f9` with this design, tests, before/after
  evidence, reproduction commands, and Windows instructions.
  `docs/BUILD_STATUS.md` and the review doc untouched. No
  unrelated features, dependency upgrades, cloud calls, or
  preview promotion.

## 8. Out of scope

- Any change to `claim_export`, `recover_jobs`, execution,
  publication, or cancellation semantics; any change to the
  budgets (3 / 1), FIFO order, or retry delay.
- Persisting exhaustion across restarts (startup recovery
  covers it), any new polling, auto-retry, or worker-status
  API beyond the per-job diagnostic.
- Any change to the API-key flow, the ChatGPT sign-in
  prototype, the frozen Windows pilot, or any provider
  op/contract.
- No global status document edited (Antigravity-owned files
  untouched, including the reviewer's BUILD_STATUS entry,
  which stays as the reviewer wrote it).
