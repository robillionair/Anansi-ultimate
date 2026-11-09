//! Adversarial tests for the provider authorization boundary,
//! driven through the actual application entry point
//! (`Core::ai_command`). Sessions are stamped exactly the way the
//! API layer stamps them from the authenticated token
//! (`callerContext`); unstamped calls are host (owner) calls.
//! Workspaces are real logical workspaces created through the
//! existing domain operation — several share one storage root.
//!
//! Recurring assertions: denied requests produce **zero** provider
//! traffic (the fake servers count their hits), and provider state
//! is isolated per logical workspace, not merely per root.

use anansi_core::{CommandError, Core, ErrorCode};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Fake Ollama (loopback, keyless) with a hit counter. Streams are
// chunked NDJSON; "slow" prompts dribble; "slow-unary" delays the
// unary response by 1.5s (for the lock-concurrency test).

struct FakeOllama {
    base_url: String,
    hits: Arc<AtomicUsize>,
    /// The model id each request named, in arrival order — the
    /// Ollama adapter binds the wire request to the selected
    /// candidate's model, so this proves WHICH model a dispatch
    /// actually reached, not just that traffic happened.
    models: Arc<Mutex<Vec<String>>>,
    /// Gate for "gated" streams: the server sends a few tokens,
    /// then holds the stream open until the test releases it — a
    /// stream that provably stays active until the test decides,
    /// on any platform's scheduler.
    release: Arc<std::sync::atomic::AtomicBool>,
    /// Set while a "slow-unary" request is being processed, so
    /// tests synchronize on the server actually being in flight
    /// instead of sleeping a guessed duration.
    slow_in_flight: Arc<std::sync::atomic::AtomicBool>,
}

fn start_fake_ollama() -> FakeOllama {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_thread = hits.clone();
    let models = Arc::new(Mutex::new(Vec::new()));
    let models_thread = models.clone();
    let release = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let release_thread = release.clone();
    let slow_in_flight = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let slow_thread = slow_in_flight.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let hits = hits_thread.clone();
            let models = models_thread.clone();
            let release = release_thread.clone();
            let slow_in_flight = slow_thread.clone();
            std::thread::spawn(move || {
                hits.fetch_add(1, Ordering::SeqCst);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
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
                let body = String::from_utf8_lossy(&body).to_string();
                if let Some(model) = requested_model(&body) {
                    models.lock().unwrap().push(model);
                }
                let streaming = body.contains("\"stream\":true");
                let slow = body.contains("slow");
                let gated = body.contains("gated");
                if streaming {
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
                        if gated && i == 2 {
                            // Hold the stream open until the test
                            // releases the gate (bounded so a dead
                            // test cannot hang the suite).
                            let gate_start = Instant::now();
                            while !release.load(Ordering::SeqCst)
                                && gate_start.elapsed() < Duration::from_secs(120)
                            {
                                std::thread::sleep(Duration::from_millis(5));
                            }
                        }
                        if slow {
                            std::thread::sleep(Duration::from_millis(60));
                        }
                    }
                    send(
                        &mut stream,
                        "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true,\"prompt_eval_count\":3,\"eval_count\":40}\n",
                    );
                    let _ = stream.write_all(b"0\r\n\r\n");
                } else {
                    if body.contains("slow-unary") {
                        slow_in_flight.store(true, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(1500));
                        slow_in_flight.store(false, Ordering::SeqCst);
                    }
                    let payload = "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"core reply\"},\"done\":true,\"prompt_eval_count\":4,\"eval_count\":2}";
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        payload.len(),
                        payload
                    );
                    let _ = stream.write_all(resp.as_bytes());
                }
            });
        }
    });
    FakeOllama {
        base_url: format!("http://{addr}"),
        hits,
        models,
        release,
        slow_in_flight,
    }
}

/// The `"model":"..."` field of an Ollama request body, if present.
fn requested_model(body: &str) -> Option<String> {
    let key = "\"model\":\"";
    let start = body.find(key)? + key.len();
    let end = body[start..].find('"')? + start;
    Some(body[start..end].to_string())
}

impl FakeOllama {
    fn models_seen(&self) -> Vec<String> {
        self.models.lock().unwrap().clone()
    }
}

// ---------------------------------------------------------------------------
// Helpers

fn create_workspace(core: &Core, name: &str) -> String {
    let details = core
        .workspace_command("create", json!({ "contractVersion": 1, "name": name }))
        .unwrap();
    details["workspace"]["id"].as_str().unwrap().to_string()
}

fn core() -> (tempfile::TempDir, Core, String) {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let ws = create_workspace(&core, "main");
    (dir, core, ws)
}

fn stamped(session_id: &str, mut body: Value) -> Value {
    body["callerContext"] = json!({ "session": session_id });
    body
}

fn ollama_config(base_url: &str, cost: &str) -> Value {
    json!({
        "providers": [{
            "config": {
                "provider": "ollama",
                "baseUrl": base_url,
                "credential": null,
                "enabled": true
            },
            "locality": "local",
            "models": [{
                "id": "llama-x",
                "provider": "ollama",
                "capabilities": { "chat": true, "streaming": true, "vision": false,
                                  "coding": true, "deepReasoning": false, "embeddings": false },
                "contextWindow": 8192,
                "cost": cost,
                "speed": "balanced"
            }]
        }]
    })
}

fn chat(text: &str) -> Value {
    json!({
        "model": "llama-x",
        "messages": [{
            "role": "user",
            "segments": [{ "text": text, "privacy": "standard" }]
        }]
    })
}

/// Host (owner) configure — unstamped.
fn configure(core: &Core, ws: &str, config: Value) -> Result<Value, CommandError> {
    core.ai_command(
        "providers-configure",
        json!({ "workspaceId": ws, "config": config }),
    )
}

fn authorize(core: &Core, ws: &str) -> String {
    let issued = core
        .ai_command(
            "providers-authorize",
            json!({
                "workspaceId": ws,
                "provider": "ollama", "model": "llama-x",
                "maxEstimatedTotalTokens": 100000
            }),
        )
        .unwrap();
    issued["permitId"].as_str().unwrap().to_string()
}

fn dispatch(
    core: &Core,
    session_id: &str,
    ws: &str,
    extra: Value,
    text: &str,
) -> Result<Value, CommandError> {
    let mut body = json!({ "workspaceId": ws, "request": chat(text), "mode": "auto" });
    if let (Some(dst), Some(src)) = (body.as_object_mut(), extra.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    core.ai_command("providers-dispatch", stamped(session_id, body))
}

fn usage(core: &Core, ws: &str) -> Value {
    core.ai_command("providers-usage", json!({ "workspaceId": ws }))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Happy path on backend state

#[test]
fn workflow_runs_on_backend_state_not_caller_json() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake.base_url, "free")).unwrap();

    let out = dispatch(&core, "s1", &ws, json!({}), "hello").unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    assert_eq!(out["disclosure"]["provider"], "ollama");
    assert_eq!(out["disclosure"]["local"], true);

    let started = core
        .ai_command(
            "providers-stream-start",
            stamped(
                "s1",
                json!({ "workspaceId": ws, "request": chat("stream please"), "mode": "auto" }),
            ),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let capability = started["capability"].as_str().unwrap().to_string();
    assert!(capability.len() >= 32);
    let mut text = String::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let poll = core
            .ai_command(
                "providers-stream-poll",
                stamped(
                    "s1",
                    json!({ "workspaceId": ws, "runId": run_id, "capability": capability }),
                ),
            )
            .unwrap();
        for event in poll["events"].as_array().unwrap() {
            if event["kind"] == "token" {
                text.push_str(event["text"].as_str().unwrap());
            }
        }
        if poll["done"].as_bool().unwrap() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(text.starts_with("tok0 tok1"));
    assert!(usage(&core, &ws)["totalRequests"].as_u64().unwrap() >= 2);
}

