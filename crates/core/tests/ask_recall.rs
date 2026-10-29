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

#[test]
fn natural_questions_recover_partial_matches_without_changing_exact_search() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let ws = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Recall acceptance"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    // Fresh wording and facts, not the benchmark's entities or templates.
    let battery = note(
        &core,
        &ws,
        "Service bulletin",
        "The Zephyr battery endurance is fourteen hours.",
    );
    let response = ask(&core, &ws, "How long will the Zephyr battery keep running?");
    assert_eq!(response.status, AskStatus::Offline);
    assert!(response.manifest.compiled_prompt.contains("fourteen hours"));
    assert!(response.citations.iter().any(|c| c.memory_id == battery));
    let conn = rusqlite::Connection::open(root.join("workspaces.sqlite3")).unwrap();
    let exact = anansi_core::hybrid::hybrid_search(
        &conn,
        &anansi_core::hybrid::HybridSearchRequest {
            workspace_id: ws.clone(),
            query: Some("Zephyr running".into()),
            query_vector: None,
            model: None,
            endpoint: None,
            limit: 10,
            lexical_weight: 1.0,
            semantic_weight: 0.0,
            rrf_k: 60.0,
            kind: None,
            include_archived: false,
            include_expired: false,
        },
    )
    .unwrap();
    assert!(
        exact.is_empty(),
        "normal Search must still require every term"
    );
    let listed = core
        .memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":ws,
        "query":"Zephyr running","fullBody":true}),
        )
        .unwrap();
    assert!(listed["memories"].as_array().unwrap().is_empty());
    let missing = ask(&core, &ws, "Qzxv meteorite xenolith?");
    assert_eq!(missing.status, AskStatus::InsufficientEvidence);
    assert!(missing.citations.is_empty());
    // A partial match cannot prove the requested fact; preserve abstention instruction.
    let unsupported = ask(&core, &ws, "What price is the Zephyr battery?");
    assert!(
        unsupported
            .manifest
            .compiled_prompt
            .contains("state truthfully that evidence is missing")
    );
    assert!(!unsupported.manifest.compiled_prompt.contains("price is 42"));
}

#[test]
fn separate_operands_survive_a_long_first_source_and_disclosure_is_exact() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let ws = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Source diversity"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let first = note(
        &core,
        &ws,
        "Kestrel allocation",
        "# Allocation\nKestrel's starting allowance is 93 credits.\n# Background\n",
    );
    let second = note(
        &core,
        &ws,
        "Kestrel debit",
        "Kestrel consumed 26 credits. No other credits moved.",
    );
    // Make the first candidate long without repeating the requested fact.
    let conn = rusqlite::Connection::open(root.join("workspaces.sqlite3")).unwrap();
    let current: String = conn
        .query_row(
            "SELECT current_revision FROM memories WHERE workspace_id=?1 AND id=?2",
            rusqlite::params![ws, first],
            |r| r.get(0),
        )
        .unwrap();
    core.memory_command("revise",json!({"contractVersion":1,"workspaceId":ws,"id":first,"expectedRevision":current,
       "draft":{"title":"Kestrel allocation","body":format!("# Allocation\nKestrel's starting allowance is 93 credits.\n# Background\n{}","Kestrel administrative explanatory material. ".repeat(240)),"privacy":"private"}})).unwrap();
    let response = ask(
        &core,
        &ws,
        "How much credit does Kestrel still have available?",
    );
    assert!(response.manifest.compiled_prompt.contains("93 credits"));
    assert!(response.manifest.compiled_prompt.contains("26 credits"));
    let cited = response
        .citations
        .iter()
        .map(|c| c.memory_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(cited.contains(first.as_str()) && cited.contains(second.as_str()));
    let disclosed = response
        .manifest
        .source_memories
        .iter()
        .map(|m| m.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(cited, disclosed);
    assert!(response.manifest.compiled_prompt.len() < 5000);
}
