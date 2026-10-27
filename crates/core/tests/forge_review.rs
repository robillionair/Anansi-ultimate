//! M7 Slice 4 integration: a structured source review of a
//! saved source package against its pinned specification's
//! acceptance criteria, saved by a direct model-free op.
//! Criteria: exact-version pinning with recomputed hashes
//! and reopen after restart; strict finding validation
//! (verbatim criteria, statuses, note bounds, file
//! membership); reference failures fail closed (missing
//! version, cross-project package/spec, archived project);
//! saving a review needs no specification approval and
//! no provider, and creates no approval and no run;
//! review revision with optimistic concurrency and no
//! package/spec switching; a review never follows a
//! newer package version; selected findings flow through
//! the existing revision engine (approval gate, reject
//! saves nothing, accept saves a new version, stale
//! parents conflict); backup/fresh restore remaps pins
//! and hashes and refuses invalid originals before any
//! destination state is published. Reviewed contents are
//! never executed.

use anansi_core::Core;
use anansi_core::content;
use anansi_core::jobs::ExportSelection;
use anansi_core::readiness::CommandError;
use anansi_core::storage::Storage;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
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
            "summary": "One page, vanilla JS, localStorage, revised.",
            "steps": ["Update index.html heading", "Keep app.js logic"],
            "buildNotes": "No build: open index.html.",
            "testingNotes": "ANANSI has not run this code; check the acceptance criteria manually."
        },
        "files": [
            { "path": "index.html", "content": "<!doctype html><title>Bean Stock Log — revised</title><script src=\"app.js\"></script>" },
            { "path": "app.js", "content": "console.log('bean log revised');" },
            { "path": "README.md", "content": "# Bean Stock Log\nPrototype. Open index.html." }
        ]
    })
    .to_string()
}

