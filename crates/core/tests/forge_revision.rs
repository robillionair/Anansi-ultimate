//! M7 Slice 2 integration: revising a saved source
//! package over the M5 stream substrate with a synthetic
//! Ollama provider. Criteria: full revise workflow with
//! server-computed diff, reject-saves-nothing, retry after
//! reject, accept as a new artifact version with exact
//! parent/specification references, approval gate on the
//! pinned specification version (no approval created or
//! switched by a revision), cancellation, invalid outputs
//! (traversal, reserved names, prefix conflicts, oversize),
//! stale-version save conflicts, restart persistence and
//! backup/fresh restore with remapped parent/spec
//! references and transferred approval.

use anansi_core::Core;
use anansi_core::content;
use anansi_core::jobs::ExportSelection;
use anansi_core::readiness::CommandError;
use anansi_core::storage::Storage;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
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

fn revised_package_json() -> String {
    json!({
        "plan": {
            "summary": "One page, vanilla JS, localStorage, plus a notes file.",
            "steps": ["Update index.html heading", "Add notes.txt", "Keep app.js logic"],
            "buildNotes": "No build: open index.html. The revision kept the specification unchanged.",
            "testingNotes": "ANANSI has not run this code; check the acceptance criteria manually."
        },
        "files": [
            { "path": "index.html", "content": "<!doctype html><title>Bean Stock Log — revised</title><script src=\"app.js\"></script>" },
            { "path": "app.js", "content": "console.log('bean log revised');" },
            { "path": "README.md", "content": "# Bean Stock Log\nPrototype. Open index.html. Limitations: no accounts; do not store secrets." },
            { "path": "notes.txt", "content": "Revision notes: heading updated." }
        ]
    })
    .to_string()
}

fn bad_package_json(kind: &str) -> String {
    let mut doc: Value = serde_json::from_str(&package_json()).unwrap();
    match kind {
        "traversal" => doc["files"][0]["path"] = json!("../evil.html"),
        "reserved" => doc["files"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "path": "CON.txt", "content": "device name" })),
        "prefix" => doc["files"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "path": "app.js/extra.js", "content": "prefix conflict" })),
        "huge" => doc["files"][1]["content"] = json!("x".repeat(40_000)),
        _ => {}
    }
    doc.to_string()
}

struct Fake {
    base_url: String,
    mode: Arc<Mutex<String>>,
    last_body: Arc<Mutex<String>>,
    dripped: Arc<AtomicBool>,
}

fn ndjson_chunks(text: &str, chunks: usize) -> Vec<String> {
    // Split on character boundaries: the revised package
    // JSON contains non-ASCII characters, and byte slicing
    // would panic mid-character.
    let chars: Vec<char> = text.chars().collect();
    let step = chars.len().div_ceil(chunks).max(1);
    chars
        .chunks(step)
        .map(|chunk| chunk.iter().collect())
        .collect()
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
                // The revision prompt carries the revision
                // request segment; the build prompt carries
                // only the product-engineer instruction.
                let is_revise = body_text.contains("Revision request");
                let is_build = body_text.contains("product engineer");
                let text = match mode.as_str() {
                    "malformed" => "I have thoughts but no JSON for you.".to_string(),
                    "refusal" => "I cannot help with that.".to_string(),
                    "revise-traversal" if is_revise => bad_package_json("traversal"),
                    "revise-reserved" if is_revise => bad_package_json("reserved"),
                    "revise-prefix" if is_revise => bad_package_json("prefix"),
                    "revise-huge" if is_revise => bad_package_json("huge"),
                    _ if is_revise => revised_package_json(),
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

fn set_mode(fake: &Fake, mode: &str) {
    *fake.mode.lock().unwrap() = mode.to_string();
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

fn spec_and_package(f: &Fixture) -> (String, String) {
    let started = f
        .core
        .ai_command(
            "forge-spec-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "sourceArtifactId": f.idea_artifact, "sourceVersion": 1,
                "conceptIndex": 0,
                "purpose": "Track bean stock without a spreadsheet.",
                "intendedUsers": "Cafe staff on a shared tablet.",
                "mustHave": "Add beans, record usage, warn when low.",
                "zeroCostLock": true
            }),
        )
        .unwrap();
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
    let spec_id = saved["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    f.core
        .ai_command(
            "forge-spec-approve",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                    "specArtifactId": spec_id, "specVersion": 1 }),
        )
        .unwrap();
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
    let polled = poll_until(f, "forge-build-poll", &run_id);
    assert_eq!(polled["status"], "complete", "{polled}");
    let saved = f
        .core
        .ai_command(
            "forge-build-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    let package_id = saved["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    (spec_id, package_id)
}

fn revise_start(
    f: &Fixture,
    package_id: &str,
    version: u32,
    instructions: &str,
) -> Result<Value, CommandError> {
    f.core.ai_command(
        "forge-revise-start",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "packageArtifactId": package_id, "packageVersion": version,
            "instructions": instructions, "zeroCostLock": true
        }),
    )
}

