# Windows ingestion test reliability — design

Branch: `muse/windows-ingestion-test-reliability`
Base: lifecycle repair 4 tip `6f34f905579aec5b7703578fb8abb57f0eae5a97`
Date: 2026-11-11 (Linux evidence only — **WINDOWS RERUN OUTSTANDING**)

## 1. The Windows finding (preserved verbatim)

From the supplied Windows evidence package
`signin-lifecycle4-windows-review-2026-11-11.zip` (SHA-256
`ecb478c7205eb3c93886a3def05f6239a55c012766db2d65767fa5eb79ccc717`,
manifest verified: all 18 files match; runtime tip `6f34f9055`):

- Broad Windows library run
  (`node scripts/cargo.mjs test --locked --workspace --lib`):
  API 4/4, then core 297/298, failing
  `ingestion::tests::docx_happy_path_publishes_document_with_section_provenance`
  at `crates/core/src/ingestion.rs:893`:
  `assertion left == right failed, left: Queued, right: Completed`.
- The unchanged test then passed alone in 6.91 seconds
  (`--exact`, 1 passed, 297 filtered out).
- Separate remaining-crate runs passed API 4/4, desktop 1/1,
  providers 63/63. Observed unique Windows library inventory is
  366 (not the Linux 367 — one lib test is platform-gated).

This is an unresolved broad-run failure, not proof of a
production defect and not a clean suite. The DOCX cause was not
established on Windows; ingestion source is unchanged across
lifecycle repairs 1–4.

Focused Windows acceptance from the same package (recorded, not
re-litigated here): `chatgpt_signin` 32/32, `provider_dispatch`
38/38 (Windows-expected count), UI 342/342, Connect AI browser
1/1, TypeScript/contracts/builds passed; real ChatGPT sign-in
stays dev-gated (confirmed in source).

## 2. Characterization: LIFO starvation + one shared worker + a fixed 5s poll

The ingestion lifecycle: `ingest-part` with `finish=true` moves
the job Paused → Queued, commits, and calls `wake_export_worker`.
One process-global worker thread (`PROCESS_WORKER` in
`crates/core/src/jobs.rs`) pops a storage from a pending queue,
claims and executes that storage's queued jobs one at a time
(`claim_export` → `execute_ingestion` → publication guard in
`memory_write` → `complete_ingestion`), then pops the next
storage. The test helper `wait_for_terminal` polls the job state
200 × 25 ms = 5 s, then returns the last-read job; the caller
asserts `Completed`.

Three facts combine into the failure:

1. **The pending queue is LIFO.** `WorkerQueue.pending` is a
   `Vec`, served with `pop()`. A storage woken early is served
   only after every storage woken later. The dedup check keeps an
   already-pending root in place; a re-wake does not promote it.
   While new wakes keep arriving — the normal state of a broad
   run — an early entry is starved indefinitely.
2. **One worker thread serves every test storage in the
   process.** Each ingestion test owns a fresh temp storage root;
   all of them share the single worker, which drains storages
   serially (SQLite open + claim + extract + publish per job).
3. **The poll window is fixed at 5 s with no diagnostics.** On
   the Windows broad run (many parallel test threads, slower
   debug-build I/O), the DOCX test's storage was still buried in
   the pending queue when its window expired, so the job was
   still `Queued` — never claimed, never failed. The failure
   message carried no state history, so the run could not say
   whether the job was slow, starved, or dropped.

Ruled out by the evidence: worker death (sibling ingestion tests
completed after the failure in the same process); lease expiry
(the state was `Queued`, not `Paused`); claim-and-fail (the state
would be `Failed`); test-thread starvation (a sleep-based poll
is wall-clock — descheduling lengthens, never shortens, it).

Linux cannot reproduce the scheduling condition on this VM: the
ingestion module passed 20/20 three times in ~1.3 s, and the
full core lib suite passed 299/299 twice with 16 test threads in
~4.7 s. The VM drains the worker too fast, with too few CPUs,
for the pending queue to starve anyone. The deterministic proof
for the scheduling fix is therefore a queue-discipline
regression (AC-K3), and the Windows rerun is the behavioral
confirmation (AC-K9).

Open, unproven, and NOT fixed here: the worker loop's
`while let Ok(Some(lease))` silently drops a storage if
`claim_export` returns `Err`, with no retry until the next wake
for that root. No evidence shows this happened on Windows, and
forcing a transient claim failure deterministically is not
possible without new hooks; the diagnostic wait (AC-K4) makes
such a stall visible — a job stuck `Queued` with zero state
transitions past the bound — instead of a bare timeout.

