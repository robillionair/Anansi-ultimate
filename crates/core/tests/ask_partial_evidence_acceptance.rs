//! Held-out Ask honesty cases: retrieval behavior separated from
//! model-answer behavior. Every question goes through the real Ask
//! path with a refused loopback endpoint, so no model answers
//! anything; the assertions grade what Ask itself returns, discloses,
//! and cites. Fresh domain vocabulary, no benchmark entities.
//!
//! Categories: genuinely absent evidence, shared vocabulary without
//! the requested fact, complete supporting evidence, conflicting
//! evidence, and excluded/private-boundary records.
use anansi_core::{
    Core,
    ask::{AskRequest, AskStatus},
};
use serde_json::json;

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

fn seal_note(root: &std::path::Path, id: &str) {
    let conn = rusqlite::Connection::open(root.join("workspaces.sqlite3")).unwrap();
    conn.execute(
        "UPDATE memory_revisions SET metadata=json_set(metadata,'$.privacy','sealed') WHERE memory_id=?1",
        rusqlite::params![id],
    )
    .unwrap();
}

fn current_revision(root: &std::path::Path, ws: &str, id: &str) -> String {
    let conn = rusqlite::Connection::open(root.join("workspaces.sqlite3")).unwrap();
    conn.query_row(
        "SELECT current_revision FROM memories WHERE workspace_id=?1 AND id=?2",
        rusqlite::params![ws, id],
        |r| r.get(0),
    )
    .unwrap()
}

fn archive(core: &Core, root: &std::path::Path, ws: &str, id: &str, title: &str, body: &str) {
    let rev = current_revision(root, ws, id);
    core.memory_command(
        "revise",
        json!({"contractVersion":1,"workspaceId":ws,"id":id,
        "expectedRevision":rev,
        "draft":{"title":title,"body":body,"privacy":"private"},
        "lifecycle":{"state":"archived","confirmed":true,"successorId":null}}),
    )
    .unwrap();
}

