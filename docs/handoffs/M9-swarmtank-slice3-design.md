# M9 SwarmTank Slice 3 — Design: Interactive Scenario Controls & Dynamic Sensitivity

Branch: `muse/m9-swarmtank-slice3` from Windows-verified Slice 2 baseline
`3d0537ea` (review branch `codex/review-m9-swarmtank-slice2`, docs-only delta
over Slice 2 runtime `855320d93`: Windows review report + 3 screenshots).

Authority: MASTER_SPEC §39 (scenario controls), §40 (report), §41 (Forge
bridge), §65 (performance/accessibility); `plans/07-swarmtank.md` Slice 3;
requirement `AN-014`. This design is committed before implementation code.

## 0. Non-negotiable invariants carried from Slices 1–2

- **The baseline engine does not change.** `evaluate_persona` in
  `crates/core/src/swarmtank.rs` (7 factors, thresholds 0.65/0.40, veto at
  priceSensitivity ≥ 0.5 with objection `"Price exceeds budget"`) is frozen.
  Changing it would invalidate every stored run: restore validation recomputes
  baseline decisions and compares field by field. All Slice 3 rules below live
  in a *separate* scenario evaluator that takes baseline decisions as input
  and never rewrites them.
- **Client data is never trusted for storage.** The browser computes scenario
  decisions instantly for display (TS mirror), but a saved branch is always
  recomputed server-side by the Rust mirror from the stored parent run.
- **Everything is synthetic and labeled.** Scenario outputs are local rule
  calculations, not fresh LLM evaluations and not evidence that real
  customers changed their minds. The two modes stay visually separate (§7).
- **No new provider traffic, no credentials, no paid inference.** The
  scenario engine is pure arithmetic; the e2e asserts fake-provider hits
  unchanged across every scenario interaction.

## 1. Scenario parameters (TS + Rust, same shape)

```ts
type ScenarioTrustSignals = {
  soc2Security: boolean;
  freeTrialAvailable: boolean;
  slaGuarantee: boolean;
  moneyBackGuarantee: boolean;
};
type ScenarioSetupDifficulty = 'effortless' | 'moderate' | 'complex';
type SwarmtankScenario = {
  priceMonthly: number | null; // null = keep the baseline price
  activeFeatures: string[]; // subset of the baseline run's input features
  trustSignals: ScenarioTrustSignals;
  setupDifficulty: ScenarioSetupDifficulty;
};
```

Rust: `SwarmtankScenario` with `serde(rename_all = "camelCase",
deny_unknown_fields)`. Bounds (enforced identically in both mirrors):

