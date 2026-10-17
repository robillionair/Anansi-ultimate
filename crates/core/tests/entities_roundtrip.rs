use anansi_core::{Core, ErrorCode, storage::DATABASE_NAME};
use serde_json::json;

fn setup() -> (tempfile::TempDir, Core, String) {
    let temp = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(temp.path().join("store"));
    let w = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Entities fixture"}),
        )
        .unwrap();
    (temp, core, w["workspace"]["id"].as_str().unwrap().into())
}

#[test]
fn all_nine_kinds_roundtrip_with_specialized_attributes_and_restart() {
    let (temp, core, w) = setup();
    let store_root = temp.path().join("store");

    let kinds = [
        ("project", json!({})),
        (
            "task",
            json!({"dueDate": "2026-10-25T15:00:00Z", "completed": false}),
        ),
        ("person", json!({})),
        ("organization", json!({})),
        ("decision", json!({"eventDate": "2026-10-17T10:00:00Z"})),
        ("event", json!({"eventDate": "2026-10-17T14:30:00Z"})),
        ("document", json!({})),
        ("note", json!({})),
        ("memory", json!({})),
    ];

    let mut created_records = Vec::new();

    for (kind, extra_fields) in &kinds {
        let mut draft = json!({
            "title": format!("Title for {kind}"),
            "body": format!("Body content for {kind}")
        });
        if let Some(obj) = extra_fields.as_object() {
            for (k, v) in obj {
                draft[k] = v.clone();
            }
        }

        let res = core
            .memory_command(
                "create",
                json!({
                    "contractVersion": 1,
                    "workspaceId": w,
                    "kind": kind,
                    "draft": draft
                }),
            )
            .unwrap();

        assert_eq!(res["memory"]["kind"], *kind);
        assert_eq!(res["memory"]["title"], format!("Title for {kind}"));
        if *kind == "task" {
            assert_eq!(res["memory"]["dueDate"], "2026-10-25T15:00:00Z");
            assert_eq!(res["memory"]["completed"], false);
        } else if *kind == "decision" {
            assert_eq!(res["memory"]["eventDate"], "2026-10-17T10:00:00Z");
        } else if *kind == "event" {
            assert_eq!(res["memory"]["eventDate"], "2026-10-17T14:30:00Z");
        }

        created_records.push((kind.to_string(), res));
    }

    // Update the task to completed = true
    let task_record = &created_records[1].1;
    let task_id = task_record["memory"]["id"].as_str().unwrap();
    let task_rev = task_record["memory"]["revisionId"].as_str().unwrap();

    let updated_task = core
        .memory_command(
            "revise",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "id": task_id,
                "expectedRevision": task_rev,
                "draft": {
                    "title": "Title for task",
                    "body": "Body content for task completed",
                    "dueDate": "2026-10-25T15:00:00Z",
                    "completed": true
                }
            }),
        )
        .unwrap();

    assert_eq!(updated_task["memory"]["completed"], true);
    assert_eq!(updated_task["body"], "Body content for task completed");

    // Restart Core and verify all 9 entities persist
    drop(core);
    let restarted_core = Core::with_storage_root(store_root);

    let list = restarted_core
        .memory_command(
            "list",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "limit": 50
            }),
        )
        .unwrap();

    assert_eq!(list["memories"].as_array().unwrap().len(), 9);

    let read_task = restarted_core
        .memory_command(
            "read",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "id": task_id
            }),
        )
        .unwrap();

    assert_eq!(read_task["memory"]["completed"], true);
    assert_eq!(read_task["body"], "Body content for task completed");
}

