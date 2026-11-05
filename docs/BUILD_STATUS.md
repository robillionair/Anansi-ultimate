## 2026-11-05 — MCP Integration Slice 4 verified on Windows

Incoming delivery `anansi-mcp-slice4-2026-11-04.zip` (delivery ZIP SHA-256 `4e728c4903deee75a9b916c1d192ec49ca53c568f80dfb93813a98a6a81a17bc`, bundle SHA-256 `18895d7ebcb1bed266032c5a4ffcdad7f03f221a8e9fd14d267945c88dfe3080`, tip `25bd957833f7fd8d8bb48f641b6ef7823f68e884` over baseline `1b55820973a8ad041933ba9cb468e21e7845f4fc`) verified on Windows. Scope: MASTER_SPEC §68 MCP client resources & prompts + extended server tools — client inspection and reading of MCP server resources (`resources/list`, `resources/read`) and prompts (`prompts/list`, `prompts/get`) over stdio only with review-only UI components (`ResourceViewer`, `PromptViewer`) and explicit click opt-in (never executed or injected into Ask ANANSI); extended server tools list (6 tools: `memory.search`, `memory.get`, `notes.search`, `projects.search`, `experiments.list`, `products.get`) with strict fail-closed privacy boundary (sealed and private entities dropped server-side; identical `Project not found.` on missing/inactive/private/expired projects). Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 278 files PASS (`validate-docs.mjs`). UI suite `npm run test:ui` 297/297 PASS in 11.36s (including 4 new tests). Production web build clean in 5.43s; API build clean in 2m 36s. Backend Rust tests: `mcp::tests` 16/16, `mcp_server::tests` 14/14, workspace `--lib` 244 passed, 0 failed across all crates. Playwright battery under shared lease: `mcp-slice4.spec.ts` 3/3 PASS in 30.4s, `mcp-server.spec.ts` 2/2 PASS, `mcp.spec.ts` 6/6 PASS; full regression battery 57/57 PASS across all 23 milestone specs in 10.5m. [Evidence](reviews/windows-mcp-slice4-verified.md). MCP Integration Slice 4 is verified and closed on Windows. AN-017 stays in progress. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-04 — MCP Server Core Slice 3 verified on Windows

Incoming revised delivery `anansi-mcp-slice3-2026-11-04.zip` (delivery ZIP SHA-256 `a4dfb770daa91d716634b62666ec4ecf854063df0df6606e1a8e86a9aa55e3cf`, bundle SHA-256 `c946199e8739a767b981accf75290fffe9bcd3711a2efc8ab4d8d31eb046b869`, tip `1e82892bee7f1cfdcc71ffd4edac158db5ed46c0` over baseline `a3c517267d58761a63e9d50795a406f0d02b5810`) verified on Windows. Scope: MASTER_SPEC §68 MCP server core — `crates/core/src/mcp_server.rs` (stdio JSON-RPC 2.0 loop, `serverInfo.name: "anansi-server"`, multi-protocol negotiation for 2024-11-05 and 2025-03-26/2025-06-18) plus `--mcp-server` / `--pairing-root` mode on `anansi-api.exe`. Exposes three knowledge tools: `memory.search` (with validated kind filter), `memory.get`, and `projects.search` (with stage metadata via Rust `derive_pipeline`, sealed artifacts excluded). Strict fail-closed privacy boundary: sealed AND private memories never leave the process; only standard/public records are served; get on missing/sealed/private is identical. Pairing-token authentication replaces bespoke env tokens, validated against `<pairing-root>/session-*/pairing.json` on every initialize. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 276 files PASS (`validate-docs.mjs`). UI suite `npm run test:ui` 293/293 PASS. Production web build clean in 9.13s; API build clean in 2m 43s. Backend Rust tests: `mcp_server::tests` 12/12, workspace `--lib` 239 passed, 0 failed across all crates. Playwright battery under shared lease: `mcp-server.spec.ts` 2/2 PASS in 28.6s, `mcp.spec.ts` 6/6 PASS in 47.6s; full regression battery 54/54 PASS across all 22 milestone specs. [Evidence](reviews/windows-mcp-slice3-verified.md). MCP Integration Slice 3 is verified and closed on Windows. AN-017 stays in progress. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-04 — MCP Client Integration Slice 2 verified on Windows

Incoming delivery `anansi-mcp-slice2-2026-11-04.zip` (delivery ZIP SHA-256 `16efd992d317752ff72f4e78919e1f223c9860abaf496dd02765e2898adaf539`, bundle SHA-256 `edc4669266edfc3bc4cceb01c1bfbb7a3fd1ee7a0cb7fe065c64b6ca1f774d91`, tip `22cef49672bb0b9bd81580534b73103b35f552d1` over baseline `84dc00b955799988e0b6dbe45e8557bfa3e87fb4`) verified on Windows. Scope: MASTER_SPEC §68 explicitly approved local tool invocation — new `/v1/mcp/tools-call` operation reusing the Slice 1 stdio channel (Windows boot environment, 30s budget, child teardown) and the factored-out dual-protocol handshake, fresh `tools/list` with server-offered schema validation (conservative JSON Schema subset, mirrored in TS), structured `McpToolCallResult` decoding (typed content parts, joined text, `isError`, structured content passthrough); UI call form per schema field, mandatory authorization dialog (server, tool, badges, exact JSON parameters, destructive warning; only "Authorize and Call Tool" dispatches; Cancel dispatches and records nothing), result below the tool card, and a session-local history (timestamp, server, tool, status only — never parameters, results, env values, or keys; not persisted). SSE invocation refused as in Slice 1; no resources, no MCP server side, no polling, no auto-invocation, no paid inference. Design committed before code. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 272 files PASS (`validate-docs.mjs`). UI suite `npm run test:ui` 293/293 PASS in 9.67s (including 4 new tests). Production web build clean in 4.46s; API build clean. Backend Rust tests: `mcp::tests` 13/13, workspace `--lib` 227 passed, 0 failed. Playwright battery under shared lease: `mcp.spec.ts` 6/6 PASS in 34.5s; full regression battery 52/52 PASS across all delivered milestones. [Evidence](reviews/windows-mcp-slice2-verified.md). MCP Client Integration Slice 2 is verified and closed on Windows. AN-017 stays in progress. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-04 — MCP Client Integration Slice 1 repair verified on Windows

Incoming repair delivery `anansi-mcp-slice1-repair-2026-11-04.zip` (delivery ZIP SHA-256 `cf4c1dcadf5ff8f07bd2bf70bd7610ec078f2fe22b9aedd82547036e5640f487`, bundle SHA-256 `4321d2055398759f24e993c4535a21e8d6a6fa05fa00c681fddbc7f154c6a0e3`, tip `6ddef0995cac6342f633eed49c1e635fb4ddc40d` over baseline `4b0b42e0077802490ee468b2c7304e28fd0f9de7`) verified on Windows. Scope: `crates/core/src/mcp.rs` Windows boot environment restoration in `StdioChannel::spawn` (restoring `SystemRoot`, `WINDIR`, and `PATH` case-insensitively when not overridden by server config) and accurate disconnect error reporting (`"Server exited or closed connection before replying."`). Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 269 files PASS (`validate-docs.mjs`). UI component suite `npm run test:ui` 289/289 PASS in 7.68s. Production web build clean in 3.21s; API build clean. Backend Rust tests: `mcp::tests` 8/8, workspace `--lib` 223 passed, 0 failed. Windows Playwright battery under shared lease: `mcp.spec.ts` 4/4 PASS in 33.7s (decisive test 2 stdio probe passed in 7.8s against genuine Node MCP server); full regression battery across M12 Slices 1–3, LARP 1–4, GHOST-PROMPT 1–3, SOMNIA 1–3, SwarmTank 1–3, Evaluations, Opportunities, Integrated Workflow, Dashboard, and MCP 50/50 PASS in 10.1m with zero provider traffic. [Evidence](reviews/windows-mcp-slice1-repair-verified.md). MCP Client Integration Slice 1 is verified and closed on Windows. AN-017 is in progress. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-04 — MCP Client Integration Slice 1 delivered (Linux; Windows rerun pending)

Delivery `anansi-mcp-slice1-2026-11-04.zip` (tip recorded in the delivery MANIFEST) over Windows-certified Dashboard Slice 2 review tip `34f39bb7da54e70c3fc25ed9dbf0791cf4bab197`. Baseline verified from origin by hash (parent exactly the Slice 2 delivery tip `330ed64e8`, docs-only diff); only the review ZIP's `.sha256` sidecar was uploaded (expected hash `df3cb507…`), so the review ZIP itself was not re-hashed — the content-addressed commit on origin is what this slice branches from. Scope: §68 MCP client side only — configuration model + registry (`packages/ui/src/Mcp.tsx`, `mcp.ts`; per-workspace `storage/mcp-servers/` files, hardened like provider-config; env values never returned, `envKeys` only), live one-shot stdio probing (`crates/core/src/mcp.rs`, new `/v1/mcp/` API group, 13 new contract DTOs) with dual-protocol negotiation (2025-11-25 `initialize` first, 2026-07-28 `server/discover` fallback on method-not-found; health method reported, never fabricated), tool discovery + capability listing (annotations, input schemas, prompts), and MCP promoted from planned tools to a live SYSTEM view (nav buttons 16 → 17). SSE servers are registrable but their probes are EXPLICITLY REFUSED (no POST HTTP client in-tree; §68 default-deny + §71 SSRF). No `tools/call`, no resources, no MCP server side, no persistent connections, no polling, no paid inference. Safety: no shell, child environment cleared and rebuilt from configured vars only, 30s probe budget, child killed on timeout/error, lists capped at 100 with a truncation flag. In-slice fixes disclosed: the TS `httpClient` path map routed unknown groups to `workspaces/` — `mcp/` was missing from the direct-path list (browser calls 404'd); fixed, plus a rebuild. The client `decode` requires `contractVersion` on every response, so `McpServer` and `McpRemoveResult` gained it. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 267 files PASS (`validate-docs.mjs`). UI component suite `npm run test:ui` 289/289 PASS (including all 5 new tests in `mcp.test.tsx`). Production web build clean; API build clean. Backend Rust tests (`--lib`): 220 passed, 0 failed (215 + 5 new MCP unit tests). Playwright browser battery under shared lease: `mcp.spec.ts` 4/4 PASS (including a probe against a real local stdio server process); full regression battery 50/50 PASS with zero provider traffic. AN-017 moves to `in progress` (Slice 1 covers registry + listing only). Windows rerun of MCP Slice 1 pending. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-04 — HOME Command Dashboard Slice 2 verified on Windows (Dashboard complete)

Incoming delivery `anansi-dashboard-slice2-2026-11-04.zip` (delivery ZIP SHA-256 `33e2a2c94963e6b6e3e4544ce03da1760005c22dbfd6ae0cf18afd5067729cea`, bundle SHA-256 `2e3a50149d60f48a4c1accea48b66ed53683df7cf56d89dd1bf6e6ec3acad2b3`, tip `330ed64e87b2b83aad4b57a007c8cb6bc18791b8` over baseline `c5de4fad9334b06f88b6114f69e6c19c32dcd34e`) verified on Windows. Scope: MASTER_SPEC §8, §50, §66 dashboard system & queue health — System & Storage Health section in `Dashboard.tsx` (backup status from genuine `readBackupSchedule`: scheduled frequency or `Manual backups only`, with a Backups action; background queue status from genuine paged `listJobs`: active count + latest job type, or calm `Background queue idle`, with a Jobs action), projects hub recency ordering (most recent artifact timestamp or project update) + 5-project cap with a "View all in Projects" link beyond 5, and `Dashboard` removed from `plannedGroups[0].items` in `Navigation.tsx` (it is live now). Contract correction confirmed: `JobState` values are capitalized, so active = `'Queued'` or `'Running'`; the "latest job type" label reuses `Jobs.tsx`'s exact selection mapping. No server op, no new schema, no paid inference, zero Rust files touched. NTFS qualification preserved: all new and touched imports explicitly use `.ts`, `.tsx`, and `.js` extensions. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 265 files PASS (`validate-docs.mjs`). UI component suite `npm run test:ui` 284/284 PASS in 6.65s (including all 7 new tests in `dashboard.test.tsx`). Production web build clean in 1.04s. Backend Rust tests (`--lib` workspace): 215 passed, 0 failed across all crates. Windows Playwright browser battery under shared lease: `dashboard.spec.ts` 6/6 PASS in 37.1s; full regression battery across M12 Slices 1–3, LARP 1–4, GHOST-PROMPT 1–3, SOMNIA 1–3, SwarmTank 1–3, Evaluations, Opportunities, Integrated Workflow, and Dashboard 46/46 PASS in 9.1m with zero provider traffic. [Evidence](reviews/windows-dashboard-slice2-verified.md). HOME Command Dashboard is 100% complete and verified on Windows. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-04 — HOME Command Dashboard Slice 1 verified on Windows

Incoming delivery `anansi-dashboard-slice1-2026-11-04.zip` (delivery ZIP SHA-256 `d590735b1d330a1b5911b1bf45f32c8e355fc6f1303be181bde5643aa5f03df4`, bundle SHA-256 `dfdc34a0b80e536d54d1979ea14518f395d4cd27ec7d6d6287e03317b6def455`, tip `f57889edbc106c8cc6ebdac09afb491c45f2986e` over baseline `ac2f05aba7647c48646ef2ae013a3be5a1084f72`) verified on Windows. Scope: MASTER_SPEC §8 HOME Command Dashboard — `Dashboard.tsx` (workspace overview with name, ID, readiness uptime badge from genuine `Readiness.uptimeMs`, telemetry counts; projects hub with title, snippet excerpt, `derivePipeline` stage badge, "Continue Pipeline"; exactly four quick actions: + New Project, Ask ANANSI, LEXI-COLLIDER, LARP Telemetry Engine), Navigation HOME reduced to exactly Dashboard + Ask ANANSI, Dashboard as the default active view upon connection (transition-guarded landing; onboarding finishes on the Dashboard), and additive `focusProjectId`/`focusSection` props on Projects for direct jumps to a project's Guided Walk or collider section. The `'workspace'` view is retained for the pre-connection flow and workspace management (reachable from the Dashboard's "Manage workspace" action). No server op, no new artifact type, no schema change, zero Rust files touched. NTFS qualification preserved: all new and touched imports explicitly use `.ts`, `.tsx`, and `.js` extensions. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 262 files PASS (`validate-docs.mjs`). UI component suite `npm run test:ui` 277/277 PASS in 9.0s (including all 7 new tests in `dashboard.test.tsx`). Production web build clean in 4.01s (`.dashboard-stats` verified in CSS). Backend Rust tests on Windows: zero Rust files modified; full workspace unit tests 215 passed, 0 failed across all crates. Windows Playwright browser battery under shared lease: `dashboard.spec.ts` 4/4 PASS in 28.7s; full regression battery across M12 Slices 1–3, LARP 1–4, GHOST-PROMPT 1–3, SOMNIA 1–3, SwarmTank 1–3, Evaluations, Opportunities, Integrated Workflow, and Dashboard 44/44 PASS in 9.0m with zero provider traffic. [Evidence](reviews/windows-dashboard-slice1-verified.md). HOME Command Dashboard Slice 1 is verified and closed on Windows. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — LARP Mode Telemetry Engine Slice 4 verified on Windows (LARP Mode complete)

Incoming delivery `anansi-larp-mode-slice4-2026-11-03.zip` (delivery ZIP SHA-256 `319fb829d4a5b3a6a3413cddbeef849b8ece5b97ef9607d71107e0ea9fdc9166`, bundle SHA-256 `f0015f2cedfd05705365b592b2a48efe81bbc7321aef3cab2cacd97aaf3cc51d`, tip `8b4e37d1f56dab412b71e8a2521fddb1bdc5991f` over baseline `6f3ce75231f28a6507a4da388f0af5e22b14368e`) verified on Windows. Scope: View 5 `ATTENTION TENSOR HEATMAP // TOKEN ATTENTION MATRIX` (one genuine word-presence alignment matrix over the real raw and compiled prompts from the latest answered Ask run, capped 32×64, with explicit "no attention heads measured" and "words, not provider tokens" legend lines; insufficient-evidence runs push no token event, so the empty state persists honestly) and View 6 `COLLIDER FIELD // CONCEPT RESONANCE ACCELERATOR` (two 2,000-particle beams through the shared `LarpComposer` pipeline converging at the origin with Doppler coloring and a bloom discharge flash on real collider generation events; beams labeled as stylized). HUD wiring for views 5–6; all six views now live, no placeholders remain. No server op, no new artifact type, no schema change, zero Rust files touched. NTFS qualification preserved: all new and touched imports explicitly use `.ts`, `.tsx`, and `.js` extensions. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 258 files PASS (`validate-docs.mjs`). UI component suite `npm run test:ui` 270/270 PASS in 5.25s (including all 8 new tests in `larp-views4.test.tsx`). Production web build clean in 864ms. Backend Rust tests on Windows: zero Rust files modified; full workspace unit tests 215 passed, 0 failed across all crates. Windows Playwright browser battery under shared lease: `larp-mode-slice4.spec.ts` 4/4 PASS in 43.9s; `larp-mode-slice1..3.spec.ts` 8/8 PASS in 1.2m; full regression battery across M12 Slices 1–3, LARP 1–4, GHOST-PROMPT 1–3, SOMNIA 1–3, SwarmTank 1–3, Evaluations, Opportunities, and Integrated Workflow 40/40 PASS in 8.6m with zero provider traffic (total 44/44 browser tests PASS). [Evidence](reviews/windows-larp-mode-slice4-verified.md). No MATRIX row exists for LARP (new feature track). LARP Mode Telemetry Engine is 100% complete on Windows. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — LARP Mode Telemetry Engine Slice 3 verified on Windows

Incoming delivery `anansi-larp-mode-slice3-2026-11-03.zip` (delivery ZIP SHA-256 `bebf74d5f603083bccab8e0eeb34be6c716644f004115ec0317b606cdc6a8b42`, bundle SHA-256 `8de32cb631c21e5baa0c59a8575aa957cfb30037d63ae58f9b11e64c0063f078`, tip `932b357bbca85327d3cc4e704190c5d503efaba1` over baseline `c8f61c8441e0c23621535fb7539e09f35b2fd285`) verified on Windows. Scope: LARP Mode Slice 3 — View 3 `ATTENTION SPIDER // GRAPH DRIFT ENGINE` (WebGL 3D force-directed graph through the shared `LarpComposer` pipeline: nodes from latest memory snapshot capped at 64, cited nodes rendered white-hot via compilation-event `citedIds`, recorded relationship edges bright, layout links dim, honest refusal on fabricated attention weights), View 4 `QUANT DRIFT & COLLIDER SPECTRUM` (high-DPI 2D canvas over real bus series: drift, compression delta percent, compilation markers, confidence strip; collision resonance explicitly disclaimed as unmeasured; window-level hover readout preserving HUD click-through), additive telemetry (`MemoryNode` relationships/confidence, compilation `citedIds`), Slice 1 placeholder assertion cleanly shifted to View 5. No server op, no new artifact type, no schema change, zero Rust files touched. NTFS qualification preserved: all new and touched imports explicitly use `.ts`, `.tsx`, and `.js` extensions. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 255 files PASS (`validate-docs.mjs`). UI component suite `npm run test:ui` 262/262 PASS in 9.48s (including all 9 new tests in `larp-views3.test.tsx`). Production web build clean in 3.22s (`.larp-spectrum-readout` verified in CSS). Backend Rust tests on Windows: zero Rust files modified; full workspace unit tests 215 passed, 0 failed across all crates. Windows Playwright browser battery under shared lease: `larp-mode-slice3.spec.ts` 3/3 PASS in 39.7s; `larp-mode-slice1.spec.ts` + `larp-mode-slice2.spec.ts` 5/5 PASS in 47.5s; full regression battery across M12 Slices 1–3, GHOST-PROMPT 1–3, SOMNIA 1–3, SwarmTank 1–3, Evaluations, Opportunities, and Integrated Workflow 36/36 specs PASS in 8.4m with zero provider traffic (total 38/38 browser tests PASS). [Evidence](reviews/windows-larp-mode-slice3-verified.md). No MATRIX row exists for LARP (new feature track). Slice 4 (Views 5–6) not commissioned. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — LARP Mode Telemetry Engine Slice 2 verified on Windows

Incoming delivery `anansi-larp-mode-slice2-2026-11-03.zip` (delivery ZIP SHA-256 `8ed7cb592ae54abe82632672fb8ed34955fecf39ea240271f8df40a4f3b46bbe`, bundle SHA-256 `cde4927ad051228a8e045f4c18034cf4ddc830194f73ec798352c7d7071b1f67`, tip `43ff2cdbb8bae1d666fa15ae145fd9f848a484e7` over baseline `f9f8bc245340a7994b898b014a345f255dead8fc`) verified on Windows. Scope: LARP Mode Slice 2 — multi-pass WebGL post-processing pipeline (`EffectComposer`: `RenderPass` → `UnrealBloomPass` 0.6/0.4/0.85 → radial chromatic aberration 0.0012 → simplex film grain 0.05 → `OutputPass`; DPR clamped to 2; idempotent disposal), View 1 `SECOND BRAIN // 3D MEMORY VAULT` (50,000 seeded particles in one `THREE.Points` draw call clustered around memory centroids, optical bokeh shader with focal planes, per-frame camera-projected cluster HTML tags, `data-frames` rendering proof counter, WebGL-failure fallback panel), View 2 `NEURAL FUNNEL // CONTEXT COMPILER` (high-DPI 2D canvas, 4 amber/gold inputs → singularity → 3 cyan L1/L2/L3 outputs, photon pulses from genuine compilation events with mount-time seeding), bare `1`–`6` hot-switch (with editable input and modifier key guards), live producers from the main Ask flow (`reportAskRun`, prompt length `/ 4` token estimates) and Memory loads (`reportMemorySnapshot`). No server op, no new artifact type, no schema change, zero Rust files touched. NTFS qualification preserved: all new and touched imports explicitly use `.ts`, `.tsx`, and `.js` extensions. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 253 files PASS (`validate-docs.mjs`). UI component suite `npm run test:ui` 253/253 PASS in 7.0s (including all 12 new tests in `larp-views.test.tsx`). Production web build clean in 2.94s (`.larp-view-container` verified in CSS). Backend Rust tests on Windows: zero Rust files modified; full workspace unit tests 215 passed, 0 failed across all crates. Windows Playwright browser battery under shared lease: `larp-mode-slice2.spec.ts` 3/3 PASS in 28.5s; `larp-mode-slice1.spec.ts` 2/2 PASS in 24.3s; full regression battery across M12 Slices 1–3, GHOST-PROMPT 1–3, SOMNIA 1–3, SwarmTank 1–3, Evaluations, Opportunities, and Integrated Workflow 33/33 specs PASS in 7.6m with zero provider traffic (total 35/35 browser tests PASS). [Evidence](reviews/windows-larp-mode-slice2-verified.md). No MATRIX row exists for LARP (new feature track). Slice 3 (Views 3–6) not commissioned. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — LARP Mode Telemetry Engine Slice 1 verified on Windows