fn ndjson_chunks(text: &str, chunks: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let step = chars.len().div_ceil(chunks).max(1);
    chars
        .chunks(step)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

struct Fake {
    base_url: String,
    last_body: Arc<Mutex<String>>,
}

fn start_fake() -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let last_body = Arc::new(Mutex::new(String::new()));
    let b2 = last_body.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let last_body = b2.clone();
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
                let is_revise = body_text.contains("Revision request");
                let is_build = body_text.contains("product engineer");
                let text = if is_revise {
                    revised_package_json()
                } else if is_build {
                    package_json()
                } else {
                    spec_json()
                };
                let chunks = ndjson_chunks(&text, 3);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                );
                for chunk in &chunks {
                    let line = format!(
                        "{{\"model\":\"llama-x\",\"message\":{{\"role\":\"assistant\",\"content\":{}}},\"done\":false}}\n",
                        serde_json::to_string(chunk).unwrap()
                    );
                    let frame = format!("{:x}\r\n{}\r\n", line.len(), line);
                    if stream.write_all(frame.as_bytes()).is_err() {
                        return;
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
        last_body,
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

fn spec_and_package(f: &Fixture, approve: bool) -> (String, String) {
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
    if approve {
        f.core
            .ai_command(
                "forge-spec-approve",
                json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                        "specArtifactId": spec_id, "specVersion": 1 }),
            )
            .unwrap();
    }
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

fn spec_doc_body(spec_id_source: &str) -> String {
    let spec: Value = serde_json::from_str(&spec_json()).unwrap();
    json!({
        "format": "anansi-product-spec", "formatVersion": 1,
        "template": "static-web-prototype",
        "sourceArtifactId": spec_id_source, "sourceVersion": 1,
        "conceptIndex": 0,
        "concept": { "name": "Bean Stock Log" },
        "answers": { "purpose": "Track bean stock.", "intendedUsers": "Cafe staff", "mustHave": "Add beans." },
        "spec": spec
    })
    .to_string()
}

fn package_doc_body(spec_id: &str) -> String {
    let package: Value = serde_json::from_str(&package_json()).unwrap();
    json!({
        "format": "anansi-source-package", "formatVersion": 1,
        "template": "static-web-prototype",
        "specArtifactId": spec_id, "specVersion": 1,
        "sourceArtifactId": "manual-source", "sourceVersion": 1,
        "conceptName": "Bean Stock Log",
        "plan": package["plan"],
        "files": package["files"],
        "limitations": ["Unexecuted prototype."]
    })
    .to_string()
}

struct Manual {
    _dir: tempfile::TempDir,
    core: Core,
    ws: String,
    project: String,
    spec_id: String,
    package_id: String,
}

/// A fixture with NO provider configured and NO
/// specification approval: the review path must work
/// on registry rows alone.
fn manual_fixture() -> Manual {
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
        .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let spec = core
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": ws, "projectId": project,
                "type": "specification", "title": "Specification: Bean Stock Log",
                "body": spec_doc_body("manual-source"),
                "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let spec_id = spec["artifact"]["artifactId"].as_str().unwrap().to_string();
    let package = core
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": ws, "projectId": project,
                "type": "source-package", "title": "Source package: Bean Stock Log",
                "body": package_doc_body(&spec_id),
                "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let package_id = package["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    Manual {
        _dir: dir,
        core,
        ws,
        project,
        spec_id,
        package_id,
    }
}

fn base_findings() -> Vec<Value> {
    vec![
        json!({ "criterionIndex": 0, "criterion": "Open index.html and add a bean", "status": "not-assessed", "note": "", "fileRefs": [] }),
        json!({ "criterionIndex": 1, "criterion": "Reload keeps entries", "status": "needs-changes", "note": "Reload loses the last entry; persist before unload in app.js.", "fileRefs": ["app.js"] }),
    ]
}

fn save_review(
    core: &Core,
    ws: &str,
    project: &str,
    package_id: &str,
    version: u32,
    findings: Vec<Value>,
) -> Result<Value, CommandError> {
    core.ai_command(
        "forge-review-save",
        json!({
            "contractVersion": 1, "workspaceId": ws, "projectId": project,
            "packageArtifactId": package_id, "packageVersion": version,
            "findings": findings
        }),
    )
}

fn read_version(core: &Core, ws: &str, artifact_id: &str, version: u32) -> Value {
    core.artifacts_command(
        "read",
        json!({ "contractVersion": 1, "workspaceId": ws, "artifactId": artifact_id, "version": version }),
    )
    .unwrap()
}

fn versions(core: &Core, ws: &str, artifact_id: &str) -> Value {
    core.artifacts_command(
        "versions",
        json!({ "contractVersion": 1, "workspaceId": ws, "artifactId": artifact_id }),
    )
    .unwrap()
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
fn manual_save_pins_exact_versions_and_reopens() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (spec_id, package_id) = spec_and_package(&f, true);
    let saved = save_review(&f.core, &f.ws, &f.project, &package_id, 1, base_findings()).unwrap();
    let review_id = saved["artifact"]["artifactId"].as_str().unwrap().to_string();
    assert_eq!(saved["artifact"]["type"], "source-review");
    let body: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["format"], "anansi-source-review");
    assert_eq!(body["formatVersion"], 1);
    assert_eq!(body["packageArtifactId"], package_id);
    assert_eq!(body["packageVersion"], 1);
    assert_eq!(body["specArtifactId"], spec_id);
    assert_eq!(body["specVersion"], 1);
    assert_eq!(body["conceptName"], "Bean Stock Log");
    assert_eq!(body["findings"], serde_json::to_value(base_findings()).unwrap());
    assert_eq!(
        body["limitations"],
        json!([
            "Inspection does not establish runtime behavior.",
            "Inspection does not establish passing tests.",
            "Inspection does not establish security certification.",
            "Inspection does not establish production readiness.",
            "The source was not executed, built, previewed or deployed."
        ])
    );
    // Pinned hashes equal the stored rows' hashes.
    let package_read = read_version(&f.core, &f.ws, &package_id, 1);
    assert_eq!(
        body["packageContentHash"],
        content::hash(package_read["body"].as_str().unwrap().as_bytes())
    );
    let spec_read = read_version(&f.core, &f.ws, &spec_id, 1);
    assert_eq!(
        body["specContentHash"],
        spec_read["artifact"]["contentHash"]
    );
    // Reopen after restart reads the same record.
    let core2 = Core::with_storage_root(f._dir.path().to_path_buf());
    let reopened = read_version(&core2, &f.ws, &review_id, 1);
    assert_eq!(reopened["body"], saved["body"]);
}

