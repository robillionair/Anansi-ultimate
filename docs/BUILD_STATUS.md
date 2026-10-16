# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0 remains verified complete at annotated tag m0-foundation-verified (268533e4c16bbf4693fd4e4981354a4f5f7eb273). Current authorized phase is M1 under the user's continuation instruction. M1.0 detailed planning and read-only readiness audit is complete; M1 is not complete. Next task is M1.1 native toolchain installation/compile-link verification.

Last entering verified checkpoint: 268533e4c16bbf4693fd4e4981354a4f5f7eb273. Current M1.0 publication/evidence is recorded after CI succeeds; metadata may retain a prior verified SHA deliberately.

## M1.0 implementation and verification

- Detailed plan in plans/PHASE-001-runnable-shell.md preserves Tauri/React, browser/API boundary, SQLite/workspace/settings, onboarding/design/navigation and actual development NSIS .exe acceptance. Six gated tasks; no M2 work.
- Tracked read-only toolchain doctor has foundation and required-native modes; six fixture tests cover missing/version/platform/SDK and Node floor behavior. It never claims buildVerified=true.
- Actual doctor:toolchain exit 0; doctor:native exit 1 with missing Rust MSVC host, Cargo, C++ tools and SDK headers/x64 libraries. WebView2 version directories detected, runtime launch untested. The nonzero native result is an open prerequisite gap, not a passed native build.
- No system tools installed and no runtime/product packages or dependencies added. Product build remains unavailable. Full test/CI evidence in [M1 evidence](evidence/M1.md).
- M0 foundation recovery/tests preserved; master specification content unchanged, 90 sections and 33 requirements retained. No product requirement marked complete by the audit.

## Next task, blockers and recovery

M1.1 must install or locate official Windows/Rust prerequisites and prove compile/link; then M1.2 builds the first actual shared readiness workflow. Do not skip the security/transport gates or present a mock app as implemented. Current native gaps are recorded in project-state.json. No desktop/browser app, local API, persistence, installer or provider integration is implemented.

Use the protected CI branch flow and scripts/backup-source.ps1 after clean commits. Preserve M0 tag and existing source; future checkpoints must not overwrite earlier bundles. Stop at task boundaries and before another milestone.
