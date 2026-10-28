# Benchmark: ANANSI token consumption and context efficiency — design

Branch: `muse/m8-benchmark-tokens`, from Windows-reviewed checkpoint
`codex/review-m8-slice3` at exactly
`df8072595e672ead29ad0df82393c31de7b6f7d8` (includes the Slice 3
consent/provenance fix `f54e466`, the receipt-integrity fix `5b3cf958f`,
and the reviewed Slice 3 tip `776259a4` as ancestors). No production
behavior changes in this task except benchmark harness code and tests.
No optimization work: measure first, record opportunities only.

## Environment finding (shapes the whole benchmark)

This Linux VM has **no suitable local model**: no `ollama` binary, no
listening loopback inference endpoint, no installed model files, and no
matching tokenizer package (no Python `tiktoken`/`tokenizers`/
`transformers`, no Node `tiktoken`, no tokenizer dependency in the
workspace). No paid/cloud inference, real credentials, model downloads
or new external services are authorized. Therefore:

- Linux runs measure the **request side exactly**: wire request bytes,
  prompt characters, ANANSI's own estimate, manifest values, retrieval
  latency, and wire-content invariants. Output tokens and
  provider-reported usage are **unavailable** on Linux and are recorded
  as `null`, never inferred from latency and never substituted with
  synthetic-server counters.
- ANANSI's internal estimate is `chars / 4` (ceiling per chunk in the
  compiler, floor on the Ask total). It is a heuristic, not a model
  tokenizer count. All Linux token figures are reported in that unit
  and labelled `estimate_chars_div_4`, alongside exact chars and bytes.
- The harness is packaged so the Windows owner can run the identical
  scenarios against an installed local model: environment variables
  `ANANSI_BENCH_ENDPOINT` (loopback URL) and `ANANSI_BENCH_MODEL`
  select a real endpoint; when set, the harness additionally records
  provider-reported usage wherever ANANSI surfaces it (M5 disclosure
  and ledger; see the Ask gap below). The same corpus, questions and
  baselines run in both modes so the numbers are comparable.

## What already exists (reused, not duplicated)

- Ask ANANSI (`crates/core/src/ask.rs`): question ≤ 500 chars;
  `max_evidence_items` default 5, clamp 1–10; evidence budget
  `min(max_items * 800, 4000)` chars. Context Compiler
  (`crates/core/src/context.rs`): hybrid retrieval, limit 10, lexical
  fallback without an embedding model; sections chunked at 1000 chars,
  budget-enforced with clean truncation, exact-content dedup,
  fail-closed Sealed exclusion; compiled prompt = instruction header +
  `[n] title - heading (line N)` evidence pack + `User Question: …
  Answer:`. Manifest carries `character_count`,
  `estimated_tokens = chars / 4`, citations and the full
  `compiled_prompt`. Insufficient evidence returns an honest answer
  with **zero** provider traffic. The Ask path posts directly to a
  loopback Ollama (`/api/generate`, `num_ctx` 2048, `num_predict` 96,
  `keep_alive` 0) or OpenAI-compatible (`/v1/chat/completions`,
  `max_tokens` 96) endpoint. **The Ask path never parses
  provider-reported usage** — no `eval_count`/`prompt_eval_count`
  handling exists there, so on the Ask path provider-reported usage is
  unavailable by design, not by test setup.
- M5 dispatch (`crates/providers/`): Ollama adapter maps
  `prompt_eval_count` → `input_tokens` and `eval_count` →
  `output_tokens` (`crates/providers/src/providers/ollama.rs`).
  `Usage` has only `input_tokens` + `output_tokens`; **there is no
  cached-token field anywhere in the ledger or disclosure**, so cached
  tokens are reported as not tracked and are never inferred from
  faster responses. `DisclosureRecord` carries provider, model,
  endpoint host, message/segment counts, redaction count,
  `estimated_input_tokens` (chars/4), cost class, `usage` and attempts.
  `providers-usage` reports ledger totals per provider.
