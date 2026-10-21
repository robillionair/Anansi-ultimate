//! M6 backup/recovery repair: artifact round trips, legacy
//! packages, corruption refusal. Fixtures use the real Core
//! commands and the real job upload/restore flow.

use anansi_core::{
    Core,
    content,
    jobs::ExportSelection,
    storage::{DATABASE_NAME, Storage},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn concept(name: &str) -> Value {
    json!({
        "name": name, "problem": "A problem.", "targetCustomer": "Cafes",
        "solution": "A tool.", "whyUseful": "Saves time.",
        "differentiationHypothesis": "A hypothesis.", "revenueModel": "Subscription.",
        "engineeringComplexity": "medium", "implementationCost": "3-5 weeks",
        "keyRisks": ["Adoption"], "requiredValidation": "Interviews.",
        "minimumViableVersion": "One workflow."
    })
}

fn workspace(core: &Core, name: &str) -> String {
    core.workspace_command("create", json!({"contractVersion":1,"name":name}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn make_project(core: &Core, ws: &str, title: &str) -> String {
    core.memory_command(
        "create",
        json!({"contractVersion":1,"workspaceId":ws,"kind":"project",
               "draft":{"title":title,"body":"A project.","privacy":"private"}}),
    )
    .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

pub struct Rich {
    pub ws: String,
    pub project: String,
    pub idea: String,
    pub rubric: String,
    pub archived_project: String,
    pub archived_artifact: String,
}

/// A workspace with a project holding an idea set, rubric
/// v1+v2 and one evaluation per rubric version, an archived
/// second project with an artifact, and a searchable note.
fn rich_fixture(core: &Core) -> Rich {
    let ws = workspace(core, "Artifact source");
    let project = make_project(core, &ws, "Harbor App");
    let concepts: Vec<Value> = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"]
        .into_iter()
        .map(concept)
        .collect();
    let idea_body = json!({ "topic": "harbor logistics", "concepts": concepts }).to_string();
    let idea = core
        .artifacts_command(
            "register",
            json!({"contractVersion":1,"workspaceId":ws,"projectId":project,
                   "type":"idea","title":"Idea set: harbor logistics","body":idea_body,
                   "sourceReferences":[],"privacy":"private"}),
        )
        .unwrap();
    let idea_id = idea["artifact"]["artifactId"].as_str().unwrap().to_string();
    let standard = core
        .artifacts_command(
            "evaluations-standard-rubric",
            json!({"contractVersion":1,"workspaceId":ws,"projectId":project}),
        )
        .unwrap();
    let rubric = core
        .artifacts_command(
            "register",
            json!({"contractVersion":1,"workspaceId":ws,"projectId":project,
                   "type":"rubric","title":"Standard evaluation rubric",
                   "body":standard["rubric"].to_string(),"sourceReferences":[],"privacy":"private"}),
        )
        .unwrap();
    let rubric_id = rubric["artifact"]["artifactId"].as_str().unwrap().to_string();
    let assess = json!([
        {"conceptIndex":0,"dimension":"problem-clarity","score":5,"rationale":"Sharp."},
        {"conceptIndex":0,"dimension":"mvp-feasibility","score":3,"rationale":""}
    ]);
    for rubric_version in [1] {
        core.artifacts_command(
            "evaluations-save",
            json!({"contractVersion":1,"workspaceId":ws,"projectId":project,
                   "sourceArtifactId":idea_id,"sourceVersion":1,
                   "rubricArtifactId":rubric_id,"rubricVersion":rubric_version,
                   "conceptIndices":[0],"assessments":assess}),
        )
        .unwrap();
    }
    // Rubric v2 (problem-clarity weighs 3), then an evaluation
    // citing v2.
    let mut rubric_v2 = standard["rubric"].clone();
    rubric_v2["dimensions"][0]["weight"] = json!(3);
    core.artifacts_command(
        "revise",
        json!({"contractVersion":1,"workspaceId":ws,"artifactId":rubric_id,"expectedVersion":1,
               "title":"Standard evaluation rubric","body":rubric_v2.to_string(),
               "sourceReferences":[]}),
    )
    .unwrap();
    core.artifacts_command(
        "evaluations-save",
        json!({"contractVersion":1,"workspaceId":ws,"projectId":project,
               "sourceArtifactId":idea_id,"sourceVersion":1,
               "rubricArtifactId":rubric_id,"rubricVersion":2,
               "conceptIndices":[0],"assessments":assess}),
    )
    .unwrap();
    // Archived second project with one artifact.
    let archived_project = make_project(core, &ws, "Retired App");
    let note = core
        .artifacts_command(
            "register",
            json!({"contractVersion":1,"workspaceId":ws,"projectId":archived_project,
                   "type":"note","title":"Retired note","body":"archived body",
                   "sourceReferences":[],"privacy":"private"}),
        )
        .unwrap();
    let archived_artifact = note["artifact"]["artifactId"].as_str().unwrap().to_string();
    let current = core
        .memory_command(
            "read",
            json!({"contractVersion":1,"workspaceId":ws,"id":archived_project}),
        )
        .unwrap();
    core.memory_command(
        "revise",
        json!({"contractVersion":1,"workspaceId":ws,"id":archived_project,
               "expectedRevision":current["memory"]["revisionId"],
               "draft":{"title":"Retired App","body":"A project.","privacy":"private"},
               "lifecycle":{"state":"archived","confirmed":true,"successorId":null}}),
    )
    .unwrap();
    // A searchable note; the search creates the FTS table.
    core.memory_command(
        "create",
        json!({"contractVersion":1,"workspaceId":ws,"kind":"note",
               "draft":{"title":"Field notes","body":"harbor quixoticzeppelin survey","privacy":"private"}}),
    )
    .unwrap();
    let hits = core
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":ws,"query":"quixoticzeppelin","fullBody":true}),
        )
        .unwrap();
    assert_eq!(hits["memories"].as_array().unwrap().len(), 1);
    Rich {
        ws,
        project,
        idea: idea_id,
        rubric: rubric_id,
        archived_project,
        archived_artifact,
    }
}

/// Every version of every artifact of a project, in a stable
/// order, as comparable JSON (ids included under "artifactId").
fn artifact_dump(core: &Core, ws: &str, project: &str) -> Vec<Value> {
    let list = core
        .artifacts_command("list", json!({"contractVersion":1,"workspaceId":ws,"projectId":project}))
        .unwrap();
    let mut out = Vec::new();
    for record in list["artifacts"].as_array().unwrap() {
        let id = record["artifactId"].as_str().unwrap();
        let versions = core
            .artifacts_command(
                "versions",
                json!({"contractVersion":1,"workspaceId":ws,"artifactId":id}),
            )
            .unwrap();
        for v in versions["versions"].as_array().unwrap() {
            let read = core
                .artifacts_command(
                    "read",
                    json!({"contractVersion":1,"workspaceId":ws,"artifactId":id,"version":v["version"]}),
                )
                .unwrap();
            out.push(json!({
                "artifactId": id, "type": v["type"], "version": v["version"],
                "title": v["title"], "body": read["body"],
                "contentHash": v["contentHash"], "generator": v["generator"],
                "validationStatus": v["validationStatus"], "privacy": v["privacy"],
                "sourceReferences": v["sourceReferences"]
            }));
        }
    }
    out.sort_by_key(|a| {
        format!(
            "{}:{}:{}",
            a["artifactId"].as_str().unwrap(),
            a["version"],
            a["type"].as_str().unwrap()
        )
    });
    out
}


/// A stable identity for an artifact across stores: its
/// type plus its body with (for evaluations) the two id
/// fields blanked - everything else about the body is
/// content that must survive the round trip.
fn identity_key(entry: &Value) -> String {
    let body = entry["body"].as_str().unwrap();
    let canonical = if entry["type"] == "evaluation" {
        let mut parsed: Value = serde_json::from_str(body).unwrap();
        parsed["sourceArtifactId"] = json!("");
        parsed["rubricArtifactId"] = json!("");
        parsed.to_string()
    } else {
        body.to_string()
    };
    format!("{}|{}", entry["type"].as_str().unwrap(), canonical)
}

/// Pair source artifact ids with restored ids by the
/// stable (type, title) identity of each artifact.
fn pair_ids(source: &[Value], restored: &[Value]) -> BTreeMap<String, String> {
    let mut by_key: BTreeMap<String, String> = BTreeMap::new();
    for entry in restored {
        by_key.insert(
            identity_key(entry),
            entry["artifactId"].as_str().unwrap().to_string(),
        );
    }
    let mut map = BTreeMap::new();
    for entry in source {
        map.insert(
            entry["artifactId"].as_str().unwrap().to_string(),
            by_key[&identity_key(entry)].clone(),
        );
    }
    map
}

fn backup(store: &Storage, ws: &str, application: bool) -> (String, String) {
    let job = store
        .enqueue_export(ws, ExportSelection::Backup { application })
        .unwrap();
    let lease = store.claim_export().unwrap().unwrap();
    store.execute_backup(&lease).unwrap();
    let done = store.read_job(ws, &job.id).unwrap();
    (job.id, done.artifact_id.unwrap())
}

fn package_bytes(root: &Path, ws: &str, artifact: &str) -> Vec<u8> {
    std::fs::read(root.join("exports").join(ws).join(format!("{artifact}.tar"))).unwrap()
}

/// The real consumer flow: upload the package into a fresh
/// storage root, preview it, restore it under `name`.
/// Returns (restore result, preview).
fn upload_restore(fresh: &Core, bytes: &[u8], name: &str) -> (Value, Value) {
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
    let restored = fresh
        .job_command(
            "restore",
            json!({"contractVersion":1,"workspaceId":landing,
                   "jobId":job["id"],"artifactId":job["artifactId"],"name":name,
                   "expectedHash":preview["sha256"],"confirm":true}),
        )
        .unwrap();
    (restored, preview)
}

fn upload_preview(fresh: &Core, bytes: &[u8]) -> Result<Value, anansi_core::CommandError> {
    let landing = workspace(fresh, "Preview landing");
    let begin = fresh.job_command(
        "upload-begin",
        json!({"contractVersion":1,"workspaceId":landing,"bytes":bytes.len(),
               "sha256":content::hash(bytes),"plaintextConsent":true}),
    )?;
    let mut job = Value::Null;
    for (index, chunk) in bytes.chunks(65536).enumerate() {
        let offset = index * 65536;
        job = fresh.job_command(
            "upload-part",
            json!({"contractVersion":1,"workspaceId":landing,"id":begin["id"],
                   "offset":offset,"bytes":chunk,
                   "finish":offset + chunk.len() == bytes.len()}),
        )?["job"]
            .clone();
    }
    fresh.job_command(
        "preview",
        json!({"contractVersion":1,"workspaceId":landing,
               "jobId":job["id"],"artifactId":job["artifactId"]}),
    )
}

/// Compare a source dump with the restored dump: identical
/// except artifact ids are reminted and evaluation bodies /
/// source references point at the restored ids.
fn assert_round_tripped(source: &[Value], restored: &[Value], id_map: &BTreeMap<String, String>) {
    assert_eq!(source.len(), restored.len(), "artifact version count");
    let mut restored_by_key: BTreeMap<String, &Value> = BTreeMap::new();
    for entry in restored {
        restored_by_key.insert(identity_key(entry), entry);
    }
    for entry in source {
        let key = identity_key(entry);
        let got = restored_by_key
            .get(&key)
            .unwrap_or_else(|| panic!("missing restored artifact {key}"));
        let new_id = id_map[entry["artifactId"].as_str().unwrap()].clone();
        assert_eq!(got["artifactId"].as_str().unwrap(), new_id);
        for field in ["type", "version", "title", "validationStatus", "privacy", "generator"] {
            assert_eq!(&entry[field], &got[field], "{field} of {key}");
        }
        // Source references remap artifact ids.
        let remap_refs = |refs: &Value| -> Value {
            Value::Array(
                refs.as_array()
                    .unwrap()
                    .iter()
                    .map(|r| {
                        let s = r.as_str().unwrap();
                        if let Some(rest) = s.strip_prefix("artifact://") {
                            let (id, v) = rest.split_once('@').unwrap();
                            Value::String(format!("artifact://{}@{}", id_map[id], v))
                        } else {
                            r.clone()
                        }
                    })
                    .collect(),
            )
        };
        assert_eq!(remap_refs(&entry["sourceReferences"]), got["sourceReferences"]);
        if entry["type"] == "evaluation" {
            let mut expected: Value = serde_json::from_str(entry["body"].as_str().unwrap()).unwrap();
            expected["sourceArtifactId"] = json!(id_map[expected["sourceArtifactId"].as_str().unwrap()]);
            expected["rubricArtifactId"] = json!(id_map[expected["rubricArtifactId"].as_str().unwrap()]);
            let got_body: Value = serde_json::from_str(got["body"].as_str().unwrap()).unwrap();
            assert_eq!(expected, got_body, "evaluation body of {key}");
            assert_eq!(
                got["contentHash"].as_str().unwrap(),
                content::hash(got["body"].as_str().unwrap().as_bytes()),
                "evaluation hash covers the remapped body"
            );
        } else {
            assert_eq!(entry["body"], got["body"], "body of {key}");
            assert_eq!(entry["contentHash"], got["contentHash"], "hash of {key}");
        }
    }
}

#[test]
fn workspace_backup_restores_artifacts_evaluations_and_archived_projects() {
    let dir = tempfile::tempdir().unwrap();
    let root_a = dir.path().join("a");
    let core_a = Core::with_storage_root(root_a.clone());
    let rich = rich_fixture(&core_a);
    let store_a = Storage::new(root_a.clone()).unwrap();
    let before = artifact_dump(&core_a, &rich.ws, &rich.project);
    let before_archived = artifact_dump(&core_a, &rich.ws, &rich.archived_project);
    let (_job, artifact) = backup(&store_a, &rich.ws, false);
    let bytes = package_bytes(&root_a, &rich.ws, &artifact);

    let root_b = dir.path().join("b");
    let core_b = Core::with_storage_root(root_b.clone());
    let (restored, _preview) = upload_restore(&core_b, &bytes, "Recovered");
    let new_ws = restored["workspaces"][0]["id"].as_str().unwrap().to_string();
    assert_ne!(new_ws, rich.ws);

    // Map source ids to restored ids through the memory
    // list (the archived project is not in the default
    // active listing, so list everything).
    let projects = core_b
        .memory_command("list", json!({"contractVersion":1,"workspaceId":new_ws,"includeArchived":true}))
        .unwrap();
    let mut new_project = String::new();
    let mut new_archived = String::new();
    for p in projects["memories"].as_array().unwrap() {
        if p["title"] == "Harbor App" {
            new_project = p["id"].as_str().unwrap().to_string();
        }
        if p["title"] == "Retired App" {
            new_archived = p["id"].as_str().unwrap().to_string();
        }
    }
    assert!(!new_project.is_empty() && !new_archived.is_empty(), "projects list: {projects}");
    let after = artifact_dump(&core_b, &new_ws, &new_project);
    let id_map = pair_ids(&before, &after);
    assert_round_tripped(&before, &after, &id_map);
    let after_archived = artifact_dump(&core_b, &new_ws, &new_archived);
    assert_eq!(after_archived.len(), 1);
    assert_eq!(after_archived[0]["body"], "archived body");
    assert_ne!(
        after_archived[0]["artifactId"].as_str().unwrap(),
        rich.archived_artifact
    );
    // The restored archived project still refuses writes.
    assert!(
        core_b
            .artifacts_command(
                "register",
                json!({"contractVersion":1,"workspaceId":new_ws,"projectId":new_archived,
                       "type":"note","title":"Nope","body":"x","sourceReferences":[],"privacy":"private"}),
            )
            .is_err()
    );
    // Evaluations reopen: the registry read path validates
    // hashes and returns the remapped body.
    let evaluation = after
        .iter()
        .find(|a| a["type"] == "evaluation")
        .expect("evaluation restored");
    let body: Value = serde_json::from_str(evaluation["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["rubric"]["dimensions"][0]["key"], "problem-clarity");
    assert_eq!(body["concepts"][0]["name"], "Alpha");
    // Original workspace untouched.
    assert_eq!(artifact_dump(&core_a, &rich.ws, &rich.project), before);
    assert_eq!(
        artifact_dump(&core_a, &rich.ws, &rich.archived_project),
        before_archived
    );
    let _ = PathBuf::new();
}

#[test]
fn application_backup_from_a_used_database_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let root_a = dir.path().join("a");
    let core_a = Core::with_storage_root(root_a.clone());
    let rich = rich_fixture(&core_a);
    // The embeddings table exists once semantic indexing ran.
    {
        let conn = rusqlite::Connection::open(root_a.join(DATABASE_NAME)).unwrap();
        anansi_core::vector::ensure_embeddings_table(&conn).unwrap();
    }
    let store_a = Storage::new(root_a.clone()).unwrap();
    let before = artifact_dump(&core_a, &rich.ws, &rich.project);
    // Finding 2 repro: creating the application backup runs
    // the strict self-verification over a database that has
    // FTS + embeddings structures.
    let (_job, artifact) = backup(&store_a, &rich.ws, true);
    let bytes = package_bytes(&root_a, &rich.ws, &artifact);

    let root_b = dir.path().join("b");
    let core_b = Core::with_storage_root(root_b.clone());
    let (restored, preview) = upload_restore(&core_b, &bytes, "App recovered");
    assert_eq!(preview["application"], true);
    let mut new_ws = String::new();
    for w in restored["workspaces"].as_array().unwrap() {
        let projects = core_b
            .memory_command(
                "list",
                json!({"contractVersion":1,"workspaceId":w["id"],"kind":"project"}),
            )
            .unwrap();
        if !projects["memories"].as_array().unwrap().is_empty() {
            new_ws = w["id"].as_str().unwrap().to_string();
        }
    }
    assert!(!new_ws.is_empty());
    let projects = core_b
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":new_ws,"kind":"project"}),
        )
        .unwrap();
    let new_project = projects["memories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "Harbor App")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let after = artifact_dump(&core_b, &new_ws, &new_project);
    let id_map = pair_ids(&before, &after);
    assert_round_tripped(&before, &after, &id_map);
    // FTS was rebuilt from restored content: the note is
    // findable, and the index rows carry the NEW workspace id.
    let hits = core_b
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":new_ws,"query":"quixoticzeppelin","fullBody":true}),
        )
        .unwrap();
    assert_eq!(hits["memories"].as_array().unwrap().len(), 1);
    let conn = rusqlite::Connection::open(root_b.join(DATABASE_NAME)).unwrap();
    let fts_rows: i64 = conn
        .query_row(
            "SELECT count(*) FROM memory_body_fts WHERE workspace_id=?1",
            rusqlite::params![new_ws],
            |r| r.get(0),
        )
        .unwrap();
    assert!(fts_rows >= 1, "FTS rebuilt for the restored workspace");
    let stale_rows: i64 = conn
        .query_row(
            "SELECT count(*) FROM memory_body_fts WHERE workspace_id=?1",
            rusqlite::params![rich.ws],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stale_rows, 0, "archived FTS rows are never imported");
    let _ = Path::new("");
}
