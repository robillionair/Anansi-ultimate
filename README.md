# ANANSI Ultimate

ANANSI is a local-first intelligence workspace planned for a shared Tauri desktop application and browser application.

## Repository status

M0 source recovery/CI and M1.0–M1.3 foundations are verified. M1.4 adds shared first-run onboarding, privacy review, skippable AI setup, navigation and durable intent/motion settings; see [build status](docs/BUILD_STATUS.md) for verification and checkpoint results. Memory, AI and the installer remain later tasks. See [development instructions](docs/DEVELOPMENT.md), [the roadmap](docs/ROADMAP.md) and [coverage ledger](docs/SPEC_COVERAGE.md).

## Workspace layout

- `apps/desktop` — Tauri desktop application
- `apps/web` — browser application
- `crates/` — Rust domain and API services
- `packages/` — shared UI and API contracts
- `docs/` — durable product and engineering memory
- `plans/` — dependency-ordered implementation phases
- `tests/` — cross-layer and acceptance tests

See `docs/BUILD_STATUS.md` for current verification state.
