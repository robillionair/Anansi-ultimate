use anansi_core::{
    Core,
    jobs::ExportSelection,
    storage::{DATABASE_NAME, Storage},
    vault::{
        VaultError,
        catalog::{BackupReference, VaultStore},
        recovery,
        session::{Caller, Sessions, Transport},
    },
};
use serde_json::json;
use std::{path::PathBuf, sync::Mutex, time::Instant};
static SERIAL: Mutex<()> = Mutex::new(());
const PASSWORD: &[u8] = b"public synthetic vault passphrase";

#[test]
fn epoch_worker() {
    let Ok(root) = std::env::var("ANANSI_VAULT_EPOCH_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let marker = PathBuf::from(std::env::var("ANANSI_VAULT_EPOCH_MARKER").unwrap());
    let host = PathBuf::from(std::env::var("ANANSI_VAULT_TEST_HOST").unwrap());
    let workspace = std::env::var("ANANSI_VAULT_TEST_WORKSPACE").unwrap();
    let store = VaultStore::new(root, host).unwrap();
    let c = caller(&store, &workspace);
    let mut sessions = Sessions::default();
    let token = store
        .unlock(&mut sessions, &c, PASSWORD, Instant::now())
        .unwrap();
    std::fs::write(marker.join("ready"), b"ready").unwrap();
    let deadline = Instant::now() + std::time::Duration::from_secs(90);
    while !marker.join("revoked").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        store.read(&mut sessions, &c, &token, Instant::now()).err(),
        Some(VaultError::Locked)
    );
}

#[test]
fn admission_worker() {
    use anansi_core::vault::admission::Admission;
    let Ok(root) = std::env::var("ANANSI_VAULT_TEST_LOCK") else {
        return;
    };
    let root = PathBuf::from(root);
    let kind = std::env::var("ANANSI_VAULT_TEST_KIND").unwrap();
    let acquired = if kind == "root" {
        Admission::root(&root)
    } else {
        Admission::kdf(&root)
    };
    match std::env::var("ANANSI_VAULT_TEST_EXPECT").unwrap().as_str() {
        "busy" => assert!(matches!(acquired, Err(VaultError::Busy))),
        "hold" => {
            let _lease = acquired.unwrap();
            std::fs::write(root.join("ready"), b"ready").unwrap();
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
        _ => {
            let _lease = acquired.unwrap();
        }
    }
}

#[test]
fn independent_process_admission_and_killed_owner_release() {
    use anansi_core::vault::admission::Admission;
    for kind in ["root", "kdf"] {
        let dir = tempfile::tempdir().unwrap();
        anansi_core::storage::private_permissions(dir.path(), true, false).unwrap();
        let spawn = |expected: &str| {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "admission_worker", "--nocapture"])
                .env("ANANSI_VAULT_TEST_LOCK", dir.path())
                .env("ANANSI_VAULT_TEST_KIND", kind)
                .env("ANANSI_VAULT_TEST_EXPECT", expected)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap()
        };
        let lease = if kind == "root" {
            Admission::root(dir.path())
        } else {
            Admission::kdf(dir.path())
        }
        .unwrap();
        assert!(spawn("busy").wait().unwrap().success());
        drop(lease);
        assert!(spawn("free").wait().unwrap().success());
        let mut owner = spawn("hold");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while !dir.path().join("ready").exists() {
            assert!(std::time::Instant::now() < deadline);
            assert!(owner.try_wait().unwrap().is_none());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(spawn("busy").wait().unwrap().success());
        owner.kill().unwrap();
        owner.wait().unwrap();
        assert!(spawn("free").wait().unwrap().success());
        assert!(
            dir.path()
                .join(if kind == "root" {
                    "vault.operation.lock"
                } else {
                    "vault.kdf.lock"
                })
                .exists()
        );
    }
}

