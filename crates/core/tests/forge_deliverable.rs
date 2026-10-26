//! M7 Slice 3 integration: a version-pinned reviewable
//! export deliverable over the M5 stream substrate with a
//! synthetic Ollama provider. Criteria: manual save with
//! backend verified facts and deterministic export (source
//! bytes identical, manifest hashes match, no license or
//! deployment claims); model draft with origin tracking
//! and user edits; approval gate on the pinned
//! specification version for draft and save (no provider
//! traffic when unapproved); deliverable-docs/ collision
//! refusal; missing/stale references and documentation
//! revision conflicts fail closed; a deliverable never
//! follows a newer package version; restart persistence
//! and backup/fresh restore with remapped package/spec
//! references, rewritten pinned hashes and continued
//! exportability. Exported contents are never executed.

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

fn deliverable_json() -> String {
    let doc = |name: &str, text: &str| json!({ "name": name, "content": text });
    json!({ "documents": [
        doc("architecture.md", "Model architecture: index.html loads app.js; state lives in localStorage."),
        doc("usage.md", "Model usage: open index.html from the filesystem for review only."),
        doc("limitations.md", "Model limitations: unexecuted prototype; nothing built, tested or deployed."),
        doc("security.md", "Model security: no authentication; do not store secrets in this prototype."),
        doc("changes.md", "Model changes: first version; no parent."),
    ] })
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
    mode: Arc<Mutex<String>>,
    last_body: Arc<Mutex<String>>,
}

fn start_fake() -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let mode = Arc::new(Mutex::new("valid".to_string()));
    let last_body = Arc::new(Mutex::new(String::new()));
    let (m2, b2) = (mode.clone(), last_body.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (mode, last_body) = (m2.clone(), b2.clone());
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
                let is_deliverable = body_text.contains("technical writer");
                let is_revise = body_text.contains("Revision request");
                let is_build = body_text.contains("product engineer");
                let text = if is_deliverable {
                    if mode == "deliverable-malformed" {
                        "Not JSON at all.".to_string()
                    } else {
                        deliverable_json()
                    }
                } else if is_revise {
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
        mode,
        last_body,
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

fn manual_docs() -> Vec<Value> {
    vec![
        json!({ "name": "architecture.md", "content": "Architecture: index.html loads app.js; state in localStorage.", "origin": "user" }),
        json!({ "name": "usage.md", "content": "Usage: open index.html from the filesystem for review only.", "origin": "user" }),
        json!({ "name": "limitations.md", "content": "Limitations: unexecuted prototype; nothing built, tested or deployed.", "origin": "user" }),
        json!({ "name": "security.md", "content": "Security: no authentication; do not store secrets in this prototype.", "origin": "user" }),
        json!({ "name": "changes.md", "content": "Changes: first version; no parent.", "origin": "user" }),
    ]
}

fn save_deliverable(
    f: &Fixture,
    package_id: &str,
    version: u32,
    docs: Vec<Value>,
) -> Result<Value, CommandError> {
    f.core.ai_command(
        "forge-deliverable-save",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "packageArtifactId": package_id, "packageVersion": version,
            "documents": docs
        }),
    )
}

