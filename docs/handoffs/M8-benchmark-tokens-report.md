# ANANSI benchmark: token consumption and context efficiency — before/after report

Branch `muse/m8-benchmark-tokens` from Windows-reviewed
checkpoint `df8072595e672ead29ad0df82393c31de7b6f7d8`.
Measurement only. No headline savings claim, no publication,
no preview promotion, no generated-code execution, no push,
no remote CI. Linux results only; Windows verification is a
separate, still-open step and nothing here closes it.

## What was compared, and what was not

- Approaches: ANANSI Ask (retrieval + context compilation +
  current question only; no conversation history ever sent)
  vs two baselines defined WITHOUT inflating them:
  B-NAIVE = same instruction header + full untruncated bodies
  of exactly the memories ANANSI cited (shared retrieval, so
  it inherits ANANSI's retrieval misses); B-HISTORY (A8 only)
  = all prior turns verbatim. A whole-workspace dump was
  deliberately excluded.
- Corpus: deterministic fictional "Harbor" corpus, frozen
  before baseline runs. Nine primary cases: A1 launch window,
  A2 long-memory annex key, A3 two-memory seats total, A4
  current-revision tide, A5 contradiction, A6 insufficient
  evidence (berth fee, zero provider calls by design),
  A11–A13 long cases, A12 budget case. A6 has no prompt for
  either approach and stays in every denominator as
  correct-insufficient, never as a saving.
- Models/settings: local Qwen qwen2.5:3b (largest installed
  instruction model proven stable at this workload) and
  qwen2.5:1.5b, via local Ollama, Ask options
  num_ctx 2048 / num_predict 96 / keep_alive 0; and, under a
  separate authorization, the free hosted model
  nvidia/nemotron-3-ultra-550b-a55b:free via OpenRouter
  (temperature 0, max_tokens 1024). Settings differ between
  the local and hosted runs, so cross-model accuracy
  differences are NOT a controlled model-size experiment.
- Three token figures are kept separate throughout and never
  added or averaged: (1) ESTIMATED input tokens = prompt
  chars/4 (the harness/manifest heuristic; the manifest
  estimate is pre-dispatch and is never a billing figure);
  (2) ACTUAL REPLAY usage = Ollama prompt_eval_count +
  eval_count when an exact prompt is replayed with Ask's exact
  options (a separate measurement of the identical prompt,
  not the ANANSI call's usage, because Ask discards provider
  usage by design); (3) END-TO-END application usage =
  provider-reported usage recorded by the application itself.
  For Ask, end-to-end usage is NOT recorded at all (Ask never
  parses provider-reported usage); only the manifest estimate
  exists. For M5 Collider/stream runs, disclosure + ledger
  records exist but only on the fake fixture so far; no
  real-model Collider usage is claimed. Nothing reconciles
  estimate vs actual, and cached tokens are always reported
  as a separate counter (0 wherever measured, never inferred
  from latency).

## Repairs (all committed, no assertion weakened, no deadline raised)

1. Concurrent migration StorageUnavailable: root cause was
   the journal_mode WAL switch failing with DatabaseBusy
   while peer connections held the database open (busy
   timeout does not apply to that pragma). Fix tolerates only
   DatabaseBusy on that switch; other errors still fail.
   Regression test: 20 iterations × 16 barrier threads,
   fails pre-fix, passes post-fix.
2. A12 context selection: chunks within each memory are now
   ordered by query relevance before the UNCHANGED 4000-char
   budget is applied (most distinct query terms, then most
   occurrences, then document order). Question, fact position
   and budget unchanged. A12 re-gated as an ordinary case.
3. A1/A3 retrieval (Ask-scoped only; global search
   unchanged): expanded Ask stopwords + stemming + prefix
   marking (a recall superset; no exact match lost).
   A1/A3 recall probes now answer.
4. Ask browser real-model mode: parameterized to
   ANANSI_REAL_MODEL_TAG, default qwen2.5:3b (was hardcoded
   qwen2.5:0.5b, not installed). Assertions preserved;
   exercised 1 passed in 25.0 s.
5. D1 accounting disclosure (before baseline): completed M5
   stream runs now copy drained Usage into the disclosure
   (was null while the ledger recorded 10/40 on the fake
   fixture). M1 asserts disclosure == ledger on the fixture.

## Qwen local results — estimated input tokens (chars/4), primary cases

Every case kept in the denominator. Correct = answer states
the rubric fact as its conclusion (reasoning-only mentions do
not count; A5 requires BOTH 120 and 180; A3 requires the
stated total 300; A6 correct-insufficient is separate from
correct answers). Each approach's tokens-per-correct uses ITS
OWN correct count. Failed-attempt tokens are included in
totals and in tokens-per-correct. Citation support is marker
presence only, reported separately from fact correctness.

### Baseline tree (original, before repairs)

- 3B runs 1–3: evidence classification and estimates
  identical. Strict harness count 5/9 per run (4 correct +
  A6). Text-verified (answers read): runs 2–3, 6/9 (5 correct
  + A6; A5 correct in both, verified by text; run 1 A5
  unverified because its answer was not retained).
  Failures: A1/A3 retrieval failures, A12 compilation
  failure. ANANSI estimated primary total 4,968 vs naive
  8,421 (all attempts, failed included). Per correct
  (own count, estimates): ANANSI 4,968/5 = 993.6 (runs 2–3).
  Naive per-correct comes from replay scoring (below), not
  from ANANSI's count.
- 1.5B run 1: same evidence/estimates (prompts are
  model-independent). A5 genuinely wrong (180 only).
- Baseline replay (3B, context-preparation replay with
  shared retrieval, 16 prompts = 8 answerable × 2
  approaches): ANANSI correct 5/8 (A2, A4, A5, A11, A13;
  A1/A3 no prompt, A12 model correctly reported the evidence
  lacked the tonnage); naive correct 5/8 (A4, A5, A11, A12,
  A13; A1/A3 no evidence, A2 wrong at num_ctx 2048).
  ACTUAL input+output usage: ANANSI input 4,567 + output
  200 = 4,767 total, 953.4 per correct (own count 5);
  naive input 5,611 + output 353 = 5,964 total, 1,192.8 per
  correct (own count 5). Truncation confirmed: A2-naive at
  num_ctx 2048 evaluated 1,026 prompt tokens and answered
  wrong; the identical prompt at 8192 evaluated 2,999 tokens
  and answered correctly. Supplementary subset (both complete
  prompts fit the window; A4, A5, A11, A12, A13): ANANSI
  3,715 total, 4/5 correct, 928.8 per correct; naive 4,607
  total, 5/5 correct, 921.4 per correct — essentially tied
  per correct, because naive answers the extra (A12) case.
- Baseline 1.5B replay: ANANSI 3/8 after manual correction
  (A13's stated answer was "forty-four tide charts" despite
  mentioning fourteen in reasoning — wrong under the rubric),
  4,757 total, 1,585.7 per correct (own count 3); naive 4/8,
  5,888 total, 1,472.0 per correct (own count 4).

### Repaired tree (after repairs 1–4)

- 3B run 1 (owner rubric, answers read): 7/9 (6 correct +
  A6). Failures: A3 retrieval fixed but answer truncated
  before stating 300 (96-token output budget — an output
  failure, not a retrieval failure); A5 wrong (120 only,
  denying the autumn evidence). Citation support among the
  6 correct: 3 with marker, 3 without.
- 3B run 2 (owner rubric, answers read): 9/9 (8 correct +
  A6). A3 states 300; A5 states both values. Citation
  support among the 8 correct: 6 with marker, 2 without
  (A4, A13). A5 citation order was B-first in both 3B runs'
  prompts, so the run 1 → run 2 A5 flip is sampling
  variation, not an order change.
- 1.5B run 1 (owner rubric, answers read): 8/9 (7 correct +
  A6). Failure: A5 wrong (180 only). Citation nuance:
  among the 7 correct, 5 carry markers but only 4 point to
  the right source — A4's 2.7-meter fact is correct but
  cites [1] (Pier log) where the value lives in [2]
  (Tide notes); A5's [1] likewise points to the wrong
  survey. A5 prompt order flipped to A-first in this run
  (vs B-first in the 3B runs): retrieval tie order is
  nondeterministic across runs, so A5 citations must be
  judged against each run's own prompt.
- Estimated primary totals, repaired (per run, all attempts):
  ANANSI 5,320 vs naive 8,694 — higher than baseline's
  4,968 vs 8,421 because the repaired tree now actually sends
  A1/A3 prompts (and A4/A12 grew slightly). Per correct
  (own count, estimates): 3B run 1, 5,320/6 = 886.7; 3B run 2,
  5,320/8 = 665.0; 1.5B run 1, 5,320/7 = 760.0; baseline
  3B runs 2–3, 993.6. Accuracy improved while estimated cost
  per correct answer fell in every repaired run.
- Secondary breakdown ONLY (successful cases, labelled):
  baseline strict secondary (4 samples): ANANSI est 3,635
  vs naive 6,397 (−43.2%) — excludes failed attempts and is
  NOT the overall result. Repaired secondary breakdowns are
  computed the same way in the analysis scripts and are
  labelled as secondary wherever shown.
- REPLAYED repaired results (actual input+output usage,
  Ask settings num_ctx 2048 / num_predict 96, answers read
  and scored independently per the rubric, each approach's
  own correct count; A6 has no replay prompt by design):
  3B replay: ANANSI 8/8, input 4,932 + output 324 = 5,256
  total, 657.0 per correct; naive 6/8, input 5,845 + output
  225 = 6,070 total, 1,011.7 per correct. ANANSI failures:
  none. Naive failures: A2 (truncation — only 1,026 prompt
  tokens evaluated, model claims the key is never mentioned;
  same window effect as baseline) and A5 (plain model miss:
  "capacity is missing" though both values are in its short
  prompt; baseline replay had naive A5 correct, so this flip
  is sampling variation). A3-anansi hit the 96-token cap
  (done=length) but states 300 before the cut, unlike the
  run 1 harness answer cut before stating it. Citation
  markers: ANANSI 7/8 (all but A13), naive 3/6 among its
  correct answers.
  1.5B replay: ANANSI 6/8, input 4,932 + output 157 = 5,089
  total, 848.2 per correct (own count 6); naive 6/8, input
  5,845 + output 234 = 6,079 total, 1,013.2 per correct (own
  count 6). ANANSI failures: A5 (180 only, same as its
  harness run) and A13 (stated "forty-four tide charts" —
  the same reasoning-vs-stated trap as baseline 1.5B replay).
  Naive failures: A2 (hallucinated "Archive Annex, main
  building" location — a different wrong mode from 3B's
  refusal and baseline 1.5B's "shelf 01"; naive A2 fails in
  every replay) and A5 (180 only). Input token counts are
  identical between 3B and 1.5B replays for every prompt
  (same Qwen2.5 tokenizer); only outputs differ.
  Baseline-to-repaired replay change: 3B ANANSI 5/8 →
  8/8 (4,767 → 5,256 total, 953.4 → 657.0 per correct);
  3B naive 5/8 → 6/8 (5,964 → 6,070 total, 1,192.8 →
  1,011.7 per correct) — naive improves ONLY because
  ANANSI's retrieval, which defines naive's evidence, now
  includes the A3 wings (and A1). 1.5B ANANSI 3/8 → 6/8
  (4,757 → 5,089 total, 1,585.7 → 848.2 per correct);
  1.5B naive 4/8 → 6/8 (5,888 → 6,079 total, 1,472.0 →
  1,013.2 per correct). No replay figure is mixed with
  estimates or with end-to-end usage.

### Answer to the repair question

Accuracy improves after repairs (baseline best 6/9 →
repaired 7/9 and 9/9 on 3B, 8/9 on 1.5B) while estimated cost
per correct answer falls. Savings remain in the sense that
ANANSI's estimated primary total (5,320) stays below naive's
(8,694) per run, and per-correct figures improve because
more cases are answered. On ACTUAL tokens the same verdict
holds: repaired 3B ANANSI total 5,256 vs naive 6,070
(−13.4% total, −35.1% per correct); repaired 1.5B ANANSI
5,089 vs naive 6,079 (−16.3% total and per correct, both
6/8). The one long-memory example (A2) does not dominate
the headline: long cases and short cases are reported
separately (see unfavorable findings), and every case stays
in the denominator.

## OpenRouter Nemotron results (separate from Qwen; synthetic corpus only)

- 32 scored responses = EIGHT distinct answerable questions,
  each asked twice per approach (2 runs × 8 cases × 2
  approaches). A6 is not among them (no prompt, zero calls).
  Smoke (A4, correct, 339 tokens) is excluded from scored
  totals. Model nvidia/nemotron-3-ultra-550b-a55b:free,
  verified zero pricing, context 1,000,000, exact model
  returned on all 33 requests, allow_fallbacks false,
  temperature 0, max_tokens 1024, no system message.
- Provider-reported usage (end-to-end through OpenRouter for
  these exact prompts; NOT ANANSI's internal manifest, which
  does not record Ask usage): ANANSI 16/16 correct, input
  9,802, output 2,336 (reasoning 2,011, visible 325), total
  12,138, 758.6 per correct (own count 16); naive 16/16
  correct, input 15,584, output 1,990 (reasoning 1,796,
  visible 194), total 17,574, 1,098.4 per correct (own count
  16). Input −37.1%, total −30.9%. All finish stop, zero
  errors, cost 0, cached tokens 0. Per run: ANANSI 8/8,
  4,901 input, 6,069 total each run; naive 8/8, 7,792 input,
  8,769 / 8,805 total.
- Citation audit (MY OWN MANUAL AUDIT, PENDING INDEPENDENT
  REVIEW): every marker in all 32 responses was checked
  against the numbered evidence entry in the prompt actually
  sent. Result: 32/32 fact-correct AND 32/32
  citation-supported (A3's 300 is arithmetic from two cited
  sources, shown as such; A5 states both values and refuses
  to pick one; A4 never states the superseded 2.4 meters as
  current). One wording looseness noted, not counted as an
  error (run 1 ANANSI A13's "remaining records" cluster is
  supported in substance by [1]–[4]). This audit checks that
  cited sources contain the facts, not the model's internal
  reasoning, and covers this synthetic corpus only.
- The earlier Gemma attempt is unchanged: zero scored
  requests (smoke HTTP 429 from the upstream shared pool,
  stopped per instruction, no retry/switch/reroute).
- Nemotron vs Qwen accuracy is NOT a controlled comparison
  (model, tree version, settings, and verbosity all differ).
  The within-model finding is: on repaired-tree prompts,
  Nemotron answers 8/8 for both approaches, and ANANSI uses
  fewer tokens for the same perfect accuracy.

## Unfavorable findings (retained, not softened)

- Short-input overhead: when evidence is short, ANANSI's
  fixed header costs more than it saves. Nemotron short cases
  (A1, A3, A4, A5) ANANSI input +7.6% vs naive per run;
  Qwen fake expanded: A4 ANANSI est +1.4%, A5 +9.9% vs naive.
  Long cases save (Nemotron long cases −40.8% input).
- Higher reasoning output: Nemotron ANANSI output +17.4%
  vs naive (2,336 vs 1,990), driven by reasoning tokens
  (2,011 vs 1,796). Savings are input-driven; total is still
  lower because input savings dominate.
- Local-model errors: Qwen 3B repaired run 1 A3 truncation
  (96-token budget) and A5 single-value denial; 3B run 2
  fixes both by sampling luck as much as by repair (A5 order
  unchanged between runs). 1.5B still fails A5 on the repaired
  tree and misattributes citations (A4, A5). Baseline Qwen
  A13 1.5B reasoning-vs-stated-answer trap (fourteen in
  reasoning, "forty-four" stated) shows why the rubric grades
  stated answers only.
- Sampling variation: same tree, same settings, different
  runs give different answers (3B repaired 7/9 then 9/9; A5
  correct/wrong/correct across runs; A5 citation order flips
  between runs). Single-run accuracy figures are not stable
  estimates; the report shows every run, not the best one.
- Truncation at num_ctx 2048: baseline A2-naive lost its
  early fact at 2048 and recovered at 8192. ANANSI's prompts
  (largest replay input 1,112 tokens baseline) never approach
  the limit, but the naive baseline's loss is a window effect,
  not a retrieval effect, and is labelled as such.
- No tokenizer exists in the workspace; all internal figures
  are chars/4 heuristics. Ask records no end-to-end provider
  usage. The M5 ledger is in-memory only. No cached-token
  benefit is claimed anywhere.

## Stability of qwen2.5:3b at the benchmark workload (not short arithmetic alone)

- Short-prompt check (2026-10-28 20:50 CET): cold 27.2 s,
  warmed ~255–276 ms, all correct — necessary but NOT
  sufficient, per instruction.
- Workload evidence: three baseline harness runs (587 s,
  587 s, 500 s incl. the A7 harness-bug run) + three repaired
  harness runs (3B r1 681 s, r2 646 s, 1.5B 318 s) + baseline
  replay (510 s) + ctx8192 experiment (188 s) + Repair 4
  browser run (25.0 s), all with prompts up to ~4,700 chars
  (repaired A12) and 13,734 chars (naive A2), num_predict 96,
  no OOM, no model failure, no Ollama crash in any run.
  Model loaded size 2.1 GB (ollama ps), context 2048, 100%
  CPU on a 2-CPU, ~7.7 GiB RAM, no-GPU machine. Latencies
  vary with model-load effects (Ask keep_alive 0); A7 repeats
  showed no caching speedup (13.5/13.1/12.8 s), consistent
  with cached tokens 0.

## Final regression checks (final tree, sequential on an idle machine)

- Full core suite, final tree: 61 binaries, 329 passed,
  0 failed, 0 ignored (log core-full-final.log). The earlier
  clean-run migration failure (299/1) is gone:
  workspace_roundtrip 11/11 including the Repair 1 barrier
  test concurrent_v1_open_migration_and_wal_switch_do_not_fail.
  The 329 total exceeds the earlier 300 because the repaired
  tree adds tests (repair regressions, harness cases).
- Earlier benchmark-branch results, preserved (not re-run on
  the final tip — stated, not hidden): providers + API
  144/0 (24 binaries); node UI/contracts 81/0, tsc clean,
  contracts clean; browser canonical e2e 17/17 and
  non-canonical 17/17 (34/34 across 28 spec files, no spec
  excluded, including the reviewer's memory-consent /
  project-switch acceptance probes and the real semantic UI
  workflow); Repair 4's Ask real-model spec exercised on the
  final tree (1 passed in 25.0 s). Fast final-tree checks
  all pass: Clippy -D warnings exit 0, fmt --check exit 0,
  docs validator PASS (140 Markdown files), tsc exit 0,
  contracts check exit 0, API + web builds exit 0. A full
  e2e rerun on the final tip was not repeated, and that
  scope limit is listed here rather than implied away.

## Remaining defects and unverified gates (not closed by this work)

- Real-model Collider workflow: harness Collider scenarios
  still use the fake M5 provider even in real mode; no
  real-model Collider usage is measured.
- Real-model Ask end-to-end usage: Ask discards provider
  usage by design; replay is a separate measurement, not the
  application's own usage. Closing this needs a product change
  (parse/record usage), not a benchmark change.
- A5 citation-order nondeterminism across runs (tied
  retrieval scores); A3 output-budget truncation risk at
  num_predict 96 for verbose answers; Qwen citation
  misattribution (1.5B A4/A5).
- Windows verification of M8 Slice 3, M7 native desktop
  acceptance + preview promotion, §32 execution sandbox,
  M5 production gates (native CredUI/keyring, live-provider
  verification, G1 external audit, remote CI) — all still
  open; Linux evidence closes none of them.
- Intermittent browser failures recorded without root cause
  (deliverable approval-screen delay; forge-review
  findings-step failure): clean reruns are not presented as
  fixes.
- Requirements matrix staleness: AN-007, AN-008, AN-026
  still marked planned though implemented and tested;
  AN-013 stays in progress. Reconcile before publishing
  coverage claims.
- §32 sandbox, SwarmTank, SOMNIA, GHOST-PROMPT, MCP, Launch
  Studio, document formats/OCR, native macOS, enterprise
  architecture, Windows NSIS signed release, OAuth-first
  setup, encrypted/sealed backup: not implemented or not
  verified here; never counted as verified.

## Reproducibility (summary; full commands in the package)

- Repo: https://github.com/robillionair/Anansi-ultimate.git
  (clone via HTTPS; push access not set up; no merge to
  master, no force-push, no remote CI by delivery rule).
- Baseline: codex/review-m8-slice3 @ df807259 (verified tip;
  ancestors 776259a4, 4d3f8bfd, 5b3cf958f).
- Branch: muse/m8-benchmark-tokens (final tip + checksums
  in the package manifest after the last commit).
- Harness: crates/core/tests/benchmark_tokens.rs; runner
  scripts/benchmark-tokens.mjs --real --out <path> (always
  with --out); analysis scripts/bench-report.py,
  scripts/bench-analyze.py; replay
  verify-benchmark-2026-10-28/replay-with-answers.sh
  <model> <prompts-dir> <out-dir> (invoke via bash; the file
  is not executable).
- Environment: Ollama 0.40.12 (official release asset,
  SHA-256 f8adcb7e…f215485ba), OLLAMA_MODELS under
  ~/workspace/ollama-models, host 127.0.0.1:11434; models
  qwen2.5:3b (digest 357c53fb…, Q4_K_M, Qwen Research License
  — NOT Apache 2.0 for 3B), qwen2.5:1.5b (Apache 2.0),
  all-minilm:22m (embedding, Apache 2.0, dimension 384).
  Machine: 2 CPUs, ~7.7 GiB RAM, no GPU, no swap, Ubuntu
  24.04.5 LTS. Keep builds/scratch under ~/workspace
  (/tmp is a 512M tmpfs).
- No cloud calls were made to prepare this report or the
  delivery package; the only cloud requests in this work are
  the previously authorized Gemma smoke (1, 429) and Nemotron
  smoke + 32 scored (33), all preserved and accounted for.