#[test]
fn relationships_create_list_delete_and_restart() {
    let (temp, core, w) = setup();
    let store_root = temp.path().join("store");

    let p = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "project",
                "draft": {"title": "Alpha Project", "body": "Project overview"}
            }),
        )
        .unwrap();
    let project_id = p["memory"]["id"].as_str().unwrap();

    let m = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "memory",
                "draft": {"title": "Design Decision", "body": "Architecture notes"}
            }),
        )
        .unwrap();
    let memory_id = m["memory"]["id"].as_str().unwrap();

    // Create relationship: project -> memory (relates_to)
    let rel = core
        .relationship_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "draft": {
                    "sourceId": project_id,
                    "targetId": memory_id,
                    "predicate": "relates_to"
                }
            }),
        )
        .unwrap();

    let rel_id = rel["relationship"]["id"].as_str().unwrap();
    assert_eq!(rel["relationship"]["sourceId"], project_id);
    assert_eq!(rel["relationship"]["targetId"], memory_id);
    assert_eq!(rel["relationship"]["predicate"], "relates_to");

    // Verify relationship appears in list
    let list = core
        .relationship_command(
            "list",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "recordId": project_id
            }),
        )
        .unwrap();

    let items = list["relationships"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], rel_id);

    // Verify readMemory on both endpoints returns rel_id in relationships array
    let read_project = core
        .memory_command(
            "read",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "id": project_id
            }),
        )
        .unwrap();
    assert!(
        read_project["memory"]["relationships"]
            .as_array()
            .unwrap()
            .contains(&json!(rel_id))
    );

    let read_mem = core
        .memory_command(
            "read",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "id": memory_id
            }),
        )
        .unwrap();
    assert!(
        read_mem["memory"]["relationships"]
            .as_array()
            .unwrap()
            .contains(&json!(rel_id))
    );

    // Restart core and verify relationship persistence
    drop(core);
    let restarted = Core::with_storage_root(store_root);

    let list_after_restart = restarted
        .relationship_command(
            "list",
            json!({
                "contractVersion": 1,
                "workspaceId": w
            }),
        )
        .unwrap();
    assert_eq!(
        list_after_restart["relationships"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // Delete relationship
    let deleted = restarted
        .relationship_command(
            "delete",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "id": rel_id
            }),
        )
        .unwrap();
    assert_eq!(deleted["relationship"]["id"], rel_id);

    // Verify list is now empty
    let list_after_delete = restarted
        .relationship_command(
            "list",
            json!({
                "contractVersion": 1,
                "workspaceId": w
            }),
        )
        .unwrap();
    assert_eq!(
        list_after_delete["relationships"].as_array().unwrap().len(),
        0
    );
}

#[test]
fn relationships_reject_self_loops_cross_workspace_and_missing_endpoints() {
    let (temp, core, w1) = setup();

    // Create second workspace
    let w2_res = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Second Workspace"}),
        )
        .unwrap();
    let w2 = w2_res["workspace"]["id"].as_str().unwrap();

    let m1 = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "draft": {"title": "Memory in W1", "body": "Body W1"}
            }),
        )
        .unwrap();
    let m1_id = m1["memory"]["id"].as_str().unwrap();

    let m2 = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w2,
                "draft": {"title": "Memory in W2", "body": "Body W2"}
            }),
        )
        .unwrap();
    let m2_id = m2["memory"]["id"].as_str().unwrap();

    // 1. Self-referencing link (source == target)
    let self_loop_err = core
        .relationship_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "draft": {
                    "sourceId": m1_id,
                    "targetId": m1_id,
                    "predicate": "relates_to"
                }
            }),
        )
        .unwrap_err();
    assert_eq!(self_loop_err.code, ErrorCode::InvalidRequest);

    // 2. Cross-workspace link (m1 in w1, m2 in w2)
    let cross_err = core
        .relationship_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "draft": {
                    "sourceId": m1_id,
                    "targetId": m2_id,
                    "predicate": "relates_to"
                }
            }),
        )
        .unwrap_err();
    assert_eq!(cross_err.code, ErrorCode::NotFound);

    // 3. Non-existent endpoint
    let missing_err = core
        .relationship_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "draft": {
                    "sourceId": m1_id,
                    "targetId": "0123456789abcdef0123456789abcdef",
                    "predicate": "relates_to"
                }
            }),
        )
        .unwrap_err();
    assert_eq!(missing_err.code, ErrorCode::NotFound);

    // 4. Invalid predicate
    let bad_pred_err = core
        .relationship_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "draft": {
                    "sourceId": m1_id,
                    "targetId": m2_id,
                    "predicate": "bad predicate with spaces!"
                }
            }),
        )
        .unwrap_err();
    assert_eq!(bad_pred_err.code, ErrorCode::InvalidRequest);

    drop(temp);
}

#[test]
fn schema_four_migrates_forward_to_schema_five_preserving_records() {
    let (temp, core, w) = setup();
    let memory = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "draft": {"title": "Legacy Memory", "body": "Preserved content"}
            }),
        )
        .unwrap();

    let path = temp.path().join("store").join(DATABASE_NAME);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE relationships; PRAGMA user_version = 4;")
        .unwrap();
    let v: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(v, 4);
    drop(conn);

    // Reopen with Core; it should detect version 4 and run migration 5
    let mem_read = core
        .memory_command(
            "read",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "id": memory["memory"]["id"]
            }),
        )
        .unwrap();
    assert_eq!(mem_read["memory"]["title"], "Legacy Memory");
    assert_eq!(mem_read["body"], "Preserved content");

    let verify_conn = rusqlite::Connection::open(&path).unwrap();
    let user_version: i64 = verify_conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(user_version, 5);
}