- Collider (`crates/core/src/collider.rs`): instruction segment +
  controls segment + verbatim Beam A/B private segments + one private
  segment per resolved memory entry (`Memory context (data) — {title}
  (revision {version}):\n{content}`), in selection order, via M5
  dispatch with `maxOutputTokens` 8192. Slice 3 bounds: ≤ 8 entries,
  ≤ 8000 chars each, ≤ 32000 total. The reviewed fix `f54e466`
  submits already-reviewed references with no pre-start re-read, and
  restore pre-validation now compares version/title/privacy labels
  against source revision metadata, not just hashes.
- Existing fake-provider patterns (`crates/core/tests/ask_anansi.rs`,
  `collider.rs`, `transfer.rs`): loopback TCP servers capturing raw
  request bodies, and the offline-endpoint trick (dead loopback port →
  `AskStatus::Offline` with the full compiled prompt in the manifest).

## Corpus (deterministic, seeded in-test)

One synthetic workspace, fictional "Harbor" domain, seeded identically
on every run:

- Relevant facts: tide observations (revision B then C with a changed
  figure), venue routing notes, mycelium gradient notes, a deployment
  decision. Each question has exactly one necessary fact.
- Irrelevant material: ~30 distractor memories on unrelated topics,
  several with bodies over 1000 chars to exercise chunking and
  truncation, plus near-duplicate bodies to exercise dedup.
- Historical revisions: "Tide notes" revised B → C; Ask questions must
  reflect C (current revision); a saved collider experiment pinned to
  B must still resolve to B (revision accuracy).
- Contradictions: two memories stating different venue capacities for
  the same venue. The harness records how the evidence pack presents
  them (both cited, one cited, neither); it does not adjudicate which
  value is "true".
- Insufficient evidence: questions about topics absent from the
  workspace (e.g. "What is the harbor's deep-water berth fee?") must
  return `InsufficientEvidence` with zero provider hits.
- Sealed material: a Sealed memory containing a canary string that
  must never appear on any wire request.
- Cross-workspace material: a second workspace holding a canary that
  must never appear in the first workspace's requests.

## Scenarios and approaches (identical questions, model, settings, caps)

Ask scenarios (A), each run under three approaches:

- **ANANSI**: `Core::ask_anansi` with default settings
  (`max_evidence_items` 5, budget 4000 chars, `num_predict` 96).
- **B-NAIVE**: the full, untruncated bodies of exactly the memories
  ANANSI cited for that question, plus the same instruction header
  and question, assembled by the harness (no budget, no chunking, no
  dedup). This isolates budgeting/chunking/dedup: same retrieval
  set, different compilation. It is what a simple RAG implementation
  would send for the same hits.
- **B-HISTORY** (multi-turn scenario only): all prior turns of the
  scripted conversation verbatim (questions + answers) plus the new
  question, with no retrieval. This is what a history-based assistant
  would send. ANANSI's per-turn input stays bounded by its evidence
  budget; B-HISTORY grows every turn.

The entire-workspace dump is **deliberately not a baseline**: no
reasonable application sends a whole workspace, and using it would
inflate the comparison. This exclusion is recorded, not hidden.

Ask scenario list:

- A1 single fact, small relevant memory, many distractors.
- A2 fact inside a long memory (> 4000 chars) — exercises truncation.
- A3 fact split across two memories — exercises multi-source citing.
- A4 current-revision fact (B → C) — revision accuracy.
- A5 contradiction (two capacities) — evidence presentation.
- A6 insufficient evidence — zero-traffic honesty.
- A7 repeated question, 3 consecutive calls — repeated-call behavior
  (identical request bytes? retrieval re-run? no caching?).
- A8 six-turn conversation on one topic — per-turn input growth
  (ANANSI vs B-HISTORY).
- A9 budget sweep: `max_evidence_items` 1 / 5 / 10 on the same
  question — budget vs recall tradeoff.

