# Recovery and backup

Follow [RECOVERY.md](RECOVERY.md) to resume work and [the M0 plan](../plans/PHASE-000-foundation.md) to implement and verify source backup/restore. This entrypoint preserves the specification section 4 path.

Current source is backed up by authorized Git pushes. A local bundle inside .git is a convenience, not an independent backup. M0 must create an independent local bundle, verify it and restore into a separate temporary directory. Workspace, application, generated artifact and credential-aware backups remain separate product requirements under specification section 76 and M2/M15; they are not implemented.
