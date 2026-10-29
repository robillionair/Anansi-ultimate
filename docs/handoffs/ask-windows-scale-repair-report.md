# Ask Windows scale repair — report

Branch `muse/ask-windows-scale-repair` from reviewed checkpoint
`8bdcbae105abdb45e0bcc93669d25994355746a9` (branch
`codex/review-ask-retrieval-verify`), one commit over the
`muse/ask-retrieval-verify` delivery `f90f9e85`. Acceptance was
frozen before implementation in
[ask-windows-scale-repair.md](ask-windows-scale-repair.md). This round
changes production code on purpose (unlike the prior round): the
permission-verification path is the defect. The reviewer's probe file
(`crates/core/tests/ask_windows_scale_probe.rs`) is untouched.

## P1 — Permission-verification cost, bounded per operation

The review measured a public Ask on 16 notes at 27,637 ms on
Windows: every question rebuilds the search index, and rebuild read
every record through `content::read`, which verified three
directories plus the body file through `private_permissions` — one
`powershell.exe` spawn per verification on Windows.

Repair, following the existing `vault/io.rs` batch-proof pattern:

- New operation-scoped `content::VerifiedPaths`: one Windows
  PowerShell process verifies a whole batch (JSON path list on stdin,
  the same `protect.ps1` owner/DACL invariant per item, verify-only
  so nothing is mutated), or per-path metadata checks on other
  platforms. Shared directories are verified once. The proof set is
  created fresh per operation and dropped at its end: no permanent
  positive cache, no cross-operation reuse.
- Ask builds one proof set per question and shares it between index
  rebuild and context compilation. Rebuild also builds its own batch
  when called without proofs, so every rebuild caller benefits and no
  rebuild/integrity behavior is removed.
- Retained: link rejection per path, owner/DACL verification
  (batched, not skipped), body-hash verification per read,
  `MAX_BODY`, UTF-8 validation, privacy/identity checks, Sealed
  exclusion, fail-closed batch failure. A read for a path outside
  the set (revision created mid-operation, or a path absent when the
  batch was built) falls back to full per-path verification, never to
  an unverified read. Revocation takes effect on the next operation,
  when proofs are rebuilt from scratch.
- Unchanged: write paths and seeding cost, semantic-only snippet
  reads in hybrid search (bounded by hit count), timing limits, and
  the rebuild itself.

Linux measurement (16 notes, same shape as the probe): one Ask uses
exactly 1 batch covering 34 paths (2 shared directories + 16 record
directories + 16 files), with 42 individual verification calls
(2N + 10: the batch's 2N + 2 paths plus 8 root/database checks from
the four connections Ask opens). The old path needed about 4N plus
compile re-reads plus those checks. Linux Ask time was 13–30 ms;
that is Linux evidence only and is not a Windows claim. On Windows
the same Ask should need about 5 permission processes (1 batch + 4
connections) instead of about 75, but only the reviewer's rerun of
the same 16-note diagnostic can confirm the Windows time.

Verification found and fixed a real defect: the first version of the
batch failed a workspace with no notes yet (its content directories
do not exist), breaking the existing empty-workspace Ask test. The
fix keeps absent paths out of the batch so they take the old
fallback path; the existing test now passes, and the fix is covered
by the batch tests below.

## P2 — Fact-to-source binding

The large-scale test's success condition searched required spans
anywhere in the compiled prompt. It now requires each span inside
its declared source's actual serialized evidence chunk, where chunk
`n` maps to the citation with `index == n`. A span in another
source's chunk, or in the 200-byte citation preview, does not
satisfy it. A chunk/citation count mismatch fails closed.

Results (1,010 records, 52 questions): answerable binding-complete
40/40, insufficient 8/8, disclosure 48/48, budget 48/48, zero
Sealed/foreign/archived/expired leaks. An isolated regression proves
discrimination: the phrase "77 crates" in a wrong source satisfies
the old whole-prompt search but fails binding to the intended
source, while binding to the carrying source passes.

One marginal shift is reported, not hidden: total cited sources
moved 146 → 145 and total prompt chars 57,265 → 57,271 versus the
prior run. The only corpus change was the three new excluded notes,
which are indexed like the other archived/expired notes; adding
documents to the shared FTS corpus shifts BM25 ranking at the
margin (one question dropped a marginal irrelevant citation, two
prompts changed by 4 chars through chunk truncation order). All 40
answerable questions still bind complete, so this is ranking
marginalia, not a retrieval regression.

## P2 — Natural negative family, retained separately

Four fresh natural questions reproduce the cycle-2 collision shapes
(Alder/Aldermere prefix, observatory, keeps/keep stemming, holds in
filler) with fresh excluded values, so an excluded-fact leak is
attributable to exactly one note. The cycle-2 wordings were not
preserved verbatim, so these are new writings in the same shapes,
not claimed restorations. Result: 4/4. All four returned Offline
with partial context (cited 4, 1, 1, and 5 sources), no excluded
content, no excluded fact span, the abstention instruction present,
and disclosure/budget clean. Actual model refusal still needs
model-answer evidence; an instruction in a prompt is not evidence a
model obeyed it.

## Verification (Linux)

- Focused Rust 86/86: lib 57 (including the new batch unit test),
  ask_anansi 5, ask_recall 2, ask_streaming 1, context_compiler 10,
  hybrid_retrieval 6, body_search 1, retrieval_evaluation 1,
  large-scale 2 (main + wrong-source regression), permission batch
  1, Windows probe 0 (Windows-only file, compiles clean).
- Strict Clippy, formatting, contracts, TypeScript, docs validation,
  API and web builds: all clean. Canonical browser Ask workflow:
  1/1.
- The reviewer's Windows probe and full 1,010-record Windows run are
  the reviewer's steps, not Linux claims.

## Held and open, unchanged by this round

- Part 1 cloud run still HELD: 0/31 attempts spent, frozen payloads
  preserved, no new provider call in this repair (the review also
  made zero calls and does not authorize another round).
- Windows acceptance of this repair (16-note probe rerun, then full
  scale), the frozen cloud run (needs the credential through the
  secure flow), M7 native desktop acceptance and preview promotion,
  M5 production gates, the §32 sandbox, G1 audit, and remote CI all
  remain open. No push, merge, CI, publication, preview promotion,
  or generated-code execution happened here.
