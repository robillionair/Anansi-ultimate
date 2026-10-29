//! Permission accounting for one public Ask across workspace sizes.
//!
//! A single test (counters are process-global, so parallel tests in
//! this binary would pollute each other's deltas). For each size the
//! test seeds N notes, snapshots the diagnostic counters, runs one
//! Ask, and reports the deltas with setup and Ask timings separated.
//! Linux permission verification spawns no processes (metadata
//! checks only); the Windows process count for an operation is, by
//! construction, `single_calls + batch_calls`, and must be confirmed
//! by running this diagnostic on Windows rather than inferred here.
use anansi_core::{
    Core,
    ask::{AskRequest, AskStatus},
    storage::permission_verification_stats,
};
use serde_json::json;
use std::time::Instant;

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

#[test]
fn permission_accounting_across_workspace_sizes() {
    for size in [0usize, 1, 16, 128, 1010] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("storage");
        let core = Core::with_storage_root(root);
        let ws = core
            .workspace_command("create", json!({"contractVersion":1,"name":"Accounting"}))
            .unwrap()["workspace"]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let seed_started = Instant::now();
        for n in 0..size {
            let body = if n == 0 {
                "The Tern archive shelf has reference code ACCT-991.".to_owned()
            } else {
                format!("Diagnostic filler entry {n} contains printed maps.")
            };
            note(&core, &ws, &format!("Accounting note {n}"), &body);
        }
        let setup_ms = seed_started.elapsed().as_millis();
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
        let batch = after.batch_calls - before.batch_calls;
        let paths = after.batch_paths - before.batch_paths;
        let single = after.single_calls - before.single_calls;
        let root_c = after.root_checks - before.root_checks;
        let db_c = after.database_checks - before.database_checks;
        let batch_path_c = after.batch_path_checks - before.batch_path_checks;
        let read_dir = after.content_read_dir_checks - before.content_read_dir_checks;
        let read_file = after.content_read_file_checks - before.content_read_file_checks;
        let other = after.other_checks - before.other_checks;
        let fb_app = after.fallback_appeared - before.fallback_appeared;
        let fb_abs = after.fallback_absent - before.fallback_absent;
        eprintln!(
            "ACCOUNTING notes={size} setup_ms={setup_ms} ask_ms={ask_ms} status={:?} cited={} batch={batch} batch_paths={paths} single={single} root={root_c} database={db_c} batch_path={batch_path_c} read_dir={read_dir} read_file={read_file} other={other} fallback_appeared={fb_app} fallback_absent={fb_abs}",
            response.status,
            response.citations.len()
        );
        if size == 0 {
            assert_eq!(response.status, AskStatus::InsufficientEvidence);
            assert_eq!(batch, 0, "empty workspace builds no batch");
            assert_eq!(paths, 0);
        } else {
            assert_eq!(response.status, AskStatus::Offline);
            assert!(response.manifest.compiled_prompt.contains("ACCT-991"));
            assert_eq!(batch, 1, "one Ask must use exactly one batch");
            assert_eq!(
                paths as usize,
                2 * size + 2,
                "batch covers shared dirs + per-record dir + file"
            );
        }
        // Outside-batch permission work is constant: four connections
        // (open, proof enumeration, rebuild, compilation), each
        // verifying the root and the database file. Everything else
        // in `single` is the batch loop itself.
        assert_eq!(
            single as usize,
            paths as usize + 8,
            "single calls must equal batch paths plus 8 connection checks"
        );
        assert_eq!(root_c, 4, "four connections verify the root");
        assert_eq!(db_c, 4, "four connections verify the database file");
        assert_eq!(fb_app, 0, "no fallback expected on a clean Ask");
        assert_eq!(fb_abs, 0, "no fallback expected on a clean Ask");
        assert_eq!(read_dir, 0, "no per-path content reads on a clean Ask");
        assert_eq!(read_file, 0, "no per-path content reads on a clean Ask");
        assert_eq!(other, 0, "no untagged checks on a clean Ask");
    }
}
