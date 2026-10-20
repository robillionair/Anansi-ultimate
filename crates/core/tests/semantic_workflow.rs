use anansi_core::{
    Core, ErrorCode,
    hybrid::{HybridSearchRequest, hybrid_search_with_status},
    semantic::{
        DEFAULT_EMBEDDING_MODEL, embed_memory, embed_memory_with_hook, embed_workspace_memories,
    },
    storage::DATABASE_NAME,
};
use rusqlite::Connection;
use serde_json::json;

#[test]
fn real_semantic_embedding_indexing_retrieval_and_sealed_refusal() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());

    // 1. Create workspace
    let ws_res = core
        .workspace_command(
            "create",
            json!({"contractVersion": 1, "name": "Semantic Integration Workspace"}),
        )
        .unwrap();
    let workspace_id = ws_res["workspace"]["id"].as_str().unwrap().to_owned();

    // 2. Sealed creation without active vault fails closed upfront
    let unvaulted_err = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace_id,
                "kind": "note",
                "draft": {
                    "title": "Confidential Vault Formula",
                    "body": "This is proprietary kitten nutritional formula.",
                    "privacy": "sealed"
                }
            }),
        )
        .unwrap_err();
    assert_eq!(unvaulted_err.code, anansi_core::ErrorCode::VaultUnavailable);

    // 3. Create unsealed memory with rich content
    let cat_mem = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace_id,
                "kind": "note",
                "draft": {
                    "title": "Feline Wellness Guide",
                    "body": "# Nutrition\nDomestic cats require a protein-rich diet and regular veterinary checkups.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let cat_id = cat_mem["memory"]["id"].as_str().unwrap().to_owned();

    // 4. Create another memory that transitions to Sealed (e.g. from vault admission)
    let secret_mem = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace_id,
                "kind": "note",
                "draft": {
                    "title": "Confidential Project",
                    "body": "Top secret feline recipe.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let secret_id = secret_mem["memory"]["id"].as_str().unwrap().to_owned();

    let conn = Connection::open(root.join(DATABASE_NAME)).unwrap();

    // Transition second memory to sealed in catalog
    conn.execute(
        "UPDATE memory_revisions SET metadata = json_set(metadata, '$.privacy', 'sealed') WHERE memory_id = ?1",
        [&secret_id],
    )
    .unwrap();

    // 5. Test Sealed refusal during embedding generation
    let sealed_err = embed_memory(
        &conn,
        &root,
        &workspace_id,
        &secret_id,
        DEFAULT_EMBEDDING_MODEL,
        None,
    )
    .unwrap_err();
    assert_eq!(sealed_err.code, anansi_core::ErrorCode::InvalidMemory);

    // 6. Embed unsealed memory using real installed all-minilm:22m model
    let chunks_indexed = embed_memory(
        &conn,
        &root,
        &workspace_id,
        &cat_id,
        DEFAULT_EMBEDDING_MODEL,
        None,
    )
    .unwrap();
    assert!(chunks_indexed >= 1);

    // Verify embedding row in SQLite
    let (dim, has_rev, has_hash): (i64, String, String) = conn
        .query_row(
            "SELECT dimension, revision_id, content_hash FROM memory_embeddings WHERE workspace_id = ?1 AND memory_id = ?2",
            rusqlite::params![workspace_id, cat_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(dim, 384);
    assert!(!has_rev.is_empty());
    assert!(!has_hash.is_empty());

    // 7. Test real semantic query retrieval with paraphrase query (0 keyword overlap: "kitty feeding schedule")
    let hybrid_req = HybridSearchRequest {
        workspace_id: workspace_id.clone(),
        query: Some("kitty feeding schedule".to_owned()),
        query_vector: None,
        model: Some(DEFAULT_EMBEDDING_MODEL.to_owned()),
        limit: 10,
        lexical_weight: 1.0,
        semantic_weight: 1.0,
        rrf_k: 60.0,
        kind: None,
        include_archived: false,
        include_expired: false,
        endpoint: None,
    };

    let response = hybrid_search_with_status(&conn, Some(&root), &hybrid_req).unwrap();
    assert!(!response.lexical_only);
    assert!(response.fallback_reason.is_none());
    let hits = response.results;
    assert!(!hits.is_empty());
    assert_eq!(hits[0].memory_id, cat_id);
    assert_eq!(hits[0].lexical_rank, None);
    assert_eq!(hits[0].semantic_rank, Some(1));
    assert!(hits[0].cosine_similarity.unwrap_or(0.0) > 0.25);
    assert!(hits[0].rerank_score.unwrap_or(0.0) > 0.0);
    assert!(
        hits[0]
            .snippet
            .as_deref()
            .unwrap_or("")
            .contains("protein-rich diet")
    );

    // 8. Test that Sealed memory is NEVER present in results
    assert!(hits.iter().all(|h| h.memory_id != secret_id));

    // 9. Test workspace batch indexing
    let batch_indexed =
        embed_workspace_memories(&conn, &root, &workspace_id, DEFAULT_EMBEDDING_MODEL, None)
            .unwrap();
    assert!(batch_indexed >= 1);
}

#[test]
fn concurrent_edit_and_privacy_change_abort_atomic_embedding_publication() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());

    let ws_res = core
        .workspace_command(
            "create",
            json!({"contractVersion": 1, "name": "Atomic Publication Workspace"}),
        )
        .unwrap();
    let workspace_id = ws_res["workspace"]["id"].as_str().unwrap().to_owned();

    let created = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace_id,
                "kind": "note",
                "draft": {
                    "title": "Orbital Navigation Spec",
                    "body": "# Propulsion\nIon thrusters provide high specific impulse for deep space transit.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let memory_id = created["memory"]["id"].as_str().unwrap().to_owned();
    let rev1 = created["memory"]["revisionId"].as_str().unwrap().to_owned();

    let conn = Connection::open(root.join(DATABASE_NAME)).unwrap();

    // Case A: Concurrent revision edit occurs while Ollama is computing embeddings
    let core_for_hook = Core::with_storage_root(root.clone());
    let ws_for_hook = workspace_id.clone();
    let mem_for_hook = memory_id.clone();
    let rev_for_hook = rev1.clone();
    let conflict_err = embed_memory_with_hook(
        &conn,
        &root,
        &workspace_id,
        &memory_id,
        DEFAULT_EMBEDDING_MODEL,
        None,
        || {
            // Another writer revises the memory before embeddings are published
            core_for_hook
                .memory_command(
                    "revise",
                    json!({
                        "contractVersion": 1,
                        "workspaceId": ws_for_hook,
                        "id": mem_for_hook,
                        "expectedRevision": rev_for_hook,
                        "draft": {
                            "title": "Orbital Navigation Spec v2",
                            "body": "# Propulsion\nChemical rockets are used for launch.",
                            "privacy": "private"
                        }
                    }),
                )
                .unwrap();
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(conflict_err.code, ErrorCode::RevisionConflict);

    // Verify 0 stale embeddings were published
    let count_after_conflict: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_embeddings WHERE workspace_id = ?1 AND memory_id = ?2",
            rusqlite::params![workspace_id, memory_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    assert_eq!(count_after_conflict, 0);

    // Case B: Concurrent transition to Sealed occurs before commit
    let mem_for_seal = memory_id.clone();
    let seal_err = embed_memory_with_hook(
        &conn,
        &root,
        &workspace_id,
        &memory_id,
        DEFAULT_EMBEDDING_MODEL,
        None,
        || {
            conn.execute(
                "UPDATE memory_revisions SET metadata = json_set(metadata, '$.privacy', 'sealed') WHERE memory_id = ?1",
                [&mem_for_seal],
            )
            .unwrap();
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(seal_err.code, ErrorCode::InvalidMemory);

    let count_after_seal: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_embeddings WHERE workspace_id = ?1 AND memory_id = ?2",
            rusqlite::params![workspace_id, memory_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    assert_eq!(count_after_seal, 0);
}

#[test]
fn partial_chunk_insert_failure_rolls_back_complete_chunk_set() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());

    let ws_res = core
        .workspace_command(
            "create",
            json!({"contractVersion": 1, "name": "Partial Failure Workspace"}),
        )
        .unwrap();
    let workspace_id = ws_res["workspace"]["id"].as_str().unwrap().to_owned();

    // Multi-section memory that produces 2 chunks
    let created = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace_id,
                "kind": "note",
                "draft": {
                    "title": "Multi-Section Architecture",
                    "body": "# Section Alpha\nFirst chunk content describing distributed consensus.\n\n## Section Beta\nSecond chunk content describing replicated state machines.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let memory_id = created["memory"]["id"].as_str().unwrap().to_owned();

    let conn = Connection::open(root.join(DATABASE_NAME)).unwrap();
    anansi_core::vector::ensure_embeddings_table(&conn).unwrap();

    // Install a SQLite trigger that aborts when chunk_index >= 1 is inserted
    conn.execute_batch(
        "CREATE TRIGGER fail_on_second_chunk
         BEFORE INSERT ON memory_embeddings
         WHEN NEW.chunk_index >= 1
         BEGIN
             SELECT RAISE(ABORT, 'simulated mid-batch storage failure on second chunk');
         END;",
    )
    .unwrap();

    let res = embed_memory(
        &conn,
        &root,
        &workspace_id,
        &memory_id,
        DEFAULT_EMBEDDING_MODEL,
        None,
    );
    assert!(res.is_err());

    // Chunk 0 must have been rolled back atomically with the savepoint
    let remaining_chunks: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_embeddings WHERE workspace_id = ?1 AND memory_id = ?2",
            rusqlite::params![workspace_id, memory_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remaining_chunks, 0);

    // Drop the trigger and verify normal atomic publication writes both chunks
    conn.execute_batch("DROP TRIGGER fail_on_second_chunk;")
        .unwrap();
    let published = embed_memory(
        &conn,
        &root,
        &workspace_id,
        &memory_id,
        DEFAULT_EMBEDDING_MODEL,
        None,
    )
    .unwrap();
    assert_eq!(published, 2);
}

#[test]
fn reranking_temporal_validity_and_contradiction_lifecycle_evaluation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("storage");
    let core = Core::with_storage_root(root.clone());

    let ws_res = core
        .workspace_command(
            "create",
            json!({"contractVersion": 1, "name": "Rerank and Temporal Evaluation"}),
        )
        .unwrap();
    let workspace_id = ws_res["workspace"]["id"].as_str().unwrap().to_owned();

    // 1. Old decision (2024) that will be superseded by the 2026 decision
    let old_decision = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace_id,
                "kind": "decision",
                "draft": {
                    "title": "Database Engine Selection (2024)",
                    "body": "# Storage Decision\nWe selected PostgreSQL as the primary database engine for local storage.",
                    "privacy": "private",
                    "confidence": 0.4,
                    "eventDate": "2024-01-15T00:00:00Z"
                }
            }),
        )
        .unwrap();
    let superseded_id = old_decision["memory"]["id"].as_str().unwrap().to_owned();
    let old_rev1 = old_decision["memory"]["revisionId"]
        .as_str()
        .unwrap()
        .to_owned();

    // 2. Active confirmed current decision (2026) that supersedes the 2024 decision
    let active_decision = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace_id,
                "kind": "decision",
                "draft": {
                    "title": "Database Engine Selection (2026)",
                    "body": "# Storage Decision\nWe selected SQLite with FTS5 as the primary database engine for local storage.",
                    "privacy": "private",
                    "confidence": 0.95,
                    "eventDate": "2026-10-01T00:00:00Z"
                }
            }),
        )
        .unwrap();
    let active_id = active_decision["memory"]["id"].as_str().unwrap().to_owned();
    let active_rev1 = active_decision["memory"]["revisionId"]
        .as_str()
        .unwrap()
        .to_owned();

    // Transition active decision to Confirmed
    core.memory_command(
        "revise",
        json!({
            "contractVersion": 1,
            "workspaceId": workspace_id,
            "id": active_id,
            "expectedRevision": active_rev1,
            "lifecycle": {
                "state": "confirmed",
                "confirmed": true
            },
            "draft": {
                "title": "Database Engine Selection (2026)",
                "body": "# Storage Decision\nWe selected SQLite with FTS5 as the primary database engine for local storage.",
                "privacy": "private",
                "confidence": 0.95,
                "eventDate": "2026-10-01T00:00:00Z"
            }
        }),
    )
    .unwrap();

    // Transition 2024 decision Proposed -> Confirmed -> Superseded by active_id
    let old_confirmed = core
        .memory_command(
            "revise",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace_id,
                "id": superseded_id,
                "expectedRevision": old_rev1,
                "lifecycle": {
                    "state": "confirmed",
                    "confirmed": true
                },
                "draft": {
                    "title": "Database Engine Selection (2024)",
                    "body": "# Storage Decision\nWe selected PostgreSQL as the primary database engine for local storage.",
                    "privacy": "private",
                    "confidence": 0.4,
                    "eventDate": "2024-01-15T00:00:00Z"
                }
            }),
        )
        .unwrap();
    let old_rev2 = old_confirmed["memory"]["revisionId"]
        .as_str()
        .unwrap()
        .to_owned();

    core.memory_command(
        "revise",
        json!({
            "contractVersion": 1,
            "workspaceId": workspace_id,
            "id": superseded_id,
            "expectedRevision": old_rev2,
            "lifecycle": {
                "state": "superseded",
                "confirmed": true,
                "successorId": active_id
            },
            "draft": {
                "title": "Database Engine Selection (2024)",
                "body": "# Storage Decision\nWe selected PostgreSQL as the primary database engine for local storage.",
                "privacy": "private",
                "confidence": 0.4,
                "eventDate": "2024-01-15T00:00:00Z"
            }
        }),
    )
    .unwrap();

    // 3. Temporally expired notice (`validUntil` in the past)
    let expired_notice = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace_id,
                "kind": "note",
                "draft": {
                    "title": "Temporary Database Freeze",
                    "body": "# Storage Decision\nPrimary database engine migrations are frozen during maintenance.",
                    "privacy": "private",
                    "validUntil": "2025-01-01T00:00:00Z"
                }
            }),
        )
        .unwrap();
    let expired_id = expired_notice["memory"]["id"].as_str().unwrap().to_owned();

    let conn = Connection::open(root.join(DATABASE_NAME)).unwrap();
    embed_workspace_memories(&conn, &root, &workspace_id, DEFAULT_EMBEDDING_MODEL, None).unwrap();

    // Query with `include_expired: false` (default)
    let req = HybridSearchRequest {
        workspace_id: workspace_id.clone(),
        query: Some("primary database engine for local storage".to_owned()),
        query_vector: None,
        model: Some(DEFAULT_EMBEDDING_MODEL.to_owned()),
        limit: 10,
        lexical_weight: 1.0,
        semantic_weight: 1.0,
        rrf_k: 60.0,
        kind: None,
        include_archived: false,
        include_expired: false,
        endpoint: None,
    };

    let response = hybrid_search_with_status(&conn, Some(&root), &req).unwrap();
    assert!(!response.lexical_only);
    let results = &response.results;

    // Expired memory must be filtered out when `include_expired` is false
    assert!(results.iter().all(|r| r.memory_id != expired_id));

    // Active 2026 decision must outrank the Superseded 2024 decision due to lifecycle + recency + confidence reranking
    let active_pos = results
        .iter()
        .position(|r| r.memory_id == active_id)
        .unwrap();
    let superseded_pos = results
        .iter()
        .position(|r| r.memory_id == superseded_id)
        .unwrap();
    assert!(
        active_pos < superseded_pos,
        "Active current decision must outrank superseded contradicted decision"
    );
    assert!(
        results[active_pos].rerank_score.unwrap_or(0.0)
            > results[superseded_pos].rerank_score.unwrap_or(0.0)
    );
    assert!(
        results[superseded_pos]
            .rerank_provenance
            .as_deref()
            .unwrap_or("")
            .contains("lifecycle(superseded)=0.35")
    );
}
