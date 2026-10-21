//! M6 Slice 2 integration: project-scoped idea generation over
//! the M5 stream substrate with a synthetic Ollama provider.
//! Criteria: happy path with provenance + context discipline,
//! cancellation, provider failure, malformed output, duplicate
//! save, isolation, and restart persistence of the saved set.

use anansi_core::Core;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Synthetic streaming provider

struct Fake {
    base_url: String,
    mode: Arc<Mutex<String>>,
    last_body: Arc<Mutex<String>>,
    dripped: Arc<AtomicBool>,
}

fn idea_json(n: usize) -> String {
    let concepts: Vec<Value> = (0..n)
        .map(|i| {
            json!({
                "name": format!("Concept {i}"),
                "problem": "A problem worth solving.",
                "targetCustomer": "Independent cafes",
                "solution": "A focused tool.",
                "whyUseful": "It saves time weekly.",
                "differentiationHypothesis": "Hypothesis: incumbents ignore this niche.",
                "revenueModel": "Subscription.",
                "engineeringComplexity": "medium",
                "implementationCost": "about 3-5 weeks for one developer",
                "keyRisks": ["Adoption may be slow"],
                "requiredValidation": "Interview five customers.",
                "minimumViableVersion": "One workflow end to end."
            })
        })
        .collect();
    json!({ "concepts": concepts }).to_string()
}

fn ndjson_chunks(text: &str, chunks: usize) -> Vec<String> {
    let step = text.len().div_ceil(chunks).max(1);
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let end = (i + step).min(text.len());
        out.push(text[i..end].to_string());
        i = end;
    }
    out
}

fn start_fake() -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let mode = Arc::new(Mutex::new("valid".to_string()));
    let last_body = Arc::new(Mutex::new(String::new()));
    let dripped = Arc::new(AtomicBool::new(false));
    let (m2, b2, d2) = (mode.clone(), last_body.clone(), dripped.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (mode, last_body, dripped) = (m2.clone(), b2.clone(), d2.clone());
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let lower = line.to_lowercase();
                    if let Some(v) = lower.strip_prefix("content-length:") {
                        content_length = v.trim().parse().unwrap_or(0);
                    }
                    if line.trim().is_empty() {
                        break;
                    }
                }
                let mut body = vec![0u8; content_length];
                reader.read_exact(&mut body).unwrap_or(());
                *last_body.lock().unwrap() = String::from_utf8_lossy(&body).to_string();
                let mode = mode.lock().unwrap().clone();
                if mode == "fail" {
                    let payload = "provider exploded";
                    let resp = format!(
                        "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        payload.len(),
                        payload
                    );
                    let _ = stream.write_all(resp.as_bytes());
                    return;
                }
                let text = match mode.as_str() {
                    "malformed" => "I have thoughts but no JSON for you.".to_string(),
                    "few" => idea_json(3),
                    _ => idea_json(6),
                };
                let slow = mode == "valid-slow";
                let chunks = ndjson_chunks(&text, if slow { 24 } else { 3 });
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                );
                for (i, chunk) in chunks.iter().enumerate() {
                    let line = format!(
                        "{{\"model\":\"llama-x\",\"message\":{{\"role\":\"assistant\",\"content\":{}}},\"done\":false}}\n",
                        serde_json::to_string(chunk).unwrap()
                    );
                    let frame = format!("{:x}\r\n{}\r\n", line.len(), line);
                    if stream.write_all(frame.as_bytes()).is_err() {
                        return;
                    }
                    if i == 0 {
                        dripped.store(true, Ordering::SeqCst);
                    }
                    if slow {
                        std::thread::sleep(Duration::from_millis(250));
                    }
                }
                let final_line = "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true,\"prompt_eval_count\":10,\"eval_count\":40}\n";
                let frame = format!("{:x}\r\n{}\r\n0\r\n\r\n", final_line.len(), final_line);
                let _ = stream.write_all(frame.as_bytes());
            });
        }
    });
    Fake {
        base_url: format!("http://{addr}"),
        mode,
        last_body,
        dripped,
    }
}

// ---------------------------------------------------------------------------
// Fixture

fn draft(title: &str, body: &str) -> Value {
    json!({ "title": title, "body": body, "privacy": "private",
            "eventDate": null, "validFrom": null, "validUntil": null,
            "confidence": null, "sourceReferences": [] })
}

struct Fixture {
    _dir: tempfile::TempDir,
    core: Core,
    ws: String,
    project: String,
    context_memory: String,
}

