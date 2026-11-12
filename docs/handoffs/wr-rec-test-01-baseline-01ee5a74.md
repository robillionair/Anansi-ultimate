# WR-REC-TEST-01 on baseline 01ee5a74 — handoff

Branch: `muse/claim-recovery-wr-rec-test-01-01ee5a74`
Base: `01ee5a74d9b4866352525284e963ae17be5716fa`
(remote `master`, `HEAD`, `codex/development-status-2026-11-12`,
`codex/windows-preview-integration` — fetched from
`https://github.com/robillionair/Anansi-ultimate.git` and
verified 2026-11-12; `origin/master` now resolves to it)
Tip: the handoff commit on this branch (exact SHA in the
delivery message); last code commit `ff6d52472`
Date: 2026-11-12 (Linux evidence only — **WINDOWS RERUN
OUTSTANDING**; no full Windows acceptance is claimed from
Linux results)
Design (committed before implementation, on the original
branch): `docs/handoffs/wr-rec-test-01-design.md`
(AC-P1..P8). Its "Base:" line names reviewer tip `4ffdd431`,
where the design was authored; this handoff records the
actual base of THIS branch.

## 1. Baseline

`01ee5a74` is reviewer tip `4ffdd431` plus three
docs/publish commits (`8d6a542e6`, `ad0ff6fec`,
`01ee5a74`, all `[skip ci]`): README development-status
update, BUILD_STATUS/REQUIREMENTS_MATRIX/ROADMAP updates,
published Windows verification transcripts under
`docs/evidence/windows-recovery-2026-11-12/`, and three
plan documents. Verified by diff: 34 files, +2272/-6, no
`.rs`/`.ts` source file touched (the only JSON file is the
evidence MANIFEST). The review document
`docs/reviews/windows-claim-recovery-cr01b-review.md` is
unchanged from `4ffdd431`.

The baseline's own words state the task. README: "The
latest recovery fixes pass focused Windows checks, but two
fault tests still time out in the broad Windows library
run." BUILD_STATUS (2026-11-12 entry): "Two
default-concurrency broad Windows runs fail recovery
fixture parking waits ... Both repeated failures use the
10-second wait at ingestion.rs:818 and pass in isolation"
and "no clean 403-test Windows acceptance or promotion is
claimed."

The WR-REC-TEST-01 fix is ABSENT at the baseline,
verified directly: `wait_for_parked` at `01ee5a74` still
carries the old total-bound panic (`was never parked
within {bound:?}`), and `claim_admissions` /
`parked_timeout` occur 0 times in the baseline tree.

## 2. What this branch is

A rebase of the delivered WR-REC-TEST-01 repair onto the
new master baseline — no new design, no scope expansion.
The three content commits from branch
`muse/claim-recovery-wr-rec-test-01` (tip `a4d12424`,
delivered 2026-11-12 as
`anansi-claim-recovery-wr-rec-test-01-2026-11-12.zip`,
SHA-256 `c177cbcc…`) were cherry-picked unchanged onto
`01ee5a74`:

- `0b27e1365` — design + AC-P1..P8 (from `77e64a93c`)
- `017a895dd` — test-only admission/attempt instrumentation
  + expiry diagnostics (from `43fe6e4ad`)
- `ff6d52472` — progress-aware parking wait + contention
  and stall-bound tests (from `cebbec665`)

Verified by diff: `crates/` and the design doc on this
branch are byte-identical to the delivered branch. The
only tree differences from the delivered branch are the
baseline's own published docs/evidence/plans and this
handoff (which replaces the original branch's
`docs/handoffs/wr-rec-test-01.md`, written against base
`4ffdd431`). The original branch and its delivery ZIP are
untouched.

## 3. The fix (summary; full account in the design doc)

**Characterization (done before the fix, on the original
branch).** One process-global worker serves every fixture
FIFO; a fault root needs 3 admissions across 2
requeue-at-back cycles, each cycle waiting for every other
pending root to drain, plus two serialized 100 ms kindness
delays. A temporary scaling probe measured time-to-park
linear in contending fault roots (N=1: 0.20s, N=10: 1.88s,
N=20: 3.74s, N=30: 5.61s; ≈187 ms/root; every root 3
admissions/3 attempts/parked). Verdict: queueing delay, no
backend defect — the park path is deterministic given 3
attempts, failures vary by run and vanish in isolation,
and the Windows core suite takes 170.79s for the same 335
tests that take ≈12s on Linux.

