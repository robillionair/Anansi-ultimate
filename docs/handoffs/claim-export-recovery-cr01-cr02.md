# Claim-export recovery CR-01/CR-02 — handoff

Branch: `muse/claim-export-recovery-cr01-cr02`
Base: reviewer tip `acf2d7f97689fac61723d85b37ad17896f1833b8`
on `codex/review-claim-export-recovery` (one commit on delivered
tip `d287f6767`: the reviewer probe + review docs, no production
change)
Tip: the handoff commit on this branch (exact SHA in the delivery
message); last code commit `c72009c61`
Date: 2026-11-12 (Linux evidence only — **WINDOWS RERUN OUTSTANDING**;
no full Windows acceptance is claimed from Linux results)
Design (committed before implementation):
`docs/handoffs/claim-export-recovery-cr01-cr02-design.md`
(`dd059490a`, AC-N1..N11)

## 1. Reviewer findings (preserved)

Windows review package
`claim-export-recovery-windows-review-2026-11-12.zip`
(SHA-256 `fb3ff20f7cbc57a8956a60b20632e076817feda054caac085653130dd6f2bf1e`,
9,562 bytes; manifest verified, all 5 files match; delivery ZIP
hash matched `da84e845…`; reviewer bundle head exactly
`acf2d7f9`; required base exactly `d287f6767`). Verdict: changes
required. The wider 380-test suite and browser acceptance were
NOT run on the delivery; the focused findings needed repair
first. The preceding 370-test accepted Windows checkpoint is
unchanged.

**CR-01 — status polling re-arms exhausted recovery.** The
probe injects persistent claim errors, waits for three attempts
and parking, issues one ordinary `job_command("list")`, and
checks the remaining fault budget. Windows result: remaining
faults change from 999997 (three attempts used) to 999994 (six
used). Documents polls that request automatically every 4 s, so
automatic observation grants fresh budgets repeatedly, including
after non-retryable parking.

**CR-02 — parked state has no actionable production
diagnostic.** After exhaustion the public response still reports
`Queued` with log `Upload complete; queued for import`, and no
failure reason, attempt-exhaustion indication, or recovery
instruction. Production `record_claim_parked` is a no-op.

## 2. What was changed

Commits on the branch (base `acf2d7f9`):

- `dd059490a` — design + acceptance criteria (AC-N1..N11)
  before code.
- `c72009c61` — the fix (below), tests, regenerated contracts,
  UI affordances.

Files changed vs base (10): the design doc, `jobs.rs`,
`ingestion.rs`, `readiness.rs`, `backup_commands.rs`, the three
generated contract files, `Jobs.tsx`, `Documents.tsx`.
`docs/BUILD_STATUS.md` and `docs/reviews/` are untouched. The
reviewer probe is byte-identical (verified by diff).

### CR-01: wake kinds + exhaustion record

- `WakeKind::{Deliberate, Passive}` is now explicit at every
  wake call site. `job_command("list")` and Pause/Cancel wake
  passively; enqueue, Retry/Resume/Recover, ingest-part finish,
  backup ops, the scheduler tick, and startup (`for_host`) wake
  deliberately.
- `WorkerQueue` carries a production exhaustion record per
  root (reason + attempts consumed). A passive wake on an
  exhausted root is a no-op and never resets a pending root's
  failure count. A deliberate wake clears the record and
  resets the count. Parking records exhaustion under the
  queue lock, sharing the wake-up mutex with pending-queue
  decisions.
- A successful `control_job` Retry/Resume/Recover clears the
  root's exhaustion after commit, so the control response is
  accurate; failed controls clear nothing.
- The test-only parked set stays separate from the production
  record: the probe clears the test signal before polling and
  still asserts no fresh attempts — clearing the signal must
  never re-arm the root.

### CR-02: diagnostic + explicit recovery path

- `Job` gains `recoveryDiagnostic?: JobRecoveryDiagnostic`
  (`reason: "storageUnavailable" | "corruptStorage"`,
  `attempts`, fixed `message`), attached at read time in
  `list_jobs` and `control_job` for Queued jobs whose own
  storage root is exhausted. The durable job row is never
  written by parking or by diagnostic reads (version, log,
  checkpoint, progress byte-identical — asserted). Messages
  are fixed strings (equality-asserted; no paths, no SQLite
  text, no record contents). Responses without a diagnostic
  serialize byte-identically.
- New `JobAction::Recover`, valid from Queued only, through
  the existing version-fenced `control_job` path (wrong
  version conflicts; other states refused). State stays
  Queued, checkpoint/progress untouched, log line "Export
  recovery requested explicitly". On success it clears the
  exhaustion and the following deliberate wake re-arms the
  worker. Execution still requires a fresh claim + the
  staged-hash gate + the publish guard: no unleased mutation,
  no permission bypass.
