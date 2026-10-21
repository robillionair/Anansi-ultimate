//! M6 Slice 1 acceptance: the §9 versioned artifact registry on
//! the existing project identity (a project is a memory with
//! kind 'project'). Covers the design doc's criteria:
//! lifecycle, §9 field completeness, immutability, integrity
//! failure, stale-write conflicts, workspace isolation,
//! privacy enforcement, archived-project refusal, reopen.

use anansi_core::Core;
use serde_json::{Value, json};

fn core_with_project() -> (tempfile::TempDir, Core, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let ws = core
        .workspace_command("create", json!({ "contractVersion": 1, "name": "Registry" }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let project = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1, "workspaceId": ws, "kind": "project",
                "draft": { "title": "Harbor App", "body": "Product project.", "privacy": "private",
                           "eventDate": null, "validFrom": null, "validUntil": null,
                           "confidence": null, "sourceReferences": [] }
            }),
        )
        .unwrap();
    let pid = project["memory"]["id"].as_str().unwrap().to_string();
    (dir, core, ws, pid)
}

fn register(core: &Core, ws: &str, pid: &str, body: &str) -> Value {
    core.artifacts_command(
        "register",
        json!({
            "contractVersion": 1, "workspaceId": ws, "projectId": pid,
            "type": "concept", "title": "Concept note", "body": body,
            "sourceReferences": ["memory://harbor"], "privacy": "private"
        }),
    )
    .unwrap()
}

fn code(result: Result<Value, anansi_core::CommandError>) -> anansi_core::ErrorCode {
    result.unwrap_err().code
}

#[test]
fn full_lifecycle_with_all_section9_fields_and_immutable_versions() {
    let (_d, core, ws, pid) = core_with_project();
    let v1 = register(&core, &ws, &pid, "First draft of the concept.");
    let a = &v1["artifact"];
    // §9 fields, one by one.
    let artifact_id = a["artifactId"].as_str().unwrap().to_string();
    assert!(!artifact_id.is_empty());
    assert_eq!(a["projectId"], pid);
    assert_eq!(a["type"], "concept");
    assert_eq!(a["version"], 1);
    assert!(a["timestamp"].as_str().unwrap().contains('T'));
    assert_eq!(a["sourceReferences"], json!(["memory://harbor"]));
    assert_eq!(a["generator"], json!({ "kind": "manual" }));
    assert_eq!(a["validationStatus"], "unvalidated");
    assert_eq!(a["privacy"], "private");
    assert_eq!(v1["body"], "First draft of the concept.");

    let v2 = core
        .artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": ws, "artifactId": artifact_id,
                "expectedVersion": 1, "title": "Concept note", "body": "Second draft, expanded.",
                "sourceReferences": ["memory://harbor", "note://retro"]
            }),
        )
        .unwrap();
    assert_eq!(v2["artifact"]["version"], 2);
    assert_eq!(v2["body"], "Second draft, expanded.");
    assert_ne!(v2["artifact"]["contentHash"], a["contentHash"]);

    // v1 is untouched and independently readable.
    let back = core
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": ws, "artifactId": artifact_id, "version": 1 }),
        )
        .unwrap();
    assert_eq!(back["body"], "First draft of the concept.");
    assert_eq!(back["artifact"]["contentHash"], a["contentHash"]);

    // List shows the latest version once; versions shows both.
    let list = core
        .artifacts_command("list", json!({ "contractVersion": 1, "workspaceId": ws, "projectId": pid }))
        .unwrap();
    assert_eq!(list["artifacts"].as_array().unwrap().len(), 1);
    assert_eq!(list["artifacts"][0]["version"], 2);
    let versions = core
        .artifacts_command(
            "versions",
            json!({ "contractVersion": 1, "workspaceId": ws, "artifactId": artifact_id }),
        )
        .unwrap();
    assert_eq!(versions["versions"].as_array().unwrap().len(), 2);
    assert_eq!(versions["projectId"], pid);
}

#[test]
fn stale_write_conflict_stores_nothing() {
    let (_d, core, ws, pid) = core_with_project();
    let v1 = register(&core, &ws, &pid, "v1 body");
    let id = v1["artifact"]["artifactId"].as_str().unwrap().to_string();
    let revise = |expected: u64| {
        core.artifacts_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": ws, "artifactId": id,
                "expectedVersion": expected, "title": "T", "body": "changed", "sourceReferences": []
            }),
        )
    };
    assert_eq!(code(revise(7)), anansi_core::ErrorCode::RevisionConflict);
    assert!(revise(1).is_ok());
    // Now 1 is stale as well.
    assert_eq!(code(revise(1)), anansi_core::ErrorCode::RevisionConflict);
    let versions = core
        .artifacts_command(
            "versions",
            json!({ "contractVersion": 1, "workspaceId": ws, "artifactId": id }),
        )
        .unwrap();
    assert_eq!(versions["versions"].as_array().unwrap().len(), 2, "only the accepted revision exists");
}

