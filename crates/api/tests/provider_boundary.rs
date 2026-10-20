//! The provider authorization boundary over real HTTP, with TWO
//! authenticated sessions (two bearer tokens) sharing one
//! workspace: backend state is shared, but session binding is not.
//! The API stamps `callerContext` from the token — this test also
//! proves a client-forged stamp is overwritten, never honored.

use anansi_api::{ApiState, router};
use anansi_core::Core;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

// Minimal slow-streaming fake Ollama.
fn start_fake_ollama() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let lower = line.to_ascii_lowercase();
                    if let Some(v) = lower.strip_prefix("content-length:") {
                        content_length = v.trim().parse().unwrap_or(0);
                    }
                    if line == "\r\n" {
                        break;
                    }
                }
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                );
                let send = |stream: &mut std::net::TcpStream, data: &str| {
                    let _ = stream.write_all(format!("{:x}\r\n", data.len()).as_bytes());
                    let _ = stream.write_all(data.as_bytes());
                    let _ = stream.write_all(b"\r\n");
                    let _ = stream.flush();
                };
                for i in 0..40 {
                    send(
                        &mut stream,
                        &format!(
                            "{{\"model\":\"llama-x\",\"message\":{{\"role\":\"assistant\",\"content\":\"tok{i} \"}},\"done\":false}}\n"
                        ),
                    );
                    std::thread::sleep(Duration::from_millis(60));
                }
                send(
                    &mut stream,
                    "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true,\"prompt_eval_count\":3,\"eval_count\":40}\n",
                );
                let _ = stream.write_all(b"0\r\n\r\n");
            });
        }
    });
    format!("http://{addr}")
}

fn state(root: &std::path::Path, token: &str) -> Arc<ApiState> {
    Arc::new(ApiState {
        core: Core::with_storage_root(root.to_path_buf()),
        token: token.into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    })
}

