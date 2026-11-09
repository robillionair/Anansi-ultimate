# SOMNIA compaction integration — Checkpoint 2 handoff

## §0 Baseline, branch, delivery

- **Baseline (verified):** Antigravity's portable Windows preview,
  `codex/windows-preview-integration` at
  `4e4e970af8ad0c406cfae35fc3da55f99662b174`, supplied as
  `anansi-windows-preview-muse-base-2026-11-09.zip`
  (SHA-256 `5d1ebe49a2c438301e98279f40a5249c8ab69d948f0b4713ea893e983cbcf9ca`).
- **Branch:** `muse/somnia-compaction-integration`.
  Merge commit `6b9cd2c10` brings in the five Checkpoint 1 commits
  unchanged (design `194cedf0f`, library `c4590cb78`, benchmark
  `a3a908551`, clippy fix `81893ac37`, handoff `b0febf9e5`).
- **Implementation commit:** `9b32f6009ecc7b0eb8ac815bff975788f64df119`
  ("SOMNIA compaction Checkpoint 2: wire import → candidate → review
  → approve → compare → save"), plus `b55efa93a` (e2e: pin the
  model-mismatch refusal, reset the fake per test).
- **Final tip:** recorded in the delivery ZIP's MANIFEST (the
  authoritative tip record); this handoff names the implementation
  commits, not the tip hash, so a handoff correction cannot stale
  it.

## §1 What was built

### Server — `crates/core/src/somnia_compaction_run.rs` (new)

Five ops behind `somnia_command` (`somnia-compaction-prepare` /
`-start` / `-poll` / `-cancel` / `-save`), plus the artifact type
`somnia-compaction-experiment` (23rd in `ARTIFACT_TYPES`), private,
format `anansi-somnia-compaction-experiment` v1.

- **Prepare is pure** (no provider call, no persistence): parses
  Format A JSON (`anansi-somnia-compaction-input` v1) or Format B
  text lines with the Checkpoint 1 library, runs `compact_input`,
  returns the full candidate (retained sections, excluded with
  reasons, quarantined with reasons, stats, `input_hash`,
  `content_hash`).
- **Approval binds to the exact reviewed inputs.** Start carries
  per-case `{caseId, records, quarantined, candidateHash,
  questions}`; the server rebuilds the candidate from the records
  plus the quarantined-as-issues and compares `content_hash`
  before any dispatch. A tampered hash or an edited record is
  refused (400) with zero dispatches.
- **Compare is a server-side background run** (SwarmTank AI
  registry pattern: process-local `Mutex<HashMap>`, 600 s TTL,
  32 per workspace, key `{workspace}/{runId}`; project-scoped —
  poll/cancel from another project return NotFound). Restart never
  resumes an in-flight run: it polls not-found and never
  double-dispatches; only saved artifacts persist.
- **Prompt template (exact, both arms):** `History:\n` + lines
  `[<id>] <key> (<source>, rev <n>): <text>` in canonical order +
  `\n\nQuestion: <text>\n\nAnswer with the exact value from the
  history, or UNKNOWN if the history does not state it.`
- **Scoring:** trimmed exact match → correct; abstention (empty,
  `UNKNOWN` case-insensitive, or `UNKNOWN` + space/punctuation) →
  correct when the expected answer is null, otherwise abstained;
  anything else → incorrect; dispatch failure/mismatch → failed.
- **Provider gates reused:** Zero-Cost Lock checked up front
  (same refusal messages as SwarmTank AI), explicit provider+model
  must be configured (else first configured, local-first), cloud
  needs explicit authorization, model-mismatch refusal
  (provider/model/response-model all checked — a mismatched arm
  fails the pair, the good arm's reply is still recorded, the run
  continues and completes). `disableOpenRouterReasoning` is set
  for OpenRouter. Max output tokens 384, temperature 0.
- **Budget:** checked before every dispatch against
  `tokens_used + (prompt chars/4 + 384)`; exhaustion ends the run
  as `budget-exhausted`. Caps: cases ≤ 40, questions/case ≤ 4,
  pairs ≤ 50, calls ≤ 100, maxTotalTokens 1000–200000 (default
  20000).
- **Cancellation:** flag checked before every dispatch and between
  arms. A pair whose original arm already dispatched is recorded
  as failed ("Run cancelled before the compacted arm.") — never
  silently dropped — and the run ends `cancelled`.