**Fix (test-only).** `#[cfg(test)]` per-root admission and
attempt counters in `jobs.rs` (compiled out of shipped
builds; zero production behavior change) plus a
progress-aware `wait_for_parked` in `ingestion.rs`: the
same 10s value, now a STALL bound (time without admission
or attempt progress, not total time), a 60s overall safety
bound mirroring `wait_for_terminal`, the 25ms poll
unchanged. On expiry the panic reports admissions and
attempts (start → end), remaining faults, job
state/version/log, and the progress history — the
diagnostic that distinguishes queueing delay (admissions
< 3) from a parking defect (3 admissions, 3 attempts,
never parked). No timeout increase, no added sleeps, no
weakened assertions, no serial execution: the default
broad command remains the acceptance command.

**Tests.** `parking_completes_despite_queued_fault_roots_ahead`
(12 fault roots ahead; exact 3/3 admissions/attempts and
exact fault consumption) and
`parking_wait_fails_fast_when_root_is_never_admitted`
(`#[should_panic]`; never-queued root fails in ≈10s with
0→0 admissions/attempts — bounded failure detection).
Both reviewer probes and both failing tests' bodies are
byte-identical (verified by diff on the original branch;
the cherry-pick preserves them).

**Preserved.** The verified CR-01b recovery implementation
is untouched: production code (worker loop, queue, park
logic, budgets, FIFO order, retry delay, fault-seam
behavior) unchanged; `wait_for_terminal` untouched; the
ChatGPT sign-in control remains disabled and dev-gated; no
real credentials, cloud calls, new features, dependency
changes, or preview promotion. `docs/BUILD_STATUS.md`,
`docs/reviews/`, README, and the published evidence are
untouched on this branch (the branch's only doc additions
are the cherry-picked design doc and this handoff).

## 4. Before / after evidence

Before — at the baseline, from the published transcripts
in the baseline tree itself
(`docs/evidence/windows-recovery-2026-11-12/claim-recovery-final-windows-lib-quiet-rerun.txt`):

```
test ingestion::tests::deliberate_wake_rearms_exhausted_root ... FAILED
test ingestion::tests::diagnostic_absent_for_other_states_and_roots ... FAILED
root "E:\\Anansi\\tmp\\.tmpLCeZ2T\\storage" was never parked within 10s; job state Queued v2 log="Upload complete; queued for import"
root "E:\\Anansi\\tmp\\.tmph5oDRG\\storage" was never parked within 10s; job state Queued v2 log="Upload complete; queued for import"
test result: FAILED. 333 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 170.79s
```

The old diagnostic carries no admission/attempt
information. The first broad run failed 4 tests (332.15s);
both named tests pass individually. The Cargo command
stopped at the core failure, so the desktop/provider
library binaries never ran in either broad invocation.

After — on this branch (Linux, §5): both named tests pass
in the default broad command (3 consecutive runs) and
individually; the stall-bound test proves the wait still
fails fast with the full diagnostic when a root is never
admitted. The true before/after verdict for the Windows
broad command comes from the rerun in §6: this slice does
not claim it from Linux results.

## 5. Evidence (Linux, this VM, on this branch)

- Rust lib, default broad command, 3 consecutive runs:
  **406 passed** (4 + 338 + 1 + 63), 0 failed each run.
- Rust lib, sequential broad run (`--test-threads=1`):
  **406 passed**, 0 failed (regression check only — the
  default parallel command is the acceptance command).
- Full workspace `--tests` run: **644 passed, 4 failed** —
  the 4 failures are the pre-existing `semantic_workflow`
  live-Ollama tests (no local model on this VM), the same
  four names as before; no other failure in any test binary.
- Integration `chatgpt_signin`: **32/32 passed** (regression).
- Integration `provider_dispatch`: **39/39 passed** (regression).
- Focused queue tests (`worker_queue` filter): **19/19**.
- Focused ingestion module: **40/40** (38 + 2 new).
- Reviewer probes (`reviewer_` filter): **2/2**.
- The two named failing tests individually: **1/1 each**.
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
- `rustfmt` state unchanged from the cherry-picked
  commits; the branch diff contains no stray reflows.

