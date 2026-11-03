# LARP Mode Telemetry Engine — Slice 1 Design

Branch `muse/larp-mode-slice1` from Windows-verified M12 Slice 3 review tip
`640f7c24026500…` (review branch `codex/review-m12-product-lifecycle-slice3`,
docs-only over delivery tip `668565d1b`; Milestone 12 CLOSED on Windows).
Baseline discrepancy, fourth in the series: the assignment named tip
`640f7c240974917…`, which does not exist; the review package's MANIFEST
`review_tip` and its bundle head agree on `640f7c24026500…`, which is
authoritative. The assignment's milestone tag `m12-product-lifecycle-verified`
did not travel inside the bundle (branch ref only) — noted, not blocking.

## Scope (Slice 1 only)

RGB Chrome toggle, global store with cold-unmount lifecycle, in-memory
telemetry bus, and the perimeter HUD frame with its three widget shells and a
view-selector placeholder. No server op, no new artifact type, no schema
change, no Rust change, no contracts change. Slice 2 owns the centerpiece
views 1–6 and any live telemetry producer.

## Module map (`packages/ui/src/larp-mode/`)

All new-module imports carry explicit `.ts`/`.tsx` extensions (NTFS rule).

- `store.ts` — `useLarpModeStore` (React `useSyncExternalStore` over a tiny
  module store, no new dependency), `setLarpMode`, `toggleLarpMode`, and the
  disposal registry (`registerDisposer` / `disposeAll`). Toggling OFF runs
  every registered disposer, clears the registry, and stops the frame
  scheduler. That is the cold-unmount contract.
- `frameScheduler.ts` — `createFrameScheduler` with injectable
  requestAnimationFrame / cancel / clock so unit tests can drive it without
  a browser. Rules: 60 FPS while interacting, 24 FPS after 5 s without
  interaction, 0 FPS while `document.hidden`. Production wires
  `visibilitychange` and pointer/key activity listeners; all listeners are
  registered disposers, so OFF removes them too.
- `hotkey.ts` — pure `matchesLarpHotkey(event)`: `ctrlKey && shiftKey &&
  key.toLowerCase() === 'l'`. No existing `Ctrl+Shift+L` binding exists in
  the UI (the only keydown handler is canvas-scoped in GhostPrompt3D), so
  there is no conflict to resolve.
- `telemetry.ts` — `AnansiTelemetryStream`: bounded in-memory ring (500
  events), `push`, `subscribe` (returns unsubscribe), `snapshot`, `clear`.
  Event kinds: `token-delta` (raw/compiled input/output tokens, latency ms),
  `drift` (score clamped to 0.000–1.000, negative token counts floored at 0),
  `memory` (`memory_nodes[]` of notes/memories/artifacts), `compilation`
  (labelled trigger). Invalid payloads never crash the HUD: out-of-range
  values are clamped/floored, never thrown.
- `LarpToggle.tsx` — the header toggle (below).
- `LarpHud.tsx` — the perimeter frame (below).
- `MemoryMatrix.tsx`, `TokenDeltaStream.tsx`, `CoherenceWaveform.tsx` —
  widget shells. Drawing math that can be pure is pure and unit-tested
  (`matrixCellsForNodes`, `tokenVelocitySeries`); canvas painting itself
  runs only in browser effects and is covered by e2e presence/structure
  assertions, not by node tests (no canvas in node:test).
- `larp-mode.css` — all LARP styles, loaded via `@import` at the top of
  `tokens.css` (the UI package's single stylesheet entry point, imported by
  `apps/web/src/main.tsx`). Verified in the Vite build output.

## Honest boundaries (stated, not smoothed over)

- **No live telemetry producer exists in Slice 1.** No client method
  returns token counts or compiler events, so inventing a "live" feed
  would be fabrication. The stream, its routing, and the widget shells
  are real; the widgets render explicit empty states ("No telemetry
  events yet.") until a real producer lands (Slice 2 scope). Unit tests
  prove routing with directly pushed events; e2e proves the frame and
  toggle, not fake data.
- **No WebGL in Slice 1.** The HUD widgets are 2D canvas. The store's
  "dispose textures/geometries" requirement is therefore implemented as
  the generic disposal registry: Slice 1 widgets register RAF handles and
  listeners; Slice 2 WebGL renderers will register their context/texture
  disposers into the same registry, and the OFF path already runs them.
  The verifiable Slice 1 claim is zero RAF callbacks, zero listeners,
  and zero retained widget state after OFF — asserted in unit tests —
  not a GPU/VRAM measurement, which this environment cannot produce.
- **Fonts.** `Berkeley Mono` / `JetBrains Mono` are named first in the
  stack (`"Berkeley Mono","JetBrains Mono",ui-monospace,Menlo,monospace`).
  No font files are bundled: Berkeley Mono is commercial, and the
  codebase already forbids remote font assets. Where neither is
  installed, the stack falls back to system monospace. This is a
  graceful degradation, not a bundled-font claim.
- **"Global header bar" mapping.** The app has no separate global header
  component; the header inside `Workspace.tsx`'s `<main>` (eyebrow +
  connection badge, the workspace status) renders on every view, so the
  toggle mounts there, next to the badge, and is effectively global.
  The HUD mounts in `Workspace` inside the shell, so it wraps the
  viewport regardless of view.

