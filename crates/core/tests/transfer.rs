//! M8 Slice 2 integration: explicitly approved
//! LEXI-COLLIDER → Product Forge transfer, model-free.
//! Criteria: nothing is created or contacted before
//! explicit approval; the concept pins the exact
//! experiment version + recomputed hash, the selected
//! combination verbatim, and the approved brief exactly
//! as submitted, with model-derived and user-authored
//! material apart; forged/stale versions, tampered
//! experiments and cross-project/workspace sources are
//! refused; identical resubmission — concurrent or
//! after restart — creates no duplicate; a newer
//! experiment version neither changes an existing
//! transfer nor inherits its approval; the transferred
//! concept enters the existing Forge spec workflow
//! (index 0 only) while creating no specification
//! approval; restart/reopen and backup/fresh restore
//! preserve valid relationships and remap identities,
//! and invalid references refuse before any destination
//! publication. No generated code, execution, or real
//! provider is involved.

use anansi_core::Core;
use anansi_core::content;
use anansi_core::jobs::ExportSelection;
use anansi_core::readiness::CommandError;
use anansi_core::storage::Storage;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

fn collision_json() -> Value {
    json!({
        "sourceUnderstanding": "Beam A describes high-frequency trading: fast signals, order books, latency arbitrage. Beam B describes mycelium networks: distributed hyphal signaling, local nutrient routing, resilience through redundancy. Hypothesis: both are decentralized coordination systems with local feedback.",
        "conceptualPrimitives": ["Distributed signaling", "Adaptive routing", "Local feedback", "Decentralized coordination", "Resource optimization"],
        "crossDomainMappings": ["Order-book depth resembles nutrient gradients: both steer flow toward scarcity.", "Latency arbitrage contrasts with mycelial patience: speed versus resilience is the core tension.", "Redundant hyphal paths transfer to redundant trade routes that degrade gracefully."],
        "combinations": [
            { "name": "Mycelial order router", "description": "A routing layer that scores venues by local congestion signals, like hyphae choosing paths by nutrient gradients. Hypothesis only." },
            { "name": "Graceful-degradation mesh", "description": "A trade mesh that reroutes through redundant paths when a venue stalls, accepting slower fills to avoid total outages. Hypothesis only." }
        ],
        "feasibility": {
            "hypotheses": ["Local congestion signals can rank venues without a central coordinator.", "Redundancy costs are bounded by capping parallel routes."],
            "dependencies": ["A venue feed with latency and fill data", "A simulator for routing policies"],
            "unknownCosts": ["Exchange fees under rerouting are unknown.", "Compute cost of continuous signal scoring is unknown."],
            "risks": ["Stale signals could route into worse fills.", "Regulatory treatment of adaptive routing is unvalidated."],
            "testability": "Testable in simulation first: replay historical venue data through the router and compare fill quality against a static baseline."
        },
        "productization": {
            "conceptBrief": "A routing advisor that proposes venue choices from local signals. Hypothesis, not a product claim.",
            "architecture": "Signal ingest, local scoring, policy engine, simulation harness, audit log. No execution of generated code.",
            "roadmap": "Phase 1: simulator and scoring prototype on paper. Phase 2: replay evaluation. Phase 3: human-reviewed pilot proposal.",
            "testPlan": "Unit-test the scorer on synthetic gradients; replay-test the router; red-team stale-signal behavior.",
            "prototypeProposal": "A paper prototype plus a simulator notebook, reviewed by a human before any live use."
        }
    })
}

fn spec_json() -> String {
    json!({
        "title": "Mycelial order router",
        "summary": "A routing advisor that proposes venue choices from local signals.",
        "purpose": "Route trade flow through venues by local congestion signals.",
        "intendedUsers": "Small trading desks",
        "features": ["Score venues by local congestion", "Reroute when a venue stalls", "Show the chosen route with its reason"],
        "pages": ["index.html"],
        "nonGoals": ["No live trading", "No accounts"],
        "dataAndState": "Signals live in memory only.",
        "acceptanceCriteria": ["Open index.html and see venue scores", "A stalled venue reroutes"],
        "openQuestions": ["Which venues first?"]
    })
    .to_string()
}

