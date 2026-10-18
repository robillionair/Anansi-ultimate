use anansi_core::{Core, ErrorCode, storage::DATABASE_NAME};
use serde_json::{Value, json};

fn list(core: &Core, w: &str, query: &str, extra: Value) -> Value {
    let mut r = json!({"contractVersion":1,"workspaceId":w,"query":query});
    r.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    core.memory_command("list", r).unwrap()
}
#[test]
fn titles_isolate_filter_rank_page_refresh_and_preserve_revision_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());
    let w = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Search fixture"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let other = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Other fixture"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let create = |workspace: &str, title: &str, kind: &str, expired: bool| {
        core.memory_command("create",json!({"contractVersion":1,"workspaceId":workspace,"kind":kind,"draft":{"title":title,"body":"bodyonlytoken <script>no()</script>","sourceReferences":["Synthetic reference"],"validUntil":if expired {Some("2000-01-01T00:00:00Z")} else {None}}})).unwrap()
    };
    let first = create(&w, "Café 世界", "memory", false);
    for kind in ["project", "task", "person", "organization", "event"] {
        let record = create(&w, &format!("Unique {kind}"), kind, false);
        assert_eq!(
            list(&core, &w, &format!("Unique {kind}"), json!({"kind":kind}))["memories"][0]["id"],
            record["memory"]["id"]
        );
    }
    create(&w, "Café notes", "note", false);
    create(&other, "Café hidden", "memory", false);
    create(&w, "Café expired", "document", true);
    let archived = create(&w, "Café archived", "decision", false);
    core.memory_command("revise",json!({"contractVersion":1,"workspaceId":w,"id":archived["memory"]["id"],"expectedRevision":archived["memory"]["revisionId"],"draft":{"title":"Café archived","body":"archived"},"lifecycle":{"state":"archived","confirmed":true}})).unwrap();
    let matches = list(&core, &w, "cafe", json!({}));
    assert_eq!(matches["memories"].as_array().unwrap().len(), 2);
    assert_eq!(
        list(&core, &w, "世界", json!({}))["memories"][0]["id"],
        first["memory"]["id"]
    );
    assert_eq!(
        list(&core, &w, "cafe", json!({"kind":"note"}))["memories"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        list(
            &core,
            &w,
            "cafe",
            json!({"includeArchived":true,"includeExpired":true})
        )["memories"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert!(
        list(&core, &w, "bodyonlytoken", json!({}))["memories"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        list(&core, &w, "cafe OR absent", json!({}))["memories"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let page = list(&core, &w, "cafe", json!({"limit":1}));
    assert_eq!(page["nextOffset"], 1);
    let page2 = list(&core, &w, "cafe", json!({"limit":1,"offset":1}));
    assert_ne!(page["memories"][0]["id"], page2["memories"][0]["id"]);
    assert_eq!(list(&core, &w, "cafe", json!({"limit":1})), page);
    let changed=core.memory_command("revise",json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"],"expectedRevision":first["memory"]["revisionId"],"draft":{"title":"Replacement title","body":"new body"}})).unwrap();
    let restarted = Core::with_storage_root(root.clone());
    assert!(
        list(&restarted, &w, "世界", json!({}))["memories"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        list(&restarted, &w, "Replacement", json!({}))["memories"][0]["revisionId"],
        changed["memory"]["revisionId"]
    );
    let read=core.memory_command("read",json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"],"revisionId":first["memory"]["revisionId"]})).unwrap();
    assert_eq!(read["memory"]["sourceReferences"][0], "Synthetic reference");
    for extra in [
        json!({"query":""}),
        json!({"query":"* :"}),
        json!({"query":"x".repeat(257)}),
        json!({"query":"1 2 3 4 5 6 7 8 9"}),
        json!({"query":"cafe","id":first["memory"]["id"]}),
    ] {
        let mut request = json!({"contractVersion":1,"workspaceId":w});
        request
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert_eq!(
            core.memory_command("list", request).unwrap_err().code,
            ErrorCode::InvalidSearch
        );
    }
    // Inject a future Sealed current metadata row without creating any encrypted data.
    let db = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
    db.execute(
        "UPDATE memory_revisions SET metadata=json_set(metadata,'$.privacy','sealed') WHERE id=?1",
        [changed["memory"]["revisionId"].as_str().unwrap()],
    )
    .unwrap();
    assert!(
        list(&core, &w, "Replacement", json!({}))["memories"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        8
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name LIKE 'titles%'",
            [],
            |row| row.get::<_, u32>(0)
        )
        .unwrap(),
        0
    );
}
