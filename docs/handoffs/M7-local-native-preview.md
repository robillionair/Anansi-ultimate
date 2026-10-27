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

## Windows results on local preparation checkpoint `990175c52`

Normal embedded desktop/API build passed with `custom-protocol`, with caches and
temporary files on E:. Desktop SHA-256:
`655490fcdf3d243e8088009eb6c4dc4c27aea6ee1f8bb2b0c311114f1efc6899`.
API SHA-256:
`ad77da3015b11df3fda307191e39b4363aea431e3266ffc35dadd5a4a508e54a`.
UI/contracts 60/60, TypeScript, web build and documentation validation passed.
`git diff cc25155ff -- apps crates packages` is empty: this checkpoint contains
preview/test materials only. Previous Windows backend/browser evidence belongs
to `cc25155ff` and is not described as a new backend-suite run here.

Disposable fixture: `E:\Anansi\tmp\m7-s23-native-Xm5oFI`, acquired under the
exclusive application-test lease. The native window was the executable in the
new `preview-m7-slices2-3` cache, never the existing M7 process.

Observed native actions passed:

- Opened seeded workspace/project/concept; generated and saved specification v1.
- Observed unapproved status and disabled source generation. Provider capture
  contained only the specification request at this point. Explicit native
  approval enabled generation; saved source v1 with execution stages unavailable.
- Requested a revision, rejected it without saving (latest remained v1), then
  requested again and accepted source v2. Specification remained pinned to v1.
- Suggested documentation, edited usage as user-written text, saved deliverable
  v1 and downloaded its export through the native Export button.
- Verified exported TAR bytes without extracting/executing: 11 regular entries,
  checksums, manifest/source hashes, exact package-v2 marker and all five status
  flags false. Export SHA-256:
  `91a0d427bc4759c6869790104efa358954eefb32ad6d20c6473f79b0252bffc9`.
- Revised architecture into deliverable v2. Selecting v1 showed its original
  model architecture in both viewer and editor. Replaced it with an unsaved
  marker, cancelled, reopened: original text returned and marker was absent.
- Restarted the owned desktop; reopened project and deliverable v2 and observed
  `Native S23 architecture v2: inert source only.` persisted correctly.

Synthetic request capture contains specification, source, two revision requests
and documentation only; no real provider, credentials or generated execution.
Fixture API backup/fresh restore passed, preserving package v1/v2, deliverable
v1/v2 and remapped parent/specification relationships. Backup SHA-256:
`d8bd7696b0cee6895060423d913f29500b8ec759255564e520ecff766ba36520`.
Restored export passed the same inert byte verifier, SHA-256:
`e11a9725683cf8f74b380c0aea37617660daa1bdddb529714f793b09889b5f6b`.
The API operation did not establish backup-consent UI acceptance.

The initial session completed restore, then reached its 90-minute lifetime while
awaiting further inspection and closed its own desktop. This is retained as a
harness timeout, not reported as a passing process exit. A separate leased
`m7-s23-native-reopen.mjs` run reopened that same retained fixture and completed
normally after native verification. No regeneration or data repair was needed.

Native reopened workspace `Native S23 Restored`:
`0fdbfed294f0b560f20e3940e4d411bb`, project
`e59d59cc3d75f3c0ecf83acc84f0b7f7`. The native deliverable viewer showed v2,
source `1070664179ce1340034a4dbbdaeea8c2` v2, specification
`9ed4c488aa5bbce8e422d507925c282b` v1, the saved architecture-v2 string and
user-written usage. Thus API recovery and native restored reopening are both
observed, with their scopes distinguished.

The owned test desktop was closed and the lease released. Installed separate
launcher `E:\Anansi\Start ANANSI Preview M7 Slices 2-3.cmd` and Desktop shortcut
`ANANSI Preview M7 Slices 2-3.lnk`. Personal root is
`%LOCALAPPDATA%\ANANSI-Preview\m7-slices2-3`; fixture content was not copied there.
The old M7 process/data and R17 launchers were preserved. No model installed,
real credential used, paid call made, generated source executed or remote action
performed. This closes this checkpoint's bounded synthetic native Forge workflow,
not the independent M5 real-CredUI/live-provider/audit/CI gates or full onboarding.

The computer-use skill's mandatory rule is: "Do not act on security or privacy
permission requests." Accordingly the privacy consent UI was left unautomated;
the installed instruction source is
`C:\Users\robin\.codex\plugins\cache\openai-bundled\computer-use\26.1021.31345\docs\guidance.md`,
referenced by that package's `skills/computer-use/SKILL.md`.
