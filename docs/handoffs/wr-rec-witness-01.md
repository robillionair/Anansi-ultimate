# WR-REC-WITNESS-01: admission gate — handoff

Branch: `muse/claim-recovery-wr-rec-witness-01-gate`
Base: reviewer tip `399bb0af76fe46838b69248c6d3f222afc168cfb`
(one docs-only commit on delivered tip `606450202`:
BUILD_STATUS +4 lines, review doc
`docs/reviews/windows-wr-rec-pre-admission-review.md`)
Tip: the handoff commit on this branch (exact SHA in the
delivery message); last code commit `7a2ac7b98`
Date: 2026-11-12 (Linux evidence only — **WINDOWS RERUN
OUTSTANDING**; no full Windows acceptance is claimed from
Linux results)
Design (committed before implementation):
`docs/handoffs/wr-rec-witness-01-design.md` (AC-R1..R8)
Review package:
`wr-rec-pre-admission-6064502-windows-review-2026-11-12.zip`
(SHA-256 `8cf551da…`, 16,975 bytes; all 7 manifest hashes
verified; review bundle head exactly `399bb0af`).

## 1. What the Windows review established

The pre-admission repair's original timeout cases are
FIXED on Windows: `deliberate_wake_rearms_exhausted_root`
and `diagnostic_absent_for_other_states_and_roots` both
passed in the default-concurrency broad run, both
reviewer probes passed, and the scripted phase-machine
tests and never-registered expected-panic case passed.
Core 345 passed + 1 failed (271.42 s); desktop 1/1 and
providers 63/63 were then run separately (not a clean
aggregate invocation); queue/mirror 22/22. The sole
failure is the new integration fixture
WR-REC-WITNESS-01:

```text
thread 'ingestion::tests::parking_wait_survives_pending_before_first_admission'
panicked at crates\core\src\ingestion.rs:2471:9:
wait observed pending before first admission:
[(0, false, true, 2, 1), (77, false, true, 3, 2)]
```

The observed root was already active with 2 admissions
when sampling began. It ultimately parked with the
asserted exact admissions, attempts, and fault
consumption; only the witness assertion failed. No bound
expired; no production failure was demonstrated.

## 2. Root cause

The witness test queued thirty roots ahead and assumed
fixture setup takes milliseconds, so the other roots'
forced delays would guarantee a pending-before-first-
admission observation. On Windows, thirty begin+part
sequences with real SQLite/filesystem I/O are slow
enough that the worker admitted the observed root twice
before the wait's first poll. The wait behaved
correctly; only the test's observation guarantee was
unsound — a relative-speed assumption, not a bound.
More roots or a delay cannot provide the required
synchronization.

## 3. The fix (test-only)

**Admission gate** (jobs.rs, `#[cfg(test)]` only): a
table of gated roots plus `gate_admissions` /
`release_admissions` / `admissions_gated`. `take_next`
gains a `#[cfg(test)]` selection branch serving the
first UNGATED entry (`position` + `VecDeque::remove`),
leaving gated entries in place and in order; when every
entry is gated the worker goes idle exactly as for an
empty queue, and the gate release re-wakes it. With no
gates armed, selection is `remove(0)` — identical to
`pop_front`. In shipped builds the branch is compiled
out and `take_next` is exactly the previous code.

**No blocking under the queue mutex:** the gate check
is a pure scan under the existing lock hold — a plain
mutex, no condvar, no sleep, no wait of any kind.
Lock order is queue → gates; gate helpers take only
the gates mutex. Fairness: ungated roots keep FIFO
order among themselves; gated entries keep their
positions. Other-root concurrency is preserved: the
worker keeps serving ungated roots while a root is
gated.

**Guard + handshake** (ingestion.rs test module):
`AdmissionGate` arms the gate; its `Drop` releases the
gate AND deliberately wakes the root, so an idle worker
restarts — cleanup runs on normal completion, assertion
failure, panic, and witness timeout (`Drop` runs during
unwinding; `thread::scope` joins the observer only
after the guard has dropped). The wait loop publishes
each poll's history to the stash and notifies a static
condvar, so the test confirms — with no sleeps — that
the observer recorded pending=true with zero admissions
(30 s bound, fails loudly) BEFORE releasing the gate.
Sequence: gate → wake → assert mirror pending +
admissions 0 → spawn scoped observer → confirm witness
→ drop gate → join → assert exact 3/3 + fault
consumption + the witness assertion (identical in
form) + parked.

**The 30-root fixture loop is removed** — it was the
unsound mechanism. The twelve-root contention test
remains the multi-cycle coverage, untouched. The phase
machine, bounds (10 s stall, 60 s overall), 25 ms poll,
parked-first check, and never-re-wake behavior are
unchanged. No extra sleeps, no weakened assertions, no
serial execution, no timeout increase.

## 4. Production unchanged

