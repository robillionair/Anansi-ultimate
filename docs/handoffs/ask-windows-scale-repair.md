# Windows scale repair — acceptance (frozen before implementation)

Baseline: reviewed checkpoint `8bdcbae105abdb45e0bcc93669d25994355746a9`
(branch `codex/review-ask-retrieval-verify`), one commit over the
`muse/ask-retrieval-verify` delivery `f90f9e85`. The reviewer's
diagnostic (`crates/core/tests/ask_windows_scale_probe.rs`, Windows
only) is review material and is not edited. No cloud calls, no preview
promotion, no new features, no timing-limit increases, no skipped work
reported as passed. The frozen 31 cloud payloads are preserved.

## P1 — Bound Windows permission-verification cost per Ask operation

Defect (review P1): a public Ask on 16 notes took 27,637 ms on
Windows. `ask_anansi_controlled` rebuilds the search index on every
question, and rebuild reads every non-Sealed current record through
`content::read`, which verifies three directories plus the body file
through `storage::private_permissions` — one `powershell.exe` spawn
per verification on Windows. Cost scales with workspace size.

Repair (bounded, checks preserved):

- Add an operation-scoped batch proof set for content reads, following
  the existing `vault/io.rs` `VerifiedFiles` pattern: one Windows
  PowerShell process verifies a whole batch (JSON path list on stdin,
  the same `protect.ps1` owner/DACL invariant per item, verify-only
  so nothing is mutated), or per-path metadata checks on other
  platforms. Paths are deduplicated (shared directories verified
  once). The proof set is created fresh per operation and dropped at
  its end: no permanent positive cache, no cross-operation reuse.
- Ask builds one proof set per question and shares it between index
  rebuild and context compilation. Rebuild also builds its own batch
  when called without proofs, so every rebuild caller benefits and no
  rebuild/integrity behavior is removed. (Correction: the measured
  total equals 2N + 10 exactly, because Ask opens four connections
  and each connection verifies the root and the database file; the
  bound below is "at most 2N + 10", not "under" it.)
- Every check is retained: link rejection per path, owner/DACL
  verification (batched, not skipped), body-hash verification per
  read, `MAX_BODY` enforcement, UTF-8 validation, privacy/identity
  checks in rebuild, and Sealed exclusion. A read for a path outside
  the proof set (for example a revision created mid-operation) falls
  back to the full per-path verification, never to an unverified read.
  A failed batch fails the operation (fail-closed for the whole
  batch, as in the vault pattern). Revocation takes effect on the
  next operation, when proofs are rebuilt from scratch.
- Write paths (note creation, protection on create) are unchanged;
  seeding cost on Windows is unchanged and stays reported separately.
  Semantic-only snippet reads in hybrid search keep per-path
  verification (bounded by hit count, unchanged behavior).

Acceptance:

- A diagnostic counter records individual permission verifications
  and batch verifications (diagnostic only, no behavior change).
  On a 16-note workspace, one Ask performs exactly 1 batch
  verification covering 34 paths (2 shared directories + 16 record
  directories + 16 files), and individual verification calls stay at
  or under 2N + 10 (42 for N = 16: the batch's 2N + 2 paths plus 8
  root/database checks from the four connections Ask opens; the old
  path needs about 4N plus compile re-reads plus those checks). The
  same bounds hold on Linux (where the batch loops metadata checks)
  and on Windows (where the batch is one process).
- Tamper cases fail closed: a corrupted body still fails integrity
  (hash check), and an overbroad file still fails batch verification;
  a fresh verification after repair succeeds.
- Existing content/integrity/privacy tests still pass, and Ask
  responses (prompts, citations, disclosure) are unchanged: the
  repair changes verification cost, not retrieval results.
- Linux timings are reported, not claimed as Windows results. The
  Windows reviewer reruns the same 16-note diagnostic before
  attempting full scale; no Windows timing threshold is set here.

## P2 — Fact-to-source binding in the large-scale scorer

Defect (review P2): the 1,007-record test searched required spans
anywhere in `compiled_prompt`; `required_ids` only counted extra
sources. A common phrase (for example `at eight`) in a wrong source
could satisfy the check.

Repair:

- Every required span is bound to its declared source: the span must
  appear in that source's actual serialized evidence chunk in the
  prompt, where chunk `n` maps to the citation with `index == n`.
  The whole-prompt search is replaced by this binding check for all
  40 answerable questions. Original questions and bars are preserved.
- An isolated regression proves discrimination: the same phrase in a
  wrong source satisfies the old whole-prompt check but fails the
  binding check, while the correct binding passes.

## P2 — Retain realistic missing-evidence cases separately

The cycle-2 natural questions (partial matches, not empty) are kept
as a separate family of 4, alongside the unchanged term-disjoint
exclusion family (8/8). Without inference, each natural negative
question must verify: no excluded content (no archived, expired,
Sealed, or foreign id cited; no excluded fact span in the prompt)
and the abstention instruction present in the compiled prompt (or
an InsufficientEvidence result with empty citations). Total
questions become 52; the original 48-question bars are unchanged.
Actual model refusal still needs model-answer evidence and is not
claimed here.

## Verification scope (Linux; Windows rerun by reviewer)

- New and strengthened tests: batch proof tests, Ask batch/counter
  test mirroring the 16-note probe, tamper fail-closed tests,
  wrong-source regression, binding checks in the 52-question test,
  natural negative family.
- Focused Rust suites (lib, ask, ask_recall, ask_streaming, context,
  hybrid, body_search, retrieval_evaluation, large-scale, permission
  batch), strict Clippy, formatting, contracts, TypeScript, docs
  validation, API/web builds, canonical browser Ask workflow.
- The reviewer's probe file itself is untouched; its Windows rerun
  is the reviewer's step, not a Linux claim.
