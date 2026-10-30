# Completed free-model comparison — accurate record (benchmark round finished)

This record closes the benchmark round. No new benchmark campaign is
started here. The measurement below is a frozen-prompt replay, not an
end-to-end app benchmark, not a general accuracy claim, not a
commercial superiority claim, not a latency improvement, and not a
production certification.

## Source package (verified, not copied into Git)

- Supplied package: `anansi-free-model-comparison-2026-10-30.zip`,
  SHA-256
  `d5069593c17cfc5f7239d0f27a2fb6d10d1375bb3481bacf21023e19f7acc347`.
  Retained at `~/workspace/user/files/`; raw payloads, responses and
  the request ledger are NOT copied into Git — this record links
  the evidence by name and hash instead.
- Package verification performed on receipt: all 83 MANIFEST file
  hashes match (none missing or mismatched); enclosed bundle
  SHA-256 `9bba632e7ad3ccc776b10ac994b75dc6dacbddafb728df0101144aa8d9db14ad`
  verifies and contains exactly
  `c7327bb7e486b0b43d4c8b12e50fef625ae78705`
  (`refs/remotes/muse/ask-a6-repair`), the baseline of this branch;
  `requests.jsonl` holds 30 lines; run log ends `COMPLETE: 30
  scored requests`; credentials sweep clean.
- Frozen fixture SHA-256
  `738ecc96a2e11dd88f8a8a4f9a0c6e94b6d7bc1f9c8c3d01b46a77aecd1f836e`
  (matches the frozen dir used by earlier rounds).

## Model and run conditions

- Model: `dots-studio/dots-3-note-preview:free`, canonical slug
  `dots-studio/dots-3-note-preview-20260813`, open-weight
  mixture-of-experts (16B active / 280B total per provider
  metadata), context length 512,000, pricing prompt 0 /
  completion 0, metadata expiration 2026-12-31.
- Selection: chosen BEFORE scoring as the first eligible
  alternate candidate to pass the exact-JSON smoke. Gemma
  returned 429; Inkling returned a harness-only 403; the previous
  Nemotron attempt returned provider overload. None was scored.
  No model was selected based on scored quality.
- Run: 30 scored requests (10 batches x 3 approaches, 10
  questions per batch, rotated order), one repeat, temperature 0,
  max output 4096, no fallback, no retries. Finish reasons:
  `stop` x 30. Zero malformed or truncated outputs. The model ID
  alone changed from the original Nemotron freeze; all question
  prompts and generation settings were retained.
- Scoring: automatic fact matches plus strict visible-evidence /
  citation checks (the existing scorer), NOT independent semantic
  accuracy grades. Missing, truncated and malformed outputs stay
  in the denominator. Audit: gold arithmetic, source-specific
  fact spans, all correct citations, all 30 inter-approach
  disagreements and all 30 true-insufficient responses were
  cross-checked offline; no scores were overridden. That audit
  is not independent human adjudication.

## Results (provider-reported usage; verified against scores.json)

| Approach | Fact /100 | Grounded /100 | Input tokens | Completion tokens | Total tokens | Reported cost |
|---|---:|---:|---:|---:|---:|---:|
| ANANSI | 100 | 100 | 40,750 | 25,020 | 65,770 | $0 |
| Basic retrieval | 70 | 70 | 47,552 | 24,193 | 71,745 | $0 |
| Full context | 100 | 100 | 210,673 | 21,648 | 232,321 | $0 |

- ANANSI vs full context: **71.69% fewer reported total tokens**
  (65,770 vs 232,321) and 80.66% fewer input tokens. Both
  percentages recomputed from the totals above.
- ANANSI vs basic: 8.33% fewer total tokens, 14.30% fewer input
  tokens, with 100 vs 70 task matches.
- Sensitivity: excluding the batch with the largest reported
  total saving (batch 2, ten questions), the ANANSI-vs-full
  total saving is 71.02% — independently reproduced from
  `requests.jsonl` (excl. b02: ANANSI 60,005 vs full 207,078).
- Abstention: all three approaches correctly abstained on all
  10 genuinely unanswerable cases (10/10 each). On answerable
  items, grounded results are ANANSI 90/90, basic 60/90, full
  90/90. Basic's 30 answerable misses are retrieval omissions
  (its supplied context lacked required evidence), not
  hallucinated answers.
- By category (grounded): single_fact ANANSI 40/40, basic 27/40,
  full 40/40; paraphrase 20/20, 14/20, 20/20; arithmetic_sum
  10/10, 7/10, 10/10; arithmetic_difference 10/10, 6/10, 10/10;
  unresolved_conflict 10/10, 6/10, 10/10; insufficient 10/10
  all three.
- Offline context coverage (from the supplied report): required
  fact spans present on answerable cases ANANSI 90/90, basic
  60/90, full 90/90; prompt characters (not tokens) ANANSI
  158,999, basic 196,512, full 947,475.

## Unfavorable findings (retained, not softened)

- ANANSI generated MORE completion tokens than full context
  (25,020 vs 21,648), including more reported reasoning tokens
  (18,149 vs 15,271; basic 18,022). Reasoning counts are a
  reported subset of completion usage, not extra tokens to add
  to totals. All scored requests had internally consistent
  counters; reasoning-greater-than-completion anomalies among
  scored requests: 0.
- Total request time: ANANSI 248.861 s, basic 254.109 s, full
  232.719 s. This run does NOT show a latency improvement over
  full context. Network/provider effects and single repetitions
  preclude speed generalization.
- The successful unscored smoke anomalously reported 100
  reasoning tokens within 94 completion tokens. That anomaly is
  retained in the raw smoke response, excluded from scored
  totals, and was not silently corrected.

## Limits (retained)

- Saved-prompt cloud replay only: not end-to-end desktop
  inference, ingestion, hybrid search, or production use.
- 100 distinct synthetic question instances from 10 repeated
  templates; synthetic formulaic names, ASCII text, single
  revisions, six documents per independent corpus, one free
  preview model, one run, no semantic embeddings.
- Document length and fact position vary but are not fully
  crossed within each question template; do not infer their
  independent causal effects.
- Basic retrieval is a named simple baseline, not a commercial
  product or a tuned state-of-the-art RAG system.
- Full context includes irrelevant eligible documents; a
  reduction versus it is not proof of superiority over
  efficient retrieval, and establishes no superiority over
  Microsoft, Anthropic, or other products.
- This full-context baseline differs from the earlier
  eight-question pilot's full bodies of cited memories. Do not
  present the larger percentage here as an algorithm
  improvement over that pilot.
- Batch usage cannot be allocated to individual questions or
  categories. Character counts are not tokenizer measurements.
- Per-case zero-evidence app requests would avoid inference;
  this replay still evaluates all question items in shared
  batches.
- No paid-cost saving is measured: all inference was free, so
  no actual money was saved. No significance or generalization
  claim. Previously tuned/repaired fixture results are not
  fresh held-out generalization evidence. No increasing savings
  with repeated app use is established.
- No automatic publication. A live replay requires fresh
  explicit authorization; do not automatically repeat the run.

## What this record does not change

No app code changed for this measurement; no preview was
promoted. Windows runtime improvements, retrieval repairs, the
A6 reporting correction, and existing preview isolation are
preserved on this branch via the `c7327bb7` baseline. The next
work on this branch is M8 Slice 4 (explicitly selected
saved-project input for LEXI-COLLIDER), recorded separately.
