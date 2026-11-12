# Claim recovery CR-01b — design

Branch: `muse/claim-recovery-cr01b`
Base: reviewer tip `c54c67b545a19d1d651bb51a5235afd50afb3829`
on `codex/review-claim-recovery-cr01-cr02` (one commit on
delivered tip `b5dc7fcb4`: the in-flight probe + review docs, no
production change)
Date: 2026-11-12
Status: design + acceptance criteria, committed BEFORE implementation.

## 1. Reviewer finding (preserved)

Windows review package
`claim-recovery-cr01b-windows-review-2026-11-12.zip`
(SHA-256 `0ae6f5aaf297ea4bd87d7fe0eae6be1982370b7f9fa9bdbed988f456e2cbb3a5`,
8,555 bytes; manifest verified, all 5 files match; incoming ZIP
hash matched `f2d6dd84…`; reviewer bundle head exactly
`c54c67b5`; runtime tip exactly `b5dc7fcb4`).
Verdict: changes required.

**CR-01b — passive polling resets an active root's failure
budget.** The deterministic queue probe takes an active entry,
models two failures, performs a passive wake while that entry
is out of the queue, then requeues the failed entry. Expected
failure count 2, actual 0 (Windows and Linux alike). Mechanism:
while the worker owns the active entry, the pending queue has
no matching entry, so `wake(Passive)` inserts a fresh
zero-count entry; `requeue(active)` sees that entry and drops
the active counter. Documents' 4 s polling can land in this
window when storage reads/locks take time. The delivered
tests covered pending and exhausted roots but not active ones.

Confirmed good in the same review: the original exhausted-root
probe passes unchanged, and its public response carries
`recoveryDiagnostic` (attempts 3, reason `storageUnavailable`,
fixed sanitized message). The 11 delivered queue tests pass;
only the new probe fails. The full 395-test Windows suite,
builds, UI, and browser acceptance were NOT rerun after this
finding. The reviewer also notes the new diagnostic and Retry
recovery UI need meaningful interaction coverage; the green
browser battery alone does not exercise the new failure UI.

## 2. What was read

- **Queue/worker:** `WorkerQueue` (`wake`, `take_next`,
  `requeue`, exhaustion record), `wake_export_worker` and the
  worker loop (claim → execute → requeue/park/drain), the
  `WakeKind` call-site map from the CR-01/CR-02 design, and
  the fault-injection seam. The loop is single-threaded and
  holds at most one active entry; taking work and deciding to
  stop share the wake-up mutex (no lost wake-up).
- **UI harness:** no DOM test environment exists (no
  jsdom/testing-library); all UI tests use
  `renderToStaticMarkup` (static markup only, no effects, no
  clicks), following the DashboardView presentational pattern.
  The browser app takes a user-typed API endpoint
  (`http://127.0.0.1:<port>/`, validated strictly) and boots via
  `readiness` → `listWorkspaces` → create/open workspace →
  `updateSettings`; `ready` is `onboardingComplete`, and a
  ready workspace auto-navigates to the dashboard. The client
  validates every response against the generated Ajv
  validators, so a mock API must return exact shapes (tsc
  covers `tests/**`, giving compile-time shape safety for a
  typed mock).

## 3. CR-01b fix: track active roots

### Active set

`WorkerQueue` gains `active: Vec<PathBuf>` — roots whose entry
the worker loop currently holds outside the pending queue.
All mutations share the wake-up mutex with pending/exhaustion
decisions.

- `take_next` marks the taken root active (deduplicated).
- A new `finish(root)` unmarks a root the worker stops serving
  without requeueing (drained: claim returned `None`).
- `requeue` unmarks the root, then applies the existing rule:
  drop when a fresh entry is already pending, else append.
- `park_root` unmarks the root, then marks exhaustion only if
  the guard (§4) allows.

