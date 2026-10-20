//! Adversarial tests for the provider authorization boundary,
//! driven through the actual application entry point
//! (`Core::ai_command`). The session stamp is injected exactly the
//! way the API layer stamps it from the authenticated token
//! (`callerContext`); the workspace comes from each Core's own
//! storage root, so every test gets an isolated backend.
//!
//! Recurring assertion: a denied request must produce **zero**
//! provider traffic (the fake servers count their hits).

use anansi_core::{Core, ErrorCode};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Fake Ollama (loopback, keyless) with a hit counter. Streams are
// chunked NDJSON; "slow" prompts dribble.

struct FakeOllama {
    base_url: String,
    hits: Arc<AtomicUsize>,
}

fn start_fake_ollama() -> FakeOllama {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_thread = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let hits = hits_thread.clone();
            std::thread::spawn(move || {
                hits.fetch_add(1, Ordering::SeqCst);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
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
    }
}

// ---------------------------------------------------------------------------
// Helpers

fn core() -> (tempfile::TempDir, Core) {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    (dir, core)
}

fn session(session: &str, mut body: Value) -> Value {
    body["callerContext"] = json!({ "session": session });
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

fn configure(core: &Core, session_id: &str, config: Value) -> Result<Value, anansi_core::CommandError> {
    core.ai_command(
        "providers-configure",
        session(session_id, json!({ "config": config })),
    )
}

fn dispatch(
    core: &Core,
    session_id: &str,
    extra: Value,
    text: &str,
) -> Result<Value, anansi_core::CommandError> {
    let mut body = json!({ "request": chat(text), "mode": "auto" });
    if let (Some(dst), Some(src)) = (body.as_object_mut(), extra.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    core.ai_command("providers-dispatch", session(session_id, body))
}

// ---------------------------------------------------------------------------
// Tests

#[test]
fn workflow_runs_on_backend_state_not_caller_json() {
    let (_dir, core) = core();
    let fake = start_fake_ollama();
    configure(&core, "s1", ollama_config(&fake.base_url, "free")).unwrap();

    // Dispatch carries no configuration at all.
    let out = dispatch(&core, "s1", json!({}), "hello").unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    assert_eq!(out["disclosure"]["provider"], "ollama");
    assert_eq!(out["disclosure"]["local"], true);

    // Streamed dispatch with the issued capability.
    let started = core
        .ai_command(
            "providers-stream-start",
            session(
                "s1",
                json!({ "request": chat("stream please"), "mode": "auto" }),
            ),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let capability = started["capability"].as_str().unwrap().to_string();
    assert!(capability.len() >= 32, "capability is unpredictable-length");
    let mut text = String::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let poll = core
            .ai_command(
                "providers-stream-poll",
                session(
                    "s1",
                    json!({ "runId": run_id, "capability": capability }),
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

    let usage = core
        .ai_command("providers-usage", session("s1", json!({})))
        .unwrap();
    assert!(usage["totalRequests"].as_u64().unwrap() >= 2);
}

#[test]
fn inline_config_in_dispatch_is_rejected_with_zero_traffic() {
    let (_dir, core) = core();
    let fake = start_fake_ollama();
    configure(&core, "s1", ollama_config(&fake.base_url, "free")).unwrap();
    let hits_before = fake.hits.load(Ordering::SeqCst);

    // The r3 request shape — configuration inside the dispatch —
    // must now fail closed (deny_unknown_fields), even though the
    // embedded configuration is the caller's own "free local" one.
    let err = core
        .ai_command(
            "providers-dispatch",
            session(
                "s1",
                json!({
                    "config": ollama_config(&fake.base_url, "free"),
                    "request": chat("hello"),
                    "mode": "auto"
                }),
            ),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);

    // A model the trusted configuration does not contain gets no
    // route, even though the caller asks for it by name.
    let mut request = chat("hello");
    request["model"] = json!("gpt-cloud-premium");
    let err = core
        .ai_command(
            "providers-dispatch",
            session("s1", json!({ "request": request, "mode": "auto" })),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(
        fake.hits.load(Ordering::SeqCst),
        hits_before,
        "denied requests produce zero provider traffic"
    );
}

#[test]
fn cloud_endpoint_cannot_masquerade_as_local_or_free() {
    let (_dir, core) = core();
    // A cloud endpoint declared Local: locality is derived from the
    // endpoint, so this configuration is refused outright.
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
    let err = configure(&core, "s1", masquerade).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert!(
        err.message.contains("locality is derived"),
        "must fail for the locality reason, not incidentally: {}",
        err.message
    );

    // The mirror lie: a loopback endpoint declared Cloud.
    let fake = start_fake_ollama();
    let mut config = ollama_config(&fake.base_url, "free");
    config["providers"][0]["locality"] = json!("cloud");
    let err = configure(&core, "s1", config).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert!(err.message.contains("locality is derived"), "{}", err.message);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);
}

#[test]
fn permits_cannot_be_minted_borrowed_or_embedded() {
    let (_dir, core) = core();
    let fake = start_fake_ollama();
    configure(&core, "session-a", ollama_config(&fake.base_url, "paid")).unwrap();

    // Paid model without any permit: refused, zero traffic.
    let err = dispatch(&core, "session-a", json!({}), "hello").unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);

    // A forged permit id: refused, zero traffic.
    let err = dispatch(
        &core,
        "session-a",
        json!({ "permitId": "permit-00000000000000000000000000000000" }),
        "hello",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);

    // An embedded permit object (minting your own authorization)
    // is an unknown field now: refused before any gate runs.
    let err = dispatch(
        &core,
        "session-a",
        json!({ "permit": {
            "id": "self-minted", "provider": "ollama", "model": "llama-x",
            "maxEstimatedTotalTokens": 1000000, "allowUnknownCost": true
        } }),
        "hello",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);

    // Issue a real permit to session A...
    let issued = core
        .ai_command(
            "providers-authorize",
            session(
                "session-a",
                json!({
                    "provider": "ollama", "model": "llama-x",
                    "maxEstimatedTotalTokens": 100000
                }),
            ),
        )
        .unwrap();
    let permit_id = issued["permitId"].as_str().unwrap().to_string();

    // ...session B cannot spend it.
    let err = dispatch(
        &core,
        "session-b",
        json!({ "permitId": permit_id }),
        "hello",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);

    // ...session A can.
    let out = dispatch(
        &core,
        "session-a",
        json!({ "permitId": permit_id }),
        "hello",
    )
    .unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    assert_eq!(fake.hits.load(Ordering::SeqCst), 1);
}

#[test]
fn consumed_permit_cannot_be_replayed_across_config_changes() {
    let (_dir, core) = core();
    let fake_a = start_fake_ollama();
    let fake_b = start_fake_ollama();
    let config_a = ollama_config(&fake_a.base_url, "paid");
    configure(&core, "s1", config_a.clone()).unwrap();
    let issued = core
        .ai_command(
            "providers-authorize",
            session(
                "s1",
                json!({
                    "provider": "ollama", "model": "llama-x",
                    "maxEstimatedTotalTokens": 100000
                }),
            ),
        )
        .unwrap();
    let permit_id = issued["permitId"].as_str().unwrap().to_string();
    dispatch(&core, "s1", json!({ "permitId": permit_id }), "first").unwrap();
    assert_eq!(fake_a.hits.load(Ordering::SeqCst), 1);

    // A → B → A: accounting and permit consumption live outside the
    // per-configuration runtime, so neither may reset.
    configure(&core, "s1", ollama_config(&fake_b.base_url, "paid")).unwrap();
    configure(&core, "s1", config_a).unwrap();
    let err = dispatch(&core, "s1", json!({ "permitId": permit_id }), "replay").unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden, "consumed permit stays consumed");
    assert_eq!(fake_a.hits.load(Ordering::SeqCst), 1, "replay produced no traffic");

    let usage = core
        .ai_command("providers-usage", session("s1", json!({})))
        .unwrap();
    assert_eq!(
        usage["totalRequests"].as_u64().unwrap(),
        1,
        "usage totals survive reconfiguration"
    );
}

#[test]
fn cross_session_stream_access_is_denied() {
    let (_dir, core) = core();
    let fake = start_fake_ollama();
    configure(&core, "owner", ollama_config(&fake.base_url, "free")).unwrap();
    let started = core
        .ai_command(
            "providers-stream-start",
            session(
                "owner",
                json!({ "request": chat("slow stream"), "mode": "auto" }),
            ),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let capability = started["capability"].as_str().unwrap().to_string();

    let poll = |session_id: &str, cap: &str| {
        core.ai_command(
            "providers-stream-poll",
            session(
                session_id,
                json!({ "runId": run_id, "capability": cap }),
            ),
        )
    };
    // Another session with the genuine capability: NotFound.
    let err = poll("intruder", &capability).unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
    // The owner with a wrong capability: NotFound.
    let err = poll("owner", "cap-00000000000000000000000000000000").unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
    // Another session cannot cancel the owner's run either.
    let err = core
        .ai_command(
            "providers-stream-cancel",
            session(
                "intruder",
                json!({ "runId": run_id, "capability": capability }),
            ),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);

    // The owner can: cancel lands and the terminal poll surfaces it.
    std::thread::sleep(Duration::from_millis(200));
    let first = poll("owner", &capability).unwrap();
    assert!(
        first["events"].as_array().unwrap().iter().any(|e| e["kind"] == "token"),
        "stream survived the intruder's cancel attempt: {first}"
    );
    core.ai_command(
        "providers-stream-cancel",
        session(
            "owner",
            json!({ "runId": run_id, "capability": capability }),
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
    // A finished run is gone: further polls are NotFound.
    let err = poll("owner", &capability).unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
}

#[test]
fn budget_comes_from_trusted_config_and_cannot_be_raised_by_the_caller() {
    let (_dir, core) = core();
    let fake = start_fake_ollama();
    let mut config = ollama_config(&fake.base_url, "free");
    config["budget"] = json!({ "maxRequests": 1 });
    configure(&core, "s1", config).unwrap();
    dispatch(&core, "s1", json!({}), "one").unwrap();
    // The caller cannot raise the configured budget — there is no
    // budget field in the dispatch contract at all.
    let err = dispatch(&core, "s1", json!({ "budget": { "maxRequests": 99 } }), "two")
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    // And the configured budget itself refuses the second dispatch.
    let err = dispatch(&core, "s1", json!({}), "two").unwrap_err();
    assert_eq!(err.code, ErrorCode::StorageUnavailable);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 1);
}

#[test]
fn strict_request_bounds_are_enforced_before_traffic() {
    let (_dir, core) = core();
    let fake = start_fake_ollama();
    configure(&core, "s1", ollama_config(&fake.base_url, "free")).unwrap();

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
            session("s1", json!({ "request": many_messages })),
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);

    let err = dispatch(&core, "s1", json!({}), &"x".repeat(40_000)).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);
}

#[test]
fn catalog_answers_without_configuration() {
    let (_dir, core) = core();
    let catalog = core
        .ai_command("providers-catalog", session("s1", json!({})))
        .unwrap();
    assert!(catalog["providers"].as_array().unwrap().len() >= 20);
    // Dispatch before configure fails closed.
    let err = dispatch(&core, "s1", json!({}), "hello").unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRequest);
}