// ---------------------------------------------------------------------------
// Policy masquerades

#[test]
fn inline_config_in_dispatch_is_rejected_with_zero_traffic() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake.base_url, "free")).unwrap();
    let hits_before = fake.hits.load(Ordering::SeqCst);

    let err = core
        .ai_command(
            "providers-dispatch",
            stamped(
                "s1",
                json!({
                    "workspaceId": ws,
                    "config": ollama_config(&fake.base_url, "free"),
                    "request": chat("hello"),
                    "mode": "auto"
                }),
            ),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);

    let mut request = chat("hello");
    request["model"] = json!("gpt-cloud-premium");
    let err = core
        .ai_command(
            "providers-dispatch",
            stamped(
                "s1",
                json!({ "workspaceId": ws, "request": request, "mode": "auto" }),
            ),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(fake.hits.load(Ordering::SeqCst), hits_before);
}

#[test]
fn cloud_endpoint_cannot_masquerade_as_local_or_free() {
    let (_dir, core, ws) = core();
    let masquerade = json!({
        "providers": [{
            "config": {
                "provider": "openai",
                "baseUrl": "https://api.example.invalid/v1",
                "credential": null,
                "enabled": true
            },
            "locality": "local",
            "models": [{
                "id": "gpt-x", "provider": "openai",
                "capabilities": { "chat": true, "streaming": true, "vision": false,
                                  "coding": true, "deepReasoning": false, "embeddings": false },
                "contextWindow": 8192, "cost": "free", "speed": "balanced"
            }]
        }]
    });
    let err = configure(&core, &ws, masquerade).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert!(
        err.message.contains("locality is derived"),
        "must fail for the locality reason: {}",
        err.message
    );

    let fake = start_fake_ollama();
    let mut config = ollama_config(&fake.base_url, "free");
    config["providers"][0]["locality"] = json!("cloud");
    let err = configure(&core, &ws, config).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert!(
        err.message.contains("locality is derived"),
        "{}",
        err.message
    );
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);
}

// ---------------------------------------------------------------------------
// Permits

#[test]
fn permits_cannot_be_minted_borrowed_or_embedded() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake.base_url, "paid")).unwrap();

    let err = dispatch(&core, "session-a", &ws, json!({}), "hello").unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    let err = dispatch(
        &core,
        "session-a",
        &ws,
        json!({ "permitId": "permit-00000000000000000000000000000000" }),
        "hello",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    let err = dispatch(
        &core,
        "session-a",
        &ws,
        json!({ "permit": {
            "id": "self-minted", "provider": "ollama", "model": "llama-x",
            "maxEstimatedTotalTokens": 1000000, "allowUnknownCost": true
        } }),
        "hello",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);

    // Host issues a permit; permits are session-bound to the
    // session they were issued FOR — issuance names its session by
    // stamping the authorize call with that session after a grant.
    // (Host issuance for the host's own session is covered by the
    // A→B→A test; here the host grants session-a, which authorizes
    // for itself.)
    core.ai_command(
        "providers-grant-session",
        json!({ "workspaceId": ws, "session": "session-a", "ttlSeconds": 600 }),
    )
    .unwrap();
    let issued = core
        .ai_command(
            "providers-authorize",
            stamped(
                "session-a",
                json!({
                    "workspaceId": ws,
                    "provider": "ollama", "model": "llama-x",
                    "maxEstimatedTotalTokens": 100000
                }),
            ),
        )
        .unwrap();
    let permit_id = issued["permitId"].as_str().unwrap().to_string();

    let err = dispatch(
        &core,
        "session-b",
        &ws,
        json!({ "permitId": permit_id }),
        "hello",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);

    let out = dispatch(
        &core,
        "session-a",
        &ws,
        json!({ "permitId": permit_id }),
        "hello",
    )
    .unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    assert_eq!(fake.hits.load(Ordering::SeqCst), 1);
}

