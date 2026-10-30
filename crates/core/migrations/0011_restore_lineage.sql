-- Restore lineage: one origins row per generation that named
-- an entity. The old PRIMARY KEY(workspace_id, new_id) allowed
-- only one original per current id, which discarded
-- intermediate generations still named by later-authored
-- snapshots. Rebuild with the original id in the key,
-- preserving every existing row verbatim.
ALTER TABLE restore_origins RENAME TO restore_origins_legacy;
CREATE TABLE restore_origins (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE RESTRICT,
    new_id TEXT NOT NULL,
    original_workspace TEXT NOT NULL,
    original_id TEXT NOT NULL,
    backup_id TEXT NOT NULL,
    PRIMARY KEY(workspace_id,new_id,original_id)
) STRICT;
INSERT INTO restore_origins SELECT * FROM restore_origins_legacy;
DROP TABLE restore_origins_legacy;