fn export_deliverable(f: &Fixture, deliverable_id: &str, version: u32) -> Value {
    f.core
        .artifacts_command(
            "forge-deliverable-export",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": deliverable_id, "version": version }),
        )
        .unwrap()
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

fn base64_decode(text: &str) -> Vec<u8> {
    let table = |c: u8| -> u8 {
        match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    };
    let clean: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let mut out = Vec::new();
    for chunk in clean.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let n = ((table(chunk[0]) as u32) << 18)
            | ((table(chunk[1]) as u32) << 12)
            | (if chunk.len() > 2 && chunk[2] != b'=' {
                (table(chunk[2]) as u32) << 6
            } else {
                0
            })
            | (if chunk.len() > 3 && chunk[3] != b'=' {
                table(chunk[3]) as u32
            } else {
                0
            });
        out.push((n >> 16) as u8);
        if chunk.len() > 2 && chunk[2] != b'=' {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 && chunk[3] != b'=' {
            out.push(n as u8);
        }
    }
    out
}

/// Parse a USTAR tarball built by `build_tar` (regular
/// files only, 512-byte headers) into (path, bytes) in
/// archive order. The contents are never executed.
fn parse_tar(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset + 512 <= bytes.len() {
        let header = &bytes[offset..offset + 512];
        if header.iter().all(|b| *b == 0) {
            break;
        }
        let name_end = header[..100].iter().position(|b| *b == 0).unwrap_or(100);
        let name = String::from_utf8_lossy(&header[..name_end]).to_string();
        let size_str = String::from_utf8_lossy(&header[124..136])
            .trim_matches(|c| c == '\0' || c == ' ')
            .to_string();
        let size = usize::from_str_radix(&size_str, 8).unwrap_or(0);
        let start = offset + 512;
        out.push((name, bytes[start..start + size].to_vec()));
        offset = start + size.div_ceil(512) * 512;
    }
    out
}

#[test]
fn manual_save_and_deterministic_export() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (spec_id, package_id) = spec_and_package(&f);
    let package_body = read_version(&f.core, &f.ws, &package_id, 1)["body"]
        .as_str()
        .unwrap()
        .to_string();
    let saved = save_deliverable(&f, &package_id, 1, manual_docs()).unwrap();
    let deliverable_id = saved["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(saved["artifact"]["version"], 1);
    let doc: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    assert_eq!(doc["format"], "anansi-deliverable");
    assert_eq!(doc["packageArtifactId"], package_id);
    assert_eq!(doc["packageVersion"], 1);
    assert_eq!(
        doc["packageContentHash"],
        content::hash(package_body.as_bytes())
    );
    assert_eq!(doc["specArtifactId"], spec_id);
    assert_eq!(doc["specVersion"], 1);
    assert_eq!(
        doc["status"],
        json!({ "executed": false, "built": false, "tested": false, "published": false, "productionReady": false })
    );
    assert_eq!(doc["documents"].as_array().unwrap().len(), 6);
    let facts = doc["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "verified-facts.md")
        .unwrap();
    assert_eq!(facts["origin"], "verified");
    assert!(facts["content"].as_str().unwrap().contains("index.html"));
    assert!(facts["content"].as_str().unwrap().contains("unexecuted"));

    let first = export_deliverable(&f, &deliverable_id, 1);
    let second = export_deliverable(&f, &deliverable_id, 1);
    assert_eq!(first["tarBase64"], second["tarBase64"]);
    assert_eq!(first["sha256"], second["sha256"]);
    assert_eq!(first["filename"], format!("{deliverable_id}-v1.tar"));
    let entries = parse_tar(&base64_decode(first["tarBase64"].as_str().unwrap()));
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "ANANSI-DELIVERABLE.json",
            "index.html",
            "app.js",
            "README.md",
            "deliverable-docs/architecture.md",
            "deliverable-docs/usage.md",
            "deliverable-docs/limitations.md",
            "deliverable-docs/security.md",
            "deliverable-docs/changes.md",
            "deliverable-docs/verified-facts.md",
        ]
    );
    let by_name = |name: &str| entries.iter().find(|(n, _)| n == name).unwrap().1.clone();
    // Source bytes are byte-identical to the package.
    let package_doc: Value = serde_json::from_str(&package_body).unwrap();
    for file in package_doc["files"].as_array().unwrap() {
        assert_eq!(
            by_name(file["path"].as_str().unwrap()),
            file["content"].as_str().unwrap().as_bytes()
        );
    }
    assert_eq!(
        by_name("deliverable-docs/usage.md"),
        b"Usage: open index.html from the filesystem for review only."
    );
    // The manifest hashes match the archive entries and
    // claim nothing about builds, tests, deployment or a
    // license.
    let manifest: Value = serde_json::from_slice(&by_name("ANANSI-DELIVERABLE.json")).unwrap();
    assert_eq!(manifest["format"], "anansi-deliverable-export");
    assert_eq!(manifest["packageArtifactId"], package_id);
    assert_eq!(manifest["packageVersion"], 1);
    assert_eq!(manifest["specArtifactId"], spec_id);
    assert_eq!(manifest["status"]["productionReady"], false);
    for entry in manifest["files"].as_array().unwrap() {
        assert_eq!(
            entry["sha256"],
            content::hash(&by_name(entry["path"].as_str().unwrap()))
        );
    }
    for entry in manifest["documents"].as_array().unwrap() {
        let path = format!("deliverable-docs/{}", entry["name"].as_str().unwrap());
        assert_eq!(entry["sha256"], content::hash(&by_name(&path)));
    }
    let manifest_text = String::from_utf8(by_name("ANANSI-DELIVERABLE.json")).unwrap();
    assert!(!manifest_text.contains("license"));
    assert!(!manifest_text.contains("deployed"));
}

