# v1 installer qualification tooling — Windows review, 2026-11-12

Verdict: automated tooling checks PASS; release build and installer/native qualification HELD by host preflight. No full installer acceptance.

Delivered tip `7c3af5eb5575aca1388e3c924dbfe6fff2a52ffc`, baseline `57029b2eb2acfa47b7d180591b9b73fd3c04ec25`. Incoming ZIP SHA-256 `d4963429f4bc9db0d54f40eff7cf0e675b699de290c5908cad88468604b328bf` matches its sidecar. All six MANIFEST file hashes match; full-history bundle verifies and fetched tip matches. Product-code diff versus runtime `8931d34b` is empty.

## Independently executed on Windows

- Node docs/hygiene/doctor/native-setup/installer/candidate-location/fixture suites: **61/61**, zero failures. The MSVC check that failed in the Linux report passed here.
- Serial existing Windows release-tooling suites: **54/54**, zero failures (96.34 seconds). This is a separate invocation; it is not an installer run.
- Fixture CLI: generated nine files into a fresh E: location, then --verify passed against the generated manifest and canonical fixtures.
- PowerShell parser: verify-installer.ps1 has zero syntax errors. Its installation/uninstallation behavior was NOT executed.
- Preflight initially reported 2.75 GiB free RAM; final rerun after tests reported **3.07 GiB**, below the existing **4 GiB** requirement. Final preflight exit 1. No target application processes, shared lease available; disk checks and discovered toolchains passed. Build intentionally not started while preflight blocks.

## Review scope

Inspected the candidate-location guards, candidate verifier parameterization, isolated WebView2 changes, lease identity propagation, build/packaging identity and CARGO_TARGET_DIR composition. No product source or test assertions changed during review. Existing browser journey and library aggregate retain their previous exact-commit scope, not newly rerun results.

The disclosed launch-acceptance default WebView2-profile limitation remains. No guarded application launch was performed. Manual CredUI/keyring, native dialogs, clean-machine install/upgrade/uninstall and data preservation remain unverified. No installer built, installed, promoted, or published. No owner app was stopped and no owner data modified. No model/credential use or remote CI.

## Next step

Free enough host memory for the unchanged preflight to pass (at least 4 GiB available), rerun it, then build the exact candidate using the explicit isolated E: layout in the handoff. Keep qualification sequential. Installer install/uninstall checks must use a clean machine/VM and retain their existing refusal of an installed owner copy. Record build identity, installer hash, and per-item native results before any pilot approval. No Muse code repair is requested by these results.
