CREATE TABLE memories (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE RESTRICT,
    id TEXT NOT NULL,
    current_revision TEXT NOT NULL,
    PRIMARY KEY(workspace_id,id),
    FOREIGN KEY(workspace_id,id,current_revision) REFERENCES memory_revisions(workspace_id,memory_id,id) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE memory_revisions (
    workspace_id TEXT NOT NULL,
    memory_id TEXT NOT NULL,
    id TEXT NOT NULL,
    version INTEGER NOT NULL CHECK(version > 0),
    metadata TEXT NOT NULL,
    body_hash TEXT NOT NULL CHECK(length(body_hash)=64),
    PRIMARY KEY(workspace_id,memory_id,id),
    UNIQUE(workspace_id,memory_id,version),
    FOREIGN KEY(workspace_id,memory_id) REFERENCES memories(workspace_id,id) ON DELETE RESTRICT
) STRICT;
