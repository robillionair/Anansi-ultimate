# M9 SwarmTank Slice 3 — Addendum

## What shipped

The final M9 slice: interactive scenario controls over a saved SwarmTank run,
with an instant local sensitivity engine, dynamic arena re-clustering, a
side-by-side comparison studio, versioned scenario-branch persistence, a
Forge bridge that can apply baseline or scenario recommendations, and
deterministic JSON/Markdown comparison export. Design committed before code
in `docs/handoffs/M9-swarmtank-slice3-design.md`; this addendum records what
changed during implementation and what the evidence actually shows.

## Implementation notes (deviations and corrections)

- **Baseline engine frozen.** `evaluate_persona` is untouched: the scenario
  evaluator is a separate function taking baseline decisions as input, so
  stored-run restore recomputation is unaffected. The scenario veto
  threshold (≥ 0.7) deliberately differs from the baseline veto (≥ 0.5);
  both are covered by tests, including the case where a baseline-Reject
  re-evaluates to Convert under a scenario.
- **Test-helper slimming.** The Rust `scenario_persona` test helper lost its
  signals/constraints parameters (8 → 6) to satisfy strict Clippy; the one
  test that needs them (soc2) sets the fields after construction.
- **Test-fixture corrections, not code fixes.** Two node-test expectations
  were wrong on first run and were corrected after reading the engine: the
  transition test's objection counts (a migration that swaps one objection
  for another leaves both counts unchanged unless a third persona joins),
  and scenario fixtures that left `activeFeatures` empty (which drops
  clarity from 4 fields to 3 and changes scores). The e2e lever-B test had
  the same class of error (a need can be covered by the value proposition,
  which no toggle removes); it now verifies its lever against the TS engine
  in-test before driving the UI.
- **Ring-line set.** The prebuilt ring set is computed from the layout
  module's spill rule (base + ring × 26, capacity floor(2πr/14)) up to 200
  agents per group, which covers the design's listed radii and more; no
  geometry is created or destroyed during transitions.
- **Orbit discontinuity, disclosed.** A Confused agent's orbit offset
  (angle + t × 0.05) appears or vanishes in one frame when its decision
  flips to or from Confused, because the orbit rides on top of the
  interpolated base angle. Positions, colors, and markers are otherwise
  continuous; the effect is a single-frame snap of one agent's angle, not
  a jump in position.
- **Badge text kept, behavior honest.** The assignment's badge strings
  (`⚡ Instant Synthetic Sensitivity Calculation`,
  `🤖 Re-run Full AI Simulation`) appear verbatim in the UI. The re-run
  button reuses the existing deterministic `evaluate` op with the scenario
  inputs and the same seed; adjacent text states it makes no model call.
  No model-generated SwarmTank path exists in this codebase, so no stronger
  claim is made.
- **Branch parent pins are historical provenance.** Like Slice 1 run source
  pins, a branch's parent pin is shape-checked and never remapped or
  dereferenced, so branches stay valid across backup/restore remaps. The
  panel's branch view falls back to synthesizing the display run from the
  branch's embedded baseline when the parent record cannot be found by pin.
- **Reason formatting.** Scenario reason strings use integer-cents
  formatting in both mirrors (Rust `fmt2`, TS `fmt2`), never Rust `{:.2}`
  vs JS `toFixed(2)`, which round exact halves differently. Baseline
  reasons keep Rust `{:.2}` and are never cross-compared.

## Linux evidence (sequential)

- Rust `swarmtank` suite: 18/18 (9 Slice 1 + 9 new scenario tests).
- Rust workspace: 571 passed + the 4 known live-Ollama `semantic_workflow`
  environment failures (identical test names as Slice 1/2; Ollama not
  listening on this VM).
- Strict Clippy (`--locked --workspace --all-targets -D warnings`) clean;
  `cargo fmt --check` clean.
- Node `test:ui`: 152/152 (138 + 9 scenario + 5 arena-animation).
- `tsc --noEmit` 0 errors; contracts `--check` clean; docs validation PASS
  (219 files).
- Web build clean.
- Browser battery: 45/46 specs, 60/61 tests. The single failure is
  `search.spec.ts`'s real-semantic test (paraphrase retrieval with zero
  keyword overlap), which needs live Ollama embeddings; Ollama is not
  listening on this VM, and this exact test was previously proven to fail
  identically on the pristine baseline. One battery failure was found and
  fixed during the run: the Slice 2 arena spec's no-WebGL test used a
  loose `getByText('Conversion rate:')` locator that now matches three
  texts (baseline metrics, scenario metrics bar, comparison); the spec
  now matches the baseline line by shape and passes 3/3 on rerun.
- New e2e spec `swarmtank-scenario.spec.ts` 3/3: price/feature/setup
  controls driving the metrics bar, comparison, and arena re-clustering
  (including a canvas click on a computed scenario agent); data-driven
  objection resolution (free-trial or feature-drop lever, engine-verified
  in-test); branch save → Forge v3 with `swarmtankScenarioDiff` →
  backup/fresh restore of both run and branch. Zero provider traffic in
  all three tests.

## Still open (not claimed)

- Independent Windows rerun of this slice by the reviewer (checklist in the
  delivery MANIFEST, including desktop-shell confirmation).
- M7 native desktop acceptance + preview promotion; §32 sandbox; M5
  production gates; Windows verification of M8 Slice 4.
- Deferred, not built: provider-backed re-evaluation, per-agent 3D labels,
  native save dialog, §39 headline/value-prop/audience edits (out of the
  assignment's §3.B schema; clarity inputs untouched).
- The orbit-offset discontinuity above, and the DNS resolver-thread
  limitation carried from the URL slice, remain disclosed limitations.