Incoming delivery `anansi-larp-mode-slice1-2026-11-03.zip` (delivery ZIP SHA-256 `92fe23d7c9c8713ae383a44f7ccedfa3c953dfd4f9ae8db160f8428772d290a6`, bundle SHA-256 `d0999c1891ed889ff8d05c50c91a7ad2c0b83b656e3aca12fe34a1d1095b9359`, tip `b840cb43741770c2bdc13df0a66c4c45e2186c73` over baseline `640f7c240`) verified on Windows. Scope: LARP Mode Slice 1 — RGB Chrome toggle, global store with cold-unmount lifecycle, in-memory telemetry bus, perimeter HUD frame. Implementation: `packages/ui/src/larp-mode/` — `store.ts` (`useLarpModeStore`, disposal registry), `frameScheduler.ts` (60 FPS active / 24 FPS after 5 s idle / 0 FPS hidden), `telemetry.ts` (`AnansiTelemetryStream`, 500-event ring, sanitizing push), `LarpToggle.tsx` (chrome pill, conic RGB border, `Ctrl+Shift+L`), `LarpHud.tsx` (fixed `100vw x 100vh` grid frame, pointer-events none except view buttons), `MemoryMatrix.tsx` (48×16 grid of 4 px cells), `TokenDeltaStream.tsx`, `CoherenceWaveform.tsx`, `larp-mode.css` (`#030305` void, `0.5px #16161a` hairlines, mono stack, `tabular-nums`, no box-shadows); toggle mounted in the Workspace header next to the connection badge, HUD mounted in the shell. No server op, no new artifact type, no schema change, no Rust change. NTFS qualification preserved: all new and touched imports explicitly use `.ts` and `.tsx` extensions. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 249 files PASS (`validate-docs.mjs`). UI component suite `npm run test:ui` 241/241 PASS in 5.01s (including all 13 new LARP tests). Production web build clean (4.37s). Backend Rust tests on Windows: zero Rust files modified; `cargo fmt` clean; full workspace 616 passed + 4 live-Ollama skips; `anansi-api` clean. Windows Playwright browser battery under shared lease: `larp-mode-slice1.spec.ts` 2/2 PASS in 17.6s; full regression battery across M12 Slices 1–3, GHOST-PROMPT 1–3, SOMNIA 1–3, SwarmTank 1–3, Evaluations, Opportunities, and Integrated Workflow 28/28 specs PASS in 7.3m with zero provider traffic (total 30/30 browser tests PASS). [Evidence](reviews/windows-larp-mode-slice1-verified.md). No MATRIX row exists for LARP Slice 1 (new feature track). Slice 2 (centerpiece views 1–6, live telemetry producers) not commissioned. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — M12 Product Lifecycle Slice 3 verified on Windows (Milestone 12 complete)

Incoming delivery `anansi-m12-product-lifecycle-slice3-2026-11-03.zip` (delivery ZIP SHA-256 `0ecf42251fe97ea03999e86869ab121003cf1dfbd6dc8cd9aca3fb456fab8cd7`, bundle SHA-256 `d9617cb0854caef24a65b749fd4d99d8fc67c97adf7ef09f7c9ad29762079acd`, tip `668565d1bc08627e0de1fc2c88f290a45070934f` over baseline `30f1ee91d`) verified on Windows. Scope: M12 Product Lifecycle Slice 3 (MASTER_SPEC §62, §85, §88). Implementation: Guided Walk panel (`GuidedWalk.tsx` + `guidedWalk.ts`) over the Slice 1 pipeline engine — six stages (Discover, Define, Simulate, Pressure-Test, Refine, Launch), Define spanning Invent + Build (completing only when both concept and specification artifacts are present), Refine never Complete by design (§53), Continue targeting the first non-Complete completable step; complete §85 walk executed end to end in the browser (opportunity set → collider concept → Forge specification draft → mid-walk client restart → 20-persona SwarmTank run → SOMNIA nightmare evaluation with sealed-case exclusion → GHOST-PROMPT session revised and wiped → Launch Studio four-channel copy → bundle download → manual `launch-material` registration); state recovery verified by restart and by full backup/restore with project-binding checks (identical walk statuses and artifact lists; fresh project in the restored workspace starts empty); shared-harness fake routed by server-side instruction strings (opportunity, spec, collider). No server op, no new artifact type, no schema change, zero provider traffic. NTFS qualification preserved: all new and touched imports explicitly use `.ts` and `.tsx` extensions. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 246 files PASS (`validate-docs.mjs`). UI component suite `pnpm test:ui` 228/228 PASS in 28.7s (including all 8 new guided-walk tests). Production web build clean (3.82s). Backend Rust tests on Windows: zero Rust files modified; `cargo fmt` clean; full workspace 616 passed + 4 live-Ollama skips; `anansi-api` clean. Windows Playwright browser battery under shared lease: `product-lifecycle-slice3.spec.ts` 2/2 PASS in 2.1m; full regression battery across M12 Slices 1–2, GHOST-PROMPT 1–3, SOMNIA 1–3, SwarmTank 1–3, Evaluations, Opportunities, and Integrated Workflow 26/26 specs PASS in 6.1m with zero provider traffic (total 28/28 browser tests PASS). [Evidence](reviews/windows-m12-product-lifecycle-slice3-verified.md). Milestone 12 complete on Windows; `AN-012` verified. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — M12 Product Lifecycle Slice 2 verified on Windows

Incoming delivery `anansi-m12-product-lifecycle-slice2-2026-11-03.zip` (delivery ZIP SHA-256 `507dc3b64dbfc48d0c739ef17a7bff94a92058292f2410bc402b94f163fee718`, bundle SHA-256 `0ad1ffdd4847c0b14e75cef7737e15138a6e7f65115d9c3eba440802cb637c37`, tip `77583b7c01f06301b6f575c7867fe96386cda423` over baseline `27c5754ac`) verified on Windows. Scope: M12 Product Lifecycle Slice 2 (MASTER_SPEC §61, §85). Implementation: deterministic rule-based marketing copy across four channels (Show HN, Product Hunt, X thread, release notes) derived from parsed concept / specification / SwarmTank run / SOMNIA evaluation evidence; provider generation gated strictly behind explicit prompt preview, cost estimate, and approval click; deterministic client-side stored-ZIP bundler (CRC32, fixed 1980 timestamps, sorted entries, `LAUNCH-MANIFEST.json`, zero unstated files) assembling channel markdown, SwarmTank objection summary, SOMNIA notes, and checked artifact bodies; `LaunchStudio.tsx` mounted in `Projects.tsx` when Launch stage is selected; navigator Launch row active; selection changes dynamically invalidate generated copy. NTFS qualification preserved: all new and touched imports explicitly use `.ts` and `.tsx` extensions. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 243 files PASS (`validate-docs.mjs`). UI component suite `pnpm test:ui` 220/220 PASS in 7.88s (including 20/20 new launch-studio tests). Production web build clean (2.26s). Backend Rust tests on Windows: zero Rust files modified; `cargo fmt` clean; full workspace 616 passed + 4 live-Ollama skips; `anansi-api` clean. Windows Playwright browser battery under shared lease: `product-lifecycle-slice2.spec.ts` 2/2 PASS in 20.3s; full regression battery across M12 Slice 1, GHOST-PROMPT 1–3, SOMNIA 1–3, SwarmTank 1–3, Evaluations, Opportunities, and Integrated Workflow 24/24 specs PASS in 5.3m with zero provider traffic (total 26/26 browser tests PASS). [Evidence](reviews/windows-m12-product-lifecycle-slice2-verified.md). `AN-012` in progress for Slices 1 & 2. Slice 3 (§85 guided walk) remains deferred. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — M12 Product Lifecycle Slice 1 verified on Windows

Incoming delivery `anansi-m12-product-lifecycle-slice1-2026-11-03.zip` (delivery ZIP SHA-256 `15aaba0a264bd68627edc9f5ca69544c1794e1578fbd8f310c7565f580035d9c`, bundle SHA-256 `e483b39e31d4e0b04bcab545b7ee2e74e402b85fae44bb49d682337d1ae30777`, tip `3b3e1a43db0ca5aeb74d750c9dc8bb06cb4fcfa2` over baseline `9222e969`) verified on Windows. Scope: M12 Product Lifecycle Slice 1 (MASTER_SPEC §60, §62, §85). Implementation: pure artifact-derived stage progression engine mirrored in Rust (`crates/core/src/pipeline.rs`, separated from memory-state machine in `lifecycle.rs`) and TypeScript (`packages/ui/src/lifecycle.ts`) tracking Discover, Define, Simulate, Pressure-Test, Refine, and Launch stages with pure status derivation from registered artifact evidence; `PipelineNavigator.tsx` stepper (`<nav aria-label="Product creation pipeline">`, per-stage badges, evidence counters, tool jump links) mounted in `Projects.tsx`; case-collision prevention on Windows NTFS (`./PipelineNavigator.tsx` import qualified); ghost e2e nav click disambiguation with exact matching. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 240 files PASS (`validate-docs.mjs`). UI component suite `pnpm test:ui` 198/198 PASS in 6.19s (including 6/6 new pipeline-navigator tests). Production web build clean (2.34s). Backend Rust tests on Windows: `pipeline` unit tests 7/7 PASS in 0.00s; full workspace 616 passed + 4 live-Ollama skips; `anansi-api` clean. Windows Playwright browser battery under shared lease: `product-lifecycle-slice1.spec.ts` 2/2 PASS in 18.6s; full regression battery across GHOST-PROMPT Slices 1–3, SOMNIA Slices 1–3, SwarmTank Slices 1–3, Evaluations, Opportunities, and Integrated Workflow 22/22 specs PASS in 5.7m with zero provider traffic (total 24/24 browser tests PASS). [Evidence](reviews/windows-m12-product-lifecycle-slice1-verified.md). `AN-012` in progress for Slice 1 scope. Slice 2 (Launch Studio copy generator & asset bundler) and Slice 3 (guided walk & full integration) remain deferred. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — M11 GHOST-PROMPT Slice 3 verified on Windows (Milestone 11 complete)

Incoming delivery `anansi-m11-ghost-prompt-slice3-2026-11-03.zip` (delivery ZIP SHA-256 `2823aeec5fb39fada8e4b634e290593761b8cd37b3c4ffffddca84eb486e9c6d`, bundle SHA-256 `d4717c54053cff1851c26c77cfd920418d805791391a80998d72a2b1eeeabf4f`, tip `03eb38001a99aad7e73703e064d0b385fb263498` over baseline `36988336`) verified on Windows. Scope: M11 GHOST-PROMPT Slice 3 (MASTER_SPEC §57–59, §86.5–7, §88). Implementation: deterministic rule-based draft analysis mirrored in Rust (`crates/core/src/ghost_prompt.rs`) and TypeScript (`packages/ui/src/ghostPrompt.ts`) detecting price adjustments, privacy removals, audience shifts, and technical term substitutions with the four §57 sentences verbatim and zero subconscious claims; explicit fragment selection behind the verbatim question *"Would you like ANANSI to consider any of these earlier alternatives?"*; dual-panel comparison studio (`packages/ui/src/GhostPromptCompare.tsx`: Final Prompt Only vs Final Prompt + Approved Draft Context, exact included context, labeled char/estimated-token/simulated-cost counts, offline reference responses by default, provider dispatch only on explicit click via `startAsk`/`pollAsk`, local feedback toggle, combine-into-prompt without auto-submit); §86 Definition of Done verified end to end in `ghost-prompt-slice3.spec.ts` (including guardrail test proving unselected fragments never reach Panel B or prompt dispatch). NTFS qualification preserved: all new and touched ghost imports explicitly use `.ts` and `.tsx` extensions. Zero server operations, zero new artifact types, zero DB writes, zero provider traffic on offline path. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 237 files PASS (`validate-docs.mjs`). UI component suite `pnpm test:ui` 192/192 PASS in 7.76s (including all 6 new ghost-prompt-analysis tests). Production web build clean (2.47s). Backend Rust tests on Windows: `ghost_prompt` unit tests 17/17 PASS in 0.04s; full workspace 609 passed + 4 live-Ollama skips; `anansi-api` clean. Windows Playwright browser battery under shared lease: `ghost-prompt-slice3.spec.ts` 2/2 PASS in 19.8s; full regression battery across GHOST-PROMPT Slices 1–2, SOMNIA Slices 1–3, SwarmTank Slices 1–3, Evaluations, Opportunities, and Integrated Workflow 20/20 specs PASS in 5.5m with zero provider traffic (total 22/22 browser tests PASS). [Evidence](reviews/windows-m11-ghost-prompt-slice3-verified.md). Milestone 11 complete on Windows; `AN-016` verified. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — M11 GHOST-PROMPT Slice 2 verified on Windows

Incoming delivery `anansi-m11-ghost-prompt-slice2-2026-11-02.zip` (delivery ZIP SHA-256 `b17133513e161f0be3bdf513b68a286df9c0d18b69cd9434c64da1197a696d51`, bundle SHA-256 `b01668c22fe5fdfbc3639600bc242ad13ced38023b2240a541c441286a93a99d`, tip `5818a45c3fdd528a52425b6bfbda303b27df6079` over baseline `9ac467fe`) verified on Windows. Scope: M11 GHOST-PROMPT Slice 2 (MASTER_SPEC §56, §65). Implementation: pure deterministic 3D layout (`packages/ui/src/ghostPromptThreeLayout.ts`, zero Three.js imports) mapping text offsets to [-200, 200] baseline coordinates with violet deleted nodes, cubic Bézier spline threads, ripple pause markers, and 8-character revision clusters; Three.js headless scene builder (`packages/ui/src/ghostPromptThreeScene.ts`, pinned `three@0.187.0`, no renderer created) with raycast picking, selection scaling, breathing animation, and full resource disposal; `GhostPrompt3D.tsx` managing low-power WebGL lifecycle (DPR clamped to 2) behind Review drafts "Expand 3D view" toggle, automatic WebGL-failure / `webglcontextlost` fallback to accessible 2D view, synchronous reduced-motion initialization from system/workspace settings with static layout toggle, SVG timeline on shared coordinates, per-node keyboard navigation, fragment detail card, and semantic `<ol>` history. NTFS qualification preserved: all new and touched ghost imports explicitly use `.ts` and `.tsx` extensions. Zero server operations, zero new artifact types, zero DB writes, zero provider traffic. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 234 files PASS (`validate-docs.mjs`). UI component suite `pnpm test:ui` 186/186 PASS in 5.92s (including 8/8 new ghost-prompt-three tests). Production web build clean (2.31s). Backend Rust tests on Windows: `ghost_prompt` unit tests 11/11 PASS in 0.35s; full workspace 603 passed + 4 live-Ollama skips; `anansi-api` clean. Windows Playwright browser battery under shared lease: `ghost-prompt-slice2.spec.ts` 2/2 PASS in 18.7s; full regression battery across GHOST-PROMPT Slice 1, SOMNIA Slices 1–3, SwarmTank Slices 1–3, Evaluations, Opportunities, and Integrated Workflow 18/18 specs PASS in 4.6m with zero provider traffic (total 20/20 browser tests PASS). [Evidence](reviews/windows-m11-ghost-prompt-slice2-verified.md). `AN-016` in progress for Slices 1 & 2. Slice 3 (§57–58 advice, comparison, provider dispatch) remains deferred. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — M12 Product Lifecycle Slice 1: pipeline navigator & stage engine (Linux; independent Windows rerun pending)