#[test]
fn model_draft_edit_and_save() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (_spec_id, package_id) = spec_and_package(&f);
    let started = f
        .core
        .ai_command(
            "forge-deliverable-draft-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "packageArtifactId": package_id, "packageVersion": 1, "zeroCostLock": true
            }),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(&f, "forge-deliverable-draft-poll", &run_id);
    assert_eq!(polled["status"], "complete", "{polled}");
    let draft_docs = polled["deliverableDraft"].as_array().unwrap();
    assert_eq!(draft_docs.len(), 5);
    assert!(draft_docs.iter().all(|d| d["origin"] == "model"));
    assert!(
        draft_docs.iter().any(|d| d["name"] == "architecture.md"
            && d["content"].as_str().unwrap().contains("index.html"))
    );
    // Editing one suggestion flips its origin to user.
    let mut docs: Vec<Value> = draft_docs.to_vec();
    for doc in docs.iter_mut() {
        if doc["name"] == "usage.md" {
            doc["content"] = json!("Usage (edited by the user): open index.html for review only.");
            doc["origin"] = json!("user");
        }
    }
    let saved = save_deliverable(&f, &package_id, 1, docs).unwrap();
    let saved_doc: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    let usage = saved_doc["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "usage.md")
        .unwrap();
    assert_eq!(usage["origin"], "user");
    assert!(
        usage["content"]
            .as_str()
            .unwrap()
            .contains("edited by the user")
    );
    let architecture = saved_doc["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "architecture.md")
        .unwrap();
    assert_eq!(architecture["origin"], "model");

    // A malformed draft is invalid and saves nothing.
    set_mode(&fake, "deliverable-malformed");
    let started = f
        .core
        .ai_command(
            "forge-deliverable-draft-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "packageArtifactId": package_id, "packageVersion": 1, "zeroCostLock": true
            }),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(&f, "forge-deliverable-draft-poll", &run_id);
    assert_eq!(polled["status"], "invalid");
}

#[test]
fn approval_gate_refuses_draft_and_save_without_provider_traffic() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (spec_id, package_id) = spec_and_package(&f);
    // A second specification that was never approved, and
    // a package pinned to it: both draft and save must
    // refuse before any provider traffic.
    let spec_body = json!({
        "format": "anansi-product-spec", "formatVersion": 1, "template": "static-web-prototype",
        "sourceArtifactId": f.idea_artifact, "sourceVersion": 1, "conceptIndex": 0,
        "concept": { "name": "Bean Stock Log" },
        "answers": { "purpose": "p", "intendedUsers": "u", "mustHave": "m" },
        "spec": json!({ "title": "Other", "summary": "s", "purpose": "p", "intendedUsers": "u",
            "features": ["f"], "pages": ["index.html"], "nonGoals": [],
            "dataAndState": "d", "acceptanceCriteria": ["a"], "openQuestions": [] })
    })
    .to_string();
    let other_spec = f
        .core
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "type": "specification", "title": "Product specification: Other",
                "body": spec_body, "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let other_spec_id = other_spec["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    let manual_body = json!({
        "format": "anansi-source-package", "formatVersion": 1,
        "template": "static-web-prototype",
        "specArtifactId": other_spec_id, "specVersion": 1,
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
                "body": manual_body, "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let manual_id = manual["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    let _ = (spec_id, package_id);
    let wire_before = fake.last_body.lock().unwrap().clone();
    let draft = f.core.ai_command(
        "forge-deliverable-draft-start",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "packageArtifactId": manual_id, "packageVersion": 1, "zeroCostLock": true
        }),
    );
    assert!(
        draft.is_err(),
        "unapproved spec must refuse draft: {draft:?}"
    );
    let saved = save_deliverable(&f, &manual_id, 1, manual_docs());
    assert!(
        saved.is_err(),
        "unapproved spec must refuse save: {saved:?}"
    );
    assert_eq!(*fake.last_body.lock().unwrap(), wire_before);
}