Every exit from serving a root clears its active mark: drain,
park, and requeue. The loop's single-threaded discipline keeps
at most one root active; `take_next` returning `None` (worker
exit) implies no active root, because the loop finishes each
root before the next take.

### Wake matrix

| Root state | Passive wake | Deliberate wake |
|---|---|---|
| Idle (none) | Add fresh (count 0), maybe start worker — unchanged healthy behavior | Add fresh, clear exhaustion |
| Pending | No-op (count kept) — CR-01 | Reset count, clear exhaustion |
| **Active** | **No-op (no fresh entry) — THE FIX** | Add a fresh entry (served after the current attempt); clear exhaustion |
| Exhausted | No-op — CR-01 | Clear exhaustion, add fresh |

A passive wake of an active root returns false without
touching the queue, so the in-flight entry's requeue carries
its true failure count. A deliberate wake of an active root
adds a fresh entry: the newer deliberate decision is what the
stale guards (§4) protect.

### Why not merge counts

Merging the active count into a pending entry (taking the
max) would also pass the probe, but it lets a stale in-flight
count override a deliberate fresh budget (0) — exactly the
failure §4 forbids. Tracking active state and distinguishing
wake kinds is the smaller correct rule.

## 4. Stale in-flight failures never override deliberate recovery

Two guards, both evaluated under the queue lock at the moment
of the stale decision:

- **Requeue:** unchanged rule, now meaningful: when a
  deliberate wake added a fresh entry while the root was
  active, `requeue` drops the stale in-flight entry and the
  fresh entry (budget 0) wins.
- **Park:** `mark_exhausted` returns whether it marked. If a
  pending entry exists for the root — which, after §3, can
  only come from a deliberate wake during the active attempt —
  the stale park is suppressed: no exhaustion record, no
  test-only parked signal. The fresh entry is then served
  with its own budget, and its own attempt decides the
  outcome. If no pending entry exists, parking proceeds as
  before.

The test-only parked signal is recorded only when exhaustion
is actually marked, so `wait_for_parked` keeps meaning "the
root really parked".

## 5. What does not change

Budgets (3 / 1), FIFO order, requeue-at-back, the 100 ms retry
delay, `claim_export`, `recover_jobs`, execution, publication,
and cancellation semantics, the exhaustion record and
diagnostic, the Recover action, and the fault-injection seam.
Wake-during-shutdown safety is preserved: taking work and
deciding to stop still share one lock hold, and active
marking happens inside `take_next` under the same hold. No
new polling, auto-retry, or worker-status API. No dependency
upgrades, cloud calls, or preview promotion. Real sign-in
stays dev-gated.

## 6. UI interaction coverage

The exhausted state is unreachable in the real browser (the
fault seam is test-only Rust), and no DOM harness exists, so
coverage comes in two complementary layers:

1. **Shared presentational component.** Extract the recovery
   notice (message + Retry recovery button) from `Jobs.tsx`
   and `Documents.tsx` into `JobRecovery.tsx`, used by both
   views with identical props (`diagnostic`, `state`, `busy`,
   `onRecover`). A new static-markup test file
   (`packages/ui/tests/job-recovery.test.tsx`, added to the
   `test:ui` script) renders it across states: diagnosed +
   Queued shows the exact message, the attempt count, and the
   button; diagnosed + non-Queued shows the message without
   the button; no diagnostic renders nothing.
2. **Mock-API e2e click-through** (`tests/e2e/job-recovery.spec.ts`).
   The spec starts a typed in-process HTTP server on
   127.0.0.1 implementing the boot surface (`readiness`,
   `list`, `create`, `settings`) plus the dashboard's reads
   (empty lists, disabled backup schedule) and the jobs ops.
   `jobs/list` first returns a diagnosed Queued job; the spec
   connects through the real UI, navigates to Jobs, asserts
   the message and button, clicks Retry recovery, asserts the
   recorded control call carries action `recover` with the
   exact job id and observed version, and asserts the UI
   updates to the recovered job (message and button gone)
   after the mock flips state. No real API, no fault
   injection, deterministic on any platform.

