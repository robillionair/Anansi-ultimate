# ANANSI Ultimate — Agent Instructions

## Scope

ANANSI is a local-first product delivered as a Tauri 2 Windows desktop application and a browser application. Both editions must share domain models, business rules, visual language, and API contracts.

## Durable context

The repository documents are authoritative project memory. Before implementation, read `docs/MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, `docs/REQUIREMENTS_MATRIX.md`, `docs/BUILD_STATUS.md`, the active plan in `plans/`, and relevant decisions in `docs/DECISIONS.md`.

## Execution policy

- Work on one approved phase at a time.
- Define acceptance criteria before coding.
- Prefer a thin end-to-end vertical slice over disconnected screens or stubs.
- Add tests with every new behavior and run relevant existing tests.
- Do not claim a requirement complete without a verified user workflow and recorded evidence.
- Preserve postponed requirements in the matrix with status and rationale.
- Do not expose native filesystem or process capabilities to an untrusted browser.
- Do not transmit private or sealed memories without explicit authorization.
- Do not hardcode credentials or fabricate provider integrations.
- Do not automatically run generated software or trigger paid inference.

## Change and recovery conventions

Use small, reviewable commits when authorized. Record verification evidence and the last verified commit in `docs/BUILD_STATUS.md`. Follow `docs/RECOVERY.md` when resuming or recovering work.

## Required validation

At each phase boundary, validate documentation consistency, run the phase test/build commands, inspect the actual application behavior where applicable, and update status before stopping.
