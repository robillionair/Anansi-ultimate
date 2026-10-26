# Windows M7 repaired preview acceptance — 2026-10-26

Application checkpoint: `518418b6a955feaadcc6212eab0a3f3fd1e1bcb9`.
Local preview branch: `codex/local-preview-m7`. Application source is unchanged.

## Build and focused checks

The normal embedded Windows desktop and API build passed with the locked dependencies,
using `anansi-desktop/custom-protocol`. Build output is under
`E:\Anansi\build-cache\preview-m7`; temporary build files are on E:.
This is a development preview, not a signed release or a credential-test build.
Desktop executable SHA-256:
`3b80a70c2ee369df3e4e5f8ad0bf1af88ad3eaade3dba5943ea367ab4be15b8e`.

Independent Windows checks on the application checkpoint passed: Forge units 8/8,
`forge_generation` 5/5, `forge_restore` 3/3, UI/contracts 55/55, TypeScript,
strict core all-target Clippy, and documentation validation. These are focused
results; this report does not claim a new full-workspace run.

## Actual native desktop workflow

The application-test lease was acquired for each session. The previous stopped
October 20 lease was preserved on E: after the owner explicitly authorized
reconciliation. Both new test sessions completed and released their own leases.

A disposable workspace/project/idea set and trusted loopback provider configuration
were seeded by the local fixture harness. This setup is not UI onboarding or
provider-configuration acceptance. All responses were fixed synthetic Ollama
responses; no model, real credential or paid provider was used.

Through the normal Tauri window, the reviewer:

1. Opened the project and saved idea set, filled the Forge brief, generated a
   specification, and saved it as a draft.
2. Opened that saved version and observed not-approved status with source
   generation disabled. Explicitly approved the version, then observed generation
   become available. Only the specification request existed before this step.
3. Generated and saved the source package. Inspected literal source text;
   Build, Tests, Preview and Deployment remained visibly Unavailable.
4. Exported the source package through the native UI. The resulting 6,144-byte
   TAR was read as data only: its manifest, exact four entries and per-file hashes
   passed the inert archive verifier. No generated HTML or JavaScript was opened
   or executed.
5. Stopped the desktop and synthetic provider, restarted the same normal desktop
   executable against the same disposable storage, and reopened the saved package.
   The saved specification, approval and package were listed; the development
   plan and literal source were readable with the provider stopped.

Export SHA-256:
`97c063174ff195bf822b99631833ee15de6523df19c8b2bd546c44976de7db5d`.
The UI download remains in Downloads as
`13ce8e70d6a70c4430d32fc400a1e9f9-v1.tar`; an evidence copy and verification JSON
are in `E:\Anansi\tmp\m7-ui-acceptance-S14T39`.
The local diagnostic scripts in `scripts/m7-ui-*-session.mjs` and
`scripts/verify-m7-native-export.mjs` document this specific synthetic fixture;
they are not general CI tests. Session scripts must run through the test-lease wrapper.

## Scope and remaining work

Native UI backup/restore and revised-spec reapproval were not repeated in this
desktop session; the focused Windows generation/restore suites cover those backend
behaviors. Real-provider inference, production CredUI/keyring acceptance, external
cryptographic audit and remote CI remain open. Generated-code execution remains
disabled and is not approved by this checkpoint.

Minor UI debt observed: the pipeline says the development plan was generated
and saved before the user explicitly saves the package. This premature status
copy is recorded for a later UI correction; it does not alter the explicit
save operation or the approval boundary.

The personal preview launcher uses `%LOCALAPPDATA%\ANANSI-Preview\m7`, separately
from the synthetic fixture and existing R17/R16 data. It installs no models and
migrates no existing data. R17 remains available as the fallback.
