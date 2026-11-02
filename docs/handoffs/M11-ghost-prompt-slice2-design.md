# M11 GHOST-PROMPT Slice 2 — Design (before code)

## Baseline — including a SHA discrepancy, reported not hidden

Branch `muse/m11-ghost-prompt-slice2` from the
Windows review head for Slice 1. The assignment
and the review MANIFEST both name parent
`9ac467fe835c2b3e839218671fc31efbe8855a8f`,
but that full SHA does not exist: the review
bundle's only head is
`9ac467fec54c28c957428187d998d73599a6a965`
(shared prefix `9ac467fe`, different tail), whose
commit message ("verify M11 GHOST-PROMPT Slice 1
on Windows (NTFS import fix)"), parent
(`0f9eec261`), and diff (review docs plus the
NTFS import fix) are exactly the review the
assignment describes. The branch was therefore
created from that verifiable bundle head — which
is also precisely the assignment's authorized
alternative ("clean tip `0f9eec261` + the NTFS
import qualification"). The discrepancy is recorded
here, in the addendum, and in the MANIFEST so the
reviewer can confirm the tail; nothing was built
on an unverified commit.

Slice 1 Windows verdict recorded (review branch
`codex/review-m11-ghost-prompt-slice1`): tsc 0,
contracts 0 drift, docs 231, UI 178/178, Rust
ghost 11/11, workspace 603 + 4 offline-Ollama
skips, Vite 2.58s, Playwright 18/18 (ghost 2/2,
regressions 16/16). AN-016 stays `in progress`
for the Slice 1 scope. One reviewer wording note:
the review report says "FNV-1a 64-bit"; the
implementation is 32-bit (offset 2166136261) in
both engines, and this slice repeats only the
32-bit claim.

## Scope: §56, §65 (Slice 2 only)

The visual exploration layer for active
GHOST-PROMPT sessions. Deferred to Slice 3:
semantic "Review My Draft" interpretations (§57),
dual-panel comparison (§58), provider-backed
model dispatch. No Slice 3 behavior is built,
stubbed, or implied.

## A. Pure layout (`ghostPromptThreeLayout.ts`, no Three.js import)

Input: a snapshot `{ currentText, fragments,
pauses, events }` where fragments carry
(seq, offset, text, hash), pauses carry
(durationMs, elapsedMs), and events carry
(seq, kind, offset, deleted, inserted,
elapsedMs). Output is deterministic: the same
snapshot always yields the same positions.

- **Coordinate frame.** Baseline along X at y=0,
  spanning [-200, 200]. Character offsets map
  linearly: `x(offset) = -200 + (offset /
  maxOffset) * 400`, where `maxOffset` is the
  largest of the current-text length, any
  fragment's offset + length, and 1. Positions
  are therefore bounded and independent of text
  length.
- **Deleted nodes.** One node per fragment:
  `id = fragment-<seq>`, position
  `(x(offset), 50 + (index % 3) * 22, 0)`.
  Three staggered rows keep nearby nodes from
  overlapping; the row is a function of node
  index only, so it is stable.
- **Threads.** One segment per node, from the
  node position straight down to its baseline
  point `(x(offset), 0, 0)`.
- **Pause markers.** A pause has no offset of its
  own, so it is placed at the baseline point of
  the next edit event after it (by elapsedMs);
  a trailing pause with no following edit sits at
  the end of the baseline. Marker: two concentric
  rings at `(x, 0.5, 0)`.
- **Revision clusters.** Edits are grouped when
  consecutive edits (by seq) have offsets within
  8 characters of the previous edit's offset
  (a "same region" run). A cluster records its
  min/max baseline x and its member seqs, and
  renders as a thin underline bar on the baseline
  spanning the range. Single-edit runs are not
  clusters and render no bar.
- **Camera (fixed, analytic).** PerspectiveCamera
  fov 45 at (0, 30, 320), lookAt (0, 30, 0). All
  content lies in the z=0 plane, so projection is
  exact without a GL context: `ndcX = x /
  (tan(22.5°) * 320 * aspect)`,
  `ndcY = (y - 30) / (tan(22.5°) * 320)`. The e2e
  spec imports this module and clicks projected
  node centers, exactly like the SwarmTank arena
  spec.

## B. Scene builder (`ghostPromptThreeScene.ts`, Three.js, no renderer)

Mirrors the SwarmTank arena builder: it builds a
scene graph and creates no renderer, so node
tests can build, pick, and dispose it headlessly.
Pinned `three@0.187.0` (already a dependency).

- **Baseline.** A crisp white line (thin box,
  400 × 1 × 1) at y=0, plus a dim field plane.
- **Deleted nodes.** One sphere (radius 8) per
  node, violet `#8b5cf6`, transparent, opacity
  0.55. Instancing is unnecessary at Slice 1
  scale (≤500 events, typically a handful of
  fragments); individual meshes keep picking
  trivial and disposal explicit. §65 draw-call
  discipline is preserved by the bounded input:
  one mesh per fragment, one line segments object
  for all threads, one geometry per ring size
  shared across markers.
- **Threads.** A single `LineSegments` object
  holding all node-to-baseline segments, dim
  violet.
- **Pause markers.** Two shared ring geometries
  (radii 6 and 10) per marker, restrained,
  lying flat at y=0.5.
- **Cluster bars.** One thin box per cluster
  spanning its x-range at y=0.5, dim violet.
- **Picking.** `pick(ndcX, ndcY)` raycasts
  against node meshes and returns the fragment
  seq or null. `setSelected(seq | null)` scales
  the selected node 1.5× and restores the rest.
- **Animation.** `animate(t)` breathes pause
  rings only: scale `1 + 0.08 * sin(t * 0.8 +
  phase)`. No particles, no node motion, no
  camera motion. Under reduced motion the
  component never calls `animate` and renders a
  single static frame (or no canvas at all — see
  §C).
- **Disposal.** `dispose()` traverses the scene
  and disposes every geometry, material, and
  texture (shared geometries disposed once).
  The component additionally disposes the
  renderer on unmount. Tests assert disposal by
  listening for `dispose` events on every
  geometry/material.
- **Capability helpers.** Re-exports
  `detectWebGL` and `isReducedMotion` from
  `colliderThreeScene.ts` (synchronous
  matchMedia-or-dataset read, the §26 lesson).

## C. Component (`GhostPrompt3D.tsx`)

Mounted in the GHOST-PROMPT expanded view: the
strip's Review drafts section gains an
`Expand 3D view` / `Collapse 3D view` toggle
rendering `GhostPrompt3D` with the current engine
snapshot. The component owns the renderer
lifecycle (the arena precedent):

- On mount with WebGL available and motion
  allowed: create `WebGLRenderer` with
  `powerPreference: 'low-power'`, DPR clamped to
  2, sized to the canvas client size; build the
  scene; render once, then a `requestAnimationFrame`
  loop calling `animate(t)` only while motion is
  allowed. On unmount or snapshot change: cancel
  the frame, dispose the scene, dispose the
  renderer.
- **WebGL failure.** If `detectWebGL()` is false
  or renderer creation throws, render the 2D
  fallback instead, with a `WebGL unavailable —
  showing the accessible 2D view.` note. A
  `webglcontextlost` listener on the canvas
  switches to the same fallback mid-session
  (the context is not restored; the fallback is
  the recovery).
- **Reduced motion.** State initializes
  synchronously from `isReducedMotion()` (system
  query or workspace dataset) — never from an
  effect that would flash the animated view
  first. A `Reduce motion (static view)`
  checkbox toggles between the canvas and the
  static layout. The static layout IS the 2D
  view, with a `Motion reduced — static view.`
  note. An optional `initialReducedMotion` prop
  exists only as a test seam (unit tests render
  the static view through it); the live path
  always reads the real signal.
- **Accessible 2D view.** An SVG timeline using
  the same layout numbers as the 3D scene:
  baseline line, violet circles for deleted
  nodes, thread lines, pause diamonds, cluster
  bars — plus HTML buttons `Select fragment
  <text>` per node for keyboard users, which set
  the same selection the canvas raycast sets.
  The 3D canvas itself is keyboard-accessible:
  it carries `tabIndex={0}` and ArrowLeft /
  ArrowRight cycle node selection.
- **Detail card.** The selected fragment renders
  in a `Selected fragment` section: fragment text
  (strikethrough), offset, hash, kind, and the
  pause (if any) that preceded its edit.
- **Screen-reader history.** A semantic `<ol>`
  listing every edit event in sequence (seq,
  kind, offset, deleted text, inserted text) and
  every pause (duration), rendered in the 2D /
  static view. The Slice 1 table stays as the
  tabular alternative.
- **Snapshot changes.** The scene rebuilds when
  the snapshot key (JSON of fragments, pauses,
  and current-text length) changes. No
  incremental scene update in this slice; session
  sizes are bounded and rebuilds are cheap.
- **Import qualification.** Every import of the
  ghost modules in new or edited files uses an
  explicit extension (`./GhostPrompt3D.tsx`,
  `./ghostPromptThreeLayout.ts`,
  `./ghostPromptThreeScene.ts`,
  `./GhostPrompt.tsx`, `./ghostPrompt.ts`) —
  the Slice 1 NTFS lesson, applied uniformly.
  Zero `innerHTML` in the new files.

## D. Tests

- Unit (`packages/ui/tests/ghost-prompt-three.test.ts`,
  added to the `test:ui` list): layout
  determinism (same snapshot → identical
  positions; node x monotonic in offset; thread
  endpoints; pause placed at next edit's offset;
  cluster grouping incl. the 8-char rule and
  single-edit exclusion); scene builder headless
  (node/thread/marker/cluster counts, pick by
  projected NDC returns the right seq, setSelected
  scales 1.5× and restores, animate moves only
  rings); disposal (every geometry/material fires
  `dispose`, and dispose twice does not throw);
  `detectWebGL` with a throwing factory → false;
  `isReducedMotion` probe cases; static renders
  (3D section with canvas + toggle, static view
  via the test seam with SVG + `<ol>` + note).
- E2E (`tests/e2e/ghost-prompt-slice2.spec.ts`,
  reusing `startSession` + the Slice 1 enable /
  type / pause / replace / delete flow):
  - Test 1: expand the 3D view, assert the canvas
    renders, project a deleted node's center with
    the pure layout module + fixed camera math,
    click it, and assert the detail card shows
    that fragment; provider traffic unchanged.
  - Test 2: with reduced motion (helper flag),
    assert the static view note and no canvas;
    uncheck the toggle → canvas appears; re-check
    → static again; uncheck, force
    `WEBGL_lose_context` on the canvas, and
    assert the 2D fallback + WebGL note.
- Full verification: Rust workspace suite
  (expect 603 + 4 known live-Ollama failures, no
  Rust change in this slice), strict Clippy/fmt
  (no Rust change to check beyond the baseline),
  node `test:ui`, tsc, contracts `--check`
  (zero drift, no new DTOs), docs validation,
  web build, regression specs (Ask, ghost Slice 1,
  SOMNIA ×3, evaluations, integrated workflow,
  opportunities, SwarmTank), full browser battery
  (expect only the known search real-semantic
  failure).

## E. Acceptance criteria

- AC-1 The 3D view renders retained baseline,
  violet deleted nodes, threads, pause ripples,
  and cluster bars exactly where the pure layout
  says they are (layout tests + e2e click).
- AC-2 Reduced motion snapshots to the static
  layout immediately, with no animation loop
  running; the toggle switches both ways.
- AC-3 WebGL failure at mount and context loss
  mid-session both fall back to the complete 2D
  view; no blank canvas, no exception.
- AC-4 Disposal fires for every geometry /
  material / texture; rebuild on snapshot change
  leaks nothing the tests can observe.
- AC-5 Keyboard users can select every fragment
  from the 2D view, and the screen-reader `<ol>`
  lists every event and pause.
- AC-6 No particle clouds, no gratuitous
  animation, DPR ≤ 2, low-power context.
- AC-7 E2E tests 1–2 green; zero provider
  traffic; contracts drift-free; docs passing.
- AC-8 Checksummed ZIP delivery with
  `review.bundle`, MANIFEST, and sidecars;
  AN-016 advanced for the Slice 2 scope, with
  Slice 3 still deferred.
