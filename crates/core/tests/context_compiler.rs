use anansi_core::Core;
use anansi_core::context::{ContextCompilationRequest, chunk_markdown_sections, compile_context};
use serde_json::json;

fn setup_test_workspace(name: &str) -> (tempfile::TempDir, Core, std::path::PathBuf, String) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());
    let ws = core
        .workspace_command("create", json!({ "contractVersion": 1, "name": name }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    (temp, core, root, ws)
}

#[test]
fn unicode_sections_do_not_panic_at_byte_budget_boundaries() {
    for text in ["世".repeat(500), "🕸".repeat(500), "é".repeat(800)] {
        let chunks = chunk_markdown_sections("memory", "Unicode", "note", &text);
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| chunk.content.len() <= 1_000));
        assert_eq!(
            chunks
                .iter()
                .map(|chunk| chunk.content.as_str())
                .collect::<String>(),
            text
        );
    }
}

#[test]
fn test_chunk_markdown_sections_preserves_headings_and_lines() {
    let body =
        "# Section 1\nLine 2 text\nLine 3 text\n\n## Section 2\nLine 6 details\nLine 7 details";
    let chunks = chunk_markdown_sections("mem_1", "Test Memory", "note", body);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].chunk_index, 0);
    assert_eq!(chunks[0].section_heading.as_deref(), Some("# Section 1"));
    assert_eq!(chunks[0].line_number, Some(2));
    assert!(chunks[0].content.contains("Line 2 text"));

    assert_eq!(chunks[1].chunk_index, 1);
    assert_eq!(chunks[1].section_heading.as_deref(), Some("## Section 2"));
    assert_eq!(chunks[1].line_number, Some(6));
    assert!(chunks[1].content.contains("Line 6 details"));
}

#[test]
fn test_budget_enforcement_truncates_chunks() {
    let (_temp, core, root, ws) = setup_test_workspace("Budget Test Workspace");
    let db_path = root.join("workspaces.sqlite3");
    let conn = rusqlite::Connection::open(&db_path).unwrap();

    // Create 3 memories with distinct content
    for i in 1..=3 {
        let section_text = format!("Detailed engineering specification section {i}. ").repeat(15);
        core.memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": ws,
                "draft": {
                    "title": format!("Engineering Spec {i}"),
                    "body": format!("# Spec Header {i}\n{section_text}"),
                    "privacy": "private"
                },
                "kind": "decision"
            }),
        )
        .unwrap();
    }

    // Set budget to 600 characters
    let req = ContextCompilationRequest {
        workspace_id: ws,
        user_prompt: "Engineering Spec".to_string(),
        max_budget_chars: Some(600),
        runtime_endpoint: None,
        model_tag: None,
        kind_filter: None,
    };

    let compiled = compile_context(&conn, &root, &req).unwrap();

    // Sum of evidence chunk lengths must not exceed 600 chars
    let evidence_char_sum: usize = compiled
        .evidence_chunks
        .iter()
        .map(|c| c.content.len())
        .sum();
    assert!(evidence_char_sum <= 600);
    assert!(!compiled.evidence_chunks.is_empty());
}

#[test]
fn test_chunk_deduplication() {
    let (_temp, core, root, ws) = setup_test_workspace("Dedup Test Workspace");
    let db_path = root.join("workspaces.sqlite3");
    let conn = rusqlite::Connection::open(&db_path).unwrap();

    let shared_body = "# Shared Architecture\nThis is an identical paragraph describing the consensus architecture across workspaces.";

    core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": ws,
            "draft": {
                "title": "Architecture Doc Alpha",
                "body": shared_body,
                "privacy": "standard"
            },
            "kind": "note"
        }),
    )
    .unwrap();

    core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": ws,
            "draft": {
                "title": "Architecture Doc Beta",
                "body": shared_body,
                "privacy": "standard"
            },
            "kind": "note"
        }),
    )
    .unwrap();

    let req = ContextCompilationRequest {
        workspace_id: ws,
        user_prompt: "Architecture".to_string(),
        max_budget_chars: Some(4000),
        runtime_endpoint: None,
        model_tag: None,
        kind_filter: None,
    };

    let compiled = compile_context(&conn, &root, &req).unwrap();

    // Since the body content is identical, the duplicate chunk must be skipped
    assert_eq!(compiled.evidence_chunks.len(), 1);
    assert_eq!(compiled.citations.len(), 1);
}