fn experiment_body() -> String {
    json!({
        "format": "anansi-collider-experiment",
        "version": 1,
        "beamA": "Trading signals.",
        "beamB": "Mycelium networks.",
        "controls": { "creativity": "balanced", "technicalDepth": "standard", "practicality": "balanced", "innovationDistance": "moderate", "budget": "", "preferredOutputType": "concept" },
        "result": collision_json(),
        "provider": "ollama",
        "model": "llama-x",
        "disclosure": {
            "provider": "ollama", "model": "llama-x", "endpointHost": "127.0.0.1",
            "local": true, "routingMode": "auto", "routingReason": "test",
            "messageCount": 3, "segmentCount": 4, "redactionCount": 0,
            "estimatedInputTokens": 100, "costClass": "free", "usage": null, "attempts": []
        }
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

/// Fake Ollama endpoint serving only the Forge spec
/// response: the transfer itself needs no provider.
struct Fake {
    base_url: String,
    hits: Arc<AtomicUsize>,
}

fn start_fake() -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let h2 = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let hits = h2.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                        content_length = v.trim().parse().unwrap_or(0);
                    }
                    if line.trim().is_empty() {
                        break;
                    }
                }
                let mut body = vec![0u8; content_length];
                reader.read_exact(&mut body).unwrap_or(());
                hits.fetch_add(1, Ordering::SeqCst);
                let text = spec_json();
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                );
                for chunk in ndjson_chunks(&text, 3) {
                    let line = format!(
                        "{{\"model\":\"llama-x\",\"message\":{{\"role\":\"assistant\",\"content\":{}}},\"done\":false}}\n",
                        serde_json::to_string(&chunk).unwrap()
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
        hits,
    }
}

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
}

fn fixture(fake: &Fake) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let ws = core
        .workspace_command(
            "create",
            json!({ "contractVersion": 1, "name": "Transfer" }),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let project = core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": ws, "kind": "project", "draft": draft("Transfer Project", "project") }),
        )
        .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string();
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
    }
}

fn register_experiment(f: &Fixture, body: &str) -> String {
    f.core
        .artifacts_command(
            "register",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "type": "experiment", "title": "Collider experiment: Trading signals",
                "body": body, "sourceReferences": [], "privacy": "private"
            }),
        )
        .unwrap()["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string()
}

fn transfer(f: &Fixture, experiment: &str, version: u32, index: usize, brief: &str) -> Value {
    f.core
        .ai_command(
            "collider-transfer",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
                "experimentArtifactId": experiment, "experimentVersion": version,
                "combinationIndex": index, "approvedBrief": brief
            }),
        )
        .unwrap()
}

fn list_concepts(f: &Fixture) -> Vec<Value> {
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
        .filter(|a| a["type"] == "concept")
        .cloned()
        .collect()
}

fn read_artifact(f: &Fixture, artifact_id: &str, version: u32) -> Value {
    f.core
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": artifact_id, "version": version }),
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
fn transfer_creates_concept_with_exact_pins_and_provenance() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    assert!(
        list_concepts(&f).is_empty(),
        "no concept exists before approval"
    );
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);

    let brief = "Mycelial order router, edited by the user: score venues by local congestion, reroute on stall.";
    let transferred = transfer(&f, &experiment, 1, 1, brief);
    assert_eq!(transferred["alreadyTransferred"], false);
    let concept_id = transferred["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    let body: Value = serde_json::from_str(transferred["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["format"], "anansi-forge-concept");
    assert_eq!(body["version"], 1);
    assert_eq!(body["sourceExperimentArtifactId"], experiment);
    assert_eq!(body["sourceExperimentVersion"], 1);
    assert_eq!(
        body["sourceExperimentContentHash"],
        content::hash(experiment_body().as_bytes())
    );
    assert_eq!(body["selectedCombinationIndex"], 1);
    assert_eq!(
        body["selectedCombination"]["name"],
        "Graceful-degradation mesh"
    );
    assert_eq!(
        body["selectedCombination"]["description"],
        "A trade mesh that reroutes through redundant paths when a venue stalls, accepting slower fills to avoid total outages. Hypothesis only."
    );
    assert_eq!(body["approvedBrief"], brief);
    assert_eq!(
        body["assumptions"],
        json!([
            "Local congestion signals can rank venues without a central coordinator.",
            "Redundancy costs are bounded by capping parallel routes."
        ])
    );
    assert_eq!(
        body["unknowns"],
        json!([
            "Exchange fees under rerouting are unknown.",
            "Compute cost of continuous signal scoring is unknown.",
            "A venue feed with latency and fill data",
            "A simulator for routing policies"
        ])
    );
    assert_eq!(
        body["productizationBrief"],
        "A routing advisor that proposes venue choices from local signals. Hypothesis, not a product claim."
    );
    assert_eq!(body["provider"], "ollama");
    assert_eq!(body["model"], "llama-x");
    assert_eq!(transferred["artifact"]["type"], "concept");
    assert_eq!(
        fake.hits.load(Ordering::SeqCst),
        0,
        "transfer contacts no provider"
    );

    // The stored row matches the returned body exactly.
    let stored = read_artifact(&f, &concept_id, 1);
    assert_eq!(stored["body"], transferred["body"]);
    assert_eq!(
        stored["artifact"]["contentHash"],
        content::hash(transferred["body"].as_str().unwrap().as_bytes())
    );
}

