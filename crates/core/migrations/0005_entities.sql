CREATE TABLE IF NOT EXISTS relationships (
    workspace_id TEXT NOT NULL,
    id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    predicate TEXT NOT NULL CHECK(length(predicate) >= 1 AND length(predicate) <= 64),
    metadata TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY(workspace_id, id),
    FOREIGN KEY(workspace_id, source_id) REFERENCES memories(workspace_id, id) ON DELETE RESTRICT,
    FOREIGN KEY(workspace_id, target_id) REFERENCES memories(workspace_id, id) ON DELETE RESTRICT,
    CHECK(source_id != target_id)
) STRICT;
CREATE INDEX IF NOT EXISTS idx_relationships_source ON relationships(workspace_id, source_id);
CREATE INDEX IF NOT EXISTS idx_relationships_target ON relationships(workspace_id, target_id);