async fn call(
    state: &Arc<ApiState>,
    token: &str,
    operation: &str,
    body: Value,
) -> (StatusCode, Value) {
    let app = router(state.clone());
    let req = Request::builder()
        .method("POST")
        .uri(format!("/v1/ai/{operation}"))
        .header("Host", "127.0.0.1:1451")
        .header("Origin", "http://127.0.0.1:1420")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

fn ollama_config(base_url: &str) -> Value {
    json!({
        "providers": [{
            "config": {
                "provider": "ollama", "baseUrl": base_url,
                "credential": null, "enabled": true
            },
            "locality": "local",
            "models": [{
                "id": "llama-x", "provider": "ollama",
                "capabilities": { "chat": true, "streaming": true, "vision": false,
                                  "coding": true, "deepReasoning": false, "embeddings": false },
                "contextWindow": 8192, "cost": "free", "speed": "balanced"
            }]
        }]
    })
}

fn chat() -> Value {
    json!({
        "model": "llama-x",
        "messages": [{
            "role": "user",
            "segments": [{ "text": "slow stream", "privacy": "standard" }]
        }]
    })
}

#[tokio::test]
async fn second_authenticated_session_cannot_touch_the_first_sessions_runs() {
    let dir = tempfile::tempdir().unwrap();
    let fake = start_fake_ollama();
    let a = state(dir.path(), "token-session-a");
    let b = state(dir.path(), "token-session-b");
    // A real logical workspace in the shared storage root, created
    // through the existing domain operation.
    let created = a
        .core
        .workspace_command("create", json!({ "contractVersion": 1, "name": "shared" }))
        .unwrap();
    let ws = created["workspace"]["id"].as_str().unwrap().to_string();

    // Before any owner authorization, session A's configure over
    // HTTP is refused: an authenticated browser session cannot set
    // provider policy on its own authority.
    let (status, body) = call(
        &a,
        "token-session-a",
        "providers-configure",
        json!({ "workspaceId": ws, "config": ollama_config(&fake) }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");

    // The host (this in-process call — the desktop transport)
    // grants session A owner authorization for this workspace.
    // The session label is the API's token fingerprint.
    let session_label = anansi_core::provider_dispatch::fingerprint("token-session-a");
    a.core
        .ai_command(
            "providers-grant-session",
            json!({ "workspaceId": ws, "session": session_label, "ttlSeconds": 600 }),
        )
        .unwrap();

    // Now A configures the shared workspace backend and starts a
    // slow stream.
    let (status, _) = call(
        &a,
        "token-session-a",
        "providers-configure",
        json!({ "workspaceId": ws, "config": ollama_config(&fake) }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, started) = call(
        &a,
        "token-session-a",
        "providers-stream-start",
        json!({ "workspaceId": ws, "request": chat(), "mode": "auto" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{started}");
    let run_id = started["runId"].as_str().unwrap().to_string();
    let capability = started["capability"].as_str().unwrap().to_string();

    // Session B — fully authenticated, same workspace — cannot poll
    // or cancel A's run, even presenting the genuine capability.
    let (status, _) = call(
        &b,
        "token-session-b",
        "providers-stream-poll",
        json!({ "workspaceId": ws, "runId": run_id, "capability": capability }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &b,
        "token-session-b",
        "providers-stream-cancel",
        json!({ "workspaceId": ws, "runId": run_id, "capability": capability }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A forged callerContext in B's body is overwritten by the API
    // stamp; it buys B nothing.
    let (status, _) = call(
        &b,
        "token-session-b",
        "providers-stream-poll",
        json!({
            "workspaceId": ws, "runId": run_id, "capability": capability,
            "callerContext": { "session": "host-session" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Without a token at all, the boundary refuses earlier still.
    let app = router(a.clone());
    let req = Request::builder()
        .method("POST")
        .uri("/v1/ai/providers-stream-poll")
        .header("Host", "127.0.0.1:1451")
        .header("Origin", "http://127.0.0.1:1420")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({ "workspaceId": ws, "runId": run_id, "capability": capability }).to_string(),
        ))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Session A still owns the run, and its own forged stamp is
    // equally ignored (overwritten with the real one): cancel works.
    let (status, _) = call(
        &a,
        "token-session-a",
        "providers-stream-cancel",
        json!({
            "workspaceId": ws, "runId": run_id, "capability": capability,
            "callerContext": { "session": "forged" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// The actual granted-browser workflow, over real HTTP: a browser
// session's refused configure attempt lands in the host's
// approval queue; the host (in-process, the desktop side)
// approves it there; the browser then configures successfully;
// revocation closes it again. No step lets the browser approve
// itself.

#[tokio::test]
async fn granted_browser_workflow_runs_through_the_host_approval_queue() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let token_a = "a".repeat(64);
    let api = state(&root, &token_a);
    // The desktop host on the same storage root.
    let host = Core::with_storage_root(root.clone());
    let ws_details = host
        .workspace_command(
            "create",
            json!({ "contractVersion": 1, "name": "browser-grant" }),
        )
        .unwrap();
    let ws = ws_details["workspace"]["id"].as_str().unwrap().to_string();
    let base = start_fake_ollama();

    // 1. The browser's configure attempt is refused...
    let (status, _) = call(
        &api,
        &token_a,
        "providers-configure",
        json!({ "contractVersion": 1, "workspaceId": ws, "config": ollama_config(&base) }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // 2. ...and queued for the host, which reads the queue and
    // learns the session's stamped identity from it.
    let access = host
        .ai_command("providers-session-access", json!({ "workspaceId": ws }))
        .unwrap();
    let pending = access["pending"].as_array().unwrap();
    assert_eq!(pending.len(), 1, "{access}");
    let session_label = pending[0]["session"].as_str().unwrap().to_string();
    assert_eq!(session_label.len(), 16, "{session_label}");
    assert!(session_label.chars().all(|c| c.is_ascii_hexdigit()), "{session_label}");

    // The browser cannot read the queue or approve itself.
    let (status, _) = call(
        &api,
        &token_a,
        "providers-session-access",
        json!({ "contractVersion": 1, "workspaceId": ws }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        &api,
        &token_a,
        "providers-grant-session",
        json!({
            "contractVersion": 1, "workspaceId": ws,
            "session": session_label, "ttlSeconds": 600
        }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // 3. The host approves from the queue: the pending entry
    // clears, the grant is listed, and the browser's retry works.
    host.ai_command(
        "providers-grant-session",
        json!({ "workspaceId": ws, "session": session_label, "ttlSeconds": 600 }),
    )
    .unwrap();
    let access = host
        .ai_command("providers-session-access", json!({ "workspaceId": ws }))
        .unwrap();
    assert!(access["pending"].as_array().unwrap().is_empty(), "{access}");
    assert_eq!(access["grants"][0]["session"], session_label);
    let (status, _) = call(
        &api,
        &token_a,
        "providers-configure",
        json!({ "contractVersion": 1, "workspaceId": ws, "config": ollama_config(&base) }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 4. Revocation closes it again immediately.
    host.ai_command(
        "providers-revoke-session",
        json!({ "workspaceId": ws, "session": session_label }),
    )
    .unwrap();
    let (status, _) = call(
        &api,
        &token_a,
        "providers-configure",
        json!({ "contractVersion": 1, "workspaceId": ws, "config": ollama_config(&base) }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
