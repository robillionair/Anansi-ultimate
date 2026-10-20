//! Real semantic embedding generation, synchronization, and query pipeline.
//!
//! Master Specification §11, §19, §77–78; Requirements Matrix AN-005, AN-010.
//! Adheres strictly to the integration acceptance conditions:
//! 1. Transactional rollback propagation on invalidation failure.
//! 2. Derived embeddings populated only following explicit application submission,
//!    reading canonical current-revision metadata and integrity-checked content,
//!    excluding Sealed records before transmission, never holding a SQLite write
//!    transaction across model inference, and atomically rechecking revision/privacy
//!    and replacing the complete chunk set in one transaction at commit.
//! 3. Generating query embeddings using the same selected model and dimension,
//!    reporting an explicit lexical-only fallback on failure without labeling it semantic.
//! 4. Invalidation on edits, lifecycle changes, sealing, and restore.

use crate::context::chunk_markdown_sections;
use crate::vector::{
    EmbeddingVector, VectorError, delete_embeddings_for_memory, ensure_embeddings_table,
    generate_ollama_embedding, store_embedding_with_revision,
};
use crate::{CommandError, ErrorCode, content};
use rusqlite::{Connection, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DEFAULT_EMBEDDING_MODEL: &str = "all-minilm:22m";
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 384;
pub const MAX_CHUNKS_PER_MEMORY: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexEmbeddingsRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub memory_id: Option<String>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexEmbeddingsResponse {
    pub contract_version: u8,
    pub indexed_memories: usize,
    pub indexed_chunks: usize,
    pub model: String,
    pub dimension: usize,
}

fn validate_model_name(model: &str) -> Result<&str, CommandError> {
    let trimmed = model.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "Provide a valid embedding model name (1 to 128 characters).",
        ));
    }
    Ok(trimmed)
}

/// Generates and stores embeddings for a single memory record using the specified local model.
///
/// Ensures:
/// - Canonical current-revision metadata and integrity-verified body are read.
/// - Sealed records are rejected fail-closed before any HTTP transmission.
/// - Model inference is performed outside of any SQLite write transactions.
/// - At commit time, a single transaction acquires the write lock, re-verifies current
///   revision, content hash, status, and privacy, deletes the previous chunk set for that
///   model, and inserts the complete new chunk set atomically.
pub fn embed_memory(
    conn: &Connection,
    storage_root: &Path,
    workspace_id: &str,
    memory_id: &str,
    model: &str,
    endpoint: Option<&str>,
) -> Result<usize, CommandError> {
    let (chunks, _dim) = embed_memory_with_hook(
        conn,
        storage_root,
        workspace_id,
        memory_id,
        model,
        endpoint,
        || Ok(()),
    )?;
    Ok(chunks)
}

