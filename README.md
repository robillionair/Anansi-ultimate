# ANANSI Ultimate

ANANSI Ultimate is a local-first product creation and intelligence workspace delivered as a Tauri 2 Windows desktop application and paired browser application. Both editions share domain models, business rules, visual design language, and API contracts.

---

## Architecture & Principles

- **Local-First & Offline by Default**: Core features operate entirely offline on the local machine with zero telemetry, zero hidden beacons, and zero required cloud dependencies.
- **Dual-Edition Parity**: Shared TypeScript/Rust contracts (`packages/contracts`) guarantee identical behavior, validation, and serialization across the Tauri native host and the web frontend.
- **Strict Privacy Boundaries**: Private and sealed memories are enforced with fail-closed refusal before model dispatch. Features with sensitive data capture (such as GHOST-PROMPT draft exploration) require explicit informed opt-in and operate in volatile memory only.
- **Deterministic & Model-Free Verification**: Complex simulation and evaluation workflows (SwarmTank, SOMNIA, GHOST-PROMPT analysis) feature byte-identical reference implementations in Rust and TypeScript, ensuring testability without paid inference or network flakiness.

---

## Milestone Progress & Capability Status

| Milestone | Capability | Status | Key Features |
|---|---|---|---|
| **M0** | Foundation & CI | Verified | Project structure, validation harnesses, recovery runbooks |
| **M1** | Desktop & Web Shell | Verified | Tauri 2 shell, Vite/React web client, per-user NSIS installer |
| **M2** | Encrypted Vault & Memory | Verified | Argon2id/AES-GCM encrypted storage, lifecycle notes, graph relations, authenticated backups |
| **M3** | Search & Embeddings | Verified | BM25 lexical + vector semantic hybrid retrieval with Reciprocal Rank Fusion (RRF), Ollama loopback |
| **M4** | AI Matrix & Privacy Boundary | Verified | Provider registry, host approval queue, token/cost budgets, credential isolation |
| **M5** | Provider Runtime & Dispatches | Verified | Streaming dispatch, mock offline fallback, retry fences, cross-process pairing |
| **M6** | Opportunity Discovery | Verified | Market opportunity ranking, hypotheses generator, rubric evaluation |
| **M7** | Product Forge | In Progress | Specification derivation, plan generation, prototype packages, model-free source review |
| **M8** | LEXI-COLLIDER | In Progress | Multi-beam language collision, pinned project/memory context, approved Forge transfer |
| **M9** | SwarmTank 3D | Verified | Synthetic persona generator, seven-factor decision engine, Three.js 3D amphitheater arena, scenario branching |
| **M10** | SOMNIA | Verified | Deterministic replay evaluator, seeded nightmare generator, consolidation proposals, Forge QA bridge, dashboard |
| **M11** | GHOST-PROMPT | Verified | Privacy-first draft exploration, in-memory revision capture, Three.js 3D timeline, rule-based draft analysis, dual-panel comparison |
| **M12** | Product Lifecycle | Verified | 7-stage pipeline engine & navigator, Launch Studio with deterministic ZIP bundling, Guided Lifecycle Walk with recovery |
| **LARP** | LARP Mode Telemetry Engine | Verified | Real-time telemetry bus & HUD, 6 visual views (Memory Vault 3D, Context Funnel, Kinetic Spider, Quant Drift Spectrum, Attention Matrix, Collider Field) |
| **HOME** | Command Dashboard | Verified | Landing hub with genuine uptime telemetry, recency-ordered projects hub with Guided Walk jump, system backup and queue health |
| **M13** | MCP Integration | Verified | Stdio MCP client (tools, resources, prompts) with explicit review; MCP server exposing 7 tools (6 read-only + mutation-gated ideas.create) with pairing auth & strict privacy |

See [docs/BUILD_STATUS.md](docs/BUILD_STATUS.md) and [docs/REQUIREMENTS_MATRIX.md](docs/REQUIREMENTS_MATRIX.md) for detailed verification evidence.

---

## Workspace Layout

- `apps/desktop` — Tauri 2 desktop application (Windows native host)
- `apps/web` — Web application (Vite + React)
- `crates/` — Rust services:
  - `crates/core` — Core domain logic, memory, search, SwarmTank, SOMNIA, GHOST-PROMPT
  - `crates/api` — Local HTTP/IPC server and pairing endpoints
  - `crates/contracts` — Schema and DTO code generators
- `packages/` — Shared TypeScript packages:
  - `packages/contracts` — Generated TypeScript interfaces and validators
  - `packages/ui` — Reusable React UI components and Three.js scenes
- `docs/` — Authoritative project documentation, architecture, and verification reviews
- `plans/` — Milestone roadmaps and implementation plans
- `tests/` — End-to-end browser suites (Playwright) and contract tests

---

## Development & Verification

### Prerequisites
- Node.js (v20+ or v22 LTS) & pnpm
- Rust toolchain (`stable-x86_64-pc-windows-msvc`)
- Visual Studio C++ Build Tools

### Common Commands
```bash
# Validate documentation links and matrix consistency
node scripts/validate-docs.mjs

# Check contract drift against Rust schemas
node scripts/contracts.mjs --check

# Typecheck workspace
pnpm exec tsc --noEmit

# Run UI component test suite (297 unit tests)
npm run test:ui

# Run Rust workspace unit and integration tests
cargo test --workspace --locked --lib

# Build production web bundle
pnpm --filter @anansi/web build

# Run Playwright browser test battery (59 tests across M0–M13)
pnpm exec playwright test
```

For detailed architectural decisions and recovery guidelines, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/RECOVERY.md](docs/RECOVERY.md).