/// Fresh corpus plus a foreign workspace. Returns (root, ws, ids by role).
fn seed() -> (
    tempfile::TempDir,
    Core,
    String,
    Vec<(String, String)>,
    String,
) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let core = Core::with_storage_root(root.clone());
    let ws = core
        .workspace_command("create", json!({"contractVersion":1,"name":"Partial"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut ids = Vec::new();
    let log = note(
        &core,
        &ws,
        "Willow Marina log",
        "The Willow Marina morning tide reaches 1.8 meters at Cedar Pier.",
    );
    ids.push(("log".to_string(), log));
    let north = note(
        &core,
        &ws,
        "North lamp",
        "The Cedar Point lamp rotates every 30 seconds.",
    );
    ids.push(("north".to_string(), north));
    let south = note(
        &core,
        &ws,
        "South lamp",
        "The Cedar Point lamp rotates every 45 seconds.",
    );
    ids.push(("south".to_string(), south));
    let sealed_vault = note(
        &core,
        &ws,
        "Sealed vault note",
        "The sealed vault code is SEALED-999.",
    );
    seal_note(&root, &sealed_vault);
    ids.push(("sealed_vault".to_string(), sealed_vault));
    let sealed_tide = note(
        &core,
        &ws,
        "Sealed tide note",
        "Willow Marina tide gauge calibration: 1.8 meters at Cedar Pier, sealed note.",
    );
    seal_note(&root, &sealed_tide);
    ids.push(("sealed_tide".to_string(), sealed_tide));
    let archived = note(
        &core,
        &ws,
        "Archived ledger",
        "The archived ledger records 777 crates.",
    );
    archive(
        &core,
        &root,
        &ws,
        &archived,
        "Archived ledger",
        "The archived ledger records 777 crates.",
    );
    ids.push(("archived".to_string(), archived));
    let ws2 = core
        .workspace_command("create", json!({"contractVersion":1,"name":"Foreign"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let foreign = note(
        &core,
        &ws2,
        "Foreign ledger",
        "The foreign ledger records 888 crates.",
    );
    (dir, core, ws, ids, foreign)
}

fn ask_offline(core: &Core, ws: &str, prompt: &str) -> anansi_core::ask::AskResponse {
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

fn id_of(ids: &[(String, String)], role: &str) -> String {
    ids.iter()
        .find(|(r, _)| r == role)
        .map(|(_, id)| id.clone())
        .unwrap()
}

fn cited_ids(out: &anansi_core::ask::AskResponse) -> Vec<String> {
    out.citations.iter().map(|c| c.memory_id.clone()).collect()
}

fn disclosed_ids(out: &anansi_core::ask::AskResponse) -> Vec<String> {
    out.manifest
        .source_memories
        .iter()
        .map(|s| s.id.clone())
        .collect()
}

fn assert_disclosure_exact(out: &anansi_core::ask::AskResponse) {
    let mut cited = cited_ids(out);
    let mut disclosed = disclosed_ids(out);
    cited.sort();
    disclosed.sort();
    cited.dedup();
    disclosed.dedup();
    assert_eq!(
        cited, disclosed,
        "disclosure must equal exactly the cited set"
    );
}

#[test]
fn genuinely_absent_evidence_abstains_with_no_citations() {
    let (_dir, core, ws, _ids, _foreign) = seed();
    let out = ask_offline(&core, &ws, "What is the Stone Quarry crushing tariff?");
    assert_eq!(out.status, AskStatus::InsufficientEvidence);
    assert!(out.citations.is_empty(), "no overlap means no citations");
    assert!(out.manifest.source_memories.is_empty());
    assert!(out.manifest.compiled_prompt.is_empty());
    assert!(
        out.answer.contains("couldn't find"),
        "abstention answer must say nothing was found"
    );
}

#[test]
fn shared_vocabulary_without_the_fact_is_partial_not_answered() {
    let (_dir, core, ws, ids, _foreign) = seed();
    let out = ask_offline(
        &core,
        &ws,
        "What is the Willow Marina deep-water mooring charge?",
    );
    assert_eq!(
        out.status,
        AskStatus::Offline,
        "partial evidence plus a refused runtime is Offline, never Answered"
    );
    assert!(
        out.manifest
            .compiled_prompt
            .contains("state truthfully that evidence is missing"),
        "partial evidence must carry the abstention instruction"
    );
    assert!(
        !out.citations.is_empty(),
        "shared vocabulary legitimately returns partial matches"
    );
    assert!(
        cited_ids(&out).contains(&id_of(&ids, "log")),
        "the partial Willow Marina source is cited"
    );
    assert!(
        !cited_ids(&out).contains(&id_of(&ids, "sealed_tide")),
        "the sealed record must never be cited"
    );
    assert!(
        out.citations.iter().all(|c| {
            let s = c.snippet.to_lowercase();
            !s.contains("mooring") && !s.contains("charge")
        }),
        "no cited chunk contains the requested fact"
    );
    assert_disclosure_exact(&out);
    assert!(out.manifest.character_count < 5500, "prompt within budget");
}

#[test]
fn complete_supporting_evidence_is_cited_and_disclosed() {
    let (_dir, core, ws, ids, _foreign) = seed();
    let out = ask_offline(
        &core,
        &ws,
        "What is the Willow Marina morning tide at Cedar Pier?",
    );
    assert_eq!(out.status, AskStatus::Offline);
    assert!(
        cited_ids(&out).contains(&id_of(&ids, "log")),
        "the record holding the fact is cited"
    );
    assert!(
        out.citations
            .iter()
            .any(|c| c.snippet.contains("1.8 meters")),
        "a cited chunk contains the fact span"
    );
    assert!(
        !cited_ids(&out).contains(&id_of(&ids, "sealed_tide")),
        "the sealed copy of the fact must not be cited"
    );
    assert_disclosure_exact(&out);
    assert!(
        out.manifest.compiled_prompt.contains("1.8 meters"),
        "the compiled prompt contains the fact"
    );
}

#[test]
fn conflicting_evidence_preserves_both_values() {
    let (_dir, core, ws, ids, _foreign) = seed();
    let out = ask_offline(&core, &ws, "How often does the Cedar Point lamp rotate?");
    assert_eq!(out.status, AskStatus::Offline);
    assert!(
        out.manifest.compiled_prompt.contains("30 seconds"),
        "first conflicting value present in the prompt"
    );
    assert!(
        out.manifest.compiled_prompt.contains("45 seconds"),
        "second conflicting value present in the prompt"
    );
    assert!(
        cited_ids(&out).contains(&id_of(&ids, "north"))
            && cited_ids(&out).contains(&id_of(&ids, "south")),
        "both conflicting sources are disclosed"
    );
    assert_disclosure_exact(&out);
}

#[test]
fn excluded_records_never_leak() {
    let (_dir, core, ws, ids, foreign) = seed();
    let out = ask_offline(&core, &ws, "What is the sealed vault code?");
    assert_eq!(out.status, AskStatus::InsufficientEvidence);
    assert!(out.citations.is_empty());
    assert!(
        !out.answer.contains("SEALED-999") && !out.manifest.compiled_prompt.contains("SEALED-999"),
        "the sealed fact must not leak anywhere in the response"
    );
    assert!(
        !cited_ids(&out).contains(&id_of(&ids, "sealed_vault")),
        "the sealed record is never cited"
    );
    let out2 = ask_offline(
        &core,
        &ws,
        "What do the archived and foreign ledgers record?",
    );
    assert_eq!(out2.status, AskStatus::InsufficientEvidence);
    assert!(out2.citations.is_empty());
    assert!(
        !out2.manifest.compiled_prompt.contains("777 crates")
            && !out2.manifest.compiled_prompt.contains("888 crates"),
        "archived and foreign facts must not leak"
    );
    assert!(
        !cited_ids(&out2).contains(&id_of(&ids, "archived"))
            && !cited_ids(&out2).contains(&foreign),
        "archived and foreign records are never cited"
    );
}
