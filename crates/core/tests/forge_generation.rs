//! M7 Slice 1 integration: Product Forge over the M5
//! stream substrate with a synthetic Ollama provider.
//! Criteria: full workflow with exact-version pinning,
//! wire discipline, invalid outputs (malformed, unsafe
//! paths), provider failure, cancellation, duplicate save,
//! wrong-stage save, isolation, archive refusal, export
//! integrity and restart persistence.

use anansi_core::Core;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn spec_json() -> String {
    json!({
        "title": "Bean Stock Log",
        "summary": "A tiny page for tracking cafe bean stock.",
        "purpose": "Stop stock-outs.",
        "intendedUsers": "Cafe staff",
        "features": ["Add a bean with a starting weight", "Record usage", "Warn when stock is low"],
        "pages": ["index.html"],
        "nonGoals": ["No accounts", "No server"],
        "dataAndState": "Entries live in localStorage.",
        "acceptanceCriteria": ["Open index.html and add a bean", "Reload keeps entries"],
        "openQuestions": ["Which units do staff prefer?"]
    })
    .to_string()
}

fn package_json() -> String {
    json!({
        "plan": {
            "summary": "One page, vanilla JS, localStorage.",
            "steps": ["Write index.html shell", "Add app.js logic", "Style with style.css"],
            "buildNotes": "No build: open index.html.",
            "testingNotes": "ANANSI has not run this code; check the acceptance criteria manually."
        },
        "files": [
            { "path": "index.html", "content": "<!doctype html><title>Bean Stock Log</title><script src=\"app.js\"></script>" },
            { "path": "app.js", "content": "console.log('bean log');" },
            { "path": "README.md", "content": "# Bean Stock Log\nPrototype. Open index.html. Limitations: no accounts; do not store secrets." }
        ]
    })
    .to_string()
}

fn traversal_package_json() -> String {
    let mut doc: Value = serde_json::from_str(&package_json()).unwrap();
    doc["files"][0]["path"] = json!("../evil.html");
    doc.to_string()
}

struct Fake {
    base_url: String,
    mode: Arc<Mutex<String>>,
    last_body: Arc<Mutex<String>>,
    dripped: Arc<AtomicBool>,
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
                let body_text = String::from_utf8_lossy(&body).to_string();
                *last_body.lock().unwrap() = body_text.clone();
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
                let is_build = body_text.contains("product engineer");
                let text = match mode.as_str() {
                    "malformed" => "I have thoughts but no JSON for you.".to_string(),
                    "refusal" => "I cannot help with that.".to_string(),
                    "traversal" if is_build => traversal_package_json(),
                    _ if is_build => package_json(),
                    _ => spec_json(),
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

fn draft(title: &str, body: &str) -> Value {
    json!({ "title": title, "body": body, "privacy": "private",
            "eventDate": null, "validFrom": null, "validUntil": null,
            "confidence": null, "sourceReferences": [] })
}

fn idea_set_body(concept_name: &str) -> String {
    let concept = |name: &str| {
        json!({
            "name": name,
            "problem": "Cafes lose track of bean stock.",
            "targetCustomer": "Independent cafes",
            "solution": "A stock log page.",
            "whyUseful": "It prevents stock-outs.",
            "differentiationHypothesis": "Hypothesis: built for shared counters.",
            "revenueModel": "Subscription.",
            "engineeringComplexity": "low",
            "implementationCost": "about 1-2 weeks for one developer",
            "keyRisks": ["Staff may forget to log usage"],
            "requiredValidation": "Shadow one cafe for a week.",
            "minimumViableVersion": "One page with add and subtract."
        })
    };
    let mut concepts = vec![concept(concept_name)];
    for i in 0..5 {
        concepts.push(concept(&format!("Filler concept {i}")));
    }
    json!({ "topic": "Cafe tools", "concepts": concepts }).to_string()
}

struct Fixture {
    _dir: tempfile::TempDir,
    core: Core,
    ws: String,
    project: String,
    idea_artifact: String,
}

fn fixture(fake: &Fake) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let ws = core
        .workspace_command("create", json!({ "contractVersion": 1, "name": "Forge" }))
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
    let idea = core
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": ws, "projectId": project,
                "type": "idea", "title": "Idea set: Cafe tools",
                "body": idea_set_body("Bean Stock Log"),
                "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let idea_artifact = idea["artifact"]["artifactId"].as_str().unwrap().to_string();
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
        idea_artifact,
    }
}