Linux evidence is Linux evidence: it does not close the
Windows rerun. The Windows verdict comes from §6.

## 6. Windows rerun instructions

1. Apply the bundle ZIP on Windows in a separate review
   worktree on top of baseline `01ee5a74` (remote
   `master`); confirm the tip stated in the delivery
   message (handoff commit; last code commit `ff6d52472`)
   and a clean tree. Preserve the running pilot and
   existing data.
2. Run the exact acceptance command, unchanged, with
   default Rust concurrency (do NOT substitute
   `--test-threads=1`):
   `node scripts/cargo.mjs test --locked --workspace --lib`
   Expect: all green. Windows library inventory is 403
   at the baseline + 2 new = 405 expected (one lib test
   is platform-gated — record the observed inventory, do
   not assume the Linux 406). The command must execute
   ALL FOUR library binaries (API, core, desktop,
   providers): the review's broad invocations stopped at
   the core failure, so this is the first full-inventory
   run at this lineage. If any test fails, preserve the
   failure output in full: the new diagnostic names
   admissions/attempts, which decides whether the cause
   is queueing delay or a parking defect. Do not report
   a clean suite on an isolated rerun alone.
3. Run the focused checks: both reviewer probes
   (`reviewer_` filter, expect 2/2), queue tests
   (`worker_queue` filter, expect 19/19), ingestion
   module (expect 40/40), the two named tests
   individually, `--test chatgpt_signin` (expect 32/32),
   `--test provider_dispatch` (expect 38/38, the
   Unix-only symlink test excluded), `tsc --noEmit` (0),
   contracts `--check` (0 drift), docs validation
   (21/21), UI suite (347/347).
4. Run the e2e regressions: connect-ai 1/1, DOCX 7/7,
   PDF 5/5, TXT/MD 9/9, Ask 1/1, job-recovery 1/1.
5. **Do not** enable the sign-in button or attempt a live
   Responses turn: eligibility is still unverified (spike
   handoff §5) — this slice changes no eligibility fact.

## 7. Reproduction commands (Linux, this VM)

- The two named tests:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib deliberate_wake_rearms_exhausted_root`
  `node scripts/cargo.mjs test --locked -p anansi-core --lib diagnostic_absent_for_other_states_and_roots`
- Reviewer probes:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib reviewer_ -- --nocapture`
- Queue discipline:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib worker_queue`
- Fault + diagnostic tests:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::`
- Broad library suite, default concurrency (acceptance):
  `node scripts/cargo.mjs test --locked --workspace --lib`
- Full test binaries:
  `node scripts/cargo.mjs test --locked --workspace --tests`
  (expect the 4 pre-existing live-Ollama failures only)
- Baseline check (fix absent at `01ee5a74`):
  `git show 01ee5a74:crates/core/src/ingestion.rs | grep -n "was never parked"`

## 8. Acceptance criteria (AC-P1..P8, from the design doc)

- **AC-P1 (characterization):** design §2 + §3 above —
  scheduling analysis, scaling-probe numbers, verdict
  with admission/attempt evidence.
- **AC-P2 (diagnostics):** the wait reports admissions
  and attempts (start → end), remaining faults, job
  state/version/log, and the progress timeline.
- **AC-P3 (progress-aware wait):** 10s stall bound + 60s
  overall bound; poll interval unchanged; no new sleeps.
- **AC-P4 (failing tests):** both named tests pass in the
  default broad command (3 runs) and individually; all
  14 `wait_for_parked` callers pass (ingestion 40/40).
- **AC-P5 (preservation):** both reviewer probes pass
  unchanged; exact attempt counts, cancellation, and
  cross-root fairness tests all green.
- **AC-P6 (test-focused):** no production behavior
  change; only `#[cfg(test)]` instrumentation in jobs.rs.
- **AC-P7 (regression coverage):** contention test +
  stall-bound test, stable on repeat (3 broad runs).
- **AC-P8 (delivery):** this handoff + the checksummed
  bundle ZIP from baseline `01ee5a74`. Reviewer docs
  untouched. No full-Windows-acceptance claim.

## 9. Out of scope (unchanged)

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
  REQUIREMENTS_MATRIX.md, ROADMAP.md, and the published
  evidence); proposed updates, if any, belong in this
  handoff only. Nothing was merged, pushed, or promoted:
  this branch ships as a verified bundle inside a ZIP.
