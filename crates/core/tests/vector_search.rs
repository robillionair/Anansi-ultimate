use anansi_core::vector::{EmbeddingVector, invalidate_embeddings, store_embedding, vector_search};
use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE memories (
            workspace_id TEXT NOT NULL,
            id TEXT NOT NULL,
            current_revision TEXT NOT NULL DEFAULT 'rev_1',
            archived INTEGER NOT NULL DEFAULT 0,
            privacy TEXT NOT NULL DEFAULT 'private',
            PRIMARY KEY (workspace_id, id)
        );",
    )
    .unwrap();
    conn
}

#[test]
fn vector_search_ranks_by_cosine_similarity() {
    let conn = setup_test_db();
    let ws = "ws_test";
    let model = "test-model";

    // Insert 4 active memories
    for (id, privacy) in [
        ("mem_ai", "private"),
        ("mem_web", "private"),
        ("mem_db", "private"),
        ("mem_crypto", "private"),
    ] {
        conn.execute(
            "INSERT INTO memories (workspace_id, id, privacy) VALUES (?1, ?2, ?3)",
            [ws, id, privacy],
        )
        .unwrap();
    }

    // Embeddings
    let v_ai = EmbeddingVector::new(vec![0.9, 0.1, 0.0, 0.0]).unwrap();
    let v_web = EmbeddingVector::new(vec![0.1, 0.8, 0.2, 0.0]).unwrap();
    let v_db = EmbeddingVector::new(vec![0.0, 0.2, 0.9, 0.1]).unwrap();
    let v_crypto = EmbeddingVector::new(vec![0.0, 0.0, 0.1, 0.9]).unwrap();

    store_embedding(&conn, ws, "mem_ai", 0, model, &v_ai).unwrap();
    store_embedding(&conn, ws, "mem_web", 0, model, &v_web).unwrap();
    store_embedding(&conn, ws, "mem_db", 0, model, &v_db).unwrap();
    store_embedding(&conn, ws, "mem_crypto", 0, model, &v_crypto).unwrap();

    // Query close to AI
    let query_ai = EmbeddingVector::new(vec![0.95, 0.05, 0.0, 0.0]).unwrap();
    let results = vector_search(&conn, ws, &query_ai, model, 10, None).unwrap();

    assert_eq!(results.len(), 4);
    assert_eq!(results[0].memory_id, "mem_ai");
    assert!(results[0].score > 0.98);
    // AI > Web > DB > Crypto relative to query_ai
    assert_eq!(results[1].memory_id, "mem_web");
    assert!(results[0].score > results[1].score);
    assert!(results[1].score > results[2].score);

    // Query close to Database
    let query_db = EmbeddingVector::new(vec![0.05, 0.15, 0.95, 0.05]).unwrap();
    let results_db = vector_search(&conn, ws, &query_db, model, 10, None).unwrap();
    assert_eq!(results_db[0].memory_id, "mem_db");
    assert!(results_db[0].score > 0.98);
}

#[test]
fn vector_search_respects_threshold_and_limits() {
    let conn = setup_test_db();
    let ws = "ws_test";
    let model = "test-model";

    conn.execute(
        "INSERT INTO memories (workspace_id, id, privacy) VALUES (?1, 'mem_1', 'private'), (?1, 'mem_2', 'private'), (?1, 'mem_3', 'private')",
        [ws],
    )
    .unwrap();

    let v1 = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
    let v2 = EmbeddingVector::new(vec![0.6, 0.8]).unwrap(); // sim 0.6 with [1.0, 0.0]
    let v3 = EmbeddingVector::new(vec![0.0, 1.0]).unwrap(); // sim 0.0 with [1.0, 0.0]

    store_embedding(&conn, ws, "mem_1", 0, model, &v1).unwrap();
    store_embedding(&conn, ws, "mem_2", 0, model, &v2).unwrap();
    store_embedding(&conn, ws, "mem_3", 0, model, &v3).unwrap();

    let query = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();

    // Limit = 2
    let top2 = vector_search(&conn, ws, &query, model, 2, None).unwrap();
    assert_eq!(top2.len(), 2);
    assert_eq!(top2[0].memory_id, "mem_1");
    assert_eq!(top2[1].memory_id, "mem_2");

    // min_score = Some(0.8) -> should only return mem_1
    let threshold = vector_search(&conn, ws, &query, model, 10, Some(0.8)).unwrap();
    assert_eq!(threshold.len(), 1);
    assert_eq!(threshold[0].memory_id, "mem_1");

    // min_score = Some(0.5) -> should return mem_1 and mem_2
    let threshold_half = vector_search(&conn, ws, &query, model, 10, Some(0.5)).unwrap();
    assert_eq!(threshold_half.len(), 2);
    assert_eq!(threshold_half[0].memory_id, "mem_1");
    assert_eq!(threshold_half[1].memory_id, "mem_2");
}

