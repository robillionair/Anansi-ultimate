# LARP Mode Telemetry Engine — Slice 1 Addendum

Branch `muse/larp-mode-slice1` from Windows-verified M12 Slice 3 review tip
`640f7c24026500…` (review branch `codex/review-m12-product-lifecycle-slice3`,
docs-only over delivery tip `668565d1b`; Milestone 12 CLOSED on Windows).
Baseline discrepancy, fourth in the series: the assignment named tip
`640f7c240974917…`, which does not exist; the review package's MANIFEST
`review_tip` and its bundle head agree on `640f7c24026500…`, which is
authoritative. The assignment's milestone tag `m12-product-lifecycle-verified`
did not travel inside the bundle (branch ref only) — noted, not blocking.
The first attempt at this slice was HELD (assignment's baseline absent,
`cat-file: bad object`, tree clean at `668565d1b`, no branch created)
until the user uploaded
`anansi-m12-product-lifecycle-slice3-windows-verified-2026-11-03.zip`,
whose SHA-256 matched the assignment's `0ef00095…` exactly; the bundle
inside matched its MANIFEST (`4119c9ae…`), its head was exactly
`640f7c24026500…` with parent `668565d1b`, and its diff was docs-only
(BUILD_STATUS, REQUIREMENTS_MATRIX, review report).

## Scope delivered

RGB Chrome toggle, global store with cold-unmount lifecycle, in-memory
telemetry bus, and the perimeter HUD frame with its three widget shells
and a view-selector placeholder. No server op, no new artifact type, no
schema change, no Rust change, no contracts change. Design before code:
`docs/handoffs/larp-mode-slice1-design.md`.

## Files

Added, all under `packages/ui/src/larp-mode/` (explicit `.ts`/`.tsx`
extensions on every new-module import, NTFS rule):

- `hotkey.ts` — pure `matchesLarpHotkey` (`Ctrl+Shift+L`).
- `store.ts` — `useLarpModeStore` (`useSyncExternalStore`, server
  snapshot included), `setLarpMode` / `toggleLarpMode`, and the disposal
  registry (`registerDisposer` / `disposeAll`). OFF runs every disposer
  before subscribers are notified; a throwing disposer cannot keep the
  rest alive.
- `frameScheduler.ts` — injectable-clock/RAF scheduler: 60 FPS while
  interacting, 24 FPS after 5 s idle, 0 FPS while hidden. The loop never
  re-schedules after `stop()`.
- `telemetry.ts` — `AnansiTelemetryStream`: 500-event ring, subscribe /
  snapshot / clear, sanitizing push (drift clamped 0–1, token counts
  floored at 0, never throws). Process singleton `telemetryStream`.
- `MemoryMatrix.tsx` — 48×16 grid of 4 px cells at devicePixelRatio;
  `matrixCellsForNodes` maps the latest memory event's nodes to colors
  (memory → amber Pinned Rules, note → cyan Active Window, artifact →
  magenta Compiled Delta, remainder → dark Evicted).
- `TokenDeltaStream.tsx` — latest-first token-delta rows, capped at 50,
  `raw → compiled (Δ%) · N ms`.
- `CoherenceWaveform.tsx` — dual bezier layers (raw cyan, compiled
  magenta) from `tokenVelocitySeries`; sub-millisecond event gaps
  collapse to 1 ms so velocity stays finite.
- `LarpToggle.tsx` — header toggle: chrome pill, conic-gradient RGB
  frame (hue-rotating while ON, static at low opacity while OFF),
  chromatic-aberration pulse + neon-cyan LED while ON, `aria-pressed`,
  click + global `Ctrl+Shift+L`.
- `LarpHud.tsx` — fixed `100vw x 100vh` grid frame (top/left/center/
  right/bottom rails), `pointer-events: none` except the view buttons,
  view selector 1–6 with local selected state and an explicit
  placeholder ("View N — arrives in Slice 2").
- `larp-mode.css` — all LARP styles, loaded via `@import` at the top of
  `tokens.css` (proven present in the Vite build output CSS).

Modified: `packages/ui/src/Workspace.tsx` (toggle mounted in the header
inside a group span next to the connection badge, so it sits next to
workspace status under the header's `space-between` layout; HUD mounted
in the shell when ON), `packages/ui/src/tokens.css` (the `@import`
line), `package.json` (`test:ui` list gains the new test file).

## Honest boundaries (from the design doc, still true)

- **No live telemetry producer.** No client method returns token counts
  or compiler events, so Slice 1 ships the bus, routing, and widget
  shells with explicit empty states — no synthesized feed presented as
  live data. Unit tests prove routing with directly pushed events; e2e
  proves the frame and toggle, not fake data.
