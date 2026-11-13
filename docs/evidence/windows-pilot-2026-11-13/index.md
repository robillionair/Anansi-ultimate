# Windows pilot evidence — 13 November 2026

These are synthetic-data verification records, not production certification. Logs retain failures as well as the corrected rerun. Application source is `6f216e3224e2c855236aac5492f69b467cf8c5d5`; later changes are qualification harnesses and documents. No binary, database, user workspace, pairing file, credential or model weights are included here.

| Evidence | Scope |
| --- | --- |
| [Build log](release-build.md) | Clean isolated web/API/desktop/NSIS build, all exit 0, total 897.50 seconds. |
| [Artifact verification](release-verify.md) | Seven recorded artifacts match hashes and lengths. |
| [Packaging identity](packaging.json) | Exact source, toolchain versions, unsigned status, installer/application/config hashes; installer itself is not published here. |
| [Native smoke](native-smoke.md) | Strengthened readiness and artifact checks, workspace refusal and restart persistence against the exact built app. Synthetic vault provider; not CredUI acceptance. |
| [Installed workflow before](installed-workflow-before.md) | Retained stale heading-selector failure after installation/payload identity had passed. |
| [Installed workflow after](installed-workflow-after.md) | Same unchanged installer; selector-only repair enables full installed native/browser workflow and uninstall/catalog preservation. |
| [Installer record](installer-verification.json) | All recorded checks true, uiSkipped=false. Same-host isolated test with no prior registered ANANSI installation, not a clean VM or upgrade. |
| [Native picker accessibility](native-picker-accessibility.md) | Actual desktop interface after picker operations; shows four successful filenames and unsupported-format refusal. |
| [Post-close picker integrity](native-picker-integrity.md) | Four document revisions, four Completed jobs, expected names and all body hashes verified read-only after app exit. |

Logs are wrapped as fenced Markdown transcripts, normalized to LF, and trailing line whitespace removed. User-profile paths, where present, are replaced with `%USERPROFILE%`; these publication copies need not have the same file hash as the original local log. File hashes below identify the exact published copies. Test workspace/record ids are synthetic. UI snapshots in the retained failure log describe synthetic data only.

[Build review](../../reviews/windows-v1-pilot-release-build.md) · [Native harness review](../../reviews/windows-nq01-native-harness-review.md) · [Installer review](../../reviews/windows-v1-installer-workflow-qualification.md) · [Picker review](../../reviews/windows-native-document-picker.md) · [Current pilot gates](../../pilot/release-decision-sheet.md)