#[test]
fn test_fail_closed_sealed_privacy_refusal() {
    let (_temp, core, root, ws) = setup_test_workspace("Privacy Boundary Workspace");
    let db_path = root.join("workspaces.sqlite3");
    let conn = rusqlite::Connection::open(&db_path).unwrap();

    // 1. Create a standard memory through normal core command
    core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": ws,
            "draft": {
                "title": "Public Vault Architecture",
                "body": "# Vault Design\nThe vault uses Argon2id key derivation.",
                "privacy": "standard"
            },
            "kind": "decision"
        }),
    )
    .unwrap();

    // 2. Directly insert a Sealed memory into SQLite and FTS5 to test fail-closed exclusion
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    let sealed_id = "mem_sealed_secret";
    let sealed_rev = "rev_sealed_1";
    conn.execute(
        "INSERT INTO memories (workspace_id, id, current_revision) VALUES (?1, ?2, ?3)",
        rusqlite::params![ws, sealed_id, sealed_rev],
    )
    .unwrap();

    let sealed_meta = json!({
        "id": sealed_id,
        "workspaceId": ws,
        "kind": "note",
        "revisionId": sealed_rev,
        "version": 1,
        "title": "Confidential Master Passwords",
        "status": "active",
        "privacy": "sealed",
        "provenance": "user",
        "sourceReferences": [],
        "validationStatus": "valid",
        "relationships": [],
        "createdAt": "2026-10-19T00:00:00Z",
        "updatedAt": "2026-10-19T00:00:00Z"
    })
    .to_string();

    conn.execute(
        "INSERT INTO memory_revisions (workspace_id, memory_id, id, version, metadata, body_hash)
         VALUES (?1, ?2, ?3, 1, ?4, '0000000000000000000000000000000000000000000000000000000000000000')",
        rusqlite::params![ws, sealed_id, sealed_rev, sealed_meta],
    )
    .unwrap();

    // Even if FTS5 had this entry indexed:
    conn.execute(
        "INSERT INTO memory_body_fts (workspace_id, memory_id, revision_id, title, body)
         VALUES (?1, ?2, ?3, 'Confidential Master Passwords', '# Vault Keys\nSuperSecretKey123456789')",
        rusqlite::params![ws, sealed_id, sealed_rev],
    )
    .unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

    let req = ContextCompilationRequest {
        workspace_id: ws,
        user_prompt: "Vault Keys".to_string(),
        max_budget_chars: Some(4000),
        runtime_endpoint: None,
        model_tag: None,
        kind_filter: None,
    };

    let compiled = compile_context(&conn, &root, &req).unwrap();

    // The Sealed memory must NEVER appear in chunks, citations, or manifest
    for chunk in &compiled.evidence_chunks {
        assert_ne!(chunk.title, "Confidential Master Passwords");
        assert!(!chunk.content.contains("SuperSecretKey123456789"));
    }
    for citation in &compiled.citations {
        assert_ne!(citation.title, "Confidential Master Passwords");
        assert!(!citation.snippet.contains("SuperSecretKey123456789"));
    }
    for source in &compiled.disclosure_manifest.source_memories {
        assert_ne!(source.title, "Confidential Master Passwords");
    }
    assert!(!compiled.system_prompt.contains("SuperSecretKey123456789"));
}

#[test]
fn test_disclosure_manifest_completeness() {
    let (_temp, core, root, ws) = setup_test_workspace("Manifest Test Workspace");
    let db_path = root.join("workspaces.sqlite3");
    let conn = rusqlite::Connection::open(&db_path).unwrap();

    core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": ws,
            "draft": {
                "title": "Quantum Computing Roadmap",
                "body": "# Milestone 4\nFull topological qubit integration scheduled for Q4.",
                "privacy": "standard"
            },
            "kind": "note"
        }),
    )
    .unwrap();

    let req = ContextCompilationRequest {
        workspace_id: ws,
        user_prompt: "Quantum Computing".to_string(),
        max_budget_chars: Some(4000),
        runtime_endpoint: Some("http://127.0.0.1:11434".to_string()),
        model_tag: Some("llama3.1:8b".to_string()),
        kind_filter: None,
    };

    let compiled = compile_context(&conn, &root, &req).unwrap();

    assert_eq!(compiled.disclosure_manifest.runtime_kind, "localLoopback");
    assert_eq!(
        compiled.disclosure_manifest.endpoint,
        "http://127.0.0.1:11434"
    );
    assert_eq!(compiled.disclosure_manifest.model_tag, "llama3.1:8b");
    assert_eq!(compiled.disclosure_manifest.source_memories.len(), 1);
    assert_eq!(
        compiled.disclosure_manifest.source_memories[0].title,
        "Quantum Computing Roadmap"
    );
    assert!(compiled.total_characters > 0);
    assert!(compiled.total_tokens > 0);
    assert!(
        compiled
            .system_prompt
            .contains("[1] Quantum Computing Roadmap")
    );
    assert!(compiled.system_prompt.contains("Milestone 4"));
}
