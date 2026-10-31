# M9 SwarmTank Slice 1 — design before code

Branch: `muse/m9-swarmtank-slice1`, from
reviewer tip `7c576b85eab5d76e8713ff2debf89f1e7a3756d4`
(Windows-verified §26 Three.js + §27
artifacts/Forge baseline; reviewer delta over
`dbc49bd16` is the synchronous `reducedMotion`
init fix, two review reports, nine screenshots,
and BUILD_STATUS entries — no Rust, contract,
schema, or release-tooling change; that fix is
preserved untouched by this slice).
Authority: MASTER_SPEC §34–37, §40–41 and
`plans/07-swarmtank.md`. This slice covers
synthetic personas, the seeded decision engine,
the report, and the Forge bridge ONLY. The §38
3D arena, price/feature scenario engine,
provider-backed generation, and landing-page
URL import are explicitly deferred (see
Non-goals); nothing in this slice claims them.

## Acceptance criteria (committed before code)

- AC-1 Input: manual fields (description,
  pricing, segment, industry, features, value
  proposition) and import from a saved §27
  experiment or a Forge concept fill a run
  input by field copy only, never by inference;
  opening the SwarmTank panel triggers zero
  provider calls (Slice 1 makes no provider
  calls at all).
- AC-2 Personas: presets 20 / 50 / 100 only;
  every persona carries `id`, `name`, `role`,
  `companySize`, `approximateBudget`, `needs`,
  `priorities`, `skepticism` (0.0–1.0),
  `priceSensitivity` (0.0–1.0),
  `purchaseConstraints`, `requiredTrustSignals`,
  `expectedImplementationEffort`; every persona
  surface is labeled simulated/synthetic;
  generation runs in validated batches of 10.
- AC-3 Decisions: each persona evaluates to
  exactly one of `Convert`, `Confused`, `Reject`
  with `reason`, `keyObjection` (required for
  Confused/Reject, null for Convert),
  `perceivedValue` in [0,1], and
  `willingnessToPay` in [0, budget]; the same
  seed + input reproduces byte-identical
  populations and decisions.
- AC-4 Report: conversion rate, average
  willingness to pay, top 3 confusion points,
  top 3 objections, categorized objection
  breakdown, persona-segment counts, and
  recommended adjustments; every recommendation
  carries assumption, synthetic evidence,
  uncertainty, and real-customer validation;
  the persona table supports search and
  decision filtering; the report exports as
  downloadable JSON.
- AC-5 Forge bridge: "Apply Recommendations
  to Forge" writes the top feedback/objections
  as a revision note onto an associated Forge
  concept (new concept version, expected-version
  guarded); re-applying the same run returns
  the existing version without duplicating it;
  with no associated concept the action fails
  closed with a clear message (it never invents
  a concept).
- AC-6 Persistence: evaluated runs survive
  restart and backup/fresh restore; a tampered
  run document is refused fail-closed at
  restore, report, and transfer time.
- AC-7 Adversarial: out-of-range numerics,
  oversized fields, non-preset populations,
  unknown seeds, and unassociated transfers are
  rejected with plain errors; no input text is
  ever rendered as HTML.
- AC-8 Tests: Rust (serialization, engine
  boundary conditions, restore round-trip),
  node (persona parsing, decision aggregation,
  report rendering), and one Playwright
  vertical slice (Input → Generate → Evaluate
  → Report → Backup/Restore → Export).
  Preservation baseline: UI 125/125 and the
  Windows-verified core suites (collider 49/49,
  transfer 19/19, forge_restore 4/4); Linux
  counts for those suites are recorded in the
  addendum from the verification run.

## Decisions

### 1. Deterministic generation, no provider inference

Slice 1 generates personas from a fixed
archetype table (the §36 examples plus
seeded variation), not from a model call.
Rationale: the standing rule bars paid
inference without owner authorization, and
deterministic generation is exactly
repeatable, which is what AC-3 needs.
Batching is structural: the generator
processes 10 personas per batch and validates
each batch before the next, so a future
provider-backed generator — separately
authorized, not claimed here — can return
batches incrementally over the same shape.
No "100 separately reasoned agents" claim is
made anywhere: the UI states the population
is generated from seeded archetypes.

