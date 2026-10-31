# M8 Three.js visualization — bounded design and acceptance criteria

Baseline: reviewer tip `9147e34949e9006cb6c07b74c9b8034e9f3a3528`
(review branch `codex/review-m8-visualization-repair`),
the Windows-verified visualization repair runtime.
Branch: `muse/m8-threejs-visualization`.
Design committed before any implementation code.
Ownership for this slice: `packages/ui/`, `apps/web/`,
and visualization test suites only. Rust crates,
SQLite schemas/migrations, API contracts, and
Windows release tooling are NOT touched.

## Authoritative sources

- `docs/MASTER_SPEC.md` §26 (LEXI-COLLIDER 3D
  visualization: visually premium Three.js
  experience; two toroidal particle streams;
  distinct beam colors; concept fragments orbiting
  the ring; controlled collision effect; recombined
  output nodes; technical relationships displayed
  after collision; elegant dark interface with
  violet, blue, and restrained white highlights;
  reflect real pipeline state; never fake
  computation progress; instanced rendering;
  GPU-efficient materials; adaptive quality;
  reduced-motion support; accessible 2D
  alternative; low-powered laptop must still access
  the full feature; 3D is a visualization, not the
  calculation engine) and §65 (adaptive rendering
  quality, instanced geometry, limited draw calls,
  bounded particle counts, reduced motion, keyboard
  interaction, accessible alternative views,
  responsive resizing, graceful WebGL failure,
  low-power mode; never block core features if 3D
  rendering is unavailable).
- Existing code: `packages/ui/src/
  colliderVisualizationScene.ts` (renderer-
  independent derivation: nodes, section-flow-only
  edges, deterministic layout, hit-test) and
  `packages/ui/src/ColliderVisualization.tsx`
  (Canvas 2D + always-rendered list alternative,
  identity-keyed state, Pause/Resume, reduced
  motion). The Three.js renderer consumes the same
  derived scene and the same layout — it does not
  re-derive, re-parse, or invent relationships.
- App settings flow: `Workspace.tsx` already writes
  the workspace motion preference to
  `document.documentElement.dataset.reducedMotion`
  while a workspace is open, so the 3D view honors
  both the system `prefers-reduced-motion` media
  query and the workspace setting without new
  plumbing.

## Dependency and workspace hygiene

