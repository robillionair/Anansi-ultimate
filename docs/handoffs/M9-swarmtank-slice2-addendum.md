# M9 SwarmTank Slice 2 — Addendum

Branch: `muse/m9-swarmtank-slice2`
Baseline: `75a0b427` (Windows-verified Slice 1 tip; review branch
`codex/review-m9-swarmtank-slice1`, docs-only delta over Slice 1 runtime
`9f9047852`: milestone plan elaboration + Windows review report)
Design: `docs/handoffs/M9-swarmtank-slice2-design.md` (committed before code, `7a3278a70`)

## What was built

A Three.js 3D arena over the saved SwarmTank run, inside the existing
SwarmTank section (no new route, no navigation change):

- **Arena (§38).** Central product monolith on a circular stage, concentric
  amphitheater rings, one instanced agent per persona. Convert agents
  (green `#22c55e`) sit closest and face the pedestal; Confused agents
  (amber `#f59e0b`) orbit the perimeter tangentially with hesitation
  markers; Reject agents (red `#ef4444`) sit outermost and face away.
  Layout is a pure function of run order (`swarmtankArenaLayout.ts`, no
  Three.js import, no RNG, no clock) — the same run always lays out
  identically, and node tests plus the e2e spec compute from the same
  function the canvas renders.
- **Raycast picking and detail card.** Clicking an agent raycasts against
  the bodies mesh only and opens a detail card (name, role, approximate
  budget, decision, key objection or "None — converted.", perceived value,
  willingness to pay, Close button). Clicking empty space clears the
  selection. Selection scales the picked body to 1.6× — scale, not color,
  because amber is already the Confused color and a color highlight would
  be ambiguous.
- **Camera, deliberately top-down.** Fixed at `(0, 640, 0)`, up `(0, 0, -1)`,
  fov 45, so every ring and facing direction stays readable (a tilted
  camera would hide far-side agents behind the monolith) and picking math
  is exact: `NDC x = x / (tan(22.5°) · 640 · aspect)`,
  `NDC y = −z / (tan(22.5°) · 640)`. The e2e spec clicks from this formula
  and the layout module, not from recorded pixels.
- **Performance guardrails (§65).** Pinned `three@0.187.0` (no new
  dependency); DPR clamped to 2; `powerPreference: 'low-power'`; every
  geometry, material, and texture disposed on unmount and on run switch;
  `webglcontextlost` falls back to a note without losing the report or
  the table. Three instanced meshes total (bodies, noses, markers) plus
  one line loop per used ring.
- **Reduced motion.** One static frame, no animation loop, when the system
  `prefers-reduced-motion` matches OR the workspace setting is on, or
  when the arena's own "Reduce motion (static arena view)" checkbox is
  checked. Reviewer lesson from §26 applied twice: the checkbox initial
  value comes from `useState(() => isReducedMotion())` (matchMedia OR
  document dataset), never `useState(false)` + effect; and WebGL failure
  is tracked per run identity (`failedRunKey === runKey`), so a run
  switch retries WebGL instead of inheriting a stale failure.
- **Accessible alternative, always present.** The complete Slice 1 persona
  table stays rendered below the arena in every state (WebGL, reduced
  motion, WebGL failure) and is synchronized both ways: picking an agent
  marks its row ("Selected in arena."), and each row's "Select in arena"
  button (with a per-persona `aria-label`, so 20 identical buttons do
  not share one accessible name) selects the agent and opens the card.
  Only the pedestal carries a 3D label sprite; per-agent text lives in
  the card and the table, not in a texture per agent.

## Commits

1. `7a3278a70` — design before code.
2. `356407e11` — arena layout, scene builder, view, section, panel/report
   integration, node tests.
3. `0b75f9145` — e2e spec + per-persona row-button accessible names
   (the spec exposed 20 identically-named buttons).

## Defects found by the tests, fixed at their cause

- Node pick test failed on a stale camera matrix: `Vector3.project`
  needs `updateMatrixWorld()` first (the production `pick()` already
  does this internally). Test fixed, not production.
- Node scale assertion failed on float32 precision (`1.600000023841858`
  vs `1.6`). Assertion now uses a tolerance.
- E2E heading collision: `SwarmTank` also matches `SwarmTank input`;
  the spec now matches exactly. The evaluate notice renders at page
  level, not inside the SwarmTank section; the spec un-scoped it.
- E2E run with seed `arena-seed-1` produces no Reject agents, so the
  reverse-sync leg now targets any non-clicked agent with the card's
  own objection fallback, instead of assuming a Reject exists.
- The arena's own comment tripped the Slice 1 zero-`innerHTML` grep
  (the word in prose); reworded to "inserted as markup".

## Known limitations (deferred, not claimed)

- No Slice 3 scenario controls (§39: sliders, re-evaluation, comparison).
- The arena visualizes the saved run only and makes no provider calls;
  no provider-backed generation or re-evaluation is implied.
- Per-agent 3D labels are intentionally absent (texture-per-agent cost);
  the detail card and the table carry the text.
- Desktop-shell (Tauri) rendering of the arena is not verified on this
  Linux VM; the component is shared UI, but the Windows rerun checklist
  covers confirming it in the desktop shell.

## Linux verification evidence

Sequential, on this Linux VM, branch tip `0b75f9145` (docs commit pending
at packaging time):

- Rust workspace: **562 passed, 4 failed** — the 4 are the known live-Ollama
  `semantic_workflow` environment failures, identical to Slice 1 (no Rust
  change in this slice). Preserved suites: collider 49 / transfer 19 /
  forge_restore 4.
- Strict Clippy (`--workspace --all-targets -D warnings`) clean; `fmt --check`
  clean; `tsc --noEmit` exit 0; contracts `--check` exit 0; docs validation
  PASS at 216 files; web build clean.
- Node `test:ui`: **138/138** (131 preserved + 7 new arena tests).
- Browser battery under the shared lease: **18/18 tests across 15 specs** —
  the new `swarmtank-arena` spec (3 tests) plus preserved Slice 1,
  collider, Three.js visualization + fallback, Canvas visualization +
  screenshots, document input, URL input, pipeline overview, transfer
  concurrency, forge, artifacts, and backups specs. No regressions.
- Working tree clean at packaging; no schema, contract, or Rust change;
  UI-only slice over the Slice 1 run document.

## Windows rerun checklist (reviewer)

- [ ] `node scripts/cargo.mjs test --locked --workspace` on Windows (expect
      the same totals minus the 4 known live-Ollama `semantic_workflow`
      environment failures; no Rust change in this slice).
- [ ] Node `test:ui` (expect 138/138), tsc exit 0, contracts `--check` exit 0,
      docs validation PASS, web build PASS.
- [ ] Playwright: `tests/e2e/swarmtank-arena.spec.ts` (3 tests) plus the
      preserved Slice 1 and collider/transfer/forge/artifacts specs.
- [ ] Confirm the arena renders in the desktop shell (Tauri) with no
      provider traffic on load, and that the reduced-motion toggle and
      the no-WebGL fallback behave there.

## Remaining gates (unchanged by this slice)

Windows verification of this slice (checklist above) and of Slice 1's
delivery note (Slice 1 itself is Windows-verified; this slice's rerun is
separate); M7 native desktop acceptance + preview promotion; §32 sandbox;
M5 production gates; Windows verification of M8 Slice 4.