#[test]
fn findings_validation_fails_closed() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (_, package_id) = spec_and_package(&f, true);
    let mut wrong_text = base_findings();
    wrong_text[0]["criterion"] = json!("Open index.html and delete a bean");
    assert!(save_review(&f.core, &f.ws, &f.project, &package_id, 1, wrong_text).is_err());
    let mut missing = base_findings();
    missing.pop();
    assert!(save_review(&f.core, &f.ws, &f.project, &package_id, 1, missing).is_err());
    let mut extra = base_findings();
    extra.push(json!({ "criterionIndex": 2, "criterion": "Extra", "status": "satisfied", "note": "", "fileRefs": [] }));
    assert!(save_review(&f.core, &f.ws, &f.project, &package_id, 1, extra).is_err());
    let mut bad_status = base_findings();
    bad_status[0]["status"] = json!("passed");
    assert!(save_review(&f.core, &f.ws, &f.project, &package_id, 1, bad_status).is_err());
    let mut long_note = base_findings();
    long_note[0]["note"] = json!("x".repeat(2001));
    assert!(save_review(&f.core, &f.ws, &f.project, &package_id, 1, long_note).is_err());
    let mut empty_change = base_findings();
    empty_change[1]["note"] = json!("   ");
    assert!(save_review(&f.core, &f.ws, &f.project, &package_id, 1, empty_change).is_err());
    let mut bad_file = base_findings();
    bad_file[1]["fileRefs"] = json!(["missing.js"]);
    assert!(save_review(&f.core, &f.ws, &f.project, &package_id, 1, bad_file).is_err());
    let mut dup_file = base_findings();
    dup_file[1]["fileRefs"] = json!(["app.js", "app.js"]);
    assert!(save_review(&f.core, &f.ws, &f.project, &package_id, 1, dup_file).is_err());
    // Unknown fields are denied at parse.
    let denied = f.core.ai_command(
        "forge-review-save",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "packageArtifactId": package_id, "packageVersion": 1,
            "findings": base_findings(), "bogus": true
        }),
    );
    assert!(denied.is_err(), "unknown fields must be denied: {denied:?}");
    // Nothing was saved by any refusal: no review artifact exists.
    let list = f
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project }),
        )
        .unwrap();
    assert!(
        !list["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["type"] == "source-review"),
        "a refused review was saved"
    );
}

#[test]
fn references_fail_closed() {
    let m = manual_fixture();
    // Nonexistent package version.
    assert!(save_review(&m.core, &m.ws, &m.project, &m.package_id, 99, base_findings()).is_err());
    // Package from another project.
    let other_project = m
        .core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": m.ws, "kind": "project", "draft": draft("Other App", "project") }),
        )
        .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let other_package = m
        .core
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": m.ws, "projectId": other_project,
                "type": "source-package", "title": "Source package: Other",
                "body": package_doc_body(&m.spec_id),
                "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let other_package_id = other_package["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        save_review(&m.core, &m.ws, &m.project, &other_package_id, 1, base_findings()).is_err(),
        "cross-project package must be refused"
    );
    // Package whose pinned specification belongs to another project.
    let other_spec = m
        .core
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": m.ws, "projectId": other_project,
                "type": "specification", "title": "Specification: Other",
                "body": spec_doc_body("manual-source"),
                "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let other_spec_id = other_spec["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    let cross_package = m
        .core
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": m.ws, "projectId": m.project,
                "type": "source-package", "title": "Source package: Cross",
                "body": package_doc_body(&other_spec_id),
                "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let cross_package_id = cross_package["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        save_review(&m.core, &m.ws, &m.project, &cross_package_id, 1, base_findings()).is_err(),
        "cross-project specification must be refused"
    );
    // Archived projects refuse.
    let read = m
        .core
        .memory_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": m.ws, "id": m.project }),
        )
        .unwrap();
    let rev = read["memory"]["revisionId"].as_str().unwrap().to_string();
    m.core
        .memory_command(
            "revise",
            json!({ "contractVersion": 1, "workspaceId": m.ws, "id": m.project,
                    "expectedRevision": rev,
                    "draft": draft("Harbor App", "project"),
                    "lifecycle": { "state": "archived", "confirmed": true, "successorId": null } }),
        )
        .unwrap();
    assert!(
        save_review(&m.core, &m.ws, &m.project, &m.package_id, 1, base_findings()).is_err(),
        "archived project must refuse the review save"
    );
}

