use anansi_core::{
    Core, ErrorCode,
    storage::{DATABASE_NAME, Storage, private_permissions},
    workspace::WorkspaceDetails,
};
use serde_json::json;
use std::fs;
fn create(core: &Core, name: &str) -> WorkspaceDetails {
    serde_json::from_value(
        core.workspace_command("create", json!({"contractVersion":1,"name":name}))
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn create_reopen_settings_and_independent_instances_survive_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    assert_eq!(
        core.workspace_command("list", json!({"contractVersion":1}))
            .unwrap()["workspaces"],
        json!([])
    );
    assert!(!root.exists());
    let original = create(&core, "Synthetic Research");
    assert!(!original.settings.reduced_motion);
    let updated = core
        .workspace_command(
            "settings",
            json!({"contractVersion":1,"id":original.workspace.id,"reducedMotion":true}),
        )
        .unwrap();
    assert_eq!(updated["settings"]["reducedMotion"], true);
    drop(core);
    let restarted = Core::with_storage_root(root);
    let opened = restarted
        .workspace_command(
            "open",
            json!({"contractVersion":1,"id":original.workspace.id}),
        )
        .unwrap();
    assert_eq!(opened["workspace"]["id"], original.workspace.id);
    assert_eq!(opened["settings"]["reducedMotion"], true);
    assert_eq!(
        restarted
            .workspace_command("list", json!({"contractVersion":1}))
            .unwrap()["workspaces"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn duplicate_creation_and_invalid_external_requests_leave_existing_data_intact() {
    let temp = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(temp.path().join("store"));
    let original = create(&core, "Research");
    assert_eq!(
        core.workspace_command("create", json!({"contractVersion":1,"name":"research"}))
            .unwrap_err()
            .code,
        ErrorCode::WorkspaceExists
    );
    for name in [
        "",
        "../escape",
        "C:\\absolute",
        "CON",
        "two/name",
        " name",
        "name ",
        "bad.name",
    ] {
        assert_eq!(
            core.workspace_command("create", json!({"contractVersion":1,"name":name}))
                .unwrap_err()
                .code,
            ErrorCode::InvalidWorkspace
        );
    }
    for (command, request) in [
        (
            "create",
            json!({"contractVersion":1,"name":"Good","path":"outside"}),
        ),
        (
            "settings",
            json!({"contractVersion":1,"id":original.workspace.id,"reducedMotion":"true"}),
        ),
        (
            "open",
            json!({"contractVersion":1,"id":original.workspace.id,"path":"outside"}),
        ),
    ] {
        assert_eq!(
            core.workspace_command(command, request).unwrap_err().code,
            ErrorCode::InvalidRequest
        );
    }
    for id in ["../escape", "C:/outside", "BAD", ""] {
        assert_eq!(
            core.workspace_command("open", json!({"contractVersion":1,"id":id}))
                .unwrap_err()
                .code,
            ErrorCode::InvalidWorkspace
        );
    }
    assert_eq!(
        core.workspace_command("open", json!({"contractVersion":1,"id":"0".repeat(32)}))
            .unwrap_err()
            .code,
        ErrorCode::WorkspaceNotFound
    );
    assert_eq!(
        core.workspace_command("list", json!({"contractVersion":2}))
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedContract
    );
    assert_eq!(
        core.workspace_command("delete", json!({"contractVersion":1}))
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    assert_eq!(
        core.workspace_command("list", json!({"contractVersion":1}))
            .unwrap()["workspaces"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn unknown_directories_and_in_repository_roots_are_never_claimed() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("unknown");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("existing.txt"), "original").unwrap();
    assert!(
        Storage::new(root.clone())
            .unwrap()
            .create("Synthetic")
            .is_err()
    );
    assert_eq!(
        fs::read_to_string(root.join("existing.txt")).unwrap(),
        "original"
    );
    assert!(!root.join(".anansi-root").exists());
    let repo = temp.path().join("repo");
    fs::create_dir(&repo).unwrap();
    fs::write(repo.join(".git"), "gitdir: elsewhere").unwrap();
    assert!(Storage::new(repo.join("nested/storage")).is_err());
    assert!(Storage::new("relative".into()).is_err());
}
#[test]
fn newer_and_corrupt_databases_are_not_rewritten_or_reset() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let store = Storage::new(root.clone()).unwrap();
    let original = store.create("Synthetic").unwrap();
    let path = root.join(DATABASE_NAME);
    fs::copy(&path, temp.path().join("preserved-original.sqlite3")).unwrap();
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.pragma_update(None, "user_version", 99).unwrap();
    }
    let newer = fs::read(&path).unwrap();
    assert_eq!(
        store.open(&original.workspace.id).unwrap_err().code,
        ErrorCode::UnsupportedSchema
    );
    assert_eq!(fs::read(&path).unwrap(), newer);
    fs::write(&path, b"synthetic corrupt database; not real user data").unwrap();
    let corrupt = fs::read(&path).unwrap();
    assert_eq!(
        store.open(&original.workspace.id).unwrap_err().code,
        ErrorCode::CorruptStorage
    );
    assert_eq!(fs::read(&path).unwrap(), corrupt);
}
#[test]
fn newer_live_wal_and_unknown_sqlite_are_inspected_read_only() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let storage = Storage::new(root.clone()).unwrap();
    let workspace = storage.create("Synthetic").unwrap();
    let file = root.join(DATABASE_NAME);
    {
        let connection = rusqlite::Connection::open(&file).unwrap();
        connection.pragma_update(None, "user_version", 3).unwrap();
        let wal = root.join(format!("{DATABASE_NAME}-wal"));
        let before = fs::read(&file).unwrap();
        let log = fs::read(&wal).unwrap();
        assert_eq!(
            storage.open(&workspace.workspace.id).unwrap_err().code,
            ErrorCode::UnsupportedSchema
        );
        assert_eq!(fs::read(&file).unwrap(), before);
        assert_eq!(fs::read(&wal).unwrap(), log);
    }
    {
        let connection = rusqlite::Connection::open(&file).unwrap();
        connection.pragma_update(None, "application_id", 0).unwrap();
    }
    let before = fs::read(&file).unwrap();
    assert_eq!(
        storage.open(&workspace.workspace.id).unwrap_err().code,
        ErrorCode::CorruptStorage
    );
    assert_eq!(fs::read(&file).unwrap(), before);
}
#[test]
fn concurrent_connections_preserve_all_workspaces_and_settings() {
    let temp = tempfile::tempdir().unwrap();
    let store = Storage::new(temp.path().join("store")).unwrap();
    let original = store.create("Original").unwrap();
    let workers: Vec<_> = (0..3)
        .map(|index| {
            let storage = store.clone();
            let id = original.workspace.id.clone();
            std::thread::spawn(move || {
                let workspace = storage.create(&format!("Concurrent {index}")).unwrap();
                storage.update(&workspace.workspace.id, true).unwrap();
                storage.open(&id).unwrap();
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(store.list().unwrap().len(), 4);
}
#[test]
fn read_only_permissions_fail_without_changing_existing_records() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let store = Storage::new(root.clone()).unwrap();
    let original = store.create("Synthetic").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o500)).unwrap();
    }
    #[cfg(windows)]
    {
        let result=std::process::Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command",r#"Import-Module (Join-Path $PSHOME 'Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1'); $acl=New-Object Security.AccessControl.DirectorySecurity; $sid=[Security.Principal.WindowsIdentity]::GetCurrent().User; $acl.SetOwner($sid); $acl.SetAccessRuleProtection($true,$false); $rule=New-Object Security.AccessControl.FileSystemAccessRule($sid,'ReadAndExecute,ChangePermissions,TakeOwnership,Synchronize','Allow'); $acl.AddAccessRule($rule); [IO.Directory]::SetAccessControl($env:ANANSI_TEST_ROOT,$acl)"#]).env("ANANSI_TEST_ROOT",&root).output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let result = store.update(&original.workspace.id, true);
    private_permissions(&root, true, false).unwrap();
    assert_eq!(result.unwrap_err().code, ErrorCode::StorageUnavailable);
    assert!(
        !store
            .open(&original.workspace.id)
            .unwrap()
            .settings
            .reduced_motion
    );
}
#[cfg(unix)]
#[test]
fn rejects_root_database_and_sidecar_symlink_escapes() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let store = Storage::new(root.clone()).unwrap();
    let original = store.create("Synthetic").unwrap();
    let alias = temp.path().join("alias");
    symlink(&root, &alias).unwrap();
    assert!(Storage::new(alias).is_err());
    let sidecar = root.join(format!("{DATABASE_NAME}-wal"));
    symlink(temp.path().join("outside"), &sidecar).unwrap();
    assert!(store.open(&original.workspace.id).is_err());
    fs::remove_file(sidecar).unwrap();
    let database = root.join(DATABASE_NAME);
    let saved = temp.path().join("saved.sqlite3");
    fs::rename(&database, &saved).unwrap();
    symlink(&saved, &database).unwrap();
    assert!(store.open(&original.workspace.id).is_err());
}
#[cfg(windows)]
#[test]
fn rejects_windows_storage_junction_escapes() {
    let temp = tempfile::tempdir().unwrap();
    let alias = temp.path().join("alias");
    let result=std::process::Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command","New-Item -ItemType Junction -Path $env:ANANSI_TEST_ALIAS -Target $env:ANANSI_TEST_TARGET | Out-Null"]).env("ANANSI_TEST_ALIAS",&alias).env("ANANSI_TEST_TARGET",temp.path()).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(Storage::new(alias.join("storage")).is_err());
    fs::remove_dir(alias).unwrap();
}

#[test]
fn v1_migration_preserves_original_workspace_and_onboarding_survives_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    // Establish the owned private root, then install a synthetic historical v1 catalog.
    let storage = Storage::new(root.clone()).unwrap();
    storage.create("Seed").unwrap();
    let file = root.join(DATABASE_NAME);
    fs::remove_file(&file).unwrap();
    {
        let connection = rusqlite::Connection::open(&file).unwrap();
        connection
            .execute_batch(include_str!("../migrations/0001_workspace.sql"))
            .unwrap();
        connection
            .pragma_update(None, "application_id", 0x414e414e)
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        connection
            .execute(
                "INSERT INTO workspaces VALUES(?1,'Original','original')",
                ["a".repeat(32)],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO workspace_settings VALUES(?1,1)",
                ["a".repeat(32)],
            )
            .unwrap();
    }
    private_permissions(&file, false, false).unwrap();
    fs::copy(&file, temp.path().join("preserved-v1.sqlite3")).unwrap();
    let workers: Vec<_> = (0..3)
        .map(|_| {
            let storage = storage.clone();
            std::thread::spawn(move || storage.open(&"a".repeat(32)).unwrap())
        })
        .collect();
    let mut reopened = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert!(
        reopened
            .iter()
            .all(|workspace| workspace.schema_version == 2 && workspace.settings.reduced_motion)
    );
    let original = reopened.pop().unwrap();
    assert_eq!(original.schema_version, 2);
    assert_eq!(original.workspace.name, "Original");
    assert!(original.settings.reduced_motion);
    assert_eq!(
        original.settings.intent,
        anansi_core::settings::Intent::Explore
    );
    assert!(!original.settings.onboarding_complete);
    let core = Core::with_storage_root(root.clone());
    for intent in [
        "second_memory",
        "invent_product",
        "build_application",
        "connect_ai",
        "explore",
    ] {
        let updated=core.workspace_command("settings",json!({"contractVersion":1,"id":original.workspace.id,"reducedMotion":true,"intent":intent,"onboardingComplete":true})).unwrap();
        assert_eq!(updated["settings"]["intent"], intent);
    }
    for (field, value) in [
        ("intent", json!("made_up")),
        ("onboardingComplete", json!("true")),
    ] {
        let mut request =
            json!({"contractVersion":1,"id":original.workspace.id,"reducedMotion":false});
        request[field] = value;
        assert_eq!(
            core.workspace_command("settings", request)
                .unwrap_err()
                .code,
            ErrorCode::InvalidRequest
        );
    }
    let reopened = Core::with_storage_root(root)
        .workspace_command(
            "open",
            json!({"contractVersion":1,"id":original.workspace.id}),
        )
        .unwrap();
    assert_eq!(
        reopened["settings"],
        json!({"reducedMotion":true,"intent":"explore","onboardingComplete":true})
    );
    let old = rusqlite::Connection::open(temp.path().join("preserved-v1.sqlite3")).unwrap();
    assert_eq!(
        old.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn failed_v1_forward_migration_rolls_back_and_keeps_original_rows_and_version() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let storage = Storage::new(root.clone()).unwrap();
    let original = storage.create("Preserved").unwrap();
    let file = root.join(DATABASE_NAME);
    // A malformed v1 fixture has a conflicting second column. The first ALTER
    // must roll back when the second fails, retaining its original row.
    {
        let connection = rusqlite::Connection::open(&file).unwrap();
        connection
            .execute_batch(
                "ALTER TABLE workspace_settings DROP COLUMN intent; PRAGMA user_version=1;",
            )
            .unwrap();
    }
    fs::copy(&file, temp.path().join("preserved-malformed-v1.sqlite3")).unwrap();
    assert_eq!(
        storage.open(&original.workspace.id).unwrap_err().code,
        ErrorCode::CorruptStorage
    );
    let connection = rusqlite::Connection::open(&file).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(
        connection
            .prepare("SELECT intent FROM workspace_settings")
            .is_err()
    );
    assert_eq!(
        connection
            .query_row("SELECT name FROM workspaces", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "Preserved"
    );
}
