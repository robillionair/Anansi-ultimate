# M8 visualization — bounded design and acceptance criteria

Baseline: reviewer tip `795aab005f4bd896d17fedaeeda3140f4ea03a69`
(review branch `codex/review-m8-url-input-repair2`),
the Windows-verified URL repair 2 runtime.
Branch: `muse/m8-visualization`.
Design committed before any implementation code.

## Authoritative sources

- `docs/MASTER_SPEC.md` §26 (LEXI-COLLIDER 3D
  visualization: two beam streams, distinct beam
  colors, concept fragments orbiting the ring,
  controlled collision effect, recombined output
  nodes, technical relationships displayed after
  collision; reflect real pipeline state; never fake
  computation progress; instanced rendering,
  GPU-efficient materials, adaptive quality,
  reduced-motion support, accessible 2D alternative;
  3D is a visualization, not the calculation engine)
  and §65 (3D performance and accessibility:
  adaptive quality, instanced geometry, limited draw
  calls, bounded particle counts, reduced motion,
  keyboard interaction, accessible alternative
  views, responsive resizing, graceful WebGL
  failure, low-power mode; never block core
  features if 3D rendering is unavailable).
- `docs/REQUIREMENTS_MATRIX.md` AN-013 (LEXI-
  COLLIDER, in progress; 3D visualization deferred
  until this slice) and AN-014 (SwarmTank 3D, M9
  planned — explicitly NOT this slice).
- `plans/06-lexi-collider.md` (required scope:
  six-stage pipeline, versioned outputs, approved
  Forge transfer, real-state 3D and accessible
  alternative).
- Existing code: `packages/ui/src/Collider.tsx`
  (`ColliderExperimentView`, the saved-experiment
  view rendered from the Projects artifact view
  with key and instance scope pinned to
  `artifactId` + `version`), `ColliderOverview.tsx`
  (WP3 accessible pipeline overview: the honest
  section vocabulary and section relations this
  slice reuses), strict client parsers
  (`parseColliderExperimentBody`,
  `parseColliderResult`).

## Scope: one workflow, saved experiments only

A visualization section inside `ColliderExperimentView`,
mounted only for an exact saved experiment version
(`artifactId` + `version` passed from the Projects
artifact view, which already keys the view by both).
The live Collider panel is untouched: a generating
run has no saved version to pin, and the assignment
is the saved-version workflow.

## Derivation: from the parsed document, nothing else

New pure module `colliderVisualization.ts`:

- Input: the already-parsed `ColliderExperimentDoc`
  (the strict parser refuses malformed bodies before
  anything renders) plus the pinned identity
  (`artifactId`, `version`, provider, model from
  the doc).
- Nodes: two beam section nodes (Beam A, Beam B,
  each carrying its beam text and its document/URL
  provenance record where present), one node per
  section for source understanding (its exact text),
  one node per primitive, per mapping, per
  combination, per feasibility list item plus
  testability, and one node per productization
  field (brief, architecture, roadmap, test plan,
  prototype proposal). Every node carries its exact
  source text — no summarization, no generated
  labels beyond a short display label cut from that
  same text (combination nodes use their exact
  name).
- Edges: ONLY section flow plus section membership.
  Section-flow edges reuse the WP3 pipeline
  relations verbatim: beams → understanding,
  understanding → primitives, primitives → mappings,
  mappings → combinations, combinations →
  feasibility, feasibility → productization. Item
  nodes belong to their section; no edge is mined
  from free text, so no relationship is invented.
  A mapping node does NOT link to particular
  primitives, and a feasibility node does NOT link
  to particular combinations, because the saved
  record does not say which.
- Layout: a deterministic pure function of the
  scene (section hubs on a ring, item nodes
  orbiting their hub at fixed angles, two beam
  arcs feeding a central collision point). No
  randomness, no timestamps: the same version
  always lays out identically, which is what makes
  the restart/restore comparisons meaningful.
  Particle count is bounded by the derived node
  count (the result parser already bounds sections:
  3–12 primitives, 2–12 mappings, 2–8 combinations,
  bounded feasibility lists, 5 productization
  fields).

## Material decision: canvas renderer now, Three.js swap open

