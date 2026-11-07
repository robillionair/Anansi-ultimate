# SOMNIA automatic memory compaction — library + frozen benchmark

## §1 Scope and baseline

- **Baseline (verified, no substitute):** `muse/swarmtank-v2` at
  `bb1cc6e6dc779499105c27db70666cb2605f3e13` (SwarmTank v2 delivery).
  Branch `muse/somnia-compaction`; the exact delivery tip is
  recorded in the package MANIFEST.
- **This slice (Checkpoint 1):** the compaction **library and
  tests** only, plus the frozen benchmark and independent oracle.
  Design + acceptance criteria were committed before code
  (`docs/handoffs/somnia-compaction-design.md`, AC-C1..AC-C16).
  Commits on `muse/somnia-compaction`: design `194cedf0f` →
  library `c4590cb78` → benchmark `a3a908551` → clippy fix
  `81893ac37` → handoff (this document). The exact delivery tip
  is recorded in the package MANIFEST.
- **Deliberately not in this slice:** UI, provider path, new API
  operation, new artifact type, contract changes, model calls.
  Those wait for Antigravity's accepted checkpoint, which includes
  newer SOMNIA work unavailable in this baseline (§6). No provider
  call was made anywhere in this slice.
- **Priority rule honored:** no Windows review findings arrived
  during this slice, so nothing pre-empted the work.

## §2 What was built

### Library (`crates/core/src/somnia_compaction.rs`, new module)

Pure functions: no I/O, no clock, no model, no network. Record
text is data throughout — grouped, ordered, selected, never
interpreted.

- **Input formats (§2 of the design).** Format A: structured JSON
  `anansi-somnia-compaction-input` v1 (deny-unknown-fields per
  record; whole-input refusal for wrong format/version or extra
  top-level fields). Format B: text records, one per line
  (`ID= | SRC= | REV= | KEY= | KIND= | TRUST= | [PRIVACY= |
  [SUPERSEDES=] TEXT=`, `TEXT` last so text may contain ` | `).
  Blank lines and `#` comments skipped.
- **Quarantine (§2.1).** Per-record damage (bad kind/trust/
  privacy, sealed, empty or over-long text, bad id/key/source, a
  `supersedes` id naming no record in the input, duplicate id,
  malformed text line) quarantines that record with a reason and
  lets the rest proceed. Nothing is repaired by guessing.
- **Compaction rules (§3).** Canonical sort by
  `(key, source, revision, id)`; exact duplicates within one
  source excluded as `duplicate-exact` (same text from two sources
  is corroboration, both retained); supersession explicit-only
  (a newer revision supersedes an older record only when it lists
  the id — otherwise the older record is retained with status
  `older`); deletions always retained and definitionally
  supersede earlier same-(key, source) revisions; same-revision
  conflicts all retained and flagged, no winner; uncertainty
  always retained; look-alike keys never merged (exact-match
  only, no normalization); unverified material structurally
  segregated from trusted evidence; every exclusion carries one
  of exactly three reasons.
- **Candidate (§4).** Retained sections (trusted / unverified /
  conflicts / deletions / uncertainties, one section per record
  by precedence deletion → conflict → uncertainty → trust),
  excluded-with-reason, quarantine, stats, `input_hash`, and a
  fail-closed `content_hash` (SHA-256 of the canonical
  serialization, hash field empty). `validate_candidate`
  recomputes the hash and the stats and refuses tampering.
- **Bounds (§3.1).** At most 200 records, 200,000 total input
  chars, 2,000 chars per record; over-limit input is refused
  with a named error, never truncated.

### Clarifications found during implementation (design unchanged)

1. **A `supersedes` link naming a deletion record is vacuous.**
   Deletions are never superseded; the link does nothing. (Unit
   test `supersedes_link_to_deletion_is_vacuous`.)
2. **Timings are recorded in microseconds, not milliseconds.**
   Every benchmark case compacts in under a millisecond
   (111–1,740µs), so millisecond integers would all read 0. The
   results JSON carries `compactionMicros` /
   `totalLatencyMicros`. The design's intent — record compaction
   time per case — is preserved with better precision.
3. **Two unit-test failures during development were test bugs,
   not library bugs.** `same_revision_deletion_and_fact_is_conflict`
   expected both records in the conflicts section, but the design's
   §4 precedence puts the deletion in the deletions section (both
   still carry the conflict flag). `unverified_segregated_from_trusted`
   used the same revision for both records, which is a conflict
   by §3.5, not a trust split; the test now uses different
   revisions. Both were fixed in the tests; the library was
   behaving per the design in each case.

### Frozen benchmark (`crates/core/tests/fixtures/somnia-compaction/`)

- Generator `generate-benchmark.py` (stdlib only; FNV-1a +
  xorshift32 over seed `somnia-compaction-bench-v1`).
  **40 cases** across 9 categories: updates 6, deletions 5,
  conflicts 5, missing-facts 4, look-alikes 4, unicode 4,
  broken-rows 5, prompt-injection 4, keep-more-context 3.
  Per-case files + `MANIFEST.json` (seed, generator version,
  case count, per-file SHA-256).
- **Freeze proven:** regenerating into a scratch directory
  reproduces all 40 case files + MANIFEST byte-identically
  (evidence `generator-determinism.log`; the only diff entry is
  the generator script itself, which is not a generated file).
  The manifest test re-verifies every SHA-256 on each run.
