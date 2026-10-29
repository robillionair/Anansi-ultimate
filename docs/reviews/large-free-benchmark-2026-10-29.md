# Larger free-model benchmark — measured result, 2026-10-29

## Verdict

The token reduction is substantial, but quality is not preserved in this lexical-only
configuration. Do not publish an unqualified efficiency-win headline. The next
engineering priority is candidate retrieval, not a larger answer model.

| Approach | Supported correct outcomes /100 | Input tokens | Completion tokens | Reported total |
|---|---:|---:|---:|---:|
| ANANSI lexical Ask | 60 | 46,339 | 12,467 | 58,806 |
| Basic OR/BM25, top 3, first 1,000 characters | 70 | 50,479 | 13,305 | 63,784 |
| Full eligible context | 100 | 228,145 | 16,277 | 244,422 |

ANANSI used 79.69% fewer API-reported input tokens and 75.94% fewer reported total
tokens than full context. Against basic retrieval it used 8.20% fewer input tokens
and 7.80% fewer reported total tokens, while losing ten supported outcomes.
Removing the largest-saving ten-question batch leaves 75.41% lower reported totals
versus full context. That sensitivity result does not remove the quality gap.

Each approach correctly refused the ten genuinely unanswerable questions. On the
90 answerable questions, ANANSI answered 50 correctly, basic 60, and full context
90. ANANSI's other 40 outcomes were refusals despite relevant evidence existing
in the corpus, rather than invented factual answers. Basic had 30 such refusals.
Every recorded outcome matched the availability of all required fact spans in its
supplied context; no model error was observed when those facts were present.
These are strict exact-value/format and citation-support diagnostics, not an
independent human assessment of general language understanding.

## What was actually run

- Frozen source/protocol commit: `21b1afb0ba1a7969825d39c61a3745bfc4c29e40`.
- 100 unique synthetic question instances across ten repeated templates and six
  categories; 100 independent workspaces, six eligible documents each. No real
  owner memories, credentials or personal documents were supplied as context.
- Real `Core::ask_anansi` on unchanged natural questions, existing normalization,
  canonical storage reads, FTS rebuild and default 4,000-byte evidence budget.
  No embeddings; an explicitly refused loopback endpoint kept preparation offline.
  Bulk SQL/private-ACL fixture setup is not an ingestion-workflow certification.
- Thirty scored OpenRouter requests, ten independent question items per request,
  three approaches, one repeat; plus one successful unscored smoke. Exact requested
  model `nvidia/nemotron-3-ultra-550b-a55b:free`, temperature 0, output cap 4,096,
  default reasoning, fallback disabled; approach order rotated by batch.
- All 30 scored calls: HTTP 200, finish `stop`, parseable JSON, 30 unique generation
  ids, 300 answer items. No retries, substituted model, truncation or omitted case.
- All inference responses reported cost zero; the postflight key endpoint also
  reported monetary usage zero. Detailed quota snapshots are retained as snapshots,
  not substituted for the actual request ledger.

Frozen fixture SHA-256:
`4e1b4d9503316fd675c1129ba33d07419043656fcf51612e1792b45034c51834`.
Protocol file hashes were checked unchanged during inference. Gold answers and
grading metadata were not transmitted. Outgoing contexts passed Sealed and
foreign-workspace canary checks.

## Accounting verification

Independently resummed all 30 raw responses, checked input + completion = total,
unique ids, model identity, finish reason, zero cost, answer ids and fixture hash.
All reported cached-token counters were zero. No reasoning > completion anomaly
appeared in these 30 scored responses; the previous pilot's eight inconsistent
records remain historical unresolved evidence, not retrospectively fixed.

Three read-only generation-statistics queries matched their native input and output
counts to the corresponding chat responses. The statistics expose a dated canonical
model id ending `550b-a55b-20260604:free` under the requested alias, with provider
Nvidia. Their separate normalized `tokens_prompt`/`tokens_completion` differ from
`native_tokens_*` and were not mixed into these totals. This is a same-service
cross-check, not independent provider replication or a measurement of GPU work.
See [OpenRouter usage definitions](https://openrouter.ai/docs/api_reference/overview)
and [generation statistics](https://openrouter.ai/docs/api/api-reference/generations/get-request-&-usage-metadata-for-a-generation).

## Interpretation and limits

`ask_search_query` retains content terms and adds prefix stemming; body search
joins terms with AND. In this fixture, questions about visitors/accommodation,
closing doors and arithmetic contain words absent from the evidence phrasing.
All 20 paraphrase and 20 arithmetic cases lose candidate recall in lexical Ask.
The full-context control answers them, so a larger generator cannot recover facts
that never reach its prompt. No retrieval repair was made during this measurement.

The basic baseline's prefix cutoff loses middle/late facts; it is deliberately a
simple named baseline, not a tuned RAG system or commercial competitor. Full
context includes irrelevant eligible documents; the previous pilot instead sent
full bodies of cited memories. Do not present 79.69% versus the old 37.1% as an
algorithm improvement. The underlying application code is unchanged.

Document length and fact position are varied but not fully crossed. Long bodies
contain repetitive synthetic filler, supporting documents occupy fixed corpus
slots, and there are only ten base question templates. There is no large-corpus,
hybrid-search, multilingual, revision-history or end-to-end desktop claim here.
Provider usage belongs to batches, not individual questions or categories.
Offline preparation took 1,336.10 seconds including fixture setup, baseline reads,
native permission checks and cleanup; it does not isolate query latency. Recorded
HTTP replay latencies are not product speedup measurements. No paid-cost saving,
statistical significance or production certification is established.

## Verification and handoff

Rust fixture checks 2/2; Python runner/scoring regressions 8/8; strict harness
Clippy, formatting, documentation consistency and Git diff validation passed.
Three Windows fixture ACL setup failures before inference are retained. Their fixes
changed test setup only; production permission/integrity checks were preserved.
No new application behavior, preview promotion, pushes or remote CI.

Artifacts: `E:/Anansi/benchmarks/large-free-2026-10-29`, including frozen cases,
all payloads/responses, request ledger, scores, generated report, source hashes,
raw-response arithmetic audit, sampled native stats and redacted budget snapshots.
Offline scoring is cross-platform Python. The explicit fixture-preparation test
currently requires an E: output path and is documented for this Windows run;
Linux reviewers can audit/replay the delivered frozen prompts without regenerating.
Live replay requires fresh authorization; do not automatically repeat it.

Next bounded implementation work should improve general candidate recall while
preserving current privacy, workspace, revision, integrity and budgeting boundaries.
Use this fixture as a regression set; any claim of generalization needs new held-out
questions, not only the now-visible ten templates. Existing native, audit, sandbox
and production CI gates remain open.
