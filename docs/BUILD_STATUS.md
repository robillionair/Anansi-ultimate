# ANANSI Ultimate — Build Status

## Current state — 2026-10-16

M0 and M1.0–M1.5 are verified complete. M1 completion checkpoint 97babb3ae9f3dc20b5602a7b46a947e833943eb2 passed all three CI jobs in run 38955878511, was published to protected master and tagged m1-desktop-foundation-verified. Final source bundle restored identical HEAD/tree/tag and passed documentation checks. The downloaded CI installer passed the full local installed-release native/browser workflow and data-preserving uninstall.

M2.1 is verified complete. Implementation checkpoint 39b0e5ae440223a87ac394c60706160a8f237ca6 passed all three jobs in [CI run 38962588037](https://github.com/robillionair/Anansi-ultimate/actions/runs/38962588037), including actual native memory verification and release packaging/install/uninstall. It was published to protected master; independent source restore matched HEAD/tree and passed frozen install, types, client/tooling/recovery tests. Last verified implementation SHA is recorded rather than this metadata commit to avoid self-reference. See [M2.1 evidence](evidence/M2-1.md). Stop before M2.2.

## M1 certification and recovery

- Actual current-user unsigned NSIS install, full installed native/browser onboarding/workspace/settings/process-restart workflow and uninstall passed. Executable/uninstaller/registration removed, private synthetic catalog files preserved byte-for-byte. Actual screenshots inspected; no real user catalog was targeted.
- CI artifact 12055044353 contains only release setup, SHA256SUMS and two JSON evidence files. Downloaded ZIP digest, installer checksum and clean source identity were verified. CI's installed-release mode explicitly skips UI on elevated hosts; separate native UI checks and actual local downloaded-installer UI tests passed.
- 40 Node foundation/tool tests, 13 source-recovery checks, 26 Rust tests, 11 client/component tests and seven browser workflows passed. Native discovery/compile/link/run, formatting, Clippy, types, contract drift, frozen installation and docs/whitespace checks passed. Initial bundle-marker/overlapping-test failures were repaired and retained as history.
- Independent source bundle restored matching HEAD/tree; restored docs, frozen install, types, 11 client/component tests, 40 foundation/tool tests and 13 recovery checks passed. Source backups exclude private catalogs, credentials and installers. Master specification and all 90 section owners, 16 plans and 33 requirements are retained.

## Remaining scope and next gate

M1 is development-foundation certification only. AN-003 shared foundation design and AN-023 local onboarding/workspace/navigation/settings have verified workflow evidence. AN-001/002/018/020 retain release/security/later-platform scope and remain in progress. Signed installers, updater and clean-machine certification remain M15; no fully offline runtime installation claim. Existing WebView2 supports tested offline operation; its embedded bootstrapper needs internet if the runtime is absent.

M2.1 local acceptance: 38 Rust tests, 12 client/component tests, eight browser workflows, actual native/browser memory workflow and both process restarts passed. Tooling/recovery checks passed. Proposed/user-authored memory revisions are implemented; notes/lifecycle, vault, jobs, export and automated backups remain later tasks. M2.6 vault approval remains separate. M2.1 acceptance/publication/recovery gates passed. M2 remains incomplete; next task is M2.2 notes/lifecycle, unstarted.