#[test]
fn vector_search_excludes_archived_memories() {
    let conn = setup_test_db();
    let ws = "ws_test";
    let model = "test-model";

    // Insert 2 memories, mem_active and mem_archived
    conn.execute(
        "INSERT INTO memories (workspace_id, id, archived, privacy) VALUES (?1, 'mem_active', 0, 'private')",
        [ws],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO memories (workspace_id, id, archived, privacy) VALUES (?1, 'mem_archived', 1, 'private')",
        [ws],
    )
    .unwrap();

    let v_perfect = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
    let v_fallback = EmbeddingVector::new(vec![0.8, 0.2]).unwrap();

    // Even though mem_archived has a higher similarity vector (1.0 vs 0.8), it must be omitted
    store_embedding(&conn, ws, "mem_archived", 0, model, &v_perfect).unwrap();
    store_embedding(&conn, ws, "mem_active", 0, model, &v_fallback).unwrap();

    let query = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
    let results = vector_search(&conn, ws, &query, model, 10, None).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].memory_id, "mem_active");
}

#[test]
fn vector_search_excludes_sealed_memories_fail_closed() {
    let conn = setup_test_db();
    let ws = "ws_test";
    let model = "test-model";

    conn.execute(
        "INSERT INTO memories (workspace_id, id, archived, privacy) VALUES (?1, 'mem_public', 0, 'private')",
        [ws],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO memories (workspace_id, id, archived, privacy) VALUES (?1, 'mem_secret', 0, 'sealed')",
        [ws],
    )
    .unwrap();

    anansi_core::vector::ensure_embeddings_table(&conn).unwrap();
    // Attempting direct insertion into memory_embeddings table bypasses store_embedding check for test purposes
    let bytes = EmbeddingVector::new(vec![1.0, 0.0]).unwrap().to_bytes();
    conn.execute(
        "INSERT INTO memory_embeddings (workspace_id, memory_id, chunk_index, model, dimension, embedding, created_at)
         VALUES (?1, 'mem_secret', 0, ?2, 2, ?3, '2026-10-19T00:00:00Z')",
        rusqlite::params![ws, model, bytes],
    )
    .unwrap();

    let v_pub = EmbeddingVector::new(vec![0.5, 0.5]).unwrap();
    store_embedding(&conn, ws, "mem_public", 0, model, &v_pub).unwrap();

    let query = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
    let results = vector_search(&conn, ws, &query, model, 10, None).unwrap();

    // Sealed memory MUST NEVER appear in search results
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].memory_id, "mem_public");
}

#[test]
fn cache_invalidation_clears_embeddings() {
    let conn = setup_test_db();
    let ws = "ws_test";
    let model = "test-model";

    conn.execute(
        "INSERT INTO memories (workspace_id, id, privacy) VALUES (?1, 'mem_updated', 'private')",
        [ws],
    )
    .unwrap();

    let v = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
    store_embedding(&conn, ws, "mem_updated", 0, model, &v).unwrap();

    let query = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
    assert_eq!(
        vector_search(&conn, ws, &query, model, 10, None)
            .unwrap()
            .len(),
        1
    );

    // Invalidate
    invalidate_embeddings(&conn, ws, "mem_updated").unwrap();

    // Now search returns 0 results for this memory
    assert_eq!(
        vector_search(&conn, ws, &query, model, 10, None)
            .unwrap()
            .len(),
        0
    );
}
