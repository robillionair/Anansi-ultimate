# v1 Windows installer workflow qualification — 2026-11-13

Verdict: PASS for isolated same-host install/installed UI/uninstall/catalog preservation after test-selector repair. Broader pilot approval remains open.

Application source `6f216e3224e2c855236aac5492f69b467cf8c5d5`, unchanged. Installer SHA-256 `2b0c1c7980b7cf8c62b1d9b621723276ef943208d988ea579a62494da5408d74`; installed payload SHA-256 `2f6bbd2b0572fca2a8e9d3d9fc616d6667a962b193c72eb7ee5ecd3ff6816580`. No binary rebuild or source-to-binary relabelling.

## Safety and before evidence

All known ANANSI uninstall registrations and remembered-location key absent, host non-elevated, test lease free. Ran existing installer verifier with explicit CARGO_TARGET_DIR and E: TestRoot under the shared application-test lease, no SkipUi. First install and payload identity passed, but installed workflow failed on stale heading expectation at verify-workspace.mjs:49: Synthetic native workspace was expected as a heading after onboarding. Actual app showed Dashboard and the name within Workspace overview. First failure log and record retained. Removed only verified owned test installation, checking registration target first; retained evidence/synthetic files hash-identical.

## Repair and after evidence

Acceptance criteria committed before edits. Only verify-workspace.mjs selectors changed: creation asserts Dashboard plus exact scoped workspace name/identity; closing uses Dashboard > Manage workspace > Close workspace. Settings, cross-adapter persistence, restart, reduced-motion, privacy, narrow-layout checks and timing bounds retained. No app code changed.

Full existing verify-installer.ps1 rerun with a fresh TestRoot passed, uiSkipped=false:
- installPassed=true; installedBinaryMatches=true;
- uiWorkflowPassed=true (native and paired browser onboarding/settings/parity/restart);
- uninstallPassed=true; dataPreserved=true (byte-identical catalog check).

The verifier's PASS lines are retained verbatim in the evidence. Test-owned uninstall registration and remembered-location key were removed. Shared lease released, owned app/API processes stopped. No owner installation/data touched, no model or real credential use, no push/promotion/CI.

## Limits and next checks

This was an isolated test installation on the existing Windows account with clean ANANSI registration, not a clean OS/VM. No SmartScreen, previous-build upgrade, downgrade, real native credential/keyring, native file-picker, native fresh restore or concurrent-ingest claim. The credential prompt requires a user-operated step; the computer-use skill prohibits automating user authentication dialogs. No actual credential-prompt entry was attempted. Those checks remain recorded as open, not fabricated passes.