#[test]
fn collision_missing_and_revision_conflict_fail_closed() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (spec_id, package_id) = spec_and_package(&f);
    // A package that already uses the documentation
    // namespace refuses a deliverable and stays unchanged.
    let colliding_body = json!({
        "format": "anansi-source-package", "formatVersion": 1,
        "template": "static-web-prototype",
        "specArtifactId": spec_id, "specVersion": 1,
        "sourceArtifactId": f.idea_artifact, "sourceVersion": 1,
        "conceptName": "Bean Stock Log",
        "plan": { "summary": "s", "steps": [], "buildNotes": "b", "testingNotes": "t" },
        "files": [
            { "path": "index.html", "content": "<title>Collision</title>" },
            { "path": "README.md", "content": "# Collision\n" },
            { "path": "deliverable-docs/notes.md", "content": "already here" }
        ],
        "limitations": []
    })
    .to_string();
    let colliding = f
        .core
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "type": "source-package", "title": "Source package: Collision",
                "body": colliding_body, "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap();
    let colliding_id = colliding["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    let refused = save_deliverable(&f, &colliding_id, 1, manual_docs());
    assert!(
        refused.is_err(),
        "namespace collision must refuse: {refused:?}"
    );
    assert_eq!(
        versions(&f.core, &f.ws, &colliding_id)["versions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // Missing package version, missing document, and a
    // client-supplied verified document all fail closed.
    assert!(save_deliverable(&f, &package_id, 99, manual_docs()).is_err());
    let mut missing = manual_docs();
    missing.pop();
    assert!(save_deliverable(&f, &package_id, 1, missing).is_err());
    let mut forged = manual_docs();
    forged.push(json!({ "name": "verified-facts.md", "content": "forged", "origin": "verified" }));
    assert!(save_deliverable(&f, &package_id, 1, forged).is_err());
    let mut wrong_origin = manual_docs();
    wrong_origin[0]["origin"] = json!("system");
    assert!(save_deliverable(&f, &package_id, 1, wrong_origin).is_err());

    // A documentation revision against a stale expected
    // version fails; the correct version saves v2 with
    // the pinned package unchanged.
    let saved = save_deliverable(&f, &package_id, 1, manual_docs()).unwrap();
    let deliverable_id = saved["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    let mut revised_docs = manual_docs();
    revised_docs[0]["content"] = json!("Architecture (revised): still index.html plus app.js.");
    let stale = f.core.ai_command(
        "forge-deliverable-save",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "packageArtifactId": package_id, "packageVersion": 1,
            "documents": revised_docs,
            "deliverableArtifactId": deliverable_id, "expectedVersion": 99
        }),
    );
    assert!(stale.is_err(), "stale revision must fail: {stale:?}");
    let revised = f
        .core
        .ai_command(
            "forge-deliverable-save",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "packageArtifactId": package_id, "packageVersion": 1,
                "documents": revised_docs,
                "deliverableArtifactId": deliverable_id, "expectedVersion": 1
            }),
        )
        .unwrap();
    assert_eq!(revised["artifact"]["version"], 2);
    let revised_doc: Value = serde_json::from_str(revised["body"].as_str().unwrap()).unwrap();
    assert_eq!(revised_doc["packageArtifactId"], package_id);
    assert_eq!(revised_doc["packageVersion"], 1);
    assert_eq!(
        revised_doc["documents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["name"] == "architecture.md")
            .unwrap()["content"],
        "Architecture (revised): still index.html plus app.js."
    );
    // A revision may not switch packages.
    let switched = f.core.ai_command(
        "forge-deliverable-save",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "packageArtifactId": colliding_id, "packageVersion": 1,
            "documents": manual_docs(),
            "deliverableArtifactId": deliverable_id, "expectedVersion": 2
        }),
    );
    assert!(switched.is_err());
}

