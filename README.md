# ANANSI Ultimate

ANANSI is a local-first intelligence workspace planned for a shared Tauri desktop application and browser application.

## Repository status

M0 foundation is complete: repository hygiene, validation, Windows/Linux CI, branch protection and verified independent source recovery. The annotated m0-foundation-verified checkpoint marks the boundary. M1.0 and M1.1 are complete: local Rust/MSVC compilation, linking and execution are verified. Next is M1.2 shared desktop/browser readiness implementation. No product implementation has started. See [the completed M0 plan](plans/PHASE-000-foundation.md). Product implementation and desktop/web launch are not yet available. See [development instructions](docs/DEVELOPMENT.md), [the roadmap](docs/ROADMAP.md) and [coverage ledger](docs/SPEC_COVERAGE.md).

## Workspace layout

- `apps/desktop` — Tauri desktop application
- `apps/web` — browser application
- `crates/` — Rust domain and API services
- `packages/` — shared UI and API contracts
- `docs/` — durable product and engineering memory
- `plans/` — dependency-ordered implementation phases
- `tests/` — cross-layer and acceptance tests

See `docs/BUILD_STATUS.md` for current verification state.
