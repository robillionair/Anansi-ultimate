# LARP Mode Telemetry Engine — Slice 2 Design

Branch `muse/larp-mode-slice2` from Windows-verified LARP Slice 1 review tip
`f9f8bc245340a7994b898b014a345f255dead8fc` (review branch
`codex/review-larp-mode-slice1`, docs-only over delivery tip `b840cb437`;
Slice 1 Windows-verified, no findings). Baseline note: this is the first
assignment in the series whose named tail MATCHED the bundle head exactly
— no discrepancy to record. The review report confirms the Slice 1
delivery (ZIP `92fe23d7…`, bundle `d0999c18…`, 19 files) and its verdict
(tsc 0, contracts 0 drift, docs 250 with report, UI 241/241, Rust 616 + 4,
battery 30/30).

## Scope (Slice 2 only)

Multi-pass WebGL post-processing pipeline, centerpiece View 1 (3D memory
vault) and View 2 (neural funnel), view switching by keys `1`/`2` and by
HUD button, and live telemetry producers from Ask ANANSI runs and
Workspace memory loads. No server op, no new artifact type, no schema
change, no Rust change, no contracts change. Views 3–6 stay placeholders;
Slice 3 owns them.

## Grounding: what the producers can honestly report

Read before designing (not assumed):

- `AskAnansi.handleSubmit` polls `startAsk`/`pollAsk` to completion and
  sets `run.response`. The response manifest carries
  `manifest.estimatedTokens` and `manifest.characterCount`, computed
  server-side in `crates/core/src/ask.rs` as `compiled_prompt.len() / 4`
  — the COMPILED system prompt's estimate, not the raw user prompt's.
- The raw input estimate is therefore client-computed the same way:
  `Math.ceil(prompt.length / 4)` with the same `/4` estimator the server
  uses, always labeled estimated (the GhostPrompt precedent: estimated
  tokens are always labeled).
- Output has no compilation step in Ask: the answer estimate goes in
  both output fields, documented as such — raw and compiled coincide
  because there is no output compiler.
- A run that completes with status `insufficientEvidence` still ran the
  context compiler (it built the prompt and found nothing), so it fires
  a `compilation` event labeled `ask-run-completed`; it does NOT fire a
  token-delta (compiled estimate 0 would fake a −100% compression).
  Token-delta fires only when status is `answered` and
  `manifest.estimatedTokens > 0`.
- `Memory.tsx` loads via `client.listMemories` (`MemoryMetadata`: `id`,
  `kind: string`, `title`). The producer maps `kind === 'note'` → node
  kind `note`, everything else → `memory`, and pushes one `memory`
  event per successful refresh (mount + filter changes + post-save
  refreshes; the ring bounds the volume).
- The GhostPrompt comparison dispatch (`askOnce`) is a separate feature
  and is NOT hooked in this slice; the assignment's "Ask ANANSI prompt
  compilation" is the main Ask flow.

## Module map

All new-module imports carry explicit `.ts`/`.tsx` extensions (NTFS rule).
Three.js addons resolve via the `three/addons/*` export map (three
0.187.0, pnpm-isolated under `packages/ui/node_modules`); `EffectComposer`
and `UnrealBloomPass` both expose real `dispose()` methods in this
version (verified in the installed sources).

- `postprocessing/shaders.ts` — pure shader objects, no GL needed:
  `chromaticAberrationShader` (uniform `uOffset = 0.0012`, radial
  R/B channel shift around the viewport center) and `filmGrainShader`
  (2D simplex noise after Ashima, uniforms `uTime`, `uAmount = 0.05`).
  Unit-tested for uniform defaults and fragment contents.