#[test]
fn acceptance_approved_brief_is_preserved_exactly() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let brief = "  Keep the user's leading indentation.\nAnd the final newline.\n";
    let saved = transfer(&f, &experiment, 1, 0, brief);
    let body: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    assert_eq!(
        body["approvedBrief"], brief,
        "approval must preserve submitted text exactly"
    );
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);
}

#[test]
fn acceptance_forge_rechecks_transfer_binding_before_provider_traffic() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let saved = transfer(&f, &experiment, 1, 0, "Use the approved combination.");
    let id = saved["artifact"]["artifactId"].as_str().unwrap();
    let mut edited: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    edited["sourceExperimentArtifactId"] = json!("f".repeat(32));
    edited["selectedCombination"]["description"] = json!("Fabricated model-derived result.");
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "artifactId": id,
                "expectedVersion": 1, "title": "Concept: edited provenance",
                "body": edited.to_string(), "sourceReferences": []
            }),
        )
        .unwrap();
    let started = f.core.ai_command(
        "forge-spec-start",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "sourceArtifactId": id, "sourceVersion": 2, "conceptIndex": 0,
            "purpose": "Route flow.", "intendedUsers": "Desks.",
            "mustHave": "Score venues.", "zeroCostLock": true
        }),
    );
    let hits = fake.hits.load(Ordering::SeqCst);
    assert!(
        started.is_err() && hits == 0,
        "invalid transfer provenance reached Forge: {started:?}; provider hits={hits}"
    );
}

#[test]
fn acceptance_generic_edit_does_not_mint_transfer_approval() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let saved = transfer(
        &f,
        &experiment,
        1,
        0,
        "Approved through the transfer operation.",
    );
    let id = saved["artifact"]["artifactId"].as_str().unwrap();
    let mut edited: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    edited["approvedBrief"] = json!("Unapproved replacement saved through the generic editor.");
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "artifactId": id,
                "expectedVersion": 1, "title": "Concept: unapproved replacement",
                "body": edited.to_string(), "sourceReferences": []
            }),
        )
        .unwrap();
    let started = f.core.ai_command(
        "forge-spec-start",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "sourceArtifactId": id, "sourceVersion": 2, "conceptIndex": 0,
            "purpose": "Route flow.", "intendedUsers": "Desks.",
            "mustHave": "Score venues.", "zeroCostLock": true
        }),
    );
    let hits = fake.hits.load(Ordering::SeqCst);
    assert!(
        started.is_err() && hits == 0,
        "a generic edit minted transfer approval: {started:?}; provider hits={hits}"
    );
}

#[test]
fn acceptance_restore_rejects_fabricated_model_derived_provenance() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let saved = transfer(&f, &experiment, 1, 0, "Use the approved combination.");
    let id = saved["artifact"]["artifactId"].as_str().unwrap();
    let mut edited: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    edited["assumptions"] = json!(["This assumption never appeared in the pinned experiment."]);
    edited["provider"] = json!("fabricated-provider");
    edited["model"] = json!("fabricated-model");
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "artifactId": id,
                "expectedVersion": 1, "title": "Concept: edited provenance",
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
    let restored = try_upload_restore(&fresh, &bytes, "Forged transfer provenance");
    assert!(
        restored.is_err(),
        "restore published fabricated model provenance: {restored:?}"
    );
    let listed = fresh
        .workspace_command("list", json!({"contractVersion": 1}))
        .unwrap();
    assert!(
        !listed["workspaces"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["name"] == "Forged transfer provenance")
    );
}

#[test]
fn acceptance_retry_after_generic_revision_keeps_original_transfer_identity() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let brief = "Original approved brief.";
    let first = transfer(&f, &experiment, 1, 0, brief);
    let id = first["artifact"]["artifactId"].as_str().unwrap();
    let mut edited: Value = serde_json::from_str(first["body"].as_str().unwrap()).unwrap();
    edited["approvedBrief"] = json!("Different text in a supported generic revision.");
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "artifactId": id,
                "expectedVersion": 1, "title": "Concept: edited brief",
                "body": edited.to_string(), "sourceReferences": []
            }),
        )
        .unwrap();
    let retried = transfer(&f, &experiment, 1, 0, brief);
    assert_eq!(
        retried["artifact"]["artifactId"], id,
        "a retry must not duplicate an existing transfer"
    );
    assert_eq!(list_concepts(&f).len(), 1);
}

