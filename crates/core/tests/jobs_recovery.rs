use anansi_core::{
    Core, ErrorCode,
    jobs::{ExportSelection, JobAction, JobState},
    storage::{DATABASE_NAME, Storage},
};
use serde_json::json;

fn setup() -> (tempfile::TempDir, Storage, String) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());
    let workspace = core
        .workspace_command("create", json!({"contractVersion":1,"name":"Job fixtures"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    (temp, Storage::new(root).unwrap(), workspace)
}

#[test]
fn all_states_versions_and_explicit_recovery_are_durable() {
    let (temp, store, w) = setup();
    let queued = store
        .enqueue_export(&w, ExportSelection::Workspace)
        .unwrap();
    assert_eq!(queued.state, JobState::Queued);
    let paused = store
        .control_job(&w, &queued.id, queued.version, JobAction::Pause)
        .unwrap();
    assert_eq!(paused.state, JobState::Paused);
    assert_eq!(
        store
            .control_job(&w, &queued.id, queued.version, JobAction::Cancel)
            .unwrap_err()
            .code,
        ErrorCode::RevisionConflict
    );
    let resumed = store
        .control_job(&w, &queued.id, paused.version, JobAction::Resume)
        .unwrap();
    let lease = store.claim_export().unwrap().unwrap();
    assert_eq!(lease.job.id, resumed.id);
    assert_eq!(lease.job.state, JobState::Running);
    assert!(store.claim_export().unwrap().is_none());
    store.checkpoint_export(&lease, 3).unwrap();
    let db = rusqlite::Connection::open(temp.path().join("store").join(DATABASE_NAME)).unwrap();
    db.execute("UPDATE jobs SET lease_until=0 WHERE id=?1", [&queued.id])
        .unwrap();
    let restarted = Storage::new(temp.path().join("store")).unwrap();
    assert_eq!(restarted.recover_jobs().unwrap(), 1);
    let interrupted = restarted.read_job(&w, &queued.id).unwrap();
    assert_eq!(interrupted.state, JobState::Paused);
    assert_eq!(interrupted.progress, 3);
    assert!(interrupted.checkpoint_version > queued.checkpoint_version);
    assert!(store.checkpoint_export(&lease, 4).is_err());
    assert!(store.execute_export(&lease).is_err());
    let cancelled = restarted
        .control_job(&w, &queued.id, interrupted.version, JobAction::Cancel)
        .unwrap();
    assert_eq!(cancelled.state, JobState::Cancelled);
    assert!(cancelled.artifact_id.is_none());
    restarted
        .control_job(&w, &queued.id, cancelled.version, JobAction::Retry)
        .unwrap();
    let retry = restarted.claim_export().unwrap().unwrap();
    restarted.fail_export(&retry).unwrap();
    let failed = restarted.read_job(&w, &queued.id).unwrap();
    assert_eq!(failed.state, JobState::Failed);
    assert!(!failed.log.contains("synthetic-private-body"));
    restarted
        .control_job(&w, &queued.id, failed.version, JobAction::Retry)
        .unwrap();
    let final_lease = restarted.claim_export().unwrap().unwrap();
    restarted.execute_export(&final_lease).unwrap();
    let completed = restarted.read_job(&w, &queued.id).unwrap();
    assert_eq!(completed.state, JobState::Completed);
    assert!(
        restarted
            .control_job(&w, &queued.id, completed.version, JobAction::Retry)
            .is_err()
    );
    assert!(restarted.execute_export(&lease).is_err());
}

#[test]
fn competing_workers_claim_once_and_cancel_fences_publication() {
    let (_temp, store, w) = setup();
    let job = store
        .enqueue_export(&w, ExportSelection::Workspace)
        .unwrap();
    let other = store.clone();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let b = barrier.clone();
    let thread = std::thread::spawn(move || {
        b.wait();
        other.claim_export().unwrap()
    });
    barrier.wait();
    let first = store.claim_export().unwrap();
    let second = thread.join().unwrap();
    assert_ne!(first.is_some(), second.is_some());
    let lease = first.or(second).unwrap();
    let current = store.read_job(&w, &job.id).unwrap();
    store
        .control_job(&w, &job.id, current.version, JobAction::Cancel)
        .unwrap();
    assert!(store.execute_export(&lease).is_err());
    let final_job = store.read_job(&w, &job.id).unwrap();
    assert_eq!(final_job.state, JobState::Cancelled);
    assert!(final_job.artifact_id.is_none());
}

#[test]
fn queue_quota_and_cross_workspace_ownership() {
    let (_temp, store, w) = setup();
    let other = store.create("Other jobs").unwrap().workspace.id;
    let first = store
        .enqueue_export(&w, ExportSelection::Workspace)
        .unwrap();
    assert_eq!(
        store.read_job(&other, &first.id).unwrap_err().code,
        ErrorCode::NotFound
    );
    assert!(store.list_jobs(&other, 0).unwrap().is_empty());
    for _ in 1..32 {
        store
            .enqueue_export(&w, ExportSelection::Workspace)
            .unwrap();
    }
    assert!(
        store
            .enqueue_export(&w, ExportSelection::Workspace)
            .is_err()
    );
    assert!(
        store
            .enqueue_export(
                &w,
                ExportSelection::Records {
                    ids: vec!["../escape".into()]
                }
            )
            .is_err()
    );
}

#[test]
fn schema_six_upgrade_is_forward_only_and_failure_atomic() {
    let (temp, store, w) = setup();
    let file = temp.path().join("store").join(DATABASE_NAME);
    let db = rusqlite::Connection::open(&file).unwrap();
    db.execute_batch("DROP TABLE jobs; DROP TABLE backup_schedules; DROP TABLE restore_origins; DROP TABLE IF EXISTS backup_uploads; PRAGMA user_version=6;")
        .unwrap();
    assert_eq!(store.open(&w).unwrap().workspace.id, w);
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        8
    );
    db.execute_batch("DROP TABLE jobs; CREATE TABLE jobs(wrong TEXT); PRAGMA user_version=6;")
        .unwrap();
    assert!(store.open(&w).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM workspaces", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn interruption_child() {
    let Ok(root) = std::env::var("ANANSI_JOB_CRASH_ROOT") else {
        return;
    };
    let storage = Storage::new(root.into()).unwrap();
    let lease = storage.claim_export().unwrap().unwrap();
    storage.checkpoint_export(&lease, 2).unwrap();
    std::process::exit(86);
}

#[test]
fn actual_worker_process_termination_retains_checkpoint_and_requires_resume() {
    let (temp, storage, w) = setup();
    let job = storage
        .enqueue_export(&w, ExportSelection::Workspace)
        .unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "interruption_child"])
        .env("ANANSI_JOB_CRASH_ROOT", temp.path().join("store"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(86));
    let running = storage.read_job(&w, &job.id).unwrap();
    assert_eq!(running.state, JobState::Running);
    assert_eq!(running.progress, 2);
    // Advance only this synthetic catalog's lease clock instead of waiting two minutes.
    let db = rusqlite::Connection::open(temp.path().join("store").join(DATABASE_NAME)).unwrap();
    db.execute("UPDATE jobs SET lease_until=0 WHERE id=?1", [&job.id])
        .unwrap();
    storage.recover_jobs().unwrap();
    let paused = storage.read_job(&w, &job.id).unwrap();
    assert_eq!(paused.state, JobState::Paused);
    assert!(storage.claim_export().unwrap().is_none());
    storage
        .control_job(&w, &job.id, paused.version, JobAction::Resume)
        .unwrap();
    let resumed = storage.claim_export().unwrap().unwrap();
    storage.execute_export(&resumed).unwrap();
    assert_eq!(
        storage.read_job(&w, &job.id).unwrap().state,
        JobState::Completed
    );
}

#[test]
fn headless_notifications_run_across_roots_without_poll_commands() {
    let (first_temp, first_store, first_workspace) = setup();
    let (second_temp, second_store, second_workspace) = setup();
    let first = Core::with_storage_root(first_temp.path().join("store"));
    let second = Core::with_storage_root(second_temp.path().join("store"));
    let a=first.job_command("export",json!({"contractVersion":1,"workspaceId":first_workspace,"selection":{"kind":"workspace"}})).unwrap()["job"]["id"].as_str().unwrap().to_string();
    let b=second.job_command("export",json!({"contractVersion":1,"workspaceId":second_workspace,"selection":{"kind":"workspace"}})).unwrap()["job"]["id"].as_str().unwrap().to_string();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        // Direct storage reads deliberately do not wake a worker, unlike UI/API list.
        let a = first_store.read_job(&first_workspace, &a).unwrap();
        let b = second_store.read_job(&second_workspace, &b).unwrap();
        if a.state == JobState::Completed && b.state == JobState::Completed {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Accepted headless jobs must not need another command to wake them"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