## 7. Tests

Both reviewer probes pass UNCHANGED (never edited):
`reviewer_status_poll_does_not_rearm_exhausted_claim_budget`
and `reviewer_worker_queue_passive_wake_preserves_inflight_budget`.

Queue unit tests (new, deterministic, no timing):

- Passive wake of an active root: no entry added, worker-start
  flag false, requeue preserves the count (the probe's
  scenario, plus a second passive wake for good measure).
- Deliberate wake of an active root: fresh entry added; the
  stale requeue drops and the fresh budget wins.
- Stale park suppression: active entry with a fresh deliberate
  entry pending is not marked exhausted and emits no parked
  signal; the fresh entry is served next.
- Park proceeds when no fresh entry is pending (existing
  behavior, now through the guarded path).
- Active mark lifecycle: `take_next` marks, `finish`/`requeue`
  clear; a finished root accepts passive wakes again.
- FIFO + dedup with active roots: a deliberate fresh entry
  keeps its position behind already-pending roots; duplicate
  wakes (either kind) never duplicate entries.

Worker-level fault tests (existing suite, unchanged
assertions): the persistent/corrupt/cancel/diagnostic tests
keep passing; the deliberate-wake and new-work re-arm tests
exercise the fresh-entry path through the real loop.

UI tests: the static `job-recovery.test.tsx` cases (§6.1) and
the mock-API e2e spec (§6.2).

## 8. Acceptance criteria

- **AC-O1 (CR-01b probe):** the in-flight probe passes
  unchanged: passive wake during active processing preserves
  the count-2 budget with exactly one queue entry.
- **AC-O2 (original probe):** the exhausted-root probe still
  passes unchanged.
- **AC-O3 (wake matrix):** passive wakes never reset or add
  across pending, active, and exhausted states; deliberate
  wakes re-arm across all three.
- **AC-O4 (stale safety):** a stale in-flight failure never
  overrides newer deliberate recovery — stale requeue drops,
  stale park is suppressed, and the fresh entry is served
  with its budget.
- **AC-O5 (preservation):** FIFO ordering, deduplication,
  startup recovery, and shutdown wake safety preserved; all
  existing queue/worker assertions intact.
- **AC-O6 (diagnostics + Recover):** the CR-01/CR-02
  diagnostic and Recover action behave identically (existing
  ingestion tests green, diagnostics unchanged).
- **AC-O7 (UI interaction):** the shared recovery notice has
  static rendering coverage across states, and the mock-API
  e2e spec proves the real click → exact control call → UI
  update path.
- **AC-O8 (verification):** focused tests, broad suite
  parallel + sequential, full `--tests`, named regressions,
  browser regressions including the new spec. Linux evidence
  kept distinct from Windows acceptance; no
  full-Windows-acceptance claim from Linux results.
- **AC-O9 (delivery):** checksummed bundle ZIP from reviewer
  tip `c54c67b5` with this design, tests, before/after
  evidence, reproduction commands, and Windows instructions.
  `docs/BUILD_STATUS.md` and the review doc untouched. No
  unrelated features, dependency upgrades, cloud calls, or
  preview promotion.

## 9. Out of scope

- Any change to `claim_export`, `recover_jobs`, execution,
  publication, or cancellation semantics; any change to the
  budgets, FIFO order, or retry delay.
- Persisting exhaustion or active state across restarts.
- Reaching the exhausted state in the real browser (needs the
  test-only fault seam); the e2e spec covers the UI against a
  mock API instead, and the exhausted state itself stays
  covered by the Rust fault tests.
- Any change to the API-key flow, the ChatGPT sign-in
  prototype, the frozen Windows pilot, or any provider
  op/contract.
- No global status document edited (Antigravity-owned files
  untouched, including the reviewer's BUILD_STATUS entry,
  which stays as the reviewer wrote it).