#[test]
fn consumed_permit_cannot_be_replayed_across_config_changes() {
    let (_dir, core, ws) = core();
    let fake_a = start_fake_ollama();
    let fake_b = start_fake_ollama();
    let config_a = ollama_config(&fake_a.base_url, "paid");
    configure(&core, &ws, config_a.clone()).unwrap();
    // Host-issued permit for the host session; dispatch unstamped
    // (host transport) so the session binding matches.
    let permit_id = authorize(&core, &ws);
    let out = core
        .ai_command(
            "providers-dispatch",
            json!({
                "workspaceId": ws, "request": chat("first"), "mode": "auto",
                "permitId": permit_id
            }),
        )
        .unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    assert_eq!(fake_a.hits.load(Ordering::SeqCst), 1);

    configure(&core, &ws, ollama_config(&fake_b.base_url, "paid")).unwrap();
    configure(&core, &ws, config_a).unwrap();
    let err = core
        .ai_command(
            "providers-dispatch",
            json!({
                "workspaceId": ws, "request": chat("replay"), "mode": "auto",
                "permitId": permit_id
            }),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(fake_a.hits.load(Ordering::SeqCst), 1);
    assert_eq!(usage(&core, &ws)["totalRequests"].as_u64().unwrap(), 1);
}

// ---------------------------------------------------------------------------
// Owner authorization on configure / authorize

#[test]
fn configure_and_authorize_require_owner_authorization() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    let config = ollama_config(&fake.base_url, "paid");

    // A stamped (browser) session cannot configure...
    let err = core
        .ai_command(
            "providers-configure",
            stamped("browser-1", json!({ "workspaceId": ws, "config": config })),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    // ...cannot authorize...
    let err = core
        .ai_command(
            "providers-authorize",
            stamped(
                "browser-1",
                json!({
                    "workspaceId": ws, "provider": "ollama", "model": "llama-x",
                    "maxEstimatedTotalTokens": 1000
                }),
            ),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    // ...and cannot grant itself authorization.
    let err = core
        .ai_command(
            "providers-grant-session",
            stamped(
                "browser-1",
                json!({ "workspaceId": ws, "session": "browser-1", "ttlSeconds": 600 }),
            ),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);

    // The host grants browser-1; now it can do both.
    core.ai_command(
        "providers-grant-session",
        json!({ "workspaceId": ws, "session": "browser-1", "ttlSeconds": 600 }),
    )
    .unwrap();
    core.ai_command(
        "providers-configure",
        stamped(
            "browser-1",
            json!({ "workspaceId": ws, "config": ollama_config(&fake.base_url, "paid") }),
        ),
    )
    .unwrap();
    core.ai_command(
        "providers-authorize",
        stamped(
            "browser-1",
            json!({
                "workspaceId": ws, "provider": "ollama", "model": "llama-x",
                "maxEstimatedTotalTokens": 1000
            }),
        ),
    )
    .unwrap();

    // Revocation takes it away again.
    core.ai_command(
        "providers-revoke-session",
        json!({ "workspaceId": ws, "session": "browser-1" }),
    )
    .unwrap();
    let err = core
        .ai_command(
            "providers-authorize",
            stamped(
                "browser-1",
                json!({
                    "workspaceId": ws, "provider": "ollama", "model": "llama-x",
                    "maxEstimatedTotalTokens": 1000
                }),
            ),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);
}

// ---------------------------------------------------------------------------
// Logical workspace isolation within one storage root

#[test]
fn two_workspaces_in_one_root_are_fully_isolated() {
    let (_dir, core, ws1) = core();
    let ws2 = create_workspace(&core, "second");
    let fake_a = start_fake_ollama();
    let fake_b = start_fake_ollama();
    configure(&core, &ws1, ollama_config(&fake_a.base_url, "free")).unwrap();
    configure(&core, &ws2, ollama_config(&fake_b.base_url, "paid")).unwrap();

    // Traffic and usage stay in their own workspace.
    dispatch(&core, "s1", &ws1, json!({}), "hello").unwrap();
    assert_eq!(fake_a.hits.load(Ordering::SeqCst), 1);
    assert_eq!(fake_b.hits.load(Ordering::SeqCst), 0);
    assert_eq!(usage(&core, &ws1)["totalRequests"].as_u64().unwrap(), 1);
    assert_eq!(usage(&core, &ws2)["totalRequests"].as_u64().unwrap(), 0);

    // A permit issued in ws2 does not exist in ws1.
    // (Host-issued in ws2; presented in ws1 — refused there.)
    let permit_ws2 = authorize(&core, &ws2);
    let err = core
        .ai_command(
            "providers-dispatch",
            json!({
                "workspaceId": ws1, "request": chat("hello"), "mode": "auto",
                "permitId": permit_ws2
            }),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(fake_a.hits.load(Ordering::SeqCst), 1);

    // A stream started in ws1 is invisible from ws2, even with the
    // genuine capability and the same session.
    let started = core
        .ai_command(
            "providers-stream-start",
            stamped(
                "s1",
                json!({ "workspaceId": ws1, "request": chat("stream please"), "mode": "auto" }),
            ),
        )
        .unwrap();
    let err = core
        .ai_command(
            "providers-stream-poll",
            stamped(
                "s1",
                json!({
                    "workspaceId": ws2,
                    "runId": started["runId"],
                    "capability": started["capability"]
                }),
            ),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);

    // An unknown workspace id is refused before any state access.
    let err = dispatch(&core, "s1", "ws-does-not-exist", json!({}), "hello").unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
    // fake_a saw exactly the ws1 dispatch + the ws1 stream start.
    assert_eq!(fake_a.hits.load(Ordering::SeqCst), 2);
    assert_eq!(fake_b.hits.load(Ordering::SeqCst), 0);
}

// ---------------------------------------------------------------------------
// Stream binding (capabilities + sessions)

#[test]
fn cross_session_stream_access_is_denied() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake.base_url, "free")).unwrap();
    let started = core
        .ai_command(
            "providers-stream-start",
            stamped(
                "owner",
                json!({ "workspaceId": ws, "request": chat("gated stream"), "mode": "auto" }),
            ),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let capability = started["capability"].as_str().unwrap().to_string();
    let poll = |session_id: &str, cap: &str| {
        core.ai_command(
            "providers-stream-poll",
            stamped(
                session_id,
                json!({ "workspaceId": ws, "runId": run_id, "capability": cap }),
            ),
        )
    };
    assert_eq!(
        poll("intruder", &capability).unwrap_err().code,
        ErrorCode::NotFound
    );
    assert_eq!(
        poll("owner", "cap-00000000000000000000000000000000")
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    let err = core
        .ai_command(
            "providers-stream-cancel",
            stamped(
                "intruder",
                json!({ "workspaceId": ws, "runId": run_id, "capability": capability }),
            ),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);

    std::thread::sleep(Duration::from_millis(200));
    let first = poll("owner", &capability).unwrap();
    assert!(
        first["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "token"),
        "stream survived the intruder's cancel attempt: {first}"
    );
    core.ai_command(
        "providers-stream-cancel",
        stamped(
            "owner",
            json!({ "workspaceId": ws, "runId": run_id, "capability": capability }),
        ),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut saw_cancelled = false;
    loop {
        let p = poll("owner", &capability).unwrap();
        for e in p["events"].as_array().unwrap() {
            if e["kind"] == "error" && e["code"] == "Cancelled" {
                saw_cancelled = true;
            }
        }
        if p["done"].as_bool().unwrap() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(saw_cancelled);
    assert_eq!(
        poll("owner", &capability).unwrap_err().code,
        ErrorCode::NotFound
    );
}

// ---------------------------------------------------------------------------
// Lock behaviour: a slow dispatch must not block the world

#[test]
fn slow_dispatch_does_not_block_poll_cancel_or_other_workspaces() {
    let (_dir, core, ws1) = core();
    let ws2 = create_workspace(&core, "other");
    let fake_slow = start_fake_ollama();
    let fake_fast = start_fake_ollama();
    configure(&core, &ws1, ollama_config(&fake_slow.base_url, "free")).unwrap();
    configure(&core, &ws2, ollama_config(&fake_fast.base_url, "free")).unwrap();
    let core = Arc::new(core);

    // A slow stream running in ws1 (owner session).
    let started = core
        .ai_command(
            "providers-stream-start",
            stamped(
                "owner",
                json!({ "workspaceId": ws1, "request": chat("gated stream"), "mode": "auto" }),
            ),
        )
        .unwrap();
    let run = json!({
        "workspaceId": ws1,
        "runId": started["runId"],
        "capability": started["capability"]
    });

    // A 1.5s unary dispatch in ws1 on another thread.
    let slow_core = core.clone();
    let slow_ws = ws1.clone();
    let slow = std::thread::spawn(move || {
        slow_core.ai_command(
            "providers-dispatch",
            stamped(
                "owner",
                json!({ "workspaceId": slow_ws, "request": chat("slow-unary"), "mode": "auto" }),
            ),
        )
    });
    // Synchronize on the server actually processing the slow
    // request — not on a guessed sleep duration.
    let wait_start = Instant::now();
    while !fake_slow.slow_in_flight.load(Ordering::SeqCst) {
        assert!(
            wait_start.elapsed() < Duration::from_secs(10),
            "slow dispatch never reached the server"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    // While it runs: polling the stream answers promptly...
    let t = Instant::now();
    let poll = core
        .ai_command("providers-stream-poll", stamped("owner", run.clone()))
        .unwrap();
    let poll_elapsed = t.elapsed();
    assert!(
        poll["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "token"),
        "poll during slow dispatch returns stream events: {poll}"
    );
    assert!(
        poll_elapsed < Duration::from_millis(800),
        "poll was blocked by the in-flight dispatch: {poll_elapsed:?}"
    );

    // ...another workspace dispatches promptly...
    let t = Instant::now();
    dispatch(&core, "owner", &ws2, json!({}), "quick").unwrap();
    assert!(
        t.elapsed() < Duration::from_millis(800),
        "other workspace blocked by the slow dispatch"
    );

    // ...and cancellation lands promptly and effectively.
    let t = Instant::now();
    core.ai_command("providers-stream-cancel", stamped("owner", run.clone()))
        .unwrap();
    assert!(t.elapsed() < Duration::from_millis(800), "cancel blocked");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_cancelled = false;
    loop {
        let p = core
            .ai_command("providers-stream-poll", stamped("owner", run.clone()))
            .unwrap();
        for e in p["events"].as_array().unwrap() {
            if e["kind"] == "error" && e["code"] == "Cancelled" {
                saw_cancelled = true;
            }
        }
        if p["done"].as_bool().unwrap() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(saw_cancelled);

    // The slow dispatch itself completes correctly.
    let out = slow.join().unwrap().unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    fake_slow.release.store(true, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// Budgets and bounds

#[test]
fn budget_comes_from_trusted_config_and_cannot_be_raised_by_the_caller() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    let mut config = ollama_config(&fake.base_url, "free");
    config["budget"] = json!({ "maxRequests": 1 });
    configure(&core, &ws, config).unwrap();
    dispatch(&core, "s1", &ws, json!({}), "one").unwrap();
    let err = dispatch(
        &core,
        "s1",
        &ws,
        json!({ "budget": { "maxRequests": 99 } }),
        "two",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    let err = dispatch(&core, "s1", &ws, json!({}), "two").unwrap_err();
    assert_eq!(err.code, ErrorCode::StorageUnavailable);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 1);
}

#[test]
fn strict_request_bounds_are_enforced_before_traffic() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake.base_url, "free")).unwrap();
    let many_messages = json!({
        "model": "llama-x",
        "messages": (0..300).map(|_| json!({
            "role": "user",
            "segments": [{ "text": "x", "privacy": "standard" }]
        })).collect::<Vec<_>>()
    });
    let err = core
        .ai_command(
            "providers-dispatch",
            stamped("s1", json!({ "workspaceId": ws, "request": many_messages })),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    let err = dispatch(&core, "s1", &ws, json!({}), &"x".repeat(40_000)).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);
}

#[test]
fn catalog_answers_without_configuration() {
    let (_dir, core, ws) = core();
    let catalog = core.ai_command("providers-catalog", json!({})).unwrap();
    assert!(catalog["providers"].as_array().unwrap().len() >= 20);
    let err = dispatch(&core, "s1", &ws, json!({}), "hello").unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
}

// ---------------------------------------------------------------------------
// Restart behavior: configuration persists; session state does not.
// The backend registry is process-global, so a "restart" inside
// one process proves nothing — these tests re-execute this test
// binary in child processes (one per phase) against a fake server
// that lives in the parent and counts hits across the boundary.

fn restart_child_phase() -> Option<(String, std::path::PathBuf)> {
    let phase = std::env::var("ANANSI_PROVIDER_RESTART_PHASE").ok()?;
    let root = std::path::PathBuf::from(std::env::var("ANANSI_PROVIDER_RESTART_ROOT").ok()?);
    Some((phase, root))
}

#[test]
fn configuration_persists_across_real_process_restarts() {
    if let Some((phase, root)) = restart_child_phase() {
        let core = Core::with_storage_root(root.clone());
        match phase.as_str() {
            "setup" => {
                let ws = create_workspace(&core, "main");
                let base = std::env::var("ANANSI_PROVIDER_RESTART_URL").unwrap();
                configure(&core, &ws, ollama_config(&base, "free")).unwrap();
                println!("RESTART_WS={ws}");
            }
            "serve" => {
                let ws = std::env::var("ANANSI_PROVIDER_RESTART_WS").unwrap();
                // No configure call: dispatch must work from the
                // persisted configuration alone.
                let out = dispatch(&core, "s1", &ws, json!({}), "after restart");
                match out {
                    Ok(v) => println!(
                        "RESTART_OK={}",
                        v["response"]["text"].as_str().unwrap_or_default()
                    ),
                    Err(e) => println!("RESTART_ERR={:?}", e.code),
                }
            }
            _ => panic!("unknown phase"),
        }
        return;
    }

    // Parent: orchestrate real child processes.
    let exe = std::env::current_exe().unwrap();
    let run_phase = |root: &std::path::Path, phase: &str, extra: &[(&str, String)]| -> String {
        let mut cmd = std::process::Command::new(&exe);
        cmd.args([
            "configuration_persists_across_real_process_restarts",
            "--exact",
            "--nocapture",
        ])
        .env("ANANSI_PROVIDER_RESTART_PHASE", phase)
        .env("ANANSI_PROVIDER_RESTART_ROOT", root);
        for (k, v) in extra {
            cmd.env(k, v);
        }
        let out = cmd.output().unwrap();
        assert!(
            out.status.success(),
            "child phase {phase} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).to_string()
    };

    // Happy path: setup in one process, serve in another.
    let dir = tempfile::tempdir().unwrap();
    let fake = start_fake_ollama();
    let stdout = run_phase(
        dir.path(),
        "setup",
        &[("ANANSI_PROVIDER_RESTART_URL", fake.base_url.clone())],
    );
    let ws = stdout
        .lines()
        .find_map(|l| l.strip_prefix("RESTART_WS="))
        .expect("child printed the workspace id")
        .trim()
        .to_string();
    let persisted = std::fs::read_to_string(
        dir.path()
            .join("provider-config")
            .join(format!("{ws}.json")),
    )
    .expect("configuration persisted next to the storage root");
    assert!(persisted.contains("llama-x"));
    assert!(!persisted.contains("sk-"), "no secret material persisted");
    let stdout = run_phase(
        dir.path(),
        "serve",
        &[("ANANSI_PROVIDER_RESTART_WS", ws.clone())],
    );
    assert!(stdout.contains("RESTART_OK=core reply"), "{stdout}");
    assert_eq!(fake.hits.load(Ordering::SeqCst), 1);

    // Tamper path: flipping the persisted locality (the configure
    // masquerade) must make the loading process refuse, with zero
    // provider traffic.
    let dir2 = tempfile::tempdir().unwrap();
    let stdout = run_phase(
        dir2.path(),
        "setup",
        &[("ANANSI_PROVIDER_RESTART_URL", fake.base_url.clone())],
    );
    let ws2 = stdout
        .lines()
        .find_map(|l| l.strip_prefix("RESTART_WS="))
        .unwrap()
        .trim()
        .to_string();
    let path = dir2
        .path()
        .join("provider-config")
        .join(format!("{ws2}.json"));
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("\"local\"", "\"cloud\"")).unwrap();
    let hits_before = fake.hits.load(Ordering::SeqCst);
    let stdout = run_phase(dir2.path(), "serve", &[("ANANSI_PROVIDER_RESTART_WS", ws2)]);
    assert!(stdout.contains("RESTART_ERR=InvalidRequest"), "{stdout}");
    assert_eq!(fake.hits.load(Ordering::SeqCst), hits_before);
}

// ---------------------------------------------------------------------------
// Persistence hardening: failure injection

#[test]
fn persistence_failure_leaves_memory_and_disk_consistent() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let core = Core::with_storage_root(root.clone());
    let ws = create_workspace(&core, "main");
    // Sabotage (after the storage root exists): "provider-config"
    // is a regular FILE, so the directory can never be created
    // and persistence must fail.
    std::fs::write(root.join("provider-config"), b"not a directory").unwrap();
    let fake = start_fake_ollama();
    let err = configure(&core, &ws, ollama_config(&fake.base_url, "free")).unwrap_err();
    assert_eq!(err.code, ErrorCode::StorageUnavailable, "{}", err.message);
    // Memory did NOT install a configuration it could not persist:
    // dispatch reports "not configured", never a phantom setup.
    let err = dispatch(&core, "s1", &ws, json!({}), "hello").unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert!(err.message.contains("not configured"), "{}", err.message);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);
}

#[cfg(unix)]
#[test]
fn persistence_refuses_a_symlinked_destination() {
    let (dir, core, ws) = core();
    let fake = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake.base_url, "free")).unwrap();
    let root = dir.path().to_path_buf();
    let dest = root.join("provider-config").join(format!("{ws}.json"));
    let target = root.join("elsewhere.json");
    std::fs::write(&target, b"SENTINEL").unwrap();
    std::fs::remove_file(&dest).unwrap();
    std::os::unix::fs::symlink(&target, &dest).unwrap();
    // Reconfiguring must refuse rather than write through (or
    // silently replace) the link, and the previous in-memory
    // configuration must survive untouched.
    let mut cfg2 = ollama_config(&fake.base_url, "free");
    cfg2["budget"] = json!({ "maxRequests": 7 });
    let err = configure(&core, &ws, cfg2).unwrap_err();
    assert_eq!(err.code, ErrorCode::StorageUnavailable, "{}", err.message);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "SENTINEL");
    assert!(dest.symlink_metadata().unwrap().file_type().is_symlink());
    // The first configuration is still the live one.
    let out = dispatch(&core, "s1", &ws, json!({}), "still configured").unwrap();
    assert_eq!(out["response"]["text"], "core reply");
}

#[test]
fn concurrent_configures_serialize_and_leave_no_temp_files() {
    let (dir, core, ws) = core();
    let fake = start_fake_ollama();
    let root = dir.path().to_path_buf();
    let core_a = Core::with_storage_root(root.clone());
    let core_b = Core::with_storage_root(root.clone());
    fn cfg(base: &str, budget: u64) -> Value {
        let mut c = ollama_config(base, "free");
        c["budget"] = json!({ "maxRequests": budget });
        c
    }
    let (ws_a, ws_b) = (ws.clone(), ws.clone());
    let base_a = fake.base_url.clone();
    let base_b = fake.base_url.clone();
    let a = std::thread::spawn(move || {
        for _ in 0..5 {
            configure(&core_a, &ws_a, cfg(&base_a, 11)).unwrap();
        }
    });
    let b = std::thread::spawn(move || {
        for _ in 0..5 {
            configure(&core_b, &ws_b, cfg(&base_b, 22)).unwrap();
        }
    });
    a.join().unwrap();
    b.join().unwrap();
    // The persisted file is exactly one of the two configurations
    // (persist + install are atomic under the backend lock, so
    // the file can never be a torn mix), and dispatch works.
    let text =
        std::fs::read_to_string(root.join("provider-config").join(format!("{ws}.json"))).unwrap();
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let budget = parsed["budget"]["maxRequests"].as_u64().unwrap();
    assert!(budget == 11 || budget == 22, "torn or stale config: {text}");
    let out = dispatch(&core, "s1", &ws, json!({}), "after the race").unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    // No temporary files are left behind, successful or not.
    let leftovers: Vec<_> = std::fs::read_dir(root.join("provider-config"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp-"))
        .collect();
    assert!(leftovers.is_empty(), "temp files leaked: {leftovers:?}");
}

// ---------------------------------------------------------------------------
// Credential lifecycle: native prompt -> keyring -> resolver -> wire

fn credential_env_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        // SAFETY: set once, inside Once::call_once, before any
        // credential prompt can read it; the value is constant
        // for the whole test process.
        unsafe {
            std::env::set_var("ANANSI_PROVIDER_TEST_CREDENTIAL", "sk-fake-test-credential");
        }
    });
}

struct FakeOpenAi {
    base_url: String,
    hits: Arc<AtomicUsize>,
    authorization: Arc<Mutex<Vec<String>>>,
}

fn start_fake_openai() -> FakeOpenAi {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(AtomicUsize::new(0));
    let authorization = Arc::new(Mutex::new(Vec::new()));
    let hits_thread = hits.clone();
    let auth_thread = authorization.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let hits = hits_thread.clone();
            let authorization = auth_thread.clone();
            std::thread::spawn(move || {
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                let mut content_length = 0usize;
                let header_end;
                loop {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                        header_end = pos + 4;
                        let text = String::from_utf8_lossy(&raw[..header_end]).to_string();
                        for line in text.lines() {
                            if let Some((k, v)) = line.split_once(':') {
                                let key = k.trim().to_ascii_lowercase();
                                if key == "content-length" {
                                    content_length = v.trim().parse().unwrap_or(0);
                                }
                                if key == "authorization" {
                                    authorization.lock().unwrap().push(v.trim().to_string());
                                }
                            }
                        }
                        break;
                    }
                }
                while raw.len() - header_end < content_length {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                }
                hits.fetch_add(1, Ordering::SeqCst);
                let body = "{\"id\":\"x\",\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"authed reply\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2}}";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            });
        }
    });
    FakeOpenAi {
        base_url: format!("http://127.0.0.1:{port}"),
        hits,
        authorization,
    }
}

