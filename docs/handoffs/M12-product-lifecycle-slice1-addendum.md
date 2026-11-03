# M12 Product Lifecycle Slice 1 — Addendum (not a §88 milestone report)

Slice 1 is foundational only; the §88 milestone
report comes with the final M12 slice. This file
records what Slice 1 built, what it deliberately
did not, and what the reviewer should check.

## Baseline

Branch `muse/m12-product-lifecycle-slice1` from
the Windows review head for M11 Slice 3. Third
consecutive MANIFEST-tail discrepancy: the
review MANIFEST names tip `9222e969a5316…`,
but the bundle's actual head is
`9222e9698a7f…` (shared prefix, different
tail), parent `03eb38001`, docs-only diff.
Bundle head authoritative, as before. The
review PASSES M11 on Windows (M11 CLOSED,
AN-016 verified complete).

## Features built

- Seven-stage pipeline engine, pure and
  model-free, mirrored in Rust
  (`crates/core/src/pipeline.rs`, 7 unit
  tests) and TypeScript
  (`packages/ui/src/lifecycle.ts`): stage
  derivation from project artifact types only,
  with gates, evidence counts, and
  missing-requirement sentences. Inspecting
  stages performs zero server mutations.
- `PipelineNavigator.tsx`: semantic `<nav
  aria-label="Product Creation Pipeline">`
  with an `<ol>` stepper, `aria-current="step"`
  on the Current stage, evidence counts, and
  requirement sentences, mounted in the
  project view above the task section and fed
  by the project's already-loaded artifact
  list (so it refreshes on every panel save).
- Stage → tool navigation: anchored sections
  for discover/invent/build/simulate/
  pressure_test, Ask-view navigation for
  refine (plumbed `Workspace → Projects →
  PipelineNavigator`), and an honestly
  disabled Launch control ("Launch Studio
  arrives in Slice 2").
- Override / Jump to Tool: Awaiting rows
  still render their tool link, labeled "Jump
  to …", de-emphasized, beside the
  requirement sentence; the Current stage
  stays visually prominent via
  `aria-current="step"`.

## Design decisions (and where they are recorded)

- §60's eight stages map to the assignment's
  seven with Evaluate folded into build
  readiness (M6 `evaluation` artifacts count
  in build's display, do not complete it).
- `refine` never completes (GHOST-PROMPT
  sessions are in-memory only), is never
  Current, and never blocks advancement; the
  Current rule is "first advanceable stage
  after the furthest evidenced stage whose
  gate is met". The design doc's first
  "furthest + 1" formulation was corrected
  before code (commit `2bc04bccd`) once
  formalizing showed it made `launch`
  unreachable as Current.
- Launch gate reads "simulation or evaluation
  artifacts" as simulation, scenario,
  somnia-evaluation, or M6 evaluation.
- Rust engine placement: the assignment names
  `crates/core/src/lifecycle.rs`, but that
  file already owns the memory-state machine
  (`MemoryState`); the engine lives in
  `pipeline.rs` instead. The TS mirror keeps
  the assigned `lifecycle.ts` name.
- Forge navigation targets the Artifacts
  section: Forge spec creation and revision
  live in the artifact views, reached from
  that list.

## Files added

- `crates/core/src/pipeline.rs`
- `packages/ui/src/lifecycle.ts`
- `packages/ui/src/PipelineNavigator.tsx`
- `packages/ui/tests/pipeline-navigator.test.tsx`
- `tests/e2e/product-lifecycle-slice1.spec.ts`
- `docs/handoffs/M12-product-lifecycle-slice1-design.md`
- `docs/handoffs/M12-product-lifecycle-slice1-addendum.md` (this file)

## Files modified

- `crates/core/src/lib.rs` (module declaration)
- `packages/ui/src/Projects.tsx` (mount + artifact feed)
- `packages/ui/src/Workspace.tsx` (plumb `onNavigateToAsk`)
- `package.json` (`test:ui` list)
- `docs/BUILD_STATUS.md`, `docs/RECOVERY.md`,
  `docs/REQUIREMENTS_MATRIX.md` (AN-012 row
  gains a Slice 1 sentence; status stays `in
  progress` — a navigator is not Launch
  Studio), `plans/PHASE-012-product-lifecycle.md`

## Database migrations

None. No schema change, no new artifact type,
no new server op.

## Tests passed (Linux, sequential)

- Rust workspace: **616 passed + 4 known
  live-Ollama `semantic_workflow` environment
  failures** (609 + 7 new pipeline tests),
  strict Clippy clean, fmt clean.
- Node `test:ui`: 198/198 (192 + 6 new
  pipeline-navigator tests).
- tsc 0, contracts `--check` 0 drift, docs
  validation PASS, web build clean.
- E2E `product-lifecycle-slice1.spec.ts`:
  2/2 (fresh project; concept + simulation
  advance the pipeline and link to tools),
  zero provider traffic.
- Regression battery: ghost Slices 1–3,
  SOMNIA Slices 1–3, evaluations, integrated
  workflow, opportunities, SwarmTank
  scenario/arena/slice1, plus the new
  lifecycle spec — totals in the MANIFEST.

## Tests failed

- The 4 pre-existing live-Ollama
  `semantic_workflow` environment failures,
  identical to baseline. No other failures.

## Known limitations and honest statements

- Launch can complete in Slice 1 only via
  manual `launch-material` registration; the
  generator arrives in Slice 2.
- `build` navigation lands on the Artifacts
  list rather than a dedicated Forge screen
  (none exists in the project view; Forge
  work happens in artifact views).
- The e2e spec's first two runs failed on the
  Simulate row (stale pre-simulation state)
  and then passed consistently; the spec's
  registration helper was hardened to wait
  for the end-of-handler "Artifact
  registered." notice rather than only the
  list row before asserting pipeline state.
  The underlying first-run stall was not
  root-caused; it did not reproduce across
  the subsequent runs, so it is disclosed
  here rather than claimed fixed.
- No §85 guided walk, no §61 copy
  generator, no execution sandbox, no
  provider traffic — all deferred per the
  assignment's non-goals.

## Security issues

None found. Credentials sweep over the
delivery diff clean. The navigator reads only
the project's already-loaded artifact types;
it sends nothing anywhere.

## Last verified commit

Branch `muse/m12-product-lifecycle-slice1`,
tip recorded in the delivery MANIFEST (this
addendum's closure commit). Baseline: Windows
review head `9222e9698a7f…` (see the
discrepancy note at the top).

## Next slice

Slice 2 (§61 Launch Studio marketing copy
generator & asset bundler) and Slice 3 (§85
guided walk) are not commissioned in this
assignment. AN-012 stays `in progress`.
