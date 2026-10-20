use anansi_core::hybrid::{HybridSearchRequest, hybrid_search};
use anansi_core::vector::{EmbeddingVector, store_embedding};
use rusqlite::Connection;

#[test]
fn malformed_queries_and_corrupt_fts_rows_fail_instead_of_disappearing() {
    let conn = setup_hybrid_test_db();
    let mut request = HybridSearchRequest {
        workspace_id: "ws".into(),
        query: Some("word ".repeat(17)),
        query_vector: None,
        model: None,
        limit: 10,
        lexical_weight: 1.0,
        semantic_weight: 1.0,
        rrf_k: 60.0,
        kind: None,
        include_archived: false,
        include_expired: false,
        endpoint: None,
    };
    assert!(hybrid_search(&conn, &request).is_err());
    request.query = Some("word".into());
    conn.execute("INSERT INTO memory_body_fts(workspace_id,memory_id,title,body) VALUES ('ws','broken','word',NULL)", []).unwrap();
    assert!(hybrid_search(&conn, &request).is_err());
    request.lexical_weight = f32::NAN;
    assert!(hybrid_search(&conn, &request).is_err());
}

fn setup_hybrid_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE memories (
            workspace_id TEXT NOT NULL,
            id TEXT NOT NULL,
            current_revision TEXT NOT NULL DEFAULT 'rev_1',
            PRIMARY KEY (workspace_id, id)
        );
        CREATE TABLE memory_revisions (
            workspace_id TEXT NOT NULL,
            memory_id TEXT NOT NULL,
            id TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            metadata TEXT NOT NULL,
            body_hash TEXT NOT NULL DEFAULT '0000000000000000000000000000000000000000000000000000000000000000',
            PRIMARY KEY (workspace_id, memory_id, id)
        );
        CREATE VIRTUAL TABLE memory_body_fts USING fts5(
            workspace_id UNINDEXED,
            memory_id UNINDEXED,
            revision_id UNINDEXED,
            title,
            body,
            tokenize='unicode61 remove_diacritics 2'
        );",
    )
    .unwrap();
    conn
}

#[allow(clippy::too_many_arguments)]
fn seed_memory(
    conn: &Connection,
    ws: &str,
    id: &str,
    title: &str,
    body: &str,
    kind: &str,
    archived: bool,
    privacy: &str,
) {
    conn.execute(
        "INSERT INTO memories (workspace_id, id, current_revision)
         VALUES (?1, ?2, 'rev_1')",
        rusqlite::params![ws, id],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO memory_revisions (workspace_id, memory_id, id, version, metadata, body_hash)
         VALUES (?1, ?2, 'rev_1', 1, ?3, '0000000000000000000000000000000000000000000000000000000000000000')",
        rusqlite::params![
            ws,
            id,
            serde_json::json!({
                "title": title,
                "kind": kind,
                "status": if archived { "archived" } else { "active" },
                "privacy": privacy,
                "updatedAt": "2026-10-19T00:00:00Z"
            })
            .to_string()
        ],
    )
    .unwrap();

    if privacy != "sealed" {
        conn.execute(
            "INSERT INTO memory_body_fts (workspace_id, memory_id, revision_id, title, body)
             VALUES (?1, ?2, 'rev_1', ?3, ?4)",
            rusqlite::params![ws, id, title, body],
        )
        .unwrap();
    }
}

#[test]
fn benchmark_exact_lexical_retrieval() {
    let conn = setup_hybrid_test_db();
    let ws = "ws_test";

    seed_memory(
        &conn,
        ws,
        "mem_sqlite",
        "SQLite Architecture Plan",
        "# Storage Architecture\nWe decided to adopt SQLite for local storage.",
        "decision",
        false,
        "private",
    );
    seed_memory(
        &conn,
        ws,
        "mem_react",
        "React Frontend Components",
        "# UI Design\nBuilding UI with React 19.",
        "note",
        false,
        "private",
    );

    let req = HybridSearchRequest {
        workspace_id: ws.to_string(),
        query: Some("sqlite storage".to_string()),
        query_vector: None,
        model: None,
        limit: 10,
        lexical_weight: 1.0,
        semantic_weight: 1.0,
        rrf_k: 60.0,
        kind: None,
        include_archived: false,
        include_expired: false,
        endpoint: None,
    };

    let results = hybrid_search(&conn, &req).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].memory_id, "mem_sqlite");
    assert_eq!(results[0].lexical_rank, Some(1));
    assert_eq!(results[0].semantic_rank, None);
    assert_eq!(
        results[0].section_heading.as_deref(),
        Some("# Storage Architecture")
    );
    assert!(results[0].rrf_score > 0.0);
}

