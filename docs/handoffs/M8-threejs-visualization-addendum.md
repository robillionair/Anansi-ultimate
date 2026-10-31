# M8 Three.js visualization — delivery addendum

Branch: `muse/m8-threejs-visualization`, from
reviewer tip `9147e34949e9006cb6c07b74c9b8034e9f3a3528`
(the Windows-verified visualization repair runtime).
Design and acceptance criteria committed before code
(`docs/handoffs/M8-threejs-visualization-design.md`).
No Rust crate, schema, migration, API contract, or
Windows release file is touched; the diff is
`packages/ui/`, `apps/web/`-adjacent workspace
metadata (`package.json`, `pnpm-lock.yaml`,
`pnpm-workspace.yaml`), and visualization tests.

## What was built

The §26 Three.js/WebGL renderer, consuming the same
renderer-independent scene model as the Canvas view
(`colliderVisualizationScene.ts`) — same derivation,
same deterministic layout, same section-flow-only
edges, same provenance. New files:

- `packages/ui/src/colliderThreeScene.ts` — pure
  scene-graph builder (no renderer, no DOM): two
  toroidal particle streams (Beam A violet `#a78bfa`,
  Beam B blue `#60a5fa`, 120 instanced particles
  each, opposite senses), 64 instanced octahedral
  concept fragments on an inclined orbit, a central
  collision marker (white core + soft glow), one
  instanced sphere per output node placed from the
  shared layout, `LineSegments` for exactly the
  derived edges, and white-on-dark text sprites for
  labels. Also `detectWebGL` (webgl2 then webgl
  through an injectable canvas factory) and
  `isReducedMotion` (system media query OR the
  workspace setting the app already writes to
  `document.documentElement.dataset.reducedMotion`).
  Raycast picking maps NDC clicks to node ids;
  `setSelected` recolors one instance gold;
  `animate(t)` rotates the streams and pulses the
  collision marker; `dispose()` releases every
  geometry, material, and texture.
- `packages/ui/src/ColliderThreeView.tsx` — renderer
  lifecycle: `WebGLRenderer` (`antialias`, low-power
  preference), pixel ratio clamped to 2, resize via
  `ResizeObserver`, one static frame under reduced
  motion or pause, otherwise a slow orbital loop,
  `webglcontextlost` → fallback signal, full
  disposal + rAF cancellation on unmount and on
  every identity switch.
- `packages/ui/src/ColliderVisualization.tsx` —
  tries the 3D view first; on WebGL failure renders
  the unchanged Canvas 2D view; the list alternative
  is always present. Reduced motion now honors the
  workspace setting as well as the system query, and
  the static note reads "Motion reduced — static
  view." (the two existing specs' assertions were
  updated to the new copy; no behavior assertion was
  weakened).

## Dependencies, pinned via pnpm

