use anansi_api::{ApiState, router};
use anansi_core::Core;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
async fn call(
    state: Arc<ApiState>,
    operation: &str,
    value: Value,
    authenticated: bool,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!("/v1/jobs/{operation}"))
        .header("host", "127.0.0.1:1451")
        .header("origin", "http://127.0.0.1:1420")
        .header("content-type", "application/json");
    if authenticated {
        builder = builder.header("authorization", "Bearer synthetic");
    }
    let response = router(state)
        .oneshot(builder.body(Body::from(value.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn backup_preview_restore_use_authenticated_owned_ids_and_require_consent() {
    let temp = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(temp.path().join("store"));
    let w = core
        .workspace_command("create", json!({"contractVersion":1,"name":"HTTP backup"}))
        .unwrap()["workspace"]["id"]
        .clone();
    let state = Arc::new(ApiState {
        core,
        token: "synthetic".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    });
    let request =
        json!({"contractVersion":1,"workspaceId":w,"application":false,"plaintextConsent":true});
    assert_eq!(
        call(state.clone(), "backup", request.clone(), false)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let mut denied = request.clone();
    denied["plaintextConsent"] = json!(false);
    assert_eq!(
        call(state.clone(), "backup", denied, true).await.0,
        StatusCode::BAD_REQUEST
    );
    let mut bad = request.clone();
    bad["path"] = json!("C:/private");
    assert_eq!(
        call(state.clone(), "backup", bad, true).await.0,
        StatusCode::BAD_REQUEST
    );
    let (status, job) = call(state.clone(), "backup", request, true).await;
    assert_eq!(status, StatusCode::OK);
    let mut complete = Value::Null;
    for _ in 0..300 {
        let (_, list) = call(
            state.clone(),
            "list",
            json!({"contractVersion":1,"workspaceId":w}),
            true,
        )
        .await;
        if let Some(j) = list["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|j| j["id"] == job["job"]["id"] && j["state"] == "Completed")
        {
            complete = j.clone();
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(!complete.is_null(), "Backup must actually finish");
    let preview_request = json!({"contractVersion":1,"workspaceId":w,"jobId":complete["id"],"artifactId":complete["artifactId"]});
    let (status, preview) = call(state.clone(), "preview", preview_request, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["records"], 0);
    let restored = json!({"contractVersion":1,"workspaceId":w,"jobId":complete["id"],"artifactId":complete["artifactId"],"name":"HTTP recovered","expectedHash":preview["sha256"],"confirm":true});
    let (status, value) = call(state.clone(), "restore", restored, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(value["workspaces"][0]["id"], w);
    let (status, schedule) = call(
        state,
        "schedule-read",
        json!({"contractVersion":1,"workspaceId":w}),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(schedule["enabled"], false);
}