On branch `muse/m12-product-lifecycle-slice1` from verified Windows review head `9222e9698a7f…` (M11 CLOSED on Windows; the review MANIFEST's `9222e969a5316…` tail does not exist — third consecutive MANIFEST-tail discrepancy, bundle head authoritative; design committed before code, record in `docs/handoffs/M12-product-lifecycle-slice1-design.md` and `docs/handoffs/M12-product-lifecycle-slice1-addendum.md`). Scope: MASTER_SPEC §60, §62, §85 (foundational only; §61 generator and §85 guided walk deferred to Slices 2–3). Implementation: pure artifact-derived 7-stage engine mirrored in Rust (`crates/core/src/pipeline.rs` — placed there because `lifecycle.rs` already owns the memory-state machine) and TypeScript (`packages/ui/src/lifecycle.ts`); `PipelineNavigator.tsx` stepper (`<nav aria-label="Product Creation Pipeline">`, `aria-current="step"`, evidence counts, requirement sentences) mounted in the project view and fed by the loaded artifact list; stage → tool navigation via anchored sections plus Ask-view plumbing for Refine; Launch honestly disabled until Slice 2; "Jump to tool" override on Awaiting rows. No server op, no new artifact type, no schema change, zero provider traffic. Linux verification: Rust workspace 616 passed + 4 known live-Ollama environment failures (609 + 7 new pipeline tests), node 198/198 (192 + 6 new), tsc 0, contracts 0 drift, docs validation PASS, web build clean, e2e `product-lifecycle-slice1.spec.ts` 2/2, regression battery per MANIFEST. AN-012 stays `in progress`. Independent Windows rerun pending (checklist in the delivery MANIFEST). M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-03 — M11 GHOST-PROMPT Slice 3: draft analysis, selection, comparison — Milestone 11 complete (Linux; Windows-verified 2026-11-03)

On branch `muse/m11-ghost-prompt-slice3` from verified Windows review head `3698833646bd92353791a5948d823e05e73c1144` (design committed before code, record in `docs/handoffs/M11-ghost-prompt-slice3-design.md` and `docs/handoffs/M11-ghost-prompt-slice3-addendum.md`, the latter being the §88 milestone report). Baseline note: the assignment and the review MANIFEST name parent `3698833634e062630a845cfeb368fa918b958807`, but that full SHA does not exist; the review bundle's only head is `3698833646bd…` (shared prefix, different tail) with the matching message, parent, and docs-only diff, and it is exactly the assignment's authorized alternative (clean tip `5818a45c3` + review-docs-only diff) — the discrepancy is recorded in the design doc, the addendum, and the delivery MANIFEST. Scope: MASTER_SPEC §57–59, §86.5–7, §88. Implementation: deterministic rule-based draft analysis mirrored in Rust (`ghost_prompt.rs`) and TypeScript (`ghostPrompt.ts`) — price signals, privacy removals, audience shifts, technical-term substitutions, with the four §57 sentences verbatim and no intent claims; explicit fragment selection behind the verbatim question "Would you like ANANSI to consider any of these earlier alternatives?"; dual-panel comparison (`GhostPromptCompare.tsx`: Final Prompt Only vs Final Prompt + Approved Draft Context, exact included context, labeled char/estimated-token/simulated-cost counts, offline reference responses by default, provider dispatch only on the explicit "Send both to Ask ANANSI" click via the existing `startAsk`/`pollAsk` path, local feedback toggle, combine-into-prompt without auto-submit); §86 Definition of Done verified end to end in `ghost-prompt-slice3.spec.ts`. No server op, no new artifact type, no schema change, zero provider traffic on the offline path. Linux verification: Rust workspace 609 passed + 4 known live-Ollama environment failures (603 + 6 new ghost analysis tests), strict Clippy/fmt clean, node 192/192 (186 + 6 new), tsc 0, contracts 0 drift, docs validation PASS, web build clean, e2e `ghost-prompt-slice3.spec.ts` 2/2 (§86 walk + unselected-fragment guardrail), regression battery 22/22. AN-016 advances to `verified` with "Milestone 11 complete" in its text (the validator has no `complete` status; SOMNIA Slice 3 precedent). Independent Windows rerun of Slice 3 PASSED 2026-11-03 (review tip `9222e9698a7f`, docs-only delta; tsc 0, contracts 0 drift, docs 237, UI 192/192, Rust ghost 17/17, workspace 609 + 4 skips, Vite 2.47s, Playwright 22/22). Milestone 11 CLOSED on Windows. M12 Slice 1 delivered 2026-11-03 (Linux; Windows rerun pending).

## 2026-11-02 — M11 GHOST-PROMPT Slice 2: 3D visualization & accessible alternative (Linux; Windows-verified 2026-11-03)

On branch `muse/m11-ghost-prompt-slice2` from verified Windows review head `9ac467fec54c28c957428187d998d73599a6a965` (design committed before code, record in `docs/handoffs/M11-ghost-prompt-slice2-design.md` and `docs/handoffs/M11-ghost-prompt-slice2-addendum.md`). Baseline note: the assignment and the review MANIFEST name parent `9ac467fe835c2b3e839218671fc31efbe8855a8f`, but that full SHA does not exist; the review bundle's only head is `9ac467fec54c…` (shared prefix, different tail) with the matching message, parent, and diff, and it is exactly the assignment's authorized alternative (clean tip `0f9eec261` + the NTFS import qualification) — the discrepancy is recorded in the design doc, the addendum, and the delivery MANIFEST. Scope: MASTER_SPEC §56, §65. Implementation: pure deterministic 3D layout (`ghostPromptThreeLayout.ts`, no Three.js import) mapping offsets to a [-200, 200] baseline with staggered violet deleted nodes, thread segments, pause markers at the next edit's baseline point, and 8-character revision clusters, plus the fixed-camera projection formula (fov 45 at (0, 30, 320)); Three.js scene builder (`ghostPromptThreeScene.ts`, pinned `three@0.187.0`, no renderer created) with raycast picking, 1.5× selection scaling, ring-only breathing animation, and full geometry/material/texture disposal; `GhostPrompt3D.tsx` owning the renderer lifecycle (low-power preference, DPR clamped to 2) behind the Review drafts section's `Expand 3D view` toggle, with WebGL-failure and mid-session `webglcontextlost` fallback to a complete 2D view, synchronous reduced-motion init with a toggle (the static layout IS the 2D view), an SVG timeline on the same layout numbers, per-node keyboard selection buttons, canvas arrow-key cycling, a selected-fragment detail card, and a semantic `<ol>` edit history. Every new or edited ghost-module import uses an explicit extension (the Slice 1 NTFS lesson, applied uniformly). No server op, no new artifact type, no schema change, no Rust change, zero provider traffic. Linux verification: Rust workspace 603 passed + 4 known live-Ollama environment failures, strict fmt clean, node 186/186 (178 + 8 new), tsc 0, contracts 0 drift, docs validation PASS, web build clean, e2e `ghost-prompt-slice2.spec.ts` 2/2 (canvas + raycast node picking → detail card; reduced-motion static view both ways + context-loss → 2D fallback), regression battery 20/20. Independent Windows rerun of Slice 2 PASSED 2026-11-03 (review tip `3698833646bd`, docs-only delta; tsc 0, contracts 0 drift, docs 234, UI 186/186, Rust ghost 11/11, workspace 603 + 4 skips, Vite 2.31s, Playwright 20/20). Slice 3 (§57–58 advice, comparison, provider dispatch) delivered 2026-11-03 (Linux; Windows rerun pending). M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-02 — M11 GHOST-PROMPT Slice 1 verified on Windows

Incoming delivery `anansi-m11-ghost-prompt-slice1-2026-11-02.zip` (delivery ZIP SHA-256 `3fc8f005ac821d86a6f122704ae461aac740182cb0dc69dfe0afc91f3c21c4ee`, bundle SHA-256 `6a529458b571e16b81e9b8d0adafb56e3cc1598f3e5986d256a0eca20fbba15a`, tip `0f9eec261fbe179706975fb399cfa2f6fed22394` over baseline `d7887ad82`) verified on Windows. Scope: M11 GHOST-PROMPT Slice 1 (MASTER_SPEC §52–55, §86.1–4, §86.8–9). Implementation: OFF-by-default informed consent model with verbatim §53 modal disclosure, persistent `Draft Exploration ON` status badge with Recording/Paused states, session controls (`Pause`, `Resume`, `Review`, `Clear`, `Stop & discard`), in-memory session engine in Rust (`crates/core/src/ghost_prompt.rs`) and TypeScript (`packages/ui/src/ghostPrompt.ts`) tracking prefix/suffix diff spans and offsets, factual 2000ms pause events (durations only, zero emotional/psychological inference), 500-event / 50KB ring buffer bounds, 4-hour auto-stop, deterministic FNV-1a / SHA-256 content hashes, password scope refusal, observable memory wipe, and Ask ANANSI `GhostPromptStrip` with Retained & Deleted Fragments table. Zero server operations, zero new artifact types, zero DB writes, zero provider traffic. Reviewer fix on Windows NTFS: explicitly qualified `./GhostPrompt.tsx` imports in `AskAnansi.tsx` and `ghost-prompt.test.tsx` to prevent case-collision with `ghostPrompt.ts` under Node's case-insensitive extension resolution. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 231 files PASS (`validate-docs.mjs`). UI component suite `pnpm test:ui` 178/178 PASS in 4.84s (including 10/10 ghost-prompt tests). Production web build clean (2.58s). Backend Rust tests on Windows: `ghost_prompt` unit tests 11/11 PASS in 0.05s; full workspace 603 passed + 4 live-Ollama skips; `anansi-api` clean. Windows Playwright browser battery under shared lease: `ghost-prompt-slice1.spec.ts` 2/2 PASS in 15.8s; regression battery across SOMNIA, SwarmTank, Forge, Opportunities, and Evaluator 16/16 specs PASS in 4.2m with zero provider traffic (total 18/18 browser tests PASS). [Evidence](reviews/windows-m11-ghost-prompt-slice1-verified.md). `AN-016` in progress for Slice 1 scope. Slices 2 (Three.js 3D visualization) and 3 (semantic advice, comparison mode, provider dispatch) remain deferred. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-02 — M11 GHOST-PROMPT Slice 1: consent, in-memory engine, revision capture (Linux; independent Windows rerun pending)

On branch `muse/m11-ghost-prompt-slice1` from verified Windows review tip `d7887ad82` (design committed before code, record in `docs/handoffs/M11-ghost-prompt-slice1-design.md` and `docs/handoffs/M11-ghost-prompt-slice1-addendum.md`). The required baseline was absent locally at assignment time, so work was held until the owner uploaded the M10 Slice 3 Windows review package, which was hash- and ancestry-verified before branching. Scope: MASTER_SPEC §52–55, §86.1–4, §86.8–9. Implementation: OFF-by-default opt-in modal with the §53 consent text verbatim, persistent `Draft Exploration ON` badge with Recording/Paused status, Pause/Resume/Review/Clear/Stop controls, Rust reference engine plus byte-semantic TS mirror (prefix/suffix diff, exact deleted spans and offsets, 2000 ms factual pause events, 500-event / 50KB ring bounds, 4-hour auto-stop, FNV-1a shared hash with Rust-only SHA-256, password-scope refusal, observable wipe with `verifyWiped()`), and the Retained & Deleted Fragments inspection table in Ask ANANSI. No server op, no new artifact type, no schema change, zero provider traffic. Linux verification: Rust workspace 603 passed + 4 known live-Ollama environment failures (592 + 11 new ghost tests), strict Clippy/fmt clean, node 178/178, tsc 0, contracts 0 drift, docs 231 files PASS, web build clean, browser battery 68 passed / 1 known search real-semantic failure of 69 tests, including the new 2-test ghost spec. Independent Windows rerun pending (checklist in the delivery MANIFEST). Slices 2 (§56 visualization) and 3 (§57–58 advice, comparison, provider dispatch) remain deferred and uncommissioned. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-02 — M10 SOMNIA Slice 3 verified on Windows (Milestone 10 complete)

Incoming delivery `anansi-m10-somnia-slice3-2026-11-02.zip` (delivery ZIP SHA-256 `dd3a8568b6fc419fb0c4202d1094a9c74f4a2ed9e6e5526ea8b9af0b54c5605c`, bundle SHA-256 `c672c2946091e094c2f47374d27cc2b03ea967f09f4d66cd39850ae1e71ed885`, tip `91db98ea07e5d8f3fa8a5342b8020078bb02ea87` over Windows baseline `b9f9edfc`) verified on Windows. Scope: M10 SOMNIA Slice 3 (MASTER_SPEC §50, §51, §87, §88). Implementation: §50 dashboard aggregates over saved evaluations (failure rate %, context efficiency ratio vs baseline, duplicate evidence/source counts, pending proposals count, 6-of-7 failure class coverage with sealed class honestly noted as uncoverable by saved evaluations, latest run identity and §9 record timestamp, history totals and regression index tracking), §51 Product Forge QA bridge (seven deterministic QA scenarios per registered Forge concept, §51 disclosure verbatim, JSON/Markdown export), §87.7 reviewed + dismissed proposal state (local-only, no database write, zero live memory mutation), dashboard action triggers, and §87 full end-to-end walk. Reviewer test synchronization fix: in `tests/e2e/somnia-slice3.spec.ts` (`saveBaselineAndNightmare`), replaced premature banner text assertion with exact count assertion on `savedList.getByRole('button', { name: 'View evaluation' })` (count 1 then 2) before reading dashboard metrics or executing backup/restore. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 228 files PASS (`validate-docs.mjs`). UI component suite `pnpm test:ui` 168/168 PASS in 4.80s. Production web build clean (1.86s). Backend Rust tests on Windows: `somnia` unit tests 21/21 PASS in 0.02s; full workspace 592 passed + 4 live-Ollama skips; `anansi-api` clean. Windows Playwright browser battery under shared lease: `somnia-slice3.spec.ts` 2/2 PASS in 31.9s; full regression battery across SOMNIA, SwarmTank, Forge, Opportunities, and Evaluator 16/16 specs PASS in 4.3m with zero provider traffic. [Evidence](reviews/windows-m10-somnia-slice3-verified.md). Milestone 10 complete on Windows; AN-015 verified. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-02 — M10 SOMNIA Slice 2 verified on Windows

Incoming delivery `anansi-m10-somnia-slice2-2026-11-02.zip` (delivery ZIP SHA-256 `cfeaa5e34314ae3680dd3342b616cd94bf9d99c23dd6f8cfab4c414a943eb64d`, bundle SHA-256 `ee7e11e2a53b854b497b99c6486bbf51400583fe0a23814d6b377c7f9d0fc815`, tip `b9f9edfcc0f2945ae8dec2446c19e13ed7017d51` over Windows baseline `8ba785ae`) verified on Windows. Scope: M10 SOMNIA Slice 2 (MASTER_SPEC §45, §47, §48, §87). Implementation: seeded nightmare generator (FNV-1a + xorshift32, byte-identical Rust/TS mirror, golden test in both suites) over seven failure classes (`conflicting-instructions`, `missing-evidence`, `simulated-tool-error`, `simulated-tool-timeout`, `malformed-record`, `untrusted-instruction`, `sealed-access-attempt`), evaluated by unchanged Slice 1 reference evaluator; review-only inert consolidation proposals derived at view time from saved evaluation (no apply path, no memory mutation, no new artifact type); before/after comparison studio over two saved evaluations with transition matrix, delta metrics, and deterministic JSON/Markdown export; SwarmTank re-run button renamed to `Re-run Deterministic Scenario Simulation` with help text stating model-free PRNG evaluation and zero live AI inference. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 226 files PASS. UI component suite `pnpm test:ui`: 165/165 PASS in 4.70s. Production web build clean (2.09s). Backend Rust tests on Windows: `somnia` unit tests 18/18 PASS in 0.02s; full workspace 589 passed + 4 live-Ollama skips; `anansi-api` clean. Windows Playwright battery under shared lease: `somnia-slice2.spec.ts` 2/2 PASS in 42.1s; full regression battery across SOMNIA, SwarmTank, Forge, Opportunities, and Evaluator 14/14 specs PASS in 3.6m with zero provider traffic. [Evidence](reviews/windows-m10-somnia-slice2-verified.md). AN-015 verified for Slice 2 scope. Milestone 10 in progress. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-02 — M10 SOMNIA Slice 1 verified on Windows

Incoming delivery `anansi-m10-somnia-slice1-2026-11-02.zip` (delivery ZIP SHA-256 `0363dc59f08c238484774112b2ba0301b7a323f6ed02f751859cebf5ca2c321a`, bundle SHA-256 `6df8a4e705a4551f44637cff2753131e5b962cfd6b4f745b8f9ba49b7c0175bb`, tip `8ba785aebb120e98eee9b7698a1d7eb11c5a3395` over Windows baseline `68ee3532`) verified on Windows. Scope: M10 SOMNIA Slice 1 (MASTER_SPEC §42–51, §72–74, §87). Implementation: deterministic reference replay evaluator in `crates/core/src/somnia.rs` enforcing rules R1–R5 (untrusted instructions, contradictory instructions, missing evidence citations, simulated tool events and error/timeout aborts), manual project selection and bounded synthetic input preview, explicit run approval, immutable `somnia-evaluation` artifact persistence via `somnia-save-evaluation` op after server-side recomputation, strict Sealed refusal gate at preview/evaluate/save, backup and fresh restore remapping with evaluation provenance intact, and "Not measured" labeling for unmeasured tokens/accuracy/production resilience. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 223 files PASS. UI component suite `pnpm test:ui`: 159/159 PASS in 4.60s. Production web build clean (2.23s). Backend Rust tests on Windows: `somnia` unit tests 12/12 PASS in 0.02s; full workspace 583 passed + 4 live-Ollama skips; `anansi-api` clean. Windows Playwright battery under shared lease: `somnia-slice1.spec.ts` 2/2 PASS in 28.5s; regression battery 12/12 specs PASS in 3.1m with zero provider traffic. [Evidence](reviews/windows-m10-somnia-slice1-verified.md). AN-015 verified for Slice 1 scope. Milestone 10 in progress. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-02 — M10 SOMNIA Slice 3: dashboard aggregates, Product Forge QA augmentation, milestone closure (Linux; independent Windows rerun pending)

On branch `muse/m10-somnia-slice3` from owner tip `b9f9edfc` (design committed before code, record in `docs/handoffs/M10-somnia-slice3-design.md` and `docs/handoffs/M10-somnia-slice3-addendum.md`, which also carries the §88 milestone report). §50 dashboard over saved evaluations (failure rate, context efficiency vs baseline, duplicate evidence/source counts, pending proposals, 6-of-7 coverage with the sealed class honestly noted as uncoverable by saved evaluations, latest run, history totals, regressions); §51 Forge QA bridge (seven deterministic QA scenarios per registered Forge concept/deliverable through the unchanged evaluator, saved via the existing op, JSON/Markdown export, §51 disclosure verbatim); §87.7 reviewed + dismissed proposal state (both local-only, no apply path). Slice 2 Windows verification is recorded in the addendum (review tip `4ca4a146`: tsc 0, contracts 0 drift, docs 226, UI 165/165, Rust somnia 18/18, workspace 589 + 4 offline-Ollama skips, Vite 2.09s, browser battery 14/14); AN-015 is `complete` for Milestone 10, with the Slice 3 scope itself still awaiting its own Windows rerun. Linux verification: Rust workspace 592 passed + 4 known live-Ollama environment failures, Rust somnia 21/21, strict Clippy/fmt clean, node 168/168, tsc/contracts/docs (226 files)/web build clean, browser battery 48/49 specs (66/67 tests; single failure the known pre-existing search real-semantic test), including the new 2-test SOMNIA Slice 3 spec. Independent Windows rerun pending (checklist in the delivery MANIFEST). Non-goals unchanged: scheduling, lesson promotion, live memory mutation, historical ingestion, provider-backed evaluation. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-02 — M10 SOMNIA Slice 2: seeded nightmares, inert proposals, comparison, SwarmTank wording repair (Linux; independent Windows rerun pending)

On branch `muse/m10-somnia-slice2` from owner tip `8ba785aeb` (design committed before code, record in `docs/handoffs/M10-somnia-slice2-design.md` and `docs/handoffs/M10-somnia-slice2-addendum.md`). Deterministic seeded nightmare generator (FNV-1a + xorshift32, byte-identical Rust/TS mirror, golden test in both suites) over seven failure classes, evaluated by the unchanged Slice 1 evaluator; review-only inert consolidation proposals derived at view time from a saved evaluation (no apply path, no new artifact type); before/after comparison studio over two saved evaluations with deterministic JSON/Markdown export; SwarmTank re-run button renamed to `Re-run Deterministic Scenario Simulation` with help text stating model-free PRNG evaluation and no live AI inference. Slice 1 Windows verification is recorded in the addendum (tsc 0, contracts 0 drift, docs 223, UI 159/159, Rust somnia 12/12, workspace 583 + 4 offline-Ollama skips, Vite 2.23s, browser subset 12/12); AN-015 is `verified` for the Slice 1 scope only. Linux verification: Rust workspace 589 passed + 4 known live-Ollama environment failures, Rust somnia 18/18, strict Clippy/fmt clean, node 165/165, tsc/contracts/docs (224 files)/web build clean, browser battery 47/48 specs (64/65 tests; single failure the known pre-existing search real-semantic test; the battery caught a genuine Slice 2 regression — the new Exclude button broke a Slice 1 sealed assertion — fixed by scoping that assertion to the error list item), including the new 2-test SOMNIA Slice 2 spec. Independent Windows rerun pending (checklist in the delivery MANIFEST). Non-goals unchanged: scheduling, lesson promotion, live memory mutation, dashboard aggregates, Forge integration, historical ingestion, provider-backed evaluation. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-02 — M10 SOMNIA Slice 1: isolated, manual, data-only replay (Linux; independent Windows rerun pending)

On branch `muse/m10-somnia-slice1` from Windows coordination baseline `68ee3532` (design committed before code, record in `docs/handoffs/M10-somnia-slice1-design.md` and `docs/handoffs/M10-somnia-slice1-addendum.md`). One complete workflow: select an existing project, supply bounded synthetic or redacted input, preview exactly what will be evaluated, approve the run explicitly, inspect decisions/violations/metrics from a deterministic seeded reference evaluator (rules R1–R5 + constraint check on every decision, no model, no real tools, no network), save one immutable `somnia-evaluation` artifact after server-side recomputation, restart/reopen and backup/fresh-restore with provenance intact. Sealed input refused at preview, evaluate, and save; evaluation changes no memory, policy, project, or permission; model tokens/accuracy/production resilience shown as Not measured, never zero-filled. Linux verification: Rust workspace 583 passed + 4 known live-Ollama environment failures (same `semantic_workflow` family as prior slices), Rust somnia 12/12, strict Clippy/fmt clean, node 159/159, tsc/contracts/docs (222 files)/web build clean, browser battery 46/47 specs (62/63 tests; single failure the known pre-existing search real-semantic test; the battery initially exposed a genuine Slice 1 regression — a "Save evaluation" button-name collision with the M6 panel — fixed by renaming to "Save SOMNIA evaluation", all affected specs pass on rerun), including the new 2-test SOMNIA spec. Independent Windows rerun pending (checklist in the delivery MANIFEST). Non-goals unchanged: scheduling, consolidation, lesson promotion, nightmare generation, historical ingestion, provider-backed evaluation. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-11-01 — M9 SwarmTank Slice 3 verified on Windows (Milestone 9 complete)

Incoming delivery `anansi-m9-swarmtank-slice3-2026-10-31.zip` (tip `bf79643eb` over Windows baseline `3d0537ea`) verified on Windows. Scope: M9 SwarmTank Slice 3 (MASTER_SPEC §39, §40, §41). Implementation: interactive scenario controls (`packages/ui/src/SwarmtankScenarioControls.tsx` with price slider/presets, feature toggles, trust toggles, setup segmented buttons), instant local sensitivity engine mirrored in Rust (`crates/core/src/swarmtank.rs`) and TypeScript (`packages/ui/src/swarmtankSensitivity.ts`) with hard budget veto at `priceSensitivity >= 0.7`, reactive metrics bar with signed deltas, dynamic 3D arena re-clustering (600ms eased radial interpolation, color sync, instant snap under reduced motion), side-by-side comparison studio (`packages/ui/src/SwarmtankComparison.tsx` with transition matrix and objection deltas), versioned scenario-branch persistence (`anansi-swarmtank-scenario` v1, artifact type `scenario`), expanded Forge bridge supporting baseline or scenario recommendations (Forge concept v3 with structured `swarmtankScenarioDiff`), and deterministic JSON/Markdown export (`swarmtankScenarioExport.ts`). Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 219 files. UI component suite `pnpm test:ui`: 152/152 PASS. Production web build clean (1.88s). Backend Rust tests on Windows: `swarmtank` 18/18 PASS in 0.05s. Windows Playwright battery under shared lease: `swarmtank-scenario.spec.ts` 3/3 PASS in 1.1m (slider updates conversion and arena re-clusters, feature toggles and trust signals resolve objections, scenario branching, Forge revision to v3, and backup/restore survival), `swarmtank-arena.spec.ts` 3/3 PASS in 25.4s, `swarmtank-slice1.spec.ts` 1/1 PASS in 32.3s, all with zero provider traffic. [Evidence](reviews/windows-m9-swarmtank-slice3-verified.md). Milestone 9 complete on Windows; AN-014 verified. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-10-31 — M9 SwarmTank Slice 3: interactive scenario controls, sensitivity engine, comparison studio (Linux; independent Windows rerun pending)

On branch `muse/m9-swarmtank-slice3` from Windows-verified Slice 2 baseline `3d0537ea` (design committed before code, record in `docs/handoffs/M9-swarmtank-slice3-design.md` and `docs/handoffs/M9-swarmtank-slice3-addendum.md`). The final M9 slice: scenario controls over a saved run (price slider + presets, feature toggles, trust toggles, setup segmented buttons) driving an instant local sensitivity engine mirrored in Rust and TypeScript (scenario veto at price sensitivity ≥ 0.7, feature-drop and complex-setup downgrades, free-trial resolution for cost-conscious freelancers and small agencies), a reactive metrics bar with signed deltas, dynamic arena re-clustering (600 ms eased interpolation, colors immediate, snap under reduced motion, no scene rebuild), a side-by-side comparison studio (transition breakdown, resolved/introduced objections, price elasticity), versioned scenario-branch persistence (`anansi-swarmtank-scenario` v1, artifact type `scenario`, self-contained with full recomputation at restore), a Forge bridge that can apply baseline or scenario recommendations (scenario diff addendum, Forge v3), and deterministic JSON/Markdown comparison export. Baseline engine frozen; branch parent pins are historical provenance, never remapped. Linux verification: Rust workspace 571 passed + 4 known live-Ollama environment failures, Rust swarmtank 18/18, strict Clippy/fmt clean, node 152/152, tsc/contracts/docs (219 files)/web build clean, browser battery 45/46 specs (60/61 tests; the single failure is search.spec.ts's real-semantic test, which needs live Ollama embeddings absent on this VM and previously failed identically on the pristine baseline), including the new 3-test scenario spec and the Slice 2 arena spec 3/3 after a locator fix (loose 'Conversion rate:' text now matches three texts; matched by shape). Independent Windows rerun pending (checklist in the delivery MANIFEST). Non-goals unchanged: provider-backed re-evaluation, per-agent 3D labels, native save dialog, §39 headline/value-prop/audience edits. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-10-31 — M9 SwarmTank Slice 2 verified on Windows

Incoming delivery `anansi-m9-swarmtank-slice2-2026-10-31.zip` (tip `855320d93` over Windows baseline `75a0b4272`) verified on Windows. Scope: M9 SwarmTank Slice 2 (MASTER_SPEC §38, §65). Implementation: Three.js 3D Arena visualization (`packages/ui/src/SwarmtankArena.tsx`, `SwarmtankThreeView.tsx`, `swarmtankThreeScene.ts`, `swarmtankArenaLayout.ts`) with central product monolith on a circular stage, concentric amphitheater rings, instanced agents clustered by decision (Convert green inward, Confused amber orbiting with animated hesitation markers, Reject red outward facing away), raycast picking with agent scaling and popover detail card, bidirectional 2D table synchronization, pinned `three@0.187.0`, DPR clamped to 2, low-power WebGL preference, full geometry/material/texture disposal, synchronous reduced-motion state, and graceful WebGL fallback. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 216 files. UI component suite `pnpm test:ui`: 138/138 PASS. Production web build clean (2.19s). Backend Rust tests: `swarmtank` 9/9 PASS. Windows browser Playwright battery under shared lease: `swarmtank-arena.spec.ts` 3/3 PASS in 35.4s (mount, picking, sync, reduced-motion toggle, WebGL fallback) and `swarmtank-slice1.spec.ts` 1/1 PASS in 49.3s with zero provider traffic. Captured 3 screenshots to `docs/reviews/evidence/windows-m9-swarmtank/`. [Evidence](reviews/windows-m9-swarmtank-slice2-verified.md). Section 39 scenario sliders (Slice 3), desktop shell rendering, and native save dialog remain open; M9 in progress.

## 2026-10-31 — M9 SwarmTank Slice 2: 3D arena visualization & accessible alternative (Linux; independent Windows rerun pending)

On branch `muse/m9-swarmtank-slice2` from Windows-verified Slice 1 baseline `75a0b427` (design committed before code, record in `docs/handoffs/M9-swarmtank-slice2-design.md` and `docs/handoffs/M9-swarmtank-slice2-addendum.md`). Three.js arena inside the SwarmTank report: central product monolith on a circular stage, concentric amphitheater rings, one instanced agent per persona — Convert green closest facing the pedestal, Confused amber orbiting with hesitation markers, Reject red outermost facing away — with raycast picking to a detail card (role, budget, key objection, perceived value) and a synchronized 2D table fallback always present. Guardrails: pinned `three@0.187.0`, DPR clamped to 2, low-power context, full disposal on unmount/run switch, `webglcontextlost` fallback, static frame under system or workspace reduced motion or the arena's own toggle (synchronous initial value, the §26 lesson), failure tracked per run identity. Linux verification: Rust workspace 562 passed + 4 known live-Ollama environment failures, strict Clippy/fmt clean, node 138/138, tsc/contracts/docs (216 files)/web build clean, browser battery 18/18 across 15 specs including the new 3-test arena spec. Independent Windows rerun pending (checklist in the delivery MANIFEST). Non-goals unchanged: Slice 3 scenario controls, provider-backed re-evaluation, per-agent 3D labels, native save dialog. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-10-31 — M9 SwarmTank Slice 1: synthetic personas & decision engine (Linux; independent Windows rerun pending)

On branch `muse/m9-swarmtank-slice1` from Windows-verified baseline `7c576b85` (design committed before code, record in `docs/handoffs/M9-swarmtank-slice1-design.md` and `docs/handoffs/M9-swarmtank-slice1-addendum.md`). Deterministic, provider-free SwarmTank panel in Projects / Innovation Studio: manual + saved-experiment / Forge-concept input import, seeded persona generation (presets 20/50/100, batches of 10, SHA-256 counter-mode PRNG, every persona labeled simulated), seven-factor Convert/Confused/Reject decision engine with price-exceeds-budget veto, derived §40 report (conversion rate, average willingness to pay, top-3 confusion/objections, rule-derived recommendations), deterministic JSON export, and "Apply Recommendations to Forge" via `artifact_revise` with an idempotent `swarmtank_addendum`. Runs persist as `simulation` artifacts (`anansi-swarmtank-run` v1) with content hashes and full decision recomputation at restore. Linux verification: Rust workspace 562 passed + 4 known live-Ollama environment failures, strict Clippy/fmt clean, node 131/131, tsc/contracts/docs (212 files)/web build clean, browser battery 15/15 across 14 specs including the new vertical slice (import → generate → evaluate → report → Forge transfer → backup/fresh restore → export, zero provider traffic). Independent Windows rerun pending (checklist in the delivery MANIFEST). Non-goals unchanged: §38 3D arena, scenario engine, provider-backed generation, URL import, spec auto-start, native save dialog. M7 native acceptance, preview promotion, §32 sandbox, M5 gates, and M8 Slice 4 Windows verification remain open.

## 2026-10-31 — M9 SwarmTank Slice 1 verified on Windows

Incoming delivery `anansi-m9-swarmtank-slice1-2026-10-31.zip` (tip `9f9047852` over Windows baseline `7c576b85e`) verified on Windows. Scope: M9 SwarmTank Slice 1 (MASTER_SPEC §34–37, §40–41). Implementation: seeded persona generator (20/50/100 presets, 10-batch chunking, role, budget, skepticism, price sensitivity, purchase constraints, clear simulated labeling in `crates/core/src/swarmtank.rs`), seven-factor probabilistic decision model (`Convert`, `Confused`, `Reject` with hard price-over-budget veto), SwarmTank report with conversion rate %, top 3 objections, top 3 confusion points, tabular persona breakdown, JSON export, and Product Forge bridge ("Apply Recommendations to Forge" via `artifact_revise` updating concept to version 2 with immutable `swarmtankAddendum`). Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 213 files. UI component suite `pnpm test:ui` 131/131 PASS. Backend Rust test suites on Windows: `swarmtank` unit tests 9/9 PASS, `forge_restore.rs` 4/4 PASS, `transfer.rs` 19/19 PASS, `anansi-api` clean. Browser Playwright vertical slice under shared lease: `swarmtank-slice1.spec.ts` PASS in 31.9s, verifying manual/import flow, persona generation, decision evaluation, report metrics, Forge concept revision to v2, backup/fresh restore survival, and JSON export with zero provider traffic. [Evidence](reviews/windows-m9-swarmtank-slice1-verified.md). Section 38 3D arena (Slice 2), scenario sliders (Slice 3), and native save dialog remain open; M9 in progress.

## 2026-10-31 — M8 §27 artifacts & Forge provisioning verified on Windows

Incoming delivery `anansi-m8-complete-artifacts-forge-2026-10-31.zip` (tip `dbc49bd16` over `7632f9476`) verified on Windows. Scope: §27 six-artifact deterministic derivation (`crates/core/src/collider_artifacts.rs` and `packages/ui/src/colliderArtifacts.ts`), embedded SHA-256 and byte counts, safe tabbed markdown viewer, deterministic STORE zip export with MANIFEST hashes, and "Build This Product" Forge bridge (`ColliderBuildProduct.tsx`) using derived `ARCHITECTURE.md` as initial spec. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 210 files. UI component suite `pnpm test:ui` 125/125 PASS. Backend Rust test suites on Windows: `collider.rs` 49/49 PASS, `forge_restore.rs` 4/4 PASS, `transfer.rs` 19/19 PASS, `anansi-api` clean. Browser Playwright vertical slice under shared lease: `collider-artifacts-forge.spec.ts` PASS in 37.5s, verifying tabs, line search, export byte-identity and manifest hashes, Forge concept creation, and backup/restore survival. Screenshots captured and inspected in `docs/reviews/evidence/windows-m8-artifacts/`. [Evidence](reviews/windows-m8-artifacts-forge-verified.md). Native host file save dialog remains open; M8 in progress.

## 2026-10-31 — M8 Three.js visualization (§26) verified on Windows

Incoming delivery `anansi-m8-threejs-visualization-2026-10-31.zip` (tip `7632f9476` over reviewer baseline `9147e3494`) verified on Windows. Scope: §26 Three.js / WebGL renderer with 2D Canvas resilient fallback and persistent accessible node list. Dependencies `three@0.187.0` and `@types/three@0.187.0` pinned. Static checks: `tsc --noEmit` 0 errors, contracts drift 0, docs 207 files. UI component suite `pnpm test:ui` 115/115 PASS. Production web build clean (767ms). Windows Chromium Playwright battery under shared lease: 5/5 PASS (1.2m) across standard and 2x high-DPI viewports, raycast selection, version switches, and WebGL context loss fallback. Reviewer fix: synchronous initialization of `reducedMotion` state prevents initial-mount animation flicker and unmounting race condition during high-DPI screenshot tests. Seven screenshots captured and visually inspected in `docs/reviews/evidence/windows-m8-threejs/`. [Evidence](reviews/windows-m8-threejs-visualization-verified.md). No backend, schema, or contract changes.

On branch `muse/m8-threejs-visualization` from
reviewer tip `9147e349` (the Windows-verified
visualization repair runtime; design committed before
code, record in
`docs/handoffs/M8-threejs-visualization-design.md` and
`docs/handoffs/M8-threejs-visualization-addendum.md`).
The §26 renderer is implemented: `three@0.187.0` and
`@types/three@0.187.0` pinned in `@anansi/ui` via pnpm
11.25.0 (frozen install verified); a pure
scene-graph builder over the shared derived scene
(two toroidal beam streams, orbiting fragments,
central collision marker, instanced output nodes,
section-flow edges, text sprites), a renderer
lifecycle component (DPR clamped to 2, resize,
disposal, rAF cancellation, reduced-motion static
frame, `webglcontextlost` fallback), and the Canvas
2D view preserved as the resilient fallback with the
list always present. One real defect found by the
fallback spec and fixed: the parent's identity effect
clobbered the child's WebGL-failure signal (child
effects run first), so fallback is now tracked per
identity. Linux verification: node 115/115, tsc/build/
contracts clean, docs 207 files, browser battery 8/8
across 7 specs incl. raycast selection, version-switch
clearing, byte-identical reduced-motion frames, and
1x/2x screenshots. No Rust/schema/contract change.
Windows rerun, native acceptance, preview promotion,
M5 gates, §32 sandbox, and Slice 4 Windows
verification remain open; this slice completes the
§26 renderer, not M8.

## 2026-10-31 — Canvas visualization repair verified on Windows

Runtime `cdae30bca` closes VIS-01/02 in focused scope: UI105/105, TypeScript/web build/contracts/docs pass; browser4/4 incl. pinned selection, restart/restore, URL/document regressions and high-DPI screenshots. All4 Windows images inspected; visible label contrast restored, dense overlap remains. [Evidence](reviews/windows-m8-visualization-verified.md). Canvas/list is a fallback, not completed section26 Three.js. No native acceptance or promotion.

## 2026-10-31 — M8 visualization repair (Linux; independent Windows rerun pending)

On branch `muse/m8-visualization-repair` from
reviewer tip `5a050f18` (design committed before
code, record in
`docs/handoffs/M8-visualization-repair-design.md` and
`docs/handoffs/M8-visualization-repair-addendum.md`).
VIS-01 (P1): the visualization helper is renamed
via `git mv` to the genuinely distinct base name
`colliderVisualizationScene.ts` — the old name
differed from the component only in case and broke
the build on case-insensitive filesystems (TS2305,
Vite MISSING_EXPORT, UI suite 43 pass with 3 failed
file loads on Windows). All imports updated; no
casing checks disabled, no resolution settings
altered, no test weakened; folded-name scan of
tracked paths is empty. VIS-02 (P2): canvas labels
were drawn in the background color and invisible;
they are now restrained white over a dark backing
rect, verified in inspected screenshots at normal
DPI (paused), reduced motion, and deviceScaleFactor
2 (shipped as evidence). Canvas/list remains the
accessible fallback; Three.js and remaining §26
requirements stay unfinished and are not claimed.
Linux verification: node 105/105, tsc/build/
contracts clean, docs 204 files, focused browser
battery 5/5 (visualization, visualization
screenshots, document-input, url-input, pipeline
overview). No backend/schema/contract change;
Antigravity directories untouched. Windows rerun of
this repair (especially the case-insensitive build)
is pending. M8 is NOT complete.

## 2026-10-31 — visualization Windows review: changes required

Runtime `93cf77605` has a case-insensitive module-name collision: TypeScript/Vite fail and3 UI test files cannot load (43 individual tests pass). Canvas labels also use the background color by source inspection. [Review](reviews/windows-m8-visualization.md). Browser acceptance not run on stale assets; section26 Three.js remains deferred, not complete.

## 2026-10-31 — M8 visualization workflow (Linux; independent Windows rerun pending)

On branch `muse/m8-visualization` from reviewer
tip `795aab00` (Windows-verified URL repair 2
runtime; design committed before code, record in
`docs/handoffs/M8-visualization-design.md` and
`docs/handoffs/M8-visualization-addendum.md`).
Complete project-scoped workflow for an exact saved
Collider experiment version: pure derivation of
nodes (beams, understanding, every primitive,
mapping, combination, feasibility item, and
productization field) with section-flow-only edges
(no text-mined relationships), deterministic layout,
canvas + always-rendered text/list selection showing
exact source text and pinned provenance, reduced
motion static view with Pause/Resume otherwise,
identity-keyed stale-state clearing, zero API calls
(no inference from opening or interacting), render
as data (no innerHTML), and restart/backup-restore
reopen via the existing artifact path. No backend,
schema, or contract change; Antigravity directories
untouched. Material decision recorded honestly: the
renderer is Canvas 2D on the existing stack (no
Three.js in the repo, no package-manager path that
preserves the pinned install), behind a
renderer-agnostic scene model — the workflow is
delivered, the §26 Three.js renderer itself remains
outstanding and is not claimed. Linux verification:
node 105/105, 26/26 across 22 specs (collider, document-input, url-input, visualization, instance anchors, integrated acceptance, memory acceptance, memory context, picker reliability x5, pipeline overview, project context, review anchors, review document input, review rejected selection, transfer, transfer concurrency, transfer identity, forge, forge deliverable, forge review, forge revision, backups), strict
Clippy/fmt/tsc/contracts/docs/builds clean,
full core 405/4 (only the 4 known live-Ollama semantic_workflow environment failures), providers+API 144/0.
Windows rerun of this slice, Slice 4 Windows
verification, M7 native acceptance + preview
promotion, M5 gates and the §32 sandbox remain open.
M8 is NOT complete.

## 2026-10-31 — URL input repair2 verified on Windows

Runtime `d0a86d5e3` passes reviewer probes3/3, URL integration21/21, UI/contracts99/99, strict core all-target Clippy/fmt, TypeScript/contracts/docs, API/web builds and browser4/4 (URL plus document regressions, save/restart/restore). URL-01/02/02b closed in focused scope. [Evidence](reviews/windows-m8-url-input-verified-2026-10-31.md). No native acceptance, preview promotion, release installer or complete M8 claim; earlier red sections are historical.

## 2026-10-30 — M8 URL input repair 2: deadline beneath rustls (Linux; independent Windows rerun pending)

On branch `muse/m8-url-input-repair2` from reviewer
tip `239dc58e` (incoming `947b87b53`; design
committed before code, record in
`docs/handoffs/M8-url-input-repair2-design.md` and
`docs/handoffs/M8-url-input-repair2-addendum.md`).
Closes the remaining URL-02b finding: the first
repair's deadline sat above TLS record assembly, so
one dripped encrypted record bypassed it (2.228s
under a 500ms policy). The TLS and plain transports
are now a deadline-aware wrapper that re-checks the
deadline and shrinks the socket timeout before every
underlying socket read/write, so record assembly,
alerts, post-handshake traffic, and fragmented writes
are all bounded; reviewer probes preserved unchanged
and passing (`review_url_tls_record` 1/1 at 0.53s,
`review_url_fetch` 2/2). No timeout increased, no
UI-only workaround. Linux verification: url_fetch
21/21, lib 84/84, node 99/99, browser battery 25/25
across 21 specs, strict Clippy/fmt/tsc/contracts/
docs/builds clean, full core 405/4 (only the 4 known live-Ollama semantic_workflow environment failures),
providers+API 144/0. Windows rerun of this
repair is pending; Slice 4 Windows verification, M7
native acceptance + preview promotion, M5 gates and
the §32 sandbox remain open. M8 is NOT complete: 3D
remains deferred.

## 2026-10-30 — URL repair Windows follow-up

Runtime `947b87b53` passes original reviewer probes2/2, URL integration18/18, UI/contracts99/99, TypeScript/docs. Remaining URL-02b: fragmented bytes inside one encrypted TLS record bypass the total deadline; new synthetic probe fails. [Review](reviews/windows-m8-url-input-repair.md). No browser/native acceptance or promotion.

## 2026-10-30 — M8 URL input repair (Linux; independent Windows rerun pending)

On branch `muse/m8-url-input-repair` from reviewer
tip `db8d4cb93` (incoming `8cf6ff86f`; design
committed before code, record in
`docs/handoffs/M8-url-input-repair-design.md` and
`docs/handoffs/M8-url-input-repair-addendum.md`).
Closes the focused Windows security review's three
items with reviewer probes preserved unchanged and
passing: URL-01 globally-routable classification
(IPv6 allowlist over `2000::/3` minus special-purpose
exclusions, mapped/6to4 by embedded IPv4, mixed DNS
answers refused, literal encodings classified),
URL-02 one total deadline across DNS/connect/TLS/
writes/headers/bodies/trailers/redirects (20s
unchanged, every socket wait uses min(remaining,
10s), explicit TLS handshake loop), and the loopback
test allowance compile-gated to test/debug builds
(release builds never read the variable and are
unconditionally strict). Linux verification:
reviewer probes 2/2, url_fetch 18/18, lib 84/84,
node 99/99, browser battery 25/25 across 21 specs,
strict Clippy/fmt/tsc/contracts/docs/builds clean,
full core 401/4 (only the 4 known live-Ollama semantic_workflow environment failures), providers+API 144/0.
Windows rerun of this repair (including native
behavior) is pending; Slice 4 Windows verification,
M7 native acceptance + preview promotion, M5 gates
and the §32 sandbox remain open. M8 is NOT complete:
3D remains deferred.

## 2026-10-30 — URL input focused Windows review: changes required

Runtime `8cf6ff86` passes incoming URL suite8/8 and UI/contracts99/99; independent probes expose reserved IPv6 admission and missing total deadline inside dribbling response reads0/2. [Review](reviews/windows-m8-url-input.md). No browser/native acceptance, promotion or production certification.

## 2026-10-30 — M8 URL text input (Linux; independent Windows rerun pending)

On branch `muse/m8-url-input` from reviewer tip
`144acc715` (the reviewed tip of the document-input
repair2 slice; design committed before code, record in
`docs/handoffs/M8-url-input-design.md` and
`docs/handoffs/M8-url-input-addendum.md`). Explicit URL
entry plus a separate Fetch action for either beam:
backend-owned bounded public-only fetch (per-hop
validation, DNS-rebinding prevention by connecting to
the validated address, redirect/time/byte/text bounds,
TLS validation, plain + conservatively extracted HTML),
fetch saves nothing and starts no inference, only the
explicit Collide press sends the reviewed text, and
generation-time text + provenance survive save,
restart, approved Forge transfer, and backup/restore.
One correction during the slice: the saved-record port
restriction was removed (the port is a fetch-policy
matter; a saved record is inert text), since it refused
records the fetch service legitimately produces under
its test policy. Linux verification: url_fetch 8/8,
collider URL tests 4/4, lib 81/81, node 99/99, browser
battery 25/25 across 21 specs, strict Clippy/fmt/tsc/
contracts/docs/builds clean, full core 386/4 (only the 4 known live-Ollama semantic_workflow environment failures),
providers+API 144/0. Windows rerun of this slice
(including no real external site contact) is pending;
Slice 4 Windows verification, M7 native acceptance +
preview promotion, M5 gates and the §32 sandbox remain
open. M8 is NOT complete: 3D remains deferred.

## 2026-10-30 — document input repairs verified on Windows

Runtime `0c62533c` passes canonical document workflow and both unchanged reviewer probes3/3, UI/contracts96/96, TypeScript, contracts, docs and web build. DI-01/02/03 closed in focused scope. [Evidence](reviews/windows-m8-document-input-verified.md). Native picker acceptance, preview promotion and standing production gates remain open. Earlier red sections describe superseded checkpoints.

## 2026-10-30 — document repair Windows follow-up

Incoming `ef0b9843` closes DI-01/DI-02 under unchanged Windows probes. Canonical workflow passes; UI/contracts95/95, TypeScript/contracts/docs/web build pass. New DI-03: rejected oversized selection fails to invalidate an older pending read, which later overwrites the beam. [Evidence](reviews/windows-m8-document-input-repair.md). Changes required; no native acceptance or promotion.

## 2026-10-30 — M8 document input Windows review: changes required

Incoming `faafa1c26` has two frontend findings: file contents read before the size guard, and obsolete read rejection clearing a newer load's dispatch guard. Backend document tests 4/4, UI/contracts 94/94, TypeScript, strict core all-target Clippy, contracts, docs and builds pass. Focused evidence and reproduction: [review](reviews/windows-m8-document-input.md). No native acceptance or promotion.

## 2026-10-30 — M8 document input repair 2: rejected-selection invalidation (Linux; independent Windows rerun pending)

On branch `muse/m8-document-input-repair2` from reviewer
tip `d3f004bc` (incoming `ef0b9843`; DI-01/DI-02 closed
and preserved; design committed before code, record in
`docs/handoffs/M8-document-input-repair2-addendum.md`).
DI-03: every new document selection invalidates the
beam's load session before any early return, so a
rejected (oversized) selection retires a gated read
instead of letting it land afterward; the rejection
clears obsolete loading state and shows the current
error while the last applied beam and attribution stay
untouched. Before repair the new probe failed only its
identity assertion; after repair all three document
probes pass unchanged (rejected-selection, DI-01/DI-02,
canonical), affected browser battery passing, node 96/96,
collider 45/45, strict Clippy/fmt/tsc/contracts/docs/
builds clean. No backend, schema, or contract change.
Windows rerun of this repair (including native
file-picker behavior) is pending; Slice 4 Windows
verification, M7 native acceptance + preview promotion,
M5 gates and the §32 sandbox remain open.

## 2026-10-30 — M8 document input Windows findings repaired (Linux; independent Windows rerun pending)

On branch `muse/m8-document-input-repair` from reviewer
tip `97ab19b4` (incoming `faafa1c26`; design committed
before code, record in
`docs/handoffs/M8-document-input-repair-addendum.md`).
DI-01: oversized files are refused on `File.size` before
any read, with the decoder's post-read check retained.
DI-02: document load outcomes never reject; one
generation gate covers successes and failures, so an
obsolete failure changes neither error nor loading state
and cannot enable dispatch while a newer read pends.
Reviewer probe unchanged: before repair 1 failed (both
finding assertions); after repair 1/1 passed including
save/restart/backup-restore. Canonical document spec 1/1,
affected browser battery passing, node 95/95, collider
45/45, strict Clippy/fmt/tsc/contracts/docs/builds clean.
No backend, schema, or contract change. Windows rerun of
this repair (including native file-picker behavior) is
pending; Slice 4 Windows verification, M7 native
acceptance + preview promotion, M5 gates and the §32
sandbox remain open.

## 2026-10-30 — M8 document input vertical slice (Linux; independent Windows rerun pending)

On branch `muse/m8-document-input` from reviewer tip `c50d6b03`
(incoming `06a3f573a`, the Windows-verified lineage runtime;
design committed before code in
`docs/handoffs/M8-document-input-design.md`, record in
`docs/handoffs/M8-document-input-addendum.md`). Explicit
`.txt`/`.md` document input for either Collider beam through
the existing M5 machinery: strict UTF-8 decode (one leading
BOM stripped and recorded, no other normalization), file ≤
65,536 bytes, text 1–8,000 characters, backend-recomputed
label/kind/hash/count provenance pinned on the saved
experiment, forged or stale records refused before provider
traffic, labels/hashes never sent to the provider, editing a
loaded beam clears its attribution, race-safe per-beam loads
(generation counters; stale reads dropped), same-name
reselect and clear/retry covered. No schema change; old
experiments without document fields restore unchanged.
After (sequential, Linux): collider 45/45, transfer 19/19,
forge_restore 4/4, backup_restore 7/7, backup_artifacts 5/5,
opportunities_restore 3/3, entities_roundtrip 7/7, lib 76/76,
node 94/94, 24 affected browser tests passing (including the
new canonical document-input spec: review → race → reselect
→ rejections → collide → save → restart → backup/restore),
strict Clippy/fmt/tsc/contracts/docs (182 files)/builds
clean, full core 369/4 (only the known live-Ollama
semantic_workflow environment failures), providers+API 144/0.
One regression found and fixed in-slice: the first file-input
labels collided with beam labels in existing specs
(renamed to first/second beam wording; no existing spec
touched). This does not close Windows verification of this
slice (native file-picker behavior in particular), Windows
verification of Slice 4, M7 native acceptance + preview
promotion, M5 gates, G1 audit, or the §32 sandbox. URL
fetching, other document formats, OCR/transcription and 3D
remain unfinished.

## 2026-10-30 — M8 lineage repair passes focused Windows acceptance

Incoming runtime `06a3f573af1cda51dab9fa1e0959b93c5897fd6a`; [independent Windows acceptance](reviews/windows-m8-lineage.md). All five prior reviewer probes pass, including resumed work after restore; extended A→B→C→D→E chain passes. Core library 74/74, vault increment-2 9/9 (both legacy/current crash matrices), genuine API-produced v10 archive import 1/1, live migration 1/1, UI/contracts 90/90 and browser 4/4 pass; strict Clippy, TypeScript/contracts/fmt/docs and API/web builds pass. 91 scoped Rust cases passed in total; no full-workspace/native/release claim. The real v10 archive preserves both memory/artifact versions on schema11 restore, stays byte-identical and refuses duplicate restore. Reviewer adds only optional fixture acceptance and docs; no runtime implementation, cloud, CI, pushes or preview promotion. Old missing-history archives retain their documented fail-closed limitation. M8 remaining feature and production gates stay open; continue from this verified runtime rather than restarting completed R1–R4 repairs.

## 2026-10-30 — R1 focused Windows probes pass; resumed-work lineage recovery remains blocked

Incoming `b6a3251c4d6fe68c97ca57b461e60f44586850bb`; [Windows provenance review](reviews/windows-m8-r1-provenance.md). All four prior reviewer probes pass, restore unit tests 10/10, UI/contracts 90/90, browser 4/4 and relevant strict/static/build checks pass. One additive valid-workflow probe fails: restore A→B, create new context work in B from a restored structured source, restore B→C successfully, then restore C→D refuses. Ultimate-origin compression drops intermediate identities still named by the B-era snapshot. This is new-code normal use, separate from the documented old-code compatibility limit. Preserve all R1–R3 fixes; Muse owns bounded lineage repair. Reviewer changes only add a probe/docs; no runtime code, cloud, CI, pushes or preview promotion.

## 2026-10-30 — M8 repair Windows review: R2/R3 pass; R1 remains open

Incoming `d0a6c1e1dd8612c954e9aceee9d735f5653780c8`; [independent Windows repair review](reviews/windows-m8-overnight-repair.md). Original Rust probes 2/2 and browser reviewer/instance/project/integrated specs 4/4 pass; source boundary 7/7, UI/contracts 90/90 and static/build checks pass. Two added generic-revision attacks remain accepted by restore: missing optional project context skips the final immutable comparison, and unknown-old/present-new IDs are accepted without actual restore-origin mapping (including a never-restored package). R2 final-hash ordering and R3 source-scoped navigation are verified in focused scope; R1 and promotion stay blocked. Reviewer changes only add tests/docs; no runtime implementation, cloud, downloads, pushes or CI.

## 2026-10-30 — M8 lineage repair: resumed-work multi-generation recovery repaired (Linux; independent Windows rerun pending)

On branch `muse/m8-lineage-repair` from reviewer tip `e03c9bde`
(incoming `b6a3251c` plus reviewer probe/docs only; design committed
before code in `docs/handoffs/M8-lineage-repair-design.md`, record
in `docs/handoffs/M8-lineage-repair-addendum.md`). Restore now
writes the full alias set (latest hop plus every prior generation
from the source package's own origins rows) under a widened
`restore_origins` key `(workspace_id, new_id, original_id)`;
schema 10 → 11 with a v10 legacy verification path that migrates
only the staging copy. Validation is unchanged (exact pair
lookup), so A-era, B-era and C-era snapshots all validate while
wrong/bogus/foreign pairs still refuse. Before: reviewer probes
4/5 (R4 fails at C → D). After (sequential, Linux): reviewer
probes 5/5, collider 41/41, transfer 19/19, forge_restore 4/4,
backup_restore 7/7, backup_artifacts 5/5, opportunities_restore
3/3, entities_roundtrip 7/7, lib 74/74, node 90/90, 23 affected
browser tests passing, strict Clippy/fmt/tsc/contracts/docs/
builds clean, full core 363/4 (only the known live-Ollama
semantic_workflow environment failures; the first broad run's
seven schema-bump failures — vault version gates and
current-version assertions — were fixed in a follow-up commit
and the suites re-run), providers+API 144/0. Pre-R1b multi-hop
packages remain
fail-closed (see addendum); a real v10 backup restore end to end
is a Windows checklist item. This does not close Windows
verification of this repair or of Slice 4, M7 native acceptance +
preview promotion, M5 gates, G1 audit, or the §32 sandbox. No
cloud, credential, download, push, merge, CI, or promotion.

## 2026-10-30 — M8 R1 provenance repair: remaining two probes repaired (Linux; independent Windows rerun pending)

On branch `muse/m8-r1-provenance-repair` from reviewer tip `7388b7fa`
(incoming `d0a6c1e1` plus reviewer probes/docs only; design committed
before code in `docs/handoffs/M8-r1-provenance-repair-design.md`,
record in `docs/handoffs/M8-r1-provenance-repair-addendum.md`). R1a:
no transform branch returns success early — absent optional context
now falls through to the final whole-document equality. R1b: a
differing id is allowed only on an exact backend-recorded
`restore_origins` pair for the owning workspace; package membership
alone never authorizes. Restore records ultimate origins, so 2nd/3rd/
Nth restores validate by single pair lookup; no schema change.
Before: reviewer probes 2/4 (original two pass, two new fail).
After (sequential, Linux): reviewer probes 4/4, collider 39/39,
transfer 19/19, forge_restore 4/4, backup_restore 7/7, restore lib
10/10, node 90/90, 23 affected browser tests passing, strict
Clippy/fmt/tsc/contracts/docs/builds clean, full core 357/4 (only
the known live-Ollama semantic_workflow environment failures),
providers+API 144/0.
Compatibility (fail-closed): packages restored twice or more under
old code carry only latest-hop origins, so a further restore under
new code refuses before publication; packages produced by new code
carry ultimate origins. This does not close Windows verification of
this repair or of Slice 4, M7 native acceptance + preview
promotion, M5 gates, G1 audit, or the §32 sandbox. No cloud,
credential, download, push, merge, CI, or promotion.

## 2026-10-30 — M8 overnight Windows repair: R1–R3 repaired (Linux; independent Windows rerun pending)

On branch `muse/m8-overnight-windows-repair` from reviewer tip `240af4ef`
(incoming `edba7ed3` plus reviewer probes/docs only; design committed
before code in `docs/handoffs/M8-overnight-windows-repair-design.md`,
record in `docs/handoffs/M8-overnight-windows-repair-addendum.md`). R1:
restored-hash provenance is no longer trusted — pristine rows must be
byte-identical to the snapshot and diverged rows must be a
package-verified allowed transformation (id substitutions and pinned-hash
rewrites only). R2: all pinned-hash fix-ups run to a bounded global
fixpoint, so a selected concept pins its final restored hash after
transfer fix-ups and a same-version cycle refuses at the cap. R3: result
headings, overview links and the collider disclosure are instance-scoped
(live keeps legacy ids; saved uses `saved-{artifactId}-v{version}`), with
the pipeline heading itself left legacy as a documented, non-target
exception. Before: Rust reviewer probes 0/2, browser reviewer probe 0/1.
After (sequential, Linux): reviewer probes 2/2 + 1/1, collider 37/37,
transfer 19/19, forge_restore 4/4, backup_restore 7/7, node 90/90, 23
affected browser tests passing, strict Clippy/fmt/tsc/contracts/docs
(173 files)/builds clean, full core 349/4 (only the 4 known live-Ollama
semantic_workflow environment failures), providers+API 144/0. This does
not close Windows verification of this repair or of Slice 4, M7 native
acceptance + preview promotion, M5 gates, G1 audit, or the §32 sandbox.
No cloud, credential, download, push, merge, CI, or promotion.

## 2026-10-30 — M8 overnight Windows review: progress verified; three findings block acceptance

Incoming `edba7ed32db2f7c5ea999466061c4e1c3966b2e7`; [independent Windows review](reviews/windows-m8-overnight.md). Current-source boundary 7/7 and UI/contracts 89/89 pass; strict Clippy, formatting, TypeScript/contracts/docs and API/web builds pass. Main integrated browser and project-context specs pass. Two additive restore probes fail: editable restored-hash metadata admits forged provenance, and a selected transferred concept receives a stale hash before a later fix-up, breaking the second restore. Additive browser probe fails: saved overview anchors resolve into the mounted live result. Acceptance/promotion stays blocked; runtime unchanged by reviewer. Preserve all original assertions and use the reviewed branch for Muse's repair. No cloud, credentials, downloads, pushes or CI.

## 2026-10-30 — M8 overnight hardening: context boundaries, picker reliability, pipeline overview (Linux; Windows pending)

On branch `muse/m8-overnight-hardening` from Slice 4 tip
`0b6ba0e8` (WP1 criteria `da9a10ea`, WP2 criteria
`f60480da`, WP3 criteria `ee359c5c`; full record in
the WP1–WP4 acceptance/addendum files). WP1: a new
collision now re-checks current source eligibility —
pinned revisions whose head was archived, sealed,
expired or reassigned refuse before provider traffic in
both pickers, while eligible older revisions still
dispatch and saved history is untouched; experiments
pinning structured artifacts now survive repeated
restore via a recorded `restoredContentHash` on
artifact items and a fix-up pass wired before the
transfer fix-ups (fixpoint for chained experiments),
with validation accepting the original or the recorded
restored hash and refusing fabricated ones. WP2: the
project picker pages memory lists past the 50-row
default, shows list failures as errors with retry,
shows privacy labels in both pre-send reviews, disables
cross-picker duplicates and 9th items with notes,
blocks overlong reviews with the bound named, retries
failed reads without losing the selection, and counts
hidden items. WP3: a pipeline overview (no new
dependency) shows only real run states, the six
sections with their relationships, section links when
complete, a static diagram plus full text list, and a
provenance link; 3D stays unfinished (no visualization
dependency; SwarmTank 3D is M9). WP4 integrated browser
spec chains selection → review → collision → save →
approved transfer → restart → backup/restore → reopen
→ second restore with snapshot and unapproved-brief
negatives. Evidence (sequential, Linux): focused
collider 34/34, transfer 19/19, forge_restore 4/4,
backup_restore 7/7, node pass, strict Clippy/fmt/tsc/
contracts/docs clean, API/web builds pass; 21 affected
browser tests pass (7 collider specs, transfer ×3,
forge ×4, backups, export); full core with
`--no-fail-fast` 345 passed / 4 failed (only the known
live-Ollama semantic_workflow environment failures;
334 + 11 new tests); providers+API 144/0. Windows
verification of Slice 4 is still in progress and
Windows verification of this checkpoint has not
started; Linux evidence must not be presented as
closing either. No cloud, credential, download, push,
merge, CI, or promotion. Details:
[M8 WP1 criteria](handoffs/M8-overnight-hardening-wp1-acceptance.md),
[M8 WP2 criteria](handoffs/M8-overnight-hardening-wp2-acceptance.md),
[M8 WP3 criteria](handoffs/M8-overnight-hardening-wp3-acceptance.md),
[M8 WP4 record](handoffs/M8-overnight-hardening-wp4-addendum.md).

## 2026-10-30 — M8 Slice 4: explicitly selected saved-project collider input (Linux; Windows pending)

On branch `muse/m8-slice4-project-context` from `c7327bb7`
(design `4bfda48f` committed before coding; the branch also carries
the `c14ce920` free-model comparison record). A collider in an
existing project can now explicitly select a saved project from the
current workspace and exactly which of its items to send: member
memories at pinned revisions and artifacts at pinned versions, with a
pre-send review of exact versions and generation-time provenance on
the saved experiment. Resolution is backend-only and immediately
before dispatch: Sealed, foreign-workspace, other-project, archived,
corrupted, duplicate and approval-row inputs refuse before provider
traffic; a memory selected through both pickers refuses as a
duplicate. Restore pre-validates every project item against
source-package rows and remaps project, memory, revision and artifact
ids fail-closed; no new transfer fix-up was needed and the new
transfer test proves a project-context concept survives restore and
stays Forge-admissible. `memory_list` gained a server-side
`projectId` filter. Focused evidence (sequential,
`--test-threads=1`): collider 24/24 (baseline 19/19), transfer 19/19
(baseline 18/18), forge_restore 4/4, backup_restore 7/7, node
UI/contracts 83/83 (baseline 81/81); strict Clippy, fmt, tsc,
contracts check, docs validation, API/web builds pass; the new
project-context e2e spec passes standalone. Full canonical e2e
battery (all 29 specs): 34 passed / 1 failed; the single failure is
the pre-existing real-semantic search.spec failure, proven identical
on a pristine `c7327bb7` worktree (same assertion, same line), not a
Slice 4 regression. Full core 334 passed / 4 failed (A6 baseline 328
+ 6 new Slice 4 tests; failures only the 4 known live-Ollama
environment failures); providers+API 144/0.
URL fetching, document ingestion and 3D visualization remain
deferred; M8 is not complete; Windows verification of Slice 4 is
pending and Linux evidence must not be presented as closing it. No
cloud, credential, download, push, merge, CI, or promotion. Details:
[M8 Slice 4 design](handoffs/M8-slice4-project-context-design.md),
[M8 Slice 4 record](handoffs/M8-slice4-project-context-addendum.md).

## 2026-10-30 — Windows findings verified; native verification integrated

Implementation `4ef90406d8726757f225e75a36e3bf57e01ebde6` on
`codex/review-ask-windows-findings`, from Muse's `c15fe5d6` repair.
[Exact Windows evidence and limits](reviews/windows-ask-findings-verified-2026-10-30.md).
All four findings pass focused Windows checks. Native verify-only integration
removes permission-helper launches: unchanged 16-note Ask is 2,264 ms (incoming
6,299; pre-batch 27,637). Full realistic corpus passes: 68/68 answerable, 80/80
bindings, both six-case negative families, disclosure/budget 80/80, no leaks,
Windows p95 4,383.3 ms below original 5,000 ms bar. Raw prompts/audit retained.
Accounting 0/1/16/128/1010 notes passes with zero helper launches.
98 distinct Rust tests, strict Clippy, UI/contracts 81/81, Python 8/8,
TypeScript/contracts, builds and browser Ask 1/1 pass. No new model answers/token
measurements, native preview promotion, cloud calls, pushes or CI. Existing
full-core A6/Ollama failures remain scoped and recorded.

## 2026-10-30 — A6 reporting correction: offline and wire records split

On top of `9cff10c2` on branch `muse/ask-a6-repair`.
A6's single mixed record is split into A6-offline
(retrieval/disclosure/offline-behavior checks only, no
provider hits) and A6-wire (fake-provider plumbing:
`Answered` is the canned fake response, and the check
covers only that the captured request carries the
abstention instruction). Each record holds only its own
request's measurements. The offline checks do NOT verify
connected-model refusal. Report wording corrected: the
old traffic-gating assertion was replaced with narrower,
model-independent checks, not preserved. Focused tests
(`benchmark_tokens` 1/1, held-out 5/5), formatting, and
strict core Clippy pass; held-out cases and production
code untouched; no scale runs, cloud, downloads, or
promotion.

## 2026-10-30 — A6 benchmark repair: assertion re-anchored, no production change

From verified integration `151565d7` on branch
`muse/ask-a6-repair` (acceptance frozen first in
`ask-a6-acceptance.md`). The A6 failure was the test's
stale proxy, not production: repaired OR retrieval
legitimately returns partial Harbor matches for the
absent berth-fee fact, so `InsufficientEvidence + zero
hits` (a pre-repair AND artifact) failed even when
honesty held. A6 now grades the model-independent offline
outcome (never Answered; abstention instructed; no
"berth" in any citation; disclosure exactly the cited
set; prompt under 5,500 chars). New held-out test
`ask_partial_evidence_acceptance` 5/5 (absent,
shared-vocabulary, complete, conflicting, excluded).
`benchmark_tokens` 1/1; focused suites, Clippy/fmt/
contracts/tsc/docs, API/web builds, browser Ask 1/1 pass;
full core 328 passed / 4 failed (only the known
live-Ollama environment failures; A6 gone). Previous
"byte-identical audit" wording corrected: audit results
match semantically, files differ in line endings
(LF vs CRLF). Preview launcher and data roots untouched;
no cloud, credential, download, push, merge, CI, or
promotion. Model-answer quality still unmeasured; the
traffic-gating question stays an open owner decision.

## 2026-10-30 — Verified integration: regressions green, quality unmeasured

From reviewed `532afccf9333522c3e15c97cd42105c71807bf35` on branch
`muse/ask-verified-integration` (integrator `4ef90406d` preserved).
Linux: focused suites, Clippy/fmt/contracts/tsc/docs, API/web builds,
browser Ask 1/1, full core 322 passed / 5 failed (same 4 live-Ollama
environment + 1 pre-existing benchmark A6 as last round, no new
failures), providers+API 144/0. Windows evidence audit reproduced
semantically (all fields equal; files differ in line endings,
LF vs CRLF, so not byte-identical) (68/68, 80/80 bindings,
p95 4,383.3 ms). Frozen cloud round
inspected unchanged, still 0 attempts. Local quality comparison
BLOCKED: no model installed (Ollama blobs empty, no .gguf, no
llama.cpp); criteria frozen in `ask-local-quality-acceptance.md`,
no quality or token claim made. Isolated preview checkpoint
prepared (new launcher + handoff, fresh data root); owner's
running preview untouched, nothing launched, no Windows/native
acceptance claimed from Linux.

## 2026-10-29 — Windows findings repair: widen-after-proof closed natively

From reviewed `6e355b02d5aa46f1aa5646b674edb01cd6ec2884` on branch
`muse/ask-windows-findings-repair`. R1: proven reads re-validate current
owner/DACL natively in-process on Windows (new `windows_security.rs`,
`windows-sys` 0.45.0 already in the lockfile) for the file and its proven
ancestor directories, matching the `protect.ps1` invariant with no per-file
process; the reviewer's refusal reproduction keeps its assertion and is now
a standing regression test. R2: accounting expectations split by platform
(Windows singles stay 8; Unix singles equal batch paths plus 8) with no
counter inflation. R3: Windows-only Clippy dead code gated by platform
(`BatchPath`, `REC3`/`REC4`; the recheck function is now used on Windows
too), no suppressions. R4: realistic-corpus binding helper searches all
matching chunks, plus a pure two-occurrence regression; no question or gold
fact changed. A6 benchmark invariant unchanged and documented as
partial-retrieval versus genuinely-empty. Linux: content 7/7, batch-proof
4/4, accounting 1/1, realistic 2/2 (68/68), large-scale 2/2, Clippy/fmt/
contracts/tsc/docs clean, API/web builds pass, browser Ask 1/1, full core
322 passed / 5 failed (4 live-Ollama environment failures, 1 pre-existing
benchmark A6; run split by a VM replacement, both logs preserved). The
Windows module compiles and passes strict Clippy for the Windows target;
the full-crate Windows check needs the reviewer's MSVC toolchain (this
machine lacks `lib.exe` and mingw). Windows rerun (refusal test, probe,
accounting, Clippy, then full scale) decides acceptance.

## 2026-10-29 — Scale hardening Windows review: faster, acceptance still open

Incoming `825f75614656aa0118d924fc5ae1fe51b3a9cf7e` verifies against its ZIP/bundle
identities. [Windows results and concrete repair requirements](reviews/windows-ask-scale-hardening-2026-10-29.md).
The unchanged 16-note probe improves from 27,637 to 6,910 ms with correct evidence.
Content adversarial checks 4/4 and source-binding regression 1/1 pass on Windows.
An explicit permission-widen-after-proof refusal reproduction FAILS; the incoming
accounting test FAILS at one note due to a Linux-only count assumption; strict
all-target Clippy FAILS on Windows-only dead code. Realistic-corpus source-binding
helper also has a static early-return defect. Full-scale Windows acceptance is
deferred; no new model answers, token measurements or preview promotion claimed.
Review adds an opt-in failing reproduction and docs only, no runtime changes.

## 2026-10-29 — Windows review: retrieval package verified, performance defect found

Incoming `f90f9e850` preserves the app repair; checksum, ancestry and all 30 frozen
scored payloads verify. [Windows review and repair requirements](reviews/windows-ask-retrieval-verify-2026-10-29.md).
A public-path Windows diagnostic on 16 notes retrieves the correct fact but takes
27,637 ms for one Ask. Rebuilding the full workspace and launching per-path
PowerShell ACL checks is an existing scaling defect. Full 1,007-record Windows
acceptance was not run or claimed. Strict Clippy for both new tests passes.

The larger test also needs source-bound fact assertions and preserved overlapping
negative cases alongside its revised term-disjoint probes. Linux 40/40 retrieval
evidence remains scoped; model-answer quality and actual token savings are still
unmeasured on repaired prompts. No production code, preview or provider calls
changed in this review. Next task is bounded Windows performance/scorer repair.

## 2026-10-29 — Ask scale hardening (permission accounting, proof race repair, realistic corpus)

Branch `muse/ask-scale-hardening` from `9b9ead8f` (that checkpoint
preserved untouched). [Full report](handoffs/ask-scale-hardening-report.md).
One Ask accounts for every permission check across 0/1/16/128/1,010
notes: batches stay 1, outside-batch work stays 8, batch paths grow
2N + 2; Linux spawns no permission processes. Adversarial tests
found a real widen-after-proof race; proven reads now re-validate
file permissions at read time on non-Windows (Windows keeps a
documented one-operation residual window). Realistic corpus:
1,008 records, 4 domains, 80 questions — 68/68 binding-complete,
6/6 disjoint, 6/6 natural negatives, disclosure and budget 80/80,
zero leaks, query p95 366.8 ms (Linux). Windows rerun of the probe,
accounting diagnostic, and full-scale corpus decides acceptance;
cloud replay still held at 0/31 attempts.

## 2026-10-29 — Ask Windows scale repair (batch proofs, fact binding)

Branch `muse/ask-windows-scale-repair` from reviewed checkpoint
`8bdcbae1`. [Full report](handoffs/ask-windows-scale-repair-report.md).
Ask now shares one operation-scoped batch permission proof between
index rebuild and compilation (Linux 16-note Ask: 1 batch, 34 paths,
42 individual checks, versus about 4N before); all ACL, link, hash,
and privacy checks retained, no permanent cache, no rebuild removed.
Large-scale scorer now binds each required span to its declared
source's actual evidence chunk (40/40), with a wrong-source
regression and a retained 4-question natural negative family (4/4).
Windows timing is NOT claimed from Linux: the reviewer's 16-note
probe rerun decides. Cloud replay still held at 0/31 attempts.

## 2026-10-29 — Ask retrieval verified at larger scale; model replay held

Branch `muse/ask-retrieval-verify` from `95d384e3`. [Full report](handoffs/ask-retrieval-verify-report.md).
Large-scale acceptance (criteria committed before implementation): 1,007 records,
48 fresh questions, answerable completeness 40/40, insufficient 8/8, disclosure
48/48, budget 48/48, zero Sealed/foreign/archived/expired leaks. No production
code changed. Model replay on the repaired fixture is HELD: 30 payloads frozen
from the hash-verified fixture, Nemotron free pricing verified publicly, but
`OPENROUTER_API_KEY` was absent in the session, so 0/31 attempts were spent.
No new answer-quality or token-saving claim. Windows acceptance still pending.

## 2026-10-29 — Ask recall repaired and locally verified

Implementation `24d3df5c7cf7a90f849f7d16c08c9fe4fd3841dd` on
`codex/ask-retrieval-recall`. [Exact evidence and limitations](reviews/ask-retrieval-recall-repair-2026-10-29.md).
The unchanged 100-case fixture now supplies every required fact for 90/90 answerable
questions (previously 50/90), with all ten genuinely empty cases still empty and
no evidence regressions. Combined prompt bytes fall from 175,658 to 158,999 (9.48%).
This is local retrieval evidence, NOT a new answer-accuracy/token measurement.

82 focused Rust tests, 2/2 full offline fixture checks, 8 Python tests, strict
all-target Clippy, formatting, TypeScript/contracts, web/API builds and the
strengthened canonical browser Ask workflow pass on Windows. Sealed/foreign,
lifecycle, current-revision, integrity, budget and exact Search boundaries retained.
No new cloud calls, preview promotion, pushes or CI. Prior LinkedIn claim stays on
hold pending model-quality/usage verification. Historical measured results follow.

## 2026-10-29 — Larger free-model benchmark completed; quality gap measured

100 controlled question instances across ten templates, three approaches, 30 scored
batch requests plus one smoke, exact free Nemotron Ultra alias, zero reported cost.
Measured protocol/source commit `21b1afb0ba1a7969825d39c61a3745bfc4c29e40` on
`codex/benchmark-large-free`; no application code changed.

ANANSI: 60/100 supported correct outcomes, 46,339 input + 12,467 completion = 58,806
reported tokens. Basic retrieval: 70/100, 50,479 + 13,305 = 63,784. Full context:
100/100, 228,145 + 16,277 = 244,422. ANANSI input reduction is 79.69% versus full
but only 8.20% versus basic, with lower quality than both. The 40 ANANSI misses
are refusals on answerable questions after candidate retrieval omitted the facts.
Do not present these savings as an unqualified efficiency win or as an improvement
over the differently defined earlier pilot baseline. Prior LinkedIn draft is on hold.

Read [full measured review and scope](reviews/large-free-benchmark-2026-10-29.md).
All 30 calls were HTTP 200, finish stop, parseable; all 300 items and raw responses
retained. No scored reasoning>completion anomaly; old pilot anomalies remain open.
100-case offline fixture checks 2/2, Python regressions 8/8, strict harness Clippy,
formatting, docs and Git validation passed. Artifacts are on E:. Next work should
improve retrieval and validate on held-out questions, not automatically rerun cloud
experiments. Production/native/audit/CI gates and the existing previews are unchanged.

## 2026-10-29 — Independent benchmark review (qualified measurement verdict)

Incoming `8639c44c30eb98848961dfd31fe1057e769d53e4`, reviewed on
`codex/review-m8-benchmark` in `E:/Anansi/worktrees/review-m8-benchmark`.
The reporting-only correction is `655c4f615`; no application behavior changed.
Read [review and measurement qualifications](reviews/windows-m8-benchmark.md)
before reusing the historical Linux counts or token headlines below.

Windows focused checks passed: context 10/10; workspace roundtrip 11/11 including
20×16 concurrent migration stress; Ask 5/5; streaming/cancellation 1/1; synthetic
Ask browser 1/1; UI/contracts 81/81; TypeScript, web/API builds, contracts drift,
strict core all-target Clippy. Reporting CLI regressions 2/2. No full Windows
suite or real-model replication is claimed. Supplemental synthetic benchmark
harness was reviewer-stopped after more than 25 minutes without a terminal result;
coverage is incomplete, with no invariant verdict. See review and attempt log.

Offline sums reproduce 37.1% lower API-reported input and 30.9% lower reported
total tokens, on eight questions/32 responses. Eight reasoning subcounter
inconsistencies remain unresolved; the largest case contributes 70.5% of reported
savings. Without it, input reduction is 19.4%, reported-total reduction 14.1%.
Raw records preserved. The supplied final Linux log is 305 core plus 24 API
passes, not 329 core passes. Public draft restricts its claim accordingly.

Existing native acceptance/promotion, §32, external audit and production/CI gates
remain open. No cloud calls, downloads, pushes, CI, publishing or preview changes.

## 2026-10-28 — M8 benchmark: automated Linux verification (real-model phase pending)

Benchmark branch `muse/m8-benchmark-tokens` on reviewed baseline `df807259`
(D1 disclosure-usage repair included; harness semantic extension + lint
repair verified before commit, production code unchanged from `7ec90dcc`).

Sequential on an idle machine, with `all-minilm:22m` installed locally:
clean full core 299/300 (46 binaries) — the single failure is an
intermittent concurrent-open v1-migration race in `workspace_roundtrip`
(10/10 on alone rerun, not root-caused, recorded as intermittent, no
assertion weakened); `semantic_workflow` 4/4 with the embedding model
(0/4 before install, reported separately). Providers+API 144/0. Node
UI/contracts 81/81, TypeScript and contracts clean. Strict Clippy and
formatting clean after a harness-only lint repair (4 lints in
`benchmark_tokens.rs`, fixed without behavior change; a first fix attempt
used a single-pattern match flagged as `single_match`, replaced by a
let-chain). Docs validator passes (138 files). API and web builds pass.
Browser: canonical 17/17 plus non-canonical 17/17 = 34/34 across 28 specs,
including the reviewer's memory-consent probes (2/2) and the
semantic-search workflow; the only browser exclusion is the `ask_anansi`
real-model branch, which hardcodes `qwen2.5:0.5b` and is not used as
largest-model evidence.

Coverage matrix Linux results filled in
[M8 benchmark coverage matrix](handoffs/M8-benchmark-coverage-matrix.md).
Real-model measurements, corpus expansion, findings report and final
delivery remain pending; no savings headline is claimed. Linux evidence
cannot certify Windows native dialogs, credential storage, desktop
acceptance, external audit, cloud providers or remote CI.

## 2026-10-28 — M8 Slice 3 focused Windows acceptance, with local corrections

Incoming `776259a49819a0166e8199dc85e29b34cf452680`, reviewed baseline
`4d3f8bfd`, ZIP/bundle identity and all four refs verified. The delivered binaries
passed Collider 19/19 and transfer 18/18 on Windows, including context-bearing
transfer restore followed by Forge admission. This is not a full final-tree rerun.

Additional expected-success probes exposed two boundary gaps: clicking Collide
re-read the current memory head and sent unseen C after the owner reviewed B;
restore pre-validation accepted fabricated revision version/title/privacy labels.
Local correction `f54e46695e7aacbeca5676c11060bed4f04a6c1a` submits reviewed
immutable references, blocks pending/error/stale reviews and duplicate starts,
invalidates selection/unmount continuations, and checks source revision labels
before restore remapping/publication. No schema/DTO/provider path changed.

Corrected implementation: restore-binding units 2/2, Forge restore 4/4, UI/contracts
81/81, TypeScript, contracts drift, web/API builds, strict core all-target Clippy,
formatting and docs passed. Confirming browser 3/3 under the shared lease: reviewed
B reaches the wire despite newer C, a held old-project review cannot populate or
transmit in the new project, context-free collision still works, and the complete
memory-context save/edit/restart/fresh-restore workflow passes. Diagnostic browser
failures and root-caused harness corrections (exact project identity and selected-
memory gate cleanup) are preserved; no assertion or time limit was weakened.

Use `codex/review-m8-slice3`, including the local fix and test commits, as the next
M8 baseline. [Exact Windows scope/evidence](reviews/windows-m8-slice3.md).
AN-013 remains in progress. Native acceptance/promotion, remaining M8 inputs/3D,
execution and production gates remain open. Existing preview/data untouched;
all new work/cache/temp on E:; no pushes, CI, real credentials or paid inference.

## 2026-10-28 — M8 Slice 3: explicitly selected memory context (Linux delivery evidence)

Slice 3 on branch `muse/m8-slice3-memory-context`, from the
Windows-reviewed baseline `codex/review-m8-acceptance-repair` @
`4d3f8bfd` (including correction `5b3cf958f`; review ZIP/bundle
hashes and ancestry verified before work). The collider panel in an
existing project now offers an optional picker for memories in the
current workspace: nothing selected automatically, selecting or
reviewing starts no inference, and a pre-send review shows exactly
which memory revisions will be sent. The client submits references
only (at most 8, each at most 8000 chars, 32000 total); the backend
resolves each pair through workspace-scoped, revision-pinned,
ownership-checked, hash-verified reads that refuse Sealed content
before any provider traffic, sends the resolved entries as
`private` data segments in selection order, and saves exact
generation-time provenance with the experiment (older experiments
stay readable). Restore pre-validates original memory/revision
bindings fail-closed, remaps references, and a new
`fix_transfer_experiment_hashes` fix-up carries concept and receipt
experiment hashes to restored rows. The slice's transfer test caught
a pre-existing restore fix-up ordering bug (receipt pinned the
concept hash before the concept body was rewritten, leaving a stale
receipt whenever an experiment hash changed on restore); the fix
reorders the fix-ups and is covered by Forge admission succeeding
on the restored workspace.

Linux verification, sequential on an idle machine: collider 19/19,
transfer 18/18, full core 294/298 with only the 4 known
environmental live-Ollama semantic_workflow failures; providers+API
144/0; node UI/contracts 81/81; TypeScript, contracts drift, strict
Clippy, formatting clean; API+web builds passed. Browser canonical
battery 17/17 including the new collider-memory-context spec
(selection → pre-send review → collision → save → restart → fresh
restore). Design:
[Slice 3 design](handoffs/M8-slice3-memory-context-design.md);
record:
[Slice 3 addendum](handoffs/M8-slice3-memory-context-addendum.md).
Windows verification of Slice 3 pending; AN-013 remains in
progress; M8 is not complete. No preview promotion, native
acceptance, §32 sandbox or production gates claimed.

## 2026-10-27 — M8 repair accepted in focused Windows checks, with local integrity correction

Incoming `1ccda62d92ae800b6730ee01aee9e8eac3a17b68` passed all 16 transfer
cases and the retained exact-beam probe on Windows. All six earlier backend
acceptance probes remained unchanged. A new receipt-checksum corruption probe
then exposed an extra integrity gap (ordinary read refused, retry/Forge accepted,
one synthetic provider hit). Local implementation
`5b3cf958f9c35e45c9226d3988ae82b414b72d13` verifies receipt body hashes in the
shared SQL reader before retry or admission; the same probe now passes with zero
hits. No schema/contract/approval semantics changed and no stored data was repaired.

Final implementation browser checks passed 4/4 under the shared lease, using the
canonical Windows config and unchanged assertions/limits: Collider cancellation,
save/restart/restore; independent experiment/draft identity; two-process atomic
transfer; complete transfer/Forge/restart/restore. UI/contracts 78/78, TypeScript,
contracts drift, web/API builds, strict core all-target Clippy, formatting and docs
passed. The full 16-case transfer suite was run before the local correction, not
relabeled a final-tree 17-case run. Exact scope and commands:
[Windows repair review](reviews/windows-m8-acceptance-repair.md).

Use branch `codex/review-m8-acceptance-repair`, including the local correction, as
the next M8 source baseline. This supersedes the focused acceptance hold below;
AN-013 remains in progress and no Slice 3 is implemented/authorized by this review.
Native acceptance/preview promotion and standing execution/production gates remain
open. Existing preview/data untouched; all new work/cache/temp is on E:; no pushes,
remote CI, real credentials, paid inference or generated execution.

## 2026-10-27 — Independent Windows M8 acceptance requires repair (historical, superseded above)

Incoming `ffbeee40a4dd3be189e2516d38cbde22b65167ce`, Slice 1 `8718df8a`
and reviewed `08465172a` ancestry, ZIP/bundle hashes, all 5 refs and an independent
scratch clone verified. Review branch `codex/review-m8-slices1-2` changes tests
and evidence only; no production repairs or preview promotion.

Windows positives: all 12 delivered Collider backend cases; UI/contracts 76/76;
TypeScript, contracts drift, strict focused Clippy, formatting, docs and web/API
builds; original Collider browser workflow 1/1 with cancellation/restart/restore.
The duplicate transfer rerun was intentionally interrupted after the dedicated
probes established failures; original transfer-suite Windows completion is not
claimed.

Acceptance blockers reproduced: five transfer probes failed (brief alteration,
forged source/model input accepted by Forge with a synthetic provider hit, generic
brief edit accepted without renewed transfer approval, fabricated derived fields
published by restore, and retry returning a new concept id after generic revise).
An additional Collider exact-beam probe failed, and the browser retained a v1
draft while displaying experiment v2 for the full 30-second assertion window.
These are expected-success tests that fail on the delivered code, not inverted
defect tests. Post-approval stale confirmation and cross-process atomicity need
further coverage; they are not counted as additional reproduced failures.

**Do not promote or start Slice 3 before the bounded acceptance repair.**
[Evidence](reviews/windows-m8-slices1-2.md) and
[Muse repair assignment](handoffs/M8-slices1-2-Windows-repair.md) retain exact
scope, commands/case names and limitations. Linux delivery evidence below remains
historical and is not relabelled Windows certification. Running preview/data
untouched; execution and production gates stay open; no paid inference or CI.

## 2026-10-27 — M8 acceptance repair (Linux verification)

Acceptance repair on branch `muse/m8-acceptance-repair`,
from the independent Windows review tip
`codex/review-m8-slices1-2` @ `c50de23b` (Slice 2 tip
`ffbeee40` verified as an ancestor; the review
commit changed tests and docs only). A version-1
`transfer-approval` receipt now pins the full
approved snapshot, and Forge admits a transferred
concept only when that receipt (generator kind
`transfer`, which manual registration can never
produce) matches the requested concept body and the
pinned experiment in the same workspace/project —
generic revisions mint no approval and all denials
happen before provider traffic. Restore
pre-validation compares every model-derived field
plus receipt rows, with remap and fix-up carrying
valid receipts to restored equivalents. Idempotency
is one SQLite `IMMEDIATE` transaction around a
receipt-bounded scan plus both inserts (no
process-local mutex, no latest-version scan, no
schema change). Beams and briefs stay verbatim
through wire, save, transfer, reopen and restore,
and the transfer/Forge panels are keyed by full
source identity with snapshot-bound responses.

Linux verification, sequential on an idle machine:
review probes pass unmodified (transfer suite
16/16, collider 13/13); full core 285/289 with only
the 4 known environmental live-Ollama
semantic_workflow failures; providers 120/0 + API
24/0 (144/0); node UI/contracts 78/78; TypeScript,
contracts drift, docs, strict Clippy and formatting
clean; API+web builds passed. Browser canonical
battery 16/16 including the strengthened transfer
spec, the new independent transfer-identity spec
and the new cross-process transfer-concurrency spec
(two real API processes, one synthetic root, same
concept id, exactly one concept). Design:
[Acceptance repair design](handoffs/M8-acceptance-repair-design.md);
record:
[Acceptance repair addendum](handoffs/M8-acceptance-repair-addendum.md).
Windows rerun of Slices 1–2 as repaired, M7 native
acceptance, preview promotion, the §32 sandbox and
the M5 production gates remain open. Synthetic only,
no generated-code execution, credentials, paid
inference, downloads, pushes or remote CI.

## 2026-10-27 — M8 Slice 2: collider → Forge transfer (Linux verification)

Explicitly approved transfer on branch
`muse/m8-slice2-forge-transfer`, from the Slice 1
delivery `muse/m8-slice1-collider` @ `8718df8a`
(ZIP/bundle hashes and `08465172a` ancestry
re-verified in a scratch repo before import).
Model-free direct op `collider-transfer` binds an
exact experiment version + recomputed content hash
+ one selected combination + the user's approved
brief into a durable `concept` artifact (existing
registry type, format `anansi-forge-concept` v1),
with verbatim model-derived fields labeled apart
from the user-authored brief and nothing
fabricated. Idempotency is structural (same
experiment/version/combination/brief returns the
same artifact, concurrent or after restart);
`forge-spec-start` admits transferred concepts at
index 0 only while the idea-set path is unchanged;
restore pre-validates transfer bindings and remaps
the experiment reference fail-closed. The transfer
panel under the experiment view saves nothing and
contacts no provider before explicit approval, and
the concept branch opens the existing Forge spec
panel with one concept, so spec generation stays a
separate explicit action and no specification
approval, paid authorization or execution
permission is created by transfer.

Linux verification, sequential on an idle machine:
backend transfer suite 8/8; full core 276/280 with
only the 4 known environmental live-Ollama
semantic_workflow failures; providers 120/0 + API
24/0 (144/0); node UI/contracts 76/76 including 3
new transfer tests; TypeScript, contracts drift,
docs (128 Markdown files after this slice), strict
Clippy and formatting clean; API+web builds passed.
Browser canonical battery 14/14 across
app_checkpoint, artifacts, backups, collider,
evaluations, forge, forge-revision,
forge-deliverable, forge-review, ideas,
opportunities, providers, vault and the new
transfer spec. The first battery run failed 2/14
(the transfer spec's raw-JSON assertion trap,
fixed by scoping, and a one-off forge-review
findings-step failure that passed alone and in
the rerun battery — recorded as intermittent, not
root-caused; only the complete rerun is cited).
Design:
[Slice 2 design](handoffs/M8-slice2-forge-transfer-design.md);
record:
[Slice 2 addendum](handoffs/M8-slice2-forge-transfer-addendum.md).
Windows verification of Slices 1–2, M7 native
acceptance, preview promotion, the §32 sandbox
and the M5 production gates remain open.
Synthetic only, no generated-code execution,
credentials, paid inference, downloads, pushes or
remote CI.

## 2026-10-27 — M8 Slice 1: text-first LEXI-COLLIDER (Linux verification)

Text-first collision on branch `muse/m8-slice1-collider`, from the independently
reviewed checkpoint `codex/review-m7-source-review` @ `08465172a` (reviewer
corrections at `befc66f11` preserved; the original Muse delivery was not
restarted from). Two bounded text beams collide through the existing M5
provider/routing/privacy/cost substrate (ops `collider-start/-poll/-cancel/-save`
on the `ai_command` providers arm); one provider request produces six
validated sections (source understanding, conceptual primitives, cross-domain
mappings, combinations, feasibility, productization) shown under honest
generating/validating states with no invented stages or percentages. Saving
is explicit and immutable through the existing registry as an
`anansi-collider-experiment` v1 body on the existing generic `experiment`
type — no type-list change — pinning exact beams, resolved controls,
provider/model provenance and the full disclosure record. Restore
pre-validation gates only format-matching collider rows; generic manual
experiment rows restore untouched. Accessible 2D only; URL fetching, document
ingestion, saved-project/memory selection, 3D visualization and approved
Forge transfer remain deferred and are shown only as a plain-text note.

Linux verification, sequential on an idle machine: backend collider suite
12/12; full core 268/272 with only the 4 known environmental live-Ollama
semantic_workflow failures; providers 120/0 + API 24/0 (144/0); node
UI/contracts 73/73 including 9 new collider tests; TypeScript, contracts
drift, docs (126 Markdown files after this slice), strict Clippy and
formatting clean; API+web builds passed. Browser canonical battery 13/13
across app_checkpoint, artifacts, backups, evaluations, forge,
forge-revision, forge-deliverable, forge-review, ideas, opportunities,
providers, vault and the new collider spec (no inference before submit,
cancel saves nothing, six sections, save, restart, backup/fresh restore),
with the Forge revision/deliverable/source-review regressions preserved.
Two failures found and fixed during verification, neither by weakening an
assertion: the API route allowlist lacked the four collider paths, so the
browser preflight blocked every collision (fixed in `crates/api`, and the
panel now surfaces a failed start instead of staying silent); and the spec's
result assertion matched the generic editor's raw JSON copy as well as the
rendered section (scoped to the combinations section). No timeout or
assertion was increased or weakened. Design:
[Slice 1 design](handoffs/M8-slice1-collider-design.md); record:
[Slice 1 addendum](handoffs/M8-slice1-collider-addendum.md). Windows
verification of this slice is pending; M7 native acceptance, preview
promotion, the §32 sandbox and the M5 production gates remain open.
Synthetic only, no generated-code execution, credentials, paid inference,
downloads, pushes or remote CI.

## 2026-10-27 — Independent Windows source-review acceptance and corrections

Incoming `7607dc7cf17f5763c5311241835211cc14c6af71` ZIP/bundle hashes,
independent scratch clone, all 8 refs and `e6d22f711` ancestry verified.
Local application correction `befc66f11` on `codex/review-m7-source-review`
keeps review-to-package viewer/history/editor selection coherent, guards malformed
review rendering, names assessments Satisfied by inspection, and rejects an
invalid typed-review format during restore pre-validation. No new feature or
execution capability. Initial probes and their limitations are retained in
[the Windows review](reviews/windows-m7-source-review.md).

Windows: all 10 delivered review backend cases passed; corrected additional
valid-control/invalid-format restore regression passed 1/1; UI/contracts 64/64;
TypeScript, contracts drift, strict focused core-library/review-test Clippy,
formatting, docs and API/web builds passed. New source-review browser flow 1/1
includes history/editor identity and captured generic-save package target;
existing deliverable and source-revision browser regressions passed 2/2, with
their restart/restore/version/Cancel coverage retained. Actual browser screenshot
reviewed. This is not a new full Windows core/provider/API suite or native run.

Use the corrected review branch as the next integration baseline. Running
preview/data untouched; source-review native desktop acceptance/promotion remain
pending. M7 and production gates remain open. Synthetic only, no generated-code
execution, credentials, paid inference, downloads, pushes or remote CI.

## 2026-10-27 — M7 Slice 4: source review (Linux verification)

Model-free source review on branch `muse/m7-source-review`, from the owner's
local integration `codex/m7-preview-discovery` @ `e6d22f711` (ancestry,
selection fixes, preview materials and discovery changes preserved; the older
`bed0dc2` integration was not substituted). New artifact type `source-review`
(19 types): a review pins the exact package and specification versions with
recomputed content hashes, carries one finding per pinned specification
criterion (statuses not-assessed / needs-changes / satisfied, notes bounded
to 2000 characters, file references exact package paths), and states the
inspection limits in the record. Direct op `forge-review-save` resolves and
validates pins in the backend, deliberately without the specification-approval
gate (inspection must be possible on unapproved packages) and without any
provider traffic: opening, editing or saving a review starts no inference and
creates no approval. Findings selection composes editable instructions
prefilled into the existing revision panel (applied only to the viewed
package version); submission keeps the unchanged approval gate, reject/accept
and stale-parent behavior. Restore validates original review bindings before
any remapping, remaps ids fail-closed and rewrites pinned hashes; invalid
references refuse before destination publication.

Linux verification, sequential on an idle machine: backend review suite 10/10;
full core 254/258 with only the 4 known environmental live-Ollama
semantic_workflow failures; providers+API 144/0; node UI/contracts 64/64;
TypeScript, contracts drift, docs (123 files after this slice), strict Clippy
and formatting clean; API+web builds passed. Browser canonical battery 13/13
including the new review spec (assess → save → reopen → review revision with
Cancel regression → findings → prefill → revision reject → revision accept →
historical v1 check → restart → backup/fresh restore → reopen) and the
existing revision and deliverable regressions; no failures in this run, and
the baseline's one-off deliverable approval-screen delay did not recur (still
recorded as intermittent, not root-caused). No timeout or assertion was
increased or weakened. Design:
[Slice 4 design](handoffs/M7-source-review-design.md); record:
[Slice 4 addendum](handoffs/M7-source-review-addendum.md). Windows
verification of Slice 4 is pending and not claimed; §32 execution sandbox,
native desktop acceptance, preview promotion and M5 production gates remain
open; M7 is not complete.