Collider scenarios (C), via `collider-start` on the M5 substrate with
a fake provider returning a fixed valid six-section result:

- C1 beams only (no context) — baseline token floor for a collision.
- C2 beams + 2 selected memories — the Slice 3 token delta.
- C3 consent wire check: review revision B, revise the memory to C
  through another handle, collide with the reviewed references — the
  wire must contain B text and must not contain C text (the reviewed
  `f54e466` behavior). This is a correctness gate, not a saving.

Accounting scenario (M), via the same M5 collider run with a fake
provider returning known usage values (`prompt_eval_count` /
`eval_count`):

- M1 ledger fidelity: `providers-usage` totals and the run
  disclosure's `usage` must equal the fake's reported values exactly,
  for input and output, across repeated runs. Fake values are test
  fixtures for pipeline fidelity only and are **never** reported as
  model measurements.

## Metrics (per request, machine-readable JSON)

`scenario, approach, question_id, status, provider_hits,
request_bytes, prompt_chars, evidence_chars, cited_memory_count,
anansi_estimate_tokens, retrieval_ms, total_ms,
provider_reported_input_tokens, provider_reported_output_tokens,
provider_reported_cached_tokens, ledger_input_tokens,
ledger_output_tokens, wire_checks_passed, notes`.
Rules: provider-reported fields are `null` on Linux Ask runs
(unavailable by design) and on all runs without a real model; cached
tokens are always `null` (not tracked by ANANSI); ledger fields are
`null` on Ask runs (Ask bypasses the M5 ledger) and populated on M5
runs. Output tokens are `null` on Linux; with a real model they come
from the M5 disclosure/ledger, never from the Ask manifest.

## Wire inspection (invariants, failures are defects)

For every captured request body the harness asserts: no Sealed canary;
no cross-workspace canary; no unselected-memory body (collider); no
un-cited memory body beyond the cited set (Ask, except the question
itself); the Ask instruction header (`GROUNDED EVIDENCE PACK`)
appears exactly once; M5 segment texts are not duplicated; collider
consent case contains B and excludes C. Omissions of necessary
evidence are quality failures, not savings: for each Ask scenario the
harness asserts the necessary fact string is present in the compiled
evidence pack (Linux) or the answer cites it (real-model run).

## Quality grading (honest about the fake provider)

On Linux the fake provider returns canned text, so answer-level
semantic grading is impossible without circularity. Linux quality is
therefore graded at the evidence level: necessary fact present in the
compiled pack, correct citations, correct revision, honest
insufficient-evidence status, wire invariants held. Answer-level
grading (expected fact present in the model answer, citation markers
used, contradiction handling, insufficient-evidence wording) runs
only in real-model mode and is recorded separately. The report states
this split plainly.

## Deliverables

- `crates/core/tests/benchmark_tokens.rs`: harness + corpus + JSON
  writer. `scripts/benchmark-tokens.mjs`: reproducible command
  (`node scripts/benchmark-tokens.mjs [--out <path>] [--real]`).
  Results default to `target/benchmark-tokens/results.json` (not
  committed); a committed sample from the Linux run lives in
  `docs/handoffs/M8-benchmark-tokens-results.json` (request-side only,
  with `null` usage fields visible).
- `docs/handoffs/M8-benchmark-tokens-report.md`: totals, savings in
  absolute tokens and percentages, regressions and cases where ANANSI
  uses more tokens, quality tradeoffs, measurement limitations, and
  measured opportunities (no optimizations implemented).
- `docs/handoffs/M8-benchmark-tokens-windows-guide.md`: how the
  Windows owner runs the inference portion against an installed local
  model, including the Ask usage gap to confirm on a real endpoint.
- Any accounting defect found becomes a focused regression test in
  the harness file (or the relevant suite) before delivery.
