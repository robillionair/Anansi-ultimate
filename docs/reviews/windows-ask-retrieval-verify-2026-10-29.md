# Windows review of Muse retrieval verification — 2026-10-29

## Verdict

Delivery identity and frozen cloud payloads verified. The incoming branch adds no
production code, and its Linux evidence remains Linux evidence. This review found
an existing Windows performance defect: a public Ask on just 16 synthetic notes
took **27,637 ms**. Evidence retrieval was correct. Full 1,007-record Windows
acceptance and model-answer/token measurements remain pending; do not promote
this as a responsive Windows preview or a measured quality/savings win.

Incoming branch `muse/ask-retrieval-verify`, tip
`f90f9e850db6cf5b6890f3d50c50815df25a4b2d`; baseline
`95d384e3e24dfc53aeb73aa94a08891f7e9d101d` and implementation `24d3df5` are ancestors.
ZIP SHA-256 `3877a6348e92900c26e07e771d325cdc570c8ca26e8b7fec783df08ffb0e6205`;
bundle SHA-256 `f5fd9e65e07a5ed72da7ff2f98e1b0913a256c2be028f9d90fb881b86172f09a`.
Both match the supplied identities; bundle verifies and fetch lands on exact tip.

## Findings

### P1 — Windows Ask scales through repeated process launches

The review's public-path diagnostic creates 16 ordinary private notes, then asks
one natural question against an explicitly refused loopback endpoint. Windows
result: seeding 46,011 ms; Ask 27,637 ms; Offline with one citation and the correct
`REVIEW-742` evidence. The whole test passes its content checks in 76.72 seconds.
This is one measured call, not a p95 distribution or a 1,007-record measurement.

Source confirms the mechanism: `ask_anansi_controlled` calls
`rebuild_search_index` on every question. Rebuild calls `content::read` for every
non-Sealed current record, even non-matching records. Each read checks three
directories and the body file through `storage::private_permissions`; Windows
starts powershell.exe for every such verification. Compiler reads and trusted
storage opens add further checks. Linux uses local metadata checks instead.

The incoming test creates 1,007 records through the public API and then performs
48 full public Ask calls. Running it unchanged on Windows would incur extensive
additional work; it was deliberately NOT launched in this review. There is no
invented timeout failure or full-suite pass. Preserve its exact scope for the
post-fix rerun; the 342.7 ms Linux p95 is not a Windows claim.

Next repair: bound Windows permission-verification process count per operation
without removing ACL, owner, reparse/link or body-hash checks. Investigate the
existing short-lived batch ACL proof pattern in vault IO. Proofs must stay bound
to an operation and covered paths, not become permanent positive ACL caches.
Retain corruption, concurrent changes, revocation and fail-closed behavior.
Do not simply remove index rebuilding or weaken validation to make tests fast.

### P2 — Evidence completeness does not assert fact/source binding

In `ask_large_scale_acceptance`, the success condition searches required fact
strings anywhere in `compiled_prompt`. `required_ids` is only used to count extra
sources, not to gate success or bind each span to its intended record. Common
phrases such as `at eight` can occur in several notes. This is a false-positive
path in the test, not proof that the reported run actually credited wrong facts.
Its aggregate counts are consistent with all 50 required source appearances being
present, but the assertion is still insufficient for provenance acceptance.

Require every declared source id and its corresponding fact in that source's
actual serialized evidence chunk, not in the 200-byte citation preview or the
whole prompt. Add an isolated regression showing the same phrase in a wrong
source cannot satisfy the scorer. Preserve the original questions and bars.

### P2 — Empty retrieval is narrower than safe refusal

The retained cycle-2 log reports 4/8 empty-result checks. Four natural questions
received partial matches. The final test changes these probes to distinctive,
term-disjoint words and gets 8/8. Muse discloses this correction, and term-disjoint
exclusion probes are useful, but it narrows what the pass demonstrates.

Keep those exclusion tests AND retain the natural overlapping negative questions
as a separate family. Without inference they should verify no excluded content,
honest partial context and the abstention instruction. Actual refusal must later
be measured with model answers; an instruction alone is not evidence it worked.
No model-answer calls are part of this Windows review.

The reported 96/146 metric counts additional unique source appearances across
questions. It is not a count of every chunk citation or an independently judged
irrelevance rate. Preserve the figures with that definition.

## Checks actually completed

- Incoming checksum, exact tip, baseline ancestry, bundle integrity and absence
  of production changes versus `95d384e3` verified.
- Delivered repaired fixture SHA matches `738ecc96...`; all 30 unique scored
  payload hashes verified and parsed payloads equal the existing runner's output
  exactly. Smoke model/fallback settings match. Zero cloud calls in this review.
- Windows diagnostic 1/1 passes its evidence and budget assertions; timing above
  exposes the performance limitation rather than certifying responsiveness.
- Strict Clippy for the incoming large-scale test and review diagnostic passes
  on Windows. Formatting and Git diff checks pass. Incoming docs validate at 149
  Markdown files; this review adds another evidence document.
- No application behavior changed since the prior verified repair. Its 82 tests
  and browser evidence remain prior evidence, not newly rerun 83/83 Windows results.

Command corrections: none of the incoming cloud requests was sent. No credential
was read from previous chat content, printed, written to a file or requested.

## Reproduction and preserved evidence

Use E: for TEMP, TMP and CARGO_TARGET_DIR, and a synthetic root only:

```powershell
node scripts/cargo.mjs test -p anansi-core --locked --test ask_windows_scale_probe -- --test-threads=1 --nocapture
node scripts/cargo.mjs clippy -p anansi-core --test ask_large_scale_acceptance --test ask_windows_scale_probe --locked -- -D warnings
```

Review logs and payload audit:
`E:/Anansi/benchmarks/ask-retrieval-verify-windows-2026-10-29`.
Incoming delivery remains unmodified under
`E:/Anansi/reviews/ask-retrieval-verify-2026-10-29`.
Review branch `codex/review-ask-retrieval-verify` preserves the incoming tip.
No pushes, master merges, CI, preview promotion, native acceptance, generated-code
execution, live provider verification or audit-monitor activation.