- `priceMonthly`: null, or finite and ≥ 0. When the baseline run states a
  numeric price, the scenario price must also be ≤ 10× that baseline price
  (the assignment's 0–10× band). When the baseline states no numeric price
  there is no relative cap; the UI slider then anchors at the run's maximum
  persona budget instead of an invented base.
- `activeFeatures`: ≤ 50 items, each 1–300 chars, no duplicates, every item
  a member of the baseline run's input features (subset check, not free text).
- `trustSignals`: exactly the four keys, all booleans.
- `setupDifficulty`: exactly one of the three literals.

UI defaults for a fresh scenario over a run: price = baseline price (null
when the baseline has none), all baseline features active, all trust signals
off, difficulty `moderate`. `moderate` means "no setup override" (the
baseline switching factor is kept), so the default scenario is exactly the
baseline run: the transition matrix is the identity, which the tests assert.
"Reset to Baseline" restores exactly these defaults.

Presets (enabled only when the baseline states a numeric price): `-50%`,
`Baseline`, `+50%`, `+100%` set `round2(baseline × factor)`; `Free Tier ($0)`
sets 0. The slider range is 0 to 10× baseline price when present, else 0 to
the run's maximum persona budget; a number input accepts exact values in
both cases. §39's headline / value-proposition / audience edits are NOT in
this slice's schema: the assignment's §3.B defines the surface, and messaging
edits stay deferred (they would change the clarity factor's inputs, which
this slice deliberately does not touch).

## 2. Sensitivity model (the math both mirrors implement)

Per persona, from the baseline input, the baseline decision, and the
scenario parameters. The factor functions are the Slice 1 functions with the
scenario substitutions below; weights (0.25/0.20/0.15/0.10×4) and thresholds
(0.65/0.40) are unchanged.

### 2.1 Scenario input

`scenarioInput = baselineInput` with `price_amount := scenario.priceMonthly
?? baseline.price_amount` and `features := scenario.activeFeatures`.

### 2.2 Factors

- **fit**: `fit_factor(persona, scenarioInput)` — the Slice 1 word-overlap
  rule (a need counts as covered when any word longer than 3 chars appears
  in the joined feature + value-proposition text, case-insensitive) run over
  the *active* features. Dropping features therefore lowers fit through the
  same rule the baseline used; no second fit definition exists.
- **budget, price**: the Slice 1 price branch over the scenario price. When
  `moneyBackGuarantee` is on, `price = min(1, price + 0.10)` (risk reduction).
- **trust**: `effectiveSkepticism = clamp(skepticism − (freeTrialAvailable ?
  0.20 : 0), 0, 1)`; `trust = 0.75 − 0.5 × effectiveSkepticism`. When
  `soc2Security` is on and the persona's required trust signals or purchase
  constraints contain "soc", "security", or "compliance" (case-insensitive),
  `trust = max(trust, 0.80)` — the floor is the mechanism by which SOC 2
  "satisfies enterprise constraints". When `slaGuarantee` is on,
  `trust = min(1, trust + 0.05)`.
- **switching**: `effortless` → 1.0; `moderate` → the baseline switching
  factor (`1 − effort_class × 0.5`); `complex` → `min(baseline switching,
  0.5)`.
- **clarity, authority**: unchanged from the baseline computation (same
  input fields, same archetype table). The TS mirror therefore needs the
  archetype table's per-role `effort_class` and `authority` (12 roles);
  that table is part of the mirror and must be updated if archetypes change.
- **score, perceivedValue, willingnessToPay**: the Slice 1 formulas over the
  scenario factors: `score` by weights; `perceivedValue =
  round2(0.6 × fit + 0.4 × score)`; `willingnessToPay =
  clamp(round(budget × perceivedValue × (1 − 0.5 × priceSensitivity)), 0,
  budget)`. The willingness-to-pay elasticity curve is this recomputation:
  as the scenario price rises, the price factor falls, perceived value falls,
  and willingness to pay falls with it — the comparison view plots the
  baseline vs scenario averages, it does not fit a separate curve.

### 2.3 Decision and the override ladder (applied in this order)

Base decision from the scenario score (≥ 0.65 Convert, ≥ 0.40 Confused,
else Reject). Then:

1. **Scenario price veto (strongest).** Scenario price > 0, price >
   approximateBudget, and priceSensitivity ≥ 0.7 → Reject with objection
   `"Price exceeds budget under high price sensitivity"`. This is a
   *scenario-only* rule: it deliberately differs from the baseline veto
   (≥ 0.5, `"Price exceeds budget"`), which stays frozen for baseline
   decisions. A scenario veto never rewrites a baseline decision.
2. **Feature-drop downgrade.** If the baseline decision was Convert, the
   scenario base decision is still Convert, at least one need was covered
   at baseline, and zero needs are covered under the scenario features →
   Confused with objection `"Missing critical required feature: <first
   baseline-covered need now uncovered>"`. "Core priority" is operationalized
   as a *need* covered at baseline: needs are the concrete requirements in
   the persona record, while priorities are abstract categories ("Price",
   "Speed") that cannot name a feature. The rule only downgrades; it never
   upgrades, and the veto above wins over it.
3. **Complex-setup downgrade.** If `setupDifficulty` is `complex`, the
   baseline decision was Convert, the scenario base decision is still
   Convert, and `expectedImplementationEffort` is `"Low"` → Confused with
   objection `"High setup overhead exceeds team capacity"`. "Low technical
   capacity" is operationalized as expecting a light setup: personas who
   expected little implementation effort are the ones a complex setup
   exceeds. Only downgrades, veto wins.
4. **Free-trial resolution (the only upgrade).** If `freeTrialAvailable` is
   on, the persona's role is `Cost-sensitive freelancer` or `Small agency
   owner`, the baseline decision was Confused, and the baseline key
   objection is `"Price sensitivity"` or `"Missing trust signals"` →
   Convert with no objection. Rationale, stated plainly: the skepticism
   cut alone moves the score by only 0.01 (trust weight 0.10), too small to
   observably "resolve hesitation objections" as the assignment requires;
   the explicit rule makes the named segments' resolution observable and
   testable. It applies only to baseline Confused, so it cannot fight rules
   2–3 (which require baseline Convert) and cannot override a veto (rule 1
   runs first and a vetoed persona is Reject, not Confused).

Result invariants, checked in both mirrors' tests: Convert carries no key
objection; Confused/Reject always carry one; perceivedValue and score stay
in [0, 1]; willingness to pay stays in [0, budget]; the default scenario
reproduces the baseline decisions exactly.

### 2.4 Number formatting (the cross-implementation trap)

Rust `{:.2}` and JS `toFixed(2)` round half-values differently (banker's vs
half-up on exact binary values), so scenario reason strings must NOT use
either. Both mirrors format factors with integer cents: `cents =
round_half_away(v × 100)` (Rust `(v * 100.0).round() as i64`, TS
`Math.round(v * 100)`), then `"<whole>.<two-digit cents>"`. `round2` itself
is already half-away in both (`(v*100).round()/100` vs
`Math.round(v*100)/100`). Scenario validation compares reason strings
exactly, so this shared routine is load-bearing, not cosmetic.

## 3. Transitions, metrics, comparison

For a baseline run and a scenario decision set over the same personas:

- **Transition matrix**: counts for all 9 baseline→scenario pairs (only
  nonzero pairs stored), plus `convertedIds` (scenario rank strictly better
  than baseline: Convert > Confused > Reject), `lostIds` (strictly worse),
  and `unchangedCount`.
- **Objection deltas** (aggregate, by objection string): `resolved` =
  objections whose count fell baseline→scenario, `introduced` = whose count
  rose, each as `{item, baselineCount, scenarioCount}` sorted by absolute
  change descending, then by string. Per-persona migration is derivable from
  the two decision sets and is rendered from them, not stored twice.
- **Metrics** (baseline and scenario): converts/confused/rejects/total,
  conversionRate (round2 percent), averageWillingnessToPay (round2),
  averagePerceivedValue (round2).
- **Price elasticity estimate**: `((conv_s − conv_b) / conv_b) /
  ((price_s − price_b) / price_b)` when both denominators are nonzero,
  else null. It is a single-scenario comparison, not a fitted curve.
- **Optimal price recommendation** (for the Forge diff): if scenario
  conversion exceeds baseline, `"Scenario price $X raises synthetic
  conversion by Y points vs the baseline price $B"`; otherwise `"No tested
  scenario price improved conversion; the baseline price $B remains the
  synthetic optimum"`. This recommends between the two tested prices only;
  no price sweep or optimizer is claimed.

## 4. Persistence: scenario branches

### 4.1 Document schema (`anansi-swarmtank-scenario`, version 1)

```ts
{
  format: 'anansi-swarmtank-scenario',
  version: 1,
  parentArtifactId: string,   // provenance only (see §4.3)
  parentVersion: number,
  parentContentHash: string,  // 64 hex
  scenario: SwarmtankScenario,
  personas: SwarmtankPersona[],        // full copies from the parent run
  baselineInput: SwarmtankInput,
  baselineDecisions: SwarmtankDecision[],
  decisions: SwarmtankDecision[],      // scenario decisions
  transitions: { from, to, count }[],
  metrics: { baseline: Metrics, scenario: Metrics, priceElasticity: number | null },
  contentHash: string,
}
```

The branch is **self-contained on purpose**: it embeds the parent's personas,
input, and baseline decisions, so validation recomputes everything from the
document itself — baseline decisions via `evaluate_population`, scenario
decisions via the scenario evaluator, transitions and metrics by
recomputation — and then checks the content hash. No parent lookup is needed
at validation time.

### 4.2 Registration

New backend op `swarmtank-scenario` (allowlist + prefix routing + HTTP path,
the Slice 1 pattern): request `{contract_version, workspaceId, projectId,
parentArtifactId, parentVersion, scenario}`. The server reads the parent
(type `simulation`, format `anansi-swarmtank-run`, validated), recomputes the
branch, and registers it as artifact type **`scenario`** — a new
`ARTIFACT_TYPES` entry (no existing type is named `scenario`; the only other
`ARTIFACT_TYPES` consumers validate membership, so the addition is safe).
Title: `"SwarmTank scenario: <baseline description excerpt, ≤160 chars>"`.
Generator metadata: `{kind: "deterministic", generator:
"anansi-swarmtank-scenario"}`. Response: `{artifact, body, report, transitions,
metrics}` where `report` is `derive_report` over a temporary run view
(personas + scenario decisions) — the §40 recommendation machinery is reused,
not duplicated. Scenario objections get categories in `category_for`:
`"Price exceeds budget under high price sensitivity"` → price,
`"Missing critical required feature: …"` (prefix) → fit,
`"High setup overhead exceeds team capacity"` → switching. Baseline reports
are unaffected: the baseline engine never emits those strings.

### 4.3 Restore and the remap trap

The Slice 1 restore gate (`validate_original_swarmtank_bindings`) currently
queries `artifact_type = 'simulation'` and gates only format
`anansi-swarmtank-run` rows. Slice 3 extends it to also gate format
`anansi-swarmtank-scenario` rows (query both types): content-hash check +
full self-contained scenario validation, fail-closed for the whole restore.
The parent pin is **historical provenance only, shape-checked, never remapped
and never dereferenced** — the Slice 1 source-pin precedent. This is what
makes branches restore-stable: after a remap the pin points at a pre-remap
id, but the branch still validates (self-contained) and still renders (its
own decisions), and Forge transfer reads the branch by its current artifact
id, not via the pin. A branch whose hash or recomputation fails refuses the
restore, same as a tampered run.

### 4.4 What the panel lists

Runs (`type = simulation`) and branches (`type = scenario`) are listed
separately. Branches never appear in the runs list, so `viewReport` (which
parses run bodies) can never be handed a branch body. Viewing a branch loads
its comparison studio and drives the arena with the branch's scenario
decisions.

## 5. UI: controls, metrics, comparison, Forge bridge, export

New files (all added to the zero-`innerHTML` grep list in
`swarmtank.test.tsx`):

- `swarmtankSensitivity.ts` — the TS mirror: `evaluateScenario`,
  `computeTransitions`, `deriveScenarioMetrics`, `defaultScenarioFor`,
  `scenarioSignature` (a stable string key for the arena effect), plus the
  12-role archetype table and the integer-cents formatter. Pure, no DOM, no
  network. Instant by construction: 100 personas × closed-form arithmetic is
  far under 5 ms; the tests assert evaluation of 100 personas completes
  (timing assertions are flaky by nature, so the suite asserts the bound
  loosely — completes within 100 ms on this VM — and the design claim rests
  on the arithmetic being O(n) with no I/O).
- `SwarmtankScenarioControls.tsx` — price slider + number input + presets
  (Free always; multipliers only with a baseline price), feature checkboxes
  (one per baseline feature), trust toggles (SOC 2, 14-day free trial,
  30-day money-back; SLA guarantee as the fourth toggle), setup-difficulty
  segmented buttons, "Reset to Baseline". Badges, verbatim: `⚡ Instant
  Synthetic Sensitivity Calculation` on the scenario panel, and `🤖 Re-run
  Full AI Simulation` on the full re-run button — with adjacent text stating
  it runs the full deterministic evaluation again with the scenario inputs
  (same seed/preset/source, new run artifact) and makes no model call. The
  badge wording is the assignment's; the behavior is honest about what
  exists: there is no model-generated SwarmTank path in this codebase, and
  none is claimed.
- Reactive metrics bar (inside the report view, above the comparison):
  conversion rate, average willingness to pay, average perceived value, each
  with a signed delta vs baseline, plus net shift counts (`+N converted, N
  regressed to Confused, N dropped to Reject` — the assignment's example
  wording, computed from the transition matrix).
- `SwarmtankComparison.tsx` — baseline vs scenario metric cards (conversion
  %, price elasticity, average perceived value), the transition breakdown
  (personas converted vs lost, by name), top newly resolved vs newly
  introduced objections, and export buttons.
- `swarmtankScenarioExport.ts` — deterministic JSON and Markdown export of
  one comparison: fixed key order, fixed rounding, personas sorted by id,
  disclaimer header. Downloaded via Blob + anchor (the Jobs/export precedent).
  No backend call.
- Panel wiring (`Swarmtank.tsx`): scenario state lives in the panel
  (params, live TS decisions, saved branch artifact/doc). The report view
  gains the controls + metrics bar + comparison; the arena gains an
  `effectiveDecisions` prop. The Forge row gains a choice, shown when a
  branch is saved for the current run: "Apply Baseline Recommendations to
  Forge" (existing behavior, no scenario pin) vs "Apply Scenario
  Recommendations to Forge" (passes the branch pin). Transfer is the only
  action that revises a concept; previewing, selecting, or sliding saves
  nothing.

## 6. Dynamic arena re-clustering (no scene rebuild)

The Slice 2 builder indexes instances in initial-layout order (grouped by
decision); that order is pinned by existing tests (colors at indices 0/2/4,
pick mapping) and must not change. Slice 3 adds persona-stable slots:

- Each instance slot keeps its initial persona forever. `updateDecisions`
  (new builder method) takes a target layout and maps it by personaId onto
  slots; it never recreates meshes, geometries, or materials.
- Per slot, the builder stores transition start/target base angle and radius
  plus the target decision. Colors and marker visibility follow the target
  decision **immediately** at `updateDecisions` time (green/amber/red as
  before); positions and facing interpolate over 600 ms with easeInOutCubic
  driven by the view's rAF loop. Facing is always `facingFor(decision,
  currentAngle)` — radial inward/outward, tangential for Confused — so
  facing stays correct mid-flight.
- The Confused orbit is a clock-derived offset (`angle + t × 0.05`) on top
  of the interpolated base angle, exactly as Slice 2. Accepting one disclosed
  discontinuity: when a slot flips to or from Confused, its orbit offset
  appears or vanishes in one frame (the radius interpolation dominates
  visually; orbit-phase continuity across flips is not attempted).
- Ring lines: all radii the layout can ever produce for ≤ 100 agents are
  prebuilt (from base radii 60/120/180, spacing 26, capacity
  `floor(2πr/14)`: {60, 86, 112, 138, 120, 146, 172, 180, 206}) and toggled
  visible by the current layout's used radii. No line geometry is created
  or destroyed during a transition.
- Hesitation markers: one mesh allocated for all slots; markers are ordered
  Confused-first in slot order (preserving the Slice 2 test's marker-index
  expectations); only the first K matrices (K = current Confused count) are
  visible, the rest are scale-0; `counts.markers` still reports K.
- Reduced motion: `updateDecisions(layout, instant = true)` snaps progress
  to 1 and the view renders a single static frame, no loop — same contract
  as Slice 2's static frame.
- The view (`SwarmtankThreeView`) keeps its build effect keyed by runKey
  only. A separate effect watches the layout signature (runKey + scenario
  signature) and calls `updateDecisions` when it changes with the same
  runKey. A run switch still rebuilds and disposes everything, as before.
- 50-tick stability test: 50 alternating `updateDecisions` calls with
  animation frames, asserting exactly 3 InstancedMeshes throughout, stable
  counts, converged matrices at progress 1, and a clean `dispose()`.

## 7. Forge bridge expansion

`SwarmtankTransferRequest` gains optional `scenarioArtifactId` /
`scenarioVersion` (both-or-neither, `serde(default)` — existing callers
without them behave exactly as Slice 1). When present, the server reads the
branch, validates it, derives the scenario report, and writes:

- `swarmtank_addendum`: the run pin (as before) with **scenario**
  recommendations when a branch is applied, baseline recommendations
  otherwise.
- `swarmtank_scenario_diff` (new optional field on `TransferConceptDoc`,
  validated in `transfer.rs`): `{scenarioArtifactId, scenarioVersion,
  scenarioContentHash, baselineConversionRate, scenarioConversionRate,
  conversionDelta, baselineAverageWillingnessToPay,
  scenarioAverageWillingnessToPay, optimalPriceRecommendation,
  resolvedObjections (≤3), introducedObjections (≤3), transitions}`.

Idempotency: `alreadyApplied` when the concept already carries this run pin
AND this scenario pin (both absent = the Slice 1 case). Applying baseline
over a scenario application (or vice versa) is a new revision guarded by
`expected_version`, and a baseline application clears a stale diff.
Concept-body validation rejects a diff with a bad pin, non-finite numbers,
or top lists over 3 items.

## 8. Tests

- `packages/ui/tests/swarmtank-scenario.test.ts`: instant evaluation math
  across all 7 factors; extreme bounds ($0, 10× baseline, zero features, all
  features); scenario veto at ≥ 0.7 (and its non-interference with the
  baseline ≥ 0.5 veto); free-trial skepticism cut + freelancer/agency
  resolution; soc2 trust floor; complex-setup downgrade; feature-drop
  downgrade with the exact objection; transition-matrix counts; default
  scenario reproduces baseline decisions; 100-persona evaluation timing.
- `packages/ui/tests/swarmtank-arena-animation.test.ts`: target-position
  recomputation on decision flips (radius bands + facing); colors update
  immediately at `updateDecisions` before any animation frame; midpoint
  interpolation; convergence at progress 1; reduced-motion snap; 50-tick
  mesh-count and disposal stability.
- `tests/e2e/swarmtank-scenario.spec.ts` (3 tests, Slice 1/2 flow reused):
  1. Price slider updates conversion + arena repositioning without reload
     or provider traffic (click math from the TS layout module at the
     scenario layout's new coordinates; fake hits unchanged).
  2. Free-trial toggle resolves hesitation objections and raises Converted
     count in the comparison table.
  3. Save branch → apply scenario recommendations to Forge (concept v3
     with the scenario diff addendum) → backup → fresh restore → baseline
     run and scenario branch survive with intact reports (branch decisions
     read back equal the displayed scenario decisions).
- Regression: node `test:ui` (138 + new), all preserved e2e specs, Rust
  workspace (swarmtank/transfer/forge_restore suites in particular).

## 9. Acceptance criteria

- AC-1: Design committed before implementation code (this document).
- AC-2: Slider/toggle changes recompute all personas locally with signed
  metric deltas and transition counts, no network call, no page reload.
- AC-3: Scenario math agrees between the TS display engine and the Rust
  branch engine: the e2e reads back branch decisions equal to displayed
  scenario decisions, and both mirrors share the hand-computed vectors in
  their unit tests.
- AC-4: Arena agents re-cluster by interpolation without scene rebuild;
  reduced motion snaps in one static frame; 50 rapid ticks leak no meshes.
- AC-5: Branches persist as versioned `scenario` artifacts with parent
  provenance, pass fail-closed restore validation, and survive backup/restore.
- AC-6: Forge transfer offers baseline vs scenario recommendations; the
  scenario path writes `swarmtank_scenario_diff` and advances the concept
  to v3 in the e2e; idempotency holds per (run, scenario) pair.
- AC-7: JSON + Markdown comparison exports are deterministic (same input →
  same bytes) and carry the synthetic disclaimer.
- AC-8: The two modes are visually separated with the assigned badges, and
  the full re-run button's behavior (deterministic re-evaluation, no model
  call) is stated next to it.
- AC-9: Baseline runs, reports, and transfers behave exactly as Slices 1–2
  (no baseline-engine change; full regression battery green).
- AC-10: Milestone closure docs: `plans/07-swarmtank.md` (all slices
  complete with evidence), `docs/REQUIREMENTS_MATRIX.md` AN-014 advanced,
  `docs/BUILD_STATUS.md`, `docs/RECOVERY.md`, and the Slice 3 addendum.
  Windows verification of this slice remains the reviewer's step (checklist
  in the delivery MANIFEST) and is not claimed from Linux evidence.