fn credential_op(
    core: &Core,
    session: Option<&str>,
    op: &str,
    ws: &str,
    reference: &str,
) -> Result<Value, CommandError> {
    let mut body = json!({
        "workspaceId": ws, "provider": "openai", "reference": reference
    });
    if let Some(s) = session {
        body["callerContext"] = json!({ "session": s });
    }
    core.ai_command(op, body)
}

#[test]
fn credential_lifecycle_prompt_keyring_resolver_wire() {
    let _lock = credential_prompt_lock();
    credential_env_once();
    let (_dir, core, ws) = core();
    let reference = format!("openai-e2e-{}", std::process::id());
    // A browser session cannot set credentials, and the refusal
    // stores nothing (the prompt is never even reached).
    let err = credential_op(
        &core,
        Some("browser-1"),
        "providers-credential-set",
        &ws,
        &reference,
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    let status =
        credential_op(&core, None, "providers-credential-status", &ws, &reference).unwrap();
    assert_eq!(status["present"], false);
    // The host sets it through the (test) native prompt: the key
    // moves prompt -> keyring only.
    let set = credential_op(&core, None, "providers-credential-set", &ws, &reference).unwrap();
    assert_eq!(set["stored"], true);
    let status =
        credential_op(&core, None, "providers-credential-status", &ws, &reference).unwrap();
    assert_eq!(status["present"], true);

    // Configure an OpenAI-shaped custom endpoint using only the
    // reference; dispatch must resolve the key from the keyring
    // and put it on the wire.
    let fake = start_fake_openai();
    let config = json!({
        "providers": [{
            "config": {
                "provider": "openai", "baseUrl": fake.base_url,
                "credential": { "provider": "openai", "reference": reference },
                "enabled": true, "customEndpoint": true
            },
            "locality": "local",
            "models": [{
                "id": "gpt-x", "provider": "openai",
                "capabilities": { "chat": true, "streaming": true, "vision": false, "coding": true, "deepReasoning": false, "embeddings": false },
                "contextWindow": 8192, "cost": "free", "speed": "balanced"
            }]
        }]
    });
    configure(&core, &ws, config).unwrap();
    let dispatch_gpt = |core: &Core| {
        core.ai_command(
            "providers-dispatch",
            stamped(
                "browser-1",
                json!({
                    "workspaceId": ws, "mode": "auto",
                    "request": {
                        "model": "gpt-x",
                        "messages": [{
                            "role": "user",
                            "segments": [{ "text": "hello", "privacy": "standard" }]
                        }]
                    }
                }),
            ),
        )
    };
    let out = dispatch_gpt(&core).unwrap();
    assert_eq!(out["response"]["text"], "authed reply");
    assert_eq!(
        fake.authorization.lock().unwrap().as_slice(),
        ["Bearer sk-fake-test-credential"],
        "the key from the native prompt reached the wire via the keyring resolver"
    );

    // Removal is real: status flips and dispatch now fails closed
    // with zero additional provider traffic.
    let removed =
        credential_op(&core, None, "providers-credential-remove", &ws, &reference).unwrap();
    assert_eq!(removed["removed"], true);
    let status =
        credential_op(&core, None, "providers-credential-status", &ws, &reference).unwrap();
    assert_eq!(status["present"], false);
    let hits = fake.hits.load(Ordering::SeqCst);
    let err = dispatch_gpt(&core).unwrap_err();
    assert_eq!(err.code, ErrorCode::Unauthorized, "{}", err.message);
    assert_eq!(fake.hits.load(Ordering::SeqCst), hits);
}

// ---------------------------------------------------------------------------
// Session access: the host approval queue

fn session_access(core: &Core, session: Option<&str>, ws: &str) -> Result<Value, CommandError> {
    let mut body = json!({ "workspaceId": ws });
    if let Some(s) = session {
        body["callerContext"] = json!({ "session": s });
    }
    core.ai_command("providers-session-access", body)
}

#[test]
fn refused_browser_attempts_queue_for_host_approval() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    let cfg = ollama_config(&fake.base_url, "free");
    // Two refused attempts by the same browser session...
    for _ in 0..2 {
        let mut body = json!({ "workspaceId": ws, "config": cfg });
        body["callerContext"] = json!({ "session": "sess-hopeful" });
        let err = core.ai_command("providers-configure", body).unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }
    // ...are visible to the host, aggregated with an attempt count.
    let access = session_access(&core, None, &ws).unwrap();
    let pending = access["pending"].as_array().unwrap();
    assert_eq!(pending.len(), 1, "{access}");
    assert_eq!(pending[0]["session"], "sess-hopeful");
    assert_eq!(pending[0]["attempts"], 2);
    // The queue itself is host-only.
    let err = session_access(&core, Some("sess-hopeful"), &ws).unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    // Granting clears the queue entry and opens the door.
    core.ai_command(
        "providers-grant-session",
        json!({ "workspaceId": ws, "session": "sess-hopeful", "ttlSeconds": 600 }),
    )
    .unwrap();
    let access = session_access(&core, None, &ws).unwrap();
    assert!(access["pending"].as_array().unwrap().is_empty(), "{access}");
    assert_eq!(access["grants"][0]["session"], "sess-hopeful");
    let mut body = json!({ "workspaceId": ws, "config": cfg });
    body["callerContext"] = json!({ "session": "sess-hopeful" });
    core.ai_command("providers-configure", body).unwrap();
    // Revocation closes it again.
    core.ai_command(
        "providers-revoke-session",
        json!({ "workspaceId": ws, "session": "sess-hopeful" }),
    )
    .unwrap();
    let mut body = json!({ "workspaceId": ws, "config": ollama_config(&fake.base_url, "free") });
    body["callerContext"] = json!({ "session": "sess-hopeful" });
    let err = core.ai_command("providers-configure", body).unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
}

