# Windows ingestion test reliability — handoff

Branch: `muse/windows-ingestion-test-reliability`
Base: lifecycle repair 4 tip `6f34f905579aec5b7703578fb8abb57f0eae5a97`
Tip: the handoff commit on this branch (exact SHA in the delivery
message); last code commit `8a92a139d`
Date: 2026-11-11 (Linux evidence only — **WINDOWS RERUN OUTSTANDING**)
Design (committed before implementation):
`docs/handoffs/windows-ingestion-test-reliability-design.md`
(`6a0b320b7`, AC-K1..K9)

## 1. The Windows finding (preserved)

From the supplied Windows evidence package
`signin-lifecycle4-windows-review-2026-11-11.zip` (SHA-256
`ecb478c7205eb3c93886a3def05f6239a55c012766db2d65767fa5eb79ccc717`;
manifest verified: all 18 files match their SHA-256; runtime tip
`6f34f9055`, review worktree
`E:\Anansi\worktrees\chatgpt-signin-lifecycle-repair-4-review`):

- Broad Windows library run: API 4/4, then core 297/298, failing
  `ingestion::tests::docx_happy_path_publishes_document_with_section_provenance`
  at `crates/core/src/ingestion.rs:893`:
  `assertion left == right failed, left: Queued, right: Completed`.
- The unchanged test then passed alone in 6.91 seconds.
- Remaining crates passed API 4/4, desktop 1/1, providers 63/63.
  Observed unique Windows library inventory is 366 (not the
  Linux 367 — one lib test is platform-gated).

This was an unresolved broad-run failure, not proof of a
production defect and not a clean suite. The DOCX cause was not
established on Windows.

Focused Windows acceptance from the same package (recorded):
`chatgpt_signin` 32/32, `provider_dispatch` 38/38
(Windows-expected count), UI 342/342, Connect AI browser 1/1,
TypeScript/contracts/builds passed; real ChatGPT sign-in stays
dev-gated.

## 2. What was changed

Commits on the branch (base `6f34f9055`):

- `6a0b320b7` — design + acceptance criteria (AC-K1..K9) before code.
- `30a44933a` — reproduction: the `WorkerQueue` discipline
  extracted into `wake`/`take_next` methods preserving LIFO
  semantics, with three deterministic unit tests; two fail
  (`take_next` returns "c" then "b" before "a"), the
  restart-semantics test passes. No behavior change.
- `8a92a139d` — the fix (below).

Files changed vs base (4 files): the design doc,
`crates/core/src/jobs.rs`, `crates/core/src/ingestion.rs`, and
this handoff. No DTO or contract change; no UI change; real
sign-in stays dev-gated.

### The fix (short form)

1. **FIFO pending queue** (`jobs.rs`): the worker's pending
   storages are now a `VecDeque` served with `pop_front`, so
   storages are served in wake order and an early entry can no
   longer be starved by later arrivals. The wake-up protocol
   (pending + running under one mutex, no lost wake-up) is
   unchanged, as is the claim/execute/publication path.
2. **Diagnostic bounded wait** (`ingestion.rs`):
   `wait_for_terminal` now polls the job state until terminal,
   up to a 60 s bound, recording every state transition
   (elapsed time, state, version, checkpoint, log). On success
   it returns the job exactly as before — every caller assertion
   is unchanged. On timeout it panics with the full transition
   history. It observes only: no re-wake, no forced completion,
   no weakened assertion.
3. **Burst probe** (`ingestion.rs`):
   `burst_of_storages_all_complete` wakes the worker for eight
   fresh storages at once and asserts every job completes — the
   deterministic probe for the broad-run scheduling condition.

### Characterization (summary; full text in the design §2)

One process-global worker thread serves every test storage
serially; the pending queue was LIFO, so an early-woken storage
waited behind every later wake; the DOCX test's fixed 5 s poll
expired while its job was still `Queued`, never claimed. Ruled
out: worker death, lease expiry, claim-and-fail, test-thread
starvation. Linux could not reproduce the condition (ingestion
module 20/20 three times in ~1.3 s; full core lib 299/299 twice
with 16 test threads in ~4.7 s) — the VM drains the worker too
fast, with too few CPUs, for the queue to starve anyone.

## 3. Before / after evidence

Reproduction run on the base tip (queue extraction only, LIFO
preserved):

```
test result: FAILED. 1 passed; 2 failed
- worker_queue_serves_storages_in_wake_order:
  left: "c", right: "a"
- worker_queue_dedup_keeps_first_position:
  left: "b", right: "a"
```

After the fix, same tests:

```
test result: ok. 3 passed; 0 failed
```

## 4. Evidence (Linux, this VM)

- Rust lib, parallel broad run: **371 passed**, 0 failed
  (367 baseline + 3 queue tests + 1 burst probe).
- Rust lib, sequential broad run (`--test-threads=1`):
  **371 passed**, 0 failed.
- Integration `chatgpt_signin`: **32/32 passed** (regression).
- Integration `provider_dispatch`: **39/39 passed** (regression).
- Focused ingestion module: **21/21 passed** (20 + burst probe).
- TypeScript: `tsc --noEmit` 0 errors.
- Contracts: `node scripts/contracts.mjs --check` exit 0, 0 drift
  (no DTO change).
- Docs validation: **21/21 passed**.
- UI suite (`pnpm test:ui`): **342/342 passed**.
- Node suite (`pnpm test`): **39/40** — the one failure is the
  pre-existing MSVC installer test, confirmed identical on the
  spike base commit in an earlier slice. Not a regression.
- e2e regressions (lease-run, one spec per invocation):
  connect-ai **1/1**, DOCX **7/7**, PDF **5/5**, TXT/MD **9/9**, Ask **1/1**.