#[test]
fn receipt_revision_leaves_original_approval_intact() {
    // Only the version-1 receipt confers approval: a
    // generic revision of the receipt creates a later
    // version the Forge lookup ignores, so editing a
    // receipt can neither revoke the original approval
    // nor transfer it to another concept version.
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let saved = transfer(&f, &experiment, 1, 0, "Use the approved combination.");
    let concept_id = saved["artifact"]["artifactId"].as_str().unwrap();
    let list = f
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project }),
        )
        .unwrap();
    let receipt = list["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["type"] == "transfer-approval")
        .expect("transfer creates a receipt");
    let receipt_id = receipt["artifactId"].as_str().unwrap();
    let read = f
        .core
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": receipt_id, "version": 1 }),
        )
        .unwrap();
    let mut edited: Value = serde_json::from_str(read["body"].as_str().unwrap()).unwrap();
    edited["approvedBrief"] = json!("Edited receipt text that must not matter.");
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "artifactId": receipt_id,
                "expectedVersion": 1, "title": "Transfer approval: edited",
                "body": edited.to_string(), "sourceReferences": []
            }),
        )
        .unwrap();
    let started = f.core.ai_command(
        "forge-spec-start",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "sourceArtifactId": concept_id, "sourceVersion": 1, "conceptIndex": 0,
            "purpose": "Route flow.", "intendedUsers": "Desks.",
            "mustHave": "Score venues.", "zeroCostLock": true
        }),
    );
    assert!(
        started.is_ok(),
        "revising a receipt must not revoke the original approval: {started:?}"
    );
}

#[test]
fn generic_registration_of_receipt_type_is_refused() {
    // Receipts are backend rows, not a registrable
    // type: the type list deliberately excludes them,
    // so no hand-written row can mint approval.
    let fake = start_fake();
    let f = fixture(&fake);
    let registered = f.core.artifacts_command(
        "register",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "type": "transfer-approval", "title": "Forged receipt",
            "body": "{\"format\":\"anansi-transfer-approval\"}",
            "sourceReferences": [], "privacy": "private"
        }),
    );
    assert!(
        registered.is_err(),
        "generic registration must not create a receipt: {registered:?}"
    );
}

#[test]
fn corrupt_receipt_hash_refuses_retry_and_forge_before_provider_traffic() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let brief = "Use the approved combination.";
    let saved = transfer(&f, &experiment, 1, 0, brief);
    let concept_id = saved["artifact"]["artifactId"].as_str().unwrap();
    let list = f
        .core
        .artifacts_command(
            "list",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project
            }),
        )
        .unwrap();
    let receipt_id = list["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["type"] == "transfer-approval")
        .unwrap()["artifactId"]
        .as_str()
        .unwrap();
    // Synthetic catalog corruption: leave the body and all approval bindings
    // intact but invalidate the receipt's own stored checksum.
    let db = rusqlite::Connection::open(f._dir.path().join("workspaces.sqlite3")).unwrap();
    assert_eq!(db.execute(
        "UPDATE artifact_versions SET content_hash = ?1 WHERE workspace_id = ?2 AND artifact_id = ?3 AND version = 1",
        rusqlite::params!["0".repeat(64), f.ws, receipt_id],
    ).unwrap(), 1);
    drop(db);
    assert!(f.core.artifacts_command("read", json!({
        "contractVersion": 1, "workspaceId": f.ws, "artifactId": receipt_id, "version": 1
    })).is_err(), "ordinary reads must detect the invalid checksum");
    let retried = f.core.ai_command(
        "collider-transfer",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "experimentArtifactId": experiment, "experimentVersion": 1,
            "combinationIndex": 0, "approvedBrief": brief
        }),
    );
    let started = f.core.ai_command(
        "forge-spec-start",
        json!({
            "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
            "sourceArtifactId": concept_id, "sourceVersion": 1, "conceptIndex": 0,
            "purpose": "Route flow.", "intendedUsers": "Desks.",
            "mustHave": "Score venues.", "zeroCostLock": true
        }),
    );
    let hits = fake.hits.load(Ordering::SeqCst);
    assert!(
        retried.is_err() && started.is_err() && hits == 0,
        "invalid receipt checksum was trusted: retry={retried:?}; start={started:?}; hits={hits}"
    );
}

#[test]
fn retry_after_generic_revision_returns_original_snapshot() {
    // Stronger than identity alone: the retry must
    // return the original approved snapshot, not the
    // latest generic revision's altered content
    // presented as the original approval.
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let brief = "Original approved brief.";
    let first = transfer(&f, &experiment, 1, 0, brief);
    let id = first["artifact"]["artifactId"].as_str().unwrap();
    let mut edited: Value = serde_json::from_str(first["body"].as_str().unwrap()).unwrap();
    edited["approvedBrief"] = json!("Different text in a supported generic revision.");
    f.core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": f.ws, "artifactId": id,
                "expectedVersion": 1, "title": "Concept: edited brief",
                "body": edited.to_string(), "sourceReferences": []
            }),
        )
        .unwrap();
    let retried = transfer(&f, &experiment, 1, 0, brief);
    assert_eq!(retried["artifact"]["artifactId"], id);
    assert_eq!(retried["alreadyTransferred"], true);
    let body: Value = serde_json::from_str(retried["body"].as_str().unwrap()).unwrap();
    assert_eq!(
        body["approvedBrief"], brief,
        "retry must return the original approved snapshot"
    );
    assert_eq!(list_concepts(&f).len(), 1);
}