#[test]
fn crash_worker() {
    use anansi_core::vault::catalog::Phase;
    let Ok(root) = std::env::var("ANANSI_VAULT_TEST_CRASH_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let host = PathBuf::from(std::env::var("ANANSI_VAULT_TEST_HOST").unwrap());
    let w = std::env::var("ANANSI_VAULT_TEST_WORKSPACE").unwrap();
    let mode = std::env::var("ANANSI_VAULT_TEST_CRASH").unwrap();
    let backup = BackupReference {
        job_id: std::env::var("ANANSI_VAULT_TEST_JOB").unwrap(),
        artifact_id: std::env::var("ANANSI_VAULT_TEST_ARTIFACT").unwrap(),
        sha256: std::env::var("ANANSI_VAULT_TEST_HASH").unwrap(),
    };
    let observe = |phase: Phase| {
        if mode.ends_with(&format!("{phase:?}")) {
            std::process::exit(86);
        }
        Ok(())
    };
    let store = VaultStore::new(root, host).unwrap();
    let (ack, recovery_text) = setup(&store, &w);
    if mode.starts_with("seal/") {
        store.seal_observed(ack, &backup, true, observe).unwrap();
    } else {
        store.seal_workspace(ack, &backup, true).unwrap();
        let c = caller(&store, &w);
        let mut sessions = Sessions::default();
        let token = store
            .unlock(&mut sessions, &c, PASSWORD, Instant::now())
            .unwrap();
        let mut rotation = store
            .rotate(
                &mut sessions,
                &c,
                &token,
                b"new public synthetic passphrase",
                Instant::now(),
            )
            .unwrap();
        let shown = rotation.display_once().unwrap();
        let rotation = rotation
            .acknowledge(&shown.text, &shown.challenge, true)
            .unwrap();
        if mode.starts_with("rotate/") {
            store
                .finish_rotation(rotation, &mut sessions, Instant::now(), observe)
                .unwrap();
        } else {
            store
                .finish_rotation(rotation, &mut sessions, Instant::now(), |_| Ok(()))
                .unwrap();
            store
                .rollback(
                    &w,
                    c.context.generation_id(),
                    &recovery::parse(&recovery_text).unwrap(),
                    &mut sessions,
                    observe,
                )
                .unwrap();
        }
    }
    panic!("The selected crash point must be reached");
}

fn copy_fixture(source: &std::path::Path, target: &std::path::Path) {
    assert!(!target.exists());
    std::fs::create_dir(target).unwrap();
    anansi_core::storage::private_permissions(target, true, false).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        assert!(!entry.file_type().unwrap().is_symlink());
        let path = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture(&entry.path(), &path);
        } else {
            std::fs::copy(entry.path(), &path).unwrap();
            anansi_core::storage::private_permissions(&path, false, false).unwrap();
        }
    }
}

