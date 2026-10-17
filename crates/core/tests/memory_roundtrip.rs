use anansi_core::{
    Core, ErrorCode, content,
    storage::{DATABASE_NAME, private_permissions},
};
use serde_json::{Value, json};
use std::fs;
fn setup() -> (tempfile::TempDir, Core, String) {
    let temp = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(temp.path().join("store"));
    let w = core
        .workspace_command("create", json!({"contractVersion":1,"name":"Synthetic"}))
        .unwrap();
    (temp, core, w["workspace"]["id"].as_str().unwrap().into())
}
fn request(w: &str) -> Value {
    json!({"contractVersion":1,"workspaceId":w,"draft":{"title":"Memory 世界","body":"# Café\nLiteral <script>no()</script>","eventDate":"2026-10-16T00:00:00Z","validFrom":null,"validUntil":null,"confidence":0.8,"sourceReferences":["user-entered reference"]}})
}
fn read(w: &str, m: &Value) -> Value {
    json!({"contractVersion":1,"workspaceId":w,"id":m["memory"]["id"]})
}
#[test]
fn unicode_revision_history_and_restart_preserve_original_bytes_and_metadata() {
    let (temp, core, w) = setup();
    let first = core.memory_command("create", request(&w)).unwrap();
    assert_eq!(first["memory"]["privacy"], "private");
    assert_eq!(first["memory"]["status"], "proposed");
    let mut revise = request(&w);
    revise["id"] = first["memory"]["id"].clone();
    revise["expectedRevision"] = first["memory"]["revisionId"].clone();
    revise["draft"]["body"] = json!("Second 🕸️ revision");
    let second = core.memory_command("revise", revise.clone()).unwrap();
    assert_eq!(second["memory"]["version"], 2);
    assert_eq!(second["memory"]["createdAt"], first["memory"]["createdAt"]);
    assert_eq!(
        core.memory_command("revise", revise).unwrap_err().code,
        ErrorCode::RevisionConflict
    );
    let restarted = Core::with_storage_root(temp.path().join("store"));
    assert_eq!(
        restarted.memory_command("read", read(&w, &second)).unwrap(),
        second
    );
    let mut historical = read(&w, &first);
    historical["revisionId"] = first["memory"]["revisionId"].clone();
    assert_eq!(restarted.memory_command("read", historical).unwrap(), first);
    let history = restarted
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"],"limit":1}),
        )
        .unwrap();
    assert_eq!(history["memories"][0]["version"], 2);
    assert_eq!(history["nextOffset"], 1);
    let old=restarted.memory_command("list",json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"],"offset":1,"limit":1})).unwrap();
    assert_eq!(old["memories"][0]["version"], 1);
}
#[test]
fn validation_and_sealed_fail_before_storage_is_created() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("absent");
    let core = Core::with_storage_root(root.clone());
    for (field, value) in [
        ("title", json!(" ")),
        ("title", json!("x".repeat(201))),
        ("body", json!("x".repeat(content::MAX_BODY + 1))),
        ("body", json!("bad\0text")),
        ("eventDate", json!("2026-02-30T00:00:00Z")),
        ("confidence", json!(1.1)),
        ("privacy", json!("invented")),
        ("sourceReferences", json!(["x".repeat(2049)])),
    ] {
        let mut input = request(&"a".repeat(32));
        input["draft"][field] = value;
        assert!(core.memory_command("create", input).is_err());
        assert!(!root.exists());
    }
    let mut sealed = request(&"a".repeat(32));
    sealed["draft"]["privacy"] = json!("sealed");
    assert_eq!(
        core.memory_command("create", sealed).unwrap_err().code,
        ErrorCode::VaultUnavailable
    );
    for extra in ["path", "projectId", "status", "generatorModel"] {
        let mut r = request(&"a".repeat(32));
        r["draft"][extra] = json!("untrusted");
        assert!(core.memory_command("create", r).is_err());
    }
    let mut dates = request(&"a".repeat(32));
    dates["draft"]["validFrom"] = json!("2026-10-16T00:00:00Z");
    dates["draft"]["validUntil"] = json!("2025-10-16T00:00:00Z");
    assert!(core.memory_command("create", dates).is_err());
    assert!(!root.exists());
}
#[test]
fn wrong_workspace_cannot_read_history_or_revise_and_titles_are_never_paths() {
    let (temp, core, w) = setup();
    let mut input = request(&w);
    input["draft"]["title"] = json!("../../CON");
    let first = core.memory_command("create", input).unwrap();
    let other = core
        .workspace_command("create", json!({"contractVersion":1,"name":"Other"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let bad = read(&other, &first);
    assert_eq!(
        core.memory_command("read", bad).unwrap_err().code,
        ErrorCode::MemoryNotFound
    );
    let history = core
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":other,"id":first["memory"]["id"]}),
        )
        .unwrap();
    assert_eq!(history["memories"], json!([]));
    let mut revise = request(&other);
    revise["id"] = first["memory"]["id"].clone();
    revise["expectedRevision"] = first["memory"]["revisionId"].clone();
    assert_eq!(
        core.memory_command("revise", revise).unwrap_err().code,
        ErrorCode::MemoryNotFound
    );
    assert!(!temp.path().join("CON").exists());
    for id in ["../escape", "BAD", ""] {
        assert!(
            core.memory_command("read", json!({"contractVersion":1,"workspaceId":w,"id":id}))
                .is_err()
        );
    }
    for limit in [0, 101] {
        assert!(
            core.memory_command(
                "list",
                json!({"contractVersion":1,"workspaceId":w,"limit":limit})
            )
            .is_err()
        );
    }
}
#[test]
fn hash_altered_missing_content_and_failed_transaction_preserve_committed_revision() {
    let (temp, core, w) = setup();
    let first = core.memory_command("create", request(&w)).unwrap();
    let root = temp.path().join("store");
    let file = root
        .join("content")
        .join(&w)
        .join(first["memory"]["id"].as_str().unwrap())
        .join(format!(
            "{}.md",
            first["memory"]["revisionId"].as_str().unwrap()
        ));
    let original = fs::read(&file).unwrap();
    fs::write(&file, b"tampered").unwrap();
    assert_eq!(
        core.memory_command("read", read(&w, &first))
            .unwrap_err()
            .code,
        ErrorCode::ContentIntegrity
    );
    fs::write(&file, &original).unwrap();
    let saved = temp.path().join("preserved.md");
    fs::rename(&file, &saved).unwrap();
    assert_eq!(
        core.memory_command("read", read(&w, &first))
            .unwrap_err()
            .code,
        ErrorCode::ContentIntegrity
    );
    fs::rename(saved, &file).unwrap();
    let connection = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
    connection.execute_batch("CREATE TRIGGER refuse_revision BEFORE INSERT ON memory_revisions BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    let mut revise = request(&w);
    revise["id"] = first["memory"]["id"].clone();
    revise["expectedRevision"] = first["memory"]["revisionId"].clone();
    assert!(core.memory_command("revise", revise).is_err());
    assert_eq!(
        core.memory_command("read", read(&w, &first)).unwrap(),
        first
    );
    assert_eq!(fs::read(file).unwrap(), original);
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM memory_revisions", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        1
    );
}
#[test]
fn privacy_downgrade_requires_confirmation_and_does_not_publish() {
    let (_temp, core, w) = setup();
    let first = core.memory_command("create", request(&w)).unwrap();
    let mut r = request(&w);
    r["id"] = first["memory"]["id"].clone();
    r["expectedRevision"] = first["memory"]["revisionId"].clone();
    r["draft"]["privacy"] = json!("public");
    assert_eq!(
        core.memory_command("revise", r.clone()).unwrap_err().code,
        ErrorCode::InvalidMemory
    );
    r["confirmPrivacyChange"] = json!(true);
    assert_eq!(
        core.memory_command("revise", r).unwrap()["memory"]["privacy"],
        "public"
    );
}

#[test]
fn maximum_body_and_unicode_title_are_valid_and_plain_markdown_is_readable() {
    let (temp, core, w) = setup();
    let mut r = request(&w);
    r["draft"]["title"] = json!("界".repeat(200));
    r["draft"]["body"] = json!("x".repeat(content::MAX_BODY));
    let saved = core.memory_command("create", r).unwrap();
    assert_eq!(saved["body"].as_str().unwrap().len(), content::MAX_BODY);
    let file = temp
        .path()
        .join("store/content")
        .join(&w)
        .join(saved["memory"]["id"].as_str().unwrap())
        .join(format!(
            "{}.md",
            saved["memory"]["revisionId"].as_str().unwrap()
        ));
    assert_eq!(
        fs::read_to_string(file).unwrap(),
        "x".repeat(content::MAX_BODY)
    );
    assert_eq!(
        content::hash(b"hello world"),
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn content_directory_link_escape_is_rejected_without_writing_outside() {
    let (temp, core, w) = setup();
    let root = temp.path().join("store");
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let alias = root.join("content");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, &alias).unwrap();
    #[cfg(windows)]
    {
        let output=std::process::Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command","New-Item -ItemType Junction -Path $env:ANANSI_TEST_ALIAS -Target $env:ANANSI_TEST_TARGET | Out-Null"]).env("ANANSI_TEST_ALIAS",&alias).env("ANANSI_TEST_TARGET",&outside).output().unwrap();
        assert!(output.status.success());
    }
    assert_eq!(
        core.memory_command("create", request(&w)).unwrap_err().code,
        ErrorCode::StorageUnavailable
    );
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    #[cfg(windows)]
    fs::remove_dir(alias).unwrap();
    #[cfg(unix)]
    fs::remove_file(alias).unwrap();
}
#[test]
fn simultaneous_revisions_have_exactly_one_winner() {
    let (_temp, core, w) = setup();
    let first = core.memory_command("create", request(&w)).unwrap();
    let mut r = request(&w);
    r["id"] = first["memory"]["id"].clone();
    r["expectedRevision"] = first["memory"]["revisionId"].clone();
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let c = core.clone();
            let r = r.clone();
            std::thread::spawn(move || c.memory_command("revise", r))
        })
        .collect();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| r
                .as_ref()
                .is_err_and(|e| e.code == ErrorCode::RevisionConflict))
            .count(),
        1
    );
}
#[test]
fn schema_two_migration_and_memory_migration_failure_preserve_settings() {
    let (temp, core, w) = setup();
    let file = temp.path().join("store").join(DATABASE_NAME);
    {
        let c = rusqlite::Connection::open(&file).unwrap();
        c.execute_batch("PRAGMA foreign_keys=OFF; DROP TABLE memory_revisions; DROP TABLE memories; DROP TABLE lifecycle_format; DROP TABLE IF EXISTS relationship_revisions; DROP TABLE IF EXISTS relationships; PRAGMA user_version=2;").unwrap();
    }
    private_permissions(&file, false, false).unwrap();
    assert!(core.memory_command("create", request(&w)).is_ok());
    {
        let c = rusqlite::Connection::open(&file).unwrap();
        c.execute_batch("DROP TABLE IF EXISTS relationship_revisions; DROP TABLE IF EXISTS relationships; PRAGMA user_version=2;").unwrap();
    }
    assert!(
        core.workspace_command("open", json!({"contractVersion":1,"id":w}))
            .is_err()
    );
    let c = rusqlite::Connection::open(file).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM workspaces", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
}
