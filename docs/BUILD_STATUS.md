# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0 and M1.0–M1.4 are verified complete. M1.5 delivers an actual current-user x64 NSIS development installer. Local installed-release acceptance passes; exact-checkpoint CI/artifact publication and independent source recovery are pending. M1 has no completion tag yet; do not start M2.

Prior verified checkpoint: 710ad9ba8499d40ce06a47b350a457e945828010. The completion record will reference the verified installer-source checkpoint after CI. See [M1.5 evidence](evidence/M1-5.md), [M1.4 history](evidence/M1-4.md) and [active plan](../plans/PHASE-001-runnable-shell.md).

## M1.5 acceptance and verification

- Actual unsigned NSIS setup built (4.28 MiB); installer and installed-payload checksums are recorded. Per-user installation needs no requested elevation, blocks downgrades, and embeds the Microsoft WebView2 bootstrapper.
- Installed into a separate generated temporary location. Actual installed Windows application and browser passed onboarding, workspace/settings, cross-adapter intent/motion and both-process restart. Uninstall removed executable/uninstaller/registration and retained byte-identical private synthetic catalog files. Actual screenshots inspected.
- Altered-checksum/existing-install preflight refusals and occupied-port refusal passed. Initial bundle-marker checksum and overlapping-test failures were diagnosed and repaired; originals/failure records retained externally.
- 40 Node foundation/tool tests, 13 source-recovery checks, 26 Rust tests, 11 client/component tests and seven production-browser workflows passed. Native doctor and actual MSVC compile/link/run passed. Types, formatting, Clippy with warnings denied, contract drift, frozen install, docs and whitespace checks are recorded in evidence.
- CI preserves Foundation Windows/Linux names, adds release packaging/install/uninstall checks and pinned artifact upload. Elevated CI explicitly skips installed-release UI; its separate native UI tests remain. Local installed-release UI was actually tested.

## Remaining scope and next gate

M2.0 is the detailed planning/approval gate only and has not started. Memory, inference, model/provider setup, encryption/vault and automatic user-data backups remain future work. AN-001 signed/clean-machine release, browser release, broader security and M15 certification remain open. The unsigned development installer is not a signed release or updater. Existing WebView2 supports offline operation; without it the embedded bootstrapper needs internet. Master specification and all 90 section owners, 16 plans and 33 requirements are preserved.
