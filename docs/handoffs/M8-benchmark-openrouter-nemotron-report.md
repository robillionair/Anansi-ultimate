# M8 benchmark: OpenRouter Nemotron experiment — results (2026-10-28)

Public-safe. Synthetic Harbor corpus only. No credentials, no
account identifiers, no Authorization headers, no personal
memories, no Sealed data, no private source code in this report
or its evidence. Kept separate from the Qwen local results and
from the unsuccessful Gemma attempt (whose status report stands
unchanged).

## Provider, model, settings (identical for both approaches)

- Provider: OpenRouter chat completions gateway. Provider
  identity beyond the model id: no provider-name header value
  was returned in any response (only the exposable-header list
  mentions it), so this report identifies the run by gateway +
  exact model id, not by an upstream provider name.
- Model (exact, as authorized, replacing the earlier
  Gemma-only restriction): `nvidia/nemotron-3-ultra-550b-a55b:free`.
- Verified before inference from the public model list: found
  exactly once, input pricing `0`, output pricing `0`,
  context length 1,000,000 tokens. Provider-reported cost on
  every scored response: `0` (including upstream cost fields).
  No paid inference.
- Sending format: each prompt sent verbatim as a single user
  message, no system message added. Temperature 0,
  max_tokens 1024 (adequate output/reasoning budget, identical
  for both approaches), provider `allow_fallbacks: false`,
  usage reporting on. Reasoning effort left at the provider
  default; reasoning text not requested. Returned model matched
  the requested id on all 33 requests (smoke + scored); any
  mismatch would have stopped the run.
- Prompts: repaired-tree prompts (branch tip `03c0898d`,
  repairs 1–3 committed), same files as the Gemma attempt's
  prompt set, frozen nine-case corpus. Largest complete prompt
  (naive A2, 13,734 chars) fits easily in the 1,000,000-token
  window; nothing was truncated. A6 has no prompt for either
  approach (no evidence exists) and stays in the denominator as
  correct-insufficient with zero calls, never as a saving.
- Request accounting: smoke 1 + scored 32 = 33 requests for
  this experiment, within the authorized 36 + 1 (the single
  spaced retry allowance was not needed). Together with the
  Gemma smoke attempt (1 request, 429) this key made 34
  requests today in this work, within the documented 50/day
  free-tier cap assuming no other use today; the key endpoint
  does not report the daily request count, so this report does
  not claim a confirmed remaining allowance.
- Logging note: as the owner noted, NVIDIA's free endpoint
  logs requests. Only synthetic corpus prompts and answers
  were sent.

## Smoke test (excluded from scored totals)

- ANANSI A4 prompt (708 chars), 2026-10-28 ~22:24 CET:
  HTTP 200, model exact, finish `stop`, 44.6 s latency
  (first-request latency; scored requests were much faster,
  see below), 173 input / 166 output / 339 total tokens,
  reasoning 147, cost 0. Answer correct (2.7 meters, cited).
  No retry was needed.

## Scored results: 32 requests, 2 runs × 8 cases × 2 approaches

Scoring is independent and per the owner rubric (A1 morning
high tide as launch window; A2 Harbor office safe; A3 stated
total 300; A4 2.7 meters as current tide; A5 BOTH 120 and 180
seats; A11 45 seconds; A12 12 tonnes; A13 fourteen/14 tide
charts). A stated fact in reasoning alone does not count; all
answers below were read in full and state their facts as
conclusions. Citation support is marker presence (`[`) only,
the same proxy as the Qwen runs, not a verified citation to
the cited memory. All 32 responses: HTTP 200, finish `stop`,
model exact, zero errors, cost 0, cached tokens 0 on every
response (cached counters reported separately, never inferred).

| approach | correct (of 16) | input | output | of which reasoning | visible output | total | per correct |
|---|---|---|---|---|---|---|---|
| ANANSI | 16 | 9,802 | 2,336 | 2,011 | 325 | 12,138 | 758.6 |
| Naive baseline | 16 | 15,584 | 1,990 | 1,796 | 194 | 17,574 | 1,098.4 |

