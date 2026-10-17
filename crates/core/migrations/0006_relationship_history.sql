ALTER TABLE relationships ADD COLUMN archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1));
ALTER TABLE relationships ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
CREATE TABLE relationship_revisions (
    workspace_id TEXT NOT NULL,
    relationship_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    snapshot TEXT NOT NULL,
    PRIMARY KEY(workspace_id, relationship_id, version),
    FOREIGN KEY(workspace_id, relationship_id) REFERENCES relationships(workspace_id,id) ON DELETE RESTRICT
) STRICT;
-- Preserve surviving schema-5 edges; authorship and deleted history cannot be reconstructed.
INSERT INTO relationship_revisions
SELECT workspace_id,id,1,json_object('id',id,'workspaceId',workspace_id,
    'sourceId',source_id,'targetId',target_id,'predicate',predicate,'createdAt',created_at,
    'updatedAt',created_at,'metadata',json(metadata),'version',1,'archived',json('false'),
    'provenance','legacy_schema5') FROM relationships;
