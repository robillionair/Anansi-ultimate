# Ask scale hardening — report

Branch `muse/ask-scale-hardening` from `9b9ead8f` (the preserved
Windows-review checkpoint; that branch is untouched). Three work
packages, all local-only, no cloud calls.

## WP1 — Every permission check during one public Ask, accounted

Diagnostic instrumentation (counters only; no paths, contents, or
credentials recorded) now splits the old single counter by call
site: root, database, batch-loop paths, content read/write
directories and files, untagged sites, fallback reasons, and
read-time rechecks. One Ask was measured on workspaces of 0, 1, 16,
128, and 1,010 notes, setup and Ask timed separately (Linux):

| notes | setup ms | ask ms | batches | batch paths | single checks | root | database | rechecks |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | 0 | 8 | 0 | 0 | 8 | 4 | 4 | 0 |
| 1 | 3 | 11 | 1 | 4 | 12 | 4 | 4 | 2 |
| 16 | 127 | 23 | 1 | 34 | 42 | 4 | 4 | 17 |
| 128 | 650 | 37 | 1 | 258 | 266 | 4 | 4 | 129 |
| 1,010 | 9,241 | 210 | 1 | 2,022 | 2,030 | 4 | 4 | 1,011 |

This reconciles the prior report exactly: the "42 checks" for 16
notes are 34 batch-loop paths plus 8 connection checks (four
connections: open, proof enumeration, rebuild, compilation, each
verifying root and database). Permission work outside the batch is
constant at every size (8). Inside the batch, path count grows
linearly (2N + 2) but batch invocations stay 1. On Linux, single
checks grow linearly because the batch loops cheap metadata
checks; no process is spawned for permission verification on Linux
at all (the only process spawns in the permission path are
Windows-gated PowerShell calls). The Windows process count for an
operation is, by construction, single + batch calls — but that
formula must be confirmed by running the same diagnostic on
Windows; Linux cannot establish Windows launch counts or Windows
speed. Ask time grows with corpus size (8 → 210 ms) from index
rebuild I/O (reading every body), not from permission work. Setup
permission work grows linearly because creating a protected file
inherently verifies it; that is setup cost, reported separately,
not Ask cost. No general repair was needed for corpus scaling:
outside-batch permission work does not grow with corpus size.

## WP2 — Adversarial batch-proof acceptance, and a real race found

Criteria were frozen before tests
([ask-batch-proof-adversarial.md](ask-batch-proof-adversarial.md)).
Verified on Linux (7 content-level + 4 Ask-level tests):

- Outside-set, other-workspace, and other-root paths fall back to
  full verification: properly permissioned files read, overbroad
  files fail. Proofs never authorize them.
- Replaced bodies fail the hash check; replaced files or
  directories fail link rejection; missing and oversized bodies
  fail integrity.
- Revised records read the new revision through fallback with the
  new hash. Sealed and archived records leave the next Ask.
  Deleting the content directory fails closed, never silently
  empty. A failed batch leaves no trace (verify-only: it repairs
  nothing and creates nothing).
- Proofs are not reused across operations: widening between two
  Asks fails the second Ask; repair restores the third.

The race: a file widened after proving still read successfully on
the stale proof. Content substitution and path swaps were already
closed by read-time hash and link checks; permission state was
proven only at batch time. Repair (this branch): on non-Windows
platforms, a proven read now re-validates the file's permission
bits at read time for one metadata check, counted separately
(rechecks = N rebuild reads + cited compilation reads, exactly as
measured above). The widen-after-proof test failed before the
repair and passes after it; an older test that documented stale
reads was updated to the stricter contract. Windows has no cheap
equivalent (ACL checks need a process spawn, which batching exists
to avoid), so Windows keeps a one-operation residual window:
proofs are fresh per operation and the next operation re-verifies.
Windows reparse points are untestable on Linux and are classified
unsupported-on-Linux, not fabricated; the same `reject_links` path
(including the Windows reparse-attribute check) is the reviewer's
step. Batch verification itself has no cancellation hook (single
synchronous step, bounded by 200,000 paths); cancellation lives at
the Ask-stream level, covered by existing streaming tests.

## WP3 — Realistic corpus (first cycle pass, no repair needed)