## 3. Fix design

### 3a. Narrow backend fix: FIFO pending queue

Replace the `Vec` with a `VecDeque` and serve with `pop_front`,
so storages are served in wake order and no entry can be starved
by later arrivals. Extract the queue discipline into small
`WorkerQueue` methods (`push` with the existing dedup,
`take_next`, `mark_running`/`mark_idle`) so the order is
unit-testable without threads or timing; the wake-up protocol
(pending + running under one mutex, no lost wake-up) is
unchanged. In production a single storage root makes LIFO vs
FIFO unobservable (the dedup keeps at most one entry per root);
nothing documents or depends on LIFO order.

### 3b. Test helper: bounded condition wait with diagnostics

`wait_for_terminal` polls the job state every 25 ms until a
terminal state, up to a 60 s bound, recording every state
transition (elapsed time, state, version, checkpoint, log). On
success it returns the job exactly as today — every caller
assertion is unchanged. On timeout it panics with the full
transition history and the final job record, so a future failure
names what the job did (slow? starved? stuck?) instead of only
its last state. The wait observes only: it never re-wakes the
worker (that would mask a lost wake-up), never forces
completion, and never weakens an assertion. The 60 s bound is a
safety net for a contended CI worker, not the fix — the FIFO
change is what makes the wait short in practice.

### 3c. Deterministic probe: burst contention test

A new test wakes the worker for eight fresh storages in a tight
burst (one ingestion job each) and asserts every job reaches
`Completed` through the diagnostic wait. It exercises the shared
worker under exactly the broad-run condition — many pending
storages at once — with no timing assumption beyond the bounded
wait. It passes on Linux; on Windows it is the deterministic
probe for the scheduling condition, alongside the queue
regression.

### 3d. Audit

All ten `wait_for_terminal` call sites (nine tests: TXT/MD, PDF,
and DOCX happy paths; DOCX and PDF refusals; duplicate import;
invalid UTF-8; stale-lease resume; sweep; concurrent begin) keep
their assertions; the helper change preserves cancellation,
restart, concurrent-upload, provenance, and exactly-once
publication coverage. Other modules' sleeps (ideas,
opportunities, forge, collider, backup, readiness, snapshots,
provider dispatch, SOMNIA) drive different machinery and are out
of scope.

## 4. Acceptance criteria

- AC-K1 (record): the Windows failure and isolated pass are
  preserved verbatim in §1 and in the handoff, with the evidence
  package hash and the reviewer's counts.
- AC-K2 (characterization): §2 records the LIFO + single-worker
  + 5 s poll mechanism, what was ruled out, the Linux
  non-reproduction experiments, and the unproven claim-error
  exposure as a documented limit.
- AC-K3 (backend fix + regression): the pending queue serves in
  wake order. Deterministic unit tests on the extracted queue
  discipline (FIFO order, dedup without reorder, wake-up
  semantics) fail against the LIFO implementation and pass after
  the switch. No other worker behavior changes.
- AC-K4 (diagnostic wait): `wait_for_terminal` waits on the
  terminal condition with a 60 s bound, records state
  transitions, and panics with the full history on timeout. All
  existing caller assertions are intact; no re-wake, no forced
  completion, no weakened assertion.
- AC-K5 (probe): the burst contention test exists, passes on
  Linux, and is documented as the deterministic Windows probe.
- AC-K6 (audit): all ten call sites audited (§3d); all 20
  ingestion tests green; cancellation, restart, concurrent,
  provenance, and exactly-once coverage preserved.
- AC-K7 (scope): no new ingestion formats, authentication
  features, cloud calls, dependency upgrades, preview promotion,
  or unrelated changes; no DTO or contract change; real ChatGPT
  sign-in stays dev-gated.
- AC-K8 (verification): focused ingestion tests, relevant
  browser workflows (connect-ai, DOCX, PDF, TXT/MD, Ask), and
  the broad Linux library suite run sequentially
  (`--test-threads=1`) plus the parallel broad run. Linux
  evidence is labelled Linux; Windows acceptance stays open
  until the rerun.
- AC-K9 (delivery): checksummed Git bundle ZIP containing the
  design, tests, evidence, reproduction commands, and an exact
  Windows rerun checklist.

## 5. Out of scope (unchanged)

- The claim-error drop (§2, open item) — documented, not fixed.
- Any worker redesign (per-storage workers, priorities,
  work-stealing), any production scheduling policy change beyond
  FIFO order, any e2e change.
- Any change to the API-key flow, the ChatGPT sign-in prototype,
  the frozen Windows pilot, or any global status document.
