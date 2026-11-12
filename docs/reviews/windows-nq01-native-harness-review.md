# NQ-01 native harness repair — Windows review, 2026-11-13

Verdict: PASS in focused Windows native-harness scope. NQ-01 closed. No installer/manual-native approval.

Harness tip `b02499277cf16224a227e96a6733e362385a7a12`, based on build reviewer `766216263a0c5860b35441ca97334a4cf0799e42`. Incoming ZIP SHA-256 `4f4b36ac41a140214c24a6c5c3d68ed7ca3a7287d3d23faf8c7b8dce72e11ce3` verified. All 19 MANIFEST file hashes match; full-history bundle verifies and fetched tip matches. App-code diff versus built source `6f216e3224e2c855236aac5492f69b467cf8c5d5` is empty. Existing standalone app hash rechecked: `47e1695d95325f574970331f89d514bb797669dc00021ded50646868901b2a7c`. No rebuild.

## Independent Windows execution

- Native artifact acceptance regressions 9/9 plus docs/hygiene 27/27: combined invocation **36/36**, no failures.
- Documentation validator PASS, 396 Markdown files before this review.
- Strengthened native-candidate-acceptance.mjs, unchanged, under shared lease with fresh synthetic data and WebView2 profile on E:: **all seven numbered checks passed**, exit 0.
- get_readiness invoked and asserted: ANANSI Ultimate 0.1.0, core 0.1.0, windows.
- Artifact registration produced real artifactId `33338263bab6a677a9365febcd0492cb`; native IPC readback matched project/type/version/title/body; same id unreadable from a second fresh workspace.
- Backup completed, candidate process restarted, memory survived, and exact artifact identity/content survived the new post-restart native readback.
- Shell execution denied by capability ACL. Screenshot captured. Owned app processes exited and shared lease released.

The helper assertions match ArtifactDetails/ArtifactRecord rather than nonexistent fields. Pure regression coverage refuses missing or mismatched identity/content. Reviewer changed no runtime, harness, test, assertion or timeout.

## Scope and next action

The automated native IPC/capability/persistence smoke is accepted; its synthetic vault provider does not evidence real CredUI/keyring behavior. The screenshot is startup/onboarding, not a completed manual interactive journey. Clean-machine install/upgrade/uninstall with storage preservation, SmartScreen observation, manual native credentials/file dialogs, native fresh restore and concurrent-ingest observation remain outstanding. Existing unsigned installer and packaging identity remain those of source 6f216e3, not this harness/review commit. No installer run, owner data changes, model calls, credentials, generated execution, promotion, push or remote CI. No further NQ-01 repair requested; proceed to installer/manual-native qualification.
