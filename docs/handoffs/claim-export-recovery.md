# Claim-export recovery — handoff

Branch: `muse/claim-export-recovery`
Base: Windows-verified tip `3e86af6b40883a357ed6a66ff5231e72c2365913`
Tip: the handoff commit on this branch (exact SHA in the delivery
message); last code commit `9a80b165f`
Date: 2026-11-12 (Linux evidence only — **WINDOWS RERUN OUTSTANDING**)
Design (committed before implementation):
`docs/handoffs/claim-export-recovery-design.md`
(`4f0ba5573`, AC-M1..M10)

## 1. The disclosed issue (preserved)

The ingestion-reliability slice disclosed, and its Windows
review confirmed as not closed, that the export worker loop's
`while let Ok(Some(lease)) = storage.claim_export()` silently
drops a storage root when the claim returns `Err`: the root was
already popped from the pending queue, so its Queued jobs stay
stranded until some later user action happens to wake the
worker for that root again. The Windows review found no
evidence of the path firing; this slice closes the limitation
rather than the observation.

The accepted Windows evidence at the base tip is preserved
unchanged: broad lib 370/370 with the DOCX happy-path passing
inside the broad run, queue 3/3, burst 1/1, lifecycle 32/32,
provider dispatch 38/38, UI 342/342, browser 23/23 across five
specs (evidence package
`ingestion-reliability-windows-reviewed-2026-11-11.zip`).

## 2. What was changed

Commits on the branch (base `3e86af6b4`):

- `4f0ba5573` — design + acceptance criteria (AC-M1..M10)
  before code.
- `dbf213923` — reproduction: the fault-injection seam
  (`claim_for_worker` wrapper + per-root fault table + parked
  signal, behavior-neutral) and six fault tests, all failing
  on the old loop (§3).
- `9a80b165f` — the fix (below).

Files changed vs base (4 files): the design doc,
`crates/core/src/jobs.rs`, `crates/core/src/ingestion.rs`, and
this handoff. No DTO, contract, UI, or schema change; no new
thread, timer, API, or status surface; real sign-in stays
dev-gated.

### The fix (short form)

1. **Failure-counted queue entries.** `WorkerQueue` entries
   are now `PendingRoot { storage, claim_failures }`. A re-wake
   of a pending root resets its count (a fresh user action
   earns a fresh budget); a successful claim resets it too.
2. **Error classification.** Only `StorageUnavailable` is
   retried. Any other code — a corrupt catalog above all —
   parks the root immediately with no retry budget consumed.
   The distinction uses the `ErrorCode` already carried on
   `CommandError`; no new error plumbing.
3. **Bounded retry.** A `StorageUnavailable` failure
   re-queues the root at the BACK of the queue (FIFO
   preserved; other roots are served before the retry), up to
   `MAX_CLAIM_FAILURES = 3` consecutive failures per arming,
   then the root is parked. Before re-claiming a root with
   prior failures the worker sleeps 100 ms — a kindness to a
   sick store, not a correctness mechanism; the attempt bound
   is what prevents a busy loop.
4. **Parking is observable within existing contracts.** A
   parked root leaves its jobs Queued (visible via `list_jobs`
   with their existing log lines); the worker goes on serving
   other roots and exits when the queue drains; any later user
   wake (any job command, a new enqueue, Retry) re-arms the
   root with a fresh budget. No job row is ever mutated
   without a lease, and recovery never auto-resumes a Paused
   job.
5. **Execution is unchanged.** Recovery re-runs `claim_export`
   and the existing claim → execute → `JobPublishGuard` path,
   so the staged-hash gate, lease fencing, exactly-once dedup,
   privacy, and project checks all still apply. There is no
   separate recovery publish path.
6. **Test-only seam.** `claim_for_worker` consults a
   `#[cfg(test)]` fault table keyed by storage-root path and
   records parking in a `#[cfg(test)]` set. The seam is
   compiled out of shipped builds; direct `claim_export`
   callers are unaffected.

## 3. Before / after evidence

Reproduction run on the old loop (seam + tests only, `dbf213923`):