fn revise_to_complete(
    f: &Fixture,
    package_id: &str,
    version: u32,
    instructions: &str,
) -> (String, Value) {
    let started = revise_start(f, package_id, version, instructions).unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(f, "forge-revise-poll", &run_id);
    (run_id, polled)
}

fn read_version(f: &Fixture, artifact_id: &str, version: u32) -> Value {
    f.core
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": artifact_id, "version": version }),
        )
        .unwrap()
}

fn versions(f: &Fixture, artifact_id: &str) -> Value {
    f.core
        .artifacts_command(
            "versions",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": artifact_id }),
        )
        .unwrap()
}

fn approval_count(f: &Fixture) -> usize {
    let list = f
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project }),
        )
        .unwrap();
    list["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["type"] == "specification-approval")
        .count()
}

#[test]
fn revision_accept_saves_new_version_with_diff_and_parent_refs() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (spec_id, package_id) = spec_and_package(&f);
    let v1 = read_version(&f, &package_id, 1);
    let v1_body = v1["body"].as_str().unwrap().to_string();

    // Reject first: nothing is saved, v1 is byte-identical.
    let (run_id, polled) = revise_to_complete(
        &f,
        &package_id,
        1,
        "Update the heading and add a notes file.",
    );
    assert_eq!(polled["status"], "complete", "{polled}");
    assert_eq!(
        polled["revisionDiff"],
        json!({ "added": ["notes.txt"], "removed": [], "changed": ["app.js", "index.html"] })
    );
    assert_eq!(polled["sourcePackage"]["specArtifactId"], spec_id);
    assert_eq!(polled["sourcePackage"]["specVersion"], 1);
    // Deliberately no save call here: rejection saves nothing.
    let _ = run_id;
    assert_eq!(
        versions(&f, &package_id)["versions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        read_version(&f, &package_id, 1)["body"].as_str().unwrap(),
        v1_body
    );

    // Retry with the same instructions and accept.
    let (run_id, polled) = revise_to_complete(
        &f,
        &package_id,
        1,
        "Update the heading and add a notes file.",
    );
    assert_eq!(polled["status"], "complete", "{polled}");
    let saved = f
        .core
        .ai_command(
            "forge-revise-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    assert_eq!(saved["alreadySaved"], false);
    assert_eq!(saved["artifact"]["artifactId"], package_id);
    assert_eq!(saved["artifact"]["version"], 2);
    let v2: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    assert_eq!(v2["parentPackageArtifactId"], package_id);
    assert_eq!(v2["parentPackageVersion"], 1);
    assert_eq!(
        v2["revisionInstructions"],
        "Update the heading and add a notes file."
    );
    // The specification is not switched: same id and version.
    assert_eq!(v2["specArtifactId"], spec_id);
    assert_eq!(v2["specVersion"], 1);
    let files: Vec<&str> = v2["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["path"].as_str().unwrap())
        .collect();
    assert!(files.contains(&"notes.txt"));
    // v1 is still the original, byte-identical.
    assert_eq!(
        read_version(&f, &package_id, 1)["body"].as_str().unwrap(),
        v1_body
    );
    // Duplicate save is idempotent.
    let again = f
        .core
        .ai_command(
            "forge-revise-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    assert_eq!(again["alreadySaved"], true);
    assert_eq!(again["artifact"]["version"], 2);
    // The revised version exports with the shared validator.
    let export = f
        .core
        .artifacts_command(
            "forge-export",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": package_id, "version": 2 }),
        )
        .unwrap();
    assert!(
        export["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p == "notes.txt")
    );

    // Restart: both versions reopen exactly.
    let dir_path = f._dir.path().to_path_buf();
    let core2 = Core::with_storage_root(dir_path);
    let read = core2
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": package_id, "version": 2 }),
        )
        .unwrap();
    let doc: Value = serde_json::from_str(read["body"].as_str().unwrap()).unwrap();
    assert_eq!(doc["parentPackageVersion"], 1);
    assert_eq!(doc["specArtifactId"], spec_id);
}

#[test]
fn revision_requires_pinned_spec_approval_and_creates_none() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (spec_id, package_id) = spec_and_package(&f);
    assert_eq!(approval_count(&f), 1);

    // Revising the specification to v2 leaves the package
    // pinned to v1; the revision still pins v1 and creates
    // no new approval record.
    let spec_v1 = read_version(&f, &spec_id, 1);
    let mut spec_doc: Value = serde_json::from_str(spec_v1["body"].as_str().unwrap()).unwrap();
    spec_doc["spec"]["title"] = json!("Bean Stock Log — counter edition");
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws,
                "artifactId": spec_id, "expectedVersion": 1,
                "title": spec_v1["artifact"]["title"], "body": spec_doc.to_string(),
                "sourceReferences": []
            }),
        )
        .unwrap();
    let (run_id, polled) = revise_to_complete(&f, &package_id, 1, "Update the heading.");
    assert_eq!(polled["status"], "complete", "{polled}");
    let saved = f
        .core
        .ai_command(
            "forge-revise-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    let v2: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    assert_eq!(v2["specVersion"], 1);
    assert_eq!(approval_count(&f), 1);

    // A package registered without any approval is refused
    // before provider traffic: the wire capture is unchanged.
    let wire_before = fake.last_body.lock().unwrap().clone();
    let manual_body = json!({
        "format": "anansi-source-package", "formatVersion": 1,
        "template": "static-web-prototype",
        "specArtifactId": spec_id, "specVersion": 2,
        "sourceArtifactId": f.idea_artifact, "sourceVersion": 1,
        "conceptName": "Bean Stock Log",
        "plan": { "summary": "s", "steps": [], "buildNotes": "b", "testingNotes": "t" },
        "files": [
            { "path": "index.html", "content": "<title>Manual</title>" },
            { "path": "README.md", "content": "# Manual\n" }
        ],
        "limitations": []
    })
    .to_string();
    let manual = f
        .core
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "type": "source-package", "title": "Source package: Manual",
                "body": manual_body,
                "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let manual_id = manual["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    let result = revise_start(&f, &manual_id, 1, "Change something.");
    assert!(
        result.is_err(),
        "unapproved pinned spec must be refused: {result:?}"
    );
    assert_eq!(*fake.last_body.lock().unwrap(), wire_before);

    // Empty and over-long instructions are refused at start.
    assert!(revise_start(&f, &package_id, 2, "").is_err());
    assert!(revise_start(&f, &package_id, 2, &"x".repeat(2001)).is_err());
    // Wrong-stage save: a build run cannot be saved via revise.
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
    let result = f.core.ai_command(
        "forge-revise-save",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": build_run }),
    );
    assert!(result.is_err(), "wrong-stage save must be refused");
}

