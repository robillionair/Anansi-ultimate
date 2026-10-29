# A6 benchmark repair — acceptance before implementation (2026-10-30)

Baseline: `muse/ask-verified-integration` at
`151565d7f7e6006f4822e9681bea524522472311`, preserving
`532afccf` and `4ef90406d`. No cloud calls, credentials,
model downloads, paid inference, generated-code execution,
pushes, master merges, remote CI, preview promotion, or
changes to the owner's running application.

## 1. Reproduction (preserved)

On the exact baseline,
`node scripts/cargo.mjs test -p anansi-core --locked --test benchmark_tokens`
fails with exit 101, panic at
`crates/core/tests/benchmark_tokens.rs:1929`, invariant
"A6 / anansi: insufficient-evidence honesty / zero hits",
line `A6 / anansi: status=Answered req_bytes=None
prompt_chars=Some(0) est_tokens=Some(0) compile_ms=Some(40)
total_ms=Some(34) wire_ok=false`. The full log is preserved
in the delivery evidence (`before/benchmark_tokens.log`).
The `prompt_chars=Some(0)` in that line is the A6 record's
hardcoded constant, not the compiled prompt; the real signal
is `status=Answered` with `wire_ok=false`.

## 2. What the code actually does (read, not assumed)

- Question: "What is the Harbor deep-water berth fee?"
  Content terms after stopword removal and stemming:
  `harbor* deep* water* berth* fee*`, OR-joined (the
  repaired Ask candidate mode). The Harbor corpus contains
  no "berth" and no fee fact anywhere; "harbor"/"water"
  match Harbor Pier records.
- `Core::ask_anansi` returns `InsufficientEvidence` if and
  only if citations are empty (`crates/core/src/ask.rs`,
  "Truthful Insufficient Evidence Handling"). With any
  citation it compiles a prompt and calls the provider:
  a refused loopback yields `Offline` with the compiled
  evidence pack; a responding provider yields `Answered`
  for any non-empty response.
- The compiler's fixed preamble is unconditional
  (`crates/core/src/context.rs`): "If the evidence is
  insufficient to answer the question, state truthfully
  that evidence is missing." Disclosure lists only
  actually included sources, not merely read candidates.
- The benchmark's fake provider returns the canned text
  "Canned grounded answer citing [1]." unconditionally;
  its own comment states the canned answer is plumbing
  only and quality on Linux is graded at the evidence
  level. In fake mode, `Answered` is therefore guaranteed
  whenever provider traffic happens; it says nothing about
  model honesty.
- Probe (temporary test, deleted before this commit) on a
  minimal Harbor corpus with a dead endpoint confirmed
  the model-independent outcome: status `Offline`, 2
  partial citations (Harbor Pier log, Tide notes), no
  cited chunk mentioning "berth", disclosure exactly
  equal to the cited set, compiled prompt (703 chars)
  containing the abstention sentence verbatim.

## 3. Defect locus: the assertion, not the fixture or production

- Fixture: correct. The fact is genuinely absent and the
  shared vocabulary is the deliberate point of the case.
- Production retrieval: correct per the frozen repair
  acceptance (`docs/handoffs/Ask-retrieval-recall-repair.md`):
  OR candidates ranked by BM25/hybrid, eligibility
  before the candidate cut, "retrieval is not proof a
  question can be answered. Unsupported questions must
  retain the instruction to abstain", and "partial
  matches may be irrelevant."
- Production status logic: correct per its own rule
  (empty citations imply insufficient evidence; a
  refused loopback honestly reports `Offline` and never
  presents an answer without a model).
- The defect is A6's assertion:
  `status == "InsufficientEvidence" && hits == 0`. It
  encodes the pre-repair AND-retrieval side effect (no
  chunk contains all terms, so the result was empty and
  no provider call happened) as if it were the honesty
  requirement. Under repaired retrieval that proxy is
  false even when honesty holds. It conflates retrieval
  emptiness, provider-traffic gating, and honest
  abstention; only abstention is the requirement
  (AN-006 "truthful missing-evidence handling";
  MASTER_SPEC "When evidence is insufficient, say so").

