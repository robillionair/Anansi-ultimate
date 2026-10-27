# M7 Slices 2–3 local Windows preview — 2026-10-27

## Authority and composition

The owner requested continuing the preview build and verification locally.
The received file is the earlier repair ZIP, not Muse's reported integration ZIP.
Commit `bed0dc2f0baf10a0e6d7d59b6147e82b07d10a4b` is not locally available.
This integration uses the already Windows-verified application baseline
`cc25155ffe758dbab68b40e9576381738055b2ee` and cherry-picks preview materials
from `3335248bcf5210b4a1b75c8caa375cd62a74421f`. Both conflicting build-status
sections are retained. No application source is reapplied or changed.

## Acceptance criteria before the native run

1. Build a normal embedded desktop/API on E:, without CDP or credential-test flags.
2. Use a fresh disposable synthetic test root and an exclusive application-test lease.
3. Through the actual native window: open a project/concept, generate/save a draft,
   observe unapproved state, approve the exact version, generate/save source.
4. Request/reject a revision without saving; request/accept as a new version.
5. Prepare/edit/save/export a deliverable. Inspect archive bytes/manifest/hashes as
   data, never execute generated source.
6. Revise documents, inspect earlier-version text, edit/cancel/reopen and confirm
   cancelled drafts do not persist.
7. Restart/reopen saved artifacts. Exercise backup/fresh restore through trusted
   fixture API operations, then reopen the restored data in the native window.
   Label API fixture operations separately; do not claim UI consent acceptance.
8. Provide a separate personal launcher/root and keep M7/R17 data and launchers.

The fixture seeds workspace/project/concept/provider configuration via the existing
API, not by driving onboarding. Synthetic native interactions establish adapter/UI
integration, not real-provider, production credential or full onboarding evidence.
The plaintext-backup consent UI is not automated as a privacy permission request;
fixture-managed API backup/restore is distinguished explicitly.

No generated-code execution, real credentials, paid inference, model downloads,
pushes, master merges or remote CI. Production gates remain open. Build/observed
workflow results are appended after verification, without inferring unrun checks.
