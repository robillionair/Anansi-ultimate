use anansi_core::{Core, ErrorCode, storage::DATABASE_NAME};
use rusqlite::{Connection, params};
use serde_json::json;

#[test]
fn invalidation_failure_rolls_back_canonical_revision_and_fts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let workspace = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Invalidation rollback"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let created = core.memory_command("create", json!({"contractVersion":1,"workspaceId":workspace,"kind":"note","draft":{"title":"Original title","body":"Original searchable content","privacy":"private"}})).unwrap();
    let id = created["memory"]["id"].as_str().unwrap();
    let revision = created["memory"]["revisionId"].as_str().unwrap();
    let conn = Connection::open(root.join(DATABASE_NAME)).unwrap();
    conn.execute("INSERT INTO memory_embeddings(workspace_id,memory_id,chunk_index,model,dimension,embedding,created_at) VALUES(?1,?2,0,'synthetic',2,?3,'2026-10-20T00:00:00Z')", params![workspace,id,vec![0u8;8]]).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_embedding_delete BEFORE DELETE ON memory_embeddings BEGIN SELECT RAISE(ABORT,'synthetic invalidation failure'); END;").unwrap();
    let error = core.memory_command("revise", json!({"contractVersion":1,"workspaceId":workspace,"id":id,"expectedRevision":revision,"draft":{"title":"Changed title","body":"Changed searchable content","privacy":"private"}})).unwrap_err();
    assert_eq!(error.code, ErrorCode::StorageUnavailable);
    let current: String = conn
        .query_row(
            "SELECT current_revision FROM memories WHERE workspace_id=?1 AND id=?2",
            params![workspace, id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(current, revision);
    let fts: String = conn
        .query_row(
            "SELECT title FROM memory_body_fts WHERE workspace_id=?1 AND memory_id=?2",
            params![workspace, id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(fts, "Original title");
    conn.execute_batch("DROP TRIGGER reject_embedding_delete;")
        .unwrap();
    core.memory_command("revise", json!({"contractVersion":1,"workspaceId":workspace,"id":id,"expectedRevision":revision,"draft":{"title":"Changed title","body":"Changed searchable content","privacy":"private"}})).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM memory_embeddings WHERE workspace_id=?1 AND memory_id=?2",
            params![workspace, id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}
