# Windows Review Report: MCP Client Integration Slice 1 (Milestone 13)

- **Date**: 2026-11-04
- **Role**: Lead Integrator & Independent Windows Reviewer
- **Target Branch**: `muse/mcp-slice1` -> `codex/review-mcp-slice1`
- **Baseline Commit**: `34f39bb7da54e70c3fc25ed9dbf0791cf4bab197` (Dashboard Slice 2 Windows-verified review tip)
- **Delivery Tip Commit**: `4b0b42e0077802490ee468b2c7304e28fd0f9de7`
- **Delivery ZIP**: `anansi-mcp-slice1-2026-11-04.zip` (uploaded as `media_1793760917417_65d14f0f.zip`)
- **Delivery ZIP SHA-256**: `b008ef8b4efa069e9cc74672f3bbef730b8a891ac7d0cf0d0b56b93502e7493c`
- **Delivery Bundle**: `anansi-mcp-slice1.bundle`
- **Delivery Bundle SHA-256**: `5ef8eac06812f5b17d101ddd1c6d8f56d6782efb23084e6c148101f0420b1015`
- **Verdict**: **PASS WITH FOCUSED WINDOWS CORRECTION** (Windows environment preservation in `StdioChannel::spawn`; all 50 browser tests, 289 UI tests, 220 Rust tests green)

---

## 1. Scope & Delivered Components

Muse delivered Slice 1 of the Model Context Protocol (MCP) Client Integration per MASTER_SPEC §68 (Milestone 13):
1. **MCP Registry & Configuration Model (`packages/ui/src/Mcp.tsx`, `crates/core/src/mcp.rs`)**:
   - Registered server metadata: name, transport (`stdio` or `sse`), command, arguments, environment key/value pairs, and optional remote URL.
   - Per-workspace durable file storage in `storage/mcp-servers.json` using atomic replace. Environment variable values are strictly withheld from UI queries (only `envKeys` returned) to prevent secret leakage.
   - Max 64 MCP servers per workspace; form validation checks names, transport parameters, and URL formats.
2. **Local Stdio Probe & Negotiation Engine (`crates/core/src/mcp.rs`)**:
   - One-shot command spawn over local `stdio` with zero persistent background daemons, zero auto-connect, and zero polling.
   - Dual-protocol negotiation: tries 2025-11-25 `initialize` + `notifications/initialized` first; falls back cleanly to 2026-07-28 stateless `server/discover` only on `-32601` (method not found).
   - Tool and prompt listing (`tools/list`, `prompts/list`) with input schema display, annotation badges (`Read-only`, `Destructive`, `Open-world`), and a 100-item truncation cap.
   - Health check: reports genuine health method (`ping` for 2025-11-25, `discover` for 2026-07-28); never fabricates a ping for stateless servers.
3. **Explicit Refusal Boundaries**:
   - Remote `sse` servers can be registered for future expansion, but probes are explicitly refused (`Only local stdio servers can be connected.`) under §68 default-deny and §71 SSRF policies.
   - Zero tool execution in Slice 1 (`tools/call` deferred to future slice requiring explicit per-call authorization).
4. **Navigation Integration (`packages/ui/src/Navigation.tsx`)**:
   - Promoted `MCP` from planned tools to a live navigation button under `SYSTEM`. Sidebar buttons updated from 16 to 17.

---

## 2. Independent Windows Finding & Focused Correction

During initial Playwright E2E verification on Windows, test 2 (`MCP stdio registration probes a real local server`) failed:
- **Diagnostic Finding**: `StdioChannel::spawn` invoked `.env_clear()`, stripping all environment variables. On Windows, Node.js uses Windows CNG / CryptoAPI for runtime startup CSPRNG initialization (`ncrypto::CSPRNG(nullptr, 0)`). Without `SystemRoot` / `WINDIR` pointing to `C:\Windows`, Windows DLL loading fails and Node aborts with exit code 134 in 0.11s.
- **Secondary Finding**: `mcp.rs` mapped any `recv_timeout` error (including `Disconnected` when child crashes) to `"Probe timed out waiting for the server."`, masking process termination as a timeout.
- **Focused Review Correction**:
  1. In `crates/core/src/mcp.rs` (`StdioChannel::spawn`), on Windows (`#[cfg(windows)]`), preserve `SystemRoot`, `WINDIR`, and fallback `PATH` (if not explicitly overridden in `server.env`) to allow standard Windows runtimes to boot their system libraries while maintaining strict credentials isolation.
  2. In `StdioChannel::request`, distinguish `mpsc::RecvTimeoutError::Timeout` from `mpsc::RecvTimeoutError::Disconnected` (`Server exited or closed connection before replying.`).
- **Post-Correction Result**: `mcp.spec.ts` passed 4/4 in 31.2s; full regression battery passed 50/50 in 9.8m.

---

## 3. Windows Verification Suite Results

| Test Category | Command | Result | Details |
|---|---|---|---|
| **TypeScript Typecheck** | `npm run typecheck` (`tsc --noEmit`) | **PASS** | 0 errors |
| **Contract Parity** | `node scripts/contracts.mjs --check` | **PASS** | 0 schema drift |
| **Documentation Integrity** | `node scripts/validate-docs.mjs` | **PASS** | 268 markdown files, 90 owners |
| **UI Component Suite** | `npm run test:ui` | **PASS** | **289/289 passed** in 10.8s (5 new tests in `mcp.test.tsx`) |
| **Vite Production Build** | `apps/web/node_modules/vite/bin/vite.js build` | **PASS** | Clean build in 3.21s |
| **Rust Core Workspace** | `node scripts/cargo.mjs test --workspace --locked --lib` | **PASS** | **220/220 passed** (4 API + 155 Core + 1 Desktop + 60 Providers) |
| **Playwright MCP Spec** | `tests/e2e/mcp.spec.ts` under test lease | **PASS** | **4/4 passed** in 31.2s (Empty registry, Local stdio probe, SSE refusal, Form validation) |
| **Playwright Full Battery** | Full 50-test suite under test lease | **PASS** | **50/50 passed** in 9.8m across all milestone suites |

---

## 4. Verification Evidence & Certification

- **Certified Review Branch**: `codex/review-mcp-slice1`
- **Certified Review Commit**: Recorded upon review commit
- **Verification Status**: Certified for Windows NTFS with platform correction. Milestone 13 Slice 1 is **VERIFIED & CLOSED**. `AN-017` is in progress.