#[test]
fn benchmark_semantic_paraphrase_retrieval() {
    let conn = setup_hybrid_test_db();
    let ws = "ws_test";
    let model = "test-embed";

    // "Veterinary Nutrition" has 0 keyword overlap with "cat food guidance"
    seed_memory(
        &conn,
        ws,
        "mem_vet",
        "Veterinary Nutrition Guide",
        "# Health\nBalanced feeding for domestic feline companions.",
        "note",
        false,
        "private",
    );

    // Vector pointing in direction [0.95, 0.05]
    let vet_vector = EmbeddingVector::new(vec![0.95, 0.05]).unwrap();
    store_embedding(&conn, ws, "mem_vet", 0, model, &vet_vector).unwrap();

    // Query with zero lexical overlap: query vector is close to [1.0, 0.0]
    let query_vector = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();

    let req = HybridSearchRequest {
        workspace_id: ws.to_string(),
        query: Some("cat food guidance".to_string()), // Lexical FTS returns 0 hits
        query_vector: Some(query_vector),
        model: Some(model.to_string()),
        limit: 10,
        lexical_weight: 1.0,
        semantic_weight: 1.0,
        rrf_k: 60.0,
        kind: None,
        include_archived: false,
        include_expired: false,
        endpoint: None,
    };

    let results = hybrid_search(&conn, &req).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].memory_id, "mem_vet");
    assert_eq!(results[0].lexical_rank, None);
    assert_eq!(results[0].semantic_rank, Some(1));
    assert_eq!(results[0].rrf_score, 1.0 / (60.0 + 1.0));
}

#[test]
fn benchmark_hybrid_synergy_ranks_highest() {
    let conn = setup_hybrid_test_db();
    let ws = "ws_test";
    let model = "test-embed";

    // Memory A matches both lexical keywords and vector similarity
    seed_memory(
        &conn,
        ws,
        "mem_synergy",
        "Quantum Physics Research",
        "# Quantum Mechanics\nEntanglement experiments and quantum computing.",
        "note",
        false,
        "private",
    );
    let v_synergy = EmbeddingVector::new(vec![0.9, 0.1]).unwrap();
    store_embedding(&conn, ws, "mem_synergy", 0, model, &v_synergy).unwrap();

    // Memory B matches only vector similarity (different keywords)
    seed_memory(
        &conn,
        ws,
        "mem_vector_only",
        "Subatomic Particle Collisions",
        "# Physics\nAccelerating hadrons and measuring particle spins.",
        "note",
        false,
        "private",
    );
    let v_physics = EmbeddingVector::new(vec![0.85, 0.15]).unwrap();
    store_embedding(&conn, ws, "mem_vector_only", 0, model, &v_physics).unwrap();

    // Memory C matches only lexical keyword "quantum", but vector is orthogonal
    seed_memory(
        &conn,
        ws,
        "mem_lexical_only",
        "Quantum Business Model",
        "# Business\nA quantum leap in financial accounting quarterly earnings.",
        "note",
        false,
        "private",
    );
    let v_biz = EmbeddingVector::new(vec![0.0, 1.0]).unwrap();
    store_embedding(&conn, ws, "mem_lexical_only", 0, model, &v_biz).unwrap();

    let query_vector = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
    let req = HybridSearchRequest {
        workspace_id: ws.to_string(),
        query: Some("quantum".to_string()),
        query_vector: Some(query_vector),
        model: Some(model.to_string()),
        limit: 10,
        lexical_weight: 1.0,
        semantic_weight: 1.0,
        rrf_k: 60.0,
        kind: None,
        include_archived: false,
        include_expired: false,
        endpoint: None,
    };

    let results = hybrid_search(&conn, &req).unwrap();
    assert!(!results.is_empty());
    // Memory A must rank #1 due to dual-modality synergy (both lexical rank and semantic rank)
    assert_eq!(results[0].memory_id, "mem_synergy");
    assert!(results[0].lexical_rank.is_some());
    assert!(results[0].semantic_rank.is_some());

    // Score must be higher than single-modality matches
    if results.len() > 1 {
        assert!(results[0].rrf_score > results[1].rrf_score);
    }
}

