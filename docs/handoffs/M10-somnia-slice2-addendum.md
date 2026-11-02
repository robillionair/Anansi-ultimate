# M10 SOMNIA Slice 2 — Delivery Addendum

## What shipped

Slice 2 (branch `muse/m10-somnia-slice2`, from owner tip
`8ba785aeb`) adds, strictly per MASTER_SPEC §45, §47, §87.6,
§87.9, plus one bounded §39 copy repair:

1. **Seeded nightmare generator (§45).** Pure and reproducible:
   FNV-1a seed state + xorshift32 stream, exact integer
   arithmetic in both the Rust reference implementation
   (`generate_nightmares` in `crates/core/src/somnia.rs`) and
   the TS preview mirror (`generateNightmares` in
   `Somnia.tsx`). Count 5 / 10 / 20 in the UI (function accepts
   1–20). Case `i` uses class `CLASSES[i % 7]`: conflicting
   instructions (R2), missing evidence (R3), simulated tool
   error (R4, prior calls counted), simulated tool timeout
   (R5, prior calls counted), untrusted retrieved instruction
   (R1, 0 calls), cost-limit exhaustion (Complete with a
   `constraint-violated` violation, decision unchanged), and
   privacy-boundary challenge (one sealed record → whole input
   refused fail-closed at preview, evaluate, and save). The
   `CATEGORIES` allow-list grows 6 → 10 for the four new
   scenario categories. A golden test pins the sequence
   byte-identically in both suites (seed
   `somnia-nightmare-golden`, count 7).
2. **Review-only inert consolidation proposals (§47, §87.6).**
   Pure derivation over a saved evaluation doc, computed at
   view time in TS with a Rust mirror and golden agreement:
   `flag-missing-evidence` (id absent in ≥ 2 scenarios),
   `archive-untrusted` (source in ≥ 2 untrusted records),
   `procedural-rule` (tool with ≥ 2 timeouts; cost-limit
   rule exceeded in ≥ 1 scenario), `supersede-conflict`
   (instruction conflict in ≥ 2 scenarios). The panel shows
   the exact proposed change, rationale, and diff-style
   preview, and offers NO apply button, NO mutation op, and
   NO API call. §87.7 approve/reject is review
   acknowledgement only (local-only "Mark as reviewed"
   toggle, never sent or persisted). No new artifact type,
   no restore-gate change, no schema change: proposals
   recompute identically from a restored doc.
3. **Before/after comparison studio (§87.9).** Two selects
   over saved evaluations (read-only), signed metric deltas
   (Complete / Refuse / Abort / simulated calls / violations),
   a violation-category matrix with "newly exposed" rows
   (baseline 0, comparison > 0), and deterministic JSON /
   Markdown export (no timestamps, no wall time) via
   download.
4. **SwarmTank wording repair (§39).** Button `🤖 Re-run Full
   AI Simulation` → `Re-run Deterministic Scenario
   Simulation`, with help text explicitly stating the button
   invokes the deterministic model-free PRNG evaluator using
   the specified seed and does not perform live AI inference
   or contact a model provider. No live-AI mode was added:
   §39's "Re-run AI Simulation" describes a model-generated
   reassessment mode that does not exist in this product.

## Slice 1 Windows verdict (recorded, not re-run on Linux)

The Slice 1 delivery was independently reviewed and verified
on Windows NTFS (reviewer's evidence, per the Slice 2
assignment): `tsc --noEmit` exit 0, `contracts --check`
exit 0 (zero drift), docs validation PASS (223 files),
`npm run test:ui` 159/159, `cargo test -p anansi-core
--lib somnia` 12/12, full workspace 583 passed (0 failures,
4 pre-existing offline-Ollama skips), Vite production build
clean in 2.23s, browser battery under shared lease 12/12
(SOMNIA Slice 1 2/2, collision regressions 3/3, SwarmTank
regressions 7/7). The named review commit `37970d3a` was
not available locally and no review ZIP arrived with the
Slice 2 assignment, so Slice 2 builds on owner tip
`8ba785aeb` per the assignment's stated fallback, and this
addendum carries the verdict. On that verdict, AN-015
advances `in progress` → `verified` for the Slice 1 scope
only; Slice 2 is a new scope and is NOT verified by it.

