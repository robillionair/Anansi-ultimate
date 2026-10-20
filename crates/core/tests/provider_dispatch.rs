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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Fake Ollama (loopback, keyless) with a hit counter. Streams are
// chunked NDJSON; "slow" prompts dribble; "slow-unary" delays the
// unary response by 1.5s (for the lock-concurrency test).

struct FakeOllama {
    base_url: String,
    hits: Arc<AtomicUsize>,
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
    let release = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let release_thread = release.clone();
    let slow_in_flight = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let slow_thread = slow_in_flight.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let hits = hits_thread.clone();
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
        release,
        slow_in_flight,
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