#[test]
fn deterministic_process_crash_cutover_and_rollback_matrix() {
    let _serial = SERIAL.lock().unwrap();
    let f = fixture(&["note", "project"]);
    rusqlite::Connection::open(f.root.join(DATABASE_NAME))
        .unwrap()
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .unwrap();
    let cases = [
        "seal/PreJournal",
        "seal/MidStage",
        "seal/PreSwitch",
        "seal/PostSwitch",
        "rotate/PreJournal",
        "rotate/MidStage",
        "rotate/PreSwitch",
        "rotate/PostSwitch",
        "rollback/PreRollback",
        "rollback/PostRollback",
    ];
    for (index, case) in cases.iter().enumerate() {
        let root = f._temp.path().join(format!("crash-{index}"));
        copy_fixture(&f.root, &root);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "crash_worker", "--nocapture"])
            .env("ANANSI_VAULT_TEST_CRASH_ROOT", &root)
            .env("ANANSI_VAULT_TEST_HOST", &f.host)
            .env("ANANSI_VAULT_TEST_WORKSPACE", &f.w)
            .env("ANANSI_VAULT_TEST_JOB", &f.backup.job_id)
            .env("ANANSI_VAULT_TEST_ARTIFACT", &f.backup.artifact_id)
            .env("ANANSI_VAULT_TEST_HASH", &f.backup.sha256)
            .env("ANANSI_VAULT_TEST_CRASH", case)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(86), "{case}");
        let db = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
        let schema = db
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap();
        let committed = !case.starts_with("seal/") || case.ends_with("PostSwitch");
        assert_eq!(schema, if committed { 9 } else { 8 }, "{case}");
        if committed {
            let store = VaultStore::new(root.clone(), f.host.clone()).unwrap();
            let c = caller(&store, &f.w);
            let mut sessions = Sessions::default();
            let password = if *case == "rotate/PostSwitch" || *case == "rollback/PreRollback" {
                b"new public synthetic passphrase".as_slice()
            } else {
                PASSWORD
            };
            let token = store
                .unlock(&mut sessions, &c, password, Instant::now())
                .unwrap();
            assert_eq!(
                store
                    .read(&mut sessions, &c, &token, Instant::now())
                    .unwrap()
                    .records
                    .len(),
                4,
                "{case}"
            );
            let epoch: i64 = db
                .query_row("SELECT epoch FROM vaults", [], |r| r.get(0))
                .unwrap();
            assert_eq!(
                epoch,
                if *case == "rollback/PostRollback" {
                    3
                } else if *case == "rotate/PostSwitch" || *case == "rollback/PreRollback" {
                    2
                } else {
                    1
                },
                "{case}"
            );
        } else {
            assert_eq!(
                db.query_row("SELECT count(*) FROM memories", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                2
            );
        }
        assert_eq!(
            db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert!(root.join("vault-generations").exists());
        eprintln!("Verified process-crash boundary: {case}");
    }
    assert_eq!(
        rusqlite::Connection::open(f.root.join(DATABASE_NAME))
            .unwrap()
            .query_row("SELECT count(*) FROM memories", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    host: PathBuf,
    w: String,
    backup: BackupReference,
}
fn fixture(kinds: &[&str]) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("catalog");
    let host = temp.path().join("admission");
    std::fs::create_dir(&host).unwrap();
    anansi_core::storage::private_permissions(&host, true, false).unwrap();
    let core = Core::with_storage_root(root.clone());
    let w = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Synthetic vault source"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut ids = Vec::new();
    for kind in kinds {
        let first=core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"kind":kind,"draft":{"title":format!("private canary {kind}"),"body":format!("original {kind} body")}})).unwrap();
        let id = first["memory"]["id"].as_str().unwrap().to_owned();
        core.memory_command("revise",json!({"contractVersion":1,"workspaceId":w,"id":id,"expectedRevision":first["memory"]["revisionId"],"kind":kind,"draft":{"title":format!("private canary {kind}"),"body":format!("latest {kind} body")}})).unwrap();
        ids.push(id);
    }
    if ids.len() > 1 {
        let created = core.relationship_command("create",json!({"contractVersion":1,"workspaceId":w,"draft":{"sourceId":ids[0],"targetId":ids[1],"predicate":"relates_to","metadata":{"label":"private graph canary"}}})).unwrap();
        let edge_id = created["relationship"]["id"].as_str().unwrap();
        core.relationship_command(
            "delete",
            json!({"contractVersion":1,"workspaceId":w,"id":edge_id,"expectedVersion":1}),
        )
        .unwrap();
        core.relationship_command(
            "restore",
            json!({"contractVersion":1,"workspaceId":w,"id":edge_id,"expectedVersion":2}),
        )
        .unwrap();
    }
    let storage = Storage::new(root.clone()).unwrap();
    let job = storage
        .enqueue_export(&w, ExportSelection::Backup { application: false })
        .unwrap();
    let lease = storage.claim_export().unwrap().unwrap();
    storage.execute_backup(&lease).unwrap();
    let complete = storage.read_job(&w, &job.id).unwrap();
    let backup = BackupReference {
        job_id: job.id,
        artifact_id: complete.artifact_id.unwrap(),
        sha256: complete.artifact_hash.unwrap(),
    };
    Fixture {
        _temp: temp,
        root,
        host,
        w,
        backup,
    }
}
fn caller(store: &VaultStore, w: &str) -> Caller {
    let (context, _) = store.context(w).unwrap();
    Caller {
        context,
        root_id: store.root_id().unwrap(),
        caller_id: "native-test".into(),
        transport: Transport::Desktop,
    }
}
fn setup(
    store: &VaultStore,
    w: &str,
) -> (
    anansi_core::vault::recovery::AcknowledgedSetup,
    zeroize::Zeroizing<String>,
) {
    let mut pending = store.begin(w, PASSWORD).unwrap();
    let shown = pending.display_once().unwrap();
    assert!(pending.display_once().is_err());
    let text = shown.text;
    let receipt = pending.acknowledge(&text, &shown.challenge, true).unwrap();
    (receipt, text)
}

