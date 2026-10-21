//! M6 Slice 4 integration: project-scoped Opportunity
//! Discovery over the M5 stream substrate with a synthetic
//! Ollama provider. Criteria: happy path with grounding +
//! provenance + wire discipline, refusals, provider failure,
//! malformed and ungrounded output, cancellation, duplicate
//! save, and restart persistence of the saved set.

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

fn opportunity_json(n: usize, refs: &[&str]) -> String {
    let concepts: Vec<Value> = (0..n)
        .map(|i| {
            json!({
                "name": format!("Opportunity {i}"),
                "problem": "Dock offices lose supplier quotes in email threads.",
                "targetCustomer": "Small marina operators",
                "solution": "A quote inbox with deadlines.",
                "whyUseful": "It stops missed renewals.",
                "differentiationHypothesis": "Hypothesis: built for shared dock-office inboxes.",
                "revenueModel": "Subscription.",
                "engineeringComplexity": "medium",
                "implementationCost": "about 3-5 weeks for one developer",
                "keyRisks": ["Adoption may be slow"],
                "requiredValidation": "Interview five marina operators.",
                "minimumViableVersion": "One inbox end to end.",
                "sourceReferences": refs,
                "assumptions": ["Quotes arrive mostly by email."]
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
                    "ungrounded" => opportunity_json(6, &["nowhere-1"]),
                    "mixed" => opportunity_json(6, &["supplied-1", "invented-9"]),
                    _ => opportunity_json(6, &["supplied-1"]),
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
    dir: tempfile::TempDir,
    core: Core,
    ws: String,
    project: String,
    context_memory: String,
}

fn fixture(fake: &Fake) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let ws = core
        .workspace_command(
            "create",
            json!({ "contractVersion": 1, "name": "Opportunities" }),
        )
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
            json!({ "contractVersion": 1, "workspaceId": ws, "draft": draft("Field notes", "The selected-memory-phrase lives here.") }),
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
        dir,
        core,
        ws,
        project,
        context_memory,
    }
}

fn start_run(f: &Fixture, extra: Value) -> Value {
    let mut body = json!({
        "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
        "sources": [{ "kind": "problem", "text": "Marina offices lose supplier quotes. The supplied-problem-phrase." }],
        "memoryIds": []
    });
    if let (Some(dst), Some(src)) = (body.as_object_mut(), extra.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    f.core
        .ai_command("opportunities-generate-start", body)
        .unwrap()
}

fn poll_until(f: &Fixture, run_id: &str, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let polled = f
            .core
            .ai_command(
                "opportunities-generate-poll",
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

fn wait_until(deadline: Duration, what: &str, mut condition: impl FnMut() -> bool) {
    let start = Instant::now();
    while !condition() {
        assert!(start.elapsed() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

// ---------------------------------------------------------------------------
// Tests

#[test]
fn discovery_completes_with_grounding_provenance_wire_discipline_and_idempotent_save() {
    let fake = start_fake();
    let f = fixture(&fake);
    let started = start_run(
        &f,
        json!({ "memoryIds": [f.context_memory], "zeroCostLock": true }),
    );
    let run_id = started["runId"].as_str().unwrap().to_string();
    assert_eq!(started["disclosure"]["provider"], "ollama");
    let done = poll_until(&f, &run_id, Duration::from_secs(15));
    assert_eq!(done["status"], "complete", "{done}");
    let set = &done["opportunitySet"];
    assert_eq!(set["format"], "anansi-opportunity-set");
    assert_eq!(
        set["topic"],
        "Marina offices lose supplier quotes. The supplied-problem-phrase."
    );
    assert_eq!(set["concepts"].as_array().unwrap().len(), 6);
    assert_eq!(
        set["concepts"][0]["sourceReferences"],
        json!(["supplied-1"])
    );
    assert_eq!(
        set["concepts"][0]["assumptions"],
        json!(["Quotes arrive mostly by email."])
    );
    // Provenance: the supplied text is embedded verbatim; the
    // memory source is recorded by id + revision.
    let sources = set["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0]["sourceId"], "supplied-1");
    assert_eq!(sources[0]["kind"], "problem");
    assert!(
        sources[0]["text"]
            .as_str()
            .unwrap()
            .contains("supplied-problem-phrase")
    );
    assert_eq!(sources[1]["kind"], "memory");
    assert_eq!(sources[1]["memoryId"], f.context_memory);
    assert!(
        sources[1]["revisionId"]
            .as_str()
            .is_some_and(|r| !r.is_empty())
    );
    // Wire discipline: supplied + selected sent, unselected not.
    let sent = fake.last_body.lock().unwrap().clone();
    assert!(
        sent.contains("supplied-problem-phrase"),
        "supplied source must be transmitted"
    );
    assert!(
        sent.contains("selected-memory-phrase"),
        "selected memory must be transmitted"
    );
    assert!(
        !sent.contains("unselected-secret-phrase"),
        "unselected memory must never be transmitted"
    );
    assert!(
        sent.contains("supplied-1"),
        "sources travel under their source ids"
    );

    // Save: one idea-type artifact with model provenance;
    // a second save resolves to the same artifact.
    let saved = f
        .core
        .ai_command(
            "opportunities-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    assert_eq!(saved["alreadySaved"], false);
    assert_eq!(saved["artifact"]["type"], "idea");
    assert!(
        saved["artifact"]["title"]
            .as_str()
            .unwrap()
            .starts_with("Opportunity set:")
    );
    assert_eq!(saved["artifact"]["generator"]["kind"], "model");
    assert_eq!(saved["artifact"]["generator"]["provider"], "ollama");
    assert_eq!(
        saved["artifact"]["sourceReferences"],
        json!([format!("memory://{}", f.context_memory)])
    );
    let saved_again = f
        .core
        .ai_command(
            "opportunities-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    assert_eq!(saved_again["alreadySaved"], true);
    assert_eq!(
        saved_again["artifact"]["artifactId"],
        saved["artifact"]["artifactId"]
    );

    // The saved body parses as an idea set (Slice 3 evaluation
    // compatibility) and survives a restart.
    let parsed = anansi_core::ideas::parse_idea_set(saved["body"].as_str().unwrap(), "")
        .expect("saved opportunity body must parse as an idea set");
    assert_eq!(parsed.concepts.len(), 6);
    let reopened = Core::with_storage_root(f.dir.path().to_path_buf());
    let read = reopened
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": saved["artifact"]["artifactId"], "version": 1 }),
        )
        .unwrap();
    assert!(
        read["body"]
            .as_str()
            .unwrap()
            .contains("supplied-problem-phrase")
    );
}

#[test]
fn concurrent_saves_register_one_artifact() {
    let fake = start_fake();
    let f = fixture(&fake);
    let started = start_run(&f, json!({}));
    let run_id = started["runId"].as_str().unwrap().to_string();
    let done = poll_until(&f, &run_id, Duration::from_secs(15));
    assert_eq!(done["status"], "complete", "{done}");
    fn save_once(root: std::path::PathBuf, ws: String, run: String) -> Value {
        let core = Core::with_storage_root(root);
        core.ai_command(
            "opportunities-save",
            json!({ "contractVersion": 1, "workspaceId": ws, "runId": run }),
        )
        .unwrap()
    }
    let root = f.dir.path().to_path_buf();
    let (ws, run) = (f.ws.clone(), run_id.clone());
    let (root2, ws2, run2) = (root.clone(), ws.clone(), run.clone());
    let t = std::thread::spawn(move || save_once(root2, ws2, run2));
    let first = save_once(root, ws, run);
    let second = t.join().unwrap();
    assert_eq!(
        first["artifact"]["artifactId"], second["artifact"]["artifactId"],
        "concurrent saves must resolve to one artifact"
    );
    let list = f
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project }),
        )
        .unwrap();
    assert_eq!(list["artifacts"].as_array().unwrap().len(), 1);
}

#[test]
fn start_refusals_no_sources_unknown_memory_bad_kind_and_archived_project() {
    let fake = start_fake();
    let f = fixture(&fake);
    // No sources at all.
    let err = f
        .core
        .ai_command(
            "opportunities-generate-start",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project, "sources": [], "memoryIds": [] }),
        )
        .unwrap_err();
    assert!(
        err.message.contains("at least one source"),
        "{}",
        err.message
    );
    // Unknown memory id (well-formed but absent).
    let err = f
        .core
        .ai_command(
            "opportunities-generate-start",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                     "sources": [], "memoryIds": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"] }),
        )
        .unwrap_err();
    assert!(
        err.message.contains("not found in this workspace"),
        "{}",
        err.message
    );
    // A memory from ANOTHER workspace is refused the same way.
    let other = f
        .core
        .workspace_command("create", json!({ "contractVersion": 1, "name": "Other" }))
        .unwrap();
    let other_ws = other["workspace"]["id"].as_str().unwrap().to_string();
    let foreign = f
        .core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": other_ws, "draft": draft("Foreign", "foreign body") }),
        )
        .unwrap();
    let err = f
        .core
        .ai_command(
            "opportunities-generate-start",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                     "sources": [], "memoryIds": [foreign["memory"]["id"]] }),
        )
        .unwrap_err();
    assert!(
        err.message.contains("not found in this workspace"),
        "{}",
        err.message
    );
    // Unknown supplied kind.
    let err = f
        .core
        .ai_command(
            "opportunities-generate-start",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                     "sources": [{ "kind": "tweet", "text": "hello" }], "memoryIds": [] }),
        )
        .unwrap_err();
    assert!(err.message.contains("kind must be"), "{}", err.message);
    // Archived project refuses generation.
    let current = f
        .core
        .memory_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "id": f.project }),
        )
        .unwrap();
    f.core
        .memory_command(
            "revise",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "id": f.project,
                    "expectedRevision": current["memory"]["revisionId"],
                    "draft": { "title": "Harbor App", "body": "project", "privacy": "private" },
                    "lifecycle": { "state": "archived", "confirmed": true, "successorId": null } }),
        )
        .unwrap();
    let err = f
        .core
        .ai_command(
            "opportunities-generate-start",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                     "sources": [{ "kind": "problem", "text": "A real problem." }], "memoryIds": [] }),
        )
        .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn provider_failure_malformed_and_ungrounded_runs_end_honestly() {
    let fake = start_fake();
    let f = fixture(&fake);
    // Provider 500 at connect: terminal failed run.
    *fake.mode.lock().unwrap() = "fail".to_string();
    let started = start_run(&f, json!({}));
    let run_id = started["runId"].as_str().unwrap().to_string();
    let done = poll_until(&f, &run_id, Duration::from_secs(15));
    assert_eq!(done["status"], "failed", "{done}");
    // Malformed output: invalid, nothing saved.
    *fake.mode.lock().unwrap() = "malformed".to_string();
    let started = start_run(&f, json!({}));
    let done = poll_until(
        &f,
        started["runId"].as_str().unwrap(),
        Duration::from_secs(15),
    );
    assert_eq!(done["status"], "invalid", "{done}");
    // Ungrounded citations: invalid with the grounding reason.
    *fake.mode.lock().unwrap() = "ungrounded".to_string();
    let started = start_run(&f, json!({}));
    let done = poll_until(
        &f,
        started["runId"].as_str().unwrap(),
        Duration::from_secs(15),
    );
    assert_eq!(done["status"], "invalid", "{done}");
    assert!(
        done["error"].as_str().unwrap().contains("does not cite"),
        "{done}"
    );
    // Fabricated citations mixed with real ones are dropped,
    // and the grounded set completes.
    *fake.mode.lock().unwrap() = "mixed".to_string();
    let started = start_run(&f, json!({}));
    let done = poll_until(
        &f,
        started["runId"].as_str().unwrap(),
        Duration::from_secs(15),
    );
    assert_eq!(done["status"], "complete", "{done}");
    assert_eq!(
        done["opportunitySet"]["concepts"][0]["sourceReferences"],
        json!(["supplied-1"])
    );
}