- Contracts regenerated (Job field, new diagnostic types,
  `JobAction` gains `"recover"`); `--check` passes with zero
  drift after regeneration.
- `Jobs.tsx` and `Documents.tsx` show the diagnostic message
  with the attempt count and offer a "Retry recovery" button
  when the diagnostic is present. Polling behavior unchanged.

### What did not change

Budgets (3 / 1), FIFO order, requeue-at-back, the 100 ms retry
delay, `claim_export`, `recover_jobs`, execution, publication,
and cancellation semantics, and the fault-injection seam. No
new polling, auto-retry, or worker-status API. No dependency
upgrades, cloud calls, or preview promotion. Real sign-in
stays dev-gated.

## 3. Before / after evidence

Reviewer probe, unchanged, on the reviewer tip (Linux):

```
status response after exhausted recovery: {...,"state":"Queued",...}
(no recoveryDiagnostic; log "Upload complete; queued for import")
assertion `left == right` failed: read-only job polling re-armed
an exhausted claim budget
  left: 999995
 right: 999997
test result: FAILED. 0 passed; 1 failed; 313 filtered out
```

(Linux consumed 2 fresh attempts before the fence completed;
Windows consumed 3. Either way the assertion fails: polling
re-armed the root.)

Reviewer probe, unchanged, after the fix (Linux):

```
status response after exhausted recovery: {...,"recoveryDiagnostic":
{"attempts":3,"message":"Recovery stopped because storage was
unavailable. Check the storage folder and permissions, then
choose Retry recovery.","reason":"storageUnavailable"},
...,"state":"Queued","version":2,...}
test result: ok. 1 passed; 0 failed; 317 filtered out
```

The durable row is untouched (version 2, original log) while
the public response carries the diagnostic.

## 4. Evidence (Linux, this VM)

- Rust lib, parallel broad run: **396 passed**, 0 failed
  (381 + 1 probe + 4 queue + 10 ingestion).
- Rust lib, sequential broad run (`--test-threads=1`):
  **396 passed**, 0 failed.
- Full workspace `--tests` run: **634 passed, 4 failed** —
  the 4 failures are the pre-existing `semantic_workflow`
  live-Ollama tests (no local model on this VM), the same
  four names as before; no other failure in any test binary.
- Integration `chatgpt_signin`: **32/32 passed** (regression).
- Integration `provider_dispatch`: **39/39 passed** (regression).
- Focused queue unit tests: **11/11 passed** (7 updated + 4 new).
- Focused ingestion module: **38/38 passed** (27 + probe + 10 new).
- TypeScript: `tsc --noEmit` 0 errors.
- Contracts: regenerated, then `--check` exit 0, 0 drift.
- Docs validation: **21/21 passed**.
- UI suite (`pnpm test:ui`): **342/342 passed**.
- Node suite (`pnpm test`): **39/40** — the one failure is the
  pre-existing MSVC installer test. Not a regression.
- e2e regressions (lease-run, one spec per invocation):
  connect-ai **1/1**, DOCX **7/7**, PDF **5/5**, TXT/MD **9/9**, Ask **1/1**.
- Warnings: one, pre-existing — `crates/core/src/mcp.rs:715`
  (`preserved_env_entries` never used). No warning from this slice.
- `rustfmt` applied to `jobs.rs` and `ingestion.rs` only (never
  to a crate root); the diff contains no stray reflows.

Linux evidence is Linux evidence: it does not close the Windows
rerun. The Windows verdict comes from §7.

## 5. Acceptance criteria — how each was met

- **AC-N1 (CR-01 probe):** the reviewer probe passes
  unchanged (§3). Verified byte-identical to the reviewer
  version.
- **AC-N2 (CR-01 polling):** `repeated_polls_do_not_rearm_...`
  (3 polls, Unavailable, 3 total) and
  `polls_do_not_rearm_non_retryable_...` (3 polls, Corrupt, 1
  total), each with a healthy fence root completing.
- **AC-N3 (CR-01 deliberate paths):**
  `new_work_rearms_exhausted_root_and_serves_stranded_job`
  (both jobs complete), `deliberate_wake_rearms_exhausted_root`
  (startup-equivalent API), and Recover recovery in the
  persistent/corrupt tests; FIFO preserved (queue tests 11/11
  + isolation behavior unchanged).
- **AC-N4 (CR-01 queue discipline):** four new queue tests
  (passive-ignores-exhausted, passive-keeps-count,
  passive-adds-healthy, exhaustion round-trip) + seven
  updated tests with explicit kinds.
- **AC-N5 (CR-02 diagnostic):**
  `diagnostic_shown_for_exhausted_queued_job_without_row_mutation`
  (reason/attempts/exact message, no root path in text),
  `diagnostic_for_corrupt_exhaustion_names_restore`,
  `diagnostic_absent_for_other_states_and_roots` (Paused job
  on the exhausted root + jobs on a healthy root carry none).