#[test]
fn forged_stale_and_cross_project_sources_are_refused() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let brief = "A brief.";

    // Stale: no such version.
    assert!(f.core.ai_command(
        "collider-transfer",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project, "experimentArtifactId": experiment, "experimentVersion": 99, "combinationIndex": 0, "approvedBrief": brief }),
    ).is_err());
    // Wrong type: an idea artifact is not an experiment.
    let idea = f.core.artifacts_command(
        "register",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project, "type": "idea", "title": "Idea", "body": "{}", "sourceReferences": [], "privacy": "private" }),
    ).unwrap()["artifact"]["artifactId"].as_str().unwrap().to_string();
    assert!(f.core.ai_command(
        "collider-transfer",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project, "experimentArtifactId": idea, "experimentVersion": 1, "combinationIndex": 0, "approvedBrief": brief }),
    ).is_err());
    // Cross-project: the experiment lives in another project.
    let other = f.core.memory_command(
        "create",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "kind": "project", "draft": draft("Other Project", "project") }),
    ).unwrap()["memory"]["id"].as_str().unwrap().to_string();
    assert!(f.core.ai_command(
        "collider-transfer",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": other, "experimentArtifactId": experiment, "experimentVersion": 1, "combinationIndex": 0, "approvedBrief": brief }),
    ).is_err());
    // Tampered experiment: a generic revise breaks the collider format.
    let mut tampered: Value = serde_json::from_str(&experiment_body()).unwrap();
    tampered["result"]["combinations"] = json!([]);
    let revision = read_artifact(&f, &experiment, 1)["artifact"]["version"]
        .as_u64()
        .unwrap() as u32;
    f.core.artifacts_command(
        "revise",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": experiment, "expectedVersion": revision, "title": "Collider experiment: Trading signals", "body": tampered.to_string(), "sourceReferences": [] }),
    ).unwrap();
    assert!(f.core.ai_command(
        "collider-transfer",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project, "experimentArtifactId": experiment, "experimentVersion": 2, "combinationIndex": 0, "approvedBrief": brief }),
    ).is_err());
    // The valid pinned v1 still transfers after the tamper created v2.
    let ok = transfer(&f, &experiment, 1, 0, brief);
    assert_eq!(ok["alreadyTransferred"], false);
    assert!(!list_concepts(&f).is_empty());
}

#[test]
fn invalid_selection_and_bounds_are_refused_before_creation() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let base = json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project, "experimentArtifactId": experiment, "experimentVersion": 1, "approvedBrief": "A brief." });
    // Out-of-range combination.
    let mut req = base.clone();
    req["combinationIndex"] = json!(2);
    assert!(f.core.ai_command("collider-transfer", req).is_err());
    // Empty brief.
    let mut req = base.clone();
    req["combinationIndex"] = json!(0);
    req["approvedBrief"] = json!("   ");
    assert!(f.core.ai_command("collider-transfer", req).is_err());
    // Overlong brief.
    let mut req = base.clone();
    req["combinationIndex"] = json!(0);
    req["approvedBrief"] = json!("x".repeat(4001));
    assert!(f.core.ai_command("collider-transfer", req).is_err());
    // Unknown field refused.
    let mut req = base.clone();
    req["combinationIndex"] = json!(0);
    req["surprise"] = json!("not part of the contract");
    assert!(f.core.ai_command("collider-transfer", req).is_err());
    assert!(list_concepts(&f).is_empty(), "no refusal created a concept");
}

#[test]
fn duplicate_concurrent_and_restart_retry_create_no_duplicates() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let brief = "The same approved brief.";
    let first = transfer(&f, &experiment, 1, 0, brief);
    let first_id = first["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    // Sequential repeat.
    let second = transfer(&f, &experiment, 1, 0, brief);
    assert_eq!(second["alreadyTransferred"], true);
    assert_eq!(second["artifact"]["artifactId"], first_id);
    assert_eq!(list_concepts(&f).len(), 1);
    // Concurrent identical submissions collapse to the same artifact.
    std::thread::scope(|s| {
        let mut handles = Vec::new();
        for _ in 0..8 {
            handles.push(s.spawn(|| {
                f.core
                    .ai_command(
                        "collider-transfer",
                        json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project, "experimentArtifactId": experiment, "experimentVersion": 1, "combinationIndex": 0, "approvedBrief": brief }),
                    )
                    .unwrap()
            }));
        }
        for h in handles {
            let got = h.join().unwrap();
            assert_eq!(got["artifact"]["artifactId"], first_id);
            assert_eq!(got["alreadyTransferred"], true);
        }
    });
    assert_eq!(list_concepts(&f).len(), 1);
    // Retry after restart (a fresh Core handle on the same root).
    let root = f._dir.path().to_path_buf();
    let reopened = Core::with_storage_root(root);
    let retry = reopened
        .ai_command(
            "collider-transfer",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project, "experimentArtifactId": experiment, "experimentVersion": 1, "combinationIndex": 0, "approvedBrief": brief }),
        )
        .unwrap();
    assert_eq!(retry["alreadyTransferred"], true);
    assert_eq!(retry["artifact"]["artifactId"], first_id);
    assert_eq!(list_concepts(&f).len(), 1);
    // A different brief is a different transfer, not a silent merge.
    let other = transfer(&f, &experiment, 1, 0, "A different approved brief.");
    assert_eq!(other["alreadyTransferred"], false);
    assert_ne!(other["artifact"]["artifactId"], first_id);
    assert_eq!(list_concepts(&f).len(), 2);
}

