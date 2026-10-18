use anansi_core::{Core, ErrorCode, storage::DATABASE_NAME};
use rusqlite::params;
use serde_json::{Value, json};

#[test]
fn corpus_overflow_is_not_partial_and_metadata_identity_is_checked() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());
    let w = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Search bounds fixture"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let first=core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"draft":{"title":"Synthetic match","body":"body"}})).unwrap();
    let mut db = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
    let revision = first["memory"]["revisionId"].as_str().unwrap();
    let original_id = first["memory"]["id"].as_str().unwrap();
    let search = || {
        core.memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":w,"query":"Synthetic"}),
        )
    };
    let mut oversized = first["memory"].clone();
    oversized["sourceReferences"] = json!(["x".repeat(32 * 1024 * 1024)]);
    db.execute(
        "UPDATE memory_revisions SET metadata=?1 WHERE id=?2",
        params![oversized.to_string(), revision],
    )
    .unwrap();
    assert_eq!(search().unwrap_err().code, ErrorCode::InvalidSearch);
    let mut mismatched = first["memory"].clone();
    mismatched["workspaceId"] = json!("f".repeat(32));
    db.execute(
        "UPDATE memory_revisions SET metadata=?1 WHERE id=?2",
        params![mismatched.to_string(), revision],
    )
    .unwrap();
    assert_eq!(search().unwrap_err().code, ErrorCode::ContentIntegrity);
    db.execute(
        "UPDATE memory_revisions SET metadata=?1 WHERE id=?2",
        params![first["memory"].to_string(), revision],
    )
    .unwrap();
    let tx = db.transaction().unwrap();
    for n in 0..10_000 {
        let id = format!("{n:032x}");
        assert_ne!(id, original_id);
        let mut metadata: Value = first["memory"].clone();
        metadata["id"] = json!(id);
        tx.execute(
            "INSERT INTO memories(workspace_id,id,current_revision) VALUES(?1,?2,?3)",
            params![w, id, revision],
        )
        .unwrap();
        tx.execute("INSERT INTO memory_revisions(workspace_id,memory_id,id,version,metadata,body_hash) VALUES(?1,?2,?3,1,?4,?5)",params![w,id,revision,metadata.to_string(),"0".repeat(64)]).unwrap();
    }
    tx.commit().unwrap();
    assert_eq!(search().unwrap_err().code, ErrorCode::InvalidSearch);
    // Filtering before the corpus bound lets users narrow a large workspace safely.
    let shrink = db.transaction().unwrap();
    shrink
        .execute(
            "DELETE FROM memory_revisions WHERE memory_id=?1",
            [format!("{:032x}", 0)],
        )
        .unwrap();
    shrink
        .execute("DELETE FROM memories WHERE id=?1", [format!("{:032x}", 0)])
        .unwrap();
    shrink.commit().unwrap();
    let at_limit = search().unwrap();
    assert_eq!(at_limit["memories"].as_array().unwrap().len(), 50);
    assert_eq!(at_limit["nextOffset"], 50);
    let narrowed = core
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":w,"query":"Synthetic","kind":"note"}),
        )
        .unwrap();
    assert!(narrowed["memories"].as_array().unwrap().is_empty());
}
