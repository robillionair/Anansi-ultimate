use anansi_api::{ApiState, router};
use anansi_core::Core;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

async fn call(state: Arc<ApiState>, operation: &str, value: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/v1/jobs/{operation}"))
        .header("host", "127.0.0.1:1451")
        .header("origin", "http://127.0.0.1:1420")
        .header("authorization", "Bearer synthetic")
        .header("content-type", "application/json")
        .body(Body::from(value.to_string()))
        .unwrap();
    let response = router(state).oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn authenticated_jobs_export_complete_chunks_and_reject_paths_and_other_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(temp.path().join("store"));
    let w = core
        .workspace_command("create", json!({"contractVersion":1,"name":"HTTP export"}))
        .unwrap()["workspace"]["id"]
        .clone();
    let other = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Other HTTP export"}),
        )
        .unwrap()["workspace"]["id"]
        .clone();
    let state = Arc::new(ApiState {
        core,
        token: "synthetic".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    });
    let (status,rejected)=call(state.clone(),"export",json!({"contractVersion":1,"workspaceId":w,"selection":{"kind":"workspace"},"path":"C:/private"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!rejected.to_string().contains("C:/private"));
    let (status, queued) = call(
        state.clone(),
        "export",
        json!({"contractVersion":1,"workspaceId":w,"selection":{"kind":"workspace"}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = queued["job"]["id"].clone();
    let mut complete = Value::Null;
    for _ in 0..100 {
        let (status, list) = call(
            state.clone(),
            "list",
            json!({"contractVersion":1,"workspaceId":w}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        if list["jobs"][0]["state"] == "Completed" {
            complete = list["jobs"][0].clone();
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert_eq!(complete["state"], "Completed");
    assert!(complete.get("owner").is_none());
    let request = json!({"contractVersion":1,"workspaceId":w,"jobId":id,"artifactId":complete["artifactId"],"offset":0});
    let (status, chunk) = call(state.clone(), "artifact", request.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!chunk["bytes"].as_array().unwrap().is_empty());
    let mut wrong = request.clone();
    wrong["workspaceId"] = other;
    let (status, _) = call(state.clone(), "artifact", wrong).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let mut path = request;
    path["destination"] = json!("C:/escape");
    let (status, value) = call(state, "artifact", path).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!value.to_string().contains("escape"));
}

#[tokio::test]
async fn authenticated_pause_cancel_and_retry_fence_old_worker_and_finish_without_ui_polling() {
    use anansi_core::{
        jobs::{ExportSelection, JobState},
        storage::Storage,
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let storage = Storage::new(root.clone()).unwrap();
    let workspace = storage.create("HTTP job controls").unwrap().workspace.id;
    let job = storage
        .enqueue_export(&workspace, ExportSelection::Workspace)
        .unwrap();
    let lease = storage.claim_export().unwrap().unwrap();
    let state = Arc::new(ApiState {
        core: Core::with_storage_root(root),
        token: "synthetic".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    });
    let control = |version, action| json!({"contractVersion":1,"workspaceId":workspace,"id":job.id,"expectedVersion":version,"action":action});
    let (status, paused) = call(
        state.clone(),
        "control",
        control(lease.job.version, "pause"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(paused["job"]["state"], "Paused");
    assert!(storage.execute_export(&lease).is_err());
    assert!(
        storage
            .read_job(&workspace, &job.id)
            .unwrap()
            .artifact_id
            .is_none()
    );
    let (status, _) = call(
        state.clone(),
        "control",
        control(lease.job.version, "cancel"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, cancelled) = call(
        state.clone(),
        "control",
        control(paused["job"]["version"].as_u64().unwrap() as u32, "cancel"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancelled["job"]["state"], "Cancelled");
    let (status, _) = call(
        state,
        "control",
        control(
            cancelled["job"]["version"].as_u64().unwrap() as u32,
            "retry",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        let current = storage.read_job(&workspace, &job.id).unwrap();
        if current.state == JobState::Completed {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}