```
test result: FAILED. 21 passed; 6 failed
- transient_claim_failure_recovers_without_another_wake
- failing_root_does_not_block_other_roots
- persistent_claim_failure_is_bounded_and_parks
- corrupt_claim_failure_parks_immediately_and_recovers_on_wake
- cancel_during_claim_retries_stays_cancelled
- recovery_publishes_once_with_declared_privacy_and_project
```

Illustrative failure (the stranded state, verbatim):

```
root "/tmp/.tmpDJGFK8/storage" was never parked within 10s;
job state Queued v2 log="Upload complete; queued for import"
```

After the fix, same tests:

```
test result: ok. 27 passed; 0 failed (ingestion module)
test result: ok. 7 passed; 0 failed (queue unit tests)
```

## 4. Evidence (Linux, this VM)

- Rust lib, parallel broad run: **381 passed**, 0 failed
  (371 baseline + 4 queue tests + 6 fault tests).
- Rust lib, sequential broad run (`--test-threads=1`):
  **381 passed**, 0 failed.
- Full workspace `--tests` run: **619 passed, 4 failed** —
  the 4 failures are the pre-existing `semantic_workflow`
  live-Ollama tests (no local model on this VM), failing in
  0.05 s as before; no other failure in any test binary.
- Integration `chatgpt_signin`: **32/32 passed** (regression).
- Integration `provider_dispatch`: **39/39 passed** (regression).
- Focused queue unit tests: **7/7 passed** (3 updated + 4 new).
- Focused ingestion module: **27/27 passed** (21 + 6 fault tests).
- TypeScript: `tsc --noEmit` 0 errors.
- Contracts: `node scripts/contracts.mjs --check` exit 0, 0 drift
  (no DTO change).
- Docs validation: **21/21 passed**.
- UI suite (`pnpm test:ui`): **342/342 passed**.
- Node suite (`pnpm test`): **39/40** — the one failure is the
  pre-existing MSVC installer test. Not a regression.
- e2e regressions (lease-run, one spec per invocation):
  connect-ai **1/1**, DOCX **7/7**, PDF **5/5**, TXT/MD **9/9**, Ask **1/1**.
- Warnings: one, pre-existing — `crates/core/src/mcp.rs:715`
  (`preserved_env_entries` never used). No warning from this slice.
- `rustfmt` applied to `jobs.rs` and `ingestion.rs` only (never
  to a crate root).

Linux evidence is Linux evidence: it does not close the Windows
rerun. The Windows verdict comes from §7.

## 5. Acceptance criteria — how each was met

- **AC-M1 (transient recovery):**
  `transient_claim_failure_recovers_without_another_wake` —
  2 injected `StorageUnavailable` failures, then the job
  Completes with no further user action, exactly one document,
  declared privacy kept.
- **AC-M2 (isolation):**
  `failing_root_does_not_block_other_roots` — root A
  persistently faulted, root B healthy; B Completes while A is
  still Queued; A parks after exactly 3 attempts.
- **AC-M3 (bounded retry):**
  `persistent_claim_failure_is_bounded_and_parks` — 1,000,000
  armed faults, exactly 3 consumed, then parked; the job stays
  Queued; the test finishes promptly under a bounded wait.
- **AC-M4 (non-retryable):**
  `corrupt_claim_failure_parks_immediately_and_recovers_on_wake`
  — `CorruptStorage` consumes exactly 1 attempt; after the
  fault clears, a plain job-command wake Completes the job.
- **AC-M5 (cancellation/shutdown):**
  `cancel_during_claim_retries_stays_cancelled` — Cancel
  succeeds mid-retry, the job stays Cancelled after the fault
  clears and a wake, nothing is published; `worker_queue_drains_after_parking`
  proves the queue reports idle after parking and a later wake
  restarts the worker, and the re-service halves of the
  persistent/corrupt tests prove the worker serves new work
  after parking.
- **AC-M6 (exactly-once/permissions):**
  `recovery_publishes_once_with_declared_privacy_and_project`
  — transient faults on a project-scoped private import yield
  exactly one document carrying the declared privacy and
  project, through the unchanged execution path.
- **AC-M7 (FIFO/wake-up):** the three existing queue tests pass
  on the new entry type, plus requeue-to-back, re-wake reset,
  requeue-dedup-against-fresh-wake, and drain-after-parking
  tests (7/7).
