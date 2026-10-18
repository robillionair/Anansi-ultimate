use anansi_core::{Core, ErrorCode, storage::DATABASE_NAME};
use serde_json::{Value, json};

fn setup() -> (tempfile::TempDir, Core, String) {
    let temp = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(temp.path().join("store"));
    let w = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Lifecycle fixture"}),
        )
        .unwrap();
    (temp, core, w["workspace"]["id"].as_str().unwrap().into())
}
fn create(core: &Core, w: &str, kind: &str) -> Value {
    core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"kind":kind,"draft":{"title":"Synthetic","body":"Original"}})).unwrap()
}
fn request(w: &str, m: &Value) -> Value {
    json!({"contractVersion":1,"workspaceId":w,"id":m["memory"]["id"],"expectedRevision":m["memory"]["revisionId"],"draft":{"title":"Synthetic","body":"Original","validUntil":"2099-01-01T00:00:00Z"}})
}
fn change(core: &Core, w: &str, m: &Value, state: &str, successor: Option<&str>) -> Value {
    let mut r = request(w, m);
    r["lifecycle"] = json!({"state":state,"confirmed":true,"successorId":successor});
    core.memory_command("revise", r).unwrap()
}
#[test]
fn notes_lifecycle_archive_history_restore_and_restart() {
    let (temp, core, w) = setup();
    let original = create(&core, &w, "note");
    let confirmed = change(&core, &w, &original, "confirmed", None);
    let retained = change(&core, &w, &confirmed, "durable", None);
    let archived = change(&core, &w, &retained, "archived", None);
    let list = |extra: Value| {
        let mut r = json!({"contractVersion":1,"workspaceId":w});
        r.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        core.memory_command("list", r).unwrap()
    };
    assert_eq!(list(json!({}))["memories"].as_array().unwrap().len(), 0);
    assert_eq!(
        list(json!({"includeArchived":true}))["memories"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let mut r = request(&w, &archived);
    r["restoreArchive"] = json!(true);
    let restored = core.memory_command("revise", r).unwrap();
    assert_eq!(restored["memory"]["status"], "durable");
    let mut r = request(&w, &restored);
    r["draft"]["body"] = json!("New text");
    let edited = core.memory_command("revise", r).unwrap();
    let mut r = request(&w, &edited);
    r["restoreRevision"] = original["memory"]["revisionId"].clone();
    let final_record = core.memory_command("revise", r).unwrap();
    assert_eq!(final_record["body"], "Original");
    assert_eq!(final_record["memory"]["status"], "durable");
    assert_eq!(final_record["memory"]["kind"], "note");
    assert_eq!(final_record["memory"]["version"], 7);
    assert_eq!(
        final_record["memory"]["createdAt"],
        original["memory"]["createdAt"]
    );
    let restarted = Core::with_storage_root(temp.path().join("store"));
    let read = json!({"contractVersion":1,"workspaceId":w,"id":original["memory"]["id"]});
    assert_eq!(
        restarted.memory_command("read", read).unwrap(),
        final_record
    );
    let mut stale = request(&w, &edited);
    stale["restoreArchive"] = json!(true);
    assert_eq!(
        core.memory_command("revise", stale).unwrap_err().code,
        ErrorCode::RevisionConflict
    );
}
#[test]
fn successor_self_cycle_cross_workspace_and_consent_are_rejected() {
    let (_temp, core, w) = setup();
    let a = change(&core, &w, &create(&core, &w, "memory"), "confirmed", None);
    let b = change(&core, &w, &create(&core, &w, "memory"), "confirmed", None);
    let mut r = request(&w, &a);
    r["lifecycle"] = json!({"state":"durable","confirmed":false});
    assert_eq!(
        core.memory_command("revise", r.clone()).unwrap_err().code,
        ErrorCode::InvalidMemory
    );
    r["lifecycle"] = json!({"state":"superseded","confirmed":true,"successorId":a["memory"]["id"]});
    assert!(core.memory_command("revise", r.clone()).is_err());
    let a = change(&core, &w, &a, "superseded", b["memory"]["id"].as_str());
    r = request(&w, &b);
    r["lifecycle"] = json!({"state":"superseded","confirmed":true,"successorId":a["memory"]["id"]});
    assert_eq!(
        core.memory_command("revise", r.clone()).unwrap_err().code,
        ErrorCode::InvalidMemory
    );
    let other = core
        .workspace_command("create", json!({"contractVersion":1,"name":"Other"}))
        .unwrap();
    let other_w = other["workspace"]["id"].as_str().unwrap();
    let foreign = create(&core, other_w, "memory");
    r["lifecycle"]["successorId"] = foreign["memory"]["id"].clone();
    assert_eq!(
        core.memory_command("revise", r).unwrap_err().code,
        ErrorCode::MemoryNotFound
    );
}
#[test]
fn temporary_expiry_is_filtered_never_deleted_and_requires_valid_expiry() {
    let (_temp, core, w) = setup();
    let first = create(&core, &w, "memory");
    let mut r = request(&w, &first);
    r["draft"]["validUntil"] = Value::Null;
    r["lifecycle"] = json!({"state":"temporary","confirmed":true});
    assert!(core.memory_command("revise", r.clone()).is_err());
    r["draft"]["validUntil"] = json!("2000-01-01T00:00:00Z");
    assert!(core.memory_command("revise", r).is_err());
    let temp = change(&core, &w, &first, "temporary", None);
    let mut r = request(&w, &temp);
    r["draft"]["validUntil"] = json!("2000-01-01T00:00:00Z");
    let expired = core.memory_command("revise", r).unwrap();
    assert_eq!(
        core.memory_command("list", json!({"contractVersion":1,"workspaceId":w}))
            .unwrap()["memories"],
        json!([])
    );
    assert_eq!(
        core.memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":w,"includeExpired":true})
        )
        .unwrap()["memories"][0],
        expired["memory"]
    );
    assert_eq!(
        core.memory_command(
            "read",
            json!({"contractVersion":1,"workspaceId":w,"id":expired["memory"]["id"]})
        )
        .unwrap()["body"],
        "Original"
    );
}
#[test]
fn generated_origin_is_preserved_without_automatic_confirmation_and_schema3_migrates() {
    let (temp, core, w) = setup();
    let first = create(&core, &w, "memory");
    let db = rusqlite::Connection::open(temp.path().join("store").join(DATABASE_NAME)).unwrap();
    // A synthetic future-generator fixture, never a model call or public create capability.
    db.execute("UPDATE memory_revisions SET metadata=json_set(metadata,'$.provenance','generated','$.generatorModel','synthetic-fixture')",[]).unwrap();
    db.execute_batch("DROP TABLE lifecycle_format; DROP TABLE IF EXISTS relationship_revisions; DROP TABLE IF EXISTS relationships; DROP TABLE IF EXISTS jobs; PRAGMA user_version=3;")
        .unwrap();
    drop(db);
    let edited = core.memory_command("revise", request(&w, &first)).unwrap();
    assert_eq!(edited["memory"]["status"], "proposed");
    assert_eq!(edited["memory"]["provenance"], "generated");
    assert_eq!(edited["memory"]["generatorModel"], "synthetic-fixture");
    assert!(
        core.workspace_command("open", json!({"contractVersion":1,"id":w}))
            .unwrap()["schemaVersion"]
            .as_u64()
            .unwrap()
            >= 4
    );
    let mut r = request(&w, &edited);
    r["draft"]["privacy"] = json!("public");
    assert_eq!(
        core.memory_command("revise", r.clone()).unwrap_err().code,
        ErrorCode::InvalidMemory
    );
    r["confirmPrivacyChange"] = json!(true);
    let public = core.memory_command("revise", r).unwrap();
    let mut r = request(&w, &public);
    r["restoreRevision"] = first["memory"]["revisionId"].clone();
    let restored = core.memory_command("revise", r).unwrap();
    assert_eq!(restored["memory"]["privacy"], "public");
    assert_eq!(restored["memory"]["status"], "proposed");
}

#[test]
fn failed_schema_four_migration_preserves_schema_three_records_and_settings() {
    let (temp, core, w) = setup();
    let first = create(&core, &w, "memory");
    let path = temp.path().join("store").join(DATABASE_NAME);
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("DROP TABLE lifecycle_format; CREATE TABLE lifecycle_format(wrong TEXT); DROP TABLE IF EXISTS relationship_revisions; DROP TABLE IF EXISTS relationships; DROP TABLE IF EXISTS jobs; PRAGMA user_version=3;").unwrap();
    let before: String = db
        .query_row("SELECT metadata FROM memory_revisions", [], |r| r.get(0))
        .unwrap();
    drop(db);
    assert!(
        core.memory_command(
            "read",
            json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"]})
        )
        .is_err()
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        db.query_row("SELECT metadata FROM memory_revisions", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        before
    );
    db.execute_batch("DROP TABLE lifecycle_format;").unwrap();
    drop(db);
    assert_eq!(
        core.memory_command(
            "read",
            json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"]})
        )
        .unwrap(),
        first
    );
    assert_eq!(
        core.workspace_command("open", json!({"contractVersion":1,"id":w}))
            .unwrap()["settings"]["intent"],
        "explore"
    );
}

#[test]
fn competing_successor_links_cannot_publish_a_cycle() {
    let (_temp, core, w) = setup();
    let a = change(&core, &w, &create(&core, &w, "memory"), "confirmed", None);
    let b = change(&core, &w, &create(&core, &w, "memory"), "confirmed", None);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = [(&a, &b), (&b, &a)]
        .into_iter()
        .map(|(from, to)| {
            let core = core.clone();
            let barrier = barrier.clone();
            let mut r = request(&w, from);
            r["lifecycle"] =
                json!({"state":"superseded","confirmed":true,"successorId":to["memory"]["id"]});
            std::thread::spawn(move || {
                barrier.wait();
                core.memory_command("revise", r)
            })
        })
        .collect();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| r
                .as_ref()
                .is_err_and(|e| e.code == ErrorCode::InvalidMemory))
            .count(),
        1
    );
}
