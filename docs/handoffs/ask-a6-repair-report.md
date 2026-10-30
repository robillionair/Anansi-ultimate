# A6 benchmark repair — report (2026-10-30)

Branch: `muse/ask-a6-repair`, from
`muse/ask-verified-integration` at
`151565d7f7e6006f4822e9681bea524522472311`, preserving
`532afccf` and `4ef90406d`. Acceptance was frozen before
any change in `docs/handoffs/ask-a6-acceptance.md` (commit
`465e272b`). No production code changed. No cloud calls,
credentials, model downloads, paid inference,
generated-code execution, pushes, master merges, remote CI,
preview promotion, or changes to the owner's running
application. No scale benchmark was repeated; the only
benchmark run is the A6 failure itself, before and after.

## What was wrong?

The benchmark's A6 invariant, "insufficient-evidence
honesty / zero hits", asserted
`status == "InsufficientEvidence" && hits == 0` for
"What is the Harbor deep-water berth fee?" — a fact
genuinely absent from the corpus. That assertion encoded
the pre-repair AND-retrieval side effect (no chunk
contains all of harbor/deep/water/berth/fee, so the
result was empty and no provider call happened) as if it
were the honesty requirement. After the repaired
OR-candidate retrieval, Harbor-related partial matches
are returned by design, citations are non-empty, Ask
proceeds to the provider, and the canned fake provider
answers — so the test failed with `status=Answered`,
even though Ask's own behavior was honest: the
model-independent offline outcome never presents an
answer without a model, carries the abstention
instruction, cites only partial sources, and discloses
exactly those sources. The defect was the assertion,
not the fixture (the fact is absent; the shared
vocabulary is the point of the case) and not production
retrieval or status logic (both match the frozen repair
acceptance: "retrieval is not proof a question can be
answered. Unsupported questions must retain the
instruction to abstain"; "partial matches may be
irrelevant").

## Did you change the app or the test, and why?

The test only — because the defect is in the test's
proxy, and production already implements the requirement
it proxies for. Changing production (for example, a
term-coverage gate before provider traffic) would
reintroduce AND-like behavior and break the recall the
repair restored; it would also invent a requirement
(the traffic-gating decision is documented as open in
the acceptance file, not decided here).

The correction re-anchors A6 to the requirement itself,
without weakening it:

- The case, the question, and the corpus are unchanged.
- The verdict is now the model-independent offline
  outcome (dead-endpoint call): status `Offline` or
  `InsufficientEvidence` (never `Answered`); the
  compiled prompt contains "state truthfully that
  evidence is missing"; no cited chunk mentions
  "berth"; disclosure source memories equal exactly the
  cited set; compiled prompt under 5,500 chars.
- The record now reports the real offline outcome
  (previously its prompt/evidence/citation fields were
  hardcoded to 0) and states the observed fake provider
  hit count plainly in its notes, with the explanation
  that traffic with partial evidence is designed
  behavior graded at the evidence level, not gated.
- The genuinely-empty case keeps its zero-traffic
  assertion in the new held-out test, where it still
  holds. Any regression in the five honesty properties
  fails the test exactly as the old assertion did.

## What now passes?

- Before: `benchmark_tokens` failed, exit 101, panic at
  line 1929, `A6 / anansi: status=Answered ...
  wire_ok=false` (log preserved in `before/`).
- After: `benchmark_tokens` passes 1/1; A6 records
  `status=Offline prompt_chars=Some(5027)
  est_tokens=Some(1256) wire_ok=true`. Note the budget
  headroom: 5,027 chars on the full corpus against the
  5,500 bound (evidence cap 4,000 bytes plus preamble
  and question). The bound holds, but it is not
  generous; a larger corpus of partial matches could
  approach it, and the bound itself is what the test
  enforces.
- New held-out test `ask_partial_evidence_acceptance`
  passes 5/5, all model-independent (refused loopback):
  genuinely absent evidence abstains with no citations;
  shared vocabulary without the fact is partial, never
  Answered, with the abstention instruction and exact
  disclosure; complete evidence is cited and disclosed
  with the fact span; conflicting evidence preserves
  both values and both sources; sealed, archived, and
  foreign records never leak (sealed-only and
  archived/foreign-only questions both yield
  `InsufficientEvidence` with empty citations).
- Focused regressions unchanged: content unit tests
  7/7, batch-proof 4/4, accounting 1/1, realistic
  corpus 2/2 (68/68, p95 349.7 ms Linux), large-scale
  2/2. Strict Clippy, formatting (one drift in the new
  A6 block caught by `fmt --check` and fixed; the
  reformat touched only that block), contracts,
  TypeScript, docs validation, API + web builds, and
  browser Ask 1/1 all pass.
- Full core suite, run once after all focused checks passed:
  328 passed, 4 failed — up from 322 passed, 5 failed. The
  delta is exactly the repair: benchmark_tokens now passes
  (+1) and the new held-out test adds 5 passes. The 4
  remaining failures are the same live-Ollama environment
  failures as every prior round (embedding model absent on
  disk, downloads forbidden), all in `semantic_workflow`:
  `concurrent_edit_and_privacy_change_abort_atomic_embedding_publication`,
  `partial_chunk_insert_failure_rolls_back_complete_chunk_set`,
  `real_semantic_embedding_indexing_retrieval_and_sealed_refusal`,
  `reranking_temporal_validity_and_contradiction_lifecycle_evaluation`.
  No new failure, no assertion weakened. The pre-existing
  A6 failure is gone; there is no longer a benchmark A6
  invariant failure to carry.
- The isolated preview launcher and all existing
  preview data roots are untouched; the diff contains
  no launcher or preview file.

## What remains blocked?

- Model-answer quality: unmeasured, as before. The
  fake provider is canned plumbing; real-model
  abstention behavior (whether a model obeys the
  abstention instruction on partial evidence) has no
  evidence in this round and no claim is made about it.
  The local quality comparison still awaits an
  installed model; the frozen cloud round still awaits
  a credential. Both remain held at 0 attempts.
- Traffic gating for unsupported questions: an open
  owner decision (see the acceptance file). This round
  implements no gate.
- The 4 live-Ollama environment failures (model absent
  on disk, downloads forbidden) are unchanged and
  unrelated to this repair.
- Native desktop acceptance, preview promotion, M5
  production gates, §32 sandbox, G1 audit, and remote
  CI remain open, as before.

## Audit wording correction (this round)

The previous delivery described the Windows evidence
audit reproduction as "byte-identical". That was wrong
and is corrected here, in the previous report, in
BUILD_STATUS, and in memory: the reproduced audit
matches the supplied independent audit in every field
(all 15 values identical), but the files differ in line
endings (reproduced LF, 515 bytes; supplied CRLF, 530
bytes; `cmp` first differs at char 2). The results
match semantically; the files are not byte-identical.