## 2026-10-27 — M7 preview workflow discovery correction

Implementation `a67487d8a` on `codex/m7-preview-discovery` corrects obsolete
first-run claims that AI/providers/Forge are unavailable and adds Ideas & Product
Forge navigation into existing Projects. No domain, transport, consent or execution
capabilities changed. Generated products remain unexecuted and uncertified.

UI/contracts 60/60, TypeScript, contracts drift, web build and docs passed.
Browser onboarding 3/3 includes shortcut activation, keyboard/focus/reload and
narrow-layout checks. Source-revision workflow passed 1/1; deliverable initially
failed at approval-screen loading during the build, then passed the unchanged
idle rerun 1/1 through restart/fresh restore. Initial diagnostic retained; no
timeout/assertion weakened and cause not conclusively established.
Windows embedded desktop build passed; new personal preview startup/copy observed
and left open. Previous executable preserved for rollback. Scope, commands and
hash: [discovery evidence](handoffs/M7-preview-discovery.md). Production gates
remain open; no paid/cloud inference, model downloads, generated execution,
pushes, master merge or remote CI.

## 2026-10-27 — Local combined native preview verified

The owner requested continuing the Windows preview build and verification.
The supplied ZIP was the earlier repair package, not the reported preview ZIP;
Muse's `bed0dc2` is unavailable locally and is not claimed verified. Local branch
`codex/preview-m7-slices2-3` starts from Windows-verified `cc25155ff` and
cherry-picks preview materials from `3335248b`, preserving both status histories.
Application source comparison is identical to `cc25155ff`.