#[test]
fn newer_experiment_version_neither_changes_nor_inherits_approval() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let transferred = transfer(&f, &experiment, 1, 0, "Brief for v1.");
    let concept_id = transferred["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    // Revise the experiment to v2 with a changed combination.
    let mut v2: Value = serde_json::from_str(&experiment_body()).unwrap();
    v2["result"]["combinations"][0]["name"] = json!("Renamed router");
    f.core.artifacts_command(
        "revise",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": experiment, "expectedVersion": 1, "title": "Collider experiment: Trading signals", "body": v2.to_string(), "sourceReferences": [] }),
    ).unwrap();
    // The existing transfer still pins v1 and the original name.
    let stored = read_artifact(&f, &concept_id, 1);
    let body: Value = serde_json::from_str(stored["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["sourceExperimentVersion"], 1);
    assert_eq!(body["selectedCombination"]["name"], "Mycelial order router");
    // Transferring v2 needs its own approval and yields its own concept.
    let v2_transfer = transfer(&f, &experiment, 2, 0, "Brief for v2.");
    assert_eq!(v2_transfer["alreadyTransferred"], false);
    assert_ne!(v2_transfer["artifact"]["artifactId"], concept_id);
    let v2_body: Value = serde_json::from_str(v2_transfer["body"].as_str().unwrap()).unwrap();
    assert_eq!(v2_body["sourceExperimentVersion"], 2);
    assert_eq!(v2_body["selectedCombination"]["name"], "Renamed router");
    assert_eq!(list_concepts(&f).len(), 2);
}

#[test]
fn transferred_concept_enters_forge_spec_workflow_without_spec_approval() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let transferred = transfer(&f, &experiment, 1, 0, "Route venues by local congestion.");
    let concept_id = transferred["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    // No specification approval exists after transfer alone.
    let approvals = f
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project }),
        )
        .unwrap()["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["type"] == "specification-approval" || a["type"] == "specification")
        .count();
    assert_eq!(approvals, 0);
    // Index 1 is refused: a transferred concept has exactly one concept.
    assert!(f.core.ai_command(
        "forge-spec-start",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project, "sourceArtifactId": concept_id, "sourceVersion": 1, "conceptIndex": 1, "purpose": "Route flow.", "intendedUsers": "Desks.", "mustHave": "Score venues.", "zeroCostLock": true }),
    ).is_err());
    // Index 0 runs the existing spec workflow against the fake provider.
    let started = f.core.ai_command(
        "forge-spec-start",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project, "sourceArtifactId": concept_id, "sourceVersion": 1, "conceptIndex": 0, "purpose": "Route flow.", "intendedUsers": "Desks.", "mustHave": "Score venues.", "zeroCostLock": true }),
    ).unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let deadline = Instant::now() + Duration::from_secs(30);
    let polled = loop {
        let polled = f
            .core
            .ai_command(
                "forge-spec-poll",
                json!({ "contractVersion": 1, "workspaceId": f.ws, "runId": run_id }),
            )
            .unwrap();
        let status = polled["status"].as_str().unwrap().to_string();
        if matches!(
            status.as_str(),
            "complete" | "invalid" | "failed" | "cancelled"
        ) || Instant::now() > deadline
        {
            break polled;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
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
    let spec = read_artifact(&f, &spec_id, 1);
    let spec_body: Value = serde_json::from_str(spec["body"].as_str().unwrap()).unwrap();
    assert_eq!(spec_body["sourceArtifactId"], concept_id);
    assert_eq!(spec_body["sourceVersion"], 1);
    // Saving a spec still does not approve it.
    let approvals = f
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project }),
        )
        .unwrap()["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["type"] == "specification-approval")
        .count();
    assert_eq!(approvals, 0);
    assert!(
        fake.hits.load(Ordering::SeqCst) >= 1,
        "spec generation used the provider"
    );
}