// ---------------------------------------------------------------------------
// Expected-target contract
// (docs/handoffs/expected-target-contract-repair-design.md):
// an expected target binds a dispatch only when the caller pins
// chooseModel + chosen; on stream-start the field is refused outright.

/// One ollama provider with two free models. Auto mode orders by
/// provider id then model id, so `aaa-decoy` is first in the chain.
fn two_model_ollama_config(base_url: &str) -> Value {
    let mut config = ollama_config(base_url, "free");
    config["providers"][0]["models"] = json!([
        { "id": "aaa-decoy", "provider": "ollama",
          "capabilities": { "chat": true, "streaming": true, "vision": false,
                            "coding": true, "deepReasoning": false, "embeddings": false },
          "contextWindow": 8192, "cost": "free", "speed": "balanced" },
        { "id": "llama-x", "provider": "ollama",
          "capabilities": { "chat": true, "streaming": true, "vision": false,
                            "coding": true, "deepReasoning": false, "embeddings": false },
          "contextWindow": 8192, "cost": "free", "speed": "balanced" },
    ]);
    config
}

fn expected_target(base_url: &str) -> Value {
    json!({
        "provider": "ollama", "model": "llama-x",
        "endpoint": base_url, "locality": "local", "cost": "free"
    })
}

fn stream_start(core: &Core, ws: &str, extra: Value) -> Result<Value, CommandError> {
    let mut body = json!({ "workspaceId": ws, "request": chat("stream please"), "mode": "auto" });
    if let (Some(dst), Some(src)) = (body.as_object_mut(), extra.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    core.ai_command("providers-stream-start", stamped("s1", body))
}

#[test]
fn expected_target_with_auto_routing_is_refused_before_traffic() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    configure(&core, &ws, two_model_ollama_config(&fake.base_url)).unwrap();

    let result = dispatch(
        &core,
        "s1",
        &ws,
        json!({ "expectedTarget": expected_target(&fake.base_url) }),
        "hello",
    );
    // On unfixed code the dispatch succeeds — and the fake proves it
    // went to the decoy, not the expected target.
    assert!(
        fake.models_seen().is_empty(),
        "auto routing must not dispatch under an expected target; models dispatched: {:?}",
        fake.models_seen()
    );
    let err = result.unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);
}

