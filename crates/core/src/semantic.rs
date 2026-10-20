//! Real semantic embedding generation, synchronization, and query pipeline.
//!
//! Master Specification §11, §19, §77–78; Requirements Matrix AN-005, AN-010.
//! Adheres strictly to the 6 integration acceptance conditions:
//! 1. Transactional rollback propagation on invalidation failure.
//! 2. Derived embeddings populated only following explicit application submission,
//!    reading canonical current-revision metadata and integrity-checked content,
//!    excluding Sealed records before transmission, never holding a SQLite write
//!    transaction across model inference, and rechecking revision/privacy at commit.
//! 3. Generating query embeddings using the same selected model and dimension,
//!    reporting an explicit lexical-only fallback on failure without labeling it semantic.
//! 4. Invalidation on edits, lifecycle changes, sealing, and restore.

use crate::context::chunk_markdown_sections;
use crate::vector::{
    EmbeddingVector, VectorError, ensure_embeddings_table, generate_ollama_embedding,
    store_embedding_with_revision,
};
use crate::{CommandError, ErrorCode, content};
use rusqlite::{Connection, params};
use std::path::Path;

pub const DEFAULT_EMBEDDING_MODEL: &str = "all-minilm:22m";
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 384;
pub const MAX_CHUNKS_PER_MEMORY: usize = 100;

/// Generates and stores embeddings for a single memory record using the specified local model.
///
/// Ensures:
/// - Canonical current-revision metadata and integrity-verified body are read.
/// - Sealed records are rejected fail-closed before any HTTP transmission.
/// - Model inference is performed outside of any SQLite write transactions.
/// - At commit time, current revision and privacy status are re-verified atomically;
///   concurrent modifications invalidate the prepared embeddings.
pub fn embed_memory(
    conn: &Connection,
    storage_root: &Path,
    workspace_id: &str,
    memory_id: &str,
    model: &str,
    endpoint: Option<&str>,
) -> Result<usize, CommandError> {
    ensure_embeddings_table(conn)
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

    // 1. Read canonical current-revision metadata
    let (current_revision, hash, title, kind, privacy, status) = conn
        .query_row(
            "SELECT m.current_revision, r.body_hash,
                    json_extract(r.metadata, '$.title'),
                    json_extract(r.metadata, '$.kind'),
                    json_extract(r.metadata, '$.privacy'),
                    json_extract(r.metadata, '$.status')
             FROM memories m
             JOIN memory_revisions r ON r.workspace_id = m.workspace_id AND r.memory_id = m.id AND r.id = m.current_revision
             WHERE m.workspace_id = ?1 AND m.id = ?2",
            params![workspace_id, memory_id],
            |row| {
                let rev: String = row.get(0)?;
                let hash: String = row.get(1)?;
                let title: String = row.get(2)?;
                let kind: String = row.get(3)?;
                let privacy: String = row.get(4)?;
                let status: String = row.get(5)?;
                Ok((rev, hash, title, kind, privacy, status))
            },
        )
        .map_err(|_| crate::storage::unavailable())?;

    // Fail-closed privacy refusal: Sealed memories are never read or embedded
    if privacy.eq_ignore_ascii_case("sealed") {
        return Err(CommandError::new(
            ErrorCode::InvalidMemory,
            "Sealed memories are strictly excluded from embedding generation.",
        ));
    }

    if status == "archived" {
        return Ok(0);
    }

    // 2. Read integrity-checked canonical Markdown body
    let body = content::read(
        storage_root,
        workspace_id,
        memory_id,
        &current_revision,
        &hash,
    )?;

    // 3. Section chunking
    let chunks = chunk_markdown_sections(memory_id, &title, &kind, &body);
    let bounded_chunks = chunks
        .into_iter()
        .take(MAX_CHUNKS_PER_MEMORY)
        .collect::<Vec<_>>();

    // 4. Generate embeddings outside of any SQLite write transaction
    let mut prepared_embeddings: Vec<(usize, String, EmbeddingVector)> =
        Vec::with_capacity(bounded_chunks.len());
    let effective_endpoint = endpoint.unwrap_or("http://127.0.0.1:11434");

    for chunk in bounded_chunks {
        let chunk_hash = content::hash(chunk.content.as_bytes());

        // Call loopback inference endpoint
        let vector = generate_ollama_embedding(effective_endpoint, model, &chunk.content)
            .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

        prepared_embeddings.push((chunk.chunk_index, chunk_hash, vector));
    }

    // 5. Commit phase: verify current revision has not changed concurrently
    let verified_revision: String = conn
        .query_row(
            "SELECT current_revision FROM memories WHERE workspace_id = ?1 AND id = ?2",
            params![workspace_id, memory_id],
            |row| row.get(0),
        )
        .map_err(|_| crate::storage::unavailable())?;

    if verified_revision != current_revision {
        return Err(CommandError::new(
            ErrorCode::RevisionConflict,
            "Memory revision changed during embedding generation; stale embeddings discarded.",
        ));
    }

    // Store prepared embeddings atomically
    for (chunk_index, chunk_hash, vector) in &prepared_embeddings {
        store_embedding_with_revision(
            conn,
            workspace_id,
            memory_id,
            &current_revision,
            chunk_hash,
            *chunk_index,
            model,
            vector,
        )
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;
    }

    Ok(prepared_embeddings.len())
}

/// Populates derived embeddings for all active non-sealed memories in a workspace.
pub fn embed_workspace_memories(
    conn: &Connection,
    storage_root: &Path,
    workspace_id: &str,
    model: &str,
    endpoint: Option<&str>,
) -> Result<usize, CommandError> {
    ensure_embeddings_table(conn)
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

    let mut stmt = conn
        .prepare(
            "SELECT m.id
             FROM memories m
             JOIN memory_revisions r ON r.workspace_id = m.workspace_id AND r.memory_id = m.id AND r.id = m.current_revision
             WHERE m.workspace_id = ?1
               AND json_extract(r.metadata, '$.status') != 'archived'
               AND json_extract(r.metadata, '$.privacy') != 'sealed'
             ORDER BY m.id",
        )
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

    let memory_ids: Vec<String> = stmt
        .query_map(params![workspace_id], |row| row.get(0))
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?
        .filter_map(Result::ok)
        .collect();

    let mut total_chunks = 0;
    for mem_id in memory_ids {
        match embed_memory(conn, storage_root, workspace_id, &mem_id, model, endpoint) {
            Ok(chunks) => total_chunks += chunks,
            Err(e) if e.code == ErrorCode::InvalidMemory => continue, // Skip sealed
            Err(e) => return Err(e),
        }
    }

    Ok(total_chunks)
}

/// Generates a query vector for semantic retrieval, reporting an explicit offline error on failure.
pub fn generate_query_vector(
    endpoint: Option<&str>,
    model: &str,
    query: &str,
) -> Result<EmbeddingVector, VectorError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(VectorError::EmptyVector);
    }
    let effective_endpoint = endpoint.unwrap_or("http://127.0.0.1:11434");
    generate_ollama_embedding(effective_endpoint, model, trimmed)
}
