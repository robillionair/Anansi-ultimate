# ANANSI Ultimate

ANANSI is a local-first intelligence workspace planned for a shared Tauri desktop application and browser application.

## Repository status

M0 recovery/CI and M1 desktop/browser foundation are verified. The per-user Windows NSIS development installer supports actual onboarding, workspace creation/reopening and durable settings shared with the browser; see [build status](docs/BUILD_STATUS.md). Memory, AI and signed production release remain later milestones. M2 planning/approval is a separate gate and has not started. See [development instructions](docs/DEVELOPMENT.md), [the roadmap](docs/ROADMAP.md) and [coverage ledger](docs/SPEC_COVERAGE.md).

## Workspace layout

- `apps/desktop` — Tauri desktop application
- `apps/web` — browser application
- `crates/` — Rust domain and API services
- `packages/` — shared UI and API contracts
- `docs/` — durable product and engineering memory
- `plans/` — dependency-ordered implementation phases
- `tests/` — cross-layer and acceptance tests

See `docs/BUILD_STATUS.md` for current verification state.