#[test]
fn base32_canonical_vectors_and_padding() {
    let zero = recovery::format(&[0; 32]);
    assert_eq!(zero.replace('-', ""), "A".repeat(52));
    assert_eq!(&*recovery::parse(&zero).unwrap(), &[0; 32]);
    let ones = recovery::format(&[255; 32]);
    assert_eq!(ones.replace('-', ""), format!("{}Q", "7".repeat(51)));
    assert!(recovery::parse(&ones.to_lowercase()).is_err());
    let mut bad = zero.to_string();
    bad.pop();
    bad.push('B');
    assert!(recovery::parse(&bad).is_err());
    assert!(recovery::parse(&zero.replace('-', "")).is_err());
    let bytes = std::array::from_fn(|i| i as u8);
    assert_eq!(
        &*recovery::parse(&recovery::format(&bytes)).unwrap(),
        &bytes
    );
}

#[test]
fn acknowledgement_and_stale_backup_refuse_activation() {
    let _serial = SERIAL.lock().unwrap();
    let f = fixture(&["note"]);
    let store = VaultStore::new(f.root.clone(), f.host.clone()).unwrap();
    let pending = store.begin(&f.w, PASSWORD).unwrap();
    assert!(matches!(
        pending.acknowledge(&recovery::format(&[0; 32]), &[0; 16], true),
        Err(VaultError::AcknowledgementRequired)
    ));
    for mode in ["unsaved", "challenge", "key"] {
        let mut pending = store.begin(&f.w, PASSWORD).unwrap();
        let shown = pending.display_once().unwrap();
        let mut challenge = shown.challenge;
        if mode == "challenge" {
            challenge[0] ^= 1;
        }
        let wrong = recovery::format(&[0; 32]);
        let text = if mode == "key" {
            wrong.as_str()
        } else {
            shown.text.as_str()
        };
        assert!(
            pending
                .acknowledge(text, &challenge, mode != "unsaved")
                .is_err()
        );
    }
    let (ack, _) = setup(&store, &f.w);
    assert_eq!(
        store.seal_workspace(ack, &f.backup, false),
        Err(VaultError::AcknowledgementRequired)
    );
    let core = Core::with_storage_root(f.root.clone());
    core.memory_command("create", json!({"contractVersion":1,"workspaceId":f.w,"kind":"note","draft":{"title":"Changed after backup","body":"preserved unsealed content"}})).unwrap();
    let (ack, _) = setup(&store, &f.w);
    assert_eq!(
        store.seal_workspace(ack, &f.backup, true),
        Err(VaultError::Conflict)
    );
    let db = rusqlite::Connection::open(f.root.join(DATABASE_NAME)).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        8
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM memories", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert!(!f.root.join("vault-generations").exists());
}