The jobs.rs diff is the gate table/helpers, the
`#[cfg(test)]` selection branch in `take_next` (exactly
one removed line: the `match self.pending.pop_front()`
head, replaced by the cfg-gated selection + `match
taken`), and unit tests. The ingestion.rs diff is
confined to the test module. No queue, worker, budget,
FIFO, fault-seam, publication, or diagnostic logic
changes in shipped builds. Verified by diff and by a
clean non-test build (only the pre-existing `mcp.rs`
warning): both reviewer probes, both formerly failing
tests, the twelve-root test, the never-admitted test,
and the scripted phase tests are byte-identical to the
reviewer tip.

## 5. Coverage

- **Gate unit tests** (2): `take_next` skips a gated
  front entry and serves the next root, the gated
  entry keeps its place and is served after release
  (wake restarts the idle worker); gating is
  root-scoped (a sibling root is served while another
  is gated). Unique root keys per test (the
  process-global-table lesson from the mirror tests).
- **Rewritten witness test** (same name, same witness
  assertion, same exact counts): single gated root,
  scoped observer thread, condvar-synchronized witness
  confirmation before release. Runs in 0.22 s (the
  thirty-root version took ~7–10 s) and passed 5
  consecutive isolated runs plus every broad run.
- **Preserved:** reviewer probes byte-identical;
  scripted phase tests, twelve-root test,
  never-admitted test, fairness/cancellation tests
  untouched and green; bounds unchanged.

Known limits (design §7): the 30 s witness bound
exists only to fail loudly if the observer never
records (a harness failure, not a product bound) — in
the passing case the witness appears on the first
poll. The gate is test-only and root-scoped, armed
only by the witness test. If every queue entry is
gated the worker goes idle until a release re-wakes
it; the guard's `Drop` always re-wakes.

## 6. Before / after evidence

Before — at the reviewer tip, from the review package's
broad log
(`wr-rec-pre-admission-windows-broad-1.log`): the
panic quoted in §1, core 345 + 1 failed in 271.42 s.
The first observation already showed active=true with
2 admissions: the test's timing assumption, not the
wait, failed.

After — on this branch (Linux, §7): the witness test
passes in the default broad command (3 consecutive
runs) and in isolation, with the gate guaranteeing
the observation by construction. The true before/after
verdict for the Windows broad command comes from the
rerun in §8: this slice does not claim it from Linux
results. Note the Windows review already established
the original timeout cases fixed; this slice repairs
only the witness fixture.

## 7. Evidence (Linux, this VM, on this branch)

- Rust lib, default broad command, 3 consecutive runs:
  **417 passed** (4 + 349 + 1 + 63), 0 failed each run.
- Rust lib, sequential broad run (`--test-threads=1`):
  **417 passed**, 0 failed (regression check only — the
  default parallel command is the acceptance command).
- Full workspace `--tests` run: **655 passed**, with only
  the 4 pre-existing live-Ollama failures
  (`partial_chunk_insert_failure_rolls_back_complete_chunk_set`,
  `concurrent_edit_and_privacy_change_abort_atomic_embedding_publication`,
  `real_semantic_embedding_indexing_retrieval_and_sealed_refusal`,
  `reranking_temporal_validity_and_contradiction_lifecycle_evaluation`).
  No other failure. (655 = 653 at the reviewer tip + 2
  gate tests.)
- Integration `chatgpt_signin`: **32/32 passed**.
- Integration `provider_dispatch`: **39/39 passed**.
- Focused queue tests (`worker_queue` filter): **19/19**
  (unchanged tests, unchanged count).
- Mirror state tests (`worker_test_state` filter):
  **3/3**.
- Gate tests (`admission_gate` filter): **2/2**.
- Focused ingestion module: **46/46** (same count: the
  witness test was rewritten 1-for-1).
- Parking-wait tests (`parking_wait_` filter): **7/7**.
- Reviewer probes (`reviewer_` filter): **2/2**.
- Witness test in isolation, 5 consecutive runs: **1/1
  each**, 0.22 s per run.
- TypeScript: `tsc --noEmit` 0 errors.
- Contracts: `--check` exit 0, 0 drift (no DTO change).
- Docs validation: **21/21 passed**.
- UI suite (`pnpm test:ui`): **347/347 passed**.
- Node suite (`pnpm test`): **39/40** — the one failure is
  the pre-existing MSVC installer test. Not a regression.
- Non-test build (`cargo build -p anansi-core`): clean
  except the one pre-existing `mcp.rs:715` warning
  (`preserved_env_entries` never used). No warning from
  this slice.
- e2e regressions (lease-run, one spec per invocation):
  connect-ai 1/1, DOCX 7/7, PDF 5/5, TXT/MD 9/9, Ask 1/1,
  job-recovery 1/1 — **24/24 passed**; lease released.
- `rustfmt` state: the two touched files formatted
  (non-root files only); the diff contains no stray
  reflows.