- Verification before delivery: harness run; `collider`, `transfer`,
  `ask_anansi`, `context_compiler`, `provider_dispatch` suites;
  strict Clippy; `fmt --check`; `validate-docs.mjs`. No e2e changes
  are required (no UI change); the existing canonical browser specs
  are unaffected.

## First-run corrections (2026-10-28, before any reporting)

First run (checkpoint `80779bff`, log `verify-benchmark-2026-10-28/bench1.log`,
33 records): 7 invariant failures, root-caused as follows. No assertion
was weakened and no limit was raised to obtain passes.

1. A1/A3/A9 recall misses are ANANSI behavior, not harness bugs.
   Lexical retrieval ANDs every content word (FTS5, exact tokens, no
   stemming) and `extract_query_terms` does not treat `at` as a
   stopword. A1 fails on `at` (not an FTS token in the evidence body);
   A3 fails on `together` (and on `wings`/`hold` vs `wing`/`holds`).
   These questions are retained as recall probes with an honesty
   invariant only (honest miss + zero provider hits). A10 keyword
   controls (absent word removed) restore recall, proving the mechanism.
2. C2/M1 wire parsing was a harness bug: the M5 Ollama chat wire format
   carries `messages[].content`, not `messages[].segments`. Segments are
   concatenated with no joiner, so content chars equal segment chars
   and the disclosed estimate must equal `ceil(content chars / 4)`
   exactly. Fixed; C2 now verifies 1764 chars → 441 exactly.
3. B-NAIVE dedup correction: chunk-level citations repeat a memory per
   chunk. The first run sent the long A2 body once per chunk (4x),
   inflating the baseline. B-NAIVE now sends each cited body once.
   A2 honest comparison: ANANSI 4640 chars vs naive ~13.4k chars.
4. Defect D1 (accounting disclosure): before repair, a completed M5
   stream run's disclosure carried `usage: null` while the ledger
   recorded the provider-reported 10/40. Root cause: the run disclosure
   is built at stream start and the settle thread writes only to the
   ledger. Repair (separate commit): stream-poll copies each reported
   Usage event into the run disclosure. M1 splits into ledger fidelity
   (asserted throughout) and disclosure agreement (asserted after the
   repair). Before evidence: first-run M1 record (disclosure null,
   ledger 10/40). After evidence: rerun M1 record (disclosure 10/40,
   ledger 10/40).
5. A9 budget sweep uses the answerable tide question: on this corpus
   all three budgets (1/5/10 items) compile the identical 587-char
   prompt, because the matching evidence (~500 chars) fits even the
   smallest budget. Budget binding is demonstrated by A2 instead.

## Machine constraint (2026-10-28)

This Linux VM: 2 CPUs, 8.1 GB RAM with ~0.8 GB available, no GPU,
141 GB free disk, Ubuntu 24.04, network egress to public model sources
confirmed. Model selection is therefore RAM-bound; see the report for
the selected models and the explicit limitation this places on
quality generalization.

## Corpus expansion + A12 budget-truncation probe (2026-10-28)

To stop a single long-memory case (A2) dominating the headline, three
long cases were added with facts at different positions, all with
keyword-control questions (A10 pattern) so they are graded, not probes:
A11 lighthouse log (~5500 chars, fact in a middle chunk), A12
warehouse manifest (~7000 chars, fact in a late chunk), A13 harbor
master office (~5000 chars, fact in the first chunk). Filler vocab
shares no content word with each case's control terms.

A12 failed on its first gated run, and the failure is a product
finding, not a harness bug. Diagnosis: `compile_context` includes a
hit memory's chunks in order until the 4000-char Ask budget is
exhausted; it does not select ranked matching chunks first. The A12
fact sits at ~5900 chars, so the first four filler chunks fill the
budget and the fact chunk is omitted, even though it is the exact
retrieval match (compiled prompt 4664 chars, cited Warehouse manifest
4 times, fact missing). A11 passes because its fact sits at ~2750
chars, inside the budget; A13 passes as a first-chunk case like A2.
The first failing run is preserved (`results-fake-expanded.json`,
41 records, A12 wire_ok=false).