#[test]
fn deliverable_does_not_follow_newer_package_version() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (_spec_id, package_id) = spec_and_package(&f);
    let saved = save_deliverable(&f, &package_id, 1, manual_docs()).unwrap();
    let deliverable_id = saved["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    // The package gains v2 through a revision; the v1
    // deliverable still names and exports v1's bytes.
    let started = f
        .core
        .ai_command(
            "forge-revise-start",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "packageArtifactId": package_id, "packageVersion": 1,
                "instructions": "Change something.", "zeroCostLock": true
            }),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let polled = poll_until(&f, "forge-revise-poll", &run_id);
    assert_eq!(polled["status"], "complete", "{polled}");
    let revised = f
        .core
        .ai_command(
            "forge-revise-save",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
        )
        .unwrap();
    assert_eq!(revised["artifact"]["version"], 2);
    let exported = export_deliverable(&f, &deliverable_id, 1);
    let entries = parse_tar(&base64_decode(exported["tarBase64"].as_str().unwrap()));
    let manifest: Value = serde_json::from_slice(
        &entries
            .iter()
            .find(|(n, _)| n == "ANANSI-DELIVERABLE.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(manifest["packageVersion"], 1);
    let index = entries
        .iter()
        .find(|(n, _)| n == "index.html")
        .unwrap()
        .1
        .clone();
    assert_eq!(
        index,
        b"<!doctype html><title>Bean Stock Log</title><script src=\"app.js\"></script>"
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
fn restart_and_fresh_restore_preserve_deliverable_and_export() {
    let fake = start_fake();
    let f = fixture(&fake);
    let (spec_id, package_id) = spec_and_package(&f);
    let saved = save_deliverable(&f, &package_id, 1, manual_docs()).unwrap();
    let deliverable_id = saved["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();

    // Restart: a new Core over the same storage root
    // reopens the deliverable and still exports it.
    let dir_path = f._dir.path().to_path_buf();
    let core2 = Core::with_storage_root(dir_path.clone());
    let reopened = read_version(&core2, &f.ws, &deliverable_id, 1);
    let reopened_doc: Value = serde_json::from_str(reopened["body"].as_str().unwrap()).unwrap();
    assert_eq!(reopened_doc["packageArtifactId"], package_id);
    let exported = core2
        .artifacts_command(
            "forge-deliverable-export",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": deliverable_id, "version": 1 }),
        )
        .unwrap();
    assert_eq!(exported["filename"], format!("{deliverable_id}-v1.tar"));

    // Fresh restore into a new workspace: references
    // remap, pinned hashes are rewritten, and the
    // restored deliverable still exports.
    let bytes = backup_bytes(
        f._dir.path(),
        &Storage::new(f._dir.path().to_path_buf()).unwrap(),
        &f.ws,
    );
    let fresh_dir = tempfile::tempdir().unwrap();
    let fresh = Core::with_storage_root(fresh_dir.path().to_path_buf());
    let restored = try_upload_restore(&fresh, &bytes, "Forge Deliverable Restored").unwrap();
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
    let r_deliverable = by_type("deliverable");
    let r_deliverable_id = r_deliverable["artifactId"].as_str().unwrap().to_string();
    assert_ne!(r_spec_id, spec_id);
    assert_ne!(r_package_id, package_id);
    assert_ne!(r_deliverable_id, deliverable_id);
    let r_read = read_version(&fresh, &restored_ws, &r_deliverable_id, 1);
    let r_doc: Value = serde_json::from_str(r_read["body"].as_str().unwrap()).unwrap();
    assert_eq!(r_doc["packageArtifactId"], r_package_id);
    assert_eq!(r_doc["specArtifactId"], r_spec_id);
    assert_eq!(r_doc["packageVersion"], 1);
    // The restored deliverable exports, and its manifest
    // names the restored ids — not the source workspace's.
    let r_exported = fresh
        .artifacts_command(
            "forge-deliverable-export",
            json!({ "contractVersion": 1, "workspaceId": restored_ws, "artifactId": r_deliverable_id, "version": 1 }),
        )
        .unwrap();
    let entries = parse_tar(&base64_decode(r_exported["tarBase64"].as_str().unwrap()));
    let manifest: Value = serde_json::from_slice(
        &entries
            .iter()
            .find(|(n, _)| n == "ANANSI-DELIVERABLE.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(manifest["packageArtifactId"], r_package_id);
    assert_eq!(manifest["specArtifactId"], r_spec_id);
    assert_eq!(manifest["deliverableArtifactId"], r_deliverable_id);
    // Approval transferred, so a new draft is possible in
    // the restored workspace.
    let status = fresh
        .ai_command(
            "forge-spec-approval-status",
            json!({ "contractVersion": 1, "workspaceId": restored_ws, "projectId": restored_project,
                    "specArtifactId": r_spec_id, "specVersion": 1 }),
        )
        .unwrap();
    assert_eq!(status["approved"], true);
}