#[test]
fn expected_target_with_auto_routing_and_chosen_is_refused_before_traffic() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    configure(&core, &ws, two_model_ollama_config(&fake.base_url)).unwrap();

    let result = dispatch(
        &core,
        "s1",
        &ws,
        json!({
            "chosen": ["ollama", "llama-x"],
            "expectedTarget": expected_target(&fake.base_url),
        }),
        "hello",
    );
    // `chosen` is ignored in auto mode, so its presence cannot rescue
    // automatic routing — on unfixed code this reaches the decoy.
    assert!(
        fake.models_seen().is_empty(),
        "chosen is ignored in auto mode; models dispatched: {:?}",
        fake.models_seen()
    );
    let err = result.unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);
}

#[test]
fn expected_target_with_choose_model_dispatches_to_the_exact_target() {
    let (_dir, core, ws) = core();
    let fake = start_fake_ollama();
    configure(&core, &ws, two_model_ollama_config(&fake.base_url)).unwrap();

    let out = dispatch(
        &core,
        "s1",
        &ws,
        json!({
            "mode": "chooseModel",
            "chosen": ["ollama", "llama-x"],
            "expectedTarget": expected_target(&fake.base_url),
        }),
        "hello",
    )
    .unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    assert_eq!(out["disclosure"]["model"], "llama-x");
    assert_eq!(fake.models_seen(), vec!["llama-x".to_string()]);

    // A mismatched chosen is refused before traffic.
    let hits_before = fake.hits.load(Ordering::SeqCst);
    let err = dispatch(
        &core,
        "s1",
        &ws,
        json!({
            "mode": "chooseModel",
            "chosen": ["ollama", "aaa-decoy"],
            "expectedTarget": expected_target(&fake.base_url),
        }),
        "hello",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(fake.hits.load(Ordering::SeqCst), hits_before);
    assert_eq!(fake.models_seen(), vec!["llama-x".to_string()]);
}

#[test]
fn stream_start_refuses_expected_target_before_traffic() {
    let (_dir, core, ws) = core();
    let fake_b = start_fake_ollama();
    let fake_a = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake_b.base_url, "free")).unwrap();

    // Mismatched expected endpoint: on unfixed code the field is
    // ignored, the stream starts, and fake B gets the traffic.
    let result = stream_start(
        &core,
        &ws,
        json!({ "expectedTarget": expected_target(&fake_a.base_url) }),
    );
    assert_eq!(
        fake_b.hits.load(Ordering::SeqCst),
        0,
        "stream-start must not dispatch under an expected target"
    );
    let err = result.unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(fake_a.hits.load(Ordering::SeqCst), 0);

    // Even a matching expected target is refused: stream-start does
    // not enforce the binding, so accepting it would be a lie.
    let err = stream_start(
        &core,
        &ws,
        json!({ "expectedTarget": expected_target(&fake_b.base_url) }),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(fake_b.hits.load(Ordering::SeqCst), 0);

    // Positive control: a stream without the field still starts.
    let started = stream_start(&core, &ws, json!({})).unwrap();
    assert!(started["runId"].as_str().is_some());
    assert!(fake_b.hits.load(Ordering::SeqCst) >= 1);
}