## 4. Correction (test-side re-anchoring, not a relaxation)

No production code changes. The case, the question, and
the corpus stay exactly as they are. A6's verdict becomes
the model-independent honesty properties of the
dead-endpoint (offline) outcome, which is what Linux
grading can observe without a model:

1. Status is `Offline` or `InsufficientEvidence` — never
   `Answered` (no answer is presented without a model).
2. The compiled prompt contains the abstention sentence
   verbatim.
3. No cited chunk contains the requested fact (for A6:
   no citation mentions "berth").
4. Disclosure source memories equal exactly the cited
   set (nothing undisclosed, nothing fabricated).
5. Evidence stays within budget (compiled prompt under
   5,500 chars; evidence under 4,000 bytes).

The old zero-provider-traffic clause is replaced, not
silently dropped: provider traffic with partial evidence
is the designed behavior of repaired retrieval, and the
report states this explicitly. The genuinely-empty case
keeps its zero-traffic assertion in the new held-out
test (case 1 below), where it still holds. The A6
record's `wire_checks_passed` becomes this honesty
verdict, and its notes state the observed provider-hit
count plainly. Any regression in the five properties
fails the test exactly as the old assertion did.

## 5. Fresh held-out cases (new test, fresh entities)

New file `crates/core/tests/ask_partial_evidence_acceptance.rs`,
model-independent (dead-endpoint calls only; no model
quality is claimed). Fresh domain vocabulary, no
benchmark entities or wording:

1. Genuinely absent evidence: a question with no
   vocabulary overlap at all. Expect `InsufficientEvidence`,
   zero citations, empty disclosure, no provider call.
2. Shared vocabulary without the requested fact: the A6
   pattern with fresh entities. Expect `Offline`,
   abstention sentence in the prompt, no cited chunk
   containing the requested fact, disclosure exactly
   equal to the cited set.
3. Complete supporting evidence: a question whose fact
   is present in one record. Expect the fact span in a
   cited chunk, disclosure exactly equal to the cited
   set, and the compiled prompt containing the fact.
4. Conflicting evidence: two separate records stating
   different values for the same fact. Expect both
   values present in the compiled prompt (no silent
   choice of one value) and both sources disclosed.
5. Excluded/private-boundary records: the only copy of
   a fact in a sealed record, plus archived and
   foreign-workspace distractors. Expect the sealed fact
   absent from citations and disclosure, and
   `InsufficientEvidence` when no other evidence exists.

## 6. Decision needed (documented, not invented)

Whether ANANSI should also gate provider traffic for
unsupported questions (a sufficiency gate before the
model call) is a genuinely open requirement, NOT decided
here. Against inventing it: the frozen repair acceptance
locates honesty in the abstention instruction, and a
term-coverage gate would reintroduce AND-like behavior
and break the recall the repair restored. For it: it
would save provider calls on unanswerable questions.
This round implements no gate; the owner decides it as
a separate slice if wanted.

## 7. Corrections and preservation in this round

- The previous report's "byte-identical audit" wording
  is corrected: the reproduced audit and the supplied
  independent audit agree in every field (all 15 values
  identical), but the files differ in line endings
  (reproduced LF, supplied CRLF; 515 vs 530 bytes,
  `cmp` differs at char 2). They match semantically,
  not byte-identically. The report and memory entries
  are corrected to say so.
- The isolated preview launcher (`Start ANANSI Preview
  Ask Verified.cmd`) and all existing preview data
  roots are untouched; the diff must not contain them.
- No scale benchmark is repeated without a regression
  to investigate; the only benchmark run is the A6
  failure itself, before and after.

## 8. Regression list for this round

Focused: `benchmark_tokens`, the new held-out test,
content unit tests, batch-proof, accounting, realistic
corpus, large-scale. Then strict Clippy, formatting,
contracts, TypeScript, docs validation, API + web
builds, browser Ask 1/1. Then the full core suite once.
Every remaining failure is reported, classified
(environmental vs pre-existing vs new), and none is
hidden by weakening an assertion.