#[test]
fn revision_invalid_outputs_and_cancellation_save_nothing() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (_spec_id, package_id) = spec_and_package(&f);
    for (mode, what) in [
        ("revise-traversal", "traversal"),
        ("revise-reserved", "reserved name"),
        ("revise-prefix", "prefix conflict"),
        ("revise-huge", "oversize file"),
        ("malformed", "malformed output"),
        ("refusal", "refusal"),
    ] {
        set_mode(&fake, mode);
        let (_run_id, polled) = revise_to_complete(&f, &package_id, 1, "Change something.");
        assert_eq!(
            polled["status"], "invalid",
            "{mode} ({what}) must be invalid: {polled}"
        );
        assert!(polled["error"].is_string());
    }
    set_mode(&fake, "valid");
    assert_eq!(
        versions(&f, &package_id)["versions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // Cancellation during streaming saves nothing.
    set_mode(&fake, "valid-slow");
    let started = revise_start(&f, &package_id, 1, "Change something.").unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !fake.dripped.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(fake.dripped.load(Ordering::SeqCst), "fake never dripped");
    // The cancel call itself may return "cancelling" while
    // the stream drains; poll until the terminal state,
    // as the Slice 1 cancellation test does.
    f.core
        .ai_command(
            "forge-revise-cancel",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    let cancelled = poll_until(&f, "forge-revise-poll", &run_id);
    assert_eq!(cancelled["status"], "cancelled", "{cancelled}");
    let result = f.core.ai_command(
        "forge-revise-save",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
    );
    assert!(result.is_err(), "a cancelled run must not save");
    assert_eq!(
        versions(&f, &package_id)["versions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn revision_stale_parent_save_conflicts() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (_spec_id, package_id) = spec_and_package(&f);
    // Two runs from the same parent version.
    let (run_a, polled_a) = revise_to_complete(&f, &package_id, 1, "First revision.");
    assert_eq!(polled_a["status"], "complete", "{polled_a}");
    let (run_b, polled_b) = revise_to_complete(&f, &package_id, 1, "Second revision.");
    assert_eq!(polled_b["status"], "complete", "{polled_b}");
    let saved_b = f
        .core
        .ai_command(
            "forge-revise-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_b }),
        )
        .unwrap();
    assert_eq!(saved_b["artifact"]["version"], 2);
    // The first run's parent version is no longer latest.
    let result = f.core.ai_command(
        "forge-revise-save",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_a }),
    );
    assert!(
        result.is_err(),
        "stale parent save must conflict: {result:?}"
    );
    assert_eq!(
        versions(&f, &package_id)["versions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

fn backup_bytes(root: &Path, store: &Storage, ws: &str) -> Vec<u8> {
    let job = store
        .enqueue_export(ws, ExportSelection::Backup { application: false })
        .unwrap();
    let lease = store.claim_export().unwrap().unwrap();
    store.execute_backup(&lease).unwrap();
    let done = store.read_job(ws, &job.id).unwrap();
    let artifact = done.artifact_id.unwrap();
    std::fs::read(
        root.join("exports")
            .join(ws)
            .join(format!("{artifact}.tar")),
    )
    .unwrap()
}

fn upload_once(fresh: &Core, bytes: &[u8], tag: &str) -> (String, String, String, String) {
    let landing = fresh
        .workspace_command(
            "create",
            json!({ "contractVersion": 1, "name": format!("Landing for {tag}") }),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let begin = fresh
        .job_command(
            "upload-begin",
            json!({"contractVersion":1,"workspaceId":landing,"bytes":bytes.len(),
                   "sha256":content::hash(bytes),"plaintextConsent":true}),
        )
        .unwrap();
    let mut job = Value::Null;
    for (index, chunk) in bytes.chunks(65536).enumerate() {
        let offset = index * 65536;
        job = fresh
            .job_command(
                "upload-part",
                json!({"contractVersion":1,"workspaceId":landing,"id":begin["id"],
                       "offset":offset,"bytes":chunk,
                       "finish":offset + chunk.len() == bytes.len()}),
            )
            .unwrap()["job"]
            .clone();
    }
    let preview = fresh
        .job_command(
            "preview",
            json!({"contractVersion":1,"workspaceId":landing,
                   "jobId":job["id"],"artifactId":job["artifactId"]}),
        )
        .unwrap();
    (
        landing,
        job["id"].as_str().unwrap().to_string(),
        job["artifactId"].as_str().unwrap().to_string(),
        preview["sha256"].as_str().unwrap().to_string(),
    )
}

fn try_upload_restore(fresh: &Core, bytes: &[u8], name: &str) -> Result<Value, CommandError> {
    let (landing, job, artifact, hash) = upload_once(fresh, bytes, name);
    fresh.job_command(
        "restore",
        json!({"contractVersion":1,"workspaceId":landing,
               "jobId":job,"artifactId":artifact,"name":name,
               "expectedHash":hash,"confirm":true}),
    )
}

#[test]
fn revised_package_restores_with_remapped_parent_spec_and_approval() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (spec_id, package_id) = spec_and_package(&f);
    let (run_id, polled) = revise_to_complete(
        &f,
        &package_id,
        1,
        "Update the heading and add a notes file.",
    );
    assert_eq!(polled["status"], "complete", "{polled}");
    let saved = f
        .core
        .ai_command(
            "forge-revise-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    assert_eq!(saved["artifact"]["version"], 2);

    let bytes = backup_bytes(
        f._dir.path(),
        &Storage::new(f._dir.path().to_path_buf()).unwrap(),
        &f.ws,
    );
    let fresh_dir = tempfile::tempdir().unwrap();
    let fresh = Core::with_storage_root(fresh_dir.path().to_path_buf());
    let restored = try_upload_restore(&fresh, &bytes, "Forge Revised Restored").unwrap();
    let restored_ws = restored["workspaces"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let projects = fresh
        .memory_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": restored_ws, "kind": "project" }),
        )
        .unwrap();
    let restored_project = projects["memories"][0]["id"].as_str().unwrap().to_string();
    let list = fresh
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": restored_ws, "projectId": restored_project }),
        )
        .unwrap();
    let by_type = |ty: &str| {
        list["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["type"] == ty)
            .unwrap()
            .clone()
    };
    let r_spec = by_type("specification");
    let r_spec_id = r_spec["artifactId"].as_str().unwrap().to_string();
    let r_package = by_type("source-package");
    let r_package_id = r_package["artifactId"].as_str().unwrap().to_string();
    assert_ne!(r_spec_id, spec_id);
    assert_ne!(r_package_id, package_id);
    // The restored v2 resolves its parent to the restored
    // package itself and its spec to the restored spec.
    let read = fresh
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": restored_ws, "artifactId": r_package_id, "version": 2 }),
        )
        .unwrap();
    let doc: Value = serde_json::from_str(read["body"].as_str().unwrap()).unwrap();
    assert_eq!(doc["parentPackageArtifactId"], r_package_id);
    assert_eq!(doc["parentPackageVersion"], 1);
    assert_eq!(doc["specArtifactId"], r_spec_id);
    assert_eq!(doc["specVersion"], 1);
    // v1 is still the original in the restored workspace.
    let read_v1 = fresh
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": restored_ws, "artifactId": r_package_id, "version": 1 }),
        )
        .unwrap();
    let doc_v1: Value = serde_json::from_str(read_v1["body"].as_str().unwrap()).unwrap();
    assert!(doc_v1.get("parentPackageArtifactId").is_none());
    // Approval for the pinned spec version transferred:
    // status is approved without re-approval, so a
    // further revision is possible in the restored
    // workspace (start is refused only for unapproved
    // specs; here we check status, not provider traffic).
    let status = fresh
        .ai_command(
            "forge-spec-approval-status",
            json!({ "contractVersion": 1, "workspaceId": restored_ws, "projectId": restored_project,
                    "specArtifactId": r_spec_id, "specVersion": 1 }),
        )
        .unwrap();
    assert_eq!(status["approved"], true);
}
