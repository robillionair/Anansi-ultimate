# LARP cinematic visual upgrade — Windows browser evidence

## Identity and scope

Owner asked for the cinematic feel of `1000062878.mp4` and supplied a LARP architectural proposal. Baseline `a3c517267`; design acceptance committed first at `10b68286c`; implementation `29c052683` on `codex/larp-cinematic-upgrade`. Shared React UI changes apply to web and the next desktop frontend build. This record verifies Windows Chromium behavior, not a newly compiled native desktop executable.

The video was decoded (HEVC, 1080x1440, 9.914 seconds) and inspected at 0, 2, 4, 5, 6, 7, 8 and 9 seconds. Main transferable features: dark observatory framing, large uninterrupted viewport, dense emissive point fields, warm gold/magenta contrast, layered orbit lines and fine instrumentation. Existing six semantic views and telemetry contracts were preserved.

## Delivered

- Full-screen observatory HUD, named six-view dock, keyboard switching and explicit Exit, responsive desktop/compact framing, honest telemetry summary and observed/estimated/unavailable distinctions.
- Memory vault: 50,000 seeded particles, depth-dependent bokeh, warm/cool color separation, animated shader detail, projected labels matching transformed positions, and 9,000 ornamental orbital particles. Empty-data clusters are explicitly labelled AMBIENT; decorative particles are never counted as user records.
- Collider: 32,000 layered orbital particles, seven thin braided orbit lines and two soft 2,000-particle beams. Actual collision events alone drive the discharge and event counter. Fixed the original oversized square sprites which erased structure with additive overexposure.
- Funnel: 28 ornamental sub-filaments for each of seven semantic paths, singularity halo, real compilation pulses, and a canvas-to-WebGL optical surface with direct 2D fallback.
- Shared WebGL pipeline: render, bloom, SMAA, radial chromatic offset 0.0012, simplex grain 0.035, output. Grain is multiplicative in linear color so dark areas remain dark after display conversion; no CSS fake-bloom substitute.
- Real scene renderers now own the frame scheduler. Active cap 60, idle target 24 after five seconds, suspended while hidden/unfocused, static reduced-motion redraw, resize redraw, subscriptions/observers/GPU resources disposed on exit. The previous standalone HUD clock performed no scene rendering; the previous scheduler also ran uncapped on high-refresh displays. Both are corrected.
- No inferred runtime attention weights, collision physics, provider prices, savings percentages or user records. Missing values stay unavailable. Existing analytical views remain event-driven and retain their empty states.

## Images inspected

[Before/after memory scene](larp-cinematic/before-after.jpg) shows the original overexposure and the separated particle structure after the upgrade. [Orbital collider](larp-cinematic/orbital-collider.jpg) is the new shared-UI render. These are real application screenshots from a fresh synthetic workspace, not mockups or model-generated pictures.

Full PNGs, reference storyboard, baseline captures, normal/reduced-motion and compact captures and command logs are retained at `E:\Anansi\reviews\larp-cinematic-2026-11-04`. The original UI was independently built from the baseline's archived package source for the before captures; the working implementation was not reverted or overwritten.

## Verification

- `npm run test:ui`: 295/295. Includes actual high-refresh clock cap, idle/hidden/stopped behavior, renderer blur/reduced-motion/disposal lifecycle and honest missing/estimated/negative-saving summaries.
- TypeScript, contracts drift, documentation validation and production web build passed. Existing Vite large-chunk warning remains (about 525 KB gzip); no new dependency or lockfile change.
- First relevant Windows browser battery: 12/13 passed. The one failure occurred during fixture setup, waiting for the synthetic Harbor App project before LARP was entered. It is recorded in `larp-browser.log`, not presented as a feature pass. The unchanged test subsequently passed in the 7/7 rerun.
- Final combined cinematic/toggle/heatmap/collider battery: 7/7. Covers six views, genuine loopback Ask/collider events, absence of data, cold unmount, reduced motion, no visualization-triggered provider requests, desktop and compact layout and no shader/page errors.
- Baseline visual capture: 1/1. Latest motion/funnel check on implementation `29c052683`: 4/4, including real compilation pulses and the WebGL-unavailable fallback, recorded in `final-motion-browser.log`.
- No Rust/domain/schema/contract implementation changed; no fresh full Rust regression or native desktop build claimed. The existing API binary supplied the synthetic browser backend. Real providers, paid inference and owner data were not used.

## Reproduction

From the repository root on Windows:

```powershell
npm run test:ui
node node_modules/typescript/bin/tsc --noEmit
node scripts/contracts.mjs --check
node scripts/validate-docs.mjs
```

Build web from `apps/web` with `node node_modules/vite/bin/vite.js build`; return to repository root before the leased browser command. With the existing API binary available at `target/debug/anansi-api.exe`:

```powershell
$env:ANANSI_LARP_EVIDENCE='E:\Anansi\reviews\larp-cinematic-reproduction'
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test tests/e2e/larp-cinematic.spec.ts tests/e2e/larp-mode-slice1.spec.ts tests/e2e/larp-mode-slice2.spec.ts tests/e2e/larp-mode-slice3.spec.ts tests/e2e/larp-mode-slice4.spec.ts --config playwright.config.ts
```

## Boundaries

This is an implemented cinematic visual pass using Three.js/WebGL, not Unreal Engine 5, photorealistic ray tracing or a claim of measured 60 FPS on every GPU. The proposed full 150,000-particle GPGPU simulation, new DAG/pipeline engines and real-time cost-ring telemetry are not implemented by this checkpoint. Existing six view meanings were preserved rather than fabricated to resemble unavailable measurements. Native desktop build/visual acceptance and promotion of an existing preview remain separate; no existing preview executable or user data root was replaced. Standing production/security/CI gates are unchanged.
