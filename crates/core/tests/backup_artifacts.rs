//! M6 backup/recovery repair: artifact round trips, legacy
//! packages, corruption refusal. Fixtures use the real Core
//! commands and the real job upload/restore flow.

use anansi_core::{
    Core, content,
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
    pub note_memory_id: String,
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
    let rubric_id = rubric["artifact"]["artifactId"]
        .as_str()
        .unwrap()
        .to_string();
    let assess = json!([
        {"conceptIndex":0,"dimension":"problem-clarity","score":5,"rationale":"Sharp."},
        {"conceptIndex":0,"dimension":"mvp-feasibility","score":3,"rationale":""}
    ]);
    // One evaluation citing rubric v1 (the v2 evaluation
    // follows the rubric revision below).
    core.artifacts_command(
        "evaluations-save",
        json!({"contractVersion":1,"workspaceId":ws,"projectId":project,
               "sourceArtifactId":idea_id,"sourceVersion":1,
               "rubricArtifactId":rubric_id,"rubricVersion":1,
               "conceptIndices":[0],"assessments":assess}),
    )
    .unwrap();
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
    // Archived second project with one artifact. The note
    // memory is created first so the artifact can carry a
    // memory:// source reference (which restore must
    // preserve intact - only artifact:// references remap).
    let note_memory = core
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,"kind":"note",
                   "draft":{"title":"Field notes","body":"harbor quixoticzeppelin survey","privacy":"private"}}),
        )
        .unwrap();
    let note_memory_id = note_memory["memory"]["id"].as_str().unwrap().to_string();
    let archived_project = make_project(core, &ws, "Retired App");
    let note = core
        .artifacts_command(
            "register",
            json!({"contractVersion":1,"workspaceId":ws,"projectId":archived_project,
                   "type":"note","title":"Retired note","body":"archived body",
                   "sourceReferences":[format!("memory://{note_memory_id}")],"privacy":"private"}),
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
    // The search creates the FTS table in the source store.
    let hits = core
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":ws,"query":"quixoticzeppelin","fullBody":true}),
        )
        .unwrap();
    assert_eq!(hits["memories"].as_array().unwrap().len(), 1);
    Rich {
        ws,
        note_memory_id,
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
        .artifacts_command(
            "list",
            json!({"contractVersion":1,"workspaceId":ws,"projectId":project}),
        )
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
    std::fs::read(
        root.join("exports")
            .join(ws)
            .join(format!("{artifact}.tar")),
    )
    .unwrap()
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
        for field in [
            "type",
            "version",
            "title",
            "validationStatus",
            "privacy",
            "generator",
        ] {
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
        assert_eq!(
            remap_refs(&entry["sourceReferences"]),
            got["sourceReferences"]
        );
        if entry["type"] == "evaluation" {
            let mut expected: Value =
                serde_json::from_str(entry["body"].as_str().unwrap()).unwrap();
            expected["sourceArtifactId"] =
                json!(id_map[expected["sourceArtifactId"].as_str().unwrap()]);
            expected["rubricArtifactId"] =
                json!(id_map[expected["rubricArtifactId"].as_str().unwrap()]);
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
    let new_ws = restored["workspaces"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(new_ws, rich.ws);

    // Map source ids to restored ids through the memory
    // list (the archived project is not in the default
    // active listing, so list everything).
    let projects = core_b
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":new_ws,"includeArchived":true}),
        )
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
    assert!(
        !new_project.is_empty() && !new_archived.is_empty(),
        "projects list: {projects}"
    );
    let after = artifact_dump(&core_b, &new_ws, &new_project);
    let id_map = pair_ids(&before, &after);
    assert_round_tripped(&before, &after, &id_map);
    let after_archived = artifact_dump(&core_b, &new_ws, &new_archived);
    assert_eq!(after_archived.len(), 1);
    assert_eq!(after_archived[0]["body"], "archived body");
    assert_eq!(
        after_archived[0]["sourceReferences"],
        json!([format!("memory://{}", rich.note_memory_id)]),
        "memory:// references are preserved intact"
    );
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

// ---------------------------------------------------------------------
// Legacy packages and failure modes.

fn tar_members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut result = BTreeMap::new();
    let mut offset = 0;
    while offset + 512 <= bytes.len() && bytes[offset..offset + 512].iter().any(|b| *b != 0) {
        let h = &bytes[offset..offset + 512];
        let end = h[..100].iter().position(|b| *b == 0).unwrap();
        let name = std::str::from_utf8(&h[..end]).unwrap().to_string();
        let size = usize::from_str_radix(
            std::str::from_utf8(&h[124..136])
                .unwrap()
                .trim_matches(['\0', ' ']),
            8,
        )
        .unwrap();
        result.insert(name, bytes[offset + 512..offset + 512 + size].to_vec());
        offset = (offset + 512 + size).div_ceil(512) * 512;
    }
    result
}

fn tar_build(members: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, data) in members {
        let mut h = [0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        h[100..108].copy_from_slice(b"0000644\0");
        h[108..116].copy_from_slice(b"0000000\0");
        h[116..124].copy_from_slice(b"0000000\0");
        h[124..136].copy_from_slice(format!("{:011o}\0", data.len()).as_bytes());
        h[136..148].copy_from_slice(b"00000000000\0");
        h[148..156].fill(b' ');
        h[156] = b'0';
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        let sum: u32 = h.iter().map(|b| u32::from(*b)).sum();
        h[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        out.extend_from_slice(&h);
        out.extend_from_slice(data);
        let pad = (512 - data.len() % 512) % 512;
        out.extend(std::iter::repeat_n(0, pad));
    }
    out.extend(std::iter::repeat_n(0, 1024));
    out
}

/// Fix the manifest entry for a replaced catalog member.
fn refresh_catalog_member(members: &mut BTreeMap<String, Vec<u8>>) {
    let catalog = members["catalog.sqlite3"].clone();
    let mut manifest: anansi_core::backup::BackupManifest =
        serde_json::from_slice(&members["manifest.json"]).unwrap();
    for member in &mut manifest.members {
        if member.path == "catalog.sqlite3" {
            member.bytes = catalog.len() as u64;
            member.sha256 = content::hash(&catalog);
        }
    }
    members.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
}

fn edit_catalog(
    members: &mut BTreeMap<String, Vec<u8>>,
    dir: &Path,
    edit: impl FnOnce(&rusqlite::Connection),
) {
    let path = dir.join("edit-catalog.sqlite3");
    std::fs::write(&path, &members["catalog.sqlite3"]).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    edit(&conn);
    drop(conn);
    members.insert("catalog.sqlite3".into(), std::fs::read(&path).unwrap());
    refresh_catalog_member(members);
}

/// Build a package in the actual older format: a catalog
/// created from the schema-8 (or schema-9) migrations,
/// populated from a real workspace's rows, packaged with the
/// current manifest/tar code and the older declared version.
fn legacy_source(scratch: &Core) -> String {
    let ws = workspace(scratch, "Legacy source");
    scratch
        .memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,"kind":"note",
                   "draft":{"title":"Legacy note","body":"legacy body one","privacy":"private"}}),
        )
        .unwrap();
    ws
}