New launcher/root/cache are separate from M7/R17. UI/contracts 60/60, TypeScript,
web build and docs validation passed. Normal embedded desktop/API build passed.
Actual Windows native synthetic workflow passed: spec draft/exact approval,
source generation, revision reject/accept, deliverable edit/save/export, immutable
version selection, Cancel/reopen, restart and native fresh-restored reopening.
Export bytes/manifests/hashes passed inert verification. Backup/restore itself
used fixture API operations; privacy-consent UI acceptance is not claimed.
The first harness expired after successful restore; the follow-up leased native
reopen completed normally. Separate personal launcher/shortcut installed on E:;
old previews preserved. Detailed evidence and hashes are in the linked record.
Plan and scope: `docs/handoffs/M7-local-native-preview.md`; owner guide:
`M7-PREVIEW.md`. No generated-code execution or paid inference, personal-data
migration, pushes, master merges or remote CI. Production gates remain open.

## 2026-10-26 — Independent Windows M7 Slice 3 repair verification

Repair baseline `74c4196744e1623e368bda302c0cbc3c62a9e38c` passed Windows
deliverable tests 9/9 (including both formerly failing review probes), Forge
units 11/11, UI/contracts 60/60, TypeScript, contracts drift, strict core
all-target Clippy, formatting, web/API builds and docs validation.

