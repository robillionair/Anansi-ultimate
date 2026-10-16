# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0 and M1.0–M1.5 are verified complete. M1 completion checkpoint 97babb3ae9f3dc20b5602a7b46a947e833943eb2 passed all three CI jobs in run 38955878511, was published to protected master and tagged m1-desktop-foundation-verified. Final source bundle restored identical HEAD/tree/tag and passed documentation checks. The downloaded CI installer passed the full local installed-release native/browser workflow and data-preserving uninstall.

Last verified checkpoint: 97babb3ae9f3dc20b5602a7b46a947e833943eb2; product implementation source 7eebcd0a7f6977a951757a65ff90a794b7c2867b. The subsequent user continuation authorizes M2.0 detailed planning only. The [Memory OS plan](../plans/02-memory-system.md) is prepared for acceptance approval before M2.1 coding. See [M2.0 evidence](evidence/M2-0.md), [M1.5 evidence](evidence/M1-5.md) and [completed M1 plan](../plans/PHASE-001-runnable-shell.md). No Memory OS behavior is implemented by this planning checkpoint.

## M1 certification and recovery

- Actual current-user unsigned NSIS install, full installed native/browser onboarding/workspace/settings/process-restart workflow and uninstall passed. Executable/uninstaller/registration removed, private synthetic catalog files preserved byte-for-byte. Actual screenshots inspected; no real user catalog was targeted.
- CI artifact 12055044353 contains only release setup, SHA256SUMS and two JSON evidence files. Downloaded ZIP digest, installer checksum and clean source identity were verified. CI's installed-release mode explicitly skips UI on elevated hosts; separate native UI checks and actual local downloaded-installer UI tests passed.
- 40 Node foundation/tool tests, 13 source-recovery checks, 26 Rust tests, 11 client/component tests and seven browser workflows passed. Native discovery/compile/link/run, formatting, Clippy, types, contract drift, frozen installation and docs/whitespace checks passed. Initial bundle-marker/overlapping-test failures were repaired and retained as history.
- Independent source bundle restored matching HEAD/tree; restored docs, frozen install, types, 11 client/component tests, 40 foundation/tool tests and 13 recovery checks passed. Source backups exclude private catalogs, credentials and installers. Master specification and all 90 section owners, 16 plans and 33 requirements are retained.

## Remaining scope and next gate

M1 is development-foundation certification only. AN-003 shared foundation design and AN-023 local onboarding/workspace/navigation/settings have verified workflow evidence. AN-001/002/018/020 retain release/security/later-platform scope and remain in progress. Signed installers, updater and clean-machine certification remain M15; no fully offline runtime installation claim. Existing WebView2 supports tested offline operation; its embedded bootstrapper needs internet if the runtime is absent.

M2.0 plan preparation is complete; acceptance approval remains pending. Next task is M2.1 only after approval. M2.6 remains a separate audited-vault design/approval gate; its implementation is required before M2 certification. Memory, jobs, export, encrypted vault and user-data backups are planned, not implemented. No product requirement status advances on documentation evidence. There is no M1 blocker.
