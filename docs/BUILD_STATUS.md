# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0 verified complete. M1.0 plan/doctor complete. M1.1 has partial progress and an actual administrator-install blocker. M1.2/product implementation has not started.

Entering verified source checkpoint a6a848e7c2786bc73c56d2337b6f9fa171514290. Active task and next task remain M1.1. No M1 completion tag or product capability completion is claimed.

## M1.1 implemented

- Installed official-checksum-verified Rust 1.99.1 x86_64-pc-windows-msvc with Cargo/rustfmt/clippy in user-local directories. Pinned rust-toolchain.toml; no global PATH changes, package workspace or Cargo.lock.
- Doctor now discovers user-local Rust/CARGO_HOME tools. Native verification compiles a known synthetic program and attempts linking/execution, with logs retained outside checkout.
- Prepared tracked Microsoft C++ installation command with read-only default, explicit -Install/admin guard, Microsoft signature verification and no auto-elevation/restart.
- Added Windows native toolchain smoke CI job alongside both existing protected Foundation checks. CI success is separate from local readiness.

## Actual verification and open acceptance

Rust/Cargo/component version checks succeeded. Rust metadata compilation succeeded. Full local MSVC linking failed: link.exe not found. Doctor native mode still returns 1 for missing C++/SDK. Default install plan passed; -Install correctly refused the non-elevated process before installing. No local smoke executable exists or has been run.

[M1 evidence](evidence/M1.md) records exact hashes, commands, installer signature, versions and retained logs. Existing master specification remains intact. Tests/CI/publication results are recorded after actual execution.

## Required next action

M1.1 requires an administrator-capable PowerShell to run scripts/install-msvc.ps1 -Install, followed by local pnpm doctor:native and pnpm verify:native. Current machine process is not elevated; no UAC request or security-setting change is initiated. Follow [development setup](DEVELOPMENT.md) for the concrete command.

Until both local checks pass, do not start M1.2. No desktop/browser runtime, installer, storage, provider integration or paid inference has been implemented/tested. Work remains within M1; later milestones unchanged.