The original browser assertion exposed a remaining post-save selection race on
Windows. Local frontend corrections on `codex/review-m7-slice3-repair` guard
late artifact reads/refreshes by selection generation and reset the document
buffer whenever editing opens. The full deliverable browser workflow then passed
1/1, including the original version-switch assertion, a strengthened Cancel-draft
check, export, restart and fresh backup/restore. UI/contracts, TypeScript and web
build passed again. Rust/backend code is unchanged from the repair baseline.
Evidence: `docs/reviews/windows-m7-slice3-repair-acceptance.md`.

Existing source-revision browser regression also passed 1/1 on the final tree,
including its version-switch, restart and restore checks. Both browser workflows
passed with unchanged timeouts. All three review findings are closed for this
focused Windows backend/browser scope; preserve the local frontend corrections
when integrating rather than reverting to the unmodified Muse tip.
Native desktop acceptance and
preview promotion of Slices 2–3 remain separate; existing preview/data preserved.
Synthetic providers only; no generated-code execution, paid inference, pushes,
master merge or remote CI. Production credential/provider/G1/CI gates remain open.

## 2026-10-26 — M7 Slice 3 acceptance repair

Branch `muse/m7-slice3-repair` over the reviewed tip `bd72a4f`
(`codex/review-m7-slice3`). All three review defects repaired
with no new features: export validates the full saved
deliverable record against backend-resolved provenance
(format/version, spec identity, concept, pinned hashes,
all-false status, exact unique document set, origins,
bounds); restore validates original package/spec bindings,
ownership, versions and pinned hashes before any id or hash
remapping, refusing invalid originals with no destination
state published; the saved-document editor is keyed by full
record/version identity with cancel resetting its buffer.
The review browser probe is consolidated into the canonical
deliverable spec as a hard assertion; the review spec is
deleted. Linux evidence: full core 244/248 (only the 4
environmental live-Ollama failures), deliverable suite 9/9
incl. both review tests, providers+API 144/0, node 60/60,
TypeScript, contracts drift, strict Clippy, formatting,
docs PASS, e2e canonical 12/12. Record:
`docs/handoffs/M7-slice3-repair-addendum.md`; design:
`docs/handoffs/M7-slice3-repair-design.md`. Windows
verification of this repair remains pending; no preview
promotion is claimed.