## LarpToggle

`<button>` with `aria-pressed`, label `[ LARP: OFF ]` / `[ LARP: ACTIVE ]`.
OFF: brushed-chrome pill (layered metallic linear gradients + bevel
highlight), RGB border present but static at low opacity (subtle
iridescent edge). ON: the same border animates (rotating conic gradient
cycling cyan → neon magenta → electric violet → gold → lime) with a
chromatic-aberration pulse layer, plus an active neon-cyan status LED.
Click toggles; `Ctrl+Shift+L` (global keydown listener in the toggle's
effect, always mounted) toggles with `preventDefault`. Reduced motion
(`document.documentElement.dataset.reducedMotion === 'true'`, the app's
existing signal) disables the border animation via CSS.

## LarpHud

`position: fixed; inset: 0;` CSS grid: top rail, left rail, center, right
rail, bottom rail; `100vw x 100vh`. The frame itself is
`pointer-events: none` so the underlying app stays usable; only the view
selector buttons are `pointer-events: auto`. Visual rules: pure `#030305`
void background (semi-transparent over the app so context stays visible),
`0.5px` `#16161a` hairlines, sharp rectangles (no border-radius, no
box-shadow — the assignment's "no 90s AI boxy graphics" rule; the toggle
pill is the single exception because the assignment explicitly specifies
a pill). Typography is the mono stack with `tabular-nums` throughout.

- **L1/L2/L3 Memory Allocation Matrix**: canvas grid of `4px x 4px` cells
  at devicePixelRatio, colors Amber `#ff8a3d` (Pinned Rules), Cyan
  `#00f0ff` (Active Window), Magenta `#ff1e7a` (Compiled Delta), Dark
  `#18181f` (Evicted), with a legend. Cells derive from the latest
  `memory` event's nodes; with no event the grid renders all-Evicted
  plus the empty-state note.
- **Token Compression Delta Stream**: list of `token-delta` events,
  latest first, capped at 50 rendered rows: `raw → compiled (Δ%) · N ms`.
  Empty state when the stream is empty.
- **Coherence Waveforms**: one canvas, two anti-aliased bezier layers —
  raw-token velocity and compiled-token velocity derived from event
  timestamps (events closer than 1 ms apart collapse to avoid
  divide-by-zero spikes). Empty state when fewer than two token events
  exist.
- **Centerpiece placeholder**: labelled region with view selector buttons
  `1`–`6` (selected state held locally, default `1`); content is an
  explicit placeholder ("View N — arrives in Slice 2"). Selector state
  works; no view content is faked.

## Frame scheduler + cold unmount (verification-relevant)

ON mounts the HUD, starts the scheduler, and registers: the RAF handle,
the `visibilitychange` listener, and the pointer/key activity listeners.
OFF (toggle or hotkey) runs `disposeAll()`: scheduler stopped (no further
RAF callbacks — the loop checks a stopped flag before re-scheduling),
listeners removed, canvases unmounted by React, stream subscriptions
dropped by unmounting widgets. Re-ON starts clean: a fresh scheduler and
fresh subscriptions; no stale RAF handle survives (unit-tested with fake
RAF: stop, then advance the clock — zero ticks).

## Tests

- `packages/ui/tests/larp-mode.test.tsx` (added to the `test:ui` list):
  toggle static markup for OFF/ON (label, `aria-pressed`, LED class,
  border class); hotkey predicate true/false matrix (including
  missing-modifier and wrong-key negatives); store toggle + subscriber
  notification; disposal registry runs all disposers on OFF and clears;
  scheduler fps selection (60 active / 24 after 5 s idle / 0 hidden)
  and zero ticks after stop; telemetry routing (push → subscriber
  receives, snapshot order, 500-event ring bound, drift clamp,
  negative-token floor); matrix cell mapping; velocity series math
  (including the sub-millisecond collapse); HUD static markup (frame
  regions, widget shells, view buttons 1–6, empty states).
- `tests/e2e/larp-mode-slice1.spec.ts`: via the shared session harness,
  click the toggle → HUD appears with all three widgets + view buttons;
  `Ctrl+Shift+L` → HUD unmounts completely (frame absent from DOM);
  toggle again → HUD returns (clean re-mount); canvases present while ON
  and absent while OFF. Click behavior is e2e-only (static markup
  cannot run effects).

## Verification commands

`./node_modules/.bin/tsc --noEmit`, `node scripts/contracts.mjs --check`,
`node scripts/validate-docs.mjs`, `npm run test:ui`,
`cd apps/web && node node_modules/vite/bin/vite.js build` (grep the
output CSS for `.larp-` to prove the stylesheet is bundled), then the
e2e spec plus the full regression battery (now 30 tests with this spec's
2). Rust suite unchanged (no Rust touched) — run once at the end to
confirm 616 + 4.
