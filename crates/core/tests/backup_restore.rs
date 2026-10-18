use anansi_core::{
    backup::{self, BACKUP_FORMAT, BACKUP_VERSION, BackupCatalog, BackupManifest},
    restore,
};
use chrono::Utc;
use std::fs;
use tempfile::tempdir;

fn workspace(core: &anansi_core::Core, name: &str) -> String {
    core.workspace_command(
        "create",
        serde_json::json!({"contractVersion":1,"name":name}),
    )
    .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn real_catalog_snapshot_restores_settings_history_edges_and_preserves_originals() {
    use anansi_core::{Core, jobs::ExportSelection, storage::Storage};
    use serde_json::json;
    let temp = tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());
    let w = workspace(&core, "Backup source");
    let other = workspace(&core, "Must remain absent");
    core.workspace_command("settings",json!({"contractVersion":1,"id":w,"reducedMotion":true,"intent":"build_application","onboardingComplete":true})).unwrap();
    core.memory_command("create",json!({"contractVersion":1,"workspaceId":other,"draft":{"title":"Other secret","body":"excluded bytes"}})).unwrap();
    let first=core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"kind":"note","draft":{"title":"Snapshot note","body":"first revision"}})).unwrap();
    let next=core.memory_command("revise",json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"],"expectedRevision":first["memory"]["revisionId"],"draft":{"title":"Snapshot note","body":"second revision"},"lifecycle":{"state":"confirmed","confirmed":true}})).unwrap();
    let person=core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"kind":"person","draft":{"title":"Snapshot person","body":"contact"}})).unwrap();
    core.relationship_command("create",json!({"contractVersion":1,"workspaceId":w,"draft":{"sourceId":first["memory"]["id"],"targetId":person["memory"]["id"],"predicate":"references"}})).unwrap();
    let store = Storage::new(root.clone()).unwrap();
    let job = store
        .enqueue_export(&w, ExportSelection::Backup { application: false })
        .unwrap();
    let lease = store.claim_export().unwrap().unwrap();
    store.execute_backup(&lease).unwrap();
    let complete = store.read_job(&w, &job.id).unwrap();
    let artifact = complete.artifact_id.unwrap();
    let preview = core
        .job_command(
            "preview",
            json!({"contractVersion":1,"workspaceId":w,"jobId":job.id,"artifactId":artifact}),
        )
        .unwrap();
    assert_eq!(preview["records"], 2);
    assert_eq!(preview["revisions"], 3);
    assert_eq!(preview["relationships"], 1);
    assert_eq!(preview["workspaceCount"], 1);
    let before = fs::read(
        root.join("content")
            .join(&w)
            .join(first["memory"]["id"].as_str().unwrap())
            .join(format!(
                "{}.md",
                next["memory"]["revisionId"].as_str().unwrap()
            )),
    )
    .unwrap();
    let restored = store
        .restore_snapshot(
            &w,
            &job.id,
            &artifact,
            "Recovered workspace",
            preview["sha256"].as_str().unwrap(),
        )
        .unwrap();
    let fresh = &restored.workspaces[0].id;
    assert_ne!(fresh, &w);
    let reopen = Core::with_storage_root(root.clone());
    let settings = reopen
        .workspace_command("open", json!({"contractVersion":1,"id":fresh}))
        .unwrap();
    assert_eq!(settings["settings"]["reducedMotion"], true);
    let list = reopen
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":fresh,"kind":"note"}),
        )
        .unwrap();
    let id = list["memories"][0]["id"].as_str().unwrap();
    let detail = reopen
        .memory_command(
            "read",
            json!({"contractVersion":1,"workspaceId":fresh,"id":id}),
        )
        .unwrap();
    assert_eq!(detail["body"], "second revision");
    assert_eq!(detail["memory"]["status"], "confirmed");
    assert_ne!(detail["memory"]["revisionId"], next["memory"]["revisionId"]);
    let history = reopen
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":fresh,"id":id}),
        )
        .unwrap();
    assert_eq!(history["memories"].as_array().unwrap().len(), 2);
    let old=reopen.memory_command("read",json!({"contractVersion":1,"workspaceId":fresh,"id":id,"revisionId":history["memories"][1]["revisionId"]})).unwrap();
    assert_eq!(old["body"], "first revision");
    let edges = reopen
        .relationship_command(
            "list",
            json!({"contractVersion":1,"workspaceId":fresh,"includeHistory":true}),
        )
        .unwrap();
    assert_eq!(edges["relationships"].as_array().unwrap().len(), 1);
    assert_eq!(
        fs::read(
            root.join("content")
                .join(&w)
                .join(first["memory"]["id"].as_str().unwrap())
                .join(format!(
                    "{}.md",
                    next["memory"]["revisionId"].as_str().unwrap()
                ))
        )
        .unwrap(),
        before
    );
    let again = store.restore_snapshot(
        &w,
        &job.id,
        &artifact,
        "Recovered workspace",
        preview["sha256"].as_str().unwrap(),
    );
    assert!(again.is_err());
    assert!(
        store
            .restore_snapshot(
                &other,
                &job.id,
                &artifact,
                "Bad restore",
                preview["sha256"].as_str().unwrap()
            )
            .is_err()
    );
}

