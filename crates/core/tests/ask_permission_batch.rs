//! Ask permission-batching proof: one operation-scoped batch covers a
//! whole Ask (rebuild + compilation), tampering still fails closed, and
//! repair restores success. Kept as a single test because the
//! verification counters are process-global: parallel tests in this
//! binary would pollute each other's deltas.
use anansi_core::{
    Core,
    ask::{AskRequest, AskStatus},
    storage::permission_verification_stats,
};
use serde_json::json;
use std::{path::Path, time::Instant};

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

fn workspace(core: &Core) -> String {
    core.workspace_command(
        "create",
        json!({"contractVersion":1,"name":"Permission batch proof"}),
    )
    .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn ask(
    core: &Core,
    ws: &str,
    prompt: &str,
) -> Result<anansi_core::ask::AskResponse, anansi_core::CommandError> {
    core.ask_anansi(AskRequest {
        contract_version: 1,
        workspace_id: ws.into(),
        prompt: prompt.into(),
        runtime_endpoint: Some("http://127.0.0.1:1".into()),
        model_tag: Some("offline-test".into()),
        embedding_model: None,
        max_evidence_items: None,
    })
}

fn body_file(root: &Path, ws: &str, id: &str) -> std::path::PathBuf {
    let dir = root.join("content").join(ws).join(id);
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1, "one body file per record");
    entries.pop().unwrap()
}

fn widen(path: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let status = std::process::Command::new("icacls.exe")
            .arg(path)
            .args(["/grant", "*S-1-1-0:(R)"])
            .creation_flags(0x08000000)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
}

#[test]
fn ask_uses_one_batch_and_tampering_fails_closed() {
    // Phase 1: 16 notes mirroring the Windows probe; one Ask must use
    // exactly one batch covering 34 paths (2 shared directories + 16
    // record directories + 16 files), with individual verification
    // calls bounded by 2N + 10 (42). The old path needed about 4N
    // plus compile re-reads plus root checks.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let ws = workspace(&core);
    for n in 0..16 {
        let body = if n == 0 {
            "The Tern archive shelf has reference code BATCH-991.".to_owned()
        } else {
            format!("Unrelated collection entry {n} contains printed maps.")
        };
        note(&core, &ws, &format!("Review note {n}"), &body);
    }
    let before = permission_verification_stats();
    let started = Instant::now();
    let response = ask(
        &core,
        &ws,
        "Which reference code belongs to the Tern archive shelf?",
    )
    .unwrap();
    let ask_ms = started.elapsed().as_millis();
    let after = permission_verification_stats();
    let batch_delta = after.batch_calls - before.batch_calls;
    let paths_delta = after.batch_paths - before.batch_paths;
    let single_delta = after.single_calls - before.single_calls;
    eprintln!(
        "PERMISSION_BATCH records=16 ask_ms={ask_ms} batch_delta={batch_delta} batch_paths_delta={paths_delta} single_delta={single_delta}"
    );
    assert_eq!(response.status, AskStatus::Offline);
    assert!(response.manifest.compiled_prompt.contains("BATCH-991"));
    assert!(response.manifest.compiled_prompt.len() < 5500);
    assert_eq!(response.citations.len(), 1);
    assert_eq!(batch_delta, 1);
    assert_eq!(paths_delta, 34);
    assert!(single_delta <= 42, "single_delta={single_delta}");

    // Phase 2: a corrupted body still fails integrity, not silently.
    let temp2 = tempfile::tempdir().unwrap();
    let root2 = temp2.path().join("storage");
    let core2 = Core::with_storage_root(root2.clone());
    let ws2 = workspace(&core2);
    let id2 = note(
        &core2,
        &ws2,
        "Tamper target",
        "The Tern archive shelf has reference code BATCH-991.",
    );
    let file2 = body_file(&root2, &ws2, &id2);
    std::fs::write(&file2, "tampered body").unwrap();
    assert!(
        ask(
            &core2,
            &ws2,
            "Which reference code belongs to the Tern archive shelf?"
        )
        .is_err()
    );

    // Phase 3: overbroad permissions still fail verification, and
    // repair (re-protecting the file) restores a successful Ask.
    let temp3 = tempfile::tempdir().unwrap();
    let root3 = temp3.path().join("storage");
    let core3 = Core::with_storage_root(root3.clone());
    let ws3 = workspace(&core3);
    let id3 = note(
        &core3,
        &ws3,
        "Widened target",
        "The Tern archive shelf has reference code BATCH-991.",
    );
    let file3 = body_file(&root3, &ws3, &id3);
    widen(&file3);
    assert!(
        ask(
            &core3,
            &ws3,
            "Which reference code belongs to the Tern archive shelf?"
        )
        .is_err()
    );
    anansi_core::storage::private_permissions(&file3, false, false).unwrap();
    let repaired = ask(
        &core3,
        &ws3,
        "Which reference code belongs to the Tern archive shelf?",
    )
    .unwrap();
    assert_eq!(repaired.status, AskStatus::Offline);
    assert!(repaired.manifest.compiled_prompt.contains("BATCH-991"));
}
