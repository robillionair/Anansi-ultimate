//! M6 Slice 4: opportunity set backup/restore identity.
//! A workspace backup restored into a fresh workspace must
//! keep the set's source relationships intact: memory
//! sources resolve to the RESTORED memories (the body ids
//! are remapped, the hash recomputed), supplied texts carry
//! over byte-identically, and the restored set can be
//! evaluated against the restored project with the existing
//! rubric workflow.

use anansi_core::{Core, content, jobs::ExportSelection, storage::Storage};
use serde_json::{Value, json};
use std::path::Path;

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

fn upload_restore(fresh: &Core, bytes: &[u8], name: &str) -> Value {
    let landing = workspace(fresh, "Recovery landing");
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
    fresh
        .job_command(
            "restore",
            json!({"contractVersion":1,"workspaceId":landing,
                   "jobId":job["id"],"artifactId":job["artifactId"],"name":name,
                   "expectedHash":preview["sha256"],"confirm":true}),
        )
        .unwrap()
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