- `postprocessing/LarpComposer.ts` — pipeline assembly + disposal:
  `clampPixelRatio(dpr)` (`Math.min(dpr, 2)`), `buildLarpPasses` taking
  an injected composer factory (so unit tests assert pass order —
  Render → Bloom → Aberration → Grain → Output — without GL), and the
  real `createLarpComposer(renderer, scene, camera, size)` factory used
  by View 1 (bloom: strength 0.6, radius 0.4, threshold 0.85 — smooth
  falloff, not a hard cut). `disposeLarpComposer` calls
  `composer.dispose()`, each pass's `dispose()` where present, and
  releases the composer's render targets; the view additionally disposes
  its own geometries/materials and calls `renderer.dispose()` +
  `forceContextLoss()`. All disposal calls are idempotent-guarded so the
  store's `disposeAll()` on OFF and React's unmount cleanup can both run
  them safely.
- `views/particleLayout.ts` — pure, seeded (mulberry32) layout:
  `clusterCentroids(nodes, fallback)` and `generateParticles(seed,
  centroids, count)` returning position/scale/seed arrays. 50,000
  particles distributed around centroids with a Gaussian-ish spread
  (Box–Muller over uniform draws). Deterministic: same seed + same
  nodes → byte-identical arrays (unit-tested). Nodes come from the
  latest `memory` event snapshot at mount; with no nodes, three fallback
  centroids (Memory / Projects / Notes) keep the vault renderable.
- `views/View1MemoryVault.tsx` — `THREE.Points` with a custom
  `ShaderMaterial` (optical bokeh): vertex passes view depth and a
  per-particle size; fragment draws a soft disc whose alpha falls with
  distance from the focal plane (`uFocalZ`), so focal-plane particles
  render as sharp white-hot cores and off-plane ones as translucent
  discs; `THREE.AdditiveBlending`, `depthWrite: false`. Slow rotation
  (skipped when reduced motion). Cluster tags: centroid positions
  projected per frame through `vector.project(camera)` into NDC →
  CSS left/top on tag divs, updated imperatively via refs (no React
  state per frame). A `data-frames` counter on the container, bumped
  via ref per rendered frame, is the e2e-visible proof of rendering.
  WebGL creation failure renders an explicit fallback panel (centroid
  list as text) instead of a blank canvas — same pattern as the
  existing 3D views' WebGL-failure fallback.
- `views/bezier.ts` — pure math: `cubicBezierPoint(p0, p1, p2, p3, t)`,
  `filamentLayout` (4 input streams → singularity → L1/L2/L3 blocks,
  fixed control-point geometry), and pulse lifecycle
  (`spawnPulse`, `advancePulses` with a 6 s life, `expirePulses`).
  Unit-tested at t = 0 / 0.5 / 1 and for expiry.
- `views/View2NeuralFunnel.tsx` — high-DPI 2D canvas (the assignment
  allows 2D/WebGL; 2D is the honest choice for ultra-thin glowing
  filaments, since WebGL lines cannot exceed 1 px or glow without quad
  extrusion). Filaments stroked in layered alpha for glow (amber/gold
  in, cyan out — input streams amber/gold, compiled fan-out cyan to
  match the HUD palette), singularity node labeled
  `ANANSI.COMPILER // COMPRESSED`, L1/L2/L3 blocks. Pulses: spawned
  from `compilation` events — live via subscription AND seeded on mount
  from compilation events in the stream snapshot (last 60 s), so mount
  order cannot lose a pulse. `data-pulses-fired` (cumulative, via ref)
  is the e2e-visible counter.
- `telemetryProducers.ts` — `reportAskRun(prompt, response, latencyMs,
  stream?)` and `reportMemorySnapshot(metadatas, stream?)`, pushing to
  the singleton by default and to an injected stream in tests. Both
  are no-ops on empty input (no prompt, no memories) rather than
  pushing empty events.
- `telemetry.ts` (modified, additive only) — `token-delta` gains
  optional `estimated?: boolean`, passed through sanitization.
  `TokenDeltaStream.tsx` renders an `est.` badge when set. Existing
  tests construct events without the field and are unaffected.