fn legacy_package(scratch_root: &Path, dir: &Path, declared: i64, ws: &str) -> Vec<u8> {
    let ws = ws.to_string();
    let source = rusqlite::Connection::open(scratch_root.join(DATABASE_NAME)).unwrap();
    let catalog_path = dir.join(format!("legacy-{declared}.sqlite3"));
    let _ = std::fs::remove_file(&catalog_path);
    let mut catalog = rusqlite::Connection::open(&catalog_path).unwrap();
    let mut ddl = String::new();
    for file in [
        "0001_workspace.sql",
        "0002_onboarding.sql",
        "0003_memory.sql",
        "0004_lifecycle.sql",
        "0005_entities.sql",
        "0006_relationship_history.sql",
        "0007_jobs.sql",
        "0008_backups.sql",
    ] {
        ddl.push_str(
            &std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("migrations")
                    .join(file),
            )
            .unwrap(),
        );
    }
    if declared >= 9 {
        ddl.push_str(
            &std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations/0009_vault.sql"),
            )
            .unwrap(),
        );
    }
    catalog.execute_batch(&ddl).unwrap();
    if declared >= 9 {
        // The vault seeds its write guard armed; real writes
        // happen inside a maintenance window (see the vault
        // catalog). Populate inside one, then close it so the
        // packaged catalog is in its resting state.
        catalog
            .execute("UPDATE vault_maintenance SET enabled=1", [])
            .unwrap();
    }
    catalog
        .pragma_update(None, "application_id", 0x414e414e)
        .unwrap();
    catalog
        .pragma_update(None, "user_version", declared)
        .unwrap();
    let tx = catalog.transaction().unwrap();
    for table in [
        "workspaces",
        "workspace_settings",
        "memories",
        "memory_revisions",
    ] {
        let key = if table == "workspaces" {
            "id"
        } else {
            "workspace_id"
        };
        let mut q = source
            .prepare(&format!("SELECT * FROM {table} WHERE {key}=?1"))
            .unwrap();
        let count = q.column_count();
        let slots = (1..=count)
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let mut insert = tx
            .prepare(&format!("INSERT INTO {table} VALUES({slots})"))
            .unwrap();
        let mut rows = q.query(rusqlite::params![ws]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            let values = (0..count)
                .map(|i| row.get::<_, rusqlite::types::Value>(i))
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            insert.execute(rusqlite::params_from_iter(values)).unwrap();
        }
    }
    tx.commit().unwrap();
    if declared >= 9 {
        catalog
            .execute("UPDATE vault_maintenance SET enabled=0", [])
            .unwrap();
    }
    drop(catalog);
    // Body members from the real revision rows.
    let revision: String = source
        .query_row(
            "SELECT id FROM memory_revisions WHERE workspace_id=?1",
            rusqlite::params![ws],
            |r| r.get(0),
        )
        .unwrap();
    let mut members = BTreeMap::new();
    members.insert(
        "catalog.sqlite3".into(),
        std::fs::read(&catalog_path).unwrap(),
    );
    members.insert(
        "snapshot.json".into(),
        serde_json::to_vec(
            &json!({"schemaVersion": declared, "application": false, "workspaces": [ws]}),
        )
        .unwrap(),
    );
    members.insert(format!("bodies/{revision}.md"), b"legacy body one".to_vec());
    let manifest = anansi_core::backup::BackupManifest {
        format: anansi_core::backup::BACKUP_FORMAT.into(),
        format_version: anansi_core::backup::BACKUP_VERSION,
        workspace_id: ws.clone(),
        workspace_name: "Legacy source".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        members: members
            .iter()
            .map(|(path, data)| anansi_core::backup::BackupMember {
                path: path.clone(),
                bytes: data.len() as u64,
                sha256: content::hash(data),
            })
            .collect(),
    };
    members.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    tar_build(&members)
}