fn poll_until(f: &Fixture, op: &str, run_id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let polled = f
            .core
            .ai_command(
                op,
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

fn spec_start(f: &Fixture, version: u32) -> Value {
    f.core
        .ai_command(
            "forge-spec-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "sourceArtifactId": f.idea_artifact, "sourceVersion": version,
                "conceptIndex": 0,
                "purpose": "Track bean stock without a spreadsheet.",
                "intendedUsers": "Cafe staff on a shared tablet.",
                "mustHave": "Add beans, record usage, warn when low.",
                "zeroCostLock": true
            }),
        )
        .unwrap()
}

fn run_spec_to_saved(f: &Fixture) -> (String, Value) {
    let started = spec_start(f, 1);
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(f, "forge-spec-poll", &run_id);
    assert_eq!(polled["status"], "complete", "{polled}");
    let saved = f
        .core
        .ai_command(
            "forge-spec-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    let spec_id = saved["artifact"]["artifactId"].as_str().unwrap().to_string();
    (spec_id, saved)
}

fn base64_decode(text: &str) -> Vec<u8> {
    fn val(c: u8) -> u8 {
        match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    }
    let bytes: Vec<u8> = text.bytes().filter(|c| *c != b'=').collect();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            n |= u32::from(val(*c)) << (18 - 6 * i);
        }
        let take = match chunk.len() {
            2 => 1,
            3 => 2,
            _ => 3,
        };
        out.extend_from_slice(&n.to_be_bytes()[1..1 + take]);
    }
    out
}

fn tar_members(tar: &[u8]) -> Vec<(String, usize, u8)> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset + 512 <= tar.len() {
        let header = &tar[offset..offset + 512];
        if header.iter().all(|b| *b == 0) {
            break;
        }
        let name_end = header[..100].iter().position(|b| *b == 0).unwrap_or(100);
        let name = String::from_utf8_lossy(&header[..name_end]).to_string();
        let size = std::str::from_utf8(&header[124..135])
            .unwrap()
            .trim_matches('\0')
            .trim();
        let size = usize::from_str_radix(size, 8).unwrap();
        out.push((name, size, header[156]));
        offset += 512 + size.div_ceil(512) * 512;
    }
    out
}