### 2. One artifact per evaluated run

A run is an artifact of the existing,
currently unused type `simulation`, with
`format: 'anansi-swarmtank-run'` v1. The
personas step is a preview: it is
deterministic and reproducible from the
recorded seed, and it is NOT persisted. The
evaluate step atomically registers the
complete run (input + seed + personas +
decisions + content hash) as version 1.
Runs are never edited; a new evaluation is a
new run. This keeps the immutability story
simple: one registration path, no run-revise
path. Runs ride the existing backup/restore
machinery through the artifacts table.

### 3. Seeded randomness without a new dependency

`sha2` 0.10.9 is already a core dependency.
The generator and any evaluation jitter use
SHA-256 counter mode over the seed string
(`SHA256(seed || counter)`), so results are
byte-identical across platforms and runs.
The seed is always recorded on the run: a
user-supplied seed reproduces exactly; when
the user supplies none, the UI generates one
at creation time and records it, so the run
stays reproducible from its own record.

### 4. Decision engine: pure, transparent, bounded

`evaluate(persona, input)` is a pure
function in `crates/core/src/swarmtank.rs`.
Seven factors from §37, each scored in
[0,1] from record fields only, with exact
formulas (no external data, no model):
fit = 0.4 base + 0.3 if the input segment
contains (case-insensitive substring) any of
the archetype's fixed segment keywords + 0.3
× the share of the persona's needs keywords
present (case-insensitive substring) in the
joined features + value proposition text;
budget = 1.0 if price ≤ budget, else
budget / price (0 when budget is 0 and
price > 0); price = 1 − priceSensitivity ×
(price / max(price, budget)) clamped to
[0,1] (0.5 when no price is stated);
trust = 0.75 − 0.5 × skepticism (a neutral
prior dampened by skepticism — the input
carries no trust-signal field, so no trust
signal is ever invented); switching = 1 −
(expectedImplementationEffort class 0–1) ×
0.5, lower for larger companies;
clarity = fraction of {description, pricing,
value proposition, non-empty features}
present, mapped to 0.9/0.7/0.5/0.3 for
4/3/2/1 present fields; authority = the
archetype's fixed seniority class
(founder 0.9, director/manager 0.7,
individual contributor 0.4).
Weighted score: fit 0.25, budget 0.20, price
0.15, trust 0.10, switching 0.10, clarity
0.10, authority 0.10. Thresholds: ≥ 0.65
Convert, 0.40–0.65 Confused, < 0.40 Reject.
Veto: price strictly above budget with
priceSensitivity ≥ 0.5 caps the outcome at
Reject with keyObjection "Price exceeds
budget". `reason` names the decisive
factor(s); `keyObjection` is the lowest
blocking factor for Confused/Reject and null
for Convert. `perceivedValue` is the
fit-weighted score in [0,1].
`willingnessToPay` = round(budget ×
perceivedValue × (1 − 0.5 ×
priceSensitivity)), clamped to [0, budget];
zero budget yields zero willingness to pay.
Confidence scores are never presented as
empirical probabilities; the report labels
them model indicators.

### 5. Report is derived, not stored

The report is a pure derivation from the
stored run document (same pattern as §27
artifacts): conversion rate, average
willingness to pay, decision distribution,
top 3 confusion points (most frequent
keyObjection among Confused), top 3
objections (most frequent keyObjection
among Reject), categorized breakdown
(price/trust/clarity/fit/authority/switching),
persona-segment counts by role, and
recommended adjustments. Each recommendation
is rule-derived from the observed top
objections (e.g. price-leading objections
yield a pricing/packaging recommendation) and
carries the §40 fields: underlying
assumption, synthetic evidence (counts),
uncertainty ("synthetic only, unvalidated"),
and how to validate with real customers.
The report never claims measured real-world
demand; the §34 disclaimer renders above it.

### 6. Transfer-to-forge revises the concept