- **AC-N6 (CR-02 no unleased mutation):** the diagnostic test
  asserts the durable row (version 2, checkpoint, progress,
  log) is identical after parking; the diagnostic is computed
  at read time only.
- **AC-N7 (CR-02 explicit recovery):**
  `recover_action_recovers_and_clears_diagnostic` (diagnostic
  present → Recover → gone → job completes) and
  `recover_action_is_version_fenced_and_state_bounded` (wrong
  version conflicts, Paused refused).
- **AC-N8 (preservation):**
  `cancel_of_diagnosed_job_stays_cancelled_with_nothing_published`
  + the existing cancel-during-retries test (polling never
  resurrects); exactly-once/privacy/project via the unchanged
  fault tests; all existing assertions intact (the
  persistent/corrupt tests changed ONLY their re-arm
  mechanism from polling to the explicit Recover action —
  the old mechanism IS the CR-01 bug).
- **AC-N9 (UI):** diagnostic + "Retry recovery" in Jobs and
  Documents; UI suite 342/342; contracts regenerated, 0 drift.
  Limit: the button rendering is type-checked, not
  browser-exercised — the browser path has no fault injection
  (§6).
- **AC-N10 (verification):** §4. Linux and Windows evidence
  kept distinct throughout; no full-Windows-acceptance claim.
- **AC-N11 (delivery):** this handoff + the checksummed bundle
  ZIP (§8). Reviewer docs untouched; no unrelated changes.

## 6. Known limits

- The exhaustion record is in-memory only: a process restart
  clears it and startup recovery proceeds. Persisting it
  across restarts is out of scope.
- The diagnostic UI (message + Retry recovery button) is
  covered by `tsc` and the API-level tests, not by a browser
  test: the browser path cannot inject claim faults. The e2e
  battery proves no regression in the Documents/Jobs flows.
- The Recover log line says "Export recovery requested
  explicitly" for ingestion jobs too, matching the control
  path's existing "Export … explicitly" wording.
- A Recover call on a healthy Queued job cannot be exercised
  deterministically (a Queued job on a healthy root is served
  immediately); Recover on Queued is proven through the
  exhausted path, and on a healthy root it only clears an
  absent record and wakes deliberately.
- No change to `chatgpt-signin-status`, the API-key flow, or
  any global status document.

## 7. Windows instructions

1. Apply the bundle ZIP on Windows in a separate review
   worktree on top of reviewer tip `acf2d7f9`
   (`codex/review-claim-export-recovery`); confirm the tip
   stated in the delivery message (handoff commit; last code
   commit `c72009c61`) and a clean tree. Preserve the running
   pilot and existing data.
2. Run the reviewer probe unchanged, exactly:
   `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::tests::reviewer_status_poll_does_not_rearm_exhausted_claim_budget -- --exact --nocapture`
   Expect: 1 passed. The printed status response should carry
   `recoveryDiagnostic` with reason `storageUnavailable`,
   attempts 3, and the fixed message.
3. Run the focused probes:
   `node scripts/cargo.mjs test --locked -p anansi-core --lib jobs::tests::worker_queue`
   (expect 11/11) and
   `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::`
   (expect 38/38, including the probe and the ten new
   CR-01/CR-02 tests).
4. Run the broad command:
   `node scripts/cargo.mjs test --locked --workspace --lib`
   Expect: all green. Windows library inventory is 381 at the
   reviewer tip + 14 new = 395 expected (one lib test is
   platform-gated — record the observed inventory, do not
   assume the Linux 396).
5. Run the focused suites: `--test chatgpt_signin` (expect
   32/32), `--test provider_dispatch` (expect 38/38, the
   Unix-only symlink test excluded), `tsc --noEmit` (0),
   contracts `--check` (0 drift), docs validation (21/21), UI
   suite (342/342).
6. Run the e2e regressions: connect-ai 1/1, DOCX 7/7, PDF 5/5,
   TXT/MD 9/9, Ask 1/1.
7. If a fault test fails, preserve the failure output in full:
   the parked-wait panic names the job's state, version, and
   log, and the fault-count assertion names the attempts
   consumed. Do not report a clean suite on an isolated
   rerun alone.
8. **Do not** enable the sign-in button or attempt a live
   Responses turn: eligibility is still unverified (spike
   handoff §5) — this slice changes no eligibility fact.

## 8. Reproduction commands (Linux, this VM)

- Reviewer probe (fails on the reviewer tip, passes after):
  `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::tests::reviewer_status_poll_does_not_rearm_exhausted_claim_budget -- --exact --nocapture`
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

## 9. Out of scope (unchanged)

- `claim_export`, `recover_jobs`, execution, publication, and
  cancellation semantics — all untouched.
- Persisting exhaustion across restarts; any new polling,
  auto-retry, or worker-status API beyond the per-job
  diagnostic.
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
