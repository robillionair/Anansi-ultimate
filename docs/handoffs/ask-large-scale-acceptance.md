# Large-scale Ask retrieval acceptance — criteria (frozen before implementation)

Scope: local-only retrieval/compiler evidence. No cloud calls, no model
answers, no token-savings claim. The test uses the real application Ask
path (`Core::ask_anansi`) with a refused loopback endpoint, so every
question yields an Offline compiled prompt (or InsufficientEvidence)
without any inference. This acceptance checks whether the repaired
retrieval holds up on a larger, shared, messier collection. It does not
re-score model answers and does not certify production, native desktop,
or Windows acceptance.

## Corpus (one shared workspace, plus one foreign workspace)

- At least 1,000 varied records in a single shared workspace.
- Fresh fictional domains and wording. The original ten question
  templates (single-fact, paraphrase, arithmetic sum, arithmetic
  difference, unresolved conflict, insufficient evidence, as instantiated
  in the 100-case fixture) are not reused: no Harbor entities, no
  repeated template sentences, no copied question shapes.
- Contents must include:
  - Similar entities and distracting near-matches (same base name with a
    different qualifier, and same fact pattern with a different value).
  - Facts placed early, in the middle, and late in long documents
    (several notes of 3,000–8,000 characters).
  - Varied document lengths (short, medium, long) and mixed insertion
    order (target notes interleaved with distractors, some first, some
    last), so order cannot rescue a weak ranker.
  - Current revisions superseding older values (revised notes whose
    current body states the current value only).
  - Conflicting current notes (two live sources disagreeing on one slot).
  - Archived notes (lifecycle archived) and expired notes (validUntil in
    the past) whose content must not be supplied.
  - Sealed notes (privacy sealed) and a foreign-workspace note, each
    carrying a unique canary string that must never appear in any prompt
    for the main workspace.
- No hand-selected evidence and no gold answers passed into retrieval:
  questions go through `ask_anansi` only; the test asserts on the
  returned compiled prompt, citations, and disclosure manifest.

## Questions (48 fresh questions)

- Plain single-fact questions: 8.
- Differently worded questions (wording not present in the note): 8.
- Multi-source questions needing facts from two notes: 6.
- Fact-position questions on long notes (early 2, middle 2, late 2): 6.
- Current-revision questions (current value, superseded values absent): 4.
- Conflict questions (both live values must be present): 4.
- Archived/expired-only questions (only excluded notes match): 4.
- Missing-evidence questions (no supporting note exists): 4.
- Near-match disambiguation questions (qualifier pins one of two
  similar entities; the correct entity's fact must be present): 4.
- Total 48: 40 answerable, 8 insufficient (the archived/expired-only
  and missing-evidence families).

## Measurements recorded per question

- Evidence completeness: every required fact span present in the
  compiled prompt (answerable questions), or InsufficientEvidence with
  empty citations (insufficient questions).
- Irrelevant context: cited sources beyond the required set, and total
  compiled-prompt characters (reported, not hard-gated except by budget).
- Budget compliance: compiled-prompt byte length for every question.
- Disclosure equality: cited memory set equals disclosed
  `source_memories` set.
- Canary integrity: Sealed and foreign canary strings in zero prompts;
  Sealed memory cited zero times.
- Timings: wall time per Ask call (mean, p95, max) and total seeding
  time, recorded from the test run log. Timings are diagnostic scope
  only, not a product speedup claim.
- Exact Search unchanged: two multi-term queries that Ask recovers must
  still return empty under exact Search (AND semantics preserved).

## Pass bars (fixed before implementation)

- Answerable completeness: at least 36/40 questions contain all
  required spans.
- Insufficient correctness: 8/8 return InsufficientEvidence with empty
  citations.
- Canary integrity: 0 leaks in 48/48 prompts; 0 Sealed citations.
- Disclosure equality: 48/48.
- Budget: 48/48 compiled prompts under 5,500 bytes (4,000-byte evidence
  cap plus fixed preamble, instruction, and question overhead).
- Exact Search: 2/2 queries still empty.
- Timing sanity: per-question Ask p95 under 5 seconds on this machine.
  A timing miss is investigated as a defect, not silently passed.

## Defect rules

- At most two repair/retest cycles. A defect found here gets a bounded
  general fix in retrieval/compilation (ranking, candidate cut,
  chunking, budget, or eligibility), never a dictionary of fixture
  answers, synonyms, entities, or question wording.
- Exact Search behavior, integrity checks (revision pinning, body
  hashes), privacy boundaries (Sealed, foreign workspace, archived,
  expired), citation/disclosure equality, and cancellation behavior
  must be retained; regressions in the existing Ask, context, hybrid,
  body-search, and streaming tests fail the cycle.
- If a remaining defect needs broader work after two cycles, stop:
  document its reproduction and deliver the usable checkpoint rather
  than looping.

## Test location and command

- `crates/core/tests/ask_large_scale_acceptance.rs`
- `node scripts/cargo.mjs test -p anansi-core --locked --test ask_large_scale_acceptance -- --test-threads=1 --nocapture`