#[test]
fn save_without_approval_or_provider_creates_nothing() {
    let m = manual_fixture();
    let saved = save_review(&m.core, &m.ws, &m.project, &m.package_id, 1, base_findings()).unwrap();
    assert_eq!(saved["artifact"]["type"], "source-review");
    // No specification approval was created or required.
    let status = m
        .core
        .ai_command(
            "forge-spec-approval-status",
            json!({ "contractVersion": 1, "workspaceId": m.ws, "projectId": m.project,
                    "specArtifactId": m.spec_id, "specVersion": 1 }),
        )
        .unwrap();
    assert_eq!(status["approved"], false);
    let list = m
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": m.ws, "projectId": m.project }),
        )
        .unwrap();
    assert!(
        !list["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["type"] == "specification-approval"),
        "saving a review created an approval"
    );
}

#[test]
fn review_revision_and_stale_conflict_fail_closed() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (_, package_id) = spec_and_package(&f, true);
    let saved = save_review(&f.core, &f.ws, &f.project, &package_id, 1, base_findings()).unwrap();
    let review_id = saved["artifact"]["artifactId"].as_str().unwrap().to_string();
    let mut revised_findings = base_findings();
    revised_findings[0]["status"] = json!("satisfied");
    revised_findings[0]["note"] = json!("Add flow works when opened from the filesystem.");
    let revised = f
        .core
        .ai_command(
            "forge-review-save",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "packageArtifactId": package_id, "packageVersion": 1,
                "findings": revised_findings,
                "reviewArtifactId": review_id, "expectedVersion": 1
            }),
        )
        .unwrap();
    assert_eq!(revised["artifact"]["version"], 2);
    let v2: Value = serde_json::from_str(revised["body"].as_str().unwrap()).unwrap();
    assert_eq!(v2["packageArtifactId"], package_id);
    assert_eq!(v2["packageVersion"], 1);
    // A revision may not switch packages.
    let (_, other_package) = spec_and_package(&f, true);
    let switched = f.core.ai_command(
        "forge-review-save",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "packageArtifactId": other_package, "packageVersion": 1,
            "findings": base_findings(),
            "reviewArtifactId": review_id, "expectedVersion": 2
        }),
    );
    assert!(switched.is_err(), "review revision switched packages: {switched:?}");
    // A stale expected version conflicts.
    let stale = f.core.ai_command(
        "forge-review-save",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "packageArtifactId": package_id, "packageVersion": 1,
            "findings": base_findings(),
            "reviewArtifactId": review_id, "expectedVersion": 1
        }),
    );
    assert!(stale.is_err(), "stale review revision must conflict: {stale:?}");
    assert_eq!(
        versions(&f.core, &f.ws, &review_id)["versions"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "refused revisions created versions"
    );
}

#[test]
fn review_does_not_follow_newer_package_version() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (_, package_id) = spec_and_package(&f, true);
    let saved = save_review(&f.core, &f.ws, &f.project, &package_id, 1, base_findings()).unwrap();
    let review_id = saved["artifact"]["artifactId"].as_str().unwrap().to_string();
    // A new package version, via the generic revise path.
    let v1 = read_version(&f.core, &f.ws, &package_id, 1);
    let mut v2_body: Value = serde_json::from_str(v1["body"].as_str().unwrap()).unwrap();
    v2_body["files"][1]["content"] = json!("console.log('bean log v2');");
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "artifactId": package_id,
                "expectedVersion": 1, "title": "Source package: Bean Stock Log",
                "body": v2_body.to_string(), "sourceReferences": []
            }),
        )
        .unwrap();
    let reopened = read_version(&f.core, &f.ws, &review_id, 1);
    let doc: Value = serde_json::from_str(reopened["body"].as_str().unwrap()).unwrap();
    assert_eq!(doc["packageVersion"], 1);
    assert_eq!(doc["packageArtifactId"], package_id);
    // A review of v2 pins v2 independently.
    let saved_v2 = save_review(&f.core, &f.ws, &f.project, &package_id, 2, base_findings()).unwrap();
    let doc_v2: Value = serde_json::from_str(saved_v2["body"].as_str().unwrap()).unwrap();
    assert_eq!(doc_v2["packageVersion"], 2);
}

