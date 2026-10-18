use anansi_core::{
    Core, content,
    export::ExportManifest,
    jobs::{ExportSelection, JobState},
    storage::{DATABASE_NAME, Storage},
};
use serde_json::json;
use std::collections::BTreeMap;

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut offset = 0;
    while offset + 512 <= bytes.len() {
        let h = &bytes[offset..offset + 512];
        if h.iter().all(|b| *b == 0) {
            break;
        }
        assert_eq!(&h[257..263], b"ustar\0");
        let name = std::str::from_utf8(&h[..100])
            .unwrap()
            .trim_end_matches('\0')
            .to_string();
        let size = usize::from_str_radix(
            std::str::from_utf8(&h[124..136])
                .unwrap()
                .trim_end_matches('\0'),
            8,
        )
        .unwrap();
        files.insert(name, bytes[offset + 512..offset + 512 + size].to_vec());
        offset += 512 + size.div_ceil(512) * 512;
    }
    files
}

#[test]
fn portable_project_export_roundtrips_history_hashes_privacy_and_hostile_text() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());
    let store = Storage::new(root).unwrap();
    let w = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Export fixture"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let project=core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"kind":"project","draft":{"title":"Export project","body":"project body"}})).unwrap();
    let id = project["memory"]["id"].as_str().unwrap();
    let hostile =
        "<script>credential</script> ![remote](https://invalid.example/private) ../../escape\n";
    let first=core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"kind":"note","draft":{"title":"Inert source","body":hostile,"projectId":id,"sourceReferences":["https://invalid.example/source"]}})).unwrap();
    let note = first["memory"]["id"].as_str().unwrap();
    let edge=core.relationship_command("create",json!({"contractVersion":1,"workspaceId":w,"draft":{"sourceId":id,"targetId":note,"predicate":"references"}})).unwrap();
    core.relationship_command("delete",json!({"contractVersion":1,"workspaceId":w,"id":edge["relationship"]["id"],"expectedVersion":1})).unwrap();
    core.memory_command("revise",json!({"contractVersion":1,"workspaceId":w,"id":note,"expectedRevision":first["memory"]["revisionId"],"draft":{"title":"Later title","body":"new revision"}})).unwrap();
    let job = store
        .enqueue_export(&w, ExportSelection::Project { id: id.into() })
        .unwrap();
    let lease = store.claim_export().unwrap().unwrap();
    store.execute_export(&lease).unwrap();
    let complete = store.read_job(&w, &job.id).unwrap();
    assert_eq!(complete.state, JobState::Completed);
    let artifact = complete.artifact_id.as_ref().unwrap();
    let mut bytes = vec![];
    loop {
        let chunk = store
            .read_export_chunk(&w, &job.id, artifact, bytes.len() as u64)
            .unwrap();
        if chunk.is_empty() {
            break;
        }
        bytes.extend(chunk);
    }
    assert_eq!(content::hash(&bytes), complete.artifact_hash.unwrap());
    let files = members(&bytes);
    let manifest: ExportManifest = serde_json::from_slice(&files["manifest.json"]).unwrap();
    assert_eq!(manifest.format_version, 1);
    assert_eq!(manifest.records.len(), 2);
    assert_eq!(manifest.relationship_history.len(), 2);
    let exported_note = manifest.records.iter().find(|r| r.id == note).unwrap();
    assert_eq!(exported_note.revisions.len(), 2);
    assert_eq!(
        serde_json::to_value(&exported_note.revisions[0].metadata).unwrap(),
        first["memory"]
    );
    assert_eq!(
        files[&exported_note.revisions[0].body_path],
        hostile.as_bytes()
    );
    for record in &manifest.records {
        for revision in &record.revisions {
            assert_eq!(content::hash(&files[&revision.body_path]), revision.sha256);
            assert_eq!(revision.metadata.workspace_id, w);
        }
    }
    let decoded: ExportManifest =
        serde_json::from_value(serde_json::to_value(&manifest).unwrap()).unwrap();
    assert_eq!(decoded.records[1].id, manifest.records[1].id);
    assert!(!complete.log.contains("credential"));
    assert!(
        store
            .read_export_chunk(&w, &job.id, &"f".repeat(32), 0)
            .is_err()
    );
    let other = store.create("Other export").unwrap().workspace.id;
    assert!(
        store
            .read_export_chunk(&other, &job.id, artifact, 0)
            .is_err()
    );
    let request = anansi_core::jobs::ReadArtifactRequest {
        contract_version: 1,
        workspace_id: w.clone(),
        job_id: job.id.clone(),
        artifact_id: artifact.clone(),
        offset: 0,
    };
    let destination = temp.path().join("trusted-save.tar");
    core.save_complete_export(&request, &destination).unwrap();
    assert_eq!(std::fs::read(&destination).unwrap(), bytes);
    assert!(core.save_complete_export(&request, &destination).is_err());
    let managed = temp
        .path()
        .join("store/exports")
        .join(&w)
        .join(format!("{artifact}.tar"));
    let mut corrupt = std::fs::read(&managed).unwrap();
    corrupt[0] ^= 1;
    std::fs::write(&managed, corrupt).unwrap();
    let rejected = temp.path().join("corrupt-save.tar");
    assert!(core.save_complete_export(&request, &rejected).is_err());
    assert!(!rejected.exists());
}