- Per run: ANANSI 8/8 in each run (input 4,901, total 6,069
  in each run; input tokens identical across runs for every
  case/approach, as expected for deterministic tokenization).
  Naive 8/8 in each run (input 7,792; totals 8,769 run 1 and
  8,805 run 2).
- Input savings: ANANSI 37.1% fewer input tokens than naive
  (9,802 vs 15,584). Total savings: 30.9% fewer total tokens
  (12,138 vs 17,574). Per correct answer (each approach's own
  count, failed attempts included — here there are no failed
  answerable attempts): 758.6 vs 1,098.4, also 30.9% fewer.
- Long/short split (per run, input tokens): long cases
  (A2, A11, A12, A13) ANANSI 4,265 vs naive 7,201 (−40.8%);
  short cases (A1, A3, A4, A5) ANANSI 636 vs naive 591
  (+7.6%, ANANSI more — the same framing-overhead pattern as
  the Qwen runs: when evidence is short, ANANSI's fixed header
  costs more than it saves).
- Unfavorable detail, kept in: ANANSI's output tokens are
  HIGHER than naive (2,336 vs 1,990, +17.4%), driven by
  reasoning tokens (2,011 vs 1,796). Savings are input-driven;
  total is still lower because input savings dominate.
  Reasoning is 86% of ANANSI's output tokens and 90% of
  naive's; visible answers are short for both (325 vs 194
  tokens across 16 answers each).
- Citation support: 16/16 for both approaches (every answer
  carries at least one marker).
- Latency (scored requests only, smoke excluded): ANANSI mean
  5,226 ms, median 3,968 ms, range 2,088–15,686 ms; naive mean
  6,633 ms, median 4,250 ms, range 2,759–27,734 ms. Latency was
  not a scored criterion and varies with provider load; the
  smoke request's 44.6 s is a cold-start outlier, not part of
  these figures.
- Failures: none among the 8 answerable cases for either
  approach in either run. A6 is the only non-answered case,
  by design (correct-insufficient, zero calls).

## What this does and does not establish

- Established (within-model, same prompts and settings):
  on repaired-tree prompts, Nemotron answers all 8 answerable
  cases correctly for BOTH approaches, and ANANSI uses 30.9%
  fewer total tokens (37.1% fewer input tokens) for the same
  perfect accuracy. ANANSI's measured savings remain on the
  larger model; they are token savings, not an accuracy
  advantage here, because naive also reaches 8/8.
- Larger-model accuracy vs Qwen: Nemotron (8/8 both
  approaches) is more accurate than Qwen 3B on repaired-tree
  prompts (ANANSI 6/8 answerable in repaired run 1, with A3
  truncated and A5 wrong), and than Qwen 3B baseline
  (5/8). But this is NOT a controlled model-size experiment:
  model, tree version (repaired vs original prompts),
  settings (max_tokens 1024 vs 96, temperature 0 vs Ollama
  defaults, chat vs generate), and answer verbosity all differ.
  The controlled repair-effect evidence remains the Qwen
  before/after comparison on identical settings (repaired run
  1: 7/9 including A6, vs baseline text-verified 6/9; run 2
  pending). Do not cite Nemotron-vs-Qwen accuracy as a pure
  model-size effect.
- Estimates kept separate: prompt chars/4 estimates for these
  prompts (ANANSI primary 5,320 vs naive 8,694 per run, from
  the harness) are NOT mixed into the provider-reported totals
  above. End-to-end ANANSI provider usage is not recorded by
  Ask itself; these figures are direct prompt measurements
  through OpenRouter, not ANANSI's internal manifest, and are
  reported as such.

## Evidence (outside the repo, preserved alongside Qwen evidence)

- `verify-benchmark-2026-10-28/openrouter-nemotron/openrouter-run1.jsonl`,
  `openrouter-run2.jsonl` (per-request: model, usage,
  reasoning, cost, finish, latency, errors).
- `verify-benchmark-2026-10-28/openrouter-nemotron/answers/`
  (32 response texts + smoke response).
- `verify-benchmark-2026-10-28/openrouter-nemotron/full-run.log`.
- Prompt files: `prompts-openrouter-repaired/` (shared with
  the Gemma attempt; repaired tree, frozen corpus).
