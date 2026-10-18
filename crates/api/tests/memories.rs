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
async fn memory_http_parity_auth_scope_limits_and_revisions() {
    let temp = tempfile::tempdir().unwrap();
    let state = Arc::new(ApiState {
        core: Core::with_storage_root(temp.path().join("store")),
        token: "synthetic".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    });
    let w = state
        .core
        .workspace_command("create", json!({"contractVersion":1,"name":"HTTP memory"}))
        .unwrap()["workspace"]["id"]
        .clone();
    let input = json!({"contractVersion":1,"workspaceId":w,"draft":{"title":"HTTP 世界","body":"x".repeat(4096)}});
    let (status, first) = call(state.clone(), request("/v1/memories/create", input.clone())).await;
    assert_eq!(status, StatusCode::OK);
    let search = json!({"contractVersion":1,"workspaceId":w,"query":"世界"});
    let (search_status, matches) =
        call(state.clone(), request("/v1/memories/list", search.clone())).await;
    assert_eq!(search_status, StatusCode::OK);
    assert_eq!(matches, state.core.memory_command("list", search).unwrap());
    assert_eq!(
        matches["memories"][0]["revisionId"],
        first["memory"]["revisionId"]
    );
    let (bad_search, _) = call(
        state.clone(),
        request(
            "/v1/memories/list",
            json!({"contractVersion":1,"workspaceId":w,"query":"***"}),
        ),
    )
    .await;
    assert_eq!(bad_search, StatusCode::BAD_REQUEST);
    let read = json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"]});
    let (_, actual) = call(state.clone(), request("/v1/memories/read", read.clone())).await;
    assert_eq!(actual, state.core.memory_command("read", read).unwrap());
    for (op, bad) in [
        (
            "create",
            json!({"contractVersion":1,"workspaceId":w,"draft":{"title":"bad","body":"ok","path":"outside"}}),
        ),
        (
            "list",
            json!({"contractVersion":1,"workspaceId":w,"limit":101}),
        ),
        (
            "read",
            json!({"contractVersion":1,"workspaceId":w,"id":"../outside"}),
        ),
    ] {
        let expected = state.core.memory_command(op, bad.clone()).unwrap_err();
        let (_, error) = call(state.clone(), request(&format!("/v1/memories/{op}"), bad)).await;
        assert_eq!(error["code"], serde_json::to_value(expected.code).unwrap());
    }
    for (header, value, status) in [
        ("authorization", "Bearer wrong", StatusCode::UNAUTHORIZED),
        ("host", "evil.example", StatusCode::FORBIDDEN),
        ("origin", "https://evil.example", StatusCode::FORBIDDEN),
    ] {
        let mut r = request("/v1/memories/create", input.clone());
        r.headers_mut().insert(header, value.parse().unwrap());
        assert_eq!(call(state.clone(), r).await.0, status);
    }
    let huge = json!({"contractVersion":1,"workspaceId":w,"draft":{"title":"oversize","body":"x".repeat(320*1024)}});
    assert_eq!(
        call(state.clone(), request("/v1/memories/create", huge))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            state.clone(),
            request(
                "/v1/workspaces/create",
                json!({"contractVersion":1,"name":"x".repeat(2048)})
            )
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut sealed = input;
    sealed["draft"]["privacy"] = json!("sealed");
    assert_eq!(
        call(state.clone(), request("/v1/memories/create", sealed))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    for op in ["create", "read", "revise", "list"] {
        let mut r = request(&format!("/v1/memories/{op}"), json!({}));
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

#[tokio::test]
async fn lifecycle_notes_and_filters_share_core_policy() {
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
            json!({"contractVersion":1,"name":"Lifecycle HTTP"}),
        )
        .unwrap()["workspace"]["id"]
        .clone();
    let (_,note)=call(state.clone(),request("/v1/memories/create",json!({"contractVersion":1,"workspaceId":w,"kind":"note","draft":{"title":"HTTP note","body":"literal"}}))).await;
    assert_eq!(note["memory"]["kind"], "note");
    let input = json!({"contractVersion":1,"workspaceId":w,"id":note["memory"]["id"],"expectedRevision":note["memory"]["revisionId"],"draft":{"title":"HTTP note","body":"literal"},"lifecycle":{"state":"confirmed","confirmed":true}});
    let (status, confirmed) =
        call(state.clone(), request("/v1/memories/revise", input.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(confirmed["memory"]["status"], "confirmed");
    assert_eq!(
        call(state.clone(), request("/v1/memories/revise", input))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let list = json!({"contractVersion":1,"workspaceId":w,"kind":"note","includeArchived":true,"includeExpired":true});
    assert_eq!(
        call(state.clone(), request("/v1/memories/list", list.clone()))
            .await
            .1,
        state.core.memory_command("list", list).unwrap()
    );
}
