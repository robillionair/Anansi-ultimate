# M9 SwarmTank Slice 2 — Design: 3D Arena Visualization & Accessible Alternative

Branch: `muse/m9-swarmtank-slice2`, from Windows-verified Slice 1 tip
`75a0b427` (review branch `codex/review-m9-swarmtank-slice1`; Slice 1
Windows-verified PASS: tsc 0, contracts PASS, node 131/131, docs 213,
Rust swarmtank 9/9, forge_restore 4/4, transfer 19/19, e2e 31.9s).

Authority: MASTER_SPEC §38 (arena), §65 (performance & accessibility),
`plans/07-swarmtank.md` Slice 2. Assignment: M9 SwarmTank Slice 2.

## Acceptance criteria

- **AC-1 — Arena placement.** The arena lives inside the existing SwarmTank
  section in Projects / Innovation Studio, rendered for the currently shown
  run (evaluated report or saved run reopened from the list). No new route,
  no navigation change.
- **AC-2 — Pedestal and amphitheater.** A central product monolith on a
  circular stage represents the product concept. Agents sit on concentric
  amphitheater rings around it, rendered as instanced meshes (§38, §65:
  instanced geometry, limited draw calls, bounded counts).
- **AC-3 — Decision clustering.** Convert agents (green) sit closest to the
  pedestal and face it; Confused agents (amber) orbit the perimeter with
  hesitation markers; Reject agents (red) sit at the outer perimeter and
  face outward. Clustering is a pure function of the run's decisions.
- **AC-4 — Raycast picking and popover.** Clicking an agent raycasts to its
  persona id and opens a detail card with role, approximate budget, key
  objection, and perceived value. Clicking empty space clears the selection.
- **AC-5 — Performance guardrails (§65).** Pinned `three@0.187.0` (no new
  dependency); device pixel ratio clamped to 2; `powerPreference:
  'low-power'`; every geometry, material, and texture disposed on unmount
  and on run switch; `webglcontextlost` falls back without losing the
  report or the table.
- **AC-6 — Reduced motion.** Static single frame (no animation loop) when
  the system `prefers-reduced-motion` matches OR the workspace
  reduced-motion setting is on, AND when the arena's own "Reduce motion"
  toggle is checked. Reviewer lesson from §26: the toggle's initial value
  is computed synchronously with `useState(() => isReducedMotion())`
  (matchMedia OR document dataset) — never `useState(false)` + effect —
  so no mount flicker or unmount race.
- **AC-7 — Accessible alternative, always present.** The complete 2D
  persona table (Slice 1) stays rendered below the arena in every state
  (WebGL, reduced motion, WebGL failure), is keyboard navigable, and is
  synchronized with 3D selection in both directions: picking an agent
  marks its table row, and a per-row "Select in arena" button selects the
  agent (scaling it and opening the same detail card).
- **AC-8 — Evidence.** Node tests for layout, builder mounting, raycast
  picking, selection, animation, and disposal; a Playwright e2e spec for
  canvas mount, agent click → popover, table sync, reduced-motion toggle,
  and no-WebGL fallback. Zero unconfigured/paid inference (the arena
  makes no provider calls at all), zero credential leakage.

## Modules (all under `packages/ui/src/`, mirroring §26)

- `swarmtankArenaLayout.ts` — PURE, no `three` import, so node tests and
  the e2e spec can use it without a GL context or module-resolution risk.
  Input: personas + decisions (persona id → decision). Output: one agent
  record per persona: `{ personaId, decision, ring, radius, angle, x, z,
  facingX, facingZ }`, plus the list of ring radii actually used.
  - Groups keep run order (personas array order, filtered by decision).
  - Base radii: Convert 60, Confused 120, Reject 180. Ring spacing 26.
    Agent spacing along a ring 14 units; a ring's capacity is
    `max(1, floor(2πr / 14))`; overflow spills to the next ring outward.
  - Angle: agent k on a ring of capacity C sits at `(k / C) * 2π +
    ringIndex * 0.35` (stagger keeps rings from aligning radially).
  - Position: `x = r·cos(angle)`, `z = r·sin(angle)`, ground plane y 0.
  - Facing (unit vector in XZ): Convert points at the pedestal
    `(-cos a, -sin a)`; Confused points tangentially `(−sin a, cos a)`
    (the orbit direction); Reject points outward `(cos a, sin a)`.
  - Deterministic: no RNG, no clock, no DOM. Same run → same layout.