#[test]
fn restart_and_fresh_restore_preserve_transfer() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let transferred = transfer(&f, &experiment, 1, 0, "Route venues by local congestion.");
    let concept_id = transferred["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    // Restart: a fresh Core handle reopens the concept unchanged.
    let root = f._dir.path().to_path_buf();
    let reopened = Core::with_storage_root(root.clone());
    let stored = reopened.artifacts_command(
        "read",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": concept_id, "version": 1 }),
    ).unwrap();
    assert_eq!(stored["body"], transferred["body"]);
    // Fresh restore into a new workspace.
    let store = Storage::new(root.clone()).unwrap();
    let bytes = backup_bytes(&root, &store, &f.ws);
    let dir2 = tempfile::tempdir().unwrap();
    let fresh = Core::with_storage_root(dir2.path().to_path_buf());
    let restored = try_upload_restore(&fresh, &bytes, "transfer").unwrap();
    let restored_ws = restored["workspaces"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(restored_ws, f.ws);
    let projects = fresh
        .memory_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": restored_ws, "kind": "project" }),
        )
        .unwrap();
    let restored_project = projects["memories"][0]["id"].as_str().unwrap().to_string();
    let list = fresh.artifacts_command(
        "list",
        json!({ "contractVersion": 1, "workspaceId": restored_ws, "projectId": restored_project }),
    ).unwrap();
    let artifacts = list["artifacts"].as_array().unwrap();
    let restored_experiment = artifacts
        .iter()
        .find(|a| a["type"] == "experiment")
        .unwrap();
    let restored_concept = artifacts.iter().find(|a| a["type"] == "concept").unwrap();
    assert_ne!(restored_experiment["artifactId"], experiment);
    assert_ne!(restored_concept["artifactId"], concept_id);
    let concept_body: Value = serde_json::from_str(
        fresh.artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": restored_ws, "artifactId": restored_concept["artifactId"].as_str().unwrap(), "version": 1 }),
        ).unwrap()["body"].as_str().unwrap(),
    ).unwrap();
    assert_eq!(
        concept_body["sourceExperimentArtifactId"],
        restored_experiment["artifactId"]
    );
    assert_eq!(concept_body["sourceExperimentVersion"], 1);
    assert_eq!(
        concept_body["sourceExperimentContentHash"],
        content::hash(experiment_body().as_bytes())
    );
    assert_eq!(
        concept_body["approvedBrief"],
        "Route venues by local congestion."
    );
    assert_eq!(
        concept_body["selectedCombination"]["name"],
        "Mycelial order router"
    );
}