#[test]
fn schedule_requires_all_consents_counts_missed_runs_and_fences_two_processes() {
    use anansi_core::{Core, backup_commands::ScheduleRequest, storage::Storage};
    use serde_json::json;
    let temp = tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());
    let w = workspace(&core, "Schedule fixture");
    let store = Storage::new(root.clone()).unwrap();
    assert!(!store.read_backup_schedule(&w).unwrap().enabled);
    let request = json!({"contractVersion":1,"workspaceId":w,"enabled":true,"intervalSeconds":3600,"plaintextConsent":false,"keepAllVersions":true,"managedDestination":true});
    assert!(
        store
            .set_backup_schedule(
                serde_json::from_value::<ScheduleRequest>(request.clone()).unwrap()
            )
            .is_err()
    );
    let mut allowed = request;
    allowed["plaintextConsent"] = json!(true);
    let schedule = store
        .set_backup_schedule(serde_json::from_value(allowed).unwrap())
        .unwrap();
    let now = schedule.next_due.unwrap() + 7200;
    assert_eq!(store.tick_backup_schedules(now).unwrap(), 1);
    assert_eq!(
        Storage::new(root)
            .unwrap()
            .tick_backup_schedules(now)
            .unwrap(),
        0
    );
    assert_eq!(store.read_backup_schedule(&w).unwrap().missed_runs, 2);
    let jobs = store.list_jobs(&w, 0).unwrap();
    assert_eq!(jobs.len(), 1);
    assert!(matches!(
        jobs[0].selection,
        anansi_core::jobs::ExportSelection::Backup { application: false }
    ));
}

#[test]
fn application_backup_has_all_workspaces_and_completed_artifacts() {
    use anansi_core::{Core, jobs::ExportSelection, storage::Storage};
    use serde_json::json;
    let temp = tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());
    let w = workspace(&core, "Application first");
    let _other = workspace(&core, "Application second");
    let store = Storage::new(root).unwrap();
    let export = store
        .enqueue_export(&w, ExportSelection::Workspace)
        .unwrap();
    let lease = store.claim_export().unwrap().unwrap();
    store.execute_export(&lease).unwrap();
    assert_eq!(
        store.read_job(&w, &export.id).unwrap().state,
        anansi_core::jobs::JobState::Completed
    );
    let job = store
        .enqueue_export(&w, ExportSelection::Backup { application: true })
        .unwrap();
    let lease = store.claim_export().unwrap().unwrap();
    store.execute_backup(&lease).unwrap();
    let complete = store.read_job(&w, &job.id).unwrap();
    let preview=core.job_command("preview",json!({"contractVersion":1,"workspaceId":w,"jobId":job.id,"artifactId":complete.artifact_id})).unwrap();
    assert_eq!(preview["workspaceCount"], 2);
    assert_eq!(preview["application"], true);
    let restored = store
        .restore_snapshot(
            &w,
            &job.id,
            complete.artifact_id.as_deref().unwrap(),
            "App recovered",
            preview["sha256"].as_str().unwrap(),
        )
        .unwrap();
    assert_eq!(restored.workspaces.len(), 2);
    let mut completed_artifacts = 0;
    for fresh in restored.workspaces {
        assert!(!store.read_backup_schedule(&fresh.id).unwrap().enabled);
        for j in store.list_jobs(&fresh.id, 0).unwrap() {
            if j.state == anansi_core::jobs::JobState::Completed {
                assert!(
                    !store
                        .read_export_chunk(&fresh.id, &j.id, j.artifact_id.as_deref().unwrap(), 0)
                        .unwrap()
                        .is_empty()
                );
                completed_artifacts += 1;
            }
        }
    }
    assert_eq!(completed_artifacts, 1);
}