#[test]
fn benchmark_missing_evidence_returns_empty() {
    let conn = setup_hybrid_test_db();
    let ws = "ws_test";
    let model = "test-embed";

    seed_memory(
        &conn,
        ws,
        "mem_cooking",
        "Italian Pasta Recipes",
        "# Recipes\nDelicious lasagna and penne pasta.",
        "note",
        false,
        "private",
    );

    // Completely unrelated query
    let req = HybridSearchRequest {
        workspace_id: ws.to_string(),
        query: Some("quantum astrophysics warp drive".to_string()),
        query_vector: None,
        model: Some(model.to_string()),
        limit: 10,
        lexical_weight: 1.0,
        semantic_weight: 1.0,
        rrf_k: 60.0,
        kind: None,
        include_archived: false,
        include_expired: false,
        endpoint: None,
    };

    let results = hybrid_search(&conn, &req).unwrap();
    assert!(results.is_empty());
}

#[test]
fn benchmark_privacy_and_boundaries() {
    let conn = setup_hybrid_test_db();
    let ws = "ws_test";
    let model = "test-embed";

    // Sealed memory
    seed_memory(
        &conn,
        ws,
        "mem_sealed",
        "Secret Key Vault",
        "# Credentials\nRoot access passwords and secret keys.",
        "decision",
        false,
        "sealed",
    );

    // Archived memory
    seed_memory(
        &conn,
        ws,
        "mem_archived",
        "Archived Project Plan",
        "# Old Project\nDeprecated architecture from 2021.",
        "decision",
        true,
        "private",
    );

    // Active memory
    seed_memory(
        &conn,
        ws,
        "mem_active",
        "Active Project Plan",
        "# Current Project\nActive modern architecture.",
        "note",
        false,
        "private",
    );

    let v = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
    store_embedding(&conn, ws, "mem_active", 0, model, &v).unwrap();
    // Invalidate/skip sealed record embedding check by inserting directly to test boundary
    anansi_core::vector::ensure_embeddings_table(&conn).unwrap();
    conn.execute(
        "INSERT INTO memory_embeddings (workspace_id, memory_id, chunk_index, model, dimension, embedding, created_at)
         VALUES (?1, 'mem_sealed', 0, ?2, 2, ?3, '2026-10-19T00:00:00Z')",
        rusqlite::params![ws, model, v.to_bytes()],
    )
    .unwrap();
    store_embedding(&conn, ws, "mem_archived", 0, model, &v).unwrap();

    // 1. Fail-closed Sealed exclusion
    let req = HybridSearchRequest {
        workspace_id: ws.to_string(),
        query: Some("project".to_string()),
        query_vector: Some(v.clone()),
        model: Some(model.to_string()),
        limit: 10,
        lexical_weight: 1.0,
        semantic_weight: 1.0,
        rrf_k: 60.0,
        kind: None,
        include_archived: false,
        include_expired: false,
        endpoint: None,
    };
    let results = hybrid_search(&conn, &req).unwrap();
    assert!(results.iter().all(|r| r.memory_id != "mem_sealed"));
    assert!(results.iter().all(|r| r.memory_id != "mem_archived"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].memory_id, "mem_active");

    // 2. Archived inclusion when requested
    let req_archived = HybridSearchRequest {
        include_archived: true,
        ..req.clone()
    };
    let results_archived = hybrid_search(&conn, &req_archived).unwrap();
    assert!(
        results_archived
            .iter()
            .any(|r| r.memory_id == "mem_archived")
    );
    assert!(results_archived.iter().all(|r| r.memory_id != "mem_sealed"));

    // 3. Kind filter
    let req_kind = HybridSearchRequest {
        kind: Some("decision".to_string()),
        include_archived: true,
        ..req
    };
    let results_kind = hybrid_search(&conn, &req_kind).unwrap();
    assert!(results_kind.iter().all(|r| r.kind == "decision"));
    assert!(results_kind.iter().all(|r| r.memory_id != "mem_sealed"));
}
