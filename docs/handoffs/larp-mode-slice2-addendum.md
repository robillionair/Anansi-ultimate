# LARP Mode Telemetry Engine — Slice 2 Addendum (Linux; Windows rerun pending)

Branch `muse/larp-mode-slice2` from Windows-verified LARP Slice 1 review tip
`f9f8bc245340a7994b898b014a345f255dead8fc` (review branch
`codex/review-larp-mode-slice1`, docs-only over delivery tip `b840cb437`).
Baseline note: the assignment's named tail MATCHED the bundle head exactly
— the first time in the series with no discrepancy. The review package
(`anansi-larp-mode-slice1-windows-verified-2026-11-03.zip`, SHA-256
`854c49a2…`) verified: ZIP hash equals the assignment's, bundle hash
equals its MANIFEST (`5aeef246…`), head is exactly the named tip with
parent `b840cb437`, diff is docs-only (BUILD_STATUS + review report),
verdict PASS, no findings to repair.

## Scope

Post-processing pipeline, centerpiece Views 1 and 2, view switching, and
live telemetry producers. No server op, no new artifact type, no schema
change, no Rust change, no contracts change. Views 3–6 stay placeholders;
Slice 3 owns them.

## Files

- `packages/ui/src/larp-mode/postprocessing/shaders.ts` — chromatic
  aberration shader (radial R/B shift, `uOffset = 0.0012`) and film grain
  shader (2D simplex noise, `uAmount = 0.05`). Pure objects, no GL.
- `packages/ui/src/larp-mode/postprocessing/LarpComposer.ts` — pipeline
  assembly: Render → UnrealBloom (strength 0.6, radius 0.4, threshold
  0.85) → aberration → grain → Output; DPR clamped to 2; disposal
  releases composer targets and every pass's resources, idempotent.
  Assembly goes through an injected factory so unit tests assert pass
  order without GL.
- `packages/ui/src/larp-mode/views/particleLayout.ts` — seeded
  (mulberry32) 50,000-particle layout clustered around centroids from
  the latest memory-event nodes (golden-angle spiral shell, radius 60);
  three fallback centroids when no nodes exist.
- `packages/ui/src/larp-mode/views/vaultShaders.ts` — optical bokeh
  shaders (focal-plane sharp white-hot cores, off-plane translucent
  discs), kept importable so tests assert on the sources.
- `packages/ui/src/larp-mode/views/View1MemoryVault.tsx` — one
  `THREE.Points` draw call through the composer, slow rotation (skipped
  when reduced motion), per-frame camera projection of cluster tags
  into CSS (imperative refs, no React state per frame), `data-frames`
  counter as the e2e-visible proof of rendering, explicit fallback
  panel on WebGL failure, full disposal (composer, geometry, material,
  renderer, forced context loss) registered as a store disposer.
- `packages/ui/src/larp-mode/views/bezier.ts` — cubic bezier math,
  funnel layout (4 amber/gold inputs → singularity → 3 cyan outputs to
  L1/L2/L3), pulse lifecycle (2.4 s traverse, expiry at t = 1).
- `packages/ui/src/larp-mode/views/View2NeuralFunnel.tsx` — high-DPI 2D
  canvas (the assignment allows 2D/WebGL; 2D is the honest choice for
  ultra-thin glowing filaments, since WebGL lines cannot exceed 1 px or
  glow without quad extrusion). Pulses spawn from real compilation
  events: live via subscription, plus a mount-time seed from compilation
  events in the snapshot (last 60 s, capped at 8) so mount order cannot
  lose a pulse. `data-pulses-fired` (cumulative, via ref) is the
  e2e-visible counter.
- `packages/ui/src/larp-mode/telemetryProducers.ts` — `reportAskRun`
  and `reportMemorySnapshot` (injectable stream for tests).
- `packages/ui/src/larp-mode/telemetry.ts` — `token-delta` gains
  optional `estimated?: boolean`, passed through sanitization.
- `packages/ui/src/larp-mode/TokenDeltaStream.tsx` — `est.` badge on
  estimated rows.
- `packages/ui/src/larp-mode/LarpHud.tsx` — bare `1`–`6` hot-switches
  views (editable targets and modifier keys ignored, so typing `1` in
  the Ask box never switches views); centerpiece renders View 1 / View 2
  / placeholder; `initialView` prop added so the placeholder stays
  statically testable.
- `packages/ui/src/AskAnansi.tsx` — one call site: on the completed
  branch of `handleSubmit`, `reportAskRun` with measured latency,
  wrapped in try/catch so telemetry can never break an Ask run.
- `packages/ui/src/Memory.tsx` — one call site: after `setMemories` in
  `refresh`, `reportMemorySnapshot`, same try/catch rule.
- `packages/ui/src/larp-mode/larp-mode.css` — view container, tag
  callouts with 0.5px leader lines, fallback panel. No border-radius on
  HUD panels, no box-shadows (unchanged rule).
