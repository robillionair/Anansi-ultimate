# M8 visualization repair — bounded design and acceptance criteria

Baseline: reviewer tip `5a050f18ff2a4cc424a5f01a720026ba95722587`
(review branch `codex/review-m8-visualization`),
one docs-only reviewer commit over the visualization
runtime `93cf77605`.
Branch: `muse/m8-visualization-repair`.
Design committed before any repair code.

## Findings (from `docs/reviews/windows-m8-visualization.md`)

- VIS-01 (P1, reproduced build blocker): the helper
  `packages/ui/src/colliderVisualization.ts` and the
  component `packages/ui/src/ColliderVisualization.tsx`
  differ only in case. On a case-insensitive
  filesystem, the extensionless import
  `./ColliderVisualization` resolves to the helper
  `.ts` before the component `.tsx`, so TypeScript
  exits 1 (TS2305, TS1261/TS1149), Vite exits 1
  (MISSING_EXPORT), and the UI suite reports 43
  pass with 3 failed test-file loads instead of the
  expected 105 cases. Linux is case-sensitive, so
  this never surfaced here.
- VIS-02 (P2, source-level observation): `drawScene`
  fills the canvas with `#0b2545` and then draws
  node labels with the exact same `#0b2545` as
  `fillText` color, with no contrasting backing —
  the labels are unreadable on the canvas. Browser
  visual acceptance was not performed by the
  reviewer because VIS-01 prevents building the app.
- Requirement coverage: §26 (Three.js, toroidal
  particle streams, instanced rendering,
  GPU-efficient materials, adaptive quality, plus
  the accessible 2D alternative) remains partially
  implemented. The Canvas/list workflow is the
  accessible fallback and must be preserved, not
  erased; the Three.js renderer stays unfinished and
  must not be claimed.

## Repair

1. VIS-01: rename the helper to a genuinely
   distinct base name, `colliderVisualizationScene.ts`
   (the reviewer's suggested name), via `git mv` so
   Git tracks the rename. Update every import:
   `ColliderVisualization.tsx` (helper import) and
   `collider-visualization.test.ts` (helper import
   and the no-markup file-path assertion). The
   component import in `Collider.tsx`
   (`./ColliderVisualization`) is unchanged — it
   already names the component. No casing checks
   disabled, no module-resolution settings altered,
   no test weakened, no extension forced at a single
   call site as a workaround. Public behavior
   (derived scene, layout, hit-test, component API)
   is unchanged; only the module path changes.
   Verification includes a case-insensitive
   collision scan of tracked names (`sort -f |
   uniq -d -i`) and a simulated case-insensitive
   checkout (copy the tree through a
   case-folding check) to prove no pair collides.
2. VIS-02: draw labels in restrained white
   (`#f5f5f4`, the §26 highlight) over a
   semi-opaque dark backing rect sized by
   `measureText`, so labels stay legible over nodes,
   edges, and the navy field. Verify with actual
   rendered screenshots — not only the list text:
   paused normal-DPI, reduced-motion, and
   high-DPI (deviceScaleFactor 2) views — inspected
   as images, and shipped as evidence. The
   screenshot steps live in the browser specs behind
   an opt-in output directory so the specs stay
   green without it.
3. Scope guard: no behavior change beyond the rename
   and the label drawing. Canvas/list remains the
   accessible fallback. Three.js and the remaining
   §26 requirements stay recorded as unfinished in
   the design, addendum, BUILD_STATUS, and plan
   copy; nothing in this repair claims M8 complete.
   Antigravity directories untouched. No backend,
   schema, or contract change.

## Acceptance criteria (all must pass before delivery)

1. No two tracked files under `packages/ui/src/`
   differ only by case; the helper's new name is
   `colliderVisualizationScene.ts`.
2. TypeScript exits 0 with casing checks intact;
   Vite web build exits 0; the UI suite reports
   105/105 individual cases (not file-load
   failures).
3. Canvas label pixels are light-on-dark in
   inspected screenshots at normal DPI (paused),
   reduced motion, and deviceScaleFactor 2.
4. The canonical visualization browser workflow
   passes unchanged (all assertions preserved),
   plus the document and URL saved-experiment specs
   (they mount the renamed component's view).
5. Strict Clippy/fmt/tsc/contracts/docs/builds
   clean; focused Rust suites unchanged (no backend
   change); docs validation passes.
6. Focused repair package: checksummed ZIP with
   Git bundle, exact baseline/tip, evidence
   (including screenshots), and Windows checklist.