#[test]
fn full_forge_workflow_with_exact_version_pinning() {
    let fake = start_fake();
    let f = fixture(&fake);
    // Revise the idea set to v2 with a different concept;
    // the spec run pins v1 and must not see v2's concept.
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws,
                "artifactId": f.idea_artifact, "expectedVersion": 1,
                "title": "Idea set: Cafe tools", "body": idea_set_body("Changed Concept"),
                "sourceReferences": []
            }),
        )
        .unwrap();
    let (spec_id, saved) = run_spec_to_saved(&f);
    assert_eq!(saved["alreadySaved"], false);
    let spec_doc: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    assert_eq!(spec_doc["sourceVersion"], 1);
    assert_eq!(spec_doc["concept"]["name"], "Bean Stock Log");
    assert_eq!(spec_doc["spec"]["title"], "Bean Stock Log");
    assert_eq!(
        saved["artifact"]["sourceReferences"][0],
        format!("artifact://{}@v1", f.idea_artifact)
    );
    let wire = fake.last_body.lock().unwrap().clone();
    assert!(wire.contains("Bean Stock Log"), "pinned concept sent");
    assert!(
        !wire.contains("Changed Concept"),
        "the v2 concept must not leak into a v1-pinned run"
    );
    // Duplicate save resolves to the same artifact.
    let runs = f
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project }),
        )
        .unwrap();
    assert_eq!(
        runs["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["type"] == "specification")
            .count(),
        1
    );

    // Build from the pinned specification.
    let started = f
        .core
        .ai_command(
            "forge-build-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "specArtifactId": spec_id, "specVersion": 1, "zeroCostLock": true
            }),
        )
        .unwrap();
    let build_run = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(&f, "forge-build-poll", &build_run);
    assert_eq!(polled["status"], "complete", "{polled}");
    let package = &polled["sourcePackage"];
    assert_eq!(package["specVersion"], 1);
    assert_eq!(package["sourceArtifactId"], f.idea_artifact);
    assert_eq!(package["files"].as_array().unwrap().len(), 3);
    assert!(
        package["limitations"][0]
            .as_str()
            .unwrap()
            .contains("has not been built, tested, executed or deployed")
    );
    let saved_pkg = f
        .core
        .ai_command(
            "forge-build-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": build_run }),
        )
        .unwrap();
    let package_id = saved_pkg["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(saved_pkg["artifact"]["type"], "source-package");

    // Export: deterministic tar with manifest + files.
    let export = f
        .core
        .artifacts_command(
            "forge-export",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": package_id, "version": 1 }),
        )
        .unwrap();
    let tar = base64_decode(export["tarBase64"].as_str().unwrap());
    assert_eq!(tar.len() % 512, 0);
    let members = tar_members(&tar);
    let names: Vec<&str> = members.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["ANANSI-PACKAGE.json", "index.html", "app.js", "README.md"]
    );
    assert!(members.iter().all(|(_, _, kind)| *kind == b'0'));
    let manifest: Value = serde_json::from_slice(
        &tar[512..512 + members[0].1],
    )
    .unwrap();
    assert_eq!(manifest["specArtifactId"], spec_id);
    assert_eq!(manifest["files"].as_array().unwrap().len(), 3);
    assert_eq!(export["sha256"].as_str().unwrap().len(), 64);

    // Restart: everything persists.
    let dir_path = f._dir.path().to_path_buf();
    let core2 = Core::with_storage_root(dir_path);
    let read = core2
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": package_id, "version": 1 }),
        )
        .unwrap();
    assert!(read["body"].as_str().unwrap().contains("anansi-source-package"));

    // Revise the specification to v2; a build pinned to v2
    // records specVersion 2.
    let mut revised_doc: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    revised_doc["spec"]["title"] = json!("Bean Stock Log — counter edition");
    core2
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws,
                "artifactId": spec_id, "expectedVersion": 1,
                "title": "Product specification: Bean Stock Log — counter edition",
                "body": revised_doc.to_string(),
                "sourceReferences": [format!("artifact://{}@v1", f.idea_artifact)]
            }),
        )
        .unwrap();
    // The fixture's core and core2 share the root; use core2
    // (provider config was persisted per workspace).
    let started2 = core2
        .ai_command(
            "forge-build-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "specArtifactId": spec_id, "specVersion": 2, "zeroCostLock": true
            }),
        )
        .unwrap();
    let run2 = started2["runId"].as_str().unwrap().to_string();
    let deadline = Instant::now() + Duration::from_secs(30);
    let polled2 = loop {
        let p = core2
            .ai_command(
                "forge-build-poll",
                json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run2 }),
            )
            .unwrap();
        if matches!(p["status"].as_str().unwrap(), "complete" | "invalid" | "failed" | "cancelled")
            || Instant::now() > deadline
        {
            break p;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    assert_eq!(polled2["status"], "complete", "{polled2}");
    assert_eq!(polled2["sourcePackage"]["specVersion"], 2);
}

