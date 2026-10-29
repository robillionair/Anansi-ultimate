# Ask candidate recall repair — acceptance before implementation

The owner's request to fix the measured recall loss authorizes this bounded
implementation on `codex/ask-retrieval-recall`, from `737d43691`. Existing benchmark
records remain immutable. No new cloud calls, model downloads, preview promotion,
CI, pushes or generated-code execution are part of this repair.

## Diagnosis and implementation boundary

Ask normalizes natural questions, but the shared lexical query requires every
remaining term. Synonyms and question wording therefore eliminate useful records.
Use a separate internal any-term candidate mode for Ask, ranked by existing BM25
and hybrid fusion. Exact Search and explicit context query semantics stay unchanged.
Apply workspace/current-revision/privacy/lifecycle/kind eligibility before the
bounded candidate cut, retaining canonical checks and hash-verified content reads.
Do not add benchmark-specific synonyms, entities or answers.

Give candidate memories a first relevant chunk before spending Ask's remaining
budget on further chunks of any one memory. Retain section offsets, deduplication,
UTF-8 safety and the existing evidence-byte budget. For lexical hits, omit extra
chunks without query overlap; preserve a body chunk for title-only hits and
semantic-only evidence. Disclosure must list only actually included sources.

## Acceptance

1. Before/after local regressions demonstrate paraphrased questions retrieve the
   named entity's evidence and two-source questions include both operands, even
   with a long first record. Use fresh non-benchmark entities and wording.
2. Exact Search still requires all terms. No-content overlap remains insufficient;
   retrieval is not proof a question can be answered. Unsupported questions must
   retain the instruction to abstain. No automatic model or cloud calls.
3. Sealed, foreign, archived, expired, stale and corrupted records remain excluded
   or fail closed; excluded records cannot consume the candidate limit.
4. Budgets, UTF-8, citation/source identity, deterministic ordering and integrity
   regressions pass, alongside existing Ask/context/hybrid/body-search tests.
5. Reprepare the unchanged 100-case fixture locally into a NEW artifact. Compare
   exact required fact availability and prompt bytes with the original; report
   all cases, not just improvements. This is retrieval regression evidence, NOT
   new answer accuracy or measured provider-token savings. Any future cloud
   comparison requires a separately bounded run.

Remaining limits: no lexical system guarantees matching completely disjoint
vocabulary, and partial matches may be irrelevant. No broad semantic-quality,
native workflow, production certification or milestone completion claim.