`swarmtank-transfer-to-forge` requires an
associated Forge concept: set at import from
a concept, or chosen in the run view from the
project's concept artifacts (listed via the
existing artifacts list, type `concept`).
It registers a NEW VERSION of that concept
through the existing artifacts revise path
with an expected-version guard, adding ONE
optional field to `TransferConceptDoc`:
`swarmtank_addendum` (run pin id/version/
hash + top objections + confusion points +
recommended adjustments). If the concept's
latest version already carries this run's
addendum, the op returns it unchanged
(`alreadyApplied`) — the §27 idempotency
pattern. Concepts transferred before this
slice parse with the field absent. The op
re-validates the run document (hash +
shape) before revising, so a tampered run
cannot reach Forge.

### 7. Import is a field copy with a source pin

Import from a §27 experiment copies
conceptBrief → description and primitives →
candidate features (capped by the feature
bound); import from a Forge concept copies
approvedBrief → description. Fields the
source does not state stay empty — never
filled by invention. The run records the
source pin (experiment id/version/hash or
concept id/version/hash). Import validates
the source document with the existing
validators before copying anything.

### 8. Validation division, mirroring §27

Rust owns shape, bounds, and hashes:
strict field bounds (description ≤ 8000,
pricing/segment/industry ≤ 500, value
proposition ≤ 3000, features ≤ 50 × ≤ 300,
needs/priorities/constraints/signals ≤ 12 ×
≤ 300, budgets ≥ 0, skepticism and
priceSensitivity in [0,1], populations only
20/50/100, batches of 10); `contentHash`
recomputed fail-closed at restore (same
restore sites as the experiment validator),
report, and transfer. The UI parser checks
shape and bounds only (no sync SHA-256 in
the browser); hash verification stays
backend + export. New files contain zero
`innerHTML` (grep-asserted in node tests,
as in §27).

### 9. UI placement

SwarmTank lives as a new section in the
Projects view (`project-swarmtank-heading`),
project-scoped like the Collider and Forge
sections: run list, input form (manual +
import), population cards (each labeled
"Simulated persona"), decision distribution,
report with search + decision filter, JSON
export (Blob + anchor, the Jobs precedent),
and the transfer action with a concept
picker when no concept is associated.
`SwarmTank` leaves `plannedGroups` (it is no
longer planned); SOMNIA and Experiments stay
Planned. The panel makes no provider calls
on load — or ever, in this slice.

## Files

- NEW `crates/core/src/swarmtank.rs`:
  input, archetypes, seeded generator,
  decision engine, run doc, content hash,
  restore validator, transfer addendum
  construction; `lib.rs` module registration.
- NEW `crates/core/tests/swarmtank.rs` (+
  restore round-trip coverage in the existing
  restore suite pattern).
- `crates/api/src/lib.rs`: four paths
  `/v1/ai/swarmtank-personas`,
  `/v1/ai/swarmtank-evaluate`,
  `/v1/ai/swarmtank-report`,
  `/v1/ai/swarmtank-transfer-to-forge`
  wired to the core ops like
  `collider-transfer`.
- `crates/contracts/src/main.rs` + regen:
  `SwarmtankInput`, `SwarmtankPersona`,
  `SwarmtankDecision`, `SwarmtankRunDoc`,
  `SwarmtankReport`, transfer
  request/response; `TransferConceptDoc`
  gains optional `swarmtank_addendum`.
- NEW `packages/ui/src/Swarmtank.tsx`
  (input form, cards, distribution, report,
  transfer) + `packages/ui/src/Projects.tsx`
  section + `Navigation.tsx` planned-list
  update; `Transfer.tsx` parser optional-key
  handling + addendum rendering.
- NEW `packages/ui/tests/swarmtank.test.tsx`
  (+ fixture) and root `package.json`
  `test:ui` list entry.
- NEW `tests/e2e/swarmtank-slice1.spec.ts`
  (Input → Generate → Evaluate → Report →
  Backup/Restore → Export), reusing the
  collider-family session helpers where they
  fit without changing shared helper
  behavior for existing specs.

## Explicit non-goals (deferred, not claimed)

- §38 3D arena; price/feature scenario
  engine; suggested-experiment generation.
- Provider-backed persona generation (needs
  separate authorization).
- Landing-page URL import (spec §35 mentions
  it; the slice assignment's input list does
  not — deferred, not silently built).
- Spec auto-start or execution from Forge
  transfer (transfer approves a revision note
  only, as §27 transfer approves no spec).
- Native save dialog for report export
  (browser download is the supported path,
  as in §27).
