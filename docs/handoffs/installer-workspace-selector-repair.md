# Installed workflow selector repair — acceptance criteria, 2026-11-13

Reproduced on built source 6f216e3 with harness b0249927: installer installed successfully and installed payload hash matched, but verify-workspace.mjs create() expected a heading named Synthetic native workspace after onboarding. Actual UI is Dashboard, with the name inside Workspace overview. No app defect established. Failed run log and installer-verification.json retained; only the owned test installation was uninstalled and synthetic evidence hash-preserved.

Scope: update stale cross-adapter harness selectors only, no app/binary changes.

Acceptance before implementation:
1. After workspace creation assert Dashboard heading, scoped Workspace overview exact workspace name, and return the workspace id from that overview.
2. Close through current Dashboard > Manage workspace > Close workspace controls; keep create/reopen/settings assertions and timeouts intact.
3. Run the existing complete installer verification, including installed native/browser onboarding, settings parity, restart, and uninstall/data-preservation, against the same exact built installer in a fresh test root. Record any new failure without hiding it.
4. Preserve first failure evidence and distinguish same-host clean registration from a clean VM/OS qualification. No SmartScreen/native credential/picker claims from this script.
5. No application rebuild, owner-data change, inference, credentials, publication or promotion.

## Verification outcome

Before repair: installed native/browser workflow failed on stale workspace-name heading after successful install and payload hash match (original log retained). Owned test installation removed; retained test evidence and synthetic files were hash-identical.

After repair: complete verify-installer.ps1 without SkipUi passed on fresh installer-test-fixed-2026-11-13 root, same installer/app hashes. Native/browser onboarding, privacy review, skip AI, intent/motion settings parity, process restarts, narrow layout and live reduced-motion CSS passed. Installed app and uninstaller/registration removed; catalog hash preservation passed. Installer verification record has installPassed/installedBinaryMatches/uiWorkflowPassed/uninstallPassed/dataPreserved all true, uiSkipped false.

Scope: same Windows host, no pre-existing registered app, isolated E: install and synthetic storage. Not a pristine Windows VM, SmartScreen observation, upgrade/downgrade, credential-store, file-picker, or complete data-format round trip. No application rebuild. Raw logs and record accompany review delivery.