#[test]
fn workspace_isolation_and_project_validation() {
    let (_d, core, ws_a, pid_a) = core_with_project();
    let v1 = register(&core, &ws_a, &pid_a, "in A");
    let id = v1["artifact"]["artifactId"].as_str().unwrap().to_string();
    let ws_b = core
        .workspace_command("create", json!({ "contractVersion": 1, "name": "Other" }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // B cannot read A's artifact, nor list A's project, nor
    // register under A's project id.
    assert_eq!(
        code(core.artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": ws_b, "artifactId": id }),
        )),
        anansi_core::ErrorCode::NotFound
    );
    assert_eq!(
        code(core.artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": ws_b, "projectId": pid_a }),
        )),
        anansi_core::ErrorCode::NotFound
    );
    // A memory that is not a project cannot hold artifacts.
    let note = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1, "workspaceId": ws_a,
                "draft": { "title": "Plain", "body": "x", "privacy": "private",
                           "eventDate": null, "validFrom": null, "validUntil": null,
                           "confidence": null, "sourceReferences": [] }
            }),
        )
        .unwrap();
    let note_id = note["memory"]["id"].as_str().unwrap().to_string();
    assert_eq!(
        code(core.artifacts_command(
            "register",
            json!({ "contractVersion": 1, "workspaceId": ws_a, "projectId": note_id,
                    "type": "note", "title": "T", "body": "b", "sourceReferences": [], "privacy": "private" }),
        )),
        anansi_core::ErrorCode::InvalidRequest
    );
}

#[test]
fn sealed_registration_is_refused_and_privacy_round_trips() {
    let (_d, core, ws, pid) = core_with_project();
    assert_eq!(
        code(core.artifacts_command(
            "register",
            json!({ "contractVersion": 1, "workspaceId": ws, "projectId": pid,
                    "type": "note", "title": "T", "body": "b", "sourceReferences": [], "privacy": "sealed" }),
        )),
        anansi_core::ErrorCode::VaultUnavailable
    );
    let ok = core
        .artifacts_command(
            "register",
            json!({ "contractVersion": 1, "workspaceId": ws, "projectId": pid,
                    "type": "note", "title": "T", "body": "b", "sourceReferences": [], "privacy": "standard" }),
        )
        .unwrap();
    assert_eq!(ok["artifact"]["privacy"], "standard");
}

#[test]
fn tampered_body_fails_integrity_and_archived_project_refuses_writes() {
    let (dir, core, ws, pid) = core_with_project();
    let v1 = register(&core, &ws, &pid, "authentic body");
    let id = v1["artifact"]["artifactId"].as_str().unwrap().to_string();
    // Tamper directly in SQLite.
    let db = dir.path().join("workspaces.sqlite3");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE artifact_versions SET body = 'forged body' WHERE artifact_id = ?1",
        rusqlite::params![id],
    )
    .unwrap();
    drop(conn);
    assert_eq!(
        code(core.artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": ws, "artifactId": id }),
        )),
        anansi_core::ErrorCode::ContentIntegrity
    );
    drop(core);

    // Archived projects refuse new artifacts (restore first).
    let (_d2, core2, ws2, pid2) = core_with_project();
    let project = core2
        .memory_command("read", json!({ "contractVersion": 1, "workspaceId": ws2, "id": pid2 }))
        .unwrap();
    let rev = project["memory"]["revisionId"].as_str().unwrap().to_string();
    core2
        .memory_command(
            "revise",
            json!({
                "contractVersion": 1, "workspaceId": ws2, "id": pid2, "expectedRevision": rev,
                "draft": { "title": "Harbor App", "body": "Product project.", "privacy": "private",
                           "eventDate": null, "validFrom": null, "validUntil": null,
                           "confidence": null, "sourceReferences": [] },
                "lifecycle": { "state": "archived", "confirmed": true, "successorId": null }
            }),
        )
        .unwrap();
    assert_eq!(
        code(core2.artifacts_command(
            "register",
            json!({ "contractVersion": 1, "workspaceId": ws2, "projectId": pid2,
                    "type": "note", "title": "T", "body": "b", "sourceReferences": [], "privacy": "private" }),
        )),
        anansi_core::ErrorCode::InvalidRequest
    );
}

#[test]
fn registry_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let (ws, pid, id) = {
        let core = Core::with_storage_root(root.clone());
        let ws = core
            .workspace_command("create", json!({ "contractVersion": 1, "name": "Persist" }))
            .unwrap()["workspace"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let project = core
            .memory_command(
                "create",
                json!({
                    "contractVersion": 1, "workspaceId": ws, "kind": "project",
                    "draft": { "title": "P", "body": "b", "privacy": "private",
                               "eventDate": null, "validFrom": null, "validUntil": null,
                               "confidence": null, "sourceReferences": [] }
                }),
            )
            .unwrap();
        let pid = project["memory"]["id"].as_str().unwrap().to_string();
        let v1 = register(&core, &ws, &pid, "durable");
        (ws, pid, v1["artifact"]["artifactId"].as_str().unwrap().to_string())
    };
    let core = Core::with_storage_root(root);
    let read = core
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": ws, "artifactId": id }),
        )
        .unwrap();
    assert_eq!(read["body"], "durable");
    assert_eq!(read["artifact"]["projectId"], pid);
}
