//! Ask-level batch-proof adversarial cases (behavior only; no
//! counter assertions, so these tests are parallel-safe).
//!
//! - Proofs are not reused across operations: widening a file
//!   between two Asks fails the second Ask, and repair restores it.
//! - Sealing or archiving a previously cited record excludes it
//!   from the next Ask.
//! - Revising a record makes the next Ask read the new revision.
//! - Deleting the content directory fails closed instead of
//!   silently reporting no evidence.
use anansi_core::{
    Core,
    ask::{AskRequest, AskStatus},
};
use serde_json::json;
use std::fs;
use std::path::Path;

fn workspace(core: &Core) -> String {
    core.workspace_command("create", json!({"contractVersion":1,"name":"Adversarial"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn note(core: &Core, ws: &str, title: &str, body: &str) -> String {
    core.memory_command(
        "create",
        json!({"contractVersion":1,"workspaceId":ws,"kind":"note",
        "draft":{"title":title,"body":body,"privacy":"private"}}),
    )
    .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn ask(core: &Core, ws: &str, prompt: &str) -> anansi_core::ask::AskResponse {
    core.ask_anansi(AskRequest {
        contract_version: 1,
        workspace_id: ws.into(),
        prompt: prompt.into(),
        runtime_endpoint: Some("http://127.0.0.1:1".into()),
        model_tag: Some("offline-test".into()),
        embedding_model: None,
        max_evidence_items: None,
    })
    .unwrap()
}

fn content_file(root: &Path, ws: &str, id: &str) -> std::path::PathBuf {
    let dir = root.join("content").join(ws).join(id);
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    entries.into_iter().next().unwrap()
}

#[test]
fn proofs_are_not_reused_across_operations() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let ws = workspace(&core);
    let id = note(
        &core,
        &ws,
        "Tern shelf",
        "The Tern archive shelf has reference code ADV-101.",
    );
    let q = "Which reference code belongs to the Tern archive shelf?";
    let first = ask(&core, &ws, q);
    assert_eq!(first.status, AskStatus::Offline);
    assert!(first.manifest.compiled_prompt.contains("ADV-101"));
    // Widen between operations: the next Ask must re-verify and fail.
    let file = content_file(&root, &ws, &id);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let status = std::process::Command::new("icacls.exe")
            .arg(&file)
            .args(["/grant", "*S-1-1-0:(R)"])
            .creation_flags(0x08000000)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
    }
    assert!(
        core.ask_anansi(AskRequest {
            contract_version: 1,
            workspace_id: ws.clone(),
            prompt: q.into(),
            runtime_endpoint: Some("http://127.0.0.1:1".into()),
            model_tag: Some("offline-test".into()),
            embedding_model: None,
            max_evidence_items: None,
        })
        .is_err()
    );
    // Repair restores the next Ask.
    anansi_core::storage::private_permissions(&file, false, false).unwrap();
    let third = ask(&core, &ws, q);
    assert_eq!(third.status, AskStatus::Offline);
    assert!(third.manifest.compiled_prompt.contains("ADV-101"));
}

#[test]
fn sealed_and_archived_records_leave_the_next_ask() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let ws = workspace(&core);
    let sealed_id = note(
        &core,
        &ws,
        "Sealed ledger",
        "The sealed ledger records 88 crates.",
    );
    let archived_id = note(
        &core,
        &ws,
        "Archived ledger",
        "The archived ledger records 77 crates.",
    );
    let kept_id = note(
        &core,
        &ws,
        "Kept ledger",
        "The kept ledger records 66 crates.",
    );
    let q = "Which ledger records how many crates?";
    let first = ask(&core, &ws, q);
    assert!(first.manifest.compiled_prompt.contains("88 crates"));
    assert!(first.manifest.compiled_prompt.contains("77 crates"));
    assert!(first.manifest.compiled_prompt.contains("66 crates"));
    // Seal via revision-metadata update (the API refuses Sealed
    // creation fail-closed; this matches existing test practice).
    // Archive via a revise with a lifecycle resolution, as the
    // large-scale test does.
    let conn = rusqlite::Connection::open(root.join("workspaces.sqlite3")).unwrap();
    conn.execute(
        "UPDATE memory_revisions SET metadata=json_set(metadata,'$.privacy','sealed') WHERE memory_id=?1",
        rusqlite::params![sealed_id],
    )
    .unwrap();
    let rev: String = conn
        .query_row(
            "SELECT current_revision FROM memories WHERE workspace_id=?1 AND id=?2",
            rusqlite::params![ws, archived_id],
            |r| r.get(0),
        )
        .unwrap();
    drop(conn);
    core.memory_command(
        "revise",
        json!({"contractVersion":1,"workspaceId":ws,"id":archived_id,
        "expectedRevision":rev,
        "draft":{"title":"Archived ledger","body":"The archived ledger records 77 crates.","privacy":"private"},
        "lifecycle":{"state":"archived","confirmed":true,"successorId":null}}),
    )
    .unwrap();
    let second = ask(&core, &ws, q);
    assert!(!second.manifest.compiled_prompt.contains("88 crates"));
    assert!(!second.manifest.compiled_prompt.contains("77 crates"));
    assert!(second.manifest.compiled_prompt.contains("66 crates"));
    assert!(!second.citations.iter().any(|c| c.memory_id == sealed_id));
    assert!(!second.citations.iter().any(|c| c.memory_id == archived_id));
    assert!(second.citations.iter().any(|c| c.memory_id == kept_id));
}

#[test]
fn revised_record_serves_the_new_revision() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let ws = workspace(&core);
    let id = note(&core, &ws, "Count note", "The depot counts 10 crates.");
    let q = "How many crates does the depot count?";
    let first = ask(&core, &ws, q);
    assert!(first.manifest.compiled_prompt.contains("10 crates"));
    let conn = rusqlite::Connection::open(root.join("workspaces.sqlite3")).unwrap();
    let rev: String = conn
        .query_row(
            "SELECT current_revision FROM memories WHERE workspace_id=?1 AND id=?2",
            rusqlite::params![ws, id],
            |r| r.get(0),
        )
        .unwrap();
    drop(conn);
    core.memory_command(
        "revise",
        json!({"contractVersion":1,"workspaceId":ws,"id":id,
        "expectedRevision":rev,
        "draft":{"title":"Count note","body":"The depot counts 25 crates.","privacy":"private"}}),
    )
    .unwrap();
    let second = ask(&core, &ws, q);
    assert!(second.manifest.compiled_prompt.contains("25 crates"));
    assert!(!second.manifest.compiled_prompt.contains("10 crates"));
}

#[test]
fn deleted_content_directory_fails_closed() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let ws = workspace(&core);
    note(
        &core,
        &ws,
        "Tern shelf",
        "The Tern archive shelf has reference code ADV-202.",
    );
    let q = "Which reference code belongs to the Tern archive shelf?";
    let first = ask(&core, &ws, q);
    assert_eq!(first.status, AskStatus::Offline);
    fs::remove_dir_all(root.join("content")).unwrap();
    // Partially initialized storage must error, never silently
    // report InsufficientEvidence for records the database claims.
    let result = core.ask_anansi(AskRequest {
        contract_version: 1,
        workspace_id: ws,
        prompt: q.into(),
        runtime_endpoint: Some("http://127.0.0.1:1".into()),
        model_tag: Some("offline-test".into()),
        embedding_model: None,
        max_evidence_items: None,
    });
    assert!(result.is_err());
}