#[test]
fn findings_to_revision_accept_and_unapproved_gate() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (_, package_id) = spec_and_package(&f, true);
    let saved = save_review(&f.core, &f.ws, &f.project, &package_id, 1, base_findings()).unwrap();
    let _review_id = saved["artifact"]["artifactId"].as_str().unwrap().to_string();
    // Instructions composed from the selected finding,
    // as the panel composes them; submission uses the
    // unchanged revision flow.
    let instructions = "Revision requested from source review of Bean Stock Log (package version 1):\n- Criterion: Reload keeps entries\n  Status: needs-changes\n  Note: Reload loses the last entry; persist before unload in app.js.\n  Files: app.js";
    let started = f
        .core
        .ai_command(
            "forge-revise-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "packageArtifactId": package_id, "packageVersion": 1,
                "instructions": instructions, "zeroCostLock": true
            }),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(&f, "forge-revise-poll", &run_id);
    assert_eq!(polled["status"], "complete", "{polled}");
    assert!(polled["revisionDiff"].is_object(), "no server-computed diff: {polled}");
    let saved_rev = f
        .core
        .ai_command(
            "forge-revise-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    assert_eq!(saved_rev["artifact"]["version"], 2);
    // The original package version is untouched.
    let v1 = read_version(&f.core, &f.ws, &package_id, 1);
    assert!(v1["body"].as_str().unwrap().contains("bean log');"));
    let v2 = read_version(&f.core, &f.ws, &package_id, 2);
    assert!(v2["body"].as_str().unwrap().contains("bean log revised"));
    // The approval gate still refuses an unapproved spec
    // before any provider traffic: a manually registered
    // package (spec never approved) with a provider
    // configured, so a dispatch would be observable.
    let m2 = manual_fixture();
    m2.core
        .ai_command(
            "providers-configure",
            json!({
                "workspaceId": m2.ws,
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
    *fake.last_body.lock().unwrap() = String::new();
    let refused = m2.core.ai_command(
        "forge-revise-start",
        json!({
            "contractVersion": 1, "workspaceId": m2.ws, "projectId": m2.project,
            "packageArtifactId": m2.package_id, "packageVersion": 1,
            "instructions": instructions, "zeroCostLock": true
        }),
    );
    assert!(refused.is_err(), "unapproved revision must be refused: {refused:?}");
    assert!(
        !fake.last_body.lock().unwrap().contains("Revision request"),
        "refused revision sent provider traffic"
    );
}

#[test]
fn restart_and_fresh_restore_preserve_review() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (spec_id, package_id) = spec_and_package(&f, true);
    let saved = save_review(&f.core, &f.ws, &f.project, &package_id, 1, base_findings()).unwrap();
    let review_id = saved["artifact"]["artifactId"].as_str().unwrap().to_string();
    // Restart reopens the review at its pinned version.
    let core2 = Core::with_storage_root(f._dir.path().to_path_buf());
    let reopened = read_version(&core2, &f.ws, &review_id, 1);
    assert_eq!(reopened["body"], saved["body"]);
    let bytes = backup_bytes(
        f._dir.path(),
        &Storage::new(f._dir.path().to_path_buf()).unwrap(),
        &f.ws,
    );
    let fresh_dir = tempfile::tempdir().unwrap();
    let fresh = Core::with_storage_root(fresh_dir.path().to_path_buf());
    let restored = try_upload_restore(&fresh, &bytes, "Forge Review Restored").unwrap();
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
    let r_spec_id = by_type("specification")["artifactId"].as_str().unwrap().to_string();
    let r_package_id = by_type("source-package")["artifactId"].as_str().unwrap().to_string();
    let r_review_id = by_type("source-review")["artifactId"].as_str().unwrap().to_string();
    assert_ne!(r_spec_id, spec_id);
    assert_ne!(r_package_id, package_id);
    assert_ne!(r_review_id, review_id);
    let r_read = read_version(&fresh, &restored_ws, &r_review_id, 1);
    let r_doc: Value = serde_json::from_str(r_read["body"].as_str().unwrap()).unwrap();
    assert_eq!(r_doc["packageArtifactId"], r_package_id);
    assert_eq!(r_doc["specArtifactId"], r_spec_id);
    assert_eq!(r_doc["packageVersion"], 1);
    assert_eq!(r_doc["specVersion"], 1);
    // Pinned hashes were rewritten to the restored rows.
    let r_package = read_version(&fresh, &restored_ws, &r_package_id, 1);
    assert_eq!(
        r_doc["packageContentHash"],
        content::hash(r_package["body"].as_str().unwrap().as_bytes())
    );
    let r_spec = read_version(&fresh, &restored_ws, &r_spec_id, 1);
    assert_eq!(r_doc["specContentHash"], r_spec["artifact"]["contentHash"]);
    // Findings survived the remap unchanged.
    assert_eq!(r_doc["findings"], serde_json::to_value(base_findings()).unwrap());
}

#[test]
fn restore_refuses_tampered_pinned_hash() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (_, package_id) = spec_and_package(&f, true);
    let saved = save_review(&f.core, &f.ws, &f.project, &package_id, 1, base_findings()).unwrap();
    let id = saved["artifact"]["artifactId"].as_str().unwrap();
    let mut edited: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    edited["packageContentHash"] = json!("0".repeat(64));
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "artifactId": id,
                "expectedVersion": 1, "title": "Source review: tampered pin",
                "body": edited.to_string(), "sourceReferences": []
            }),
        )
        .unwrap();
    let bytes = backup_bytes(
        f._dir.path(),
        &Storage::new(f._dir.path().to_path_buf()).unwrap(),
        &f.ws,
    );
    let fresh_dir = tempfile::tempdir().unwrap();
    let fresh = Core::with_storage_root(fresh_dir.path().to_path_buf());
    let result = try_upload_restore(&fresh, &bytes, "Invalid pin review");
    assert!(
        result.is_err(),
        "restore silently repaired an invalid original pin: {result:?}"
    );
    let no_restored_workspace = |fresh: &Core| {
        let listed = fresh
            .workspace_command("list", json!({ "contractVersion": 1 }))
            .unwrap();
        !listed["workspaces"].as_array().unwrap().iter().any(|w| {
            w["name"] == "Invalid pin review" || w["name"] == "Invalid pin review again"
        })
    };
    assert!(
        no_restored_workspace(&fresh),
        "failed restore published a destination workspace"
    );
    let second = try_upload_restore(&fresh, &bytes, "Invalid pin review again");
    assert!(
        second.is_err(),
        "second restore attempt succeeded: {second:?}"
    );
    assert!(
        no_restored_workspace(&fresh),
        "second failed restore published a destination workspace"
    );
    assert_eq!(
        versions(&f.core, &f.ws, id)["versions"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "source review versions changed during the refused restore"
    );
}