## Linux verification (sequential)

- Rust workspace: **589 passed + 4 known live-Ollama
  environment failures** (583 + 6 new Slice 2 tests; the 4
  failures are the same `semantic_workflow` family as prior
  slices, `StorageUnavailable` with the embedding backend
  absent on this VM).
- Rust SOMNIA lib suite: **18/18** on the final tree (12 +
  6 new: golden sequence, determinism/bounds, per-class
  decisions, sealed refusal, proposal derivation, comparison
  math).
- Strict Clippy clean (one real finding fixed during the
  run: a nested `if let` chain in the proposal derivation
  collapsed), `cargo fmt --check` clean.
- Node `test:ui`: **165/165** (159 + 6 new Slice 2 tests).
- `tsc --noEmit` exit 0, `contracts --check` exit 0 (zero
  drift; the `uint` format lines are pre-existing validator
  noise on untouched schemas), docs validation PASS at 224
  files (223 + this addendum), web build clean.
- E2E `somnia-slice2.spec.ts` **2/2**: generate 10
  nightmares → sealed refusal → explicit exclusion → run 9
  (Complete 1, Refuse 5, Abort 3, calls 7) → save → exactly
  the two threshold-biting proposals for this seed (no apply
  control) → comparison vs the built-in baseline (Refuse
  +2, Abort +2, calls +4, violations +4, tool-error newly
  exposed) → JSON and Markdown exports asserted →
  backup/fresh restore → proposals and comparison identical
  → zero provider traffic throughout.
- Regressions green: Slice 1, evaluations, integrated
  workflow, opportunities, SwarmTank slice 1 / arena /
  scenario.
- Browser battery: **47/48 specs, 64/65 tests**. The single
  failure is `search.spec.ts`'s real-semantic test, which
  needs live Ollama embeddings absent on this VM and
  previously failed identically on pristine baselines. The
  battery initially showed one additional failure in
  `somnia-slice1.spec.ts`: a genuine Slice 2 regression —
  the new "Exclude sealed case" button and its explanation
  added a second `/sealed/` match inside the preview-errors
  section, breaking that spec's page-level assertion. Fixed
  by scoping the Slice 1 assertion to the error list item
  (same fix shape as the Slice 2 spec's own assertion);
  Slice 1 + Slice 2 specs pass 4/4 on rerun.

## Known limitations and honest notes

- The generator's variation is template slots over fixed
  word lists driven by a small PRNG — enough for
  reproducible failure-class coverage, not a model of real
  agent behavior. Nothing here proves a real agent improved.
- The sealed nightmare case can never run: that is the
  design (fail-closed), not a gap — the panel requires an
  explicit user exclusion before the rest can run.
- Proposal thresholds (≥ 2, cost ≥ 1) are Slice 2
  conventions, stated in the design doc, not measured
  optima.
- Comparison inputs are saved evaluations only; the
  current unsaved run joins the list after Save.
- The SwarmTank re-run button still reuses the
  deterministic evaluate op and makes no model call — the
  repair makes the label honest about that; it does not add
  a live-AI mode.
- Windows verification of Slice 2 remains the reviewer's
  step (checklist in the delivery MANIFEST).

## Requirement coverage

- AN-015: `verified` for the Slice 1 scope (Windows review
  evidence above); Slice 2 scope is `in progress` in this
  delivery's terms and awaits its own Windows rerun.
- §87 items now covered: 1–4 (Slice 1), 5 (recurring
  weaknesses via proposals), 6 (review-only proposals),
  8 (re-run via nightmare regeneration), 9 (comparison
  studio). Item 7 (approve/reject) is review
  acknowledgement only in this slice; item 10 (restore the
  previous version) remains open — backup/restore survival
  is verified, version rollback is not claimed.

## Recommended next slice

§50 dashboard aggregates over saved SOMNIA evaluations
(failure rate, coverage, pending proposals) as a read-only
view, still manual-only and still without lesson promotion
or scheduling. §48 lesson promotion, §49 scheduling, §51
Forge integration, and historical ingestion stay out until
separately approved.
