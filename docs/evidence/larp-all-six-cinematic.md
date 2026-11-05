# All six LARP views — cinematic continuation

## Checkpoint

Owner authorized giving every LARP view the cinematic treatment while preserving functionality. Continuation baseline `77dfc1d5f`; acceptance extension committed before code at `3060b0f9b`; implementation `db2ae67a569f066eeb8c6c89688e846afd1205dd` on `codex/larp-cinematic-upgrade`. This extends, rather than replaces, the [first visual upgrade evidence](larp-cinematic-upgrade.md).

[All six populated views](larp-cinematic/all-six-views.jpg) are cropped from real Windows browser screenshots, not mockups. [Memory topology](larp-cinematic/memory-topology.jpg) retains its actual record labels and relationships. The fixtures contain synthetic records; these images are visual/function evidence, not model-quality or token-saving benchmarks.

## Delivered behavior

1. Memory constellation, context funnel and orbital collider retain the prior cinematic particle, bokeh, filament and optical work.
2. Graph: up to 22,000 soft decorative cloud particles around the actual graph layout, 8,000 ornamental orbital particles, luminous node cores and curved relationship/layout links. Semantic node/edge counts remain unchanged; a legend distinguishes recorded relationships, layout links and dust. Label projection still uses the transformed graph matrix. Memory/citation snapshot identity keys now refresh mounted graph/vault scenes when real telemetry changes.
3. Signal observatory: calibrated chart framing, perspective stage, fine grid, actual compilation markers, compression traces and confidence bars through the shared GPU optics. No drift sample means explicitly unreported drift, never a made-up series. Existing event-order semantics and compression bounds remain unchanged.
4. Word alignment: luminous exact-match cells, crisp row and sampled column labels, measured matrix counts and layout shared by drawing/hit-testing. Words remain words, not provider token IDs or attention weights. Hover maps to the correct source cell after resizing and ignores points outside the grid.
5. Event-driven instruments have shared film surfaces with text-safe exposure, direct Canvas 2D fallback, resize/visibility/focus redraw, and complete surface/observer/listener cleanup on OFF. They have no continuous animation clock. Empty states keep their truthful messages with restrained ornamental framing.

No backend, schema, contract implementation, provider policy, model configuration default or dependency/lockfile change. Existing preview executables/data roots were not replaced.

## Windows verification

- UI/contracts suite: **296/296**. The new layout/hit-test regression covers first/middle/last cells, boundaries and three viewport geometries.
- TypeScript, contracts drift, production web build and Git diff validation: PASS. Existing Vite large-chunk warning remains (about 527 KB gzip).
- Relevant six-view browser battery: **14/14** in 5.8 minutes, including real record graph/refresh, Ask compilation signals, collider discharge, hotkeys/buttons, reduced motion, empty/unavailable states and cold unmount. Log: `all-six-final-browser.log`.
- Final strengthened positive/fallback battery: **5/5** in 2.6 minutes, after text-exposure/focus cleanup refinements. The new test explicitly configures its own loopback synthetic runtime, requires **three Answered outcomes and exactly three runtime hits**, then verifies visualization interaction adds no calls. Actual compilation/token events populate the charts/matrix. It also checks graph refresh from 8 to 9 records and exact `raw "what"` cell identity at two viewport sizes.
- Forced WebGL-unavailable test: both chart and populated matrix retain working direct 2D presentation and cell inspection in reduced motion; offline compiled evidence remains available and no provider-catalog fake is hit. This is included in the 5/5 final battery.
- Inspected screenshots of all six populated views plus 2D fallback. Full images/logs: `E:\Anansi\reviews\larp-cinematic-2026-11-04\all-six` and `all-six-positive-final.log` in the parent directory. Shared application-test lease was released.

The test executions above overlap; they are not 19 distinct workflows. The first new live-refresh probe initially failed because re-clicking the already-selected Memory navigation does not request fresh data. It was corrected to use the actual **Refresh memories** operation; the 9-node assertion was retained. Initial chart screenshots exercised compilation/offline behavior but had no completed-model delta events. The final test strengthens that gap with the owned synthetic runtime, Answered-status and exact-hit assertions; those are plumbing/visual evidence, not real-model performance.

## Reproduction

Build web from `apps/web`, then return to repository root:

```powershell
npm run test:ui
node node_modules/typescript/bin/tsc --noEmit
node scripts/contracts.mjs --check
node scripts/validate-docs.mjs
$env:ANANSI_LARP_EVIDENCE='E:\Anansi\reviews\larp-all-six-reproduction'
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test tests/e2e/larp-all-six-cinematic.spec.ts tests/e2e/larp-cinematic.spec.ts tests/e2e/larp-mode-slice1.spec.ts tests/e2e/larp-mode-slice2.spec.ts tests/e2e/larp-mode-slice3.spec.ts tests/e2e/larp-mode-slice4.spec.ts --config playwright.config.ts
```

Use an available existing API binary at `target/debug/anansi-api.exe`. No full Rust rerun claimed for this shared-UI-only change. These are Windows Chromium checks; native Tauri desktop rebuild/visual acceptance and preview promotion remain separate. Target rendering cadence is preserved, not a universal GPU-throughput guarantee. All production/security/CI gates remain unchanged. No perfection or full Unreal Engine equivalence claim is made.
