CREATE TABLE backup_schedules (
    workspace_id TEXT PRIMARY KEY REFERENCES workspaces(id) ON DELETE RESTRICT,
    enabled INTEGER NOT NULL CHECK(enabled IN (0,1)),
    interval_seconds INTEGER NOT NULL CHECK(interval_seconds BETWEEN 3600 AND 2592000),
    next_due INTEGER NOT NULL,
    missed_runs INTEGER NOT NULL DEFAULT 0,
    consent_version INTEGER NOT NULL CHECK(consent_version=1)
) STRICT;
CREATE TABLE restore_origins (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE RESTRICT,
    new_id TEXT NOT NULL,
    original_workspace TEXT NOT NULL,
    original_id TEXT NOT NULL,
    backup_id TEXT NOT NULL,
    PRIMARY KEY(workspace_id,new_id)
) STRICT;
CREATE TABLE backup_uploads (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE RESTRICT,
    id TEXT NOT NULL,
    expected_bytes INTEGER NOT NULL CHECK(expected_bytes BETWEEN 1024 AND 268435456),
    expected_hash TEXT NOT NULL CHECK(length(expected_hash)=64),
    received INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(workspace_id,id)
) STRICT;
