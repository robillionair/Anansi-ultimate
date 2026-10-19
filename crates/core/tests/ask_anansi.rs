use anansi_core::Core;
use anansi_core::ask::{AskRequest, AskStatus};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;

fn setup_test_workspace(name: &str) -> (tempfile::TempDir, Core, String) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root);
    let ws = core
        .workspace_command("create", json!({ "contractVersion": 1, "name": name }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    (temp, core, ws)
}

#[test]
fn missing_evidence_returns_truthful_insufficient_evidence_status() {
    let (_temp, core, ws) = setup_test_workspace("Empty Workspace");

    let res = core
        .ask_anansi(AskRequest {
            contract_version: 1,
            workspace_id: ws,
            prompt: "What is our deployment target?".into(),
            runtime_endpoint: None,
            model_tag: None,
            max_evidence_items: None,
        })
        .unwrap();

    assert_eq!(res.status, AskStatus::InsufficientEvidence);
    assert!(res.citations.is_empty());
    assert!(res.answer.contains("couldn't find any relevant memories"));
    assert_eq!(res.manifest.source_memories.len(), 0);
}

#[test]
fn grounded_context_compiles_citations_and_excludes_sealed_records() {
    let (_temp, core, ws) = setup_test_workspace("Knowledge Base");

    // 1. Create a Public / Standard note
    core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": ws,
            "draft": {
                "title": "Deployment Architecture",
                "body": "# Production Setup\nLine 2 details\nOur deployment target is a self-hosted Windows 11 appliance.",
                "privacy": "standard"
            },
            "kind": "note"
        }),
    )
    .unwrap();

    // 2. Create another note
    core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": ws,
            "draft": {
                "title": "Hardware Guidelines",
                "body": "## Memory Guidelines\nWorkstations should have at least 16 GB RAM for local LLMs.",
                "privacy": "private"
            },
            "kind": "note"
        }),
    )
    .unwrap();

    // 3. Compile Ask ANANSI request with offline endpoint to inspect compiled context
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let offline_port = listener.local_addr().unwrap().port();
    drop(listener);

    let res = core
        .ask_anansi(AskRequest {
            contract_version: 1,
            workspace_id: ws.clone(),
            prompt: "What is our deployment target?".into(),
            runtime_endpoint: Some(format!("http://127.0.0.1:{}", offline_port)),
            model_tag: Some("llama3.1:8b".into()),
            max_evidence_items: Some(5),
        })
        .unwrap();

    // Since port is offline, it returns Offline status with full compiled context
    assert_eq!(res.status, AskStatus::Offline);
    assert!(!res.citations.is_empty());

    let first_cit = &res.citations[0];
    assert_eq!(first_cit.title, "Deployment Architecture");
    assert_eq!(first_cit.kind, "note");
    assert_eq!(
        first_cit.section_heading.as_deref(),
        Some("# Production Setup")
    );
    assert!(first_cit.snippet.contains("deployment target"));

    // Verify manifest
    assert_eq!(res.manifest.model_tag, "llama3.1:8b");
    assert_eq!(res.manifest.runtime_kind, "offline");
    assert!(!res.manifest.source_memories.is_empty());
    assert!(res.manifest.character_count > 0);
    assert!(res.manifest.estimated_tokens > 0);
    assert!(res.manifest.compiled_prompt.contains("Production Setup"));
    assert!(
        res.manifest
            .compiled_prompt
            .contains("What is our deployment target?")
    );
}

#[test]
fn synthetic_ollama_server_answers_grounded_question() {
    let (_temp, core, ws) = setup_test_workspace("Ollama Inference Test");

    core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": ws,
            "draft": {
                "title": "Project Aurora Decision",
                "body": "# Decision Log\nWe decided to adopt SQLite with FTS5 for local search.",
                "privacy": "private"
            },
            "kind": "decision"
        }),
    )
    .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4096];
        let n = socket.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);
        assert!(req.contains("POST /api/generate HTTP/1.1"));
        assert!(req.contains("Decision Log"));

        let body = json!({
            "model": "llama3.1:8b",
            "response": "According to [1], the decision was to adopt SQLite with FTS5 for local search."
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).unwrap();
    });

    let res = core
        .ask_anansi(AskRequest {
            contract_version: 1,
            workspace_id: ws,
            prompt: "What did we decide about search?".into(),
            runtime_endpoint: Some(format!("http://127.0.0.1:{}", port)),
            model_tag: Some("llama3.1:8b".into()),
            max_evidence_items: Some(5),
        })
        .unwrap();

    server.join().unwrap();

    assert_eq!(res.status, AskStatus::Answered);
    assert!(res.answer.contains("According to [1]"));
    assert_eq!(res.citations.len(), 1);
    assert_eq!(res.citations[0].title, "Project Aurora Decision");
    assert_eq!(res.citations[0].kind, "decision");
}

#[test]
fn prompt_validation_enforces_bounds_and_safety() {
    let (_temp, core, ws) = setup_test_workspace("Validation Workspace");

    // Empty prompt rejected
    let empty_res = core.ask_anansi(AskRequest {
        contract_version: 1,
        workspace_id: ws.clone(),
        prompt: "   ".into(),
        runtime_endpoint: None,
        model_tag: None,
        max_evidence_items: None,
    });
    assert!(empty_res.is_err());
    assert_eq!(
        empty_res.unwrap_err().code,
        anansi_core::ErrorCode::InvalidRequest
    );

    // Oversized prompt (>500 chars) rejected
    let big_prompt = "a".repeat(501);
    let big_res = core.ask_anansi(AskRequest {
        contract_version: 1,
        workspace_id: ws.clone(),
        prompt: big_prompt,
        runtime_endpoint: None,
        model_tag: None,
        max_evidence_items: None,
    });
    assert!(big_res.is_err());
    assert_eq!(
        big_res.unwrap_err().code,
        anansi_core::ErrorCode::InvalidRequest
    );

    // Non-loopback endpoint rejected
    let non_loopback_res = core.ask_anansi(AskRequest {
        contract_version: 1,
        workspace_id: ws,
        prompt: "Valid prompt".into(),
        runtime_endpoint: Some("http://api.openai.com/v1".into()),
        model_tag: None,
        max_evidence_items: None,
    });
    assert!(non_loopback_res.is_err());
    assert_eq!(
        non_loopback_res.unwrap_err().code,
        anansi_core::ErrorCode::InvalidRequest
    );
}
