# ANANSI Ultimate — Recovery Guide

1. Read `AGENTS.md`, `docs/MASTER_SPEC.md`, `docs/BUILD_STATUS.md`, the active plan, and `docs/DECISIONS.md`.
2. Check `git status` and inspect the last verified commit recorded in build status.
3. Do not discard uncommitted work. Identify the active phase and its acceptance criteria.
4. Re-run the phase's verification commands before changing code.
5. Continue only the unfinished acceptance criteria, then record evidence and update status.
6. If requirements conflict, stop implementation and record the conflict in `docs/DECISIONS.md`.

Recovery checkpoints should contain source, tests, and the corresponding status/documentation updates. Never treat a build artifact alone as a recoverable project state.

## Reconciled resume entrypoint

Begin with [the M2 detailed plan](../plans/02-memory-system.md), the active/next task in project-state.json, and [the canonical roadmap](ROADMAP.md). M0 and M1.0–M1.5 are complete. The user accepted the M2 plan. M2.1, M2.2 and the M2.3 acceptance repair are verified locally; the last verified implementation is 5b83338. M2.4 persistent jobs/open-format export is next and remains unstarted. Verify the latest checkpoint/publication before proceeding. Preserve the M1 completion tag at 97babb3. Check BUILD_STATUS.md for actual acceptance/publication. Read the detailed acceptance plan before coding. Milestone IDs follow master section 82; legacy numeric plan filenames do not define milestone numbers. Run `node --test tests/repository-hygiene.test.mjs` and `node scripts/validate-planning.mjs --self-test` for current foundation checks; these do not run product tests or replace source recovery checks.

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

## M2.2 schema and history recovery

M2.2 moves recognized schema-1/2/3 roots forward to schema 4 without rewriting original revision JSON or Markdown. Preserve the entire closed root before migration; schema-3 binaries must refuse it. Never lower user_version to run an older binary. Failed migration retains its previous schema and records. Source bundles do not contain private application data.

Archive and expiry retain bytes and history. Include archived/expired records explicitly to reopen them. Restore archive returns to the saved prior state; an expired record remains filtered until explicitly requested. Compare history and restore selected content as a new revision to retain a recoverable audit trail. This preserves current privacy and lifecycle; it does not restore a database snapshot, erase originals, or implement M2.5 backup recovery. Stale writes must reopen the latest revision rather than overwriting it. See [M2.2 evidence](evidence/M2-2.md).

## M2.3 entities and relationships recovery

M2.3 advances recognized schema-1/2/3/4 roots forward to schema 5, creating the `relationships` table with composite foreign keys `(workspace_id, source_id)` and `(workspace_id, target_id)` referencing `memories(workspace_id, id)`. Preserve the entire closed root before forward migration; older binaries (< schema 5) refuse to open schema 5 databases. Never manually lower `user_version` to run an older binary. Failed forward migrations rollback atomically within a transaction and retain the prior schema version and records. Source bundles exclude private application data.

All nine user knowledge kinds (`memory`, `note`, `project`, `task`, `person`, `organization`, `decision`, `event`, `document`) and type-specific attributes (`dueDate`, `completed`, `projectId`, `eventDate`) are stored and validated at the boundary. Archiving or deleting a record does not cascade-delete relationships automatically; relationships pointing to archived records remain intact in the relationship graph for auditing and history. Cross-workspace relationships and self-loops are rejected by SQLite composite foreign keys and core validation. To recover or repair relational data, inspect relationships using `listRelationships` and verify both endpoints exist in the workspace. Stale or conflicting writes must re-read existing relationships rather than force-overwriting. See [M2.3 evidence](evidence/M2-3.md).


### M2.3 acceptance repair recovery

Schema 6 migrates recognized earlier catalogs forward without changing applied migrations. Preserve the entire closed storage root before upgrading. Do not downgrade user_version or run schema-5 binaries against the upgraded root. A migration error rolls back schema/version together. Surviving schema-5 links retain their IDs/metadata with legacy_schema5 provenance; previously deleted links and unrecorded past actions cannot be reconstructed. Link removal now archives with a required expectedVersion; inspect scoped relationship history and restore the latest archived version. Record revision snapshots and independent relationship action history have different scopes; neither should be rewritten to match the current graph. See ADR-0019 and [M2.3 repair evidence](evidence/M2-3.md).


The M2.3 completion source checkpoint is independently bundled/restored after protected publication. Its machine ledger is `C:/Users/robin/Documents/ChatGPT/Anansi-source-recovery/M2-3-verified-b4a8e110d61042fbb11a3faf434bf9b4/source-verification.json`; read its actual HEAD/tree/digest/results rather than infer a self-referential commit from this document. Actual memory/entity verification JSON and screenshots are retained in the same external directory. This source ledger is separate from private-data backup/recovery.

## M2.4 recovery contract

The active branch is `codex/m2-4-jobs-export`; local acceptance passed at 8f7ff87 in [M2.4 evidence](evidence/M2-4.md). Schema 7 migrates forward from recognized schemas 1–6 without changing existing migrations. Preserve the entire closed private root before rollback; schema-6 binaries must refuse schema 7 rather than modifying its version. SQLite remains canonical for job state, histories and ownership. Interrupted Running jobs become Paused after lease expiry (120 seconds); explicitly resume to rebuild the snapshot stage. Cancel/retry never publishes old partial attempts. Keep unreferenced private export files for diagnosis; no automatic deletion or import is authorized. A portable knowledge export is not a full application backup or a product restore package. No M2 milestone tag is created. M2.5 remains unstarted until this task is safely published/recoverable and the user continues.

M2.4 source recovery package: `C:/Users/robin/Documents/ChatGPT/Anansi-source-recovery/M2-4-checkpoint-mvd70n47`. The final `source-verification.json` records actual published HEAD/tree, source.bundle SHA256, fresh-clone checks, CI URLs/status and real screenshot hashes. The implementation-only source checkpoint precedes the acceptance metadata to avoid self-reference. Use the final ledger and exact GitHub checks when reconciling pending-at-authoring CI statements; do not assume a source bundle contains application data.