- **Save revalidates fail-closed** (`validate_experiment`):
  recomputes the candidate from retained ∪ excluded + quarantine
  and compares hashes, recomputes every pair prompt from the
  candidate + question, re-scores every pair, recomputes metrics
  and `content_hash`. Any mismatch refuses. Metrics count
  abstentions independently of correctness and never zero-fill
  tokens (a `None` total stays `None` with a reported-pairs count).

### UI — `packages/ui/src/SomniaCompaction.tsx` (new), mounted in `Somnia.tsx` after `SomniaModelCompare`

Import (textarea + format select + frozen-case picker) → Prepare →
review (retained by section, excluded with reason, quarantined
with reason, both hashes shown) → Approve (shows the hash; any
edit clears approval, mirroring `SomniaModelCompare.change()`) →
run with 1 s polling and Cancel → results table → Save → saved
list with a strict parser (`parseSomniaCompactionExperiment`)
for reopening. No provider call on mount (UI test + e2e). The
client computes metrics and the canonical SHA-256 with the same
field order as the server, including the omitted empty
`supersedes`; the server is the authority and refuses a mismatch.
LARP scene 09 inherits the flow automatically (same
`SomniaPanel`); entering the scene dispatches nothing.

### Frozen cases in the UI — `packages/ui/src/somniaCompactionBenchmark.ts` (generated)

All 40 cases / 50 questions, generated by
`scripts/generate-somnia-compaction-benchmark.mjs` from
`crates/core/tests/fixtures/somnia-compaction/`, each file
verified against the benchmark MANIFEST SHA-256 before embedding.
Regenerate after any fixture change; do not edit by hand.

### Contracts and routing

18 new DTOs registered in `crates/contracts/src/main.rs`
(`SomniaCompactionRecord`, `QuarantinedRecord`,
`SomniaCompactionCandidate`, `CompactionQuestion`,
`CompactionStartCase`, prepare/start/usage/pair/poll/cancel/
metrics/experiment/save request+response); client methods in
`packages/contracts/src/client.ts`; validators regenerated
(`contracts.mjs --check` clean). Routing in all four lists:
`crates/api/src/lib.rs` path allowlist, `somnia_command`
dispatcher, `ai_discovery.rs` op allowlist, `ai_discovery.rs`
persisted-config lazy-load list (`somnia-compaction-start` only).
Also fixed pre-existing preview-baseline drift in `4dbeb186a`:
`disableOpenRouterReasoning` was missing from `generated.ts` /
`readiness.json` / `validators.mjs` at `4e4e970a` (0 occurrences
in the preview tip; the field existed only in the Rust type).

## §2 Verification evidence (Linux)

All numbers below are from this Linux VM unless labelled otherwise.
Linux evidence never closes Windows verification.

| Check | Result |
|---|---|
| Rust workspace lib (`cargo test --locked --workspace --lib`) | **339/339** (api 4 + core 273 + desktop 1 + providers 61; +6 new run-module tests) |
| `tsc --noEmit` | **0 errors** |
| `node scripts/contracts.mjs --check` | **0 drift** |
| `node scripts/validate-docs.mjs` | **324 PASS** (323 + this handoff) |
| UI tests (`npm run test:ui`) | **335/335** (incl. 8 new compaction tests) |
| e2e `somnia-compaction.spec.ts` | **6/6** (workflow, approval refusal, cancellation, LARP scene 09, project scoping, model mismatch) |
| e2e `somnia-compaction-benchmark.spec.ts` | **1/1** (40 cases, 50 pairs, 100 dispatches) |
| e2e regressions | SOMNIA slices 1–3 + model comparison 9/9; documents ingestion + PDF 22/22 with SwarmTank AI + timeline; SwarmTank market/scenario/arena/slice1 10/10 |

## §3 Synthetic benchmark results (verbatim-extractor fake)

The fake answers from the prompt it was given: it replies with
the expected answer when the arm's history contains it verbatim,
otherwise UNKNOWN (or UNKNOWN when the history should not state
it). Accuracy through this fake is **synthetic by construction** —
it measures whether the compacted history still contains the
answer string, not model quality. Raw results:
`somnia-compaction-benchmark-synthetic.json` in the delivery
evidence (every pair, with replies and scores).

