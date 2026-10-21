//! M6 Slice 4: opportunity set backup/restore identity.
//! A workspace backup restored into a fresh workspace must
//! keep the set's source relationships intact: memory
//! sources resolve to the RESTORED memories (the body ids
//! are remapped, the hash recomputed), supplied texts carry
//! over byte-identically, and the restored set can be
//! evaluated against the restored project with the existing
//! rubric workflow.

use anansi_core::{CommandError, Core, content, jobs::ExportSelection, storage::Storage};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn workspace(core: &Core, name: &str) -> String {
    core.workspace_command("create", json!({"contractVersion":1,"name":name}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn draft(title: &str, body: &str) -> Value {
    json!({ "title": title, "body": body, "privacy": "private",
            "eventDate": null, "validFrom": null, "validUntil": null,
            "confidence": null, "sourceReferences": [] })
}

fn concept(name: &str, refs: &[&str]) -> Value {
    json!({
        "name": name, "problem": "A problem.", "targetCustomer": "Cafes",
        "solution": "A tool.", "whyUseful": "Saves time.",
        "differentiationHypothesis": "A hypothesis.", "revenueModel": "Subscription.",
        "engineeringComplexity": "medium", "implementationCost": "3-5 weeks",
        "keyRisks": ["Adoption"], "requiredValidation": "Interviews.",
        "minimumViableVersion": "One workflow.",
        "sourceReferences": refs, "assumptions": ["Counts are weekly."]
    })
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

/// Upload a backup package once; returns (landing workspace,
/// job id, artifact id, package hash) for one or more
/// restore attempts against the same uploaded artifact.
fn upload_once(fresh: &Core, bytes: &[u8], tag: &str) -> (String, String, String, String) {
    let landing = workspace(fresh, &format!("Landing for {tag}"));
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

fn restore_job(
    fresh: &Core,
    landing: &str,
    job: &str,
    artifact: &str,
    hash: &str,
    name: &str,
) -> Result<Value, CommandError> {
    fresh.job_command(
        "restore",
        json!({"contractVersion":1,"workspaceId":landing,
               "jobId":job,"artifactId":artifact,"name":name,
               "expectedHash":hash,"confirm":true}),
    )
}

fn try_upload_restore(fresh: &Core, bytes: &[u8], name: &str) -> Result<Value, CommandError> {
    let (landing, job, artifact, hash) = upload_once(fresh, bytes, name);
    restore_job(fresh, &landing, &job, &artifact, &hash, name)
}

fn upload_restore(fresh: &Core, bytes: &[u8], name: &str) -> Value {
    try_upload_restore(fresh, bytes, name).unwrap()
}

fn read_memory(core: &Core, ws: &str, id: &str, revision: Option<&str>) -> Value {
    let mut input = json!({"contractVersion":1,"workspaceId":ws,"id":id});
    if let Some(revision) = revision {
        input["revisionId"] = json!(revision);
    }
    core.memory_command("read", input).unwrap()
}

fn revise_memory(core: &Core, ws: &str, id: &str, title: &str, body: &str) -> String {
    let current = read_memory(core, ws, id, None);
    core.memory_command(
        "revise",
        json!({"contractVersion":1,"workspaceId":ws,"id":id,
               "expectedRevision":current["memory"]["revisionId"],
               "draft":draft(title, body)}),
    )
    .unwrap()["memory"]["revisionId"]
        .as_str()
        .unwrap()
        .to_string()
}

fn find_memory(core: &Core, ws: &str, title: &str) -> String {
    let list = core
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":ws,"limit":100,"includeArchived":true}),
        )
        .unwrap();
    list["memories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["title"] == title)
        .unwrap_or_else(|| panic!("memory {title} not found in {ws}"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn restored_opportunity_set_resolves_sources_to_restored_memories() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let core = Core::with_storage_root(root.clone());
    let store = Storage::new(root.clone()).unwrap();
    let ws = workspace(&core, "Opportunity source");
    let project = core
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,"kind":"project","draft":draft("Harbor App","project")}),
        )
        .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let memory = core
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,"draft":draft("Field notes","Counts happen on Fridays.")}),
        )
        .unwrap();
    let memory_id = memory["memory"]["id"].as_str().unwrap().to_string();
    let revision_id = memory["memory"]["revisionId"].as_str().unwrap().to_string();

    // The opportunity set exactly as discovery saves it:
    // supplied text embedded, memory source by id + revision,
    // concepts citing both.
    let supplied_text = "Dock radios fail in the rain. The supplied-restore-phrase.";
    let body = json!({
        "format": "anansi-opportunity-set",
        "formatVersion": 1,
        "topic": "Dock radios fail in the rain.",
        "note": null,
        "sources": [
            { "sourceId": "supplied-1", "kind": "problem", "text": supplied_text },
            { "sourceId": memory_id, "kind": "memory", "label": "Field notes",
              "memoryId": memory_id, "revisionId": revision_id }
        ],
        "concepts": (0..5).map(|i| concept(&format!("Opp {i}"), &["supplied-1", &memory_id])).collect::<Vec<_>>()
    })
    .to_string();
    let registered = core
        .artifacts_command(
            "register",
            json!({"contractVersion":1,"workspaceId":ws,"projectId":project,
                   "type":"idea","title":"Opportunity set: Dock radios fail in the rain.",
                   "body":body,"sourceReferences":[format!("memory://{memory_id}")],
                   "privacy":"private"}),
        )
        .unwrap();
    let artifact_id = registered["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();

    // Back up the workspace and restore it into a fresh root.
    let bytes = backup_bytes(&root, &store, &ws);
    let fresh_dir = tempfile::tempdir().unwrap();
    let fresh = Core::with_storage_root(fresh_dir.path().to_path_buf());
    let restored = upload_restore(&fresh, &bytes, "Opportunity restored");
    let restored_ws = restored["workspaces"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let restored_memory = find_memory(&fresh, &restored_ws, "Field notes");
    let restored_project = find_memory(&fresh, &restored_ws, "Harbor App");
    assert_ne!(
        restored_memory, memory_id,
        "restore must assign fresh memory ids"
    );

    let artifacts = fresh
        .artifacts_command(
            "list",
            json!({"contractVersion":1,"workspaceId":restored_ws,"projectId":restored_project}),
        )
        .unwrap();
    let restored_artifact = artifacts["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["type"] == "idea")
        .expect("the opportunity set must be restored")
        .clone();
    assert_ne!(restored_artifact["artifactId"], artifact_id);
    // artifact_read verifies the content hash: a successful
    // read proves the recomputed hash matches the body.
    let read = fresh
        .artifacts_command(
            "read",
            json!({"contractVersion":1,"workspaceId":restored_ws,
                   "artifactId":restored_artifact["artifactId"],"version":1}),
        )
        .unwrap();
    let restored_body: Value = serde_json::from_str(read["body"].as_str().unwrap()).unwrap();
    // Memory source identity: remapped to the restored memory.
    assert_eq!(restored_body["sources"][1]["memoryId"], restored_memory);
    assert_eq!(restored_body["sources"][1]["sourceId"], restored_memory);
    // Exact-revision provenance: remapped to the restored
    // memory's (only) revision, never left pointing at the
    // original revision, and resolvable to the original text.
    let restored_revision =
        read_memory(&fresh, &restored_ws, &restored_memory, None)["memory"]["revisionId"]
            .as_str()
            .unwrap()
            .to_string();
    let body_revision = restored_body["sources"][1]["revisionId"]
        .as_str()
        .expect("the restored source must keep its pinned revision");
    assert_eq!(body_revision, restored_revision);
    assert_ne!(body_revision, revision_id);
    let resolved = read_memory(&fresh, &restored_ws, &restored_memory, Some(body_revision));
    assert_eq!(resolved["body"], "Counts happen on Fridays.");
    // Supplied source: byte-identical text, marker untouched.
    assert_eq!(restored_body["sources"][0]["sourceId"], "supplied-1");
    assert_eq!(restored_body["sources"][0]["text"], supplied_text);
    // Concept citations point at the restored memory.
    assert_eq!(
        restored_body["concepts"][0]["sourceReferences"],
        json!(["supplied-1", restored_memory])
    );

    // The restored set evaluates against the restored project
    // with the existing rubric workflow, resolving to the
    // restored source.
    let standard = fresh
        .artifacts_command(
            "evaluations-standard-rubric",
            json!({"contractVersion":1,"workspaceId":restored_ws,"projectId":restored_project}),
        )
        .unwrap();
    let rubric = fresh
        .artifacts_command(
            "register",
            json!({"contractVersion":1,"workspaceId":restored_ws,"projectId":restored_project,
                   "type":"rubric","title":"Standard evaluation rubric",
                   "body":standard["rubric"].to_string(),"sourceReferences":[],"privacy":"private"}),
        )
        .unwrap();
    let evaluation = fresh
        .artifacts_command(
            "evaluations-save",
            json!({"contractVersion":1,"workspaceId":restored_ws,"projectId":restored_project,
                   "sourceArtifactId":restored_artifact["artifactId"],"sourceVersion":1,
                   "rubricArtifactId":rubric["artifact"]["artifactId"],"rubricVersion":1,
                   "conceptIndices":[0],
                   "assessments":[{"conceptIndex":0,"dimension":"problem-clarity","score":5,"rationale":"Grounded in the supplied problem."}]}),
        )
        .unwrap();
    assert_eq!(
        evaluation["artifact"]["sourceReferences"][0],
        format!(
            "artifact://{}@v1",
            restored_artifact["artifactId"].as_str().unwrap()
        )
    );
}

// ---------------------------------------------------------------------------
// Exact-revision round trip (R17): a real discovery run pins
// the memory revision that was current at generation time;
// after a later revision and a backup/restore, the restored
// source must resolve — by restored memory id AND restored
// revision id — to exactly the text generation used.

struct Fake {
    base_url: String,
    last_body: Arc<Mutex<String>>,
}

fn start_fake() -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let last_body = Arc::new(Mutex::new(String::new()));
    let captured = last_body.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let captured = captured.clone();
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
                *captured.lock().unwrap() = String::from_utf8_lossy(&body).to_string();
                let concepts: Vec<Value> = (0..5)
                    .map(|i| {
                        json!({
                            "name": format!("Opportunity {i}"),
                            "problem": "Dock offices lose supplier quotes in email threads.",
                            "targetCustomer": "Small marina operators",
                            "solution": "A quote inbox with deadlines.",
                            "whyUseful": "It stops missed renewals.",
                            "differentiationHypothesis": "Hypothesis: built for shared inboxes.",
                            "revenueModel": "Subscription.",
                            "engineeringComplexity": "medium",
                            "implementationCost": "about 3-5 weeks for one developer",
                            "keyRisks": ["Adoption may be slow"],
                            "requiredValidation": "Interview five marina operators.",
                            "minimumViableVersion": "One inbox end to end.",
                            "sourceReferences": ["supplied-1"],
                            "assumptions": ["Quotes arrive mostly by email."]
                        })
                    })
                    .collect();
                let text = json!({ "concepts": concepts }).to_string();
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                );
                let line = format!(
                    "{{\"model\":\"llama-x\",\"message\":{{\"role\":\"assistant\",\"content\":{}}},\"done\":true,\"prompt_eval_count\":10,\"eval_count\":40}}\n",
                    serde_json::to_string(&text).unwrap()
                );
                let frame = format!("{:x}\r\n{}\r\n0\r\n\r\n", line.len(), line);
                let _ = stream.write_all(frame.as_bytes());
            });
        }
    });
    Fake {
        base_url: format!("http://{addr}"),
        last_body,
    }
}

