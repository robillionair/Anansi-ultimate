CREATE TABLE artifact_versions (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE RESTRICT,
    artifact_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    version INTEGER NOT NULL CHECK(version > 0),
    artifact_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    source_references TEXT NOT NULL,
    generator TEXT,
    validation_status TEXT NOT NULL,
    privacy TEXT NOT NULL CHECK(privacy IN ('standard','private','sealed')),
    created_at TEXT NOT NULL,
    PRIMARY KEY(workspace_id, artifact_id, version)
);
CREATE INDEX artifact_versions_project ON artifact_versions(workspace_id, project_id, artifact_id);