The repository has no Three.js dependency and no
working package-manager path that preserves the
pinned `node_modules` invariant (pnpm is absent;
adding a dependency would change the install
surface for web and desktop together). The slice
therefore renders with Canvas 2D on the existing
stack — available on low-powered hardware with no
GPU requirement, which §65 itself demands — behind
the renderer-agnostic derived scene model:
derivation, layout, selection, provenance, and the
text alternative do not depend on the renderer, so
a Three.js/WebGL renderer can replace the canvas
renderer later without changing them. This is
recorded honestly: the complete visualization
workflow is delivered, but the §26 Three.js
renderer itself remains outstanding and is NOT
claimed here. The canvas renderer still honors the
§26 visual concept (two beam streams in distinct
colors, concept fragments orbiting the ring, a
collision point, recombined output nodes, section
relationships) and the §65 list (bounded draw work
— one canvas, one animation loop, device pixel
ratio capped at 2, resize handling, reduced motion,
keyboard-accessible alternative, canvas-failure
fallback to the list alone).

## Interaction, accessibility, staleness

- Selecting a visual element (canvas click via
  hit-testing against the deterministic layout, or
  the equivalent button in the text/list
  alternative) reveals a detail panel with the
  node's exact source text and its provenance:
  experiment artifact id, version, provider, model,
  plus the beam's document/URL record where the
  node is a beam. The list alternative is always
  rendered, not only on canvas failure, and is the
  keyboard path (canvas nodes are not keyboard
  targets; the list buttons are).
- Reduced motion: `prefers-reduced-motion` renders
  one static frame and no animation loop. Otherwise
  a slow orbital animation runs with a Pause/Resume
  control; animation moves orbital angles only and
  never changes nodes, edges, or selection.
- Stale state: the component's selection and
  animation-frame state are keyed by
  `artifactId@version`; switching experiments or
  versions clears the selection and cancels the
  animation frame before the new scene renders
  (the Projects view already remounts on its key;
  the component additionally resets on identity
  change as defense in depth). No asynchronous
  fetch exists in the visualization: derivation is
  synchronous from the already-loaded document.
- No inference: the visualization makes zero API
  calls. Opening it, selecting nodes, pausing, or
  switching versions adds no provider traffic; the
  browser workflow test asserts the fake-server hit
  count is unchanged by visualization interaction.
- Render as data: all text reaches the screen via
  React text nodes or canvas `fillText`, never via
  `innerHTML` or `dangerouslySetInnerHTML`. A node
  test asserts the new files contain neither.
  Saved experiment text is untrusted model output;
  it is displayed, never executed, and no script,
  markup, or instruction inside it can run.

## Persistence and recovery: no parallel system

Nothing new is persisted. The visualization is
derived at view time from the artifact body, so
restart and backup/fresh-restore correctness reduce
to the existing artifact restore path: reopening
the same artifact version after restore derives the
identical scene. The browser workflow test covers
save → restart → backup/fresh-restore → reopen,
asserting identical node lists and working
selection, plus a second-version switch clearing
selection.

## Acceptance criteria (all must pass before delivery)

1. Saved experiment view shows a visualization
   section for the exact pinned version, with the
   version identity visible.
2. Node and edge counts match the parsed document
   exactly (unit tests over a fixture doc, including
   a doc with no document/URL provenance).
3. No edge claims a relationship the record does
   not state (unit test: mappings have no edges to
   individual primitives; feasibility items have no
   edges to individual combinations).
4. Selecting any node (canvas and list paths)
   shows its exact source text and full provenance.
5. Text/list alternative lists every node and is
   keyboard-operable; reduced-motion renders
   static; Pause/Resume controls animation.
6. Switching experiments or versions clears the
   selection and any animation state (unit + browser
   tests).
7. Restart and backup/fresh-restore reopen the same
   version with identical nodes and working
   selection (browser test).
8. Visualization interaction causes zero new
   provider/server hits (browser test).
9. Malformed or forged experiment bodies still
   refuse before rendering (existing parser tests
   keep passing; visualization renders nothing for
   an unparsable body).
10. All closed boundaries preserved: document, URL,
    approval, and provenance behavior unchanged
    (existing suites + browser battery); Antigravity
    directories (`scripts/windows-release/`,
    `tests/windows-release/`,
    `docs/reviews/windows-release/`) untouched.
11. Strict Clippy/fmt/tsc/contracts/docs/builds
    clean; node and browser suites green; broad
    core and providers+API totals recorded.

## Open items (not fabricated)

- Three.js/WebGL renderer swap (see decision
  above) — the scene model is ready for it; the
  renderer is not claimed.
- M8 is NOT complete after this slice: native
  desktop acceptance, preview promotion, M5
  production gates, §32 sandbox, and Slice 4
  Windows verification remain open.