#[test]
fn legacy_schema_8_and_9_packages_restore_and_unknown_versions_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let scratch_root = dir.path().join("scratch");
    let scratch = Core::with_storage_root(scratch_root.clone());
    let legacy_ws = legacy_source(&scratch);
    for declared in [8, 9] {
        let bytes = legacy_package(&scratch_root, dir.path(), declared, &legacy_ws);
        let fresh_root = dir.path().join(format!("fresh-{declared}"));
        let fresh = Core::with_storage_root(fresh_root);
        let (restored, _preview) = upload_restore(&fresh, &bytes, &format!("Legacy {declared}"));
        let new_ws = restored["workspaces"][0]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let list = fresh
            .memory_command("list", json!({"contractVersion":1,"workspaceId":new_ws}))
            .unwrap();
        assert_eq!(list["memories"].as_array().unwrap().len(), 1);
        let read = fresh
            .memory_command(
                "read",
                json!({"contractVersion":1,"workspaceId":new_ws,"id":list["memories"][0]["id"]}),
            )
            .unwrap();
        assert_eq!(read["body"], "legacy body one");
    }
    // Declared versions outside {8, 9, 10} fail closed, and a
    // declaration that disagrees with the catalog does too.
    for (declared, actual) in [(7, 7), (11, 11), (9, 8)] {
        let mut bytes = legacy_package(&scratch_root, dir.path(), actual, &legacy_ws);
        let mut members = tar_members(&bytes);
        members.insert(
            "snapshot.json".into(),
            serde_json::to_vec(
                &json!({"schemaVersion": declared, "application": false,
                        "workspaces": [serde_json::from_slice::<anansi_core::backup::BackupManifest>(&members["manifest.json"]).unwrap().workspace_id]}),
            )
            .unwrap(),
        );
        // snapshot.json is itself a manifest member; refresh hashes.
        let snap = members["snapshot.json"].clone();
        let mut manifest: anansi_core::backup::BackupManifest =
            serde_json::from_slice(&members["manifest.json"]).unwrap();
        for member in &mut manifest.members {
            if member.path == "snapshot.json" {
                member.bytes = snap.len() as u64;
                member.sha256 = content::hash(&snap);
            }
        }
        members.insert(
            "manifest.json".into(),
            serde_json::to_vec(&manifest).unwrap(),
        );
        bytes = tar_build(&members);
        let fresh = Core::with_storage_root(dir.path().join(format!("refuse-{declared}-{actual}")));
        assert!(
            upload_preview(&fresh, &bytes).is_err(),
            "declared {declared} / actual {actual} must be refused"
        );
    }
}