- **AC-M8 (contracts/scope):** no job-row mutation without a
  lease (the worker only re-runs `claim_export`); Paused jobs
  are never auto-resumed (`recover_jobs` semantics untouched);
  no DTO, contract, UI, or schema change; no new features,
  authentication changes, dependency upgrades, cloud calls, or
  preview promotion; real sign-in stays dev-gated.
- **AC-M9 (verification):** focused tests, broad suite
  parallel and sequential, full `--tests` run, named
  regressions, and browser regressions (§4). Linux and Windows
  evidence are kept distinct throughout; the base tip's
  accepted Windows evidence is preserved in §1.
- **AC-M10 (delivery):** this handoff + the checksummed bundle
  ZIP (see §8). All existing assertions intact (21/21 prior
  ingestion tests, 3/3 prior queue tests, all other suites).

## 6. Known limits

- Parking has no dedicated status surface: the observable
  state is the jobs left Queued plus the next wake re-arming
  the root. A dedicated worker-status API would be a new
  feature and is out of scope.
- The 100 ms retry delay is fixed, not exponential; with a
  3-attempt budget the total added latency is at most 200 ms
  per arming.
- `StorageUnavailable` covers both transient locks and
  persistent permission problems; the budget — not the code —
  is what bounds the persistent case. A permission problem
  still needs the user to fix permissions; the next wake then
  recovers.
- The fault-injection seam exists only in test builds. A real
  claim failure in production follows the same code path but
  cannot be forced on demand outside tests.
- No change to `chatgpt-signin-status`, the UI, the API-key
  flow, or any global status document.

## 7. Windows rerun checklist

1. Apply the bundle ZIP on Windows in a separate review
   worktree; confirm the tip stated in the delivery message
   (handoff commit; last code commit `9a80b165f`) and a clean
   tree. Preserve the running pilot and existing data.
2. Run the broad command:
   `node scripts/cargo.mjs test --locked --workspace --lib`
   Expect: all green. Windows library inventory is 370
   baseline + 10 new = 380 expected (one lib test is
   platform-gated — record the observed inventory, do not
   assume the Linux 381).
3. Run the deterministic probes in isolation:
   `node scripts/cargo.mjs test --locked -p anansi-core --lib jobs::tests::worker_queue`
   (expect 7/7) and
   `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::`
   (expect 27/27, including the six fault tests by name:
   transient, failing-root isolation, bounded persistent,
   corrupt-immediate, cancel-during-retries,
   publishes-once-with-privacy-and-project).
4. Run the focused suites: `--test chatgpt_signin` (expect
   32/32), `--test provider_dispatch` (expect 38/38, the
   Unix-only symlink test excluded), `tsc --noEmit` (0),
   contracts `--check` (0 drift), docs validation (21/21), UI
   suite (342/342).
5. Run the e2e regressions: connect-ai 1/1, DOCX 7/7, PDF 5/5,
   TXT/MD 9/9, Ask 1/1.
6. If a fault test fails, preserve the failure output in full:
   the parked-wait panic names the job's state, version, and
   log, and the fault-count assertion names the attempts
   consumed. Do not report a clean suite on an isolated
   rerun alone.
7. **Do not** enable the sign-in button or attempt a live
   Responses turn: eligibility is still unverified (spike
   handoff §5) — this slice changes no eligibility fact.

## 8. Reproduction commands (Linux, this VM)

- Fault tests (fail on the old loop, pass after the fix):
  `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::`
- Queue discipline:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib worker_queue`
- Broad library suite, parallel and sequential:
  `node scripts/cargo.mjs test --locked --workspace --lib`
  `node scripts/cargo.mjs test --locked --workspace --lib -- --test-threads=1`
- Full test binaries:
  `node scripts/cargo.mjs test --locked --workspace --tests`
  (expect the 4 pre-existing live-Ollama failures only)

## 9. Out of scope (unchanged)

- `claim_export`, `recover_jobs`, execution, publication, and
  cancellation semantics — all untouched.
- Any worker-status API, job state, log-line mutation, or
  observability surface.
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
  untouched); proposed updates, if any, belong in this handoff only.
