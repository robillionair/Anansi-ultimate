# Installed workflow selector repair — acceptance criteria, 2026-11-13

Reproduced on built source 6f216e3 with harness b0249927: installer installed successfully and installed payload hash matched, but verify-workspace.mjs create() expected a heading named Synthetic native workspace after onboarding. Actual UI is Dashboard, with the name inside Workspace overview. No app defect established. Failed run log and installer-verification.json retained; only the owned test installation was uninstalled and synthetic evidence hash-preserved.

Scope: update stale cross-adapter harness selectors only, no app/binary changes.

Acceptance before implementation:
1. After workspace creation assert Dashboard heading, scoped Workspace overview exact workspace name, and return the workspace id from that overview.
2. Close through current Dashboard > Manage workspace > Close workspace controls; keep create/reopen/settings assertions and timeouts intact.
3. Run the existing complete installer verification, including installed native/browser onboarding, settings parity, restart, and uninstall/data-preservation, against the same exact built installer in a fresh test root. Record any new failure without hiding it.
4. Preserve first failure evidence and distinguish same-host clean registration from a clean VM/OS qualification. No SmartScreen/native credential/picker claims from this script.
5. No application rebuild, owner-data change, inference, credentials, publication or promotion.