Separate local-only test, criteria frozen before running
([ask-realistic-corpus-acceptance.md](ask-realistic-corpus-acceptance.md)):
1,008 records, four domains (Meridian Rail, Halcyon Botanical,
Solenne Vineyards, Ferrum Mining), 80 fresh questions (22 single,
12 reworded, 12 multi-source, 8 conflict with the dated correction
as gold, 8 current-revision, 6 date, 6 term-disjoint, 6 natural),
varied structures including a headed multi-section note with a
table, similar names, shared terminology, 14 distractors, and
Sealed/foreign/archived/expired canaries. Filler is combinatorial
(16 templates × place/item/name/date vocabularies), not repeated
sentences.

Results: binding-complete 68/68 (bar 62), term-disjoint 6/6
InsufficientEvidence with empty prompts, natural negatives 6/6
(all Offline partial context, no excluded fact, abstention
instruction present), disclosure 80/80, budget 80/80 (prompts under
5,500 chars), zero canary leaks, zero excluded-record citations.
Setup 12.7 s for 1,008 records; query mean 318.8 ms, p95 366.8 ms,
max 455.8 ms (bar 5,000 ms). Honest metric, defined as in prior
rounds: additional source appearances = unique cited sources minus
declared gold sources, summed over answerable questions = 600
(680 cited over 68 questions, 80 gold bindings; every question
fills the 10-item evidence budget, so additional citations are
substantial while completeness and budgets hold). Natural
negatives show partial retrieval, not model refusal: with no model
call, no refusal claim is made.

## Verification (Linux) and Windows handoff

- Full Rust core suite: 321 passed, 5 failed — none caused by
  this branch. Four are the known live-Ollama environment family
  (three fail connecting to 127.0.0.1:11434; the fourth expects a
  revision conflict but gets StorageUnavailable because the
  embedding runtime is offline); the embedding model is not on
  disk here and the assignment forbids downloads, so these cannot
  pass in this session. The fifth is a pre-existing benchmark
  invariant: `benchmark_tokens` A6 expects InsufficientEvidence
  for "What is the Harbor deep-water berth fee?", but repaired OR
  retrieval returns partial matches (Offline). A pristine
  `9b9ead8f` worktree fails A6 identically, proving it predates
  this branch; fixing it would mean changing retrieval semantics
  or a benchmark expectation, which is an owner/reviewer decision,
  not a silent edit. Both reproductions are retained in the
  delivery evidence.
- Strict Clippy, formatting, contracts, TypeScript, docs
  validation, API and web builds: all clean (a Clippy-only
  refactor of the realistic test — unused `mut`, an unqueried
  structured note renamed, negatives built as a literal — was
  re-verified: the realistic test re-passes identically, query
  p95 390.0 ms on rerun versus 366.8 ms first cycle).
- Canonical browser Ask workflow: 1/1.
- The prior branch `muse/ask-windows-scale-repair` (9b9ead8f) is
  preserved untouched; this branch is separate.

Windows handoff — what must be rerun, and what decides acceptance:
1. The 16-note probe (reviewer's file, untouched) plus the new
   accounting diagnostic (`ask_permission_accounting`): acceptance
   is batch = 1 per Ask at every size with single + batch
   processes matching the formula, and Windows Ask time for the
   16-note probe well below the pre-repair 27,637 ms.
2. The adversarial tests on Windows, especially the widen-after-
   proof case: on Windows the proven read is expected to succeed
   within the one-operation residual window, while the next Ask
   fails — if the reviewer requires the window closed on Windows
   too, that needs a native ACL recheck, which is a separate,
   larger change.
3. The realistic corpus test on Windows at full scale (1,008
   records): acceptance is the same bars (68/68 binding, negatives,
   disclosure, budget, zero leaks) plus a Windows query p95 the
   reviewer judges acceptable; Linux p95 (366.8 ms) is not a
   Windows number.

## Verdict

Improved: permission cost per Ask is now fully accounted and
provably constant outside the batch; the widen-after-proof race
is closed where a cheap check exists; retrieval holds on a
realistic 1,008-record corpus with binding, disclosure, budget,
and leakage bars all passing first cycle. Remains slow or unsafe:
Windows Ask time is still unmeasured (the rerun decides); the
Windows one-operation permission window is a documented residual;
evidence budgets fill with additional citations (600 over 68
questions) even as completeness holds; model refusal is still
unmeasured (frozen cloud round held at 0/31); full core is not
green in this session for the environment and pre-existing reasons
above, neither caused by this branch. Ready for Windows
testing: yes — this checkpoint is a sound basis for the reviewer's
rerun, with the two Windows-only items above explicitly open.
