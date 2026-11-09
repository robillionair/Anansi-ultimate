# ANANSI Ultimate

ANANSI Ultimate is a local-first workspace for researching a product idea, building a prototype plan, testing assumptions, and preserving the evidence behind each decision. It has a Windows desktop application built with Tauri 2 and a React browser interface paired to a local API. Both interfaces use the same Rust domain logic and generated contracts.

This README describes the Windows preview integration branch. The verified source checkpoint is `4e4e970af8ad0c406cfae35fc3da55f99662b174`. It also includes Antigravity's focused Windows acceptance checkpoint and Muse's SOMNIA compaction integration. See the [build status ledger](docs/BUILD_STATUS.md) for test details and scope.

- [Screenshots](#screenshots)
- [What you can do](#what-you-can-do)
- [Windows preview](#windows-preview)
- [SOMNIA compaction](#somnia-compaction)
- [Run from source](#run-from-source)
- [Development and verification](#development-and-verification)
- [Repository map](#repository-map)
- [Project roadmap](#project-roadmap)

## Screenshots

Application captures from Windows review builds using synthetic workspaces. The captures come from several preview checkpoints.

**ANANSI Entity · LARP scene 07**
The holographic avatar and the grounded Ask ANANSI conversation panel.

![ANANSI Entity holographic portrait beside the Ask ANANSI conversation panel](docs/screenshots/larp-anansi-entity.png)

**Second Brain · LARP scene 01**
A navigable constellation of sample workspace memories.

![Second Brain memory constellation with eight synthetic memory nodes](docs/screenshots/larp-second-brain.png)

**SwarmTank · LARP scene 08**

The interactive persona matrix with a selected synthetic buyer and scenario controls.

![SwarmTank persona matrix and scenario controls](docs/screenshots/larp-swarmtank.png)

**SOMNIA · LARP scene 09**

The replay chamber during a synthetic model comparison, with the automatic compaction panel visible.

![SOMNIA replay chamber showing an active comparison beside the automatic compaction panel](docs/screenshots/larp-somnia.png)

**LEXI-COLLIDER · Project visualization**
The reviewed Three.js view of a saved synthetic experiment.

![Three.js Collider scene showing the two beams and the experiment's sections](docs/reviews/evidence/windows-m8-threejs/viz-three-animated.png)

## What you can do

| Area | Workflow |
| --- | --- |
| **Home and Projects** | Resume recent work, open a project or guided lifecycle step, and see local status, queued jobs, and backup health. |
| **Memory and search** | Capture and revise records, link them to projects and other records, archive and restore them, and search through lexical and configured semantic retrieval. |
| **Ask ANANSI** | Ask questions against approved workspace material. Review the grounding evidence and citations alongside the answer. |
| **Opportunity Discovery** | Supply problems, observations, excerpts, and selected memories. Generate hypotheses with sources and assumptions, save them as project artifacts, and compare them against a rubric. |
| **Product Forge** | Turn an opportunity into a specification or implementation plan, review proposed revisions, and export a prototype package. |
| **LEXI-COLLIDER** | Combine prompts, concepts, and selected project context. Review the result before transferring it into Forge. |
| **SwarmTank** | Explore synthetic buyers in the arena, compare assumptions and scenarios, and replay how outcomes change across a timeline. |
| **SOMNIA** | Replay isolated scenarios, inspect failures, compare two reviewed model inputs, and save comparison evidence in a private project artifact. |
| **GHOST-PROMPT** | Explore prompt drafts, revisions, and alternative formulations through an explicit opt-in workflow. |
| **Launch Studio** | Follow the product lifecycle, review its deliverables, and package an export for handoff. |
| **Documents and MCP** | Ingest text, Markdown, and supported PDF text. Review MCP tools, resources, and prompts, with scoped approval for mutations. |
| **Backups and recovery** | Create and inspect a backup, preview a restore, and recover into a fresh workspace. |

A typical session starts with a workspace and project. Add source material, explore an opportunity, move a candidate into Forge, then test the assumptions in SwarmTank or SOMNIA. Save the inputs and results as project artifacts so you can inspect the evidence behind a decision later.

### SwarmTank v2

SwarmTank starts with synthetic personas and an explicit scenario. The preview adds searchable territories, researched profiles with source and date information, and custom assumptions for markets without a profile. Its timeline supports a configurable horizon and step size, scheduled events, and what-if branches such as a price change or competitor launch.

Choose the deterministic path for repeatable scenario exploration, or explicitly configure and authorize a provider for model-backed simulation. The Windows review exercised provider dispatch with a synthetic provider, including malformed responses, budget exhaustion, and cancellation that preserves completed rounds. Synthetic persona outcomes support comparison and hypothesis generation. The calibration plan describes future work to compare them with independent observations. [SwarmTank v2 design](docs/handoffs/swarmtank-v2-design.md) · [Timeline and what-if branches](docs/handoffs/swarmtank-v2-checkpoint3.md) · [Calibration plan](docs/handoffs/swarmtank-v2-calibration-plan.md)

### SOMNIA replay and comparison

1. Open SOMNIA from a project or from LARP scene 09.
2. Use the isolated replay and nightmare tools to evaluate a scenario and inspect findings.
3. For a model comparison, review the original and proposed inputs and select the configured model.
4. Approve the comparison. The two arms use the same model through the existing provider and privacy gates.
5. Inspect both outcomes and their reported token usage, then save the comparison to the project.

The verified comparison flow handles cancellation between dispatches, project changes during requests, duplicate-save attempts, and attribution of token counts to the active comparison. The OpenRouter adapter has an explicit option to disable reasoning for answer-focused runs. You can supply prepared inputs or generate a compaction candidate from an imported history. [SOMNIA compaction](#somnia-compaction) describes the reviewed-candidate workflow. [Comparison and provider handoff](docs/handoffs/somnia-model-comparison.md)

### Document ingestion and MCP

Document ingestion accepts text, Markdown, and supported text-bearing PDFs. The PDF path limits extraction, tracks source pages and line information, and supports page navigation, search, and grounded Ask ANANSI citations. Scanned, encrypted, malformed, and unsupported PDFs can be rejected without publishing a partial document. [PDF ingestion handoff](docs/handoffs/m13-pdf-integration.md)

The MCP integration includes a stdio client for reviewing tools, resources, and prompts, and an authenticated server surface with scoped operations. The product treats proposed tool mutations as separate from read-only exploration. [Architecture and trust boundaries](docs/ARCHITECTURE.md)

### LARP Mode

Toggle LARP Mode with **Ctrl+Shift+L**. Choose a scene from the dock or use **1–9** outside text entry fields. Use **Exit** to return to the workspace.

| Scene | View |
| --- | --- |
| 01 | Second Brain |
| 02 | Neural Funnel |
| 03 | Attention Spider |
| 04 | Quant Drift |
| 05 | Token Alignment |
| 06 | Orbital Collider |
| 07 | ANANSI Entity |
| 08 | SwarmTank |
| 09 | SOMNIA |

Scene **08** presents the SwarmTank persona matrix and scenario controls. Scene **09** presents the SOMNIA replay chamber, with 3D filaments, activity indicators, camera controls, pause and resume, and a fallback for systems without WebGL. The SOMNIA scene reflects replay and comparison state. Entering it and running offline replay require no model dispatch. [SwarmTank LARP handoff](docs/handoffs/larp-swarmtank.md) · [SOMNIA LARP handoff](docs/handoffs/larp-somnia.md)

### Data and provider boundaries

- The Windows application opens its local storage directly through the desktop host. The browser uses a paired local API session. The shared contracts keep the two clients on the same operations.
- Local workflows can run without a cloud model. When a workflow needs inference, review its evidence and provider settings before dispatch. Cloud routing uses the provider policy and authorization checks, including privacy classes and Zero-Cost Lock.
- Treat backup files and exported project artifacts as data-bearing files. Use a separate preview root and synthetic workspace when testing imports, recovery, or model behavior.
- Product Forge exports generated source for inspection and handoff. Generated-code execution and deployment require a separately approved execution boundary.
- LARP visualizations combine recorded workflow activity with clearly labelled visual effects. For example, Token Alignment compares prompt words and does not measure a model's attention weights.

## Windows preview

The verified preview combines SwarmTank v2, PDF text ingestion, LARP scenes 08 and 09, SOMNIA model comparison, and the OpenRouter reasoning fix. It is a development preview. Signed production installers, updater qualification, and broader release certification are tracked under M15. [Build status](docs/BUILD_STATUS.md) · [Roadmap](docs/ROADMAP.md)

| Recorded check at the verified preview | Result |
| --- | --- |
| TypeScript and generated contract drift | Zero errors and zero drift |
| UI and contract suite | 327 passed |
| Rust workspace library suite | 305 passed |
| Windows browser acceptance | 37 passed |
| SwarmTank coverage within the browser run | 17 passed |
| LARP and SOMNIA coverage within the browser run | 6 passed |
| Document and PDF coverage within the browser run | 14 passed |
| Native WebView2 desktop smoke test | Passed |

The native smoke test checked startup, Windows platform detection, the shell capability boundary, contract error parity, status refresh, and rendering. The browser run covered the listed product workflows. These counts refer to the recorded Windows verification at the source checkpoint above. [Verification ledger](docs/BUILD_STATUS.md)

Antigravity's Windows acceptance checkpoint `20664e187` adds a native Forge source review and revision run, a browser backup and fresh-restore check, focused Collider picker and lineage coverage, and document/PDF and MCP regressions. WebView2 file-event injection covers the application's picker handling. Interactive Windows file-dialog acceptance and the owner-led credential and vault checks are tracked separately. [Acceptance ledger](docs/BUILD_STATUS.md) · [Manual security checklist](docs/reviews/owner-manual-security-checklist.md)

## SOMNIA compaction

Muse's separate branch, `muse/somnia-compaction`, delivers the Checkpoint 1 compactor through `b0febf9e5f6ddf0e6c0fbd9320f3633345bef9ac`. Its bounded deterministic rules retain explicit deletion evidence, conflicts, uncertainty, and source identities. They separate unverified material and explain exclusions. The Windows rerun passed 23 module tests and all 40 frozen benchmark cases.

Checkpoint 2, delivered at `726aa8a07`, integrates **import → compact → inspect → approve → compare → save** into Projects and LARP scene 09. The server binds approval to the exact reviewed candidate and recomputes its hash before dispatch. Both arms use the selected model through the existing provider gates. Saved comparisons are private project artifacts. Entering LARP scene 09 starts no inference.

Muse's Linux verification passed 339 Rust library tests, 335 UI and contract tests, six synthetic workflow tests and the frozen benchmark. The synthetic provider scored both arms 21/50, with zero differing or failed pairs. The merged Windows preview also passed all nine focused browser tests and reproduced the synthetic benchmark. This checks the integration against a fixed oracle. The live-model rerun on automatically compacted inputs remains open. The compaction panel can run through a configured local model. Cloud comparisons require an authorization permit, which the current panel does not supply. [Checkpoint 2 implementation and verification](docs/handoffs/somnia-compaction-integration.md) · [Synthetic per-pair results](docs/evidence/somnia-compaction-checkpoint2-synthetic.json)

| Evidence | Scope |
| --- | --- |
| **40/40 compaction cases passed** | Library-level oracle checks. Covers updates, deletions, conflicting facts, missing information, look-alike names, Unicode, damaged rows, embedded instructions, and cases that require more context. |
| **11/48 → 38/48 correct answers** | Separate in-app live-model run on prepared histories using Nemotron 3.5 Super VL. Six paired batches completed and saved. |
| **64,215 → 10,330 total tokens** | Provider-reported input plus output tokens for the same prepared-history run, an 83.9% reduction for those inputs. |

The live benchmark included 36 synthetic file excerpts with broken OCR, stale exports, malformed rows, and embedded instructions. The preparation step supplied both histories. A separate live rerun is required to measure answer quality and token usage for app-generated candidates. [In-app comparison evidence](docs/handoffs/somnia-model-comparison.md) · [SOMNIA milestone plan](plans/08-somnia.md)

## Run from source

For the verified preview, clone the Windows integration branch.

```powershell
git clone --branch codex/windows-preview-integration https://github.com/robillionair/Anansi-ultimate.git
Set-Location Anansi-ultimate
node scripts/doctor.mjs --require-native
pnpm install --frozen-lockfile --ignore-scripts
```

The toolchain doctor checks Node, pnpm, and Git. Native builds also need the pinned Rust toolchain, Visual Studio C++ Build Tools, the Windows SDK, and WebView2. The current repository pins pnpm in `package.json` and Rust in `rust-toolchain.toml`. [Toolchain doctor](scripts/doctor.mjs) · [Package manifest](package.json) · [Rust toolchain](rust-toolchain.toml)

### Browser

In separate PowerShell sessions from the repository root, start the local API and the web client.

```powershell
pnpm dev:api
```

```powershell
pnpm dev:web
```

Open `http://127.0.0.1:1420` and pair it with the local API using the pairing flow. The browser client calls the authenticated loopback API. Keep the API running while using the paired browser session. [Desktop and browser architecture](docs/ARCHITECTURE.md)

### Windows desktop

Build the preview from the repository root and launch the resulting executable with a separate preview storage root.

```powershell
pnpm build:desktop
$env:ANANSI_STORAGE_ROOT = Join-Path $env:LOCALAPPDATA 'ANANSI-Preview/windows-integration'
& ./target/release/anansi-desktop.exe
```

Use a synthetic workspace for acceptance. For backup and restore, follow the documented recovery steps and preview the target before applying a restore. [Recovery and backup](docs/RECOVERY.md)

## Development and verification

| Command | Purpose |
| --- | --- |
| `node scripts/validate-docs.mjs` | Validate document links and roadmap consistency. |
| `node scripts/contracts.mjs --check` | Compare checked-in contracts with Rust schemas. |
| `pnpm typecheck` | Typecheck the shared UI and browser workspace. |
| `pnpm test:ui` | Run UI and contract tests. |
| `node scripts/cargo.mjs test --workspace --locked --lib` | Run the Rust workspace library suite. |
| `pnpm build:web` | Build production browser assets. |
| `pnpm build:desktop` | Build production browser assets and the desktop preview. |

For browser acceptance, build the web assets and API and install the Playwright browser. The test lease coordinates runs that share the application ports and resources.

```powershell
pnpm exec playwright install chromium
pnpm build:web
node scripts/cargo.mjs build -p anansi-api --locked
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test
```

For the focused Windows desktop smoke test, pass the built executable path to `node scripts/verify-desktop.mjs` under the shared application-test lease. On an elevated Windows host, use `node scripts/build-desktop-test.mjs` after building the web assets and pass the resulting debug executable instead. The verifier creates an isolated storage fixture. [Native desktop verifier](scripts/verify-desktop.mjs) · [Application-test lease](scripts/with-application-test-lease.mjs)

## Repository map

| Path | Responsibility |
| --- | --- |
| `apps/desktop` | Tauri host, native capabilities, desktop entry point, and development packaging. |
| `apps/web` | Vite and React browser client with local pairing. |
| `crates/core` | Workspace domain logic, storage, search, ingestion, artifacts, and evaluation workflows. |
| `crates/api` | Authenticated local HTTP interface and request routing. |
| `crates/providers` | Provider dispatch, routing, and provider-specific wire handling. |
| `crates/contracts` | Rust schema generator for shared API contracts. |
| `packages/contracts` | Generated TypeScript contracts and validators, plus shared clients. |
| `packages/ui` | Shared React screens, components, and LARP scenes. |
| `tests/e2e` | Playwright product workflows. |
| `docs`, `plans` | Specification, decisions, acceptance evidence, and milestone plans. |

## Project roadmap

The preview carries the desktop and web foundation, local memory and retrieval, model dispatch controls, opportunity discovery, Forge and LEXI-COLLIDER workflows, SwarmTank, SOMNIA, GHOST-PROMPT, the product lifecycle, and the Home and LARP interfaces. Remaining roadmap work includes broader document and connector coverage, release qualification, and the live-model qualification of SOMNIA compaction. The requirements matrix tracks scope and evidence at a finer level than the feature list.

Start with the [master specification](docs/MASTER_SPEC.md) for behavior, [roadmap](docs/ROADMAP.md) for sequencing, [requirements matrix](docs/REQUIREMENTS_MATRIX.md) for scope, [architecture](docs/ARCHITECTURE.md) and [decisions](docs/DECISIONS.md) for design, and [build status](docs/BUILD_STATUS.md) for verification.
