use anansi_api::{ApiState, router};
use anansi_core::{CommandError, Core};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
fn state(root: std::path::PathBuf) -> Arc<ApiState> {
    Arc::new(ApiState {
        core: Core::with_storage_root(root),
        token: "synthetic-workspace-test".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    })
}
fn request(operation: &str, input: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/v1/workspaces/{operation}"))
        .header("host", "127.0.0.1:1451")
        .header("origin", "http://127.0.0.1:1420")
        .header("content-type", "application/json")
        .header("authorization", "Bearer synthetic-workspace-test")
        .body(Body::from(input.to_string()))
        .unwrap()
}
async fn call(state: Arc<ApiState>, operation: &str, input: Value) -> (StatusCode, Value) {
    let response = router(state)
        .oneshot(request(operation, input))
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status();
    let value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    (status, value)
}
#[tokio::test]
async fn authenticated_workspace_http_matches_core_and_preserves_settings() {
    let temp = tempfile::tempdir().unwrap();
    let s = state(temp.path().join("store"));
    let (status, created) = call(
        s.clone(),
        "create",
        json!({"contractVersion":1,"name":"Synthetic HTTP"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = created["workspace"]["id"].as_str().unwrap();
    let input = json!({"contractVersion":1,"id":id});
    let (_, opened) = call(s.clone(), "open", input.clone()).await;
    assert_eq!(opened, s.core.workspace_command("open", input).unwrap());
    let (_, updated) = call(
        s.clone(),
        "settings",
        json!({"contractVersion":1,"id":id,"reducedMotion":true}),
    )
    .await;
    assert_eq!(updated["settings"]["reducedMotion"], true);
    assert_eq!(
        updated,
        s.core
            .workspace_command("open", json!({"contractVersion":1,"id":id}))
            .unwrap()
    );
    for (operation, input) in [
        (
            "create",
            json!({"contractVersion":1,"name":"Synthetic HTTP"}),
        ),
        ("open", json!({"contractVersion":1,"id":"../outside"})),
        (
            "settings",
            json!({"contractVersion":1,"id":id,"reducedMotion":"true"}),
        ),
        ("list", json!({"contractVersion":1,"path":"outside"})),
    ] {
        let expected = s
            .core
            .workspace_command(operation, input.clone())
            .unwrap_err();
        let (status, value) = call(s.clone(), operation, input).await;
        assert!(status.is_client_error());
        let actual: CommandError = serde_json::from_value(value).unwrap();
        assert_eq!(actual, expected);
    }
}
#[tokio::test]
async fn every_workspace_route_requires_pairing_and_exact_origin_before_any_write() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let s = state(root.clone());
    for operation in ["list", "create", "open", "settings"] {
        let mut missing = request(operation, json!({"contractVersion":1,"name":"Synthetic"}));
        missing.headers_mut().remove("authorization");
        assert_eq!(
            router(s.clone()).oneshot(missing).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        let mut hostile = request(operation, json!({"contractVersion":1,"name":"Synthetic"}));
        hostile
            .headers_mut()
            .insert("origin", "https://evil.test".parse().unwrap());
        assert_eq!(
            router(s.clone()).oneshot(hostile).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        let preflight = Request::builder()
            .method("OPTIONS")
            .uri(format!("/v1/workspaces/{operation}"))
            .header("host", "127.0.0.1:1451")
            .header("origin", "http://127.0.0.1:1420")
            .header("access-control-request-method", "POST")
            .header(
                "access-control-request-headers",
                "authorization,content-type",
            )
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            router(s.clone()).oneshot(preflight).await.unwrap().status(),
            StatusCode::NO_CONTENT
        );
    }
    assert!(!root.exists());
}