- **21/50 correct original, 21/50 correct compacted, 0 failed
  pairs, 1600 tokens (800 + 800, 16 per call), 7293 µs compaction
  total, 1019 ms total latency** (this run; compaction µs and
  latency vary run to run, the scores do not).
- **Zero pairs differ between arms.** The compacted history
  answers exactly the same questions as the original.
- 29/50 abstained on both arms (concrete expected answers whose
  phrasing is not verbatim in any record, e.g. "In euros." vs
  "payable in euros"); the 4 null-expected questions are inside
  the 21 correct (correct abstentions).
- Per category (pairs / correct original / correct compacted /
  abstained / failed): broken-rows 5/5/5/0/0; conflicts 5/0/0/5/0;
  deletions 6/1/1/5/0; keep-more-context 7/2/2/5/0; look-alikes
  4/0/0/4/0; missing-facts 4/4/4/0/0; prompt-injection 7/7/7/0/0;
  unicode 4/0/0/4/0; updates 8/2/2/6/0.

Workflow spec (case-001, 2 questions): 4 dispatches, 64 tokens,
correct 2/2 both arms, failed 0; saved experiment read back
private with metrics `correctOriginal 2, correctCompacted 2,
tokensOriginal 32, tokensCompacted 32, failedPairs 0` and a
64-hex `contentHash` (`saved-experiment.json` in the evidence).

## §4 AC-I10 live rerun: BLOCKED on this VM (not faked)

No authorized live provider is reachable from this Linux VM: no
ollama binary, port 11434 closed, no provider credential in the
environment, and `ANANSI_SOMNIA_LIVE_ENDPOINT` /
`ANANSI_SOMNIA_LIVE_MODEL` are unset. The live rerun is reported
blocked per the design gate — it was not faked, not substituted
with the synthetic provider, and no credential was invented.

The rerun path is ready for the Windows side (or any environment
with an authorized provider). It follows the model-comparison
spec's env-var pattern:

```
ANANSI_SOMNIA_LIVE_ENDPOINT=http://127.0.0.1:11434 \
ANANSI_SOMNIA_LIVE_MODEL=qwen2.5:3b \
node scripts/with-application-test-lease.mjs \
  node_modules/@playwright/test/cli.js test \
  --config playwright.config.ts \
  tests/e2e/somnia-compaction-benchmark.spec.ts
```

In live mode the spec points the provider config at the endpoint,
passes `maxTotalTokens: 200000` (a live 100-call run estimates
~50k tokens against the per-call estimate), and writes the same
raw report with per-category accuracy, provider-reported tokens,
failed pairs, compaction time, and total latency — labelled
`live`. Expected live behaviour to confirm: 50 pairs, 100 calls,
0 failed pairs (or the failures listed per pair), tokens reported
for every pair.

## §5 Acceptance criteria

- **AC-I1** Met — design + AC committed before code (`e0550b539`).
- **AC-I2** Met — prepare pure; tampered hash refused at start
  with zero dispatches (e2e).
- **AC-I3** Met — review shows all sections with reasons; approval
  binds to `input_hash` + `content_hash`; edits clear approval in
  the UI and are refused by the server (e2e).
- **AC-I4** Met — both histories to the same configured model
  through the provider gates; mismatch refuses the pair (new e2e
  test: original arm failed with the mismatch message, good arm's
  reply preserved, run completes).
- **AC-I5** Met — cancellation stops the run between dispatches
  and records the dispatched call; restart polls not-found and
  never resumes (e2e).
- **AC-I6** Met — poll/cancel from another project return 404;
  switching projects shows no run and no saved experiments (e2e).
- **AC-I7** Met — save stores the complete private experiment
  with fail-closed hash; reopens with no provider call (e2e).
- **AC-I8** Met — synthetic e2e 6/6 incl. cancellation, project
  switch, restart/reopen.
- **AC-I9** Met — frozen 40-case benchmark through the run op
  with the synthetic provider; accuracy labelled synthetic (§3).
- **AC-I10** BLOCKED — no authorized live provider reachable on
  this VM; rerun path ready (§4).