#[test]
fn tampered_artifact_and_dangling_reference_packages_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let root_a = dir.path().join("a");
    let core_a = Core::with_storage_root(root_a.clone());
    let rich = rich_fixture(&core_a);
    let store_a = Storage::new(root_a.clone()).unwrap();
    let (_job, artifact) = backup(&store_a, &rich.ws, false);
    let original = package_bytes(&root_a, &rich.ws, &artifact);

    // Control: the untouched package previews fine.
    let fresh = Core::with_storage_root(dir.path().join("control"));
    assert!(upload_preview(&fresh, &original).is_ok());

    // (a) A flipped artifact body byte (hash no longer matches).
    let mut members = tar_members(&original);
    edit_catalog(&mut members, dir.path(), |conn| {
        conn.execute(
            "UPDATE artifact_versions SET body = body || 'x' WHERE artifact_type='idea'",
            [],
        )
        .unwrap();
    });
    let fresh = Core::with_storage_root(dir.path().join("tamper-body"));
    assert!(upload_preview(&fresh, &tar_build(&members)).is_err());

    // (b) An evaluation pointing at an artifact that is not
    // in the package (body edited, its row hash recomputed so
    // only the reference check can catch it).
    let mut members = tar_members(&original);
    edit_catalog(&mut members, dir.path(), |conn| {
        let body: String = conn
            .query_row(
                "SELECT body FROM artifact_versions WHERE artifact_type='evaluation' LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let mut doc: Value = serde_json::from_str(&body).unwrap();
        doc["sourceArtifactId"] = json!("f".repeat(32));
        let body = doc.to_string();
        let hash = content::hash(body.as_bytes());
        conn.execute(
            "UPDATE artifact_versions SET body=?1, content_hash=?2 WHERE artifact_type='evaluation'",
            rusqlite::params![body, hash],
        )
        .unwrap();
    });
    let fresh = Core::with_storage_root(dir.path().join("tamper-ref"));
    assert!(upload_preview(&fresh, &tar_build(&members)).is_err());

    // (c) A sealed artifact row in a plaintext package.
    let mut members = tar_members(&original);
    edit_catalog(&mut members, dir.path(), |conn| {
        conn.execute(
            "UPDATE artifact_versions SET privacy='sealed' WHERE artifact_type='note'",
            [],
        )
        .unwrap();
    });
    let fresh = Core::with_storage_root(dir.path().join("tamper-sealed"));
    assert!(upload_preview(&fresh, &tar_build(&members)).is_err());
}