#[test]
fn sealed_integrity_and_filesystem_failures_publish_nothing() {
    for failure in ["sealed", "sealed_history", "missing", "disk"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        let core = Core::with_storage_root(root.clone());
        let store = Storage::new(root.clone()).unwrap();
        let w = store.create("Failure fixture").unwrap().workspace.id;
        let memory=core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"draft":{"title":"private secret title","body":"synthetic-private-body"}})).unwrap();
        let id = memory["memory"]["id"].as_str().unwrap();
        let revision = memory["memory"]["revisionId"].as_str().unwrap();
        if failure == "sealed_history" {
            core.memory_command("revise",json!({"contractVersion":1,"workspaceId":w,"id":id,"expectedRevision":revision,"draft":{"title":"Private latest","body":"latest plaintext"}})).unwrap();
            let db = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
            db.execute("UPDATE memory_revisions SET metadata=json_set(metadata,'$.privacy','sealed') WHERE version=1",[]).unwrap();
        }
        if failure == "sealed" {
            let db = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
            db.execute(
                "UPDATE memory_revisions SET metadata=json_set(metadata,'$.privacy','sealed')",
                [],
            )
            .unwrap();
        }
        if failure == "missing" {
            std::fs::remove_file(
                root.join("content")
                    .join(&w)
                    .join(id)
                    .join(format!("{revision}.md")),
            )
            .unwrap();
        }
        if failure == "disk" {
            std::fs::write(root.join("exports"), b"not a directory").unwrap();
        }
        let job = store
            .enqueue_export(&w, ExportSelection::Workspace)
            .unwrap();
        let lease = store.claim_export().unwrap().unwrap();
        assert!(store.execute_export(&lease).is_err());
        store.fail_export(&lease).unwrap();
        let failed = store.read_job(&w, &job.id).unwrap();
        assert_eq!(failed.state, JobState::Failed);
        assert!(failed.artifact_id.is_none());
        assert!(!failed.log.contains("synthetic-private-body"));
        assert!(!failed.log.contains("secret"));
    }
}

#[test]
fn record_quota_is_not_silent_truncation_and_boundary_edges_disclose_no_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());
    let store = Storage::new(root.clone()).unwrap();
    let w = store.create("Selection boundary").unwrap().workspace.id;
    let mut records = vec![];
    for name in ["Selected", "Excluded"] {
        records.push(core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"draft":{"title":name,"body":format!("{name} private body")}})).unwrap());
    }
    let first = records[0]["memory"]["id"].as_str().unwrap();
    let second = records[1]["memory"]["id"].as_str().unwrap();
    let relation=core.relationship_command("create",json!({"contractVersion":1,"workspaceId":w,"draft":{"sourceId":first,"targetId":second,"predicate":"references","metadata":{"sensitive":"Excluded edge secret"}}})).unwrap();
    let job = store
        .enqueue_export(
            &w,
            ExportSelection::Records {
                ids: vec![first.into()],
            },
        )
        .unwrap();
    let lease = store.claim_export().unwrap().unwrap();
    store.execute_export(&lease).unwrap();
    let complete = store.read_job(&w, &job.id).unwrap();
    let bytes = store
        .read_export_chunk(&w, &job.id, complete.artifact_id.as_ref().unwrap(), 0)
        .unwrap();
    let files = members(&bytes);
    let manifest: ExportManifest = serde_json::from_slice(&files["manifest.json"]).unwrap();
    assert_eq!(manifest.records.len(), 1);
    assert!(manifest.relationship_history.is_empty());
    assert!(
        manifest
            .boundary_references
            .contains(&relation["relationship"]["id"].as_str().unwrap().to_string())
    );
    assert!(!String::from_utf8_lossy(&bytes).contains("Excluded"));
    // Install a bounded synthetic catalog just beyond the record quota. No source body
    // fixtures are required: the worker must refuse the selection before reading them.
    let mut db = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
    let tx = db.transaction().unwrap();
    for index in 0..anansi_core::jobs::MAX_RECORDS {
        let id = format!("{index:032x}");
        let revision = format!("{:032x}", index + 100_000);
        let mut metadata = records[0]["memory"].clone();
        metadata["id"] = json!(id);
        metadata["revisionId"] = json!(revision);
        tx.execute(
            "INSERT INTO memories(workspace_id,id,current_revision) VALUES(?1,?2,?3)",
            rusqlite::params![w, id, revision],
        )
        .unwrap();
        tx.execute("INSERT INTO memory_revisions(workspace_id,memory_id,id,version,metadata,body_hash) VALUES(?1,?2,?3,1,?4,?5)",rusqlite::params![w,id,revision,metadata.to_string(),"a".repeat(64)]).unwrap();
    }
    tx.commit().unwrap();
    let job = store
        .enqueue_export(&w, ExportSelection::Workspace)
        .unwrap();
    let lease = store.claim_export().unwrap().unwrap();
    assert!(store.execute_export(&lease).is_err());
    store.fail_export(&lease).unwrap();
    assert!(store.read_job(&w, &job.id).unwrap().artifact_id.is_none());
    for application in [false, true] {
        let job = store
            .enqueue_export(&w, ExportSelection::Backup { application })
            .unwrap();
        let lease = store.claim_export().unwrap().unwrap();
        assert_eq!(
            store.execute_backup(&lease).unwrap_err().code,
            anansi_core::ErrorCode::InvalidRequest
        );
        store.fail_export(&lease).unwrap();
        assert!(store.read_job(&w, &job.id).unwrap().artifact_id.is_none());
    }
}
