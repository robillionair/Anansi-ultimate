# ANANSI Ultimate — Recovery Guide

1. Read `AGENTS.md`, `docs/MASTER_SPEC.md`, `docs/BUILD_STATUS.md`, the active plan, and `docs/DECISIONS.md`.
2. Check `git status` and inspect the last verified commit recorded in build status.
3. Do not discard uncommitted work. Identify the active phase and its acceptance criteria.
4. Re-run the phase's verification commands before changing code.
5. Continue only the unfinished acceptance criteria, then record evidence and update status.
6. If requirements conflict, stop implementation and record the conflict in `docs/DECISIONS.md`.

Recovery checkpoints should contain source, tests, and the corresponding status/documentation updates. Never treat a build artifact alone as a recoverable project state.

## Reconciled resume entrypoint

Begin with [the M2 detailed plan](../plans/02-memory-system.md), the active/next task in project-state.json, and [the canonical roadmap](ROADMAP.md). M0 and M1.0–M1.5 are complete. The user accepted the M2 plan; M2.1 local memory acceptance passes and M2.2 remains unstarted. Verify the latest checkpoint/publication before proceeding. Preserve the M1 completion tag at 97babb3. Check BUILD_STATUS.md for actual acceptance/publication. Read the detailed acceptance plan before coding. Milestone IDs follow master section 82; legacy numeric plan filenames do not define milestone numbers. Run `node --test tests/repository-hygiene.test.mjs` and `node scripts/validate-planning.mjs --self-test` for current foundation checks; these do not run product tests or replace source recovery checks.

## Workspace data recovery after M1.3

M2.1 uses schema 3. Recognized schema-1/2 catalogs migrate forward without removing original workspace fields; older binaries cannot open schema 3. Preserve a complete closed-root copy before manual application rollback. Do not downgrade user_version or copy only selected rows/files. Onboarding-complete is only a UI preference and grants no cloud-sharing/recording consent.

Source bundles exclude application data. Close both desktop and API before preserving the entire configured storage root, including .anansi-root, workspaces.sqlite3, all content/ Markdown revisions and any WAL/SHM/journal files. Keep the original unchanged outside Git and use only a verified compatible copy for manual recovery. Never copy just the main database while a client is running, delete a failed initialization, lower a schema version or reset an unknown catalog automatically. Read-only, corrupt and newer-schema failures leave a recoverable error; interrupted first initialization may require inspection and manual recovery. Automatic user-data backup/restore remains future work. See [M1.3 evidence](evidence/M1-3.md).

If a documentation checkpoint records the previous verified implementation commit, inspect subsequent metadata commits as well. Never reset to a recorded SHA just because it is older than HEAD. Fresh recovery uses `git clone https://github.com/robillionair/Anansi-ultimate.git`; bundle recovery uses `git bundle verify <bundle>` followed by `git clone <bundle> <new-directory>`. M0 independent backup/restore has been proved and recorded in docs/evidence/M0.md; follow RECOVERY_AND_BACKUP.md to restore or create subsequent source backups.

## M1.5 installer recovery

For M2.1 integrity errors, preserve the complete root unchanged and use a compatible copy for diagnosis. Unreferenced Markdown may be a failed/conflicted save; never infer that it should be imported or deleted. Do not repair a missing body by creating an empty file or editing revision pointers. Source bundles exclude all application data. Restore/backup UI remains M2.5/M2.7, not a current product recovery feature.

Installer verification uses a fresh external temporary target and refuses existing ANANSI registration/remembered locations. A failed verification retains its exact target and installer-verification.json. Inspect that record and compare the registered InstallLocation and installed payload hash before running only the generated test uninstaller. Never remove a real application, edit its registration or delete data merely to unblock the test. Successful verification removes only its originally absent, test-owned remembered-location product key after exact location readback. Private workspace fixtures remain outside source.

Development installer artifacts are rebuildable and are excluded from source bundles. CI release artifacts retain installer SHA256 and payload/build manifests for 14 days; they are unsigned development builds. No API service, model downloads, updater or signed-release guarantee is bundled. M2 requires its separate detailed plan and approval; do not start it at the M1 completion boundary.

## M1 completion boundary

M1 development desktop/browser foundation is verified complete; see [M1.5 evidence](evidence/M1-5.md). The completion tag is m1-desktop-foundation-verified at 97babb3, applied after protected publication. Source/tag readback and independent restore passed. Subsequent user continuation accepted M2.0 and authorized M2.1. Its schema-3 memory implementation is separate from M1; see evidence/M2-1.md. Preserve the M1 tag and distinguish source bundles from private-data backups.