- `swarmtankThreeScene.ts` — pure scene builder (no renderer), mirroring
  `colliderThreeScene.ts`. Reuses `detectWebGL` and `isReducedMotion`
  from that module; no duplication.
  - Field `#0b2545` (same navy as §26). Pedestal: cylinder stage (radius
    40, height 8) + monolith box (24 × 60 × 24) + canvas label sprite
    "Product concept". Ring lines (muted `#57534e`) for every used radius.
  - Bodies: one `InstancedMesh` of spheres (radius 5) at y 6, per-instance
    color Convert `#22c55e` / Confused `#f59e0b` / Reject `#ef4444`.
  - Noses: one `InstancedMesh` of cones (radius 3, height 9), geometry
    pre-rotated to point +Z, per-instance rotation Y by
    `atan2(facingX, facingZ)`, offset 8 units along facing from the body.
    The nose is the facing indicator; bodies alone carry no orientation.
  - Hesitation markers: one `InstancedMesh` of octahedra (radius 3) above
    each Confused agent (base y 18).
  - Camera (fixed, documented for the e2e spec): top-down, position
    `(0, 640, 0)`, `lookAt(0, 0, 0)`, up `(0, 0, -1)`, fov 45. Top-down is
    deliberate: every ring and facing direction stays readable (a tilted
    camera would hide far-side agents behind the monolith), and picking
    math is exact: `NDC x = x / (tan(22.5°) · 640 · aspect)`,
    `NDC y = −z / (tan(22.5°) · 640)`.
  - `pick(ndcX, ndcY)`: raycast against the bodies mesh only, returns the
    persona id or null. `setSelected(id)`: rescales the selected body to
    1.6× (scale, not color — amber is already the Confused color, so a
    color highlight would be ambiguous) and restores the rest.
    `animate(t)`: Confused agents orbit (`angle + t · 0.05`, matrices
    recomputed; Convert/Reject static) and hesitation markers bob
    (`y = 18 + 3·sin(t·2 + index)`). `dispose()`: traverse-dispose every
    geometry, material, and texture (the §26 pattern).
  - Returns `{ scene, camera, agents, counts, pick, setSelected, animate,
    dispose }` for headless node testing.
- `SwarmtankThreeView.tsx` — renderer lifecycle only, mirroring
  `ColliderThreeView.tsx`: `WebGLRenderer({ canvas, antialias: true,
  powerPreference: 'low-power' })`, DPR clamped to 2, `ResizeObserver`
  resizing camera + renderer together, `webglcontextlost` →
  `onUnavailable`, rAF loop unless `reducedMotion` (then one static
  frame), selection effect → `setSelected` + one frame, click → NDC →
  `pick` → `onSelect`. Canvas `data-three-canvas`, `role="img"`,
  aria-label naming the run's agent counts and pointing at the table.
  Disposal on unmount and on run-key change, before the next build.
- `SwarmtankArena.tsx` — the arena section: heading "SwarmTank 3D arena",
  the "Reduce motion (static arena view)" checkbox (synchronous initial
  value per AC-6), the ThreeView, the selected-persona detail card
  (`aria-label="Selected persona"`: name, role, approximate budget, key
  objection or "None — converted.", perceived value, willingness to pay,
  Close button), and the WebGL-failure note ("3D arena unavailable — the
  complete table below carries the same information."). Failure is
  tracked per run identity (`failedRunKey === runKey`, the §26 lesson:
  a run switch retries WebGL instead of inheriting a stale failure).
- `Swarmtank.tsx` — panel owns `selectedPersonaId` (reset on run change)
  and passes it through `SwarmtankReportView` (new props
  `selectedPersonaId`, `onSelectPersona`); the report view renders the
  arena above the table. Each table row gains a "Select in arena" button
  (`aria-label="Select {name} in arena"`); the selected row shows a
  "Selected in arena." marker. Existing report assertions are unaffected
  (marker renders only when a persona is selected).

## What is NOT in this slice

- Slice 3 scenario controls (§39: sliders, re-evaluation, comparison).
- Provider-backed generation or re-evaluation; the arena visualizes the
  saved run only and makes no provider calls.
- Native save dialog for export; per-agent text sprites (100 labels would
  cost a texture each — the detail card + table carry the text).
- Any Rust/API/contract change: run data already flows from Slice 1, and
  the arena is pure UI over the saved run document.
- No new route, no navigation change, no release-engineering directories.

## Verification plan

- Node (`packages/ui/tests/swarmtank-arena.test.ts` + additions to
  `swarmtank.test.tsx`, both added to the root `test:ui` list): layout
  determinism, ring ordering (Convert radii < Confused < Reject), facing
  vectors (Convert toward center, Reject outward, Confused tangential),
  ring overflow, builder counts/colors, `pick` via `Vector3.project`
  through the real camera, `setSelected` scaling, `animate` moving only
  Confused agents, `dispose` without throwing, static-markup canvas +
  table presence, zero-`innerHTML` grep over the new files.
- E2E (`tests/e2e/swarmtank-arena.spec.ts`, 3 tests, shared run builder:
  collide → Build → import experiment → fill → generate → evaluate):
  arena mount + agent click (layout module + documented top-down
  projection, actual canvas box) → popover contents + table-row marker +
  reverse sync via row button; reduced-motion toggle (workspace setting
  via `startSession(page, true)` plus the arena checkbox) → static note;
  no-WebGL fallback (`addInitScript` getContext stub before navigation,
  the §26 pattern) → fallback note + table intact.
- Full battery after: Rust workspace, node, tsc, contracts `--check`,
  docs, web build, preserved browser battery (Slice 1 spec + collider /
  visualization / transfer / forge / artifacts / backups specs).
- Delivery: `anansi-m9-swarmtank-slice2-2026-10-31.zip` with full-history
  bundle + MANIFEST + SHA-256 sidecar, scratch-verified as in Slice 1.
  Linux evidence never presented as closing Windows verification.
