//! M6 Slice 1 over real HTTP: the artifact registry workflow on
//! the §9 project identity — register, revise (with a stale
//! write refused as 409), read both immutable versions, sealed
//! refused, and cross-workspace isolation, through the actual
//! `/v1/artifacts/*` routes and bearer/origin pipeline.

use anansi_api::{ApiState, router};
use anansi_core::Core;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

async fn call(
    app: &axum::Router,
    token: &str,
    group: &str,
    op: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/v1/{group}/{op}"))
                .header(header::ORIGIN, "http://127.0.0.1:1420")
                .header(header::HOST, "127.0.0.1:1451")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn artifact_registry_workflow_over_http() {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let token = "a".repeat(32);
    let app = router(Arc::new(ApiState {
        core,
        token: token.clone(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    }));

    let (dbg_status, ws_a) = call(
        &app,
        &token,
        "workspaces",
        "create",
        json!({ "contractVersion": 1, "name": "A" }),
    )
    .await;
    assert_eq!(dbg_status, StatusCode::OK, "{ws_a}");
    let a = ws_a["workspace"]["id"].as_str().unwrap().to_string();
    let (_, ws_b) = call(
        &app,
        &token,
        "workspaces",
        "create",
        json!({ "contractVersion": 1, "name": "B" }),
    )
    .await;
    let b = ws_b["workspace"]["id"].as_str().unwrap().to_string();

    let draft = json!({ "title": "Harbor App", "body": "project", "privacy": "private",
                        "eventDate": null, "validFrom": null, "validUntil": null,
                        "confidence": null, "sourceReferences": [] });
    let (status, project) = call(
        &app,
        &token,
        "memories",
        "create",
        json!({ "contractVersion": 1, "workspaceId": a, "kind": "project", "draft": draft }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let pid = project["memory"]["id"].as_str().unwrap().to_string();

    let (status, v1) = call(
        &app,
        &token,
        "artifacts",
        "register",
        json!({ "contractVersion": 1, "workspaceId": a, "projectId": pid,
                "type": "concept", "title": "Concept", "body": "draft one",
                "sourceReferences": [], "privacy": "private" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{v1}");
    let id = v1["artifact"]["artifactId"].as_str().unwrap().to_string();
    assert_eq!(v1["artifact"]["validationStatus"], "unvalidated");

    let (status, _) = call(
        &app,
        &token,
        "artifacts",
        "revise",
        json!({ "contractVersion": 1, "workspaceId": a, "artifactId": id,
                "expectedVersion": 9, "title": "Concept", "body": "stale", "sourceReferences": [] }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "stale write -> 409");

    let (status, v2) = call(
        &app,
        &token,
        "artifacts",
        "revise",
        json!({ "contractVersion": 1, "workspaceId": a, "artifactId": id,
                "expectedVersion": 1, "title": "Concept", "body": "draft two", "sourceReferences": [] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v2["artifact"]["version"], 2);

    let (status, back) = call(
        &app,
        &token,
        "artifacts",
        "read",
        json!({ "contractVersion": 1, "workspaceId": a, "artifactId": id, "version": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(back["body"], "draft one", "v1 immutable over HTTP too");

    // Isolation: B sees nothing of A.
    let (status, _) = call(
        &app,
        &token,
        "artifacts",
        "read",
        json!({ "contractVersion": 1, "workspaceId": b, "artifactId": id }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // Sealed refused through the API mapping.
    let (status, _) = call(
        &app,
        &token,
        "artifacts",
        "register",
        json!({ "contractVersion": 1, "workspaceId": a, "projectId": pid,
                "type": "note", "title": "T", "body": "b", "sourceReferences": [], "privacy": "sealed" }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