// ---------------------------------------------------------------------------
// Connect AI: atomic add/update, host-generated references, status
// (docs/handoffs/connect-ai-api-key-setup-design.md, AC-C3..C8)

/// Serializes the tests that drive the native-prompt test hook:
/// the hook reads process-global environment, so two such tests
/// must never run concurrently.
fn credential_prompt_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

fn configure_add(core: &Core, ws: &str, provider: Value) -> Result<Value, CommandError> {
    core.ai_command(
        "providers-configure-add",
        json!({ "workspaceId": ws, "provider": provider }),
    )
}

fn provider_status(core: &Core, session: Option<&str>, ws: &str) -> Result<Value, CommandError> {
    let mut body = json!({ "workspaceId": ws });
    if let Some(s) = session {
        body["callerContext"] = json!({ "session": s });
    }
    core.ai_command("providers-status", body)
}

fn credential_reference(
    core: &Core,
    session: Option<&str>,
    ws: &str,
    provider: &str,
) -> Result<Value, CommandError> {
    let mut body = json!({ "workspaceId": ws, "provider": provider });
    if let Some(s) = session {
        body["callerContext"] = json!({ "session": s });
    }
    core.ai_command("providers-credential-reference", body)
}

/// One provider entry: `provider` id, `base_url`, optional
/// credential reference, one model with the given cost class.
fn provider_entry(provider: &str, base_url: &str, reference: Option<&str>, model: &str, cost: &str) -> Value {
    json!({
        "config": {
            "provider": provider, "baseUrl": base_url,
            "credential": reference.map(|r| json!({ "provider": provider, "reference": r })),
            "enabled": true, "customEndpoint": provider != "ollama"
        },
        "locality": "local",
        "models": [{
            "id": model, "provider": provider,
            "capabilities": { "chat": true, "streaming": true, "vision": false,
                              "coding": true, "deepReasoning": false, "embeddings": false },
            "contextWindow": 8192, "cost": cost, "speed": "balanced"
        }]
    })
}

fn status_entry<'a>(status: &'a Value, provider: &str) -> &'a Value {
    status["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["provider"] == provider)
        .unwrap_or_else(|| panic!("provider {provider} missing from status: {status}"))
}

#[test]
fn configure_add_preserves_providers_budget_and_accounting() {
    let _lock = credential_prompt_lock();
    credential_env_once();
    let (_dir, core, ws) = core();
    let fake_a = start_fake_ollama();
    let fake_b = start_fake_openai();

    // Provider A with a budget, via the full-replacement op.
    let mut config_a = ollama_config(&fake_a.base_url, "free");
    config_a["budget"] = json!({ "maxRequests": 5, "maxTotalTokens": 100000 });
    configure(&core, &ws, config_a).unwrap();
    assert_eq!(fake_a.hits.load(Ordering::SeqCst), 0, "configure makes no provider calls");

    // One successful dispatch: usage accounting now records A.
    let out = dispatch(&core, "browser-1", &ws, json!({}), "hello").unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    assert_eq!(usage(&core, &ws)["totalRequests"], 1);

    // Host-generated reference for B, then the native prompt
    // (test hook) stores the key under it.
    let reference = credential_reference(&core, None, &ws, "openai").unwrap();
    let reference = reference["reference"].as_str().unwrap().to_string();
    assert!(reference.starts_with("openai-"), "{reference}");
    assert_eq!(reference.len(), "openai-".len() + 16);
    let set = credential_op(&core, None, "providers-credential-set", &ws, &reference).unwrap();
    assert_eq!(set["stored"], true);

    // Add B: the atomic op must keep A, the budget, and usage.
    let added = configure_add(
        &core,
        &ws,
        provider_entry("openai", &fake_b.base_url, Some(&reference), "gpt-x", "unknown"),
    )
    .unwrap();
    assert_eq!(added["added"], "openai");
    assert_eq!(added["updated"], false);
    assert_eq!(added["providers"], json!(["ollama", "openai"]));

    // A browser may read status: both providers, budget intact,
    // credential stored vs connection verified kept apart.
    let status = provider_status(&core, Some("browser-1"), &ws).unwrap();
    assert_eq!(status["budget"]["maxRequests"], 5);
    assert_eq!(status["budget"]["maxTotalTokens"], 100000);
    let a = status_entry(&status, "ollama");
    assert_eq!(a["credentialStored"], false);
    assert_eq!(a["connectionVerified"], true, "A dispatched once");
    let b = status_entry(&status, "openai");
    assert_eq!(b["credentialStored"], true);
    assert_eq!(b["connectionVerified"], false, "B never dispatched");
    assert_eq!(usage(&core, &ws)["totalRequests"], 1, "usage accounting survived the add");

    // Setup made zero inference calls: the only hit is the one
    // dispatch above; B's fake server was never contacted.
    assert_eq!(fake_a.hits.load(Ordering::SeqCst), 1);
    assert_eq!(fake_b.hits.load(Ordering::SeqCst), 0, "setup must not call any provider");

    // B's model is cost-unknown: under Zero-Cost Lock it is
    // refused without a permit, with zero traffic.
    let err = core
        .ai_command(
            "providers-dispatch",
            stamped(
                "browser-1",
                json!({
                    "workspaceId": ws, "mode": "chooseModel", "zeroCostLock": true,
                    "chosen": ["openai", "gpt-x"],
                    "request": {
                        "model": "gpt-x",
                        "messages": [{ "role": "user",
                                       "segments": [{ "text": "hello", "privacy": "standard" }] }]
                    }
                }),
            ),
        )
        .unwrap_err();
    // The router refuses an unknown-cost candidate under the
    // lock as Unsupported (mapped to InvalidRequest) — before
    // any traffic.
    assert_eq!(err.code, ErrorCode::InvalidRequest, "{}", err.message);
    assert!(err.message.contains("cost filters"), "{}", err.message);
    assert_eq!(fake_b.hits.load(Ordering::SeqCst), 0);

    // Updating B in place keeps the provider list and budget.
    let updated = configure_add(
        &core,
        &ws,
        provider_entry("openai", &fake_b.base_url, Some(&reference), "gpt-y", "unknown"),
    )
    .unwrap();
    assert_eq!(updated["updated"], true);
    assert_eq!(updated["providers"], json!(["ollama", "openai"]));
    let status = provider_status(&core, None, &ws).unwrap();
    assert_eq!(status["budget"]["maxRequests"], 5);
    assert_eq!(status_entry(&status, "openai")["models"][0]["id"], "gpt-y");
}