#[test]
fn restore_refuses_cross_workspace_spec() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (_, package_id) = spec_and_package(&f, true);
    let saved = save_review(&f.core, &f.ws, &f.project, &package_id, 1, base_findings()).unwrap();
    let id = saved["artifact"]["artifactId"].as_str().unwrap().to_string();
    // A specification in a different workspace.
    let other_dir = tempfile::tempdir().unwrap();
    let other = Core::with_storage_root(other_dir.path().to_path_buf());
    let other_ws = other
        .workspace_command("create", json!({ "contractVersion": 1, "name": "Other" }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let other_project = other
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": other_ws, "kind": "project", "draft": draft("Other App", "project") }),
        )
        .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let other_spec = other
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": other_ws, "projectId": other_project,
                "type": "specification", "title": "Specification: Other",
                "body": spec_doc_body("manual-source"),
                "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let other_spec_id = other_spec["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    // Rewrite the review to point at that foreign spec.
    let mut edited: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    edited["specArtifactId"] = json!(other_spec_id);
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "artifactId": id,
                "expectedVersion": 1, "title": "Source review: foreign spec",
                "body": edited.to_string(), "sourceReferences": []
            }),
        )
        .unwrap();
    let bytes = backup_bytes(
        f._dir.path(),
        &Storage::new(f._dir.path().to_path_buf()).unwrap(),
        &f.ws,
    );
    let fresh_dir = tempfile::tempdir().unwrap();
    let fresh = Core::with_storage_root(fresh_dir.path().to_path_buf());
    let result = try_upload_restore(&fresh, &bytes, "Foreign spec review");
    assert!(
        result.is_err(),
        "restore accepted a cross-workspace specification pin: {result:?}"
    );
    let listed = fresh
        .workspace_command("list", json!({ "contractVersion": 1 }))
        .unwrap();
    assert!(
        !listed["workspaces"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["name"] == "Foreign spec review"),
        "failed restore published a destination workspace"
    );
}
