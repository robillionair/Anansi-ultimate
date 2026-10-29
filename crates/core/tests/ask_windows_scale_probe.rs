//! Small public-path Windows diagnostic, not the 1007-record acceptance test.
//! Keeps production permission and integrity checks; no model or cloud calls.
#![cfg(windows)]

use anansi_core::{
    Core,
    ask::{AskRequest, AskStatus},
};
use serde_json::json;
use std::time::Instant;

#[test]
fn small_workspace_reports_unmodified_public_ask_cost() {
    let temp = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(temp.path().join("storage"));
    let ws = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Windows review timing"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let seed = Instant::now();
    for n in 0..16 {
        let body = if n == 0 {
            "The Tern archive shelf has reference code REVIEW-742.".to_owned()
        } else {
            format!("Unrelated collection entry {n} contains printed maps.")
        };
        core.memory_command(
            "create",
            json!({"contractVersion":1,"workspaceId":ws,"kind":"note",
            "draft":{"title":format!("Review note {n}"),"body":body,"privacy":"private"}}),
        )
        .unwrap();
    }
    eprintln!(
        "WINDOWS_PROBE records=16 seed_ms={}",
        seed.elapsed().as_millis()
    );
    let started = Instant::now();
    let response = core
        .ask_anansi(AskRequest {
            contract_version: 1,
            workspace_id: ws,
            prompt: "Which reference code belongs to the Tern archive shelf?".into(),
            runtime_endpoint: Some("http://127.0.0.1:1".into()),
            model_tag: Some("offline-test".into()),
            embedding_model: None,
            max_evidence_items: None,
        })
        .unwrap();
    eprintln!(
        "WINDOWS_PROBE records=16 ask_ms={} status={:?} citations={}",
        started.elapsed().as_millis(),
        response.status,
        response.citations.len()
    );
    assert_eq!(response.status, AskStatus::Offline);
    assert!(response.manifest.compiled_prompt.contains("REVIEW-742"));
    assert!(response.manifest.compiled_prompt.len() < 5500);
}