## 2026-10-26 — M7 Slice 3 Windows review: three acceptance repairs required

Delivery `cbb95a6960f0bb1379741bdc6a6a34e900611e18` passed supplied Windows
deliverable tests 6/6, Forge units 11/11, UI/contracts 60/60, TypeScript,
contracts drift check, strict core all-target Clippy, web/API builds and docs.
Additional expected-success probes failed: export accepts inconsistent generic
artifact edits (false production-ready status, wrong specification identity,
duplicate documents and unsupported format); restore replaces an invalid original
package pin instead of refusing it; the saved documentation editor retains a v2
draft after switching to v1. Backend probes: 0 passed / 2 failed. Browser probe:
1 failed on the new version-switch assertion, with the remaining original workflow
assertions including restart/restore completing. No preview promotion is approved
by this review. Application production code is unchanged; review adds tests and
evidence on `codex/review-m7-slice3`.

Evidence: `docs/reviews/windows-m7-slice3-review.md`; bounded repair assignment:
`docs/reviews/M7-slice3-repair-assignment.md`. Existing preview and data preserved.
No generated software execution, real-provider calls, pushes, master merge or CI.
Standing native credential/live-provider/G1 audit/CI production gates remain open.

## 2026-10-26 — M7 Slice 3: a complete, reviewable export deliverable

Branch `muse/m7-slice3-deliverable` over the
Windows-reviewed Slice 2 checkpoint `0f586f8`
(review branch `codex/review-m7-slice2`). The §33
deliverable workflow for an unexecuted prototype:
open an exact source-package version, prepare five
documents (architecture, usage, limitations, security,
change summary) with user/model/verified origins,
inspect and edit drafts before saving, save an
immutable `deliverable` artifact pinning the exact
package/specification versions and hashes, and export
a deterministic archive of the exact source bytes plus
approved documentation plus a version/hash manifest —
with the unexecuted status stated throughout and no
license, deployment or build claims. Documentation
lives under `deliverable-docs/`; a package already
using that namespace refuses save/export rather than
moving source bytes. Editing documentation saves a new
version; a saved deliverable never follows a newer
package version. Restore remaps the pinned references
fail-closed and rewrites the pinned hashes, so the
restored deliverable still exports. Linux evidence:
forge units 11/11, `forge_deliverable` 6/6,
`forge_revision` 5/5, `forge_generation` 5/5,
`forge_restore` 4/4, full core 241/245 (only the 4
environmental live-Ollama failures), providers+API
144/0, node contracts/UI 60/60, tsc clean, contracts
check PASS, docs PASS, Clippy + fmt clean, e2e
canonical 12/12 incl. the new
`forge-deliverable.spec.ts` and the reviewed
version-switch regression. Record:
`docs/handoffs/M7-slice3-addendum.md`; design:
`docs/handoffs/M7-slice3-deliverable-design.md`.
**Pending: Windows verification of this slice;
native desktop acceptance and preview promotion of
Slices 2–3 (the preview branch is preserved as a
separate ref, not promoted); the §32 execution
sandbox as a later separately approved slice; M5
production gates remain open.**

## 2026-10-26 — M7 Slice 2 independent Windows review and UI correction

Delivery `9928bf5cb09f07cb90c911b7c5279ad69bc748cb` passed focused Windows
revision 5/5, generation 5/5, restore 4/4, Forge units 10/10, UI/contracts 57/57,
TypeScript, contracts drift check, strict core all-target Clippy, web/API builds,
formatting and docs validation. Review found a stale-proposal UI defect when
switching package versions; the revision panel is now keyed by full parent
identity. The expanded browser workflow failed on the delivered code and passed
after the correction, including reject/accept, version switching, restart and
backup/fresh restore. No timeout or backend approval rule was relaxed.
Evidence: `docs/reviews/windows-m7-slice2-review.md`.

Review branch: `codex/review-m7-slice2`. Native Slice 2 desktop acceptance and
preview promotion remain pending. Existing preview `3335248b` and its data remain
unchanged. Synthetic responses only; no generated software execution, paid
inference, pushes, master merges or remote CI. Standing production gates remain open.

## 2026-10-26 — M7 Slice 2: revising an existing source package

Branch `muse/m7-slice2-package-revision` over the
Windows-verified M7 Slice 1 repair tip `518418b6`.
The §30 Revision stage, non-executing: open a saved
source package at an exact version, request a bounded
revision, inspect the backend-computed diff (added/
removed/changed) as inert text, reject (nothing saved)
or accept as a new version of the same artifact with
exact parent and specification references. The pinned
specification version must hold backend approval before
any provider traffic; a revision creates no approval and
switches no specification. Restore remaps the parent
reference fail-closed; approval travels as in the repair.
Also fixes the observed status-copy issue: pipeline
stages now read "Generated — not yet saved as a project
artifact." until persistence succeeds (previously the
draft was shown as saved). Linux evidence: forge units
10/10, `forge_revision` 5/5, `forge_generation` 5/5,
`forge_restore` 4/4, full core 234/238 (only the 4
environmental live-Ollama failures), providers+API 144/0,
node contracts/UI 57/57, tsc clean, contracts check PASS,
docs PASS, Clippy + fmt clean, e2e canonical 11/11 incl.
the new `forge-revision.spec.ts` (reject → retry →
accept → restart → backup/fresh restore → reopen).
Record: `docs/handoffs/M7-slice2-addendum.md`; design:
`docs/handoffs/M7-slice2-package-revision-design.md`.
**Pending: Windows verification of this slice; Windows
preview promotion (the preview branch
`codex/local-preview-m7` @ `3335248` is not in this
bundle's history — see the addendum's integration note);
the §32 execution sandbox as a later separately approved
slice; M5 production gates remain open.**
## 2026-10-26 — Windows M7 repaired preview available locally

Application checkpoint `518418b6a955feaadcc6212eab0a3f3fd1e1bcb9` passed focused
Windows Forge generation/restore/unit checks, UI/contracts, TypeScript and strict
core all-target Clippy. The normal embedded desktop/API build succeeded on E:.
Actual native synthetic-provider acceptance passed draft generation, explicit
version approval, source generation/save, inert text inspection, TAR export with
manifest/hash verification, and restart/reopen with the provider stopped.
No generated code was executed. Evidence and precise limitations:
`docs/reviews/windows-m7-518418b-acceptance.md`.

Local branch `codex/local-preview-m7` adds preview materials only. Launch
`E:\Anansi\Start ANANSI Preview M7.cmd`; guide: `M7-PREVIEW.md`.
Separate personal data root: `%LOCALAPPDATA%\ANANSI-Preview\m7`.
R17 and personal data are preserved. Native UI backup/restore was not repeated;
backend restore checks passed. Real-provider/native credential, G1 audit and
remote-CI gates remain open. No push or master merge.

## 2026-10-26 — M7 Slice 1 repair: specification approval + portable export paths

Branch `muse/m7-slice1-repair` over M7 Slice 1 tip
`a439bf0`. Two owner-diagnosed acceptance gaps, both
fixed in the backend with no new features: (1)
`forge-build-start` accepted a revised specification
version with no new approval — now approval is a
backend `specification-approval` artifact created only
by `forge-spec-approve`, bound to the exact workspace,
project, specification identity, version and
registry-checked content hash, and enforced in
`forge-build-start` before any provider traffic (a
client approval flag is refused; saving/revising never
confers approval; only an approval artifact's v1 record
counts; restore transfers approval only to the restored
equivalent of the same version); (2) export paths were
not fully portable — the shared validator now refuses
Windows reserved stems in any segment, trailing
dot/space segments, file-versus-directory prefix
conflicts and case-folded directory collisions, in both
generated-output parsing and `forge-export` (before any
tar is built), with all prior protections kept. UI: the
spec panel saves a draft (no longer claims approval)
and the build panel shows per-version approval status
with an explicit approve action and a disabled Generate
button while unapproved. Linux evidence: forge units
8/8, `forge_generation` 5/5, `forge_restore` 3/3, full
core 226/230 (only the 4 environmental live-Ollama
failures), providers+API 144/0, node contracts/UI
55/55, tsc clean, contracts check PASS, docs PASS (106
files), Clippy + fmt clean, Forge e2e 1/1 with the
approval step exercised. Record:
`docs/handoffs/M7-slice1-repair-addendum.md`; design:
`docs/handoffs/M7-slice1-repair-design.md`.
**Pending: Windows preview promotion of this repair
(owner review); Windows verification of M7 Slice 1; the
§32 execution sandbox as a later separately approved
slice; M5 production gates remain open.**

## 2026-10-21 — M7 Slice 1: Product Forge (concept → specification → source package)

Branch `muse/m7-slice1-product-forge` over the verified
R17 preview checkpoint `e915f63` (Windows UI acceptance
recorded by Antigravity at `15affdc2`). One template: a
dependency-free static web application, labelled a
prototype throughout. `forge-spec-*` / `forge-build-*`
runs ride the M5 substrate with the established run
discipline; generation pins exact artifact versions (spec
from the pinned concept version, build from the pinned
specification version); outputs are strict-validated
(package files: 2–12, portable relative paths, allowlisted
extensions, size caps, root index.html + README.md);
saves are immutable `specification` / `source-package`
artifacts; `forge-export` returns a deterministic USTAR
tarball + provenance manifest; restore remaps the
spec/package reference chain fail-closed. Per §32 there
is no execution sandbox in this slice: build/test/
preview/deployment stages are shown Unavailable and
export-for-review is the deliverable; generated code is
never executed. M6 status documentation reconciled in
the same checkpoint (PHASE-006 status, proposal banner;
nothing declared complete that is not). Linux evidence:
core 220/225 (the 4 environmental live-Ollama failures
plus one test list-ordering assumption in the new forge
suite, fixed in this checkpoint and re-verified
standalone: forge unit 6/6, forge_generation 3/3,
forge_restore 2/2); providers+API 144/0; node contracts/UI
54/54; e2e canonical subset 10/10 incl. the new
forge.spec; strict Clippy + fmt clean; docs PASS.
Record: `docs/handoffs/M7-slice1-addendum.md`; design:
`docs/handoffs/M7-slice1-design.md`; AN-012 in progress.
**Pending: Windows verification of this slice; the §32
execution sandbox is a later separately approved slice;
M5 production gates remain open.**

## 2026-10-21 — R17 integrated preview checkpoint (this checkpoint)

