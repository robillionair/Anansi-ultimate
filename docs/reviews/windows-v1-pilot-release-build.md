# v1 pilot isolated Windows release build — 2026-11-13

Build verdict: PASS. Installer/native qualification remains partial, not customer-release approval.

Exact source `6f216e3224e2c855236aac5492f69b467cf8c5d5`, clean at build. Preflight now PASS: 7.13 GiB RAM free, C: 25.44 GiB and E: 87.12 GiB free, toolchains detected, no target processes, lease free. Previous RAM hold is superseded by this actual run; no thresholds were changed and no owner process was closed.

## Build and identity

Existing build-verify.mjs executed with explicit E:/Anansi/qualification/{build-cache,artifacts,tmp}. Web exit 0 (9.00s), release API exit 0 (330.24s), desktop standalone exit 0 (462.49s), NSIS installer exit 0 (91.06s), total 897.50s. Offline verification PASS: seven artifacts, all recorded hashes and sizes match. installer-checksums.mjs PASS and both manifests agree on source, clean state, installer and NSIS payload hashes. Signature status NotSigned, expected unsigned development build.

- Installer: `E:/Anansi/qualification/artifacts/ANANSI Ultimate_0.1.0_x64-setup.exe`
- Installer SHA-256: `2b0c1c7980b7cf8c62b1d9b621723276ef943208d988ea579a62494da5408d74`
- Standalone app SHA-256: `47e1695d95325f574970331f89d514bb797669dc00021ded50646868901b2a7c`
- Installed/NSIS payload app SHA-256: `2f6bbd2b0572fca2a8e9d3d9fc616d6667a962b193c72eb7ee5ecd3ff6816580`
- Config SHA-256: `8868aded25eb36066561ceb3a3404b01219dda839b14985a571d4a794dd77c4d`

Keep packaging.json beside the installer. Version 0.1.0 alone is insufficient identification. No installed owner copy was updated.

## Automated native smoke and limitation

Existing native-candidate-acceptance.mjs ran unchanged under the shared test lease against the exact standalone app hash, fresh synthetic E:/Anansi/qualification/native-2026-11-13/data and isolated WebView2 UDF. Startup, shell capability denial, workspace creation, memory/search, project/artifact registration invocation, completed backup and memory persistence after process restart ran successfully. Script exited 0 and printed all seven checks passed. Screenshot visually inspected: actual native app renders onboarding, not a completed interactive onboarding workflow. Most checks invoke native IPC, so this is not manual UI acceptance.

Qualification finding NQ-01: the artifact log reads artRes.artifact.id and printed undefined, with no assertion/readback; actual ArtifactRecord uses artifactId. Independent post-run read-only SQLite inspection confirmed exactly one Synthesized Idea Artifact row at version 1 with artifact_id 413c75aa59a09ef9637d3df347049b78. The app saved it; the script still needs a real identifier assertion and native readback to substantiate that numbered step. Do not use its blanket 7/7 banner as full native acceptance. No source or test changes made in this run.

## Outstanding

Clean-machine installer execution, SmartScreen observation, install/upgrade/uninstall/storage preservation, real CredUI/keyring and file-dialog interaction, native fresh restore, and concurrent ingestion remain separate checklist items. The synthetic vault provider in the smoke is not production credential evidence. OAuth stays deferred. No model calls, credentials, generated execution, owner data changes, promotion, push, publication or remote CI. Shared lease released and owned candidate process exited.

Next: fix NQ-01 in the qualification harness with a failing regression; continue qualification on an isolated/clean Windows environment without replacing owner installation. No application rebuild is needed solely for a harness/docs repair if app/source identity is explicitly preserved.