fn fixture(fake: &Fake) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let ws = core
        .workspace_command("create", json!({ "contractVersion": 1, "name": "Ideas" }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let project = core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": ws, "kind": "project", "draft": draft("Harbor App", "project") }),
        )
        .unwrap();
    let project = project["memory"]["id"].as_str().unwrap().to_string();
    let context = core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": ws, "draft": draft("Field notes", "The approved-context-phrase lives here.") }),
        )
        .unwrap();
    let context_memory = context["memory"]["id"].as_str().unwrap().to_string();
    core.memory_command(
        "create",
        json!({ "contractVersion": 1, "workspaceId": ws, "draft": draft("Private diary", "The unselected-secret-phrase must never be sent.") }),
    )
    .unwrap();
    core.ai_command(
        "providers-configure",
        json!({
            "workspaceId": ws,
            "config": {
                "providers": [{
                    "config": { "provider": "ollama", "baseUrl": fake.base_url, "credential": null, "enabled": true },
                    "locality": "local",
                    "models": [{
                        "id": "llama-x", "provider": "ollama",
                        "capabilities": { "chat": true, "streaming": true, "vision": false, "coding": true, "deepReasoning": false, "embeddings": false },
                        "contextWindow": 8192, "cost": "free", "speed": "balanced"
                    }]
                }],
                "budget": { "maxRequests": 50, "maxTotalTokens": 500000 }
            }
        }),
    )
    .unwrap();
    Fixture {
        _dir: dir,
        core,
        ws,
        project,
        context_memory,
    }
}

fn start_run(f: &Fixture, extra: Value) -> Value {
    let mut body = json!({
        "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
        "topic": "harbor logistics", "customer": "marina operators", "budget": "small",
        "technology": "rust", "objective": "find a product", "contextMemoryIds": []
    });
    if let (Some(dst), Some(src)) = (body.as_object_mut(), extra.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    f.core.ai_command("ideas-generate-start", body).unwrap()
}

fn poll_until(f: &Fixture, run_id: &str, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let polled = f
            .core
            .ai_command(
                "ideas-generate-poll",
                json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
            )
            .unwrap();
        let status = polled["status"].as_str().unwrap();
        if matches!(status, "complete" | "invalid" | "failed" | "cancelled")
            || Instant::now() > deadline
        {
            return polled;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

// ---------------------------------------------------------------------------
// Tests

#[test]
fn generation_completes_with_provenance_context_discipline_and_idempotent_save() {
    let fake = start_fake();
    let f = fixture(&fake);
    let started = start_run(
        &f,
        json!({ "contextMemoryIds": [f.context_memory], "zeroCostLock": true }),
    );
    let run_id = started["runId"].as_str().unwrap().to_string();
    assert_eq!(started["disclosure"]["provider"], "ollama");
    let done = poll_until(&f, &run_id, Duration::from_secs(15));
    assert_eq!(done["status"], "complete", "{done}");
    assert_eq!(done["ideaSet"]["concepts"].as_array().unwrap().len(), 6);
    assert_eq!(done["ideaSet"]["topic"], "harbor logistics");
    assert_eq!(done["disclosure"]["provider"], "ollama");
    assert_eq!(done["disclosure"]["model"], "llama-x");
    // Context discipline on the wire: the selected memory was
    // sent; the unselected one was not.
    let sent = fake.last_body.lock().unwrap().clone();
    assert!(sent.contains("approved-context-phrase"), "selected context must be transmitted");
    assert!(
        !sent.contains("unselected-secret-phrase"),
        "unselected memories must never be transmitted"
    );

    // Save is idempotent.
    let save = |f: &Fixture| {
        f.core
            .ai_command(
                "ideas-save",
                json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
            )
            .unwrap()
    };
    let first = save(&f);
    assert_eq!(first["alreadySaved"], false);
    assert_eq!(first["artifact"]["type"], "idea");
    assert_eq!(first["artifact"]["validationStatus"], "validated");
    assert_eq!(first["artifact"]["generator"]["kind"], "model");
    assert_eq!(first["artifact"]["generator"]["provider"], "ollama");
    assert_eq!(first["artifact"]["generator"]["model"], "llama-x");
    assert_eq!(first["artifact"]["sourceReferences"], json!([format!("memory://{}", f.context_memory)]));
    let second = save(&f);
    assert_eq!(second["alreadySaved"], true);
    assert_eq!(second["artifact"]["artifactId"], first["artifact"]["artifactId"]);
    let list = f
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project }),
        )
        .unwrap();
    assert_eq!(list["artifacts"].as_array().unwrap().len(), 1, "exactly one artifact after duplicate saves");
}

#[test]
fn cancellation_terminates_and_cannot_be_saved() {
    let fake = start_fake();
    *fake.mode.lock().unwrap() = "valid-slow".to_string();
    let f = fixture(&fake);
    let started = start_run(&f, json!({}));
    let run_id = started["runId"].as_str().unwrap().to_string();
    while !fake.dripped.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(10));
    }
    f.core
        .ai_command(
            "ideas-generate-cancel",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    let done = poll_until(&f, &run_id, Duration::from_secs(15));
    assert_eq!(done["status"], "cancelled", "{done}");
    let err = f
        .core
        .ai_command(
            "ideas-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap_err();
    assert_eq!(err.code, anansi_core::ErrorCode::InvalidRequest);
}

#[test]
fn provider_failure_and_malformed_output_are_terminal_and_unsavable() {
    let fake = start_fake();
    let f = fixture(&fake);
    *fake.mode.lock().unwrap() = "fail".to_string();
    let started = start_run(&f, json!({}));
    let done = poll_until(&f, started["runId"].as_str().unwrap(), Duration::from_secs(15));
    assert_eq!(done["status"], "failed", "{done}");
    assert!(done["error"].as_str().is_some_and(|e| !e.is_empty()));

    *fake.mode.lock().unwrap() = "malformed".to_string();
    let started = start_run(&f, json!({}));
    let done = poll_until(&f, started["runId"].as_str().unwrap(), Duration::from_secs(15));
    assert_eq!(done["status"], "invalid", "{done}");
    assert!(done["error"].as_str().unwrap().contains("did not return a structured idea set"));

    *fake.mode.lock().unwrap() = "few".to_string();
    let started = start_run(&f, json!({}));
    let run_id = started["runId"].as_str().unwrap().to_string();
    let done = poll_until(&f, &run_id, Duration::from_secs(15));
    assert_eq!(done["status"], "invalid", "{done}");
    assert!(f
        .core
        .ai_command("ideas-save", json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }))
        .is_err());
}