- **No WebGL in Slice 1.** The widgets are 2D canvas. The store's
  texture/geometry disposal requirement is implemented as the generic
  disposal registry; Slice 2 WebGL renderers will register into the same
  registry, and the OFF path already runs it. The verifiable Slice 1
  claim is zero RAF callbacks, zero listeners, and zero retained widget
  state after OFF — asserted in unit tests — not a GPU/VRAM measurement,
  which this environment cannot produce.
- **Scheduler ticks are no-ops in Slice 1.** Slice 1 widgets paint on
  data change, not per frame, so the loop's tick currently does no work;
  the loop exists to prove the lifecycle/throttling contract and is the
  hook Slice 2 painters will consume. An empty RAF callback costs
  microseconds and stops entirely on OFF/hidden, so this does not
  contradict the zero-overhead goal.
- **Fonts.** `Berkeley Mono` / `JetBrains Mono` head the stack with
  system-monospace fallback. No font files are bundled (Berkeley Mono is
  commercial; the codebase forbids remote font assets).
- **"Global header bar" mapping.** The app has no separate global header
  component; the `Workspace` header renders on every view, so mounting
  there makes the toggle effectively global.

## Architecture decisions

- `useSyncExternalStore` over module state, not a new dependency, so
  the toggle, the HUD mount, and the hotkey all read one source of
  truth and tests can drive it without a browser.
- Widgets take data props and stay presentational; only `LarpHud`
  subscribes to the stream. That keeps every widget unit-testable via
  static markup (effects do not run there) and leaves click behavior to
  e2e, per the project's established split.
- The matrix node-kind → role-color mapping is a stated Slice 1 proxy,
  not a real allocation taxonomy; it only colors cells when real nodes
  arrive, and the grid is honestly all-Evicted otherwise.

## Tests passed

- `tsc --noEmit`: 0 errors.
- `node scripts/contracts.mjs --check`: 0 drift.
- `node scripts/validate-docs.mjs`: PASS (249 files with this addendum).
- `npm run test:ui`: 241/241 (228 + 13 new LARP tests).
- Vite production build: clean; `.larp-hud` confirmed in the bundled CSS.
- `tests/e2e/larp-mode-slice1.spec.ts`: 2/2 (toggle click → HUD with all
  widgets + view buttons 1–6 → view 3 placeholder → `Ctrl+Shift+L` →
  full unmount → clean re-mount; cold-unmount leg asserts canvases and
  frame are gone and the toggle reads OFF).
- Full regression battery: 30/30 (lifecycle slices 1–3 2/2 each, LARP
  Slice 1 2/2, ghost 1–3 2/2 each, somnia 1–3 2/2 each, evaluations 1/1,
  integrated_workflow 1/1, opportunities 1/1, swarmtank-scenario 3/3,
  swarmtank-arena 3/3, swarmtank-slice1 1/1), run sequentially on an
  idle machine after the Rust suite — no concurrent-load flake this
  round.
- Rust workspace: 616 passed + 4 known live-Ollama env failures
  (`semantic_workflow`, identical to the M12 baselines; no Rust files
  touched), run with `--no-fail-fast` to a file and summed from the
  `test result` lines plus the failures sections directly.

## Tests failed / in-slice fixes (all disclosed)

- My first e2e run failed in the harness, not the app: I called
  `session.pairAndOpen()` with no argument, but it takes the workspace
  id. Fixed to `session.pairAndOpen(session.workspaceId)`; both tests
  then progressed.
- My first passing-shape run then failed on strict mode: `[ LARP:
  ACTIVE ]` matched both the toggle label and the HUD status line.
  Fixed by scoping the ACTIVE assertions (toggle `aria-pressed`, HUD
  scoped text) rather than changing app text.
- Two tsc findings in my own test helpers (a too-narrow structural type
  for `latestMemoryNodes`, an un-narrowed union passed to
  `compressionDelta`) fixed by narrowing types, not by weakening prod
  code. No app-code defects were found by the suite.

## Known limitations

- View buttons 1–6 select state only; centerpiece content arrives in
  Slice 2.
- Telemetry stream starts empty in production; the first real producer
  is Slice 2 scope.
- The HUD centerpiece panel covers the viewport center while ON; the
  frame is otherwise click-through, and one keypress exits.
- **No MATRIX row exists for LARP Slice 1.** LARP Mode is new work
  with no requirement ID in `REQUIREMENTS_MATRIX.md` or
  `MASTER_SPEC.md`; none was invented for this slice. If the owner
  assigns one, the row belongs in a follow-up commit, not retrofitted
  here.

## Security

No new network, filesystem, or provider surface. The hotkey listener is
window-scoped and only toggles local UI state. The telemetry bus is
in-memory only and holds no credentials; its events are UI state, never
transmitted. Credentials sweep over the 16-file diff: clean (no matches).

## Last verified commit

Tip recorded in the delivery MANIFEST.

## Next

Windows rerun of this slice (checklist in the delivery MANIFEST). Slice 2
(centerpiece views 1–6, live telemetry producers) is not commissioned.