fn configure_provider(core: &Core, ws: &str, fake: &Fake) {
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
}

#[test]
fn restored_opportunity_set_resolves_the_exact_historical_revision() {
    let fake = start_fake();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let core = Core::with_storage_root(root.clone());
    let store = Storage::new(root.clone()).unwrap();
    let ws = workspace(&core, "Revision provenance");
    let project = core
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,"kind":"project","draft":draft("Harbor App","project")}),
        )
        .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    configure_provider(&core, &ws, &fake);

    // A memory with multiple revisions: A, then B. Discovery
    // runs against B — the revision current at generation.
    let created = core
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,"draft":draft("Field notes","Revision A text, superseded before generation.")}),
        )
        .unwrap();
    let memory_id = created["memory"]["id"].as_str().unwrap().to_string();
    let generation_text = "The generation-revision-phrase marks revision B.";
    let revision_b = revise_memory(&core, &ws, &memory_id, "Field notes", generation_text);

    let started = core
        .ai_command(
            "opportunities-generate-start",
            json!({"contractVersion":1,"workspaceId":ws,"projectId":project,
                   "sources":[{"kind":"problem","text":"Dock offices lose supplier quotes."}],
                   "memoryIds":[memory_id]}),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let deadline = Instant::now() + Duration::from_secs(30);
    let saved = loop {
        let polled = core
            .ai_command(
                "opportunities-generate-poll",
                json!({"contractVersion":1,"workspaceId":ws,"runId":run_id}),
            )
            .unwrap();
        match polled["status"].as_str().unwrap() {
            "running" | "cancelling" => {
                assert!(Instant::now() < deadline, "discovery run timed out");
                std::thread::sleep(Duration::from_millis(50));
            }
            "complete" => {
                break core
                    .ai_command(
                        "opportunities-save",
                        json!({"contractVersion":1,"workspaceId":ws,"runId":run_id}),
                    )
                    .unwrap();
            }
            other => panic!("discovery run ended {other}: {:?}", polled["error"]),
        }
    };
    // Generation used revision B's text, not A's.
    let wire = fake.last_body.lock().unwrap().clone();
    assert!(wire.contains("generation-revision-phrase"), "{wire}");
    assert!(!wire.contains("Revision A text"), "{wire}");
    let artifact_id = saved["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    let original_read = core
        .artifacts_command(
            "read",
            json!({"contractVersion":1,"workspaceId":ws,"artifactId":artifact_id,"version":1}),
        )
        .unwrap();
    let original_body: Value =
        serde_json::from_str(original_read["body"].as_str().unwrap()).unwrap();
    assert_eq!(original_body["sources"][1]["revisionId"], revision_b);

    // The memory moves on to revision C after generation.
    let latest_text = "The post-generation-phrase marks revision C.";
    let revision_c = revise_memory(&core, &ws, &memory_id, "Field notes", latest_text);
    assert_ne!(revision_c, revision_b);

    // Backup and restore into a fresh root.
    let bytes = backup_bytes(&root, &store, &ws);
    let fresh_dir = tempfile::tempdir().unwrap();
    let fresh = Core::with_storage_root(fresh_dir.path().to_path_buf());
    let (landing, job, artifact, hash) = upload_once(&fresh, &bytes, "Revision restored");
    let restored = restore_job(
        &fresh,
        &landing,
        &job,
        &artifact,
        &hash,
        "Revision restored",
    )
    .unwrap();
    let restored_ws = restored["workspaces"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let restored_memory = find_memory(&fresh, &restored_ws, "Field notes");
    let restored_project = find_memory(&fresh, &restored_ws, "Harbor App");
    let artifacts = fresh
        .artifacts_command(
            "list",
            json!({"contractVersion":1,"workspaceId":restored_ws,"projectId":restored_project}),
        )
        .unwrap();
    let restored_artifact = artifacts["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["type"] == "idea")
        .expect("the opportunity set must be restored")
        .clone();
    let read = fresh
        .artifacts_command(
            "read",
            json!({"contractVersion":1,"workspaceId":restored_ws,
                   "artifactId":restored_artifact["artifactId"],"version":1}),
        )
        .unwrap();
    let restored_body: Value = serde_json::from_str(read["body"].as_str().unwrap()).unwrap();
    assert_eq!(restored_body["sources"][1]["memoryId"], restored_memory);
    let restored_revision = restored_body["sources"][1]["revisionId"]
        .as_str()
        .expect("the restored source must keep its pinned revision")
        .to_string();
    assert_ne!(
        restored_revision, revision_b,
        "the pinned revision must be remapped, not carried over"
    );
    // It is the HISTORICAL revision: the restored memory's
    // current revision is C's counterpart, a different id.
    let current = read_memory(&fresh, &restored_ws, &restored_memory, None);
    assert_ne!(current["memory"]["revisionId"], restored_revision);
    assert_eq!(current["body"], latest_text);
    // Resolution by (restored memory, restored revision)
    // returns exactly the text generation used — revision B,
    // not the latest revision.
    let resolved = read_memory(
        &fresh,
        &restored_ws,
        &restored_memory,
        Some(&restored_revision),
    );
    assert_eq!(resolved["body"], generation_text);

    // The original workspace is untouched: its set still pins
    // the original revision B, which still reads as B.
    let original_again = core
        .artifacts_command(
            "read",
            json!({"contractVersion":1,"workspaceId":ws,"artifactId":artifact_id,"version":1}),
        )
        .unwrap();
    let original_again_body: Value =
        serde_json::from_str(original_again["body"].as_str().unwrap()).unwrap();
    assert_eq!(original_again_body["sources"][1]["revisionId"], revision_b);
    assert_eq!(
        read_memory(&core, &ws, &memory_id, Some(&revision_b))["body"],
        generation_text
    );

    // Evaluation references still resolve on the restored set.
    let standard = fresh
        .artifacts_command(
            "evaluations-standard-rubric",
            json!({"contractVersion":1,"workspaceId":restored_ws,"projectId":restored_project}),
        )
        .unwrap();
    let rubric = fresh
        .artifacts_command(
            "register",
            json!({"contractVersion":1,"workspaceId":restored_ws,"projectId":restored_project,
                   "type":"rubric","title":"Standard evaluation rubric",
                   "body":standard["rubric"].to_string(),"sourceReferences":[],"privacy":"private"}),
        )
        .unwrap();
    let evaluation = fresh
        .artifacts_command(
            "evaluations-save",
            json!({"contractVersion":1,"workspaceId":restored_ws,"projectId":restored_project,
                   "sourceArtifactId":restored_artifact["artifactId"],"sourceVersion":1,
                   "rubricArtifactId":rubric["artifact"]["artifactId"],"rubricVersion":1,
                   "conceptIndices":[0],
                   "assessments":[{"conceptIndex":0,"dimension":"problem-clarity","score":5,"rationale":"Grounded in the supplied problem."}]}),
        )
        .unwrap();
    assert_eq!(
        evaluation["artifact"]["sourceReferences"][0],
        format!(
            "artifact://{}@v1",
            restored_artifact["artifactId"].as_str().unwrap()
        )
    );

    // Original identities survive through the restore
    // provenance mechanism (restore_origins): the same
    // uploaded backup cannot be restored twice into this
    // root, under any name.
    let err = restore_job(
        &fresh,
        &landing,
        &job,
        &artifact,
        &hash,
        "Revision restored again",
    )
    .unwrap_err();
    assert!(err.message.contains("already restored"), "{}", err.message);
}

// ---------------------------------------------------------------------------
// Missing and mismatched revision references fail the whole
// restore closed: no partial workspace is materialized.

/// Build a backup whose opportunity set cites the FIRST
/// memory with a revision chosen by `cite`: "first" (its own
/// revision — the control case), "second" (the second
/// memory's revision — a mismatch), or "missing" (an id that
/// is not in the package at all).
fn package_with_set(cite: &str) -> (tempfile::TempDir, Vec<u8>) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let core = Core::with_storage_root(root.clone());
    let store = Storage::new(root.clone()).unwrap();
    let ws = workspace(&core, "Revision refusal source");
    let project = core
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,"kind":"project","draft":draft("Harbor App","project")}),
        )
        .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let first = core
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,"draft":draft("Field notes","First memory text.")}),
        )
        .unwrap();
    let first_id = first["memory"]["id"].as_str().unwrap().to_string();
    let first_revision = first["memory"]["revisionId"].as_str().unwrap().to_string();
    let second = core
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,"draft":draft("Lab notes","Second memory text.")}),
        )
        .unwrap();
    let second_revision = second["memory"]["revisionId"].as_str().unwrap().to_string();
    let cited_revision = match cite {
        "first" => first_revision.clone(),
        "second" => second_revision.clone(),
        _ => "f".repeat(32),
    };
    let body = json!({
        "format": "anansi-opportunity-set",
        "formatVersion": 1,
        "topic": "Dock radios fail in the rain.",
        "note": null,
        "sources": [
            { "sourceId": "supplied-1", "kind": "problem", "text": "Dock radios fail in the rain." },
            { "sourceId": first_id, "kind": "memory", "label": "Field notes",
              "memoryId": first_id, "revisionId": cited_revision }
        ],
        "concepts": (0..5).map(|i| concept(&format!("Opp {i}"), &["supplied-1", &first_id])).collect::<Vec<_>>()
    })
    .to_string();
    core.artifacts_command(
        "register",
        json!({"contractVersion":1,"workspaceId":ws,"projectId":project,
               "type":"idea","title":"Opportunity set: Dock radios fail in the rain.",
               "body":body,"sourceReferences":[format!("memory://{first_id}")],
               "privacy":"private"}),
    )
    .unwrap();
    let bytes = backup_bytes(&root, &store, &ws);
    (dir, bytes)
}