Branch `codex/local-testing-r17`, created directly from the
Windows-verified R17 repair tip `ab95e14` (history
preserved; the R11 preview commit `f6cecd2` and the Tauri
`artifact_command` capability fix are ancestors, verified
present in-tree, not re-applied). Contents: everything in
the R16 preview plus M6 Slice 4 (Opportunity Discovery)
and the R17 exact-revision restore repair. This checkpoint
changes only preview materials: `PREVIEW.md` rewritten for
R17 (composition, owner test guide extended with
opportunity discovery and the exact-revision provenance
check) and `Start ANANSI Preview.cmd` pointed at the new
separate data root `%LOCALAPPDATA%\ANANSI-Preview\r17`
(never the owner's existing data; R11/R16 roots untouched).
The owner has confirmed Windows verification of the R17
repair (restore regressions 3/3, opportunity units 5/5,
strict all-target Clippy, docs) on top of the R17 pass.
Linux evidence for this branch: documentation validation
PASS; browser e2e subset re-run on the final tree —
app checkpoint, integrated workflow, opportunities —
**3/3**. Application code is identical to the repair
checkpoint (core 210/214, only the 4 environmental
live-Ollama failures). **Pending: Windows build/launch of
this preview checkpoint.** M5 production gates remain open.

## 2026-10-21 — R17 repair: exact-revision provenance at restore (this checkpoint)

Branch `muse/m6-r17-repair` over the Slice 4 checkpoint
`7253b56`. Windows verification of R17
passed opportunities_generation 6/6, opportunity units 5/5,
opportunities_restore 1/1, UI/contracts 51/51, TypeScript,
strict Clippy and docs — and a focused diagnostic confirmed
one defect: restored opportunity sets kept the SOURCE
workspace's `revisionId` for memory sources. The repair
remaps `revisionId` through the restore identity map and
validates it against the package's own `memory_revisions`
rows (the revision must belong to the referenced memory in
the artifact's own workspace; missing or mismatched fails
the restore closed before anything is written). Original
identities remain recoverable via `restore_origins`.
`opportunities_restore` grows to 3 tests: the original
identity test strengthened with revision assertions, a
real discovery round trip proving the restored set
resolves its pinned historical revision to exactly the
text generation used (not the memory's later revision),
and missing/mismatched revision refusals with a control.
Full record: `docs/handoffs/M6-r17-repair-addendum.md`.
Linux evidence on the final tree: core **210/214** (only
the 4 environmental live-Ollama failures), clippy
`--all-targets` + fmt clean. **Windows verification of
this repair is pending**; M5 production gates remain open.

## 2026-10-21 — M6 Slice 4: project-scoped Opportunity Discovery (this checkpoint)

Branch `muse/m6-slice4-opportunity-discovery` over the
integrated preview checkpoint `2665e22` (tip `4e24f02`).
Owner-approved scope: discovery from **deliberately supplied
information only** — typed/pasted problem, complaint or
observation, a supplied document excerpt, and explicitly
selected memories; no background scanning, web research,
connectors or autonomous actions. New `opportunities-*`
operations ride the existing M5 substrate and Slice 2 run
discipline (no parallel system): supplied sources are
run-identified, privacy-classed wire segments framed as data
never instructions; parsing enforces grounding (citations
to non-supplied ids dropped; a set citing nothing supplied
is refused); saved sets are immutable `idea` artifacts in
the `anansi-opportunity-set` format, directly evaluable by
the Slice 3 rubric workflow; restore remaps body memory
source ids to the restored memories (fail-closed) while
supplied text passes through byte-identical. UI gains an
Opportunity discovery section with a pre-send review of
exactly what will be sent. Full record:
`docs/handoffs/M6-slice4-addendum.md`; design + acceptance:
`docs/handoffs/M6-slice4-design.md`.
Linux evidence on the final tree: core **208/212** (only
the 4 environmental live-Ollama `semantic_workflow` tests
fail; new opportunities unit 5/5, generation 6/6, restore
1/1; restore-change regressions artifacts 6/6,
backup_artifacts 5/5, backup_restore 7/7, evaluations 3/3,
ideas 7/7 + 4/4); providers+API **144/0**; node contracts+UI
**51/51**; tsc/contracts clean; web build ok; browser e2e
**9/9** sequential incl. the new `opportunities.spec.ts`
(discover → save → evaluate → backup → fresh restore →
restored sources resolve); clippy `--all-targets` + fmt
clean. **Windows verification of Slice 4 is pending** — the
owner's R16 preview pass and Windows `vault_increment2` 9/9
do not cover it. M5 production gates remain open.

## 2026-10-21 — R16 integrated preview checkpoint + vault ladder reconciliation (this checkpoint)

Branch `codex/local-testing-r16`, from the verified R16 tip
`a1b0c80` directly (the R11 preview commit `f6cecd2` is an
ancestor; the Tauri `artifact_command` capability fix is
preserved). Composition: R16 + the vault schema-ladder
reconciliation (branch `muse/vault-ladder-reconciliation`;
production fix: vault snapshot verification/staging no
longer migrates the live catalog it opens, so a genuine
schema-8 seal now lands at 9 and a pre-switch abort leaves
schema 8 with no registry table; crash matrix runs on real
legacy-8 and explicit schema-10 fixtures — see
`docs/handoffs/vault-ladder-reconciliation-addendum.md`) +
updated PREVIEW.md/launcher (separate data root
`%LOCALAPPDATA%\ANANSI-Preview\r16`; Windows build/launch
instructions; owner test guide) + a new integrated browser
spec (project → ideas → saved artifact → evaluation →
backup → fresh restore → restart → reopen; restored
evaluations resolve to restored sources) + a production UI
fix for a rubric-load race the verification exposed (rubric
edits made while a rubric read was in flight were silently
overwritten; the form is now disabled during loads and
superseded reads never apply) + an order-agnostic
evaluations e2e history check. Full record:
`docs/handoffs/R16-preview-checkpoint-addendum.md`.
Linux evidence on the final tree: core suite 196/200 (only
the 4 live-Ollama `semantic_workflow` tests fail —
environmental, no Ollama on this machine; `vault_increment2`
9/9, ladder closed on Linux); `backup_artifacts` 5/5,
`backup_restore` 7/7, lib 34/34; vault suites 11/11 + 2/2 +
1/1 + 3/3 + 1/1 + 2/2; providers+API 144 passed / 0 failed;
node contracts+UI 47/47, tsc clean, contracts check clean,
web build ok; browser e2e 8/8 (integrated workflow,
checkpoint, artifacts, ideas, evaluations, providers,
vault, backups); clippy `-D warnings` clean incl.
`--all-targets`; fmt clean. **Windows verification of this
checkpoint is pending** (the owner's pass covers R16 and
R15's suites); the PREVIEW.md Windows steps were not
executed on the preparing (Linux) machine. M5 production
gates remain open (Windows native acceptance incl. real
CredUI/keyring, live-provider verification, G1 audit,
remote CI); Slice 4 stays owner-gated. No pushes, no
master merge.

## 2026-10-21 — M6 R15 repair: eviction-test synchronization + strict clippy (this checkpoint)

Repairs the two remaining findings from the owner's Windows
review of R15, on branch `muse/m6-r15-repair` from `d56ed52`.
Test files only — no production code changed; the R15
recovery implementation is preserved byte-for-byte. The
cap-eviction test's timing assumption (32 merely-slow streams
could finish naturally before the evicting start on a slow
machine) is replaced with server-gated synchronization: the
synthetic provider holds streams behind a gate, observes
client closes directly as socket EOF, and attributes
connections to runs by topic marker; the test proves
oldest + neighbour active (running + one ledger token hold
each), eviction closes exactly the oldest connection, the
evicted reservation is released (32 holds / 33 total
requests), and cleanup drains every test-owned stream to
zero reservations. A mutation check (eviction-cancel
disabled, then reverted) fails the test immediately, so the
backend behaviour is confirmed sound. Strict clippy on the
`backup_artifacts` target is fixed at the cause (unused
`legacy_package` parameter removed; one-element loop in
`rich_fixture` inlined), no suppressions. Linux evidence:
eviction test 10/10 repeats + 3/3 under CPU contention;
focused suites `ideas_generation` 7/7, `backup_artifacts`
5/5, `ideas_validation` 4/4, `evaluations` 3/3,
`backup_restore` 7/7, `artifacts_registry` 6/6, lib 34/34;
clippy `-D warnings` clean for lib, both touched test
targets and `--all-targets`; fmt clean; full core suite
193/199 with only the known failures (4 live-Ollama; 2
`vault_increment2` ladder assertions, left unchanged for
the vault owner). Full record:
`docs/handoffs/M6-r15-repair-addendum.md`. Not merged, not
pushed. Windows re-verification of the rewritten test,
application acceptance and preview promotion remain
pending; the vault assertions stay explicitly open.

## 2026-10-21 — M6 backup/recovery repair (R15 checkpoint)

Repairs the four independently reviewed backup/recovery
defects (Antigravity R12-DEF-01..04) on branch
`muse/m6-backup-recovery` from the R14 tip `bb7cc24`
(the R11 preview commit `f6cecd2` is an ancestor, not
re-cherry-picked): workspace backups include
`artifact_versions`; schema verification recognises the
exact core/vault/FTS/embeddings object groups instead of
one monolithic schema, so application backups of used
databases verify; artifact rows are validated in the
staged catalog (hashes, privacy, version contiguity,
reference resolution incl. evaluation provenance);
restore imports artifacts with ids and embedded references
remapped (fail-closed on references outside the package),
refuses duplicate restores of the same backup artifact,
and rebuilds FTS from restored content; schema-8/9
packages restore through a validated staging-copy
migration. Linux evidence: new `backup_artifacts` suite
5/5, focused backup/artifact/ideas/evaluation suites
27/27, lib 34/34, providers 120/120, UI/contracts 47/47,
browser e2e 5/5, full core suite 193/199 with only the
known failures (4 live-Ollama; 2 `vault_increment2`
assertions root-caused to the Slice 1 schema ladder -
baseline `fc2540f` passes that suite 8/8 - left unchanged
for the vault owner). Design and full evidence:
`docs/handoffs/M6-backup-recovery-design.md`,
`docs/handoffs/M6-backup-recovery-addendum.md`. Not
merged, not pushed. Windows verification of this
checkpoint and preview promotion remain pending.

# ANANSI Ultimate — Build Status

## M6 Slices 2–3 + R13 reliability repair — 2026-10-21 (Linux evidence)

Branch `muse/m6-slice3-evaluation` (over the R13 repair checkpoint on
`muse/m6-slice2-idea-generation`; R11 preview ancestry `f6cecd2` preserved).
Last verified commit: see the delivery addenda
(`docs/handoffs/M6-r13-repair-addendum.md`, `M6-slice3-addendum.md`).

- **R13 repair** of the independent Windows review's four findings:
  atomic save idempotency (claim under lock, register outside it),
  idea-run lifecycle controller (cancel on project switch, stale-response
  guards, serialized polling), bounded ideas registry (expiry/eviction
  cancels active provider streams outside the lock, with drain-to-terminal
  settlement), strict clippy clean at the causes. New regressions for each.
- **M6 Slice 3 — transparent concept evaluation & comparison** (the M6
  plan's "Scoring rubric v1", with owner decision 1 settled): user-entered
  1–5 assessments with rationales under a versioned rubric artifact
  (owner-approved standard rubric: problem clarity, customer specificity,
  MVP feasibility, budget/technology fit, validation readiness; equal
  weights); the only computation is core's deterministic weighted summary
  over assessed dimensions (missing = "not assessed", never zero);
  evaluations are immutable artifacts citing exact source/rubric versions
  with the rubric snapshot embedded; rubric revision never rewrites
  history; saves are content-hash idempotent. No model calls in this slice.
- Linux evidence: evaluations units 4/4 + integration 3/3; ideas 11/11 +
  eviction units 2/2; providers 120/120; API 24/24; UI/contracts 47/47;
  clippy `-D warnings` clean; fmt/tsc/contracts/docs clean; browser e2e
  5/5 (checkpoint, artifacts, ideas, evaluations, providers).
- Full anansi-core suite: 188 passed / 6 failed of 194 — the 4 known
  live-Ollama semantic tests plus 2 `vault_increment2` tests whose harness
  deadlines are load-sensitive on this VM; the identical failure was
  reproduced on the pre-repair tip `cb8c1cb`, i.e. not a regression of
  these changes. (Correction recorded in the R13 addendum: R11/R12 reports'
  ~497/506 core pass totals were miscounted aggregates; the suite holds
  187 tests pre-Slice-3, 194 now.)
- Still open and unclaimed: Windows verification of the repair + Slice 3,
  M5's native/manual acceptance, live-provider verification, G1 external
  audit, remote CI. M6 Slice 4 (Opportunity Engine) awaits the owner's
  remaining plan decisions; no further milestone work is started.


## Local integrated preview — 2026-10-21

Branch `codex/local-testing-r11` contains Muse R11 (`07be39ca`, including integrated M2–M5 and model-free M6 Slice 1), with a local repair granting the main native window access to `artifact_command`. A normal embedded Windows development executable was built and launched with a responsive `ANANSI Ultimate` window and an isolated preview catalog. See [preview instructions](../PREVIEW.md). Six registry tests, 39 UI/contract tests, TypeScript, web build and documentation checks passed. This is build/launch evidence, not full native artifact workflow acceptance or release certification. Existing audit, manual credential, remote CI and source-publication deferrals remain. The primary checkout and protected master were not changed.

## Current authoritative summary — 2026-10-20

Local M2–M4 integration and regression tasks on `codex/m2-m3-integration` are complete through implementation commits `ee92afb` and `b6d472c` (starting from `5dd40d3` / `b543c7b`, retained M2 checkpoint `bd2e2a9`). This summary supersedes historical completion/certification and stop instructions below. Integration owner/worktree and acceptance checklist: [handoff](handoffs/M2-M4-integration-owner.md).

Verified locally in desktop (Tauri/WebView2) and browser (Playwright + loopback HTTP API):
- **Semantic Search, Explicit Indexing, Visible Fallback, Atomic Publication & Reranking (`ee92afb`)**: Separate generation (`modelTag`) and embedding (`embeddingModel`) configuration; explicit Search UI model/endpoint controls and workspace indexing (`#search-index-embeddings`); visible `.status-lexical-fallback` and `.status-semantic-active` status banners; atomic multi-chunk `SAVEPOINT` publication with concurrent-edit and partial-failure rollback tests; post-fusion `rerankScore` with `rerankProvenance`, `validUntil` temporal filtering, and contradiction lifecycle evaluation; full UI E2E workflow (`save record -> index -> retrieve paraphrase with zero keyword overlap -> revise -> verify old embeddings invalidated -> restart and search again`) against live `all-minilm:22m`.
- **Vault Browser Native Authorization & Encrypted Backup/Export/Restore (`b6d472c`)**: Origin/workspace/process-bound browser native authorization via `WindowsCredentialProvider::for_browser` (`/v1/vault/*` with secret-DTO rejection and desktop-only initial `setup`/`seal`); encrypted ciphertext backup, encrypted export, locked archive preview, tamper/wrong-key/cancel refusal, and fresh sealed restore verified in both Playwright (`tests/e2e/vault.spec.ts`) and Windows Tauri/WebView2 (`scripts/verify-vault-desktop.mjs`).
- **Local Ask & Context Compiler**: Real-model Ask streaming (`qwen2.5:0.5b`), cited SQLite answers, disclosure manifest, and backend socket cancellation pass in browser and Windows Tauri UI (`scripts/verify-ask-native.mjs`). Big Brain (`AN-011`) remains design-only (`docs/designs/BIG_BRAIN_SPEC.md`, ADR-0030).

Remaining local prerequisite and separate deferred release/publication gates (recorded in [M5 readiness](evidence/M5-READINESS.md)):
1. **Local development prerequisite — Manual Win32 Credential & Recovery Dialog Check**: Prepared via `node scripts/with-application-test-lease.mjs scripts/manual-vault-acceptance.mjs` (without `ANANSI_VAULT_TEST_PROVIDER`) for human execution of real Win32 `CredUIPromptForCredentialsW` and `MessageBoxW` dialogs (never automated). The initial human recovery acknowledgement failed; a successful retry and remaining manual workflow have not been reported. M5 task preparation remains pending that local result.
2. **Pre-Production External Cryptographic Audit (`G1-External-Audit`)**: Open pre-production gate under accepted engineering risk; audit monitor remains paused per Owner direction.
3. **Remote CI & Protected Publication**: Deferred by Owner capacity policy. No M5 implementation is started.

## Historical evidence and claims (superseded where inconsistent above)

## Owner amendment — M2.7-Core isolation (2026-10-19)

M2.7-Core Increment 1 is recoverable at `a7b4794fd0a0f4d39d9999f278a536711efa767d`; all three exact-SHA jobs passed in [run 39195589192](https://github.com/robillionair/Anansi-ultimate/actions/runs/39195589192), including Ubuntu Foundation, confirming the optional Linux Secret Service feature repair, Windows Foundation and native smoke. [Increment 1 evidence](evidence/M2-7.md) preserves its exact test sequencing and limitations.

Increment 2 local core acceptance passed: explicit schema-9 encrypted full metadata/history/graph generations, cross-process root/KDF admission, durable epochs and atomic cutover, full DEK rotation, recovery acknowledgement and encrypted backup/export/fresh restore. Passed across runs: 87 distinct Rust entries (one opt-in native-store test ignored), all ten process-crash boundaries, 40 tooling tests and 13 recovery checks, fmt/clippy/docs. [Increment 2 evidence](evidence/M2-7-increment2.md) preserves exact sequencing, including a host-sleep expiry refusal and clean full matrix rerun without bypass. This is not a complete or application-enabled vault. Search/UI/adapters/workflows, the live schema-8 Core and Sealed refusal remain unchanged. Legacy plaintext copies and interrupted stages are retained; no forensic erasure or complete whole-root at-rest conversion is claimed. Prior scoped verified source checkpoint is `a7b4794`; final new checkpoint/tree/CI/source restore belong in the external Increment 2 ledger after commit. No M3/master publication is performed here.

M2 integration increment: the vault-owned `SearchBoundary` is now a tested domain hook for the later M3 consumer. It excludes Sealed records before persistent indexing and rechecks caller, session lifetime and durable generation epoch immediately before any Sealed result publication. [Handoff](handoffs/M2-to-M3.md) records the boundary and the required stale-FTS/rebuild/lock/revocation review. This does not activate Sealed commands, modify M3 search/UI files, or claim M2.8 acceptance.

M2 integration increment 2 adds vault-owned transactional FTS invalidation helpers for one record or an entire workspace, with focused in-memory SQLite tests. They are designed to run in the same sealing/revocation/generation transaction and do not expose vault keys. M3 application wiring, native/browser workflows, encrypted backup/restore integration, and M2.8 acceptance remain pending.

M2/M3 shared search integration is now applied at `94c27b8` plus local contract repairs: M3’s bounded body search and rebuild/error-integrity fixes are present, `MemoryMetadata` carries optional response-only ranking/evidence fields, and memory writes synchronize or invalidate FTS rows in the same SQLite transaction. Local body-search, corpus-limit, and schema-8 sealing-transition tests pass. The true Sealed vertical slice remains blocked by the missing adapter-facing vault setup/session command and trusted native credential/prompt path; Sealed creation continues to fail closed. No whole-root encryption or secure erasure is inferred from search exclusion.

M2.7 runtime increment adds `VaultRuntime`, which owns the non-serializable acknowledged setup and process-local sessions behind a native-only credential provider. Browser setup is rejected before credential input; setup cannot publish without a verified M2.5 backup reference. [Runtime evidence](evidence/M2-7-runtime-acceptance.md) records the focused check. Adapter command wiring and trusted Windows prompt implementation remain next; no Sealed activation or M2.8 claim is made.

The follow-up runtime increment adds adapter-facing `context` and authenticated `read` methods. They return only a vault context/snapshot after the existing session, caller, deadline and epoch checks; no key or credential transport is introduced. Focused runtime test and warnings-denied Clippy pass. Native prompt, command transport, browser authorization and integrated Sealed workflow remain pending.

The process-owned `VaultService` increment now provides a cloneable adapter handle over a mutex-protected `VaultRuntime`, with setup, seal, unlock, authenticated read, lock and context operations. This preserves process-local secret/session ownership and is locally compiled and linted. It does not claim desktop/browser activation; native prompt and transport wiring remain incomplete.

M2.7 native application integration now passes the actual Windows Tauri/WebView2 workflow: persistent adapter-owned service, trusted native credential/recovery controls, verified-backup sealing of a synthetic Private record, authorized encrypted read, manual-lock denial, restart-locked denial and restart unlock/read. [Evidence](evidence/M2-7-native-application.md) records the exact boundary and synthetic evidence root. Browser authorization, encrypted portable backup/restore and full M2.8 remain pending; remote CI/publication is deferred and G1-External-Audit remains open.

[Owner risk acceptance](approvals/M2-6-owner-risk-acceptance.md) explicitly supersedes historical G1 pre-implementation stop language below. G1-External-Audit remains open for post-M3/pre-production/commercialization certification; no audit coverage is fabricated. M2.5 source/CI recovery baseline was reverified. M2.7-Core is authorized in a separate worktree; M3 Search/UI wiring and shared-checkout files remain untouched. G2–G4 are unchanged. No vault completion, production certification or implemented UI disclosure is claimed. AN-018/019/031 remain in progress.

## Current state — 2026-10-19

M4.5 cross-edition UI and integrated acceptance certification is verified under [M4.5 evidence](evidence/M4-5.md), completing Milestone 4 (Semantic Intelligence). Exposes 3 search modalities ("Title only", "Full-body search (includes Markdown text)", and "Hybrid semantic") across shared UI and Playwright E2E browser testing, wires scoped HTTP (`/v1/ai/hybrid-search`, `/v1/ai/context`) and Tauri IPC (`ai_command` with `'hybrid-search'` and `'context'`) endpoints, presents grounded evidence with RRF scores, distinct lexical/semantic ranks, line numbers, and section headings, and enforces fail-closed Sealed privacy refusal across all search and context boundaries. Passed: full workspace Cargo test battery (`anansi-core`, `anansi-api`), 30/30 UI/contract tests, 2/2 Playwright browser integration tests (`search.spec.ts` and `ask_anansi.spec.ts`), 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift, and full repository planning self-test validation. Milestone 4 is 100% certified.

M4.4 complete Context Compiler and Big Brain deliberative design gate are verified under [M4.4 evidence](evidence/M4-4.md). Multi-chunk section-aware context compilation (`crates/core/src/context.rs`), strict token/character budget packing with clean chunk truncation, grounded citation generation (`[1]`, `[2]`), chunk deduplication across memories, fail-closed Sealed privacy refusal, complete data disclosure manifest ("View What ANANSI Sent"), and architectural resolution of the Big Brain (AN-011) design gate via formal behavior contract and state machine specification (`docs/designs/BIG_BRAIN_SPEC.md` and ADR-0030) are verified. Passed: 5/5 targeted context compiler tests, 19/19 combined M4/Ask tests, 27/27 UI/contracts tests, 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. Next scope is M4.5 cross-edition UI and integrated acceptance certification.

M4.3 hybrid retrieval fusing SQLite FTS5 lexical BM25 ranking and vector cosine ranking via Reciprocal Rank Fusion (RRF) is verified under [M4.3 evidence](evidence/M4-3.md). Dual-modality retrieval pipeline, configurable RRF scoring ($w_{\text{lex}} / (k + \text{rank}_{\text{lex}}) + w_{\text{sem}} / (k + \text{rank}_{\text{sem}})$), grounded evidence locations (Markdown heading and line number), metadata and entity filtering (`kind`), archive exclusion (`archived = 0`), fail-closed Sealed exclusion (`privacy != 'sealed'`), and a semantic evaluation benchmark suite covering exact lexical matches, semantic paraphrases, dual-modality synergy, and missing evidence are verified. Passed: 5/5 targeted hybrid retrieval tests, 15/15 combined M4 tests, 27/27 UI/contracts tests, 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. Next scope is M4.4 complete Context Compiler and Big Brain tracked design.

M4.2 vector search and SQLite cache synchronization are verified under [M4.2 evidence](evidence/M4-2.md). In-memory cosine similarity vector search (`vector_search`), descending similarity ranking with stable tie-breaking, threshold filtering (`min_score`), limit pagination (1..100), active memory filtering (`archived = 0`), fail-closed Sealed exclusion (`privacy != 'sealed'`), and transactional cache invalidation upon memory mutation (`invalidate_embeddings` in `sync_index_tx`) are verified. Passed: 5/5 targeted vector search tests, 5/5 targeted vector embedding tests, 27/27 UI/contracts tests, 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. Next scope is M4.3 hybrid retrieval and Reciprocal Rank Fusion (RRF).

M4.1 local embedding generation and pure Rust vector space primitives are verified under [M4.1 evidence](evidence/M4-1.md). Mathematical vector operations (L2 norm, normalization, dot product, cosine similarity clamped to `[-1.0, 1.0]`), strict dimension bounds (2 to 1536 float32 dimensions), NaN/infinity validation, IEEE 754 little-endian binary serialization (`as_chunks::<4>()`), SQLite embedding cache persistence (`memory_embeddings` table with upsert, workspace listing, and memory cascade deletion), loopback-only Ollama embedding client (`/api/embeddings` with 10s timeout), and fail-closed Sealed memory exclusion are verified. Passed: 5/5 targeted vector tests, 27/27 UI/contracts tests, 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. Next scope is M4.2 vector search and SQLite cache synchronization.

M3.5 integrated certification across full-body search, local runtime discovery, minimum Context Compiler, and local Ask ANANSI is verified under [M3.5 evidence](evidence/M3-5.md). Parity between the native desktop and browser editions (over authenticated loopback HTTP API and native IPC `ai_command`), graceful offline transitions, grounded citations with Markdown section headings and line numbers, truthful missing-evidence handling without hallucination, data disclosure manifest ("View What ANANSI Sent"), coordinated fail-closed Sealed exclusion with M2 vault boundary, and 100% loopback isolation with zero external requests (`external === 0`) are verified. Passed: 1/1 Playwright end-to-end integration test (`tests/e2e/ask_anansi.spec.ts`), 27/27 UI/contracts tests, 4/4 targeted Ask tests, 7/7 AI discovery tests, 2/2 vault search invalidation tests, 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. M3 milestone local acceptance is complete.

M3.4 minimum Context Compiler and local Ask ANANSI query pipeline is verified under [M3.4 evidence](evidence/M3-4.md). Core grounding engine, citation compiler (`GroundedCitation` with index, memory ID, title, kind, section heading, line number, snippet), strict context budget enforcement (bounded to 4,000 characters / ~1,000 tokens), deduplication, truthful missing-evidence handling (`AskStatus::InsufficientEvidence`), offline resilience (`AskStatus::Offline`), full data disclosure manifest ("View What ANANSI Sent"), pure loopback inference over Ollama (`/api/generate`) and LM Studio (`/v1/chat/completions`), Core/Tauri/API routes, generated TypeScript contracts, and the "HOME -> Ask ANANSI" shared UI view are verified. Passed: 4/4 targeted Ask tests, 27/27 UI/contract tests, 0 clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. Next scope is M3.5 integrated certification.

M3.3 authorized local AI runtime discovery and hardware profiling is verified under [M3.3 evidence](evidence/M3-3.md). Native hardware inspection (CPU architecture/cores, total/available physical RAM via Win32 `GlobalMemoryStatusEx`, storage free space via `GetDiskFreeSpaceExW`, discrete GPU name/VRAM inspection), strict loopback isolation and offline resilience (<2s timeouts, loopback-only validation), tiered model recommendations (1.5B/3B, 7B/8B, 14B), explicit download consent enforcement (no automatic pulls/telemetry), Core/Tauri/API command routing, typed contracts, and the "SYSTEM -> Local Models" UI are verified. Passed: 94 Rust tests, 25 UI/contract tests, 40 tooling and 13 recovery tests, zero clippy warnings with `-D warnings`, zero format diffs, zero contract drift.

M2 vault/search handoff consumed read-only from isolated checkpoint `e67010ebd2461868874e311c9bd764554f574326`. M3 added `vault_search_integration`, which simulates a schema-8 Private→Sealed transition and verifies rebuild plus restart remove a plaintext FTS canary. This is a current-index privacy regression only; live `SearchBoundary` session/epoch wiring remains pending coordinated M2 integration. Exact shared-file obligations are recorded in [the M3→M2 handoff](handoffs/M3-to-M2.md). Local test passed; remote CI remains deferred.

Current authorized increment: **M3.2 full-body indexing and retrieval evaluation**, verified under [the detailed plan](../plans/03-search-and-ollama.md) and [evidence ledger](evidence/M3-2.md). M3.1 is verified complete at published checkpoint `8c9406b`. The user explicitly authorized work independent of audit feedback. This supersedes earlier blanket stop-before-M3 scheduling text for M3 search increments. M2 remains incomplete and G1 remains mandatory. [RustCrypto member feedback](https://github.com/RustCrypto/password-hashes/issues/940#issuecomment-6256202461) says he does not think argon2 has ever been audited; [our authorized acknowledgement](https://github.com/RustCrypto/password-hashes/issues/940#issuecomment-6256248503) retains the gate. An active hourly follow-up checks for new actionable evidence and checkpoint CI. No audit commission, crypto dependency or inference was introduced.

M2.6 G1 follow-up: the user selected **“Retain pure Rust; obtain matching audit evidence.”** [The evidence review](evidence/M2-6-audit-review.md) records verified direct-package source identities and report scope exclusions. [The library comparison](../plans/M2-vault-library-options.md) retains native libsodium as unselected research. G1 remains open for matching independent Argon2 evidence and current AEAD/dependency mapping. After explicit user authorization, [the precise maintainer inquiry](reviews/M2-6-audit-inquiry.md) was published as [upstream issue 940](https://github.com/RustCrypto/password-hashes/issues/940). No paid review occurred; a request does not close G1. No vault product work or dependency change. Prior design checkpoint is `57abb23c8fa2065ae888dc4cbb1cf7325cfe6c10`; publication/recovery for this follow-up is recorded after validation in the external audit-review ledger.

M2.5 is verified and published at `6c193ef8c9891e966c547451b4e01b7db74215b4` (implementation `cfc28af`). All three implementation and checkpoint CI jobs passed, including native backup workflows and unsigned installer verification; protected master and independent source recovery are verified. See [M2.5 evidence](evidence/M2-5.md) and its permanent ledger. M2 remains incomplete.

Retained M2 blocker: M2.6 security-design reconciliation. The user's conditional M2.7 approval is preserved in [the original contract](approvals/M2-6-user-contract.md), correcting the obsolete awaiting-approval claim. [The detailed plan](../plans/M2-vault-security.md) and [review evidence](evidence/M2-6.md) record official source/version checks and the adopted [contract corrections](approvals/M2-6-corrections.md). G2–G4 are resolved at design level; G1 audit coverage remains open. No crypto dependency, vault code, schema change or secret-store write was added. M2.7 is blocked on that audit evidence; no repeated blanket user approval is needed. Do not start M2.7 until G1 closes; the independently authorized M3.1 scheduling exception is recorded above.

M0 and M1.0–M1.5 are verified complete. M1 completion checkpoint 97babb3ae9f3dc20b5602a7b46a947e833943eb2 passed all three CI jobs in run 38955878511, was published to protected master and tagged m1-desktop-foundation-verified. Final source bundle restored identical HEAD/tree/tag and passed documentation checks. The downloaded CI installer passed the full local installed-release native/browser workflow and data-preserving uninstall.

M2.1 is verified complete. Implementation checkpoint 39b0e5ae440223a87ac394c60706160a8f237ca6 passed all three jobs in [CI run 38962588037](https://github.com/robillionair/Anansi-ultimate/actions/runs/38962588037), including actual native memory verification and release packaging/install/uninstall. It was published to protected master; independent source restore matched HEAD/tree and passed frozen install, types, client/tooling/recovery tests. Last verified implementation SHA is recorded rather than this metadata commit to avoid self-reference. See [M2.1 evidence](evidence/M2-1.md). M2.2 completion is recorded below.

M2.2 is verified complete. Implementation checkpoint aac601dcee12067a1bbee44b62d72db0a3a6122b passed all three jobs in [CI run 38990088817](https://github.com/robillionair/Anansi-ultimate/actions/runs/38990088817), including actual native/browser lifecycle/restart verification and development release packaging/install/uninstall. It was published to protected master. Independent source restore matched HEAD/tree and passed frozen install, docs, types, 13 client/component tests, 40 tooling tests and 13 recovery checks. Local acceptance included 46 Rust tests and nine browser workflows; the final pending-save repair passed both memory workflows again. See [M2.2 evidence](evidence/M2-2.md). Last verified implementation SHA is recorded rather than this metadata commit to avoid self-reference.

M2.3 acceptance repair is verified at implementation checkpoint 5b83338d08884349558a7145de9a5f30c75dd469, superseding the incomplete 82789d3 completion claim. Schema 6 adds immutable relationship revisions/provenance and reversible optimistic archive/restore; historical record reads retain saved link snapshots; project membership requires a same-workspace project, omission retains assignment and explicit null clears it. All eight linked record kinds, history, completed tasks, clear/reassign and both desktop/API process restarts passed actual shared-UI verification with zero external requests. Narrow filter/action readability was inspected and tested after correction. Local checks: 56 Rust tests, 18 UI/contract tests, 10 browser tests, 40 tooling/hygiene tests and 13 recovery checks; types/contracts/format/clippy/build/docs passed. Both required foundation jobs in [CI run 39078924117](https://github.com/robillionair/Anansi-ultimate/actions/runs/39078924117) passed; the separate native CI job was still running when this record was authored (the same native verifiers already passed locally). Consult the linked run for its final packaging results. See [M2.3 evidence](evidence/M2-3.md). Last verified implementation SHA is used rather than the subsequent evidence checkpoint to avoid self-reference. Stop before M2.4.

## M1 certification and recovery

- Actual current-user unsigned NSIS install, full installed native/browser onboarding/workspace/settings/process-restart workflow and uninstall passed. Executable/uninstaller/registration removed, private synthetic catalog files preserved byte-for-byte. Actual screenshots inspected; no real user catalog was targeted.
- CI artifact 12055044353 contains only release setup, SHA256SUMS and two JSON evidence files. Downloaded ZIP digest, installer checksum and clean source identity were verified. CI's installed-release mode explicitly skips UI on elevated hosts; separate native UI checks and actual local downloaded-installer UI tests passed.
- 40 Node foundation/tool tests, 13 source-recovery checks, 26 Rust tests, 11 client/component tests and seven browser workflows passed. Native discovery/compile/link/run, formatting, Clippy, types, contract drift, frozen installation and docs/whitespace checks passed. Initial bundle-marker/overlapping-test failures were repaired and retained as history.
- Independent source bundle restored matching HEAD/tree; restored docs, frozen install, types, 11 client/component tests, 40 foundation/tool tests and 13 recovery checks passed. Source backups exclude private catalogs, credentials and installers. Master specification and all 90 section owners, 16 plans and 33 requirements are retained.

## Remaining scope and next gate

M1 is development-foundation certification only. AN-003 shared foundation design and AN-023 local onboarding/workspace/navigation/settings have verified workflow evidence. AN-001/002/018/020 retain release/security/later-platform scope and remain in progress. Signed installers, updater and clean-machine certification remain M15; no fully offline runtime installation claim. Existing WebView2 supports tested offline operation; its embedded bootstrapper needs internet if the runtime is absent.

M2.1–M2.5 acceptance gates passed within their recorded scopes. M2.6 reconciliation now records the supplied approval and concrete unresolved prerequisites. Next work: close G1 audit coverage in the M2.6 plan, then M2.7 implementation under the existing conditional authorization; M2.8 integrated certification remains required before M2 completion and integrated downstream certification; the user-authorized M3.1 independent slice is the explicit scheduling exception. Documentation checks do not verify encrypted product behavior.


## Historical M2.3 acceptance repair — 2026-10-18

The authorized audit repair is complete; ADR-0019 records the schema/history/membership decisions and [repair evidence](evidence/M2-3.md) records actual results and intermediate failures. The source checkpoint ledger is written after protected publication at `C:/Users/robin/Documents/ChatGPT/Anansi-source-recovery/M2-3-verified-b4a8e110d61042fbb11a3faf434bf9b4/source-verification.json` and contains the actual source HEAD/tree, bundle digest and independent restore results. Screenshots and machine verification records are retained alongside it. Source recovery excludes private application catalogs. The master specification and M0/M1 tags are unchanged. M2.4 is the next approved task and remains unstarted; M2 as a whole is incomplete.

## M2.4 local acceptance — 2026-10-18

User authorized continuation after M2.3. Work is on `codex/m2-4-jobs-export`; [M2.4 acceptance criteria and verification ledger](evidence/M2-4.md) were created before implementation. The previous verified checkpoint remains c141939 (production implementation 5b83338). M2.4 local acceptance now passed at 8f7ff87, superseding the earlier in-progress entry. Remote publication/source recovery are finalized through the ledger referenced above. M2.5 has not started.