use anansi_api::{ApiState, router};
use anansi_core::Core;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

fn request(path: &str, value: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("host", "127.0.0.1:1451")
        .header("origin", "http://127.0.0.1:1420")
        .header("authorization", "Bearer synthetic")
        .header("content-type", "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}

async fn call(state: Arc<ApiState>, r: Request<Body>) -> (StatusCode, Value) {
    let response = router(state).oneshot(r).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn entities_all_kinds_and_specialized_attributes_via_http() {
    let temp = tempfile::tempdir().unwrap();
    let state = Arc::new(ApiState {
        core: Core::with_storage_root(temp.path().join("store")),
        token: "synthetic".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    });

    let w = state
        .core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Entities HTTP"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // 1. Create a Project entity
    let (status, proj_res) = call(
        state.clone(),
        request(
            "/v1/memories/create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "project",
                "draft": {
                    "title": "Quantum Leap",
                    "body": "Developing the next generation engine."
                }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let proj_id = proj_res["memory"]["id"].as_str().unwrap().to_string();

    // 2. Create a Task entity referencing the project with due date and completion
    let (status, task_res) = call(
        state.clone(),
        request(
            "/v1/memories/create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "task",
                "draft": {
                    "title": "Wire engine sensors",
                    "body": "Ensure harness is grounded",
                    "projectId": proj_id,
                    "dueDate": "2026-11-01T12:00:00Z",
                    "completed": false
                }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let task_id = task_res["memory"]["id"].as_str().unwrap().to_string();
    assert_eq!(task_res["memory"]["projectId"], proj_id);
    assert_eq!(task_res["memory"]["completed"], false);

    let task_rev_id = task_res["memory"]["revisionId"]
        .as_str()
        .unwrap()
        .to_string();

    // Complete the task via revision
    let (status, task_revised) = call(
        state.clone(),
        request(
            "/v1/memories/revise",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "id": task_id,
                "expectedRevision": task_rev_id,
                "kind": "task",
                "draft": {
                    "title": "Wire engine sensors",
                    "body": "Sensor harness wired and verified",
                    "projectId": proj_id,
                    "dueDate": "2026-11-01T12:00:00Z",
                    "completed": true
                }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(task_revised["memory"]["completed"], true);

    // 3. Verify all other 7 kinds roundtrip
    let kinds = [
        "memory",
        "note",
        "person",
        "organization",
        "decision",
        "event",
        "document",
    ];
    for kind in kinds {
        let mut draft = json!({
            "title": format!("Entity {kind}"),
            "body": format!("Body for {kind}")
        });
        if kind == "event" || kind == "decision" {
            draft["eventDate"] = json!("2026-12-31T23:59:59Z");
        }
        let (status, res) = call(
            state.clone(),
            request(
                "/v1/memories/create",
                json!({
                    "contractVersion": 1,
                    "workspaceId": w,
                    "kind": kind,
                    "draft": draft
                }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(res["memory"]["kind"], kind);
        if kind == "event" || kind == "decision" {
            assert_eq!(res["memory"]["eventDate"], "2026-12-31T23:59:59Z");
        }
    }

    // 4. Invalid kind rejected
    let (status, err) = call(
        state.clone(),
        request(
            "/v1/memories/create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "invalid_kind",
                "draft": {
                    "title": "Bad kind",
                    "body": "Should fail"
                }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["code"], "INVALID_MEMORY");
}

#[tokio::test]
async fn relationships_http_endpoints_and_security_guards() {
    let temp = tempfile::tempdir().unwrap();
    let state = Arc::new(ApiState {
        core: Core::with_storage_root(temp.path().join("store")),
        token: "synthetic".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    });

    let w1 = state
        .core
        .workspace_command("create", json!({"contractVersion":1,"name":"Rel WS1"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let w2 = state
        .core
        .workspace_command("create", json!({"contractVersion":1,"name":"Rel WS2"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create 2 entities in w1
    let (status, r1) = call(
        state.clone(),
        request(
            "/v1/memories/create",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "kind": "person",
                "draft": {
                    "title": "Alice Developer",
                    "body": "Lead architect"
                }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let alice_id = r1["memory"]["id"].as_str().unwrap().to_string();

    let (status, r2) = call(
        state.clone(),
        request(
            "/v1/memories/create",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "kind": "project",
                "draft": {
                    "title": "Anansi Core",
                    "body": "Memory subsystem"
                }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let proj_id = r2["memory"]["id"].as_str().unwrap().to_string();

    // Create entity in w2
    let (status, r3) = call(
        state.clone(),
        request(
            "/v1/memories/create",
            json!({
                "contractVersion": 1,
                "workspaceId": w2,
                "kind": "person",
                "draft": {
                    "title": "Bob Contributor",
                    "body": "Remote engineer"
                }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let bob_id = r3["memory"]["id"].as_str().unwrap().to_string();

    // 1. Create valid relationship Alice -> works_on -> Anansi Core
    let (status, created) = call(
        state.clone(),
        request(
            "/v1/relationships/create",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "draft": {
                    "sourceId": alice_id,
                    "targetId": proj_id,
                    "predicate": "works_on",
                    "metadata": { "role": "lead" }
                }
            }),
        ),
    )
    .await;
    if status != StatusCode::OK {
        panic!("create relationship failed with status {status}: {created}");
    }
    assert_eq!(status, StatusCode::OK);
    let rel_id = created["relationship"]["id"].as_str().unwrap().to_string();
    assert_eq!(created["relationship"]["predicate"], "works_on");

    // Verify reading Alice shows relationship ID in metadata
    let (status, alice_mem) = call(
        state.clone(),
        request(
            "/v1/memories/read",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "id": alice_id
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rels = alice_mem["memory"]["relationships"].as_array().unwrap();
    assert!(rels.iter().any(|r| r.as_str() == Some(&rel_id)));

    // 2. List relationships by record_id
    let (status, listed) = call(
        state.clone(),
        request(
            "/v1/relationships/list",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "recordId": alice_id
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = listed["relationships"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], rel_id);

    // 3. Reject self-loop
    let (status, err) = call(
        state.clone(),
        request(
            "/v1/relationships/create",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "draft": {
                    "sourceId": alice_id,
                    "targetId": alice_id,
                    "predicate": "relates_to"
                }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["code"], "INVALID_REQUEST");

    // 4. Reject cross-workspace relationship
    let (status, _err) = call(
        state.clone(),
        request(
            "/v1/relationships/create",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "draft": {
                    "sourceId": alice_id,
                    "targetId": bob_id,
                    "predicate": "collaborates_with"
                }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 5. Delete relationship
    let (status, del) = call(
        state.clone(),
        request(
            "/v1/relationships/delete",
            json!({
                "contractVersion": 1,
                "workspaceId": w1,
                "id": rel_id, "expectedVersion": 1
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(del["relationship"]["id"], rel_id);

    let (status, _) = call(
        state.clone(),
        request(
            "/v1/relationships/restore",
            json!({"contractVersion":1,"workspaceId":w1,"id":rel_id,"expectedVersion":1}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, restored) = call(
        state.clone(),
        request(
            "/v1/relationships/restore",
            json!({"contractVersion":1,"workspaceId":w1,"id":rel_id,"expectedVersion":2}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(restored["relationship"]["version"], 3);
    let (status, history) = call(
        state.clone(),
        request(
            "/v1/relationships/list",
            json!({"contractVersion":1,"workspaceId":w1,"recordId":alice_id,"includeHistory":true}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(history["relationships"].as_array().unwrap().len(), 3);

    // 6. Security guards: auth, host, origin
    for (header, value, expected_status) in [
        ("authorization", "Bearer wrong", StatusCode::UNAUTHORIZED),
        ("host", "evil.example", StatusCode::FORBIDDEN),
        ("origin", "https://evil.example", StatusCode::FORBIDDEN),
    ] {
        let mut r = request(
            "/v1/relationships/list",
            json!({"contractVersion":1,"workspaceId":w1}),
        );
        r.headers_mut().insert(header, value.parse().unwrap());
        assert_eq!(call(state.clone(), r).await.0, expected_status);
    }

    // 7. OPTIONS preflight
    for op in ["create", "delete", "restore", "list"] {
        let mut r = request(&format!("/v1/relationships/{op}"), json!({}));
        *r.method_mut() = axum::http::Method::OPTIONS;
        r.headers_mut()
            .insert("access-control-request-method", "POST".parse().unwrap());
        r.headers_mut().insert(
            "access-control-request-headers",
            "authorization,content-type".parse().unwrap(),
        );
        let response = router(state.clone()).oneshot(r).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
}