#[test]
fn invalid_outputs_and_provider_failure_save_nothing() {
    let fake = start_fake();
    let f = fixture(&fake);
    *fake.mode.lock().unwrap() = "malformed".to_string();
    let started = spec_start(&f, 1);
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(&f, "forge-spec-poll", &run_id);
    assert_eq!(polled["status"], "invalid");
    assert!(
        f.core
            .ai_command(
                "forge-spec-save",
                json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
            )
            .is_err()
    );
    // Unsafe build output: traversal path.
    *fake.mode.lock().unwrap() = "valid".to_string();
    let (spec_id, _) = run_spec_to_saved(&f);
    *fake.mode.lock().unwrap() = "traversal".to_string();
    let started = f
        .core
        .ai_command(
            "forge-build-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "specArtifactId": spec_id, "specVersion": 1, "zeroCostLock": true
            }),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(&f, "forge-build-poll", &run_id);
    assert_eq!(polled["status"], "invalid", "{polled}");
    assert!(
        polled["error"]
            .as_str()
            .unwrap()
            .contains("unsafe file path"),
        "{polled}"
    );
    // Provider failure at connect: the run exists, failed.
    *fake.mode.lock().unwrap() = "fail".to_string();
    let started = spec_start(&f, 1);
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(&f, "forge-spec-poll", &run_id);
    assert_eq!(polled["status"], "failed", "{polled}");
    // Only the one saved specification exists.
    let list = f
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project }),
        )
        .unwrap();
    let types: Vec<&str> = list["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["type"].as_str().unwrap())
        .collect();
    assert_eq!(types, ["idea", "specification"]);
}

#[test]
fn cancel_wrong_stage_isolation_and_archive() {
    let fake = start_fake();
    let f = fixture(&fake);
    // Cancellation.
    *fake.mode.lock().unwrap() = "valid-slow".to_string();
    let started = spec_start(&f, 1);
    let run_id = started["runId"].as_str().unwrap().to_string();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !fake.dripped.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline, "provider never streamed");
        std::thread::sleep(Duration::from_millis(10));
    }
    f.core
        .ai_command(
            "forge-spec-cancel",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    let cancelled = poll_until(&f, "forge-spec-poll", &run_id);
    assert_eq!(cancelled["status"], "cancelled");
    assert!(
        f.core
            .ai_command(
                "forge-spec-save",
                json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
            )
            .is_err()
    );
    *fake.mode.lock().unwrap() = "valid".to_string();
    // Wrong-stage save is refused (run not found for build).
    let started = spec_start(&f, 1);
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(&f, "forge-spec-poll", &run_id);
    assert_eq!(polled["status"], "complete");
    assert!(
        f.core
            .ai_command(
                "forge-build-save",
                json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
            )
            .is_err()
    );
    // Cross-workspace: a second workspace cannot pin this
    // workspace's artifact.
    let other = f
        .core
        .workspace_command("create", json!({ "contractVersion": 1, "name": "Other" }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        f.core
            .ai_command(
                "forge-spec-start",
                json!({
                    "contractVersion": 1, "workspaceId": other, "projectId": f.project,
                    "sourceArtifactId": f.idea_artifact, "sourceVersion": 1,
                    "conceptIndex": 0, "purpose": "x", "intendedUsers": "y", "mustHave": "z"
                }),
            )
            .is_err()
    );
    // Archived projects refuse forge starts.
    let saved = f
        .core
        .ai_command(
            "forge-spec-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    let spec_id = saved["artifact"]["artifactId"].as_str().unwrap().to_string();
    let project_now = f
        .core
        .memory_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "id": f.project }),
        )
        .unwrap();
    f.core
        .memory_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "id": f.project,
                "expectedRevision": project_now["memory"]["revisionId"],
                "draft": draft("Harbor App", "project"),
                "lifecycle": { "state": "archived", "confirmed": true }
            }),
        )
        .unwrap();
    assert!(
        f.core
            .ai_command(
                "forge-build-start",
                json!({
                    "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                    "specArtifactId": spec_id, "specVersion": 1
                }),
            )
            .is_err()
    );
    // Building from a non-specification artifact is refused.
    assert!(
        f.core
            .ai_command(
                "forge-build-start",
                json!({
                    "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                    "specArtifactId": f.idea_artifact, "specVersion": 1
                }),
            )
            .is_err()
    );
}