#[test]
fn canonical_history_graph_epoch_rotation_and_explicit_rollback() {
    let _serial = SERIAL.lock().unwrap();
    let f = fixture(anansi_core::entities::SUPPORTED_KINDS);
    let store = VaultStore::new(f.root.clone(), f.host.clone()).unwrap();
    let (ack, recovery) = setup(&store, &f.w);
    let stale_db = rusqlite::Connection::open(f.root.join(DATABASE_NAME)).unwrap();
    // Prepare under schema 8; SQLite must recheck the schema-9 barrier later.
    let mut stale_write = stale_db
        .prepare("INSERT INTO memories(workspace_id,id,current_revision) VALUES(?1,?2,?3)")
        .unwrap();
    store.seal_workspace(ack, &f.backup, true).unwrap();
    let mut first = Sessions::default();
    let mut second = Sessions::default();
    let c = caller(&store, &f.w);
    let token = store
        .unlock(&mut first, &c, PASSWORD, Instant::now())
        .unwrap();
    let other = store
        .unlock(&mut second, &c, PASSWORD, Instant::now())
        .unwrap();
    let marker = f._temp.path().join("epoch-handshake");
    std::fs::create_dir(&marker).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "epoch_worker", "--nocapture"])
        .env("ANANSI_VAULT_EPOCH_ROOT", &f.root)
        .env("ANANSI_VAULT_TEST_HOST", &f.host)
        .env("ANANSI_VAULT_TEST_WORKSPACE", &f.w)
        .env("ANANSI_VAULT_EPOCH_MARKER", &marker)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + std::time::Duration::from_secs(90);
    while !marker.join("ready").exists() {
        assert!(Instant::now() < deadline);
        assert!(child.try_wait().unwrap().is_none());
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let mut snapshot = store.read(&mut first, &c, &token, Instant::now()).unwrap();
    assert_eq!(snapshot.records.len(), 18);
    assert_eq!(snapshot.current_revisions.len(), 9);
    assert_eq!(snapshot.graph.len(), 3);
    assert!(snapshot.graph[1].archived);
    assert_eq!(
        snapshot.current_edges.values().copied().collect::<Vec<_>>(),
        vec![3]
    );
    assert!(
        snapshot
            .records
            .iter()
            .any(|r| r.body == "original note body")
    );
    snapshot.records[0].tags = vec!["private synthetic tag".into()];
    let encoded = snapshot.records[0].encode(&c.context).unwrap();
    let decoded = anansi_core::vault::payload::RecordPayload::decode(&encoded, &c.context).unwrap();
    assert_eq!(decoded.tags, vec!["private synthetic tag"]);
    snapshot.records[0].tags.clear();
    let body_hash = snapshot.records[0].body_hash.clone();
    snapshot.records[0].body_hash = "0".repeat(64);
    assert!(snapshot.validate(&c.context).is_err());
    snapshot.records[0].body_hash = body_hash;
    let removed = snapshot.records.remove(0);
    assert!(snapshot.validate(&c.context).is_err());
    snapshot.records.insert(0, removed);
    let edge = snapshot.graph.pop().unwrap();
    assert!(snapshot.validate(&c.context).is_err());
    snapshot.graph.push(edge);
    assert!(snapshot.validate(&c.context).is_ok());
    let db = rusqlite::Connection::open(f.root.join(DATABASE_NAME)).unwrap();
    for table in [
        "memories",
        "memory_revisions",
        "relationships",
        "relationship_revisions",
    ] {
        assert_eq!(
            db.query_row(
                &format!("SELECT count(*) FROM {table} WHERE workspace_id=?1"),
                [&f.w],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        9
    );
    let invalid_switch = db.unchecked_transaction().unwrap();
    invalid_switch
        .execute("UPDATE vaults SET active_generation=?1", ["0".repeat(32)])
        .unwrap();
    assert!(
        invalid_switch.commit().is_err(),
        "committed pointers must reference a retained generation"
    );
    assert_eq!(
        store.context(&f.w).unwrap().0.generation_id(),
        c.context.generation_id()
    );
    assert!(
        Core::with_storage_root(f.root.clone())
            .workspace_command("list", json!({"contractVersion":1}))
            .is_err()
    );
    assert!(
        stale_write
            .execute(rusqlite::params![f.w, "a".repeat(32), "b".repeat(32)])
            .unwrap_err()
            .to_string()
            .contains("vault-aware")
    );
    store.lock(&f.w, &mut first).unwrap();
    std::fs::write(marker.join("revoked"), b"revoked").unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(
        store.read(&mut second, &c, &other, Instant::now()).err(),
        Some(VaultError::Locked)
    );
    let token = store
        .unlock(&mut first, &c, PASSWORD, Instant::now())
        .unwrap();
    let mut stale_rotation = store
        .rotate(
            &mut first,
            &c,
            &token,
            b"abandoned synthetic passphrase",
            Instant::now(),
        )
        .unwrap();
    let shown = stale_rotation.display_once().unwrap();
    let stale_rotation = stale_rotation
        .acknowledge(&shown.text, &shown.challenge, true)
        .unwrap();
    store.lock(&f.w, &mut second).unwrap();
    assert_eq!(
        store.finish_rotation(stale_rotation, &mut first, Instant::now(), |_| Ok(())),
        Err(VaultError::Conflict)
    );
    assert_eq!(
        store.context(&f.w).unwrap().0.generation_id(),
        c.context.generation_id()
    );
    let token = store
        .unlock(&mut first, &c, PASSWORD, Instant::now())
        .unwrap();
    let mut expired = store
        .rotate(
            &mut first,
            &c,
            &token,
            b"expired synthetic passphrase",
            Instant::now(),
        )
        .unwrap();
    let shown = expired.display_once().unwrap();
    let expired = expired
        .acknowledge(&shown.text, &shown.challenge, true)
        .unwrap();
    assert_eq!(
        store.finish_rotation(
            expired,
            &mut first,
            Instant::now() + std::time::Duration::from_secs(901),
            |_| Ok(())
        ),
        Err(VaultError::Locked)
    );
    let token = store
        .unlock(&mut first, &c, PASSWORD, Instant::now())
        .unwrap();
    let mut rotation = store
        .rotate(
            &mut first,
            &c,
            &token,
            b"new public synthetic passphrase",
            Instant::now(),
        )
        .unwrap();
    let shown = rotation.display_once().unwrap();
    let rotation = rotation
        .acknowledge(&shown.text, &shown.challenge, true)
        .unwrap();
    store
        .finish_rotation(rotation, &mut first, Instant::now(), |_| Ok(()))
        .unwrap();
    assert!(store.read(&mut second, &c, &other, Instant::now()).is_err());
    let next = caller(&store, &f.w);
    assert_ne!(next.context.generation_id(), c.context.generation_id());
    let read_slots = |generation: &str| {
        let (passphrase, recovery) = db.query_row(
            "SELECT passphrase_slot,recovery_slot FROM vault_generations WHERE generation_id=?1", [generation],
            |r| Ok((r.get::<_,Vec<u8>>(0)?, r.get::<_,Vec<u8>>(1)?))).unwrap();
        anansi_core::vault::keys::KeySlots {
            passphrase,
            recovery,
        }
    };
    let old_dek = read_slots(c.context.generation_id())
        .unlock(&c.context, PASSWORD, false)
        .unwrap();
    let new_dek = read_slots(next.context.generation_id())
        .unlock(&next.context, b"new public synthetic passphrase", false)
        .unwrap();
    assert_ne!(
        *old_dek, *new_dek,
        "rotation must replace the DEK, not just its wrapping slots"
    );
    assert!(
        store
            .unlock(&mut first, &next, PASSWORD, Instant::now())
            .is_err()
    );
    let token = store
        .unlock(
            &mut first,
            &next,
            b"new public synthetic passphrase",
            Instant::now(),
        )
        .unwrap();
    assert_eq!(
        store
            .read(&mut first, &next, &token, Instant::now())
            .unwrap()
            .records
            .len(),
        18
    );
    store
        .rollback(
            &f.w,
            c.context.generation_id(),
            &recovery::parse(&recovery).unwrap(),
            &mut first,
            |_| Ok(()),
        )
        .unwrap();

    let token = store
        .unlock(&mut first, &c, PASSWORD, Instant::now())
        .unwrap();
    let backup = store.ciphertext_backup(&f.w).unwrap();
    let header_len = u32::from_be_bytes(backup[8..12].try_into().unwrap()) as usize;
    let header: serde_json::Value = serde_json::from_slice(&backup[12..12 + header_len]).unwrap();
    let rebuild = |header: &serde_json::Value| {
        let raw = serde_json::to_vec(header).unwrap();
        let mut bytes = backup[..8].to_vec();
        bytes.extend_from_slice(&(raw.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&raw);
        bytes.extend_from_slice(&backup[12 + header_len..]);
        bytes
    };
    let mut missing = header.clone();
    missing["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["token"] != "manifest");
    assert!(anansi_core::vault::archive::CiphertextBackup::decode(&rebuild(&missing)).is_err());
    let mut duplicate = header.clone();
    duplicate["files"]
        .as_array_mut()
        .unwrap()
        .push(header["files"][0].clone());
    assert!(anansi_core::vault::archive::CiphertextBackup::decode(&rebuild(&duplicate)).is_err());
    let mut trailing = backup.clone();
    trailing.push(0);
    assert!(anansi_core::vault::archive::CiphertextBackup::decode(&trailing).is_err());
    let mut excessive = header.clone();
    excessive["files"][0]["bytes"] = json!(u64::MAX);
    assert!(anansi_core::vault::archive::CiphertextBackup::decode(&rebuild(&excessive)).is_err());
    let mut unknown = header.clone();
    unknown["unexpected"] = json!(true);
    assert!(anansi_core::vault::archive::CiphertextBackup::decode(&rebuild(&unknown)).is_err());
    assert!(
        !anansi_core::vault::archive::CiphertextBackup::preview(&backup)
            .unwrap()
            .authenticated
    );
    assert!(
        !backup
            .windows(b"private canary".len())
            .any(|s| s == b"private canary")
    );
    let exported = store
        .encrypted_export(
            &mut first,
            &c,
            &token,
            b"synthetic export password",
            Instant::now(),
        )
        .unwrap();
    assert!(
        store
            .decrypt_export(&exported, b"wrong export password")
            .is_err()
    );
    let unpacked = store
        .decrypt_export(&exported, b"synthetic export password")
        .unwrap();
    let destination = f._temp.path().join("destination");
    let landing = Core::with_storage_root(destination.clone());
    landing
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Existing untouched landing"}),
        )
        .unwrap();
    let restored_store = VaultStore::new(destination.clone(), f.host.clone()).unwrap();
    let mut swapped = header.clone();
    swapped["vaultId"] = json!("0".repeat(32));
    assert!(
        restored_store
            .prepare_restore(&rebuild(&swapped), PASSWORD, "Rejected", PASSWORD)
            .is_err()
    );
    assert!(
        restored_store
            .prepare_restore_recovery(&backup, &[0; 32], "Rejected", PASSWORD)
            .is_err()
    );
    // Recovery authentication succeeds without the forgotten source passphrase;
    // dropping the unacknowledged receipt still cannot activate a destination.
    drop(
        restored_store
            .prepare_restore_recovery(
                &backup,
                &recovery::parse(&recovery).unwrap(),
                "Recovery verified",
                PASSWORD,
            )
            .unwrap(),
    );
    assert!(
        restored_store
            .prepare_restore(&backup, b"wrong source key", "Rejected", PASSWORD)
            .is_err()
    );
    let mut tampered = backup.clone();
    *tampered.last_mut().unwrap() ^= 1;
    assert!(
        restored_store
            .prepare_restore(&tampered, PASSWORD, "Rejected", PASSWORD)
            .is_err()
    );
    let mut restore = restored_store
        .prepare_restore(
            &unpacked,
            b"synthetic export password",
            "Fresh sealed restore",
            b"fresh destination passphrase",
        )
        .unwrap();
    let shown = restore.display_once().unwrap();
    let ack = restore
        .acknowledge(&shown.text, &shown.challenge, true)
        .unwrap();
    let fresh = restored_store.finish_restore(ack, |_| Ok(())).unwrap();
    assert_ne!(fresh, f.w);
    let fresh_caller = caller(&restored_store, &fresh);
    let mut fresh_sessions = Sessions::default();
    let fresh_token = restored_store
        .unlock(
            &mut fresh_sessions,
            &fresh_caller,
            b"fresh destination passphrase",
            Instant::now(),
        )
        .unwrap();
    let recovered = restored_store
        .read(
            &mut fresh_sessions,
            &fresh_caller,
            &fresh_token,
            Instant::now(),
        )
        .unwrap();
    assert_eq!(recovered.records.len(), 18);
    assert_eq!(recovered.graph.len(), 3);
    assert!(recovered.graph[1].archived);
    assert!(
        recovered
            .records
            .iter()
            .all(|r| !snapshot.current_revisions.contains_key(&r.record_id))
    );
    assert!(
        recovered
            .records
            .iter()
            .any(|r| r.body == "original note body")
    );
    assert!(
        recovered
            .graph
            .iter()
            .all(|e| recovered.current_revisions.contains_key(&e.source_id)
                && recovered.current_revisions.contains_key(&e.target_id))
    );
    assert_eq!(recovered.origin.as_ref().unwrap().workspace_id, f.w);
    assert_eq!(store.ciphertext_backup(&f.w).unwrap(), backup);
    let dest_db = rusqlite::Connection::open(destination.join(DATABASE_NAME)).unwrap();
    assert_eq!(
        dest_db
            .query_row("SELECT count(*) FROM workspaces", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
}