- **Oracle** (`crates/core/tests/somnia_compaction_benchmark.rs`,
  independent of the compactor — it reads only case annotations
  and the candidate): needed-id coverage per question,
  forbidden ids never retained, missing facts stay missing
  (abstain + no retained record contains the missing fact),
  injection containment (nothing excluded because of content;
  injected text appears only inside record text), and the
  coverage invariant (retained ∪ excluded ∪ quarantined covers
  every input record exactly once). Every case stays in the
  results, failures included.

## §3 Verification evidence (Linux, this branch)

- Rust workspace lib: **300/300** (api 4 + core 235 + desktop 1
  + providers 60). Core grew 212 → 235 with the 23 new
  `somnia_compaction` module tests. Full log: `evidence/rust-lib.log`.
- Module tests: **23/23** (`evidence/rust-compaction-module.log`).
- Benchmark oracle: **40/40 cases pass** (2 test fns: manifest +
  all-cases). Total compaction **9,186µs** for all 40 cases
  (per-case 111–1,740µs). Provider tokens: **not-measured**
  (library level; no provider involved — recorded as such, never
  zero-filled). Per-case results JSON:
  `evidence/benchmark-results.json`; run log:
  `evidence/benchmark-run.log`.
- `tsc --noEmit`: **0**. Contracts `--check`: **0 drift**.
  Docs validator: **314 PASS** (313 at baseline + the design doc).
- UI tests: **326/326** (unchanged count — no UI touched).
- Clippy (`-p anansi-core --lib`): exit 0; **zero mentions** of
  `somnia_compaction` (one `type_complexity` warning in the new
  module was fixed by naming the duplicate-group key type; the
  remaining 14 warnings are pre-existing in `mcp.rs` /
  `swarmtank_ai.rs` / others). `rustfmt --check` clean on both
  touched Rust files.
- Focused e2e regressions (SOMNIA slice 1–3 + SwarmTank market,
  AI, AI-timeline, LARP AI, LARP regression, arena): **22/22
  passed** in 2.1 min (SOMNIA 6, market 3, AI 4, timeline 4,
  LARP AI 1, LARP regression 1, arena 3). API build exit 0
  (only the pre-existing `mcp.rs:715` warning); web build exit 0
  ("built in 2.33s", chunk-size warning only). Log:
  `evidence/e2e-regressions.log`.

## §4 Known limitations

- The library is unreferenced by any API operation or UI panel:
  it cannot be exercised through the application until the
  integration slice wires it in. That is the intended shape of
  this checkpoint, not a gap in it.
- Library-level "answer accuracy" is oracle retention coverage
  (§5.3 of the design), not model accuracy. Whether a model
  answers correctly from a compacted history is measured only
  by the app-workflow benchmark run after integration.
- The compactor never infers that a newer revision restates an
  older one (explicit-only supersession). Histories without
  `supersedes` links therefore retain older revisions; the
  candidate is conservative by design, and the benchmark's
  keep-more-context cases prove the retention.

## §5 Proposed global-status updates

**None to AN-015.** Milestone 10 stays `verified` / "Milestone
10 complete" as reviewed; this is new scope on top of the
verified milestone (the SwarmTank v2 precedent on verified M9),
not a reopening of it. No new requirement row is proposed in
this slice; the integration slice will propose one when it wires
the import → candidate → approve → compare → save workflow.
Provider-backed evaluation and historical ingestion remain
deferred in AN-015; this slice implements neither.

## §6 Integration slice (deferred — blocked on Antigravity's checkpoint)

After Antigravity's accepted checkpoint arrives (newer SOMNIA
work unavailable in this baseline), the next slice wires the
library in, in this order:

1. Import UI for the two formats (synthetic or explicitly
   approved records only; sealed refused; quarantine displayed
   for review).
2. Candidate inspection view (retained sections, excluded with
   reasons, quarantine) + explicit approval step. No inference
   before approval.
3. Provider path: compare both histories (original vs compacted)
   with the same configured model through the existing provider
   layer, local-first default, Zero-Cost Lock checked up front,
   existing privacy/disclosure/authorization/cost gates. A failed
   provider call is visible in the experiment history.
4. Save the complete experiment as a new artifact type (input +
   candidate + both runs + metrics, fail-closed hash),
   reopenable without rerunning inference.
5. Run the frozen benchmark through the actual SOMNIA
   application workflow and record, per case: compaction time,
   retained evidence, answer accuracy, abstentions,
   provider-reported input/output tokens, failures, total
   latency. Keep every case in the results. Real-model
   verification depends on an available authorized provider;
   synthetic-provider tests remain labelled.
6. Windows review findings, if any, are repaired before any
   scope expansion.

## §7 Windows acceptance checklist

- [ ] Bundle + ZIP hashes verify; branch fetches to the exact tip.
- [ ] Rust workspace lib suite green on Windows (300 tests at
      Linux parity + the 4 known live-Ollama environment failures
      if the Windows host has no local model either).
- [ ] Benchmark oracle 40/40 on Windows (same frozen fixtures).
- [ ] tsc 0, contracts 0 drift, docs validator PASS, UI 326/326.
- [ ] Focused e2e regressions green (SOMNIA slice 1–3 +
      SwarmTank market/AI/timeline/LARP/arena).
- [ ] Confirm no UI, API, contract, or artifact-type change is
      present in this slice (library + tests + fixtures only).
