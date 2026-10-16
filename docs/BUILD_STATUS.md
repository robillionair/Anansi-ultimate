# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0 and M1.0–M1.5 are verified complete. M1 delivers the actual per-user x64 NSIS development installer and shared browser/workspace foundation. Source checkpoint 7eebcd0 passed all three jobs in CI run 38922142058, including release packaging/install/uninstall/artifact upload, and was published to protected master. The downloaded CI installer also passed the full local installed-release native/browser workflow and data-preserving uninstall. Stop before M2.

Verified implementation checkpoint: 7eebcd0a7f6977a951757a65ff90a794b7c2867b. This completion record references the prior verified source SHA to avoid self-reference. Annotated completion tag name: m1-desktop-foundation-verified; create/push it only after this completion record's CI succeeds. See [M1.5 evidence](evidence/M1-5.md), [M1 history](evidence/M1.md) and [completed plan](../plans/PHASE-001-runnable-shell.md).

## M1 certification and recovery

- Actual current-user unsigned NSIS install, full installed native/browser onboarding/workspace/settings/process-restart workflow and uninstall passed. Executable/uninstaller/registration removed, private synthetic catalog files preserved byte-for-byte. Actual screenshots inspected; no real user catalog was targeted.
- CI artifact 12055044353 contains only release setup, SHA256SUMS and two JSON evidence files. Downloaded ZIP digest, installer checksum and clean source identity were verified. CI's installed-release mode explicitly skips UI on elevated hosts; separate native UI checks and actual local downloaded-installer UI tests passed.
- 40 Node foundation/tool tests, 13 source-recovery checks, 26 Rust tests, 11 client/component tests and seven browser workflows passed. Native discovery/compile/link/run, formatting, Clippy, types, contract drift, frozen installation and docs/whitespace checks passed. Initial bundle-marker/overlapping-test failures were repaired and retained as history.
- Independent source bundle restored matching HEAD/tree; restored docs, frozen install, types, 11 client/component tests, 40 foundation/tool tests and 13 recovery checks passed. Source backups exclude private catalogs, credentials and installers. Master specification and all 90 section owners, 16 plans and 33 requirements are retained.

## Remaining scope and next gate

M1 is development-foundation certification only. AN-003 shared foundation design and AN-023 local onboarding/workspace/navigation/settings have verified workflow evidence. AN-001/002/018/020 retain release/security/later-platform scope and remain in progress. Signed installers, updater and clean-machine certification remain M15; no fully offline runtime installation claim. Existing WebView2 supports tested offline operation; its embedded bootstrapper needs internet if the runtime is absent.

M2.0 detailed planning and approval is the next gate and has not started. Do not implement Memory OS automatically. Memory, inference, hardware/model/provider setup, vault/encryption and automatic user-data backups remain their planned milestones. There is no M1 blocker.