pub fn embed_memory_with_hook(
    conn: &Connection,
    storage_root: &Path,
    workspace_id: &str,
    memory_id: &str,
    model: &str,
    endpoint: Option<&str>,
    before_commit: impl FnOnce() -> Result<(), CommandError>,
) -> Result<(usize, usize), CommandError> {
    let model = validate_model_name(model)?;
    ensure_embeddings_table(conn)
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

    // 1. Read canonical current-revision metadata without holding a write transaction
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
        delete_embeddings_for_memory(conn, workspace_id, memory_id)
            .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;
        return Ok((0, 0));
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
    let effective_endpoint = endpoint
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("http://127.0.0.1:11434");

    let mut expected_dim: Option<usize> = None;
    for chunk in bounded_chunks {
        let chunk_hash = content::hash(chunk.content.as_bytes());

        // Call loopback inference endpoint
        let vector = generate_ollama_embedding(effective_endpoint, model, &chunk.content)
            .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

        if let Some(dim) = expected_dim {
            if vector.dimension() != dim {
                return Err(CommandError::new(
                    ErrorCode::StorageUnavailable,
                    "Embedding model returned inconsistent vector dimensions across chunks.",
                ));
            }
        } else {
            expected_dim = Some(vector.dimension());
        }

        prepared_embeddings.push((chunk.chunk_index, chunk_hash, vector));
    }

    // Optional hook for deterministic concurrent-modification tests
    before_commit()?;

    // 5. Atomic commit phase: replace complete chunk set and recheck revision & privacy in one transaction
    conn.execute_batch("SAVEPOINT anansi_embed_publish;")
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

    let rollback = |conn: &Connection| {
        let _ = conn.execute_batch(
            "ROLLBACK TO SAVEPOINT anansi_embed_publish; RELEASE SAVEPOINT anansi_embed_publish;",
        );
    };

    // Acquire the write lock and clear previous chunk set for this (workspace, memory, model)
    if let Err(e) = conn.execute(
        "DELETE FROM memory_embeddings WHERE workspace_id = ?1 AND memory_id = ?2 AND model = ?3",
        params![workspace_id, memory_id, model],
    ) {
        rollback(conn);
        return Err(CommandError::new(
            ErrorCode::StorageUnavailable,
            &e.to_string(),
        ));
    }

    // Recheck canonical revision, body_hash, privacy, and status under the write lock
    let current_state = conn.query_row(
        "SELECT m.current_revision, r.body_hash,
                json_extract(r.metadata, '$.privacy'),
                json_extract(r.metadata, '$.status')
         FROM memories m
         JOIN memory_revisions r ON r.workspace_id = m.workspace_id AND r.memory_id = m.id AND r.id = m.current_revision
         WHERE m.workspace_id = ?1 AND m.id = ?2",
        params![workspace_id, memory_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    );

    let (verified_revision, verified_hash, verified_privacy, verified_status) = match current_state
    {
        Ok(state) => state,
        Err(_) => {
            rollback(conn);
            return Err(crate::storage::unavailable());
        }
    };

    if verified_privacy.eq_ignore_ascii_case("sealed") {
        rollback(conn);
        return Err(CommandError::new(
            ErrorCode::InvalidMemory,
            "Sealed memories are strictly excluded from embedding generation.",
        ));
    }

    if verified_revision != current_revision
        || verified_hash != hash
        || verified_status == "archived"
    {
        rollback(conn);
        return Err(CommandError::new(
            ErrorCode::RevisionConflict,
            "Memory revision changed during embedding generation; stale embeddings discarded.",
        ));
    }

    // Insert the complete new chunk set inside the same transaction
    for (chunk_index, chunk_hash, vector) in &prepared_embeddings {
        if let Err(e) = store_embedding_with_revision(
            conn,
            workspace_id,
            memory_id,
            &current_revision,
            chunk_hash,
            *chunk_index,
            model,
            vector,
        ) {
            rollback(conn);
            return Err(CommandError::new(
                ErrorCode::StorageUnavailable,
                &e.to_string(),
            ));
        }
    }

    conn.execute_batch("RELEASE SAVEPOINT anansi_embed_publish;")
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

    Ok((prepared_embeddings.len(), expected_dim.unwrap_or(0)))
}

/// Populates derived embeddings for all active non-sealed memories in a workspace.
pub fn embed_workspace_memories(
    conn: &Connection,
    storage_root: &Path,
    workspace_id: &str,
    model: &str,
    endpoint: Option<&str>,
) -> Result<usize, CommandError> {
    let (_memories, chunks, _dim) =
        embed_workspace_memories_detailed(conn, storage_root, workspace_id, model, endpoint)?;
    Ok(chunks)
}

/// Populates derived embeddings for all active non-sealed memories in a workspace,
/// returning `(indexed_memories, indexed_chunks, dimension)`.
pub fn embed_workspace_memories_detailed(
    conn: &Connection,
    storage_root: &Path,
    workspace_id: &str,
    model: &str,
    endpoint: Option<&str>,
) -> Result<(usize, usize, usize), CommandError> {
    let model = validate_model_name(model)?;
    crate::workspace::validate_id(workspace_id)?;
    ensure_embeddings_table(conn)
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

    // Prune any orphaned, archived, or sealed embeddings first
    crate::vector::prune_stale_embeddings_for_workspace(conn, workspace_id)
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
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

    let mut indexed_memories = 0usize;
    let mut total_chunks = 0usize;
    let mut detected_dim = 0usize;
    for mem_id in memory_ids {
        match embed_memory_with_hook(
            conn,
            storage_root,
            workspace_id,
            &mem_id,
            model,
            endpoint,
            || Ok(()),
        ) {
            Ok((chunks, dim)) => {
                if chunks > 0 {
                    indexed_memories += 1;
                    total_chunks += chunks;
                    if dim > 0 {
                        detected_dim = dim;
                    }
                }
            }
            Err(e)
                if e.code == ErrorCode::InvalidMemory || e.code == ErrorCode::RevisionConflict =>
            {
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Ok((indexed_memories, total_chunks, detected_dim))
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
    let effective_endpoint = endpoint
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("http://127.0.0.1:11434");
    generate_ollama_embedding(effective_endpoint, model.trim(), trimmed)
}