#[test]
fn restore_refuses_tampered_transfer_without_destination_mutation() {
    let fake = start_fake();
    let f = fixture(&fake);
    let experiment = register_experiment(&f, &experiment_body());
    let transferred = transfer(&f, &experiment, 1, 0, "Route venues by local congestion.");
    let concept_id = transferred["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    let root = f._dir.path().to_path_buf();
    let store = Storage::new(root.clone()).unwrap();
    let good_bytes = backup_bytes(&root, &store, &f.ws);
    // Tamper: rewrite the pinned source hash in the transfer body.
    let mut tampered: Value = serde_json::from_str(transferred["body"].as_str().unwrap()).unwrap();
    tampered["sourceExperimentContentHash"] = json!("0".repeat(64));
    f.core.artifacts_command(
        "revise",
        json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": concept_id, "expectedVersion": 1, "title": "Concept: Mycelial order router", "body": tampered.to_string(), "sourceReferences": [] }),
    ).unwrap();
    let bad_bytes = backup_bytes(&root, &store, &f.ws);
    // Good catalog restores.
    let dir_good = tempfile::tempdir().unwrap();
    let fresh_good = Core::with_storage_root(dir_good.path().to_path_buf());
    assert!(try_upload_restore(&fresh_good, &good_bytes, "good").is_ok());
    // Tampered catalog refuses twice on the same fresh core and
    // publishes no destination workspace named for the restore.
    let dir_bad = tempfile::tempdir().unwrap();
    let fresh_bad = Core::with_storage_root(dir_bad.path().to_path_buf());
    assert!(try_upload_restore(&fresh_bad, &bad_bytes, "bad").is_err());
    assert!(try_upload_restore(&fresh_bad, &bad_bytes, "bad-again").is_err());
    let listed = fresh_bad
        .workspace_command("list", json!({ "contractVersion": 1 }))
        .unwrap();
    assert!(
        !listed["workspaces"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["name"] == "bad" || w["name"] == "bad-again"),
        "failed restore published a destination workspace"
    );
    // The source transfer is untouched by the refused attempts.
    let stored = read_artifact(&f, &concept_id, 2);
    let body: Value = serde_json::from_str(stored["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["sourceExperimentContentHash"], "0".repeat(64));
}

#[test]
fn context_experiment_transfer_survives_restore_and_stays_forge_admissible() {
    // M8 Slice 3: a context-bearing experiment's body
    // changes on restore (its memory ids remap), so its
    // hash changes. The concept and receipt pinned
    // experiment hashes must follow the restored row,
    // or Forge admission on the restored workspace
    // would fail closed on a valid transfer.
    let fake = start_fake();
    let f = fixture(&fake);
    let memory = f
        .core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "draft": draft("Tide notes", "Revision B tide observations.") }),
        )
        .unwrap();
    let mem_id = memory["memory"]["id"].as_str().unwrap().to_string();
    let mem_rev = memory["memory"]["revisionId"].as_str().unwrap().to_string();
    let mut body: Value = serde_json::from_str(&experiment_body()).unwrap();
    body["memoryContext"] = json!([{
        "memoryId": mem_id,
        "revisionId": mem_rev,
        "version": 1,
        "title": "Tide notes",
        "content": "Revision B tide observations.",
        "contentHash": content::hash("Revision B tide observations.".as_bytes()),
        "privacy": "private"
    }]);
    let experiment = register_experiment(&f, &body.to_string());
    let _ = transfer(&f, &experiment, 1, 0, "Route venues by local congestion.");
    let root = f._dir.path().to_path_buf();
    let store = Storage::new(root.clone()).unwrap();
    let bytes = backup_bytes(&root, &store, &f.ws);
    let dir2 = tempfile::tempdir().unwrap();
    let fresh = Core::with_storage_root(dir2.path().to_path_buf());
    let restored = try_upload_restore(&fresh, &bytes, "context-transfer").unwrap();
    let restored_ws = restored["workspaces"][0]["id"].as_str().unwrap().to_string();
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
    let artifacts = list["artifacts"].as_array().unwrap();
    let restored_experiment = artifacts.iter().find(|a| a["type"] == "experiment").unwrap();
    let restored_concept = artifacts.iter().find(|a| a["type"] == "concept").unwrap();
    let restored_receipt = artifacts.iter().find(|a| a["type"] == "transfer-approval").unwrap();
    let restored_experiment_id = restored_experiment["artifactId"].as_str().unwrap();
    let restored_concept_id = restored_concept["artifactId"].as_str().unwrap();
    let experiment_body: Value = serde_json::from_str(
        fresh
            .artifacts_command(
                "read",
                json!({ "contractVersion": 1, "workspaceId": restored_ws, "artifactId": restored_experiment_id, "version": 1 }),
            )
            .unwrap()["body"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    // The snapshot survived with remapped references.
    assert_eq!(experiment_body["memoryContext"][0]["content"], "Revision B tide observations.");
    assert_ne!(experiment_body["memoryContext"][0]["memoryId"], mem_id);
    assert_ne!(experiment_body["memoryContext"][0]["revisionId"], mem_rev);
    let restored_hash = content::hash(
        fresh
            .artifacts_command(
                "read",
                json!({ "contractVersion": 1, "workspaceId": restored_ws, "artifactId": restored_experiment_id, "version": 1 }),
            )
            .unwrap()["body"]
            .as_str()
            .unwrap()
            .as_bytes(),
    );
    let concept_body: Value = serde_json::from_str(
        fresh
            .artifacts_command(
                "read",
                json!({ "contractVersion": 1, "workspaceId": restored_ws, "artifactId": restored_concept_id, "version": 1 }),
            )
            .unwrap()["body"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(concept_body["sourceExperimentArtifactId"], restored_experiment_id);
    assert_eq!(concept_body["sourceExperimentContentHash"], restored_hash);
    let receipt_body: Value = serde_json::from_str(
        fresh
            .artifacts_command(
                "read",
                json!({ "contractVersion": 1, "workspaceId": restored_ws, "artifactId": restored_receipt["artifactId"].as_str().unwrap(), "version": 1 }),
            )
            .unwrap()["body"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(receipt_body["sourceExperimentArtifactId"], restored_experiment_id);
    assert_eq!(receipt_body["sourceExperimentContentHash"], restored_hash);
    // Forge admission on the restored workspace still
    // accepts the transferred concept (receipt + hashes
    // match the restored equivalents). The restored
    // workspace needs its own provider configuration.
    fresh
        .ai_command(
            "providers-configure",
            json!({
                "workspaceId": restored_ws,
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
    let started = fresh
        .ai_command(
            "forge-spec-start",
            json!({ "contractVersion": 1, "workspaceId": restored_ws, "projectId": restored_project, "sourceArtifactId": restored_concept_id, "sourceVersion": 1, "conceptIndex": 0, "purpose": "Route flow.", "intendedUsers": "Desks.", "mustHave": "Score venues.", "zeroCostLock": true }),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        let polled = fresh
            .ai_command(
                "forge-spec-poll",
                json!({ "contractVersion": 1, "workspaceId": restored_ws, "runId": run_id }),
            )
            .unwrap();
        let status = polled["status"].as_str().unwrap().to_string();
        if matches!(status.as_str(), "complete" | "invalid" | "failed" | "cancelled")
            || Instant::now() > deadline
        {
            break status;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    assert_eq!(status, "complete", "restored transfer must stay Forge-admissible");
}
