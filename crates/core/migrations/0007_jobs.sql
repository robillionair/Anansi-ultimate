CREATE TABLE jobs (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE RESTRICT,
    id TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('Queued','Running','Paused','Completed','Failed','Cancelled')),
    version INTEGER NOT NULL CHECK(version > 0),
    checkpoint_version INTEGER NOT NULL CHECK(checkpoint_version > 0),
    checkpoint TEXT NOT NULL,
    selection TEXT NOT NULL,
    owner TEXT,
    lease_until INTEGER,
    progress INTEGER NOT NULL DEFAULT 0 CHECK(progress >= 0 AND progress <= 10000),
    artifact_id TEXT,
    artifact_bytes INTEGER,
    artifact_hash TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    log TEXT NOT NULL,
    PRIMARY KEY(workspace_id,id),
    CHECK((state = 'Running') = (owner IS NOT NULL AND lease_until IS NOT NULL)),
    CHECK((state = 'Completed') = (artifact_id IS NOT NULL AND artifact_bytes IS NOT NULL AND artifact_hash IS NOT NULL))
) STRICT;
CREATE INDEX jobs_pending ON jobs(state,created_at,id);
