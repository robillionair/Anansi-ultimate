# M9 SwarmTank Slice 1 — Addendum

Branch: `muse/m9-swarmtank-slice1`
Baseline: `7c576b85eab5d76e8713ff2debf89f1e7a3756d4` (Windows-verified §26 + §27 package)
Design: `docs/handoffs/M9-swarmtank-slice1-design.md` (committed before code, `679a4d47c`)

## What was built

A deterministic, provider-free SwarmTank panel inside Projects / Innovation Studio:

- **Input ingestion (AC-1).** Manual fields (description, pricing display text,
  optional numeric monthly price amount, segment, industry, features, value
  proposition) plus field-copy import from a saved §27 Collider experiment
  (concept brief → description, conceptual primitives → features) or a Forge
  concept (approved brief → description). The live panel makes no provider
  calls at all in Slice 1; the e2e spec asserts fake-provider hit counts are
  unchanged across generate and evaluate.
- **Synthetic persona generation (AC-2).** Presets 20 / 50 / 100, batches of
  10, seeded SHA-256 counter-mode PRNG over the recorded seed string, so the
  same seed reproduces the same population byte-for-byte. Every persona is
  labeled "Simulated persona." in the UI, and every report surface carries the
  §34 disclaimer that these are simulated personas, not real customers.
  Persona generation is a preview: it is not persisted until evaluation.
- **Probabilistic decision model (AC-3).** A pure function in
  `crates/core/src/swarmtank.rs` scores seven documented factors (fit, budget,
  price, trust, switching, clarity, authority), thresholds at ≥0.65 Convert,
  0.40–0.65 Confused, <0.40 Reject, with a hard price-exceeds-budget veto at
  priceSensitivity ≥ 0.5. Each decision records `reason` (all seven factor
  scores at 2 decimals), `keyObjection` (null iff Convert), `perceivedValue`,
  and `willingnessToPay`. Evaluation regenerates the population server-side
  from the recorded seed; the client preview is never trusted.
- **Report (AC-4).** Derived, not stored (the §27 pattern): conversion rate,
  average willingness to pay, top-3 confusion points, top-3 objections,
  categorized breakdown, segment counts by role, and rule-derived
  recommendations in fixed order (price → trust → clarity → fit → switching
  → authority), each with §40 assumption / syntheticEvidence / uncertainty /
  validation fields. Persona table with search and decision filter, plus a
  deterministic JSON export download.
- **Bridge to Forge (AC-5).** "Apply Recommendations to Forge" revises the
  associated concept through `artifact_revise` (expected-version guard),
  adding one optional `swarmtank_addendum` field (run pin id/version/hash,
  top objections, top confusion, recommendations). Re-applying the same run
  returns the existing version unchanged (`alreadyApplied`, the §27
  idempotency pattern). No associated concept → fails closed; a concept is
  never invented.
- **Persistence (AC-6).** One artifact per evaluated run: existing artifact
  type `simulation`, format `anansi-swarmtank-run` v1, with `contentHash`
  (SHA-256 of the doc with the hash field empty) and a full
  field-by-field decision recomputation in `validate_swarmtank_run_doc`.
  Runs ride the existing backup/restore path via the artifacts table; the
  restore gate validates original `simulation` bindings (body hash + doc
  validation) before publication. Runs are never edited; a run is version 1
  forever.

## Commits

1. `679a4d47c` — design before code.
2. `fec308741` — Rust core, API paths, restore gate.
3. `f0eb1df12` — Clippy fixes.
4. `458f4f015` — contracts regen, client methods, concept addendum parsing/view.
5. `17e0255cd` — UI panel, report view, transfer UI, node tests.
6. `127ff0eb8` — e2e spec.

## Known limitations (deferred, not claimed)

- **Run source pins are historical provenance.** The run records the
  experiment/concept id, version, and hash it was imported from, but restore
  does not remap those pins and never dereferences them; the run itself
  stays fully functional after a remap. Remapping source pins on restore is
  deferred to a later slice.
- **No provider-backed generation.** Batching is structural (batches of 10,
  each validated before the next) and is the seam for a future
  separately-authorized provider generator. No claim of "100 separately
  reasoned agents" is made anywhere.
- **Non-goals from the design doc, unchanged:** §38 3D arena; price/feature
  scenario engine; suggested-experiment generation; landing-page URL import
  (in spec §35 but not in the assignment's input list); spec auto-start or
  execution from transfer; native save dialog for export (browser download
  used, as in §27).
- **§27 provenance rule, exercised by this slice's own spec:** restore
  refuses a concept whose source experiment pin does not resolve to a real
  artifact in the same workspace/project. The first draft of the e2e spec
  fabricated such a concept and restore correctly returned 400; the spec
  was rewritten to collide and Build a real concept, which also covers the
  experiment-import path.

## Linux verification evidence

Sequential, on this Linux VM, branch tip `127ff0eb8` (docs commit pending
at packaging time):

- Rust workspace: **562 passed, 4 failed** — the 4 are the known live-Ollama
  `semantic_workflow` environment failures (`StorageUnavailable`, embedding
  model absent), identical family to prior slices. Includes the 9 new
  SwarmTank lib tests and the preserved collider 49 / transfer 19 /
  forge_restore 4 suites.
- Strict Clippy (`--workspace --all-targets -D warnings`) clean; `fmt --check`
  clean; `tsc --noEmit` exit 0; contracts `--check` exit 0; docs validation
  PASS at 212 files; web build clean.
- Node `test:ui`: **131/131** (125 preserved + 6 new).
- Browser battery under the shared lease: **15/15 tests across 14 specs** —
  the new `swarmtank-slice1` spec plus preserved collider, Three.js
  visualization + fallback, Canvas visualization + screenshots, document
  input, URL input, pipeline overview, transfer concurrency, forge,
  artifacts, and backups specs. No regressions.
- Working tree clean at packaging; no schema change; only additive optional
  contract fields (`swarmtankAddendum` on the concept doc).

## Windows rerun checklist (reviewer)

- [ ] `node scripts/cargo.mjs test --locked --workspace` on Windows (expect
      the same totals minus the 4 known live-Ollama `semantic_workflow`
      environment failures).
- [ ] Node `test:ui` (expect 131/131), tsc exit 0, contracts `--check` exit 0,
      docs validation PASS, web build PASS.
- [ ] Playwright: `tests/e2e/swarmtank-slice1.spec.ts` plus the preserved
      collider/transfer/forge/artifacts specs (no regressions).
- [ ] Confirm the SwarmTank panel renders in the desktop shell (Tauri) with
      no provider traffic on load.

## Remaining gates (unchanged by this slice)

Windows verification of this slice (checklist above); M7 native desktop
acceptance + preview promotion; §32 sandbox; M5 production gates; Windows
verification of M8 Slice 4. M8 is NOT complete, and Linux evidence here
does not close Windows verification.