Handling, disclosed rather than hidden: A12 was moved out of the gated
`ask_cases` loop into a dedicated budget-truncation probe section.
Only wire/header/status invariants gate the probe; its evidence miss
is recorded as wire_checks_passed=false and `bench-analyze.py` now
excludes wire_checks_passed=false Ask records from answerable totals
and charts, labelling them quality misses, not savings. Fixing the
compiler to include ranked matching chunks would be an optimization
and is out of scope for this measurement benchmark; moving the fact
earlier would be cherry-picking. Neither was done. Fake expanded
totals after this change: answerable ANANSI est 3802 vs naive 6549
(-41.9%), of which long cases A2+A11+A13 are 3489 vs 6253 (-44.2%)
and short cases A4+A5 are 313 vs 296 (+5.7%, ANANSI more expensive).
The report must show long vs short splits alongside any aggregate,
and must never quote the aggregate without them.

## A12 repair: relevance-ordered chunks within the budget (2026-10-28)

Repair (production change, budget unchanged): `compile_context` now
orders each hit memory's chunks by query relevance before applying
the existing budget — most distinct query terms first, then most
occurrences, then document order — instead of taking chunks in
document order until the budget ran out. Memory order from hybrid
retrieval is unchanged, citations keep their original line numbers,
and the 4000-char Ask budget is unchanged. A12 is re-gated as an
ordinary ask case (same question, same fact position, same budget);
the probe section was removed. Fake evidence: A12 wire_ok false →
true, est 1166 → 1180 (fact chunk plus filler within budget, 5
citations instead of 4); A11/A13 est unchanged (1158/1171).
Regressions in `context_compiler`: late-fact, middle-fact, and a
distractor test (partial-term chunk with many occurrences ranks
below the full-fact chunk, proving distinct terms dominate).
The repair does not change retrieval itself: A1/A3 lexical misses
remain, addressed separately by the retrieval repair.

## A1/A3 retrieval repair: Ask-scoped query softening (2026-10-28)

Root causes, both in Ask's retrieval query only (global search
semantics unchanged): the AND query required every question word,
including prepositions/aggregation words absent from evidence
("at" in A1; "many", "do", "together" in A3), and exact tokens
("wings" vs body "wing", "hold" vs "holds"). Repair, all in
`ask.rs` except where noted: expanded Ask stopwords (at, by, from,
into, over, under, after, before, during, between, many, much,
total, together, each, per, do, does); each remaining term is
stemmed (trailing "es" then "s") and prefix-marked ("wing"*,
"hold"*) — a recall superset, so no exact match is lost; hybrid
search strips '*' before semantic embedding so FTS syntax never
pollutes the semantic query; chunk scoring matches terms
prefix-tolerantly in either direction so stemmed queries still
rank fact chunks first. A unit test pins the A1/A3 queries exactly
(it caught "do", which the first list missed). Fake evidence: A1
and A3 move from InsufficientEvidence to Answered with correct
citations (A1: Harbor Pier log, est 148; A3: both wings, est 159).
No assertion weakened; body_search/hybrid/title_search suites
unchanged and passing.

## Ask browser real-model mode parameterization (Repair 4, 2026-10-28)

`tests/e2e/ask_anansi.spec.ts` no longer hardcodes
`qwen2.5:0.5b`: the real-model branch reads
`ANANSI_REAL_MODEL_TAG` and defaults to the selected installed
model `qwen2.5:3b`. The manifest assertion still checks the
manifest shows the model actually requested (now via the
variable, not a literal), the answer assertion still requires
/SQLite/i, and the citation and cancel assertions are
unchanged. Exercised on this machine with
`ANANSI_REAL_MODEL=1` (default 3b, Ollama serve already
running): 1 passed in 25.0s, including the real-model answer,
manifest, and cancel flow. The mode exclusion recorded in the
browser inventory is therefore closed for Linux.