`three@0.187.0` (runtime) and `@types/three@0.187.0`
(dev) added to `@anansi/ui` with pnpm 11.25.0 (the
repo's pinned `packageManager`), exact versions, no
ranges. `pnpm install --frozen-lockfile
--ignore-scripts` passes, so a fresh checkout
reproduces this tree. Two environment notes, kept
honest:

- This Linux VM has no pnpm on PATH and a
  filesystem that refuses `chown` to another uid,
  which broke root-run pnpm with `EPERM fchown`.
  The install was done as a non-root user in group
  `nogroup` against the original store, with
  `--ignore-scripts` (the repo's established install
  mode; esbuild's postinstall is not needed because
  its platform binary package is already present).
  A stray pnpm-generated `allowBuilds: esbuild`
  placeholder ("set this to true or false") was
  removed from `pnpm-workspace.yaml`; the
  pnpm-generated `minimumReleaseAgeExclude` entries
  for the two new packages were kept. None of this
  changes the delivered tree: the lockfile is
  pnpm-generated and frozen-install verified.
- The assignment's `E:\Anansi` paths are the owner's
  Windows locations. On this VM the repo lives at
  `~/workspace/Anansi-ultimate` and deliverables in
  `~/workspace/deliveries/` + `~/workspace/your_files/`,
  as in every prior slice; the bundle/ZIP are
  portable to `E:\Anansi\deliveries`. Windows
  screenshots are for the independent verifier; this
  delivery ships Linux Chromium screenshots at 1x and
  2x from the same specs.

## Corrections found by testing (not by inspection)

1. Fallback identity clobber (real defect, fixed in
   this slice): the first fallback implementation
   reset `webglFailed` in the parent's identity
   effect. Child effects run before parent effects,
   so on mount — and on every identity switch — the
   parent reset overwrote the child's fallback signal
   in the same commit and the 3D canvas stayed
   mounted forever on a WebGL-less page. The fix
   tracks the failed identity instead
   (`failedIdentity === scene.identity`): a new
   identity simply does not match, so the 3D view is
   retried per version and no reset effect can
   clobber the signal. The browser fallback spec
   caught this; it passes after the fix.
2. GPU-disable launch flags (`--disable-gpu`,
   `--disable-software-rasterizer`,
   `--disable-gpu-compositing`, `--disable-3d-apis`)
   did NOT remove WebGL in this Chromium
   (SwiftShader still served contexts), so the
   no-WebGL browser leg stubs `getContext` for
   `webgl*` via `addInitScript` instead — the same
   in-page interception technique the reviewer probes
   use. A plain-JS string is required there: a
   TypeScript-annotated function stringifies with
   annotations that are syntax errors in the page.
3. The canonical visualization spec's canvas click
   targeted 2D layout coordinates, which miss the
   hub under the fixed 3D camera (fov 45, distance
   900). The click now uses the documented
   projection (`NDC y = ring / (900 * tan(fov/2))`),
   and the raycast pick passes.

## Linux evidence (sequential)

- Node `test:ui`: **115/115** (105 + 10 new Three.js
  tests: detection, reduced motion, scene counts,
  determinism, raycast round-trip, selection recolor,
  animation advance, disposal counters, headless
  sprite skip, no-markup).
- TypeScript exit 0, Vite web build exit 0,
  contracts `--check` exit 0, docs validation PASS
  (206 files before this addendum; 207 with it).
- Browser battery **8/8 across 7 specs** (1.4 min):
  canonical visualization, visualization screenshots
  (2x), Three.js visualization (1x + 2x legs),
  Three.js fallback, document input, URL input,
  pipeline overview. The Three.js spec asserts
  WebGL canvas presence, raycast Beam A selection
  with exact text, zero new provider traffic, list
  selection, version-switch clearing, and
  byte-identical reduced-motion frames; the fallback
  spec asserts Canvas 2D + list under stubbed
  no-WebGL.
- Screenshots shipped (7): `viz-three-animated.png`,
  `viz-three-reduced-motion.png`,
  `viz-three-hidpi-reduced-motion.png` (new) plus
  the four legacy 2D shots re-captured in the same
  battery. Inspected: violet/blue toroidal streams,
  diamond fragments, central collision marker,
  hubs, edges, and white labels are all legible;
  dense-cluster label crowding persists as the
  disclosed Canvas limitation (the list carries
  every node's exact text).
- Folded-name scan of tracked paths empty (new
  module names are genuinely distinct);
  credentials sweep clean (only the standard
  ephemeral local pairing-token plumbing shared by
  all e2e specs).
- No Rust files changed, so Rust totals stand from
  the verified checkpoint (full core 405/4, only the
  4 known live-Ollama `semantic_workflow` failures;
  providers+API 144/0). No backend, schema, or
  contract change.

## What is still open (not claimed)

- Independent Windows rerun of this slice by
  ChatGPT (checklist in the MANIFEST), especially
  the case-insensitive build with the new module
  names, `pnpm check` + `pnpm --filter @anansi/web
  build` on Windows, and Windows screenshots at 1x
  and 2x in animated and reduced-motion states.
- Native Tauri acceptance, preview promotion, M5
  production gates, §32 sandbox, Windows
  verification of Slice 4 — all unchanged and still
  open. This slice completes the §26 Three.js
  renderer; it does not claim M8 complete.
