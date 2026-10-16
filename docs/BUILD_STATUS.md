# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0 verified complete. M1.0 and M1.1 are complete; next task is M1.2 shared desktop/browser readiness. M1 as a whole remains incomplete; no M1 completion tag. No product application has been implemented.

Verified M1.1 source checkpoint: 7125d9cb329d0cbe19c50a5178f18e8f42a5fa80, published to protected master after required CI. Project-state records completed M1.1 and next M1.2 with no current toolchain blocker.

## M1.1 acceptance passed locally

- User completed the Microsoft C++ installation, exit 0; no automatic restart. vswhere confirms complete installation, no reboot required.
- Rust/Cargo 1.99.1 MSVC with rustfmt/Clippy; VS Build Tools 17.14.42, MSVC 14.44.35207, Windows SDK 10.0.26100.0 measured.
- `pnpm doctor:native`: exit 0, no missing prerequisites. Fixed stale SDK registry-root detection and added regression test requiring real UM/UCRT headers/libraries.
- `pnpm verify:native`: exit 0; actual compile, link and execution returns ANANSI_NATIVE_SMOKE_OK. Logs retained outside source; paths and versions in [M1 evidence](evidence/M1.md).
- Doctor discovery alone still does not claim a verified build. The separate smoke executable supplies build evidence.

## Verification and remaining scope

Full local documentation/foundation/recovery regressions passed: 36 Node tests, 13 PowerShell recovery checks, frozen install and whitespace checks. Actual CI run 38847062467 passed both Foundation runners and Windows native smoke. Original master specification and architecture preserved. No new system installation or restart by this session.

M1.2 is the next approved task. Real Tauri/WebView2 embedding, authenticated local API, shared contracts/UI, persistence and development installer acceptance are not yet implemented or verified. Toolchain success does not complete those product requirements. M2 and later scope unchanged.