- `packages/ui/tests/larp-views.test.tsx` (12 tests) +
  `packages/ui/tests/larp-mode.test.tsx` (placeholder test now renders
  `initialView={3}`) + `tests/e2e/larp-mode-slice1.spec.ts` (placeholder
  text updated to "arrives in a later slice.").
- `tests/e2e/larp-mode-slice2.spec.ts` (3 tests).

## Producer honesty, stated precisely

- The server manifest's `estimatedTokens` is the COMPILED prompt's
  estimate (`compiled_prompt.len() / 4` in `crates/core/src/ask.rs`).
  The raw input estimate uses the same `/4` estimator on the prompt the
  user actually typed. Output has no compilation step in Ask, so the
  answer estimate goes in both output fields. All Ask figures carry
  `estimated: true` and render with an `est.` badge.
- A run completing with `insufficientEvidence` still ran the context
  compiler, so it fires a `compilation` event labeled
  `ask-run-completed` — but no token-delta, because a compiled estimate
  of 0 would fake a −100% compression. Token-delta fires only when
  status is `answered` and `estimatedTokens > 0`.
- The GhostPrompt comparison dispatch (`askOnce`) is a separate feature
  and is NOT hooked in this slice; the assignment's "Ask ANANSI prompt
  compilation" is the main Ask flow.
- The e2e Ask leg asks an unrelated question so the run completes via
  the insufficient-evidence path with no provider call; the compilation
  event still fires, which is exactly what the funnel's pulses subscribe
  to. The pulse asserted in e2e is therefore real end-to-end producer
  evidence, not a stubbed event.

## Tests passed (Linux)

- tsc 0 errors; contracts `--check` exit 0; docs PASS 251 files (252
  with this addendum — verify at commit time).
- node 253/253 (241 + 12 new LARP view tests).
- Vite build clean; `.larp-view-container` confirmed in bundled CSS.
- LARP Slice 2 e2e 3/3 (View 1 frames, View 2 + keys, Ask → pulse,
  WebGL-failure fallback).
- Full regression battery: 33/33 (lifecycle slices 1–3 2/2 each, LARP
  Slice 1 2/2, LARP Slice 2 3/3, ghost 1–3 2/2 each, somnia 1–3 2/2 each,
  evaluations 1/1, integrated_workflow 1/1, opportunities 1/1,
  swarmtank-scenario 3/3, swarmtank-arena 3/3, swarmtank-slice1 1/1),
  run sequentially on an idle machine after the Rust suite — no
  concurrent-load flake this round. (The Slice 1 spec's first test now
  takes ~27 s because the default centerpiece view is the WebGL vault,
  not the old placeholder; it still passes.)
- Rust workspace: 616 passed + 4 known live-Ollama env failures
  (`semantic_workflow`, identical to the M12/LARP Slice 1 baselines; no
  Rust files touched), run with `--no-fail-fast` to a file and summed
  from the `test result` lines plus the failures sections directly.
- Credentials sweep over the diff: clean (no matches).

## In-slice fixes, disclosed

- The Slice 1 HUD unit test rendered the default view, which is now
  View 1 (a live view, not the placeholder), so its placeholder
  assertion failed. Fixed by adding an `initialView` prop to `LarpHud`
  (default 1) and rendering the test with `initialView={3}` — the
  placeholder assertion stays meaningful instead of being deleted.
- The first tsc run failed with null-narrowing errors inside the view
  effects' nested closures (TS resets narrowing there). Fixed by
  wrapping the guarded elements in a single non-null `els` object after
  the guard, in both views — no `!` assertions.
- The evidence tsc run caught a type error the test runs had missed:
  the WebGL-failure leg's `addInitScript` callback was typed with an
  `unknown` return, which is not assignable to `getContext`'s overloads.
  Fixed by passing the stub as a plain-JS string (which is also the
  established `addInitScript` rule: no TS annotations inside). Runtime
  behavior is identical; the Slice 2 spec was re-run after the fix.

## Known limitations

- View 1's cluster layout snapshots the memory nodes at mount; nodes
  arriving later do not re-cluster until the view remounts.
- View 2's pulses travel input filaments only and are absorbed at the
  singularity; they do not continue onto the output filaments.
- First-frame time for View 1 under software GL can reach seconds; the
  e2e frame assertion uses a 60 s timeout for that reason.
- The HUD scheduler's ticks remain no-ops from Slice 1; each view owns
  its own RAF loop so disposal stays per-view.

## Security

No new network, filesystem, or provider surface. The view switcher's
keydown listener ignores editable targets and modifier keys. Producers
push only in-memory UI state (prompt/answer lengths as estimates,
memory titles/ids already visible in the UI); nothing is transmitted.
The Ask hook is try/catch-guarded so a producer failure cannot break an
Ask run. Credentials sweep over the diff: clean (no matches).

Last verified commit: tip recorded in the delivery MANIFEST.

Next: Windows rerun of this slice; Slice 3 (Views 3–6) not commissioned.