#[test]
fn configure_add_survives_restart() {
    let _lock = credential_prompt_lock();
    credential_env_once();
    let (dir, core, ws) = core();
    let fake_a = start_fake_ollama();
    let fake_b = start_fake_openai();

    let mut config_a = ollama_config(&fake_a.base_url, "free");
    config_a["budget"] = json!({ "maxRequests": 5, "maxTotalTokens": 100000 });
    configure(&core, &ws, config_a).unwrap();
    let reference = credential_reference(&core, None, &ws, "openai").unwrap()["reference"]
        .as_str()
        .unwrap()
        .to_string();
    credential_op(&core, None, "providers-credential-set", &ws, &reference).unwrap();
    configure_add(
        &core,
        &ws,
        provider_entry("openai", &fake_b.base_url, Some(&reference), "gpt-x", "unknown"),
    )
    .unwrap();
    // The merged configuration is on disk, references only.
    let file = dir
        .path()
        .join("provider-config")
        .join(format!("{ws}.json"));
    assert!(file.exists(), "config file missing");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(!text.contains("sk-fake"), "config file must never hold a secret");
    assert!(text.contains(&reference), "config file holds the reference");

    // Restart: a fresh Core over the same storage root.
    drop(core);
    let core2 = Core::with_storage_root(dir.path().to_path_buf());
    // The first dispatch lazily installs the persisted config.
    let out = dispatch(&core2, "browser-1", &ws, json!({}), "hello").unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    let status = provider_status(&core2, None, &ws).unwrap();
    assert_eq!(status["providers"].as_array().unwrap().len(), 2);
    assert_eq!(status["budget"]["maxRequests"], 5);
    assert_eq!(status["budget"]["maxTotalTokens"], 100000);
    assert_eq!(status_entry(&status, "openai")["credentialStored"], true);
}

#[test]
fn configure_add_is_atomic_on_persist_failure() {
    let (_dir, core, ws) = core();
    let fake_a = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake_a.base_url, "free")).unwrap();

    // Break persistence: replace the provider-config directory
    // with a regular file, so create_dir_all fails.
    let root = _dir.path();
    std::fs::remove_dir_all(root.join("provider-config")).unwrap();
    std::fs::write(root.join("provider-config"), b"not a directory").unwrap();

    let err = configure_add(
        &core,
        &ws,
        provider_entry("openai", "http://127.0.0.1:9", None, "gpt-x", "unknown"),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::StorageUnavailable, "{}", err.message);
    // Memory is exactly as before: only A, still usable.
    let status = provider_status(&core, None, &ws).unwrap();
    assert_eq!(status["providers"].as_array().unwrap().len(), 1);
    assert_eq!(status_entry(&status, "ollama")["provider"], "ollama");
    let out = dispatch(&core, "browser-1", &ws, json!({}), "hello").unwrap();
    assert_eq!(out["response"]["text"], "core reply");

    // Clean retry once persistence works again.
    std::fs::remove_file(root.join("provider-config")).unwrap();
    let added = configure_add(
        &core,
        &ws,
        provider_entry("openai", "http://127.0.0.1:9", None, "gpt-x", "unknown"),
    )
    .unwrap();
    assert_eq!(added["providers"], json!(["ollama", "openai"]));
}

#[test]
fn browser_refusal_leaves_state_unchanged() {
    let _lock = credential_prompt_lock();
    credential_env_once();
    let (_dir, core, ws) = core();
    let fake_a = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake_a.base_url, "free")).unwrap();

    for (op, body) in [
        (
            "providers-configure-add",
            json!({ "workspaceId": ws, "provider": provider_entry("openai", "http://127.0.0.1:9", None, "gpt-x", "unknown") }),
        ),
        (
            "providers-credential-reference",
            json!({ "workspaceId": ws, "provider": "openai" }),
        ),
        (
            "providers-credential-set",
            json!({ "workspaceId": ws, "provider": "openai", "reference": "openai-0000000000000000" }),
        ),
    ] {
        let mut stamped_body = body.clone();
        stamped_body["callerContext"] = json!({ "session": "browser-1" });
        let err = core.ai_command(op, stamped_body).unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden, "{op}: {}", err.message);
    }
    // Nothing changed; the refusal is queued for host approval.
    let status = provider_status(&core, Some("browser-1"), &ws).unwrap();
    assert_eq!(status["providers"].as_array().unwrap().len(), 1);
    let access = session_access(&core, None, &ws).unwrap();
    assert!(
        access["pending"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["session"] == "browser-1"),
        "{access}"
    );
}

#[test]
fn cancelled_prompt_stores_nothing_and_leaves_config_unchanged() {
    let _lock = credential_prompt_lock();
    credential_env_once();
    let (_dir, core, ws) = core();
    let fake_a = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake_a.base_url, "free")).unwrap();
    let reference = credential_reference(&core, None, &ws, "openai").unwrap()["reference"]
        .as_str()
        .unwrap()
        .to_string();

    // Cancel the native prompt (test hook): nothing is stored,
    // the configuration is untouched.
    unsafe { std::env::set_var("ANANSI_PROVIDER_TEST_CANCEL", "1") };
    let err = credential_op(&core, None, "providers-credential-set", &ws, &reference).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest, "{}", err.message);
    assert!(err.message.contains("cancelled"), "{}", err.message);
    unsafe { std::env::remove_var("ANANSI_PROVIDER_TEST_CANCEL") };
    let status =
        credential_op(&core, None, "providers-credential-status", &ws, &reference).unwrap();
    assert_eq!(status["present"], false);
    let status = provider_status(&core, None, &ws).unwrap();
    assert_eq!(status["providers"].as_array().unwrap().len(), 1);

    // Clean retry with the prompt working again.
    let set = credential_op(&core, None, "providers-credential-set", &ws, &reference).unwrap();
    assert_eq!(set["stored"], true);
    let status =
        credential_op(&core, None, "providers-credential-status", &ws, &reference).unwrap();
    assert_eq!(status["present"], true);
}

#[test]
fn status_is_readable_by_a_browser_and_shows_no_secrets() {
    let (_dir, core, ws) = core();
    let fake_a = start_fake_ollama();
    configure(&core, &ws, ollama_config(&fake_a.base_url, "free")).unwrap();
    let status = provider_status(&core, Some("browser-1"), &ws).unwrap();
    assert_eq!(status["providers"][0]["provider"], "ollama");
    assert_eq!(status["providers"][0]["connectionVerified"], false);
    let text = serde_json::to_string(&status).unwrap();
    assert!(!text.contains("sk-"), "status must never carry secret material");
}
