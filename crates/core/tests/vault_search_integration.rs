//! M3-side privacy regression coverage. These tests exercise the current
//! schema-8 search implementation only; they do not claim live vault
//! session/epoch integration from M2.
use anansi_core::Core;
use rusqlite::Connection;
use serde_json::json;

#[test]
fn rebuild_removes_plaintext_fts_rows_after_sealing_transition() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());
    let workspace = core
        .workspace_command("create", json!({"contractVersion":1,"name":"Vault boundary"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let created = core
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":workspace,"kind":"note","draft":{"title":"Transition","body":"vault-only-canary","privacy":"private"}}),
        )
        .unwrap();
    let memory_id = created["memory"]["id"].as_str().unwrap();
    let db = root.join("workspaces.sqlite3");
    let connection = Connection::open(db).unwrap();
    connection
        .execute(
            "UPDATE memory_revisions SET metadata=json_set(metadata,'$.privacy','sealed') WHERE memory_id=?1",
            [memory_id],
        )
        .unwrap();

    // The old plaintext index may still contain the canary until rebuild.
    core.rebuild_search_index(&workspace).unwrap();
    let result = core
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":workspace,"query":"vault-only-canary","fullBody":true}),
        )
        .unwrap();
    assert!(result["memories"].as_array().unwrap().is_empty());

    // Restart/rebuild must preserve the same privacy result.
    let restarted = Core::with_storage_root(root);
    restarted.rebuild_search_index(&workspace).unwrap();
    let result = restarted
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":workspace,"query":"vault-only-canary","fullBody":true}),
        )
        .unwrap();
    assert!(result["memories"].as_array().unwrap().is_empty());
}