#[test]
fn isolation_and_project_validation() {
    let fake = start_fake();
    let f = fixture(&fake);
    let started = start_run(&f, json!({}));
    let run_id = started["runId"].as_str().unwrap().to_string();
    // Second workspace: the run is invisible there.
    let ws_b = f
        .core
        .workspace_command("create", json!({ "contractVersion": 1, "name": "B" }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = f
        .core
        .ai_command(
            "ideas-generate-poll",
            json!({ "contractVersion": 1, "workspaceId": ws_b, "runId": run_id }),
        )
        .unwrap_err();
    assert_eq!(err.code, anansi_core::ErrorCode::NotFound);
    // A foreign context memory is refused at start.
    let foreign = f
        .core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": ws_b, "draft": draft("B note", "b") }),
        )
        .unwrap();
    let err = f
        .core
        .ai_command(
            "ideas-generate-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "topic": "t", "customer": "c", "budget": "b", "technology": "x", "objective": "o",
                "contextMemoryIds": [foreign["memory"]["id"].as_str().unwrap()]
            }),
        )
        .unwrap_err();
    assert_eq!(err.code, anansi_core::ErrorCode::InvalidRequest);
    // A non-project record cannot host generation.
    let note = f
        .core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "draft": draft("plain", "x") }),
        )
        .unwrap();
    let err = f
        .core
        .ai_command(
            "ideas-generate-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": note["memory"]["id"].as_str().unwrap(),
                "topic": "t", "customer": "c", "budget": "b", "technology": "x", "objective": "o",
                "contextMemoryIds": []
            }),
        )
        .unwrap_err();
    assert_eq!(err.code, anansi_core::ErrorCode::InvalidRequest);
}

#[test]
fn saved_set_survives_restart_as_a_registry_artifact() {
    let fake = start_fake();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let (ws, project, artifact_id) = {
        let f = {
            let core = Core::with_storage_root(root.clone());
            // Rebuild the fixture on this root.
            let ws = core
                .workspace_command("create", json!({ "contractVersion": 1, "name": "Ideas" }))
                .unwrap()["workspace"]["id"]
                .as_str()
                .unwrap()
                .to_string();
            let project = core
                .memory_command(
                    "create",
                    json!({ "contractVersion": 1, "workspaceId": ws, "kind": "project", "draft": draft("Harbor App", "project") }),
                )
                .unwrap();
            let project = project["memory"]["id"].as_str().unwrap().to_string();
            core.ai_command(
                "providers-configure",
                json!({
                    "workspaceId": ws,
                    "config": {
                        "providers": [{
                            "config": { "provider": "ollama", "baseUrl": fake.base_url, "credential": null, "enabled": true },
                            "locality": "local",
                            "models": [{
                                "id": "llama-x", "provider": "ollama",
                                "capabilities": { "chat": true, "streaming": true, "vision": false, "coding": true, "deepReasoning": false, "embeddings": false },
                                "contextWindow": 8192, "cost": "free", "speed": "balanced"
                            }]
                        }],
                        "budget": { "maxRequests": 50, "maxTotalTokens": 500000 }
                    }
                }),
            )
            .unwrap();
            Fixture { _dir: tempfile::tempdir().unwrap(), core, ws, project, context_memory: String::new() }
        };
        let started = start_run(&f, json!({}));
        let run_id = started["runId"].as_str().unwrap().to_string();
        let done = poll_until(&f, &run_id, Duration::from_secs(15));
        assert_eq!(done["status"], "complete");
        let saved = f
            .core
            .ai_command("ideas-save", json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }))
            .unwrap();
        (
            f.ws.clone(),
            f.project.clone(),
            saved["artifact"]["artifactId"].as_str().unwrap().to_string(),
        )
    };
    let core = Core::with_storage_root(root);
    let read = core
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": ws, "artifactId": artifact_id }),
        )
        .unwrap();
    assert_eq!(read["artifact"]["projectId"], project);
    assert_eq!(read["artifact"]["generator"]["kind"], "model");
    let set: Value = serde_json::from_str(read["body"].as_str().unwrap()).unwrap();
    assert_eq!(set["concepts"].as_array().unwrap().len(), 6);
}
