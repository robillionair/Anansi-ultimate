use anansi_core::{
    Core,
    hybrid::{HybridSearchRequest, hybrid_search},
    semantic::{DEFAULT_EMBEDDING_MODEL, embed_memory, embed_workspace_memories},
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

    // 7. Test real semantic query retrieval with paraphrase query
    // Paraphrase query: "cat diet and health" has semantic relevance to "Feline Wellness Guide"
    let hybrid_req = HybridSearchRequest {
        workspace_id: workspace_id.clone(),
        query: Some("cat diet and health".to_owned()),
        query_vector: None,
        model: Some(DEFAULT_EMBEDDING_MODEL.to_owned()),
        limit: 10,
        lexical_weight: 1.0,
        semantic_weight: 1.0,
        rrf_k: 60.0,
        kind: None,
        include_archived: false,
    };

    let hits = hybrid_search(&conn, &hybrid_req).unwrap();
    assert!(!hits.is_empty());
    assert_eq!(hits[0].memory_id, cat_id);
    assert!(hits[0].semantic_rank.is_some());

    // 8. Test that Sealed memory is NEVER present in results
    assert!(hits.iter().all(|h| h.memory_id != secret_id));

    // 9. Test workspace batch indexing
    let batch_indexed =
        embed_workspace_memories(&conn, &root, &workspace_id, DEFAULT_EMBEDDING_MODEL, None)
            .unwrap();
    assert!(batch_indexed >= 1);
}