- **AC-I11** Met — scene 09 shows the compaction flow (picker,
  Prepare button visible in the chamber console) with zero
  provider traffic on entry; the session runs with reduced motion
  on. The pre-existing `larp-somnia.spec.ts` instruments failure
  below is unrelated to the compaction mount (proven on the
  pristine baseline).
- **AC-I12** Met, with one pre-existing failure — regressions
  green except `larp-somnia.spec.ts` (§6).

## §6 Pre-existing failures (not regressions, out of scope)

- **`tests/e2e/larp-somnia.spec.ts` — instruments test fails.**
  Clicking "Load built-in cases" times out on scroll-into-view.
  Reproduced identically on the pristine preview baseline
  `4e4e970a` in a worktree, and with the compaction component
  unmounted — it is not caused by this slice. The chamber's
  console is a sibling of the chamber region, not a child (found
  while writing the AC-I11 test).
- **`tests/e2e/backups.spec.ts`** — stale "Workspace" locator
  (Dashboard Slice 1 removed the button; pre-existing, recorded
  in earlier handoffs).
- **4 `semantic_workflow` live-Ollama Rust failures** — no local
  model on this VM (pre-existing).
- **Strict-Clippy 4 pre-existing lints** (`mcp.rs:715`,
  `mcp.rs:1225`, `mcp_server.rs:584`, `mcp_server.rs:987`);
  `cargo fmt --check` fails on the pristine baseline (never
  bulk-formatted; `rustfmt --edition 2024` scoped to touched
  files only).

## §7 Proposed global-status updates

None. AN-015 stays `verified`; no new requirement row until
Windows review. No Antigravity-owned file was edited
(`BUILD_STATUS.md`, ROADMAP, REQUIREMENTS_MATRIX, RECOVERY,
SPEC_COVERAGE, release tooling untouched).

## §8 Reproduction steps

```
# From the delivery branch (muse/somnia-compaction-integration):
node scripts/cargo.mjs test --locked --workspace --lib   # Rust 339/339
node_modules/.bin/tsc --noEmit                          # 0 errors
node scripts/contracts.mjs --check                      # 0 drift
node scripts/validate-docs.mjs                          # 324 PASS
npm run test:ui                                         # 335/335
node scripts/cargo.mjs build --locked -p anansi-api
cd apps/web && node node_modules/vite/bin/vite.js build && cd ../..
node scripts/with-application-test-lease.mjs \
  node_modules/@playwright/test/cli.js test \
  --config playwright.config.ts \
  tests/e2e/somnia-compaction.spec.ts \
  tests/e2e/somnia-compaction-benchmark.spec.ts        # 6/6 + 1/1 synthetic
```

Live rerun: see §4. Regenerate the UI benchmark module after
any fixture change:
`node scripts/generate-somnia-compaction-benchmark.mjs`
(fails unless every fixture matches the benchmark MANIFEST hash).

## §9 Files changed (implementation commit `9b32f6009`)

- `crates/core/src/somnia_compaction_run.rs` (new, ~1600 lines
  incl. 6 unit tests), `crates/core/src/lib.rs`,
  `crates/core/src/somnia.rs`, `crates/core/src/artifacts.rs`,
  `crates/core/src/ai_discovery.rs`, `crates/api/src/lib.rs`
- `crates/contracts/src/main.rs`,
  `packages/contracts/schemas/readiness.json`,
  `packages/contracts/src/generated.ts`,
  `packages/contracts/src/validators.mjs`,
  `packages/contracts/src/validators.d.mts`,
  `packages/contracts/src/client.ts`
- `packages/ui/src/SomniaCompaction.tsx` (new),
  `packages/ui/src/somniaCompactionBenchmark.ts` (generated),
  `packages/ui/src/Somnia.tsx`,
  `packages/ui/tests/somnia-compaction.test.tsx` (new, 8 tests),
  `package.json` (`test:ui` list)
- `scripts/generate-somnia-compaction-benchmark.mjs` (new)
- `tests/e2e/somnia-compaction.spec.ts` (new, 6 tests),
  `tests/e2e/somnia-compaction-benchmark.spec.ts` (new),
  `tests/e2e/somnia-compaction-helpers.ts` (new) — the mismatch
  test and per-test fake reset in `b55efa93a`
- `docs/handoffs/somnia-compaction-integration-design.md`
  (§2 prompt template + §3 caps, committed with the implementation)
