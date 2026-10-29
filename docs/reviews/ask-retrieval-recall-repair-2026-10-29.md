# Ask recall repair — Windows evidence, 2026-10-29

## Result and scope

The measured retrieval defect is repaired on the original 100-case regression
fixture. Implementation `24d3df5c7cf7a90f849f7d16c08c9fe4fd3841dd`, branch
`codex/ask-retrieval-recall`, parent `737d43691`. Final evidence updates change
documentation and a source comment only, not application behavior.

| Local evidence diagnostic | Before | After |
|---|---:|---:|
| Answerable cases with every required fact in the supplied sources | 50/90 | 90/90 |
| Required fact spans present in their correct source | 60/120 | 120/120 |
| Genuinely unanswerable cases returning no evidence | 10/10 | 10/10 |
| Recovered answerable cases | — | 40 |
| Previously complete evidence sets regressed | — | 0 |
| Combined replay prompt bytes, including per-item instructions | 175,658 | 158,999 |
| Maximum evidence bytes in any case | 4,000 limit | 4,000 observed |

The repaired prompts contain 9.48% fewer bytes than the old ANANSI prompts and
83.22% fewer than full context (947,475 bytes). These are UTF-8 byte measurements,
NOT measured model tokens, generation cost, reasoning work, latency or answer
accuracy. There were **zero new cloud calls and zero successful model calls**.
The earlier measured 60/100 ANANSI answer score remains an earlier result; it has
not become a measured 100/100 result. Keep the public efficiency-win claim on hold
until an explicitly scoped model rerun verifies quality and actual usage.

## Repair

Ask's retained question words were all mandatory in FTS. Ordinary paraphrases
and arithmetic questions contained words their evidence did not, losing candidate
recall before generation. Ask now uses an internal OR candidate query with existing
BM25/hybrid ranking. The literal parser still enforces bounds and escapes operator
text. Exact Search and explicit context compilation retain AND semantics.

Ask filters workspace, current revision, privacy, lifecycle, expiry and kind before
the 100-row candidate cut, retaining canonical publication checks and body hashes.
The compiler offers each ranked memory's best chunk before further chunks from
any one memory. Extra lexical chunks without query overlap are omitted; title-only
matches retain a body chunk, and semantic candidates retain their evidence path.
Disclosed source identities now equal the sources actually included in chunks.
No model, dictionary of benchmark answers/synonyms, schema or contract was added.

## Independent checks within this review

The original fixture, gold facts, natural questions, strata and both comparison
baselines are byte-for-byte/equality checked against the new preparation. Only
ANANSI's approach changes. A second calculation, separate from the comparison
script, verifies each source chunk occurs in its canonical source body, its memory,
workspace, revision and body-hash identity, the 4,000-byte cap, all 120 required
fact spans and citation/disclosure equality. This is a second local calculation by
the integrator, not an external human accuracy audit.

Original fixture SHA-256:
`4e1b4d9503316fd675c1129ba33d07419043656fcf51612e1792b45034c51834`.

Repaired fixture SHA-256:
`738ecc96a2e11dd88f8a8a4f9a0c6e94b6d7bc1f9c8c3d01b46a77aecd1f836e`.

Category coverage after repair: single facts 40/40; paraphrases 20/20; sums 10/10;
differences 10/10; conflicting sources 10/10. Both operands/conflicting facts must
be present, not just a matching document id. All ten insufficient cases remain
empty. The fixture is now a known regression set, ten repeated templates across
100 instances and six eligible documents each, not a fresh held-out study.

## Windows validation

- Two new public Ask regressions failed before the production change: a fresh
  reworded battery question and a two-note credit calculation lost evidence.
  Both pass after repair, without adding these words or answers to production.
- 82 Rust tests pass: core library 56; Ask 5; new recall 2; streaming 1; body
  search 1; context 10; hybrid 6; retrieval evaluation 1. These include corruption,
  privacy, exact Search, Unicode, budget, citations and cancellation regressions.
- A new unit fixture places 105 records in each of six excluded classes ahead
  of two eligible partial/strict matches, proving exclusion before candidate cap.
- Unchanged large-fixture test 2/2 passes, all 100 public Ask calls, in 1,569.97 s.
  This includes setup, Windows permission checks, baselines and cleanup. API build
  and the short browser check overlapped early preparation; no isolated latency
  or speedup claim is made.
- Eight Python runner/scorer regressions pass. Strict core all-target Clippy,
  formatting, contracts drift and TypeScript checks pass. API and web builds pass.
- Canonical browser Ask workflow 1/1 passes in 37.4 s (41.8 s runner total): missing
  evidence, offline grounding, citations/disclosure and the new natural-language
  title-only query. Real-model mode is disabled; its displayed default model is
  a form value, not evidence of an inference run. No external browser requests.
  Shared application lease acquired and released; synthetic roots only.

Diagnostic command corrections: unittest discovery initially collected zero tests;
running the test file directly produced the reported 8/8. An initial Vite command
used the wrong monorepo module path; the actual successful build ran from apps/web
with its own node_modules. Neither attempt is counted as a successful check.

## Remaining limits and next verification

Partial lexical matches can still be irrelevant; absent vocabulary cannot always
be recovered without explicitly configured semantic retrieval. Top-ten candidates
and bounded chunk allocation can omit evidence in larger or differently structured
corpora. Some common query words still include irrelevant text (visible in the
arithmetic-difference and conflict categories). No new speed or general-language
quality claim, native preview promotion, G1 audit, sandbox or CI certification.

No pushes, master merges, preview changes, real credentials or private user data.
Before a public quality/savings claim, authorize a bounded model replay and use
fresh held-out questions for generalization. Do not automatically spend remaining
free-tier allowance or rerun the original cloud harness.

Artifacts: `E:/Anansi/benchmarks/ask-recall-repair-2026-10-29`, including repaired
immutable cases, comparison, separate audit, logs and browser screenshot. Original
benchmark data remains unchanged in its original directory.

## Reproduction

Use a new output JSON path on E: for Windows preparation; never overwrite either
frozen fixture. Set TEMP/TMP and CARGO_TARGET_DIR to E: as in existing build policy.

```powershell
node scripts/cargo.mjs test -p anansi-core --locked --lib --test ask_recall --test ask_anansi --test ask_streaming --test context_compiler --test hybrid_retrieval --test body_search --test retrieval_evaluation -- --test-threads=1
node scripts/cargo.mjs clippy -p anansi-core --all-targets --locked -- -D warnings
$env:ANANSI_LARGE_OUT = 'E:\Anansi\benchmarks\new-review\cases.json'
node scripts/cargo.mjs test -p anansi-core --locked --test benchmark_large -- --include-ignored --nocapture --test-threads=1
python scripts/compare-retrieval-repair.py before.json after.json comparison.json
```

Create the new-review parent first. The offline comparator works cross-platform on
the delivered JSON. Browser reproduction uses the standard application-test lease,
built web/API, canonical Playwright config and ANANSI_REAL_MODEL=0.
