# M8 visualization — focused Windows review — 2026-10-31

Incoming93cf77605f861cfd519b2aff7da0fdd158550830, baseline795aab005f4bd896d17fedaeeda3140f4ea03a69. ZIP SHA256dce73c1437e3dfb08a3225c9e904370c09c7ea48d9549d362fd9244fac8d77e9; bundle2166f03229bbb296374b5d1f481cd6b082fa61298d4d31226c4eb70d0202509f. Bundle verification/hash/ancestry pass. Backend/schema/contracts unchanged. Antigravity directories untouched.

Verdict: changes required before Windows/browser acceptance. No production changes by reviewer.

## VIS-01 (P1): case-insensitive module resolution breaks application build

Component packages/ui/src/ColliderVisualization.tsx and helper packages/ui/src/colliderVisualization.ts have base names differing only in case. Extensionless import ./ColliderVisualization resolves to the helper .ts on Windows before the component .tsx. Confirmed independently:
- TypeScript exits1 with TS2305 missing ColliderVisualization export and TS1261/TS1149 inconsistent casing.
- Vite build exits1: MISSING_EXPORT from ColliderVisualization.ts.
- UI test invocation reports43 pass,3 failed test-file loads (visualization, collider, components),46 reported total; it never reaches the expected105 individual cases. These are not3 evaluated behavior tests.

Rename helper/component to genuinely distinct base names (for example colliderVisualizationScene.ts and ColliderVisualization.tsx), update every import, and inspect Git tracked names and a case-insensitive checkout. Do not disable casing checks, alter module resolution settings, weaken tests or work around it solely by forcing an extension at one call site. Keep same public behavior and rerun Node/TypeScript/build/canonical browser workflow.

## VIS-02 (P2, source-level observation): canvas labels use the background color

ColliderVisualization.tsx drawScene fills the entire canvas with #0b2545, then uses the exact same #0b2545 for fillText labels drawn below each node. Those labels have no contrasting backing and are unreadable on the canvas. Use an appropriate contrasting label color/backing and verify actual browser pixels/screenshot at normal and high DPI, paused/reduced motion, rather than only testing selectable text in the alternative list. This observation follows directly from drawing commands; browser visual acceptance was not performed because VIS-01 prevents building the current app. Keep it distinguished from the reproduced build blocker.

## Requirement coverage: partial renderer, not completed section26

MASTER_SPEC section26 explicitly requires Three.js, toroidal particle streams, instanced rendering, GPU-efficient materials and adaptive quality plus the accessible2D alternative. This delivery implements Canvas2D and the accessible node/list workflow; it honestly records the Three.js renderer as deferred. Keep that boundary in requirement coverage and UI/status copy. Lack of an installed dependency/package-manager command is a setup dependency to resolve, not evidence that the mandated renderer is complete. Do not erase the Canvas/list work; preserve it as fallback. Renderer completion remains separate from this bounded repair.

## Evidence and limits

Fresh Windows TypeScript1, Vite1, UI43pass/3 failed file-loads. Docs validation201 files passes. Current browser workflow not run, so no stale previous web build is accepted as this tree. Rust runtime unchanged from prior verified URL checkpoint; no Rust rerun transferred. Logs: node-windows.log, typescript-windows.log, web-build-windows.log in delivery. No cloud/model/external fetch, owner data or preview changes, installs, pushes, CI or promotion. Owner release tooling remains separate.