#[test]
fn restore_refuses_missing_and_mismatched_revision_references() {
    // (a) The pinned revision is not in the package at all.
    let (_dir, bytes) = package_with_set("missing");
    let fresh_dir = tempfile::tempdir().unwrap();
    let fresh = Core::with_storage_root(fresh_dir.path().to_path_buf());
    let err = try_upload_restore(&fresh, &bytes, "Missing revision restore").unwrap_err();
    assert!(err.message.contains("revision"), "{}", err.message);
    let workspaces = fresh
        .workspace_command("list", json!({"contractVersion":1}))
        .unwrap();
    assert_eq!(
        workspaces["workspaces"].as_array().unwrap().len(),
        1,
        "a refused restore must not materialize a workspace"
    );

    // (b) The pinned revision belongs to a DIFFERENT memory
    // in the same workspace.
    let (_dir2, bytes2) = package_with_set("second");
    let fresh_dir2 = tempfile::tempdir().unwrap();
    let fresh2 = Core::with_storage_root(fresh_dir2.path().to_path_buf());
    let err = try_upload_restore(&fresh2, &bytes2, "Mismatched revision restore").unwrap_err();
    assert!(err.message.contains("revision"), "{}", err.message);
    let workspaces = fresh2
        .workspace_command("list", json!({"contractVersion":1}))
        .unwrap();
    assert_eq!(workspaces["workspaces"].as_array().unwrap().len(), 1);

    // Control: citing the memory's OWN revision restores
    // cleanly — the refusals above are about the reference,
    // not the package shape.
    let (_dir3, bytes3) = package_with_set("first");
    let fresh_dir3 = tempfile::tempdir().unwrap();
    let fresh3 = Core::with_storage_root(fresh_dir3.path().to_path_buf());
    let restored = try_upload_restore(&fresh3, &bytes3, "Own revision restore").unwrap();
    assert_eq!(restored["workspaces"].as_array().unwrap().len(), 1);
}