#[test]
fn verifies_members_and_restores_only_into_a_new_target() {
    let root = tempdir().unwrap();
    let source = root.path().join("source");
    let input = root.path().join("input.txt");
    let target = root.path().join("target");
    fs::create_dir_all(&source).unwrap();
    fs::write(&input, b"portable memory").unwrap();
    let member = backup::copy_member(&input, &source, "records.txt").unwrap();
    let manifest = BackupManifest {
        format: BACKUP_FORMAT.to_owned(),
        format_version: BACKUP_VERSION,
        workspace_id: "0123456789abcdef0123456789abcdef".to_owned(),
        workspace_name: "Synthetic workspace".to_owned(),
        created_at: Utc::now().to_rfc3339(),
        members: vec![member],
    };
    backup::write_manifest(&source, &manifest).unwrap();
    let preview = restore::preview(&source).unwrap();
    assert!(preview.verified);
    let restored = restore::restore_to_new_target(&source, &target).unwrap();
    assert_eq!(restored.total_bytes, preview.total_bytes);
    assert_ne!(
        restored.manifest.workspace_id,
        preview.manifest.workspace_id
    );
    assert_eq!(
        fs::read(target.join("records.txt")).unwrap(),
        b"portable memory"
    );
    assert!(restore::restore_to_new_target(&source, &target).is_err());
}

#[test]
fn refuses_tampered_members_and_duplicate_paths() {
    let root = tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("body.md"), b"original").unwrap();
    let member = backup::copy_member(&source.join("body.md"), &source, "body-copy.md").unwrap();
    fs::write(source.join("body-copy.md"), b"tampered").unwrap();
    let manifest = BackupManifest {
        format: BACKUP_FORMAT.to_owned(),
        format_version: BACKUP_VERSION,
        workspace_id: "0123456789abcdef0123456789abcdef".to_owned(),
        workspace_name: "Synthetic workspace".to_owned(),
        created_at: Utc::now().to_rfc3339(),
        members: vec![member.clone(), member],
    };
    backup::write_manifest(&source, &manifest).unwrap();
    assert!(restore::preview(&source).is_err());
}

#[test]
fn refuses_manifest_member_and_source_target_collision() {
    let root = tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir_all(&source).unwrap();
    assert!(backup::copy_member(&source.join("missing"), &source, "manifest.json").is_err());
    assert!(restore::restore_to_new_target(&source, &source).is_err());
}

#[test]
fn writes_and_reads_workspace_scoped_catalog() {
    let root = tempdir().unwrap();
    let catalog = BackupCatalog {
        format: BACKUP_FORMAT.to_owned(),
        format_version: BACKUP_VERSION,
        workspace_id: "0123456789abcdef0123456789abcdef".to_owned(),
        workspace_name: "Workspace A".to_owned(),
        settings: serde_json::json!({"reducedMotion": true}),
        records: vec![serde_json::json!({"id":"record-a"})],
        relationships: vec![],
    };
    let member = backup::write_catalog(root.path(), &catalog).unwrap();
    assert_eq!(member.path, "catalog.json");
    assert_eq!(backup::read_catalog(root.path()).unwrap(), catalog);
}