Linux evidence is Linux evidence: it does not close the
Windows rerun. The Windows verdict comes from §8.

## 8. Windows rerun instructions

1. Apply the bundle ZIP on Windows in a separate review
   worktree on top of reviewer tip `399bb0af`
   (docs-only commit on `606450202`); confirm the tip
   stated in the delivery message (handoff commit; last
   code commit `7a2ac7b98`) and a clean tree. Preserve
   the running pilot and existing data.
2. Run the exact acceptance command, unchanged, with
   default Rust concurrency (do NOT substitute
   `--test-threads=1`):
   `node scripts/cargo.mjs test --locked --workspace --lib`
   Expect: all green. Windows library inventory is 414
   at the reviewer tip + 2 new gate tests = 416
   expected (one lib test is platform-gated — record
   the observed inventory, do not assume the Linux
   417). The command must execute ALL FOUR library
   binaries in ONE invocation: the prior review ran
   desktop/provider separately after the core failure,
   so a clean single-invocation aggregate run is still
   the open claim. If any test fails, preserve the
   failure output in full. Do not report a clean suite
   on an isolated rerun alone.
3. Run the focused checks: both reviewer probes
   (`reviewer_` filter, expect 2/2), queue tests
   (`worker_queue` filter, expect 19/19), mirror tests
   (`worker_test_state` filter, expect 3/3), gate tests
   (`admission_gate` filter, expect 2/2), ingestion
   module (expect 46/46), parking-wait tests
   (`parking_wait_` filter, expect 7/7), the witness
   test individually (expect 1/1, fast — no
   thirty-root setup), the two formerly failing tests
   individually, `--test chatgpt_signin` (expect
   32/32), `--test provider_dispatch` (expect 38/38,
   the Unix-only symlink test excluded),
   `tsc --noEmit` (0), contracts `--check` (0 drift),
   docs validation (21/21), UI suite (347/347).
4. Run the e2e regressions: connect-ai 1/1, DOCX 7/7,
   PDF 5/5, TXT/MD 9/9, Ask 1/1, job-recovery 1/1.
5. **Do not** enable the sign-in button or attempt a live
   Responses turn: eligibility is still unverified (spike
   handoff §5) — this slice changes no eligibility fact.

## 9. Reproduction commands (Linux, this VM)

- The witness test (repeat to shake out races):
  `node scripts/cargo.mjs test --locked -p anansi-core --lib parking_wait_survives_pending_before_first_admission`
- Gate unit tests:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib admission_gate`
- Reviewer probes:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib reviewer_ -- --nocapture`
- Queue discipline + mirror:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib worker_queue`
  `node scripts/cargo.mjs test --locked -p anansi-core --lib worker_test_state`
- Phase machine + witness:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib parking_wait_`
- Fault + diagnostic tests:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::`
- Broad library suite, default concurrency (acceptance):
  `node scripts/cargo.mjs test --locked --workspace --lib`
- Full test binaries:
  `node scripts/cargo.mjs test --locked --workspace --tests`
  (expect the 4 pre-existing live-Ollama failures only)
- Non-test build (gate compiled out):
  `node scripts/cargo.mjs build --locked -p anansi-core`

## 10. Acceptance criteria (AC-R1..R8, from the design doc)

- **AC-R1 (gate):** a gated root stays pending with
  zero admissions while armed; ungated roots are still
  served (unit + witness proof).
- **AC-R2 (handshake):** the observer's history
  contains pending=true with zero admissions BEFORE
  the gate releases (condvar-synchronized, no timing
  assumption).
- **AC-R3 (completion):** after release, the root
  parks with exact 3/3 + exact fault consumption, and
  the witness assertion holds.
- **AC-R4 (no blocking under lock):** the gate check
  is a non-blocking scan; nothing waits while the
  queue mutex is held (verified by inspection).
- **AC-R5 (cleanup):** the gate releases on normal
  completion, failure, panic, and witness timeout
  (Drop guard + scope join ordering).
- **AC-R6 (preservation):** reviewer probes
  byte-identical; scripted, twelve-root,
  never-admitted, fairness/cancellation tests
  untouched and green; bounds unchanged.
- **AC-R7 (production unchanged):** gate branch and
  helpers are `#[cfg(test)]`; `take_next` compiles to
  exactly the previous code in non-test builds.
- **AC-R8 (delivery):** this handoff + the checksummed
  bundle ZIP from reviewer tip `399bb0af`. Reviewer
  docs untouched. No full-Windows-acceptance claim.

## 11. Out of scope (unchanged)

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
  REQUIREMENTS_MATRIX.md, ROADMAP.md, the review doc,
  and the published evidence); proposed updates, if any,
  belong in this handoff only. Nothing was merged, pushed,
  or promoted: this branch ships as a verified bundle
  inside a ZIP.
