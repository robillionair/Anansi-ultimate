# Large synthetic benchmark — frozen acceptance plan, 2026-10-29

The owner's latest instruction authorizes a new cloud benchmark now, superseding
the earlier no-new-round restriction for this bounded experiment only. Branch
`codex/benchmark-large-free` from reviewed `cf3880bc5`. No production changes,
preview promotion, paid models, downloads, private owner data, pushes or CI.

## Acceptance and experimental limits

- Prepare 100 distinct synthetic questions before inference. Cover short, medium,
  and long documents, early/middle/late evidence, multiple sources, conflicts,
  absent evidence and paraphrasing. Record actual category counts and corpus size.
  The fixture uses ten repeated question templates with varied entities/values,
  lengths and fact positions: report that effective diversity explicitly.
- ANANSI must use its real lexical Ask/context path, not hand-selected evidence.
  No embedding model is configured for this fixture: semantic retrieval is outside this experiment.
  Bulk SQL fixture creation is test setup only; production reads/integrity/privacy
  enforcement remain intact. Document setup omissions and timing scope honestly.
- Compare full eligible context, a frozen basic lexical top-three/first-chunk
  baseline, and ANANSI with unchanged default budget. Preserve retrieval misses.
  No Sealed or other-workspace content can enter any outgoing prompt.
- Freeze expected answers, source identities and prompts before model calls.
  Score all questions, including refusals, missing answers and malformed output.
  Deterministic fact matching and citation-index validity are diagnostic scores,
  not independent human verification of every semantic claim.
  Before scored calls, a read-only review identified numeric-substring, false
  abstention and final-request-completeness scoring defects. The frozen runner
  now requests exact values, integer arithmetic, pipe-separated conflicting
  values and the explicit INSUFFICIENT_EVIDENCE abstention sentinel. The scorer
  follows that format; no decimal/negative substring match or flag-only refusal
  gets credit. All 30 unique batch/approach pairs require validated accounting.
- Use only `nvidia/nemotron-3-ultra-550b-a55b:free`, after official model metadata
  confirms zero input/output pricing. Exact model, fallback disabled. Maximum
  31 inference attempts including smoke; stop on nonzero cost, unexpected model,
  auth/payment/rate-limit/server error or ambiguous failed request. No auto-retry.
- Ten independent question items per request: 10 batches × 3 approaches = 30
  scored calls, plus one smoke. Within each batch avoid repeated identical corpus
  contexts where possible. This batching is a free-tier constraint: report usage
  at batch level, never pretend to have per-question provider token counters or
  300 independent model calls. Use one repeat only and make no significance claim.
- Identical answer instructions, temperature 0, maximum output 4096, provider
  default reasoning, and usage reporting across scored calls. Rotate approach
  order by batch; no tuning after answers are visible. Preserve exact payloads,
  response bodies, timings, counter anomalies, fixture hashes and partial results.
- API-reported input/completion/total counters stay separate from estimates.
  Reasoning subcounters can be inconsistent; preserve and flag, never normalize.
  Report accuracy alongside savings, by stratum and with outlier sensitivity.
  No claim of paid-cost savings, production performance or competitor superiority.

The preflight key endpoint authenticated and reported free-tier access; spending
limit remaining is not the remaining daily request count. No account identifier
or credential will be stored in deliverables.
