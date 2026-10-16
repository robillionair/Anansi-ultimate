# ANANSI Ultimate

ANANSI is a local-first intelligence workspace planned for a shared Tauri desktop application and browser application.

## Repository status

This repository contains the planning foundation and completed M0.1 repository hygiene/development contract. Next is M0.2 in [the approved M0 plan](plans/PHASE-000-foundation.md). Product implementation and desktop/web launch are not yet available. See [development instructions](docs/DEVELOPMENT.md), [the roadmap](docs/ROADMAP.md) and [coverage ledger](docs/SPEC_COVERAGE.md).

## Workspace layout

- `apps/desktop` — Tauri desktop application
- `apps/web` — browser application
- `crates/` — Rust domain and API services
- `packages/` — shared UI and API contracts
- `docs/` — durable product and engineering memory
- `plans/` — dependency-ordered implementation phases
- `tests/` — cross-layer and acceptance tests

See `docs/BUILD_STATUS.md` for current verification state.
