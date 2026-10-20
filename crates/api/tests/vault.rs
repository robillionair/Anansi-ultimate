use anansi_api::{ApiState, router};
use anansi_core::{
    Core,
    jobs::ExportSelection,
    storage::Storage,
    vault::{service::AdapterVaultState, session::Transport},
};
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
async fn browser_vault_authorization_and_encrypted_backup_restore_http_boundary() {
    unsafe {
        std::env::set_var("ANANSI_VAULT_TEST_PROVIDER", "1");
        std::env::remove_var("ANANSI_VAULT_TEST_CANCEL");
        std::env::remove_var("ANANSI_VAULT_TEST_PASSPHRASE");
        std::env::remove_var("ANANSI_VAULT_TEST_SOURCE_PASSPHRASE");
    }

    let temp = tempfile::tempdir().unwrap();
    let store_root = temp.path().join("store");
    let state = Arc::new(ApiState {
        core: Core::with_storage_root(store_root.clone()),
        token: "synthetic".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    });

    let workspace = state
        .core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Browser vault HTTP"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    state
        .core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace,
                "kind": "note",
                "draft": {
                    "title": "Paired browser secret note",
                    "body": "Encrypted payload read via paired browser session.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();

    // 1. Browser cannot perform initial vault setup or seal
    let (status, body) = call(
        state.clone(),
        request(
            "/v1/vault/setup",
            json!({"contractVersion":1,"workspaceId":workspace}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "FORBIDDEN");

    // 2. Reject secret-bearing DTO fields on HTTP vault routes
    for forbidden in ["passphrase", "password", "recoveryKey", "dek", "token"] {
        let (status, body) = call(
            state.clone(),
            request(
                "/v1/vault/unlock",
                json!({"contractVersion":1,"workspaceId":workspace, forbidden: "secret"}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "INVALID_REQUEST");
    }

    // 3. Seal workspace via Desktop adapter on the shared storage root
    let storage = Storage::new(store_root.clone()).unwrap();
    let job = storage
        .enqueue_export(&workspace, ExportSelection::Backup { application: false })
        .unwrap();
    let lease = storage.claim_export().unwrap().unwrap();
    storage.execute_backup(&lease).unwrap();
    let complete = storage.read_job(&workspace, &job.id).unwrap();

    let desktop = AdapterVaultState::for_storage_root(store_root.clone()).unwrap();
    desktop
        .execute(
            Transport::Desktop,
            "desktop-main",
            None,
            "setup",
            json!({"contractVersion":1,"workspaceId":workspace}),
        )
        .unwrap();
    desktop
        .execute(
            Transport::Desktop,
            "desktop-main",
            None,
            "seal",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace,
                "jobId": job.id,
                "artifactId": complete.artifact_id.unwrap(),
                "sha256": complete.artifact_hash.unwrap()
            }),
        )
        .unwrap();

    // 4. Locked read from browser fails closed with 401 UNAUTHORIZED
    let (status, _) = call(
        state.clone(),
        request(
            "/v1/vault/read",
            json!({"contractVersion":1,"workspaceId":workspace}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 5. Paired browser unlocks via native provider and reads authenticated snapshot
    let (status, unlocked) = call(
        state.clone(),
        request(
            "/v1/vault/unlock",
            json!({"contractVersion":1,"workspaceId":workspace}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(unlocked["status"], "unlocked");
    assert!(unlocked.get("token").is_none());
    assert!(unlocked.get("dek").is_none());

    let (status, snapshot) = call(
        state.clone(),
        request(
            "/v1/vault/read",
            json!({"contractVersion":1,"workspaceId":workspace}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        snapshot["records"][0]["body"],
        "Encrypted payload read via paired browser session."
    );

    // 6. Encrypted backup, preview, encrypted export, and fresh restore via HTTP
    let (status, backup_res) = call(
        state.clone(),
        request(
            "/v1/vault/backup",
            json!({"contractVersion":1,"workspaceId":workspace}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let archive_id = backup_res["archiveId"].as_str().unwrap().to_owned();
    let archive_sha = backup_res["sha256"].as_str().unwrap().to_owned();

    let (status, preview_res) = call(
        state.clone(),
        request(
            "/v1/vault/preview",
            json!({"contractVersion":1,"workspaceId":workspace,"archiveId":archive_id}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview_res["authenticated"], false);
    assert_eq!(preview_res["sha256"], archive_sha);

    let (status, export_res) = call(
        state.clone(),
        request(
            "/v1/vault/export",
            json!({"contractVersion":1,"workspaceId":workspace}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let export_id = export_res["archiveId"].as_str().unwrap().to_owned();
    let export_sha = export_res["sha256"].as_str().unwrap().to_owned();

    // Wrong hash on restore fails closed
    let (status, _) = call(
        state.clone(),
        request(
            "/v1/vault/restore",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace,
                "archiveId": export_id,
                "name": "Rejected tampered restore",
                "encryptedExport": true,
                "expectedSha256": "0".repeat(64)
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Fresh restore of encrypted export creates a new sealed workspace
    let (status, restored) = call(
        state.clone(),
        request(
            "/v1/vault/restore",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace,
                "archiveId": export_id,
                "name": "Restored browser vault",
                "encryptedExport": true,
                "expectedSha256": export_sha
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let restored_id = restored["restoredWorkspaceId"].as_str().unwrap().to_owned();
    assert_ne!(restored_id, workspace);

    // 7. Lock revokes the paired browser session
    let (status, locked) = call(
        state.clone(),
        request(
            "/v1/vault/lock",
            json!({"contractVersion":1,"workspaceId":workspace}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(locked["status"], "locked");

    let (status, _) = call(
        state.clone(),
        request(
            "/v1/vault/read",
            json!({"contractVersion":1,"workspaceId":workspace}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 8. Re-unlock, then simulate API restart (new ApiState) and verify session is not persisted
    let (status, _) = call(
        state.clone(),
        request(
            "/v1/vault/unlock",
            json!({"contractVersion":1,"workspaceId":workspace}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let restarted = Arc::new(ApiState {
        core: Core::with_storage_root(store_root),
        token: "synthetic-restarted".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    });
    let restarted_req = Request::builder()
        .method("POST")
        .uri("/v1/vault/read")
        .header("host", "127.0.0.1:1451")
        .header("origin", "http://127.0.0.1:1420")
        .header("authorization", "Bearer synthetic-restarted")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"contractVersion":1,"workspaceId":workspace}).to_string(),
        ))
        .unwrap();
    let (status, _) = call(restarted, restarted_req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