- `three` and `@types/three` are added to
  `@anansi/ui` (`packages/ui/package.json`) via
  pnpm 11.25.0 (the repo's pinned `packageManager`),
  at exact pinned versions (`three@0.187.0`,
  `@types/three@0.187.0`, no ranges), so
  `pnpm install --frozen-lockfile` stays
  reproducible and `pnpm check` (TypeScript) plus
  `pnpm --filter @anansi/web build` (Vite) compile
  with 0 errors. The lockfile is updated by pnpm
  itself, not hand-edited. No other workspace
  dependency changes.
- Environment note (honest, not a claim): this Linux
  VM stands in for the owner's Windows paths. The
  repo lives at `~/workspace/Anansi-ultimate` here,
  not `E:\Anansi`; deliverables land in the
  workspace `deliveries/` and `your_files/`
  directories as in every prior slice, and the
  bundle/ZIP are portable to `E:\Anansi\deliveries`.
  Windows screenshots in the acceptance list are
  captured by the independent Windows verifier; this
  delivery ships Linux Chromium screenshots at 1x
  and 2x from the same specs plus the specs
  themselves.

## Visual architecture

New module `packages/ui/src/colliderThreeScene.ts`
(pure scene-graph construction over the derived
scene, no DOM, no renderer creation — so node tests
can build, raycast, and dispose it headlessly) and
new component `packages/ui/src/ColliderThreeView.tsx`
(renderer lifecycle, canvas mount, animation loop,
resize, raycast clicks, fallback signaling):

- Dark field `#0b2545`; Beam A violet `#a78bfa`,
  Beam B blue `#60a5fa`; restrained white `#f5f5f4`
  for labels and hubs.
- Two toroidal streams: one `TorusGeometry` ring
  per beam in its beam color, each carrying an
  `InstancedMesh` of particles distributed along
  the torus. The two particle groups rotate slowly
  in opposite senses — the streams, not a progress
  simulation.
- Orbital concept fragments: a second
  `InstancedMesh` of small fragments on inclined
  orbits around the rings, rotating as a group.
- Central collision effect: a glowing sphere plus a
  point light at the origin, gently pulsing only
  while animating (static when paused or reduced
  motion). It is a marker for the saved collision
  point, not a computation.
- Recombined output nodes: one `InstancedMesh` of
  spheres placed from `layoutVisualizationScene`
  (same deterministic 2D layout, mapped to the XZ
  plane with a small per-section height offset so
  hubs and items do not z-fight), colored by
  section, sized by role (hubs larger, items
  smaller). Edges are `LineSegments` between hub
  positions, exactly the derived section-flow edges.
- Labels: text sprites (canvas-generated
  `CanvasTexture` billboards, white text on a dark
  backing — the VIS-02 contrast rule carried over),
  one per node, bounded by the parsed result.
  Sprites are inert textures: saved text is drawn,
  never executed, and never reaches `innerHTML`.

## GPU efficiency and lifecycle

- Instancing everywhere particles/fragments/nodes
  repeat: three `InstancedMesh` objects plus two
  torus meshes, one collision sphere, one
  `LineSegments`, and bounded sprites. No
  per-particle or per-node meshes.
- Pixel ratio: `renderer.setPixelRatio(Math.min(
  window.devicePixelRatio || 1, 2))`; resize via
  `ResizeObserver` on the container, updating
  camera aspect and renderer size.
- Disposal is a first-class API on the scene
  builder: `dispose()` traverses the scene and
  disposes every geometry, material, and texture
  (including sprite textures), and the component
  cancels `requestAnimationFrame`, removes the
  `webglcontextlost` listener, and calls
  `renderer.dispose()` on unmount and on every
  identity (`artifactId@v{version}`) switch before
  building the next scene.

## Fallback and capability detection

- `detectWebGL(createCanvas?)` tries
  `webgl2` then `webgl` context creation through an
  injectable canvas factory (so node tests can
  drive null/throwing/valid factories). The
  component attempts `WebGLRenderer` construction
  in a try/catch; on failure, or on a
  `webglcontextlost` event, it sets fallback state
  and renders the existing Canvas 2D + list view
  (the current `ColliderVisualization` canvas path
  is preserved verbatim as the fallback, not
  rewritten). The accessible list view is always
  rendered regardless of renderer, and remains the
  keyboard path.
- A WebGL failure never blocks the saved view: the
  list, the detail panel, and provenance all work
  without it.

## Reduced motion and animation controls

- Reduced motion is true when the system media query
  matches OR `document.documentElement.dataset.
  reducedMotion === 'true'` (workspace setting).
  Under reduced motion the component renders
  exactly one static 3D frame and starts no
  animation loop. Otherwise a slow orbital loop
  runs with an explicit Pause/Resume control that
  also freezes the loop (same control vocabulary as
  the Canvas view). Animation moves rotations and
  the collision pulse only — never nodes, edges,
  selection, or provenance.
- The Canvas 2D component's reduced-motion check is
  extended by the same dataset read (one line, same
  behavior family) so both renderers agree.

## Data truthfulness and security

- The 3D scene is built only from
  `deriveVisualizationScene` output for the pinned
  version. No fake progress, no simulated streaming,
  no provider traffic from opening, selecting,
  pausing, or switching (the component makes zero
  API calls, like the Canvas view).
- Text appears only via React text nodes (list and
  detail) or canvas-generated sprite textures. No
  `innerHTML` or `dangerouslySetInnerHTML` anywhere
  in the new files (node test asserts this, as
  before).

## Tests

- Node (`packages/ui/tests/
  collider-three.test.ts`, added to the `test:ui`
  list): capability detection across null/throwing/
  valid canvas factories; scene-graph counts (2
  rings, particle/fragment/node instance counts,
  edge segments) from a fixture derived scene;
  deterministic layout mapping (same scene, same
  positions); raycast picking maps a node's world
  position back to its id; disposal calls dispose
  on geometries/materials/textures (prototype
  counters); fallback decision (detection false →
  fallback, detection true → 3D); no-markup
  assertion over the new files.
- Browser e2e (`tests/e2e/
  collider-three-visualization.spec.ts`): WebGL
  canvas renders for a saved experiment; raycast
  selection by clicking the projected Beam A hub
  (fixed camera, documented projection math in the
  spec) shows the exact detail; list selection still
  works; reduced-motion renders two identical frames
  (screenshot buffers equal) and shows the static
  note; version switching clears selection and
  rebuilds the scene; screenshots at 1x and 2x
  (opt-in `VIZ_SHOT_DIR`, plus the existing
  screenshots spec pattern).
- Fallback e2e is attempted with GPU-disabled
  launch args; if Chromium still provides a WebGL
  context there, that leg is dropped and fallback
  stays unit-covered (recorded honestly, not faked).

## Acceptance criteria (all must pass before delivery)

1. `three@0.187.0` and `@types/three@0.187.0`
   pinned in `@anansi/ui`; `pnpm check` and
   `pnpm --filter @anansi/web build` exit 0; no
   case-sensitive resolution issues (folded-name
   scan empty; the new module names are distinct).
2. The 3D view shows two toroidal streams in beam
   colors, orbiting fragments, a central collision
   marker, output nodes, and section-flow edges
   matching the derived scene (unit + screenshot
   evidence).
3. Instanced rendering only for repeated elements;
   DPR clamped to 2; resize handled; disposal
   verified by test; rAF cancelled on unmount and
   identity switch.
4. WebGL failure or context loss falls back to
   Canvas 2D + list with no loss of selection or
   provenance; list always available and keyboard
   operable.
5. Reduced motion (system or workspace) renders
   exactly one static frame; Pause/Resume controls
   the loop otherwise.
6. No fake progress, no new API calls, no
   `innerHTML`; node and browser suites green;
   strict Clippy/fmt/tsc/contracts/docs/builds
   clean (no Rust files changed, so Rust totals
   stand from the prior checkpoint).
7. Focused repair-style package: checksummed ZIP
   with Git bundle, exact baseline/tip, design/
   acceptance document, build/test logs, and
   screenshots; Windows rerun checklist; Three.js
   scope distinguished from any remaining M8 work
   (native acceptance, preview promotion, M5 gates,
   §32 sandbox, Slice 4 Windows verification).