#[test]
fn cancel_stops_the_stream_and_nothing_can_be_saved() {
    let fake = start_fake();
    *fake.mode.lock().unwrap() = "valid-slow".to_string();
    let f = fixture(&fake);
    let started = start_run(&f, json!({}));
    let run_id = started["runId"].as_str().unwrap().to_string();
    wait_until(Duration::from_secs(10), "the stream to begin", || {
        fake.dripped.load(Ordering::SeqCst)
    });
    let cancelling = f
        .core
        .ai_command(
            "opportunities-generate-cancel",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    assert!(
        matches!(
            cancelling["status"].as_str().unwrap(),
            "cancelling" | "cancelled"
        ),
        "{cancelling}"
    );
    let done = poll_until(&f, &run_id, Duration::from_secs(20));
    assert_eq!(done["status"], "cancelled", "{done}");
    let err = f
        .core
        .ai_command(
            "opportunities-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap_err();
    assert!(err.message.contains("Only a completed"), "{}", err.message);
}

#[test]
fn runs_are_isolated_between_workspaces() {
    let fake = start_fake();
    let f = fixture(&fake);
    let started = start_run(&f, json!({}));
    let run_id = started["runId"].as_str().unwrap().to_string();
    let other = f
        .core
        .workspace_command(
            "create",
            json!({ "contractVersion": 1, "name": "Elsewhere" }),
        )
        .unwrap();
    let other_ws = other["workspace"]["id"].as_str().unwrap().to_string();
    let err = f
        .core
        .ai_command(
            "opportunities-generate-poll",
            json!({ "contractVersion": 1, "workspaceId": other_ws, "runId": run_id }),
        )
        .unwrap_err();
    assert_eq!(err.code, anansi_core::ErrorCode::NotFound);
    let done = poll_until(&f, &run_id, Duration::from_secs(15));
    assert_eq!(done["status"], "complete", "{done}");
}
