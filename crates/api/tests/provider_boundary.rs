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
use serde_json::{json, Value};
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

    // Session A configures the shared workspace backend and starts
    // a slow stream.
    let (status, _) = call(
        &a,
        "token-session-a",
        "providers-configure",
        json!({ "config": ollama_config(&fake) }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, started) = call(
        &a,
        "token-session-a",
        "providers-stream-start",
        json!({ "request": chat(), "mode": "auto" }),
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
        json!({ "runId": run_id, "capability": capability }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &b,
        "token-session-b",
        "providers-stream-cancel",
        json!({ "runId": run_id, "capability": capability }),
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
            "runId": run_id, "capability": capability,
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
            json!({ "runId": run_id, "capability": capability }).to_string(),
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
            "runId": run_id, "capability": capability,
            "callerContext": { "session": "forged" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}