#[test]
fn repeat_restore_conflicts_and_failed_restore_imports_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root_a = dir.path().join("a");
    let core_a = Core::with_storage_root(root_a.clone());
    let rich = rich_fixture(&core_a);
    let store_a = Storage::new(root_a.clone()).unwrap();
    let (job, artifact) = backup(&store_a, &rich.ws, false);
    let preview = core_a
        .job_command(
            "preview",
            json!({"contractVersion":1,"workspaceId":rich.ws,"jobId":job,"artifactId":artifact}),
        )
        .unwrap();
    let hash = preview["sha256"].as_str().unwrap().to_string();
    let restored = store_a
        .restore_snapshot(&rich.ws, &job, &artifact, "Recovered workspace", &hash)
        .unwrap();
    assert_eq!(restored.workspaces.len(), 1);
    let count = |store: &Storage| store.list().unwrap().len();
    let before = count(&store_a);
    // Same name: refused. Different name: still refused - the
    // backup was already restored from this artifact.
    assert!(
        store_a
            .restore_snapshot(&rich.ws, &job, &artifact, "Recovered workspace", &hash)
            .is_err()
    );
    let retry = store_a.restore_snapshot(&rich.ws, &job, &artifact, "Another name", &hash);
    let error = retry.err().expect("repeat restore must be refused");
    assert!(
        error.message.contains("already restored"),
        "the refusal names the prior restore: {}",
        error.message
    );
    assert_eq!(count(&store_a), before, "no duplicate import");

    // A failed restore (name already taken by an unrelated
    // workspace) imports nothing at all.
    let root_b = dir.path().join("b");
    let core_b = Core::with_storage_root(root_b.clone());
    workspace(&core_b, "Taken name");
    let bytes = package_bytes(&root_a, &rich.ws, &artifact);
    let store_b = Storage::new(root_b.clone()).unwrap();
    let landing = workspace(&core_b, "Landing");
    let begin = core_b
        .job_command(
            "upload-begin",
            json!({"contractVersion":1,"workspaceId":landing,"bytes":bytes.len(),
                   "sha256":content::hash(&bytes),"plaintextConsent":true}),
        )
        .unwrap();
    let mut job_b = Value::Null;
    for (index, chunk) in bytes.chunks(65536).enumerate() {
        let offset = index * 65536;
        job_b = core_b
            .job_command(
                "upload-part",
                json!({"contractVersion":1,"workspaceId":landing,"id":begin["id"],
                       "offset":offset,"bytes":chunk,
                       "finish":offset + chunk.len() == bytes.len()}),
            )
            .unwrap()["job"]
            .clone();
    }
    let preview_b = core_b
        .job_command(
            "preview",
            json!({"contractVersion":1,"workspaceId":landing,
                   "jobId":job_b["id"],"artifactId":job_b["artifactId"]}),
        )
        .unwrap();
    let before_b = count(&store_b);
    let failed = core_b.job_command(
        "restore",
        json!({"contractVersion":1,"workspaceId":landing,
               "jobId":job_b["id"],"artifactId":job_b["artifactId"],"name":"Taken name",
               "expectedHash":preview_b["sha256"],"confirm":true}),
    );
    assert!(failed.is_err());
    assert_eq!(count(&store_b), before_b, "failed restore imported nothing");
}