- `LarpHud.tsx` (modified) — view state now drives the centerpiece:
  view 1 → `View1MemoryVault`, view 2 → `View2NeuralFunnel`, views 3–6
  → the existing placeholder. A window keydown effect maps bare `1`–`6`
  (no modifiers) to `setView`, IGNORING events whose target is an
  input, textarea, select, or content-editable element — typing `1` in
  the Ask box must never switch views. Buttons keep working as before.
- `AskAnansi.tsx` (modified, one call site) — on the completed branch
  of `handleSubmit`, after `setResult`, call `reportAskRun` with the
  measured `performance.now()` latency. No change to the Ask flow
  itself; a producer failure must never break Ask, so the call is
  wrapped in try/catch (a dropped telemetry event is acceptable; a
  broken Ask run is not).
- `Memory.tsx` (modified, one call site) — after `setMemories` in
  `refresh`, call `reportMemorySnapshot`. Same try/catch rule.
- `larp-mode.css` (appended) — view container, tag callouts with
  0.5px leader lines, fallback panel, View 2 canvas sizing. No
  border-radius on HUD panels, no box-shadows (unchanged rule).

## Frame loop + disposal, precisely

Each view owns its RAF loop (not the HUD scheduler — the scheduler's
ticks are still no-ops from Slice 1 and stay that way; views render on
their own loops so disposal is per-view). View unmount (switch or OFF)
cancels RAF, disposes composer + passes + geometries + materials,
disposes the renderer, and forces context loss. The view registers the
same cleanup as a store disposer, so OFF disposes even if React cleanup
ordering surprises us; guards make double-dispose a no-op. Switching
views therefore never leaks the previous scene's GL objects — asserted
in e2e by canvas replacement and in unit tests by disposer invocation.

## Performance notes (stated, not promised)

- 50,000 points is one draw call; bloom cost scales with backing size,
  which is the centerpiece size at `min(devicePixelRatio, 2)` — in
  headless e2e (DPR 1) that is roughly 720×660, fine for software GL;
  on a real 4K display it is a real GPU's job, which is the point of
  the DPR clamp. First-frame time in software GL may reach seconds, so
  the e2e frame assertion uses a generous timeout, not the default 8 s.
- View 2's 2D canvas repaints on data change and on a slow pulse
  animation loop; filament count is fixed (4 in, 3 out), so per-frame
  cost is trivial.

## Tests

- `packages/ui/tests/larp-views.test.tsx` (added to `test:ui`): particle
  layout determinism + count + centroid clustering + bounds; bokeh
  shader uniform defaults; bezier endpoints/midpoint; pulse
  spawn/advance/expire; composer pass order via injected factory; DPR
  clamp (1 → 1, 2 → 2, 3 → 2); aberration offset 0.0012; producers
  (Ask answered → compilation + estimated token-delta with the /4
  math; insufficientEvidence → compilation only; empty memory list →
  no event; memory kind mapping); estimated badge in the delta stream.
- `tests/e2e/larp-mode-slice2.spec.ts`: toggle on → View 1 canvas
  present + `data-frames` ≥ 1 (generous timeout); button → View 2
  canvas + filaments + singularity label; key `2` → View 2, key `1` →
  View 1 (from a non-input focus target); Ask run (unrelated question
  → result section) → View 2 `data-pulses-fired` ≥ 1; OFF → HUD gone
  entirely; WebGL-failure leg (stubbed `getContext` → null, same
  pattern as the arena spec) → View 1 fallback panel visible.
  Click/key behavior is e2e-only, as established.

## Verification commands

`./node_modules/.bin/tsc --noEmit`, `node scripts/contracts.mjs --check`,
`node scripts/validate-docs.mjs`, `npm run test:ui`,
`cd apps/web && node node_modules/vite/bin/vite.js build` (rebuild before
e2e — preview serves `dist`), then the Slice 2 spec plus the full
regression battery (now 32 tests) sequentially on an idle machine, then
the Rust suite (no Rust touched; expect 616 + 4).
