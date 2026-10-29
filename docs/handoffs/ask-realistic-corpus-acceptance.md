# Realistic retrieval acceptance — frozen criteria (before running)

Corpus: a separate local-only test, `ask_realistic_corpus_acceptance.rs`,
with at least 1,000 records, at least four fictional domains (Meridian
Rail, Halcyon Botanical, Solenne Vineyards, Ferrum Mining), at least
80 fresh questions, varied structures and lengths, similar names,
shared terminology, plausible distractors, revisions, conflicts,
dates, multi-source questions, and missing facts. No original
benchmark template is reused; filler is combinatorial (templates with
slot vocabularies), not repetitions of a few sentences.

Question families (80): single-fact 22, reworded 12, multi-source 12,
conflict 8 (gold is the dated correction note, the superseded note is
a distractor), current-revision 8, date 6, term-disjoint negatives 6,
natural negatives 6 (three whose fact lives only in an excluded
note, three fabricated combinations that no note states).

Gold bindings: every answerable question declares (span, source id)
pairs. Completeness means every declared span appears inside its
declared source's actual serialized evidence chunk (same binding
rule as the large-scale test). Nothing gold is passed into
retrieval; questions go through the real public Ask/compiler path.

Bars (fixed before running; failures are reported, not bar-lowered):
- Binding-complete: at least 62 of 68 answerable questions.
- Term-disjoint negatives: 6/6 InsufficientEvidence.
- Natural negatives: 6/6 with an honest outcome (Offline partial
  context or InsufficientEvidence), the abstention instruction
  present where partial context is returned, no excluded fact span
  in any prompt, disclosure and budget clean.
- Disclosure: cited == disclosed for all 80 questions.
- Budget: compiled prompt under 5,500 chars for all 80 questions.
- Leakage: zero citations of any Sealed, foreign-workspace,
  archived, or expired record in any question, and zero canary
  strings in any prompt.
- Timings: setup and query times reported separately; query p95
  under 5,000 ms across all 80 questions.

Metric definition (honest, as required): "additional source
appearances" counts, per answerable question, unique cited source
ids minus declared gold source ids, summed across answerable
questions. It is not a per-chunk irrelevance rate.

For natural negatives, partial retrieval is distinguished from
model refusal: with no model call, no claim is made that an AI
correctly refused. The check is honest context plus the abstention
instruction, not refusal evidence.

Raw per-question evidence (status, prompt chars, cited count,
additional count, unbound pairs) is printed by the test and retained
in the delivery evidence. At most two repair/retest cycles per
concrete defect; a fixture correction must be explained and must
never weaken a bar.
