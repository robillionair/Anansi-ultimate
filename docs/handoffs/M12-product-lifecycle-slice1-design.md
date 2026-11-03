# M12 — Integrated Product Lifecycle & Launch Studio, Slice 1: Design

## Baseline

Branch `muse/m12-product-lifecycle-slice1` from the
Windows review head for M11 Slice 3. The review
MANIFEST names tip `9222e969a5316…`, but the
bundle's actual head is `9222e9698a7f…` (shared
prefix, different tail), parent `03eb38001`, with
a docs-only diff (BUILD_STATUS,
REQUIREMENTS_MATRIX, review report) — the third
consecutive review package with a wrong MANIFEST
tail. The bundle head is authoritative, as
before, and the discrepancy is recorded here and
in the delivery MANIFEST. The review confirms
the Slice 3 delivery hashes and PASSES M11 on
Windows (M11 CLOSED; AN-016 verified complete).

Scope: MASTER_SPEC §60 (unified pipeline), §62
(working example), §85 (Definition of Done —
product creation, as the Slice 3 guided walk's
target, not this slice's deliverable). Non-goals:
§61 Launch Studio copy generator & asset
bundler (Slice 2), the §85 end-to-end guided
walk (Slice 3). No execution sandbox, no
provider traffic, no new server op, no new
artifact type, no schema change.

## The §60-to-7-stage mapping (decision)

§60 lists eight stages (Discover, Invent,
Evaluate, Build, Simulate, Stress-Test,
Improve, Prepare Launch). The assignment fixes
seven stage ids for Slice 1. Mapping:

- Discover → `discover` (Opportunity Engine, AN-009)
- Invent → `invent` (LEXI-COLLIDER, AN-013)
- Evaluate → folded into `build` readiness: M6
  `evaluation` artifacts are counted in build's
  evidence display but do not complete `build`
  (only Forge outputs do). Rationale: the
  assignment's stage list has no evaluate stage,
  and concept scoring is input to Forge, not a
  pipeline gate of its own.
- Build → `build` (Product Forge, AN-012)
- Simulate → `simulate` (SwarmTank, AN-014)
- Stress-Test → `pressure_test` (SOMNIA, AN-015)
- Improve → `refine` (GHOST-PROMPT, AN-016)
- Prepare Launch → `launch` (Launch Studio, AN-012/M12)

## Stage engine (pure, model-free, idempotent)

New Rust module `crates/core/src/lifecycle.rs`
(reference) and TS mirror
`packages/ui/src/lifecycle.ts` (live). Both take
only artifact type strings — the UI derives them
from the already-loaded `listProjectArtifacts`
response — and return the same stage states.
Inspecting stages performs zero server
mutations. Unknown artifact types are ignored,
never errors: the engine is a view over the
registry, not a validator of it.

### Evidence (what completes a stage)

| Stage | Completing artifact types (verified in code) |
|---|---|
| `discover` | `idea` — M6 idea sets and M6 opportunity sets both register as `idea` (`ideas.rs`, `opportunities.rs`) |
| `invent` | `concept` — collider → Forge transfer |
| `build` | `specification`, `source-package`, `deliverable`, `source-review` — Forge generated path |
| `simulate` | `simulation` — SwarmTank run save; `scenario` — SwarmTank branch save |
| `pressure_test` | `somnia-evaluation` — SOMNIA save |
| `refine` | none — GHOST-PROMPT sessions are in-memory only and are never registered (Slice 1–3 design). `refine` therefore never auto-completes; it is a tool stage, not an artifact gate, and it never blocks advancement. |
| `launch` | `launch-material` — the type exists in `ARTIFACT_TYPES`; no generator exists until Slice 2, so in Slice 1 launch can complete only via manual registration. Stated, not hidden. |

### Gates (what unlocks a stage)

- `discover`, `invent`, `refine`: always open.
  §62 starts from a user idea, not an
  opportunity artifact, so `invent` must not
  wait on `discover`; GHOST-PROMPT is an
  editor-scoped prompt tool usable at any time.
- `build`: at least one `concept` artifact.
- `simulate`: at least one `concept` artifact
  (assignment explicit).
- `pressure_test`: at least one `concept`
  artifact (the §51 SOMNIA QA bridge runs per
  Forge concept).
- `launch`: at least one `simulation`,
  `scenario`, `somnia-evaluation`, or
  `evaluation` artifact (assignment: "completed
  simulation or evaluation artifacts" — read
  to include both the SOMNIA and M6 evaluation
  types; the choice is stated here).

### Statuses (deterministic)

Let `furthest` be the highest stage index with
completing evidence, or −1 for a fresh project.
`current` is the first stage after `furthest`
that is advanceable and whose gate is met, or
none when there is no such stage. `refine` is
never advanceable: it holds no evidence and
cannot complete, so it can never be Current —
it is always Available (its gate is always
met), a tool to jump to, never a blocker. This
is what lets `launch` become Current: after
pressure-test evidence exists, the next
advanceable stage is `launch` (its gate is met
by the evaluation), with `refine` offered
alongside it as Available.

- `Complete`: the stage has completing evidence
  (`refine` never).
- `Current / In Progress`: the stage is
  `current`.
- `Available`: the stage's gate is met, it is
  not Complete, and it is not Current.
- `Awaiting Preceding Stage`: the stage's gate
  is unmet and it is not Complete (evidence
  always beats a missing gate: a directly
  registered `simulation` without a `concept`
  still shows `simulate` Complete).

Worked examples (pinned by unit tests):

- Fresh project: `discover` Current;
  `invent`/`refine` Available; `build`/
  `simulate`/`pressure_test`/`launch`
  Awaiting.
- Concept only: `invent` Complete, `build`
  Current, `simulate`/`pressure_test`
  Available (gate met), `launch` Awaiting.
- Concept + simulation: `invent`/`simulate`
  Complete, `pressure_test` Current, `launch`
  Available (gate met via simulation).
- Concept + simulation + somnia-evaluation:
  `pressure_test` Complete, `launch` Current,
  `refine` Available (never Current, never a
  blocker — see the rule above).
- All types incl. `launch-material`: every
  completable stage Complete, no Current stage,
  navigator shows a pipeline-complete note.

### Readiness metrics and diagnostics

Each stage state carries per-type evidence
counts (e.g. "2 concepts · 1 simulation") and,
for Awaiting stages, one missing-requirement
sentence ("Needs at least one Forge concept
artifact.", "Needs a completed simulation or
evaluation artifact."). No counts are
estimated; they are registry counts.

## PipelineNavigator UI

New `packages/ui/src/PipelineNavigator.tsx`,
mounted in the project view (`Projects.tsx`,
when a project is open, above the task
section) and fed by the project's already-
loaded `artifacts` state, so it refreshes when
any panel saves (collider/swarmtank/somnia
`onSaved` handlers already reload artifacts).

- Semantic `<nav aria-label="Product Creation
  Pipeline">` containing an `<ol>`; the Current
  stage carries `aria-current="step"` and the
  visually prominent styling.
- Every row shows the stage name, its status
  text, its evidence counts, and (for Awaiting
  rows) its requirement sentence.
- Navigation targets (verified against the
  current project view, which is one scrolling
  view with anchored sections):
  - `discover` → `#project-opportunities-heading`
    (Opportunity discovery section; idea
    generation sits directly above it).
  - `invent` → `#project-collider-heading`.
  - `build` → `#project-artifacts-heading`
    (Forge work starts from concept artifacts:
    spec creation and revision live in the
    artifact views, reached from this list).
  - `simulate` → `#project-swarmtank-heading`.
  - `pressure_test` → `#project-somnia-heading`.
  - `refine` → the Ask ANANSI view, via a new
    optional `onNavigateToAsk` prop plumbed
    `Workspace → Projects → PipelineNavigator`
    (GHOST-PROMPT lives only in the Ask strip;
    there is no anchor for it in the project
    view, so anchor navigation would be a lie).
  - `launch` → no target in Slice 1. Its
    control is disabled with the note "Launch
    Studio arrives in Slice 2." This is a
    stated deviation from "every available
    stage navigates": inventing a Launch Studio
    destination would be a false claim.
- Override / Jump to Tool: Awaiting rows still
  render their tool control, labeled "Jump to
  tool", de-emphasized, beside the requirement
  sentence. The recommended (Current) stage
  stays visually prominent; the jump never
  hides the requirement. For `refine`, the jump
  is the same Ask-view button (always open, so
  it is never an override case in practice).

## Tests

- Rust unit tests in `lifecycle.rs`: fresh
  project, concept-only, concept+simulation,
  full pipeline incl. `launch-material`,
  refine-never-completes, unknown types
  ignored, evidence-beats-gate.
- UI tests `packages/ui/tests/pipeline-
  navigator.test.tsx` (static markup, as with
  the other component tests): stage derivation
  across artifact sets, `aria-current="step"`
  placement, anchor hrefs, refine button
  presence, launch disabled + Slice 2 note.
  Click navigation is covered by e2e, not by
  static markup (effects do not run there).
- E2E `tests/e2e/product-lifecycle-slice1.spec.ts`
  (via the shared `startSession` harness,
  Harbor App project):
  1. Fresh project: navigator shows Discover
     Current, Build/Simulate/Pressure test/
     Launch Awaiting.
  2. Register a `concept` and a `simulation`
     artifact through the existing Artifacts
     manual-registration form; assert Invent
     and Simulate become Complete, Pressure
     test becomes Current, Launch becomes
     Available; click the Simulate stage link
     and assert the URL hash lands on the
     SwarmTank section; click Invent and
     assert the collider section; click
     Refine and assert the Ask ANANSI view
     opens. Zero provider traffic (manual
     registration only; `fakeHits() === 0`).

## Files

Added: `crates/core/src/lifecycle.rs`,
`packages/ui/src/lifecycle.ts`,
`packages/ui/src/PipelineNavigator.tsx`,
`packages/ui/tests/pipeline-navigator.test.tsx`,
`tests/e2e/product-lifecycle-slice1.spec.ts`,
this design doc, and the closure addendum.
Modified: `crates/core/src/lib.rs` (module
declaration), `packages/ui/src/Projects.tsx`
(mount + artifact types feed),
`packages/ui/src/Workspace.tsx` (plumb
`onNavigateToAsk`), `package.json` (`test:ui`
list), `docs/BUILD_STATUS.md`,
`docs/RECOVERY.md`, `docs/REQUIREMENTS_MATRIX.md`
(AN-012 row gains a Slice 1 sentence; status
stays `in progress` — a navigator is not
Launch Studio), `plans/PHASE-012-product-lifecycle.md`.