- Warnings: one, pre-existing — `crates/core/src/mcp.rs:715`
  (`preserved_env_entries` never used). No warning from this slice.
- `rustfmt` applied to `jobs.rs` and `ingestion.rs` only (never
  to a crate root).

Linux evidence is Linux evidence: it does not close the Windows
rerun. The Windows verdict comes from §7.

## 5. Acceptance criteria — how each was met

- **AC-K1 (record):** the Windows failure and isolated pass are
  preserved verbatim in §1 and in the design §1, with the
  evidence package hash and the reviewer's counts.
- **AC-K2 (characterization):** design §2 records the LIFO +
  single-worker + 5 s poll mechanism, what was ruled out, the
  Linux non-reproduction experiments, and the unproven
  claim-error exposure as a documented limit (§6).
- **AC-K3 (backend fix + regression):** the pending queue serves
  in wake order; three deterministic unit tests fail against
  LIFO and pass after the switch (§3). No other worker behavior
  changes.
- **AC-K4 (diagnostic wait):** `wait_for_terminal` waits on the
  terminal condition with a 60 s bound, records transitions,
  and panics with the full history on timeout. All existing
  caller assertions are intact; no re-wake, no forced
  completion, no weakened assertion.
- **AC-K5 (probe):** `burst_of_storages_all_complete` exists,
  passes on Linux (21/21 module), and is documented as the
  deterministic Windows probe (§7 step 4).
- **AC-K6 (audit):** all ten `wait_for_terminal` call sites
  audited (design §3d); all 21 ingestion tests green;
  cancellation, restart, concurrent-upload, provenance, and
  exactly-once publication coverage preserved (the same tests,
  the same assertions).
- **AC-K7 (scope):** no new ingestion formats, authentication
  features, cloud calls, dependency upgrades, preview
  promotion, or unrelated changes; no DTO or contract change;
  real sign-in stays dev-gated.
- **AC-K8 (verification):** focused ingestion tests, browser
  workflows, and the broad Linux library suite run both
  parallel and sequentially (§4). Linux and Windows evidence
  are kept distinct throughout.
- **AC-K9 (delivery):** this handoff + the checksummed bundle ZIP
  (see §8).

## 6. Known limits

- The worker loop's `while let Ok(Some(lease))` silently drops a
  storage if `claim_export` returns `Err`, with no retry until
  the next wake for that root. No evidence shows this happened
  on Windows; it is documented, not fixed, in this slice. If it
  ever occurs, the diagnostic wait now exposes it: a job stuck
  `Queued` with zero state transitions past the bound.
- The 60 s wait bound is a safety net for a contended worker,
  not the fix; the FIFO change is what makes the wait short in
  practice. A timeout panic now carries the transition history
  needed to distinguish slow, starved, and stuck.
- No change to `chatgpt-signin-status`, the UI, the API-key
  flow, or any global status document.

## 7. Windows rerun checklist

1. Apply the bundle ZIP on Windows in a separate review
   worktree; confirm the tip stated in the delivery message
   (handoff commit; last code commit `8a92a139d`) and a clean
   tree. Preserve the running pilot and existing data.
2. Run the exact command that failed:
   `node scripts/cargo.mjs test --locked --workspace --lib`
   Expect: the DOCX happy-path test passes in the broad run.
   Record the full counts (Windows library inventory is 366
   baseline + 4 new = 370 expected; one lib test is
   platform-gated — record the observed inventory, do not
   assume the Linux 371).
3. Run the deterministic probes in isolation:
   `node scripts/cargo.mjs test --locked -p anansi-core --lib jobs::tests::worker_queue`
   (expect 3/3) and
   `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::tests::burst_of_storages_all_complete -- --exact`
   (expect 1/1).
4. Run the focused suites: `--test chatgpt_signin` (expect
   32/32), `--test provider_dispatch` (expect 38/38, the
   Unix-only symlink test excluded), `tsc --noEmit` (0),
   contracts `--check` (0 drift), docs validation (21/21), UI
   suite (342/342).
5. Run the e2e regressions: connect-ai 1/1, DOCX 7/7, PDF 5/5,
   TXT/MD 9/9, Ask 1/1.
6. If the broad run fails again, preserve the failure output in
   full: the diagnostic wait's panic now carries the job's
   transition history — that history, not just the final state,
   is the finding. Do not report a clean suite on an isolated
   rerun alone.
7. **Do not** enable the sign-in button or attempt a live
   Responses turn: eligibility is still unverified (spike
   handoff §5) — this slice changes no eligibility fact.

## 8. Reproduction commands (Linux, this VM)

- Queue regression (fails on LIFO, passes on FIFO):
  `node scripts/cargo.mjs test --locked -p anansi-core --lib worker_queue`
- Focused ingestion module:
  `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::`
- Broad library suite, parallel and sequential:
  `node scripts/cargo.mjs test --locked --workspace --lib`
  `node scripts/cargo.mjs test --locked --workspace --lib -- --test-threads=1`
- Amplified-contention characterization (clean on Linux):
  `node scripts/cargo.mjs test --locked -p anansi-core --lib -- --test-threads=16`

## 9. Out of scope (unchanged)

- The claim-error drop (§6) — documented, not fixed.
- Any worker redesign, production scheduling policy change
  beyond FIFO order, or e2e change.
- Any change to the API-key flow, the ChatGPT sign-in prototype,
  the frozen Windows pilot, or any provider op/contract.
- `providers.spec.ts` 240 s timeout and the other pre-existing
  failures (larp-somnia instruments test, backups.spec.ts
  'Workspace' button, 4 semantic_workflow live-Ollama failures,
  node pwsh failure, MSVC installer test — all pre-existing on
  this VM).
- No global status document was edited (Antigravity-owned files
  untouched); proposed updates, if any, belong in this handoff only.
