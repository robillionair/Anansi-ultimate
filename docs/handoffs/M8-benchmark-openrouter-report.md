# M8 benchmark: OpenRouter Gemma experiment — status report (2026-10-28)

Public-safe. Synthetic corpus only. No credentials, no account
identifiers, no Authorization headers, no personal memories, no
Sealed data, no private source code in this report or its evidence.

## Status: verification complete, inference blocked by upstream rate limit

No scored OpenRouter request completed. Zero of the planned scored
requests ran. Nothing was published, pushed, deployed, or paid for.

## What was verified before any inference

- Provider: OpenRouter, chat completions API.
- Model (exact, as authorized): `google/gemma-4-31b-it:free`.
- Model found in the public model list: yes.
- Pricing, both input and output: `0` (zero). No paid inference.
- Context length: 262,144 tokens. The largest complete prompt in
  this experiment is the naive A2 prompt (13,734 chars, about
  3,000 tokens by the earlier direct measurement), so both
  approaches' complete prompts fit with no truncation and no
  silent prompt shortening.
- Key check (spending fields only): usage `0`, spending cap `10`,
  free-tier flag true. The spending fields do not report the daily
  free-request count; the documented free-tier cap for accounts
  without $10 of lifetime credits is 50 free-model requests per
  day and 20 per minute (docs, not the key endpoint). The plan
  below fits that cap if no prior free requests were used today,
  but the key endpoint cannot confirm the remaining count, so
  this report does not claim a confirmed remaining allowance.
- Fallback/substitution controls in the request: exact model id,
  provider `allow_fallbacks: false`, no other model, no other key.
  Any returned model differing from the requested id would stop
  the run (the runner enforces this).

## Prompts: repaired tree, frozen corpus, preserved

- Source tree: benchmark branch tip `03c0898d` (repairs 1–3
  committed: migration WAL-switch tolerance, A12 relevance
  ordering, A1/A3 retrieval softening). This OpenRouter run is a
  repaired-tree experiment, not a baseline-tree experiment.
- Prompt generation: fake-mode harness run on the repaired tree
  with prompt dumps to
  `verify-benchmark-2026-10-28/prompts-openrouter-repaired/`
  (outside the repo, preserved alongside the Qwen evidence).
- Cases: the frozen nine-case corpus (A1, A2, A3, A4, A5, A6,
  A11, A12, A13). Prompt files exist for the 8 answerable cases
  × 2 approaches = 16 files. A6 has no prompt for either
  approach by design (no evidence exists; ANANSI correctly makes
  zero provider calls), so the scored inference plan is 8 × 2 ×
  2 runs = 32 requests, plus at most 1 smoke test = 33 total,
  within the authorized 36 + 1. A6 remains in the denominator as
  correct-insufficient with zero calls for both approaches; it
  is never counted as a saving and never sent as a prompt.
- Prompt sizes (chars): ANANSI A1 592, A2 4640, A3 636, A4 708,
  A5 670, A11 4635, A12 4721, A13 4684; naive A1 575, A2 13734,
  A3 580, A4 679, A5 606, A11 6308, A12 7320, A13 4966.
- Canary check: no Sealed canary and no cross-workspace canary
  in any prompt file.
- Sending format (planned, not yet executed): each prompt sent
  verbatim as a single user message, no system message added,
  temperature 0, max_tokens 256, usage reporting on. Identical
  settings for both approaches. Reasoning-token usage, finish
  reasons, latency, and errors were to be captured per request
  from provider-reported usage.

## Smoke test result: HTTP 429, upstream shared pool

- Smoke request: ANANSI A4 prompt (708 chars), the short
  tide-revision case, sent once at 2026-10-28 ~22:14 CET.
- Result: HTTP 429 in 470 ms, no answer, no usage tokens, no
  model returned. The provider error states the model
  `google/gemma-4-31b-it:free` is temporarily rate-limited
  upstream (provider Google AI Studio, limit source
  "upstream_provider_shared_pool"), with a hint to retry shortly,
  add a provider key, or route to another provider.
- This is an upstream shared-pool limit, not the account daily
  cap and not a billing failure. No scored request followed.
- Action taken, per the owner's instruction: stopped
  immediately, no retry loop (failed 429s can still consume
  daily quota), no key switch, no model switch, no provider
  rerouting, no paid fallback. Partial evidence preserved:
  `verify-benchmark-2026-10-28/openrouter/openrouter-smoke.jsonl`
  and the smoke response/headers files. The account identifier
  in the raw smoke response was removed from the evidence copy;
  the error details (message, code, provider, limit source) are
  preserved.

## What this does and does not establish

- Established: model identity, zero pricing, context capacity,
  prompt set, sending settings, and the current upstream block.
- NOT established: Gemma accuracy on any case, ANANSI vs naive
  token usage on Gemma, tokens per correct answer, citation
  support, latency, or whether the larger model improves
  accuracy or preserves savings. No such claim is made here.
- The Qwen local results (baseline and repaired-tree comparison)
  are unaffected and remain separate. The pending Qwen
  before/after repair comparison and Repair 4 (browser mode)
  are still open and are not replaced by this experiment.

## What would unblock the scored run

- A later retry of the same smoke request (same key, same
  model, same settings) after the upstream shared pool clears,
  or on a later day if the daily cap is the binding limit.
  Any retry needs the owner's go-ahead, because each attempt
  consumes free-tier allowance whether it succeeds or 429s.
- If the retry succeeds with the requested model returned and
  no substitution, the planned run is 32 scored requests in
  fixed order (run 1 then run 2; per run, cases A1, A2, A3, A4,
  A5, A11, A12, A13; per case, ANANSI then naive), with a short
  pause between requests, incremental JSONL results, and
  independent rubric scoring (A5 requires both 120 and 180
  seats; each approach's tokens-per-correct uses its own
  correct count; failed attempts stay in totals; successful-case
  savings only as a labelled secondary breakdown; estimates
  kept separate from provider-reported usage).
