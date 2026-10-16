# ANANSI Ultimate

ANANSI is a local-first intelligence workspace planned for a shared Tauri desktop application and browser application.

## Repository status

M0 source recovery and CI, M1.0 planning, M1.1 native tooling and M1.2 shared readiness are verified. M1.3 adds local workspace creation/reopening and durable reduced-motion settings shared by desktop and browser; see [build status](docs/BUILD_STATUS.md) for verification and checkpoint results. Memory, AI, onboarding/navigation and the installer remain later tasks. See [development instructions](docs/DEVELOPMENT.md), [the roadmap](docs/ROADMAP.md) and [coverage ledger](docs/SPEC_COVERAGE.md).

## Workspace layout

- `apps/desktop` — Tauri desktop application
- `apps/web` — browser application
- `crates/` — Rust domain and API services
- `packages/` — shared UI and API contracts
- `docs/` — durable product and engineering memory
- `plans/` — dependency-ordered implementation phases
- `tests/` — cross-layer and acceptance tests

See `docs/BUILD_STATUS.md` for current verification state.
