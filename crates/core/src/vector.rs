//! Deterministic local vector embeddings and pure Rust vector operations.
//! Supports loopback embedding inference (Ollama), SQLite embedding cache persistence,
//! and fail-closed Sealed record exclusion.

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    time::Duration,
};

pub const MIN_DIMENSIONS: usize = 2;
pub const MAX_DIMENSIONS: usize = 1536;
pub const MAX_EMBEDDING_INPUT_BYTES: usize = 8192;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EmbeddingVector {
    pub values: Vec<f32>,
}

#[derive(Debug)]
pub enum VectorError {
    DimensionMismatch {
        expected: usize,
        actual: usize,
    },
    EmptyVector,
    InvalidDimensions {
        min: usize,
        max: usize,
        actual: usize,
    },
    NonFiniteValue,
    DeserializationError(String),
    Offline(String),
    InvalidEndpoint(String),
    ServiceError(String),
    InputTooLarge {
        actual: usize,
        max: usize,
    },
    SealedRecordRefused,
    StorageError(String),
}

impl std::fmt::Display for VectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "Dimension mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            Self::EmptyVector => write!(f, "Vector cannot be empty"),
            Self::InvalidDimensions { min, max, actual } => {
                write!(
                    f,
                    "Vector dimensions must be between {} and {}, got {}",
                    min, max, actual
                )
            }
            Self::NonFiniteValue => write!(
                f,
                "Vector contains non-finite float value (NaN or infinity)"
            ),
            Self::DeserializationError(msg) => write!(f, "Vector deserialization error: {}", msg),
            Self::Offline(msg) => write!(f, "Loopback embedding runtime is offline: {}", msg),
            Self::InvalidEndpoint(msg) => {
                write!(f, "Non-loopback embedding endpoint rejected: {}", msg)
            }
            Self::ServiceError(msg) => write!(f, "Embedding service error: {}", msg),
            Self::InputTooLarge { actual, max } => {
                write!(
                    f,
                    "Input text exceeds maximum allowed size ({} > {} bytes)",
                    actual, max
                )
            }
            Self::SealedRecordRefused => {
                write!(f, "Sealed memories are strictly excluded from embedding")
            }
            Self::StorageError(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}

impl std::error::Error for VectorError {}

impl EmbeddingVector {
    pub fn new(values: Vec<f32>) -> Result<Self, VectorError> {
        if values.is_empty() {
            return Err(VectorError::EmptyVector);
        }
        if values.len() < MIN_DIMENSIONS || values.len() > MAX_DIMENSIONS {
            return Err(VectorError::InvalidDimensions {
                min: MIN_DIMENSIONS,
                max: MAX_DIMENSIONS,
                actual: values.len(),
            });
        }
        for &v in &values {
            if !v.is_finite() {
                return Err(VectorError::NonFiniteValue);
            }
        }
        Ok(Self { values })
    }

    pub fn dimension(&self) -> usize {
        self.values.len()
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.values
    }

    pub fn l2_norm(&self) -> f32 {
        self.values.iter().map(|&x| x * x).sum::<f32>().sqrt()
    }

    pub fn normalize(&self) -> Result<Self, VectorError> {
        let norm = self.l2_norm();
        if norm <= f32::EPSILON {
            return Err(VectorError::NonFiniteValue);
        }
        let inv = 1.0 / norm;
        let normalized: Vec<f32> = self.values.iter().map(|&x| x * inv).collect();
        Self::new(normalized)
    }

    pub fn dot_product(&self, other: &Self) -> Result<f32, VectorError> {
        if self.dimension() != other.dimension() {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimension(),
                actual: other.dimension(),
            });
        }
        let sum: f32 = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(&a, &b)| a * b)
            .sum();
        Ok(sum)
    }

    pub fn cosine_similarity(&self, other: &Self) -> Result<f32, VectorError> {
        if self.dimension() != other.dimension() {
            return Err(VectorError::DimensionMismatch {
                expected: self.dimension(),
                actual: other.dimension(),
            });
        }
        let norm_a = self.l2_norm();
        let norm_b = other.l2_norm();
        if norm_a <= f32::EPSILON || norm_b <= f32::EPSILON {
            return Err(VectorError::NonFiniteValue);
        }
        let dot = self.dot_product(other)?;
        let sim = dot / (norm_a * norm_b);
        // Clamp to [-1.0, 1.0] to account for floating point inaccuracies
        Ok(sim.clamp(-1.0, 1.0))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.values.len() * 4);
        for &val in &self.values {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, VectorError> {
        if bytes.is_empty() {
            return Err(VectorError::EmptyVector);
        }
        if !bytes.len().is_multiple_of(4) {
            return Err(VectorError::DeserializationError(format!(
                "Byte length {} is not a multiple of 4",
                bytes.len()
            )));
        }
        let (chunks, _) = bytes.as_chunks::<4>();
        let mut values = Vec::with_capacity(chunks.len());
        for &chunk in chunks {
            values.push(f32::from_le_bytes(chunk));
        }
        Self::new(values)
    }
}

pub fn validate_loopback_endpoint(endpoint: &str) -> Result<(String, u16), VectorError> {
    let trimmed = endpoint.trim().trim_end_matches('/');
    let without_scheme = if let Some(stripped) = trimmed.strip_prefix("http://") {
        stripped
    } else if let Some(stripped) = trimmed.strip_prefix("https://") {
        stripped
    } else {
        trimmed
    };

    let host_port = without_scheme.split('/').next().unwrap_or(without_scheme);
    let mut parts = host_port.split(':');
    let host = parts.next().unwrap_or("");
    let port: u16 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(11434);

    let is_loopback = match host {
        "127.0.0.1" | "localhost" | "[::1]" => true,
        other => {
            if let Ok(ip) = other.parse::<std::net::IpAddr>() {
                ip.is_loopback()
            } else {
                false
            }
        }
    };

    if !is_loopback {
        return Err(VectorError::InvalidEndpoint(format!(
            "Endpoint must be loopback (127.0.0.1 or localhost), got: {}",
            endpoint
        )));
    }

    Ok((host.to_string(), port))
}

fn dechunk_http_body(mut bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let mut output = Vec::new();
    while let Some(end) = bytes.windows(2).position(|v| v == b"\r\n") {
        let size_str = std::str::from_utf8(&bytes[..end])
            .map_err(|_| ())?
            .split(';')
            .next()
            .ok_or(())?
            .trim();
        let size = usize::from_str_radix(size_str, 16).map_err(|_| ())?;
        bytes = &bytes[end + 2..];
        if size == 0 {
            break;
        }
        if size > 256 * 1024 {
            return Err(());
        }
        if bytes.len() < size + 2 {
            break;
        }
        if &bytes[size..size + 2] != b"\r\n" {
            return Err(());
        }
        output.extend_from_slice(&bytes[..size]);
        bytes = &bytes[size + 2..];
    }
    Ok(output)
}

pub fn generate_ollama_embedding(
    endpoint: &str,
    model: &str,
    text: &str,
) -> Result<EmbeddingVector, VectorError> {
    if text.len() > MAX_EMBEDDING_INPUT_BYTES {
        return Err(VectorError::InputTooLarge {
            actual: text.len(),
            max: MAX_EMBEDDING_INPUT_BYTES,
        });
    }

    let (host, port) = validate_loopback_endpoint(endpoint)?;

    let addr_str = format!("{}:{}", host, port);
    let socket_addrs: Vec<SocketAddr> = addr_str
        .to_socket_addrs()
        .map_err(|e| {
            VectorError::Offline(format!("DNS resolution failed for {}: {}", addr_str, e))
        })?
        .collect();

    let socket_addr = socket_addrs.first().ok_or_else(|| {
        VectorError::Offline("No socket addresses found for loopback".to_string())
    })?;

    let timeout = Duration::from_secs(10);
    let mut stream = TcpStream::connect_timeout(socket_addr, timeout).map_err(|e| {
        VectorError::Offline(format!(
            "Cannot connect to local Ollama server at {}: {}",
            addr_str, e
        ))
    })?;

    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| VectorError::Offline(e.to_string()))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| VectorError::Offline(e.to_string()))?;

    #[derive(Serialize)]
    struct OllamaEmbeddingRequest<'a> {
        model: &'a str,
        prompt: &'a str,
    }

    let req_body = serde_json::to_string(&OllamaEmbeddingRequest {
        model,
        prompt: text,
    })
    .map_err(|e| VectorError::ServiceError(e.to_string()))?;

    let http_request = format!(
        "POST /api/embeddings HTTP/1.1\r\n\
         Host: {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        host,
        req_body.len(),
        req_body
    );

    stream
        .write_all(http_request.as_bytes())
        .map_err(|e| VectorError::Offline(format!("Failed to send embedding request: {}", e)))?;

    let mut response_bytes = Vec::new();
    // Bound allocation before parsing even if a local server is malformed.
    (&mut stream)
        .take(256 * 1024 + 1)
        .read_to_end(&mut response_bytes)
        .map_err(|e| VectorError::Offline(format!("Failed to read embedding response: {}", e)))?;
    if response_bytes.len() > 256 * 1024 {
        return Err(VectorError::ServiceError(
            "Embedding response exceeds its limit".into(),
        ));
    }

    let header_end = response_bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| {
            VectorError::DeserializationError("Malformed HTTP response: no header delimiter".into())
        })?;

    let header_bytes = &response_bytes[..header_end];
    let header_str = String::from_utf8_lossy(header_bytes);
    if !header_str.contains("200 OK") {
        return Err(VectorError::ServiceError(format!(
            "Ollama server returned non-200 status: {}",
            header_str.lines().next().unwrap_or("Unknown error")
        )));
    }

    let raw_body = &response_bytes[header_end + 4..];
    let is_chunked = header_str
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked");
    let unchunked = if is_chunked {
        dechunk_http_body(raw_body).map_err(|_| {
            VectorError::DeserializationError("Failed to dechunk HTTP response".into())
        })?
    } else {
        raw_body.to_vec()
    };

    let body_str = String::from_utf8_lossy(&unchunked);

    #[derive(Deserialize)]
    struct OllamaEmbeddingResponse {
        embedding: Option<Vec<f32>>,
        error: Option<String>,
    }

    let parsed: OllamaEmbeddingResponse = serde_json::from_str(&body_str).map_err(|e| {
        VectorError::DeserializationError(format!("Malformed response JSON: {}", e))
    })?;

    if let Some(err) = parsed.error {
        return Err(VectorError::ServiceError(err));
    }

    let raw_embedding = parsed.embedding.ok_or_else(|| {
        VectorError::ServiceError("Response contained no embedding vector".to_string())
    })?;

    EmbeddingVector::new(raw_embedding)
}

pub fn ensure_embeddings_table(conn: &Connection) -> Result<(), VectorError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memory_embeddings (
            workspace_id TEXT NOT NULL,
            memory_id TEXT NOT NULL,
            revision_id TEXT NOT NULL DEFAULT '',
            content_hash TEXT NOT NULL DEFAULT '',
            chunk_index INTEGER NOT NULL DEFAULT 0,
            model TEXT NOT NULL,
            dimension INTEGER NOT NULL,
            embedding BLOB NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (workspace_id, memory_id, chunk_index, model)
        );
        CREATE INDEX IF NOT EXISTS idx_memory_embeddings_workspace_model
            ON memory_embeddings(workspace_id, model);",
    )
    .map_err(|e| VectorError::StorageError(e.to_string()))?;

    // Backwards compatibility for existing test/catalog tables
    let has_rev: bool = conn
        .query_row(
            "SELECT COUNT(1) FROM pragma_table_info('memory_embeddings') WHERE name = 'revision_id'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if !has_rev {
        let _ = conn.execute(
            "ALTER TABLE memory_embeddings ADD COLUMN revision_id TEXT NOT NULL DEFAULT ''",
            [],
        );
    }

    let has_hash: bool = conn
        .query_row(
            "SELECT COUNT(1) FROM pragma_table_info('memory_embeddings') WHERE name = 'content_hash'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if !has_hash {
        let _ = conn.execute(
            "ALTER TABLE memory_embeddings ADD COLUMN content_hash TEXT NOT NULL DEFAULT ''",
            [],
        );
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn store_embedding_with_revision(
    conn: &Connection,
    workspace_id: &str,
    memory_id: &str,
    revision_id: &str,
    content_hash: &str,
    chunk_index: usize,
    model: &str,
    embedding: &EmbeddingVector,
) -> Result<(), VectorError> {
    ensure_embeddings_table(conn)?;

    // Fail-closed privacy check: Verify memory is not Sealed using canonical revision metadata
    let privacy: Option<String> = conn
        .query_row(
            "SELECT json_extract(r.metadata, '$.privacy')
             FROM memory_revisions r
             JOIN memories m ON m.workspace_id = r.workspace_id AND m.id = r.memory_id AND m.current_revision = r.id
             WHERE m.workspace_id = ?1 AND m.id = ?2",
            params![workspace_id, memory_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| VectorError::StorageError(e.to_string()))?
        .flatten();

    if let Some(p) = privacy
        && p.eq_ignore_ascii_case("sealed")
    {
        return Err(VectorError::SealedRecordRefused);
    }

    let bytes = embedding.to_bytes();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO memory_embeddings (workspace_id, memory_id, revision_id, content_hash, chunk_index, model, dimension, embedding, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT (workspace_id, memory_id, chunk_index, model)
         DO UPDATE SET revision_id = excluded.revision_id, content_hash = excluded.content_hash, dimension = excluded.dimension, embedding = excluded.embedding, created_at = excluded.created_at",
        params![
            workspace_id,
            memory_id,
            revision_id,
            content_hash,
            chunk_index as i64,
            model,
            embedding.dimension() as i64,
            bytes,
            now
        ],
    )
    .map_err(|e| VectorError::StorageError(e.to_string()))?;

    Ok(())
}

pub fn store_embedding(
    conn: &Connection,
    workspace_id: &str,
    memory_id: &str,
    chunk_index: usize,
    model: &str,
    embedding: &EmbeddingVector,
) -> Result<(), VectorError> {
    store_embedding_with_revision(
        conn,
        workspace_id,
        memory_id,
        "",
        "",
        chunk_index,
        model,
        embedding,
    )
}

pub fn get_embedding(
    conn: &Connection,
    workspace_id: &str,
    memory_id: &str,
    chunk_index: usize,
    model: &str,
) -> Result<Option<EmbeddingVector>, VectorError> {
    ensure_embeddings_table(conn)?;

    let mut stmt = conn
        .prepare(
            "SELECT embedding FROM memory_embeddings
             WHERE workspace_id = ?1 AND memory_id = ?2 AND chunk_index = ?3 AND model = ?4",
        )
        .map_err(|e| VectorError::StorageError(e.to_string()))?;

    let result = stmt.query_row(
        params![workspace_id, memory_id, chunk_index as i64, model],
        |row| {
            let bytes: Vec<u8> = row.get(0)?;
            Ok(bytes)
        },
    );

    match result {
        Ok(bytes) => EmbeddingVector::from_bytes(&bytes).map(Some),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(VectorError::StorageError(e.to_string())),
    }
}

pub fn delete_embeddings_for_memory(
    conn: &Connection,
    workspace_id: &str,
    memory_id: &str,
) -> Result<(), VectorError> {
    ensure_embeddings_table(conn)?;
    conn.execute(
        "DELETE FROM memory_embeddings WHERE workspace_id = ?1 AND memory_id = ?2",
        params![workspace_id, memory_id],
    )
    .map_err(|e| VectorError::StorageError(e.to_string()))?;
    Ok(())
}

pub fn delete_embeddings_for_workspace(
    conn: &Connection,
    workspace_id: &str,
) -> Result<(), VectorError> {
    ensure_embeddings_table(conn)?;
    conn.execute(
        "DELETE FROM memory_embeddings WHERE workspace_id = ?1",
        params![workspace_id],
    )
    .map_err(|e| VectorError::StorageError(e.to_string()))?;
    Ok(())
}

pub fn prune_stale_embeddings_for_workspace(
    conn: &Connection,
    workspace_id: &str,
) -> Result<(), VectorError> {
    ensure_embeddings_table(conn)?;
    conn.execute(
        "DELETE FROM memory_embeddings
         WHERE workspace_id = ?1
           AND NOT EXISTS (
               SELECT 1 FROM memories m
               JOIN memory_revisions r ON r.workspace_id = m.workspace_id
                                      AND r.memory_id = m.id
                                      AND r.id = m.current_revision
               WHERE m.workspace_id = memory_embeddings.workspace_id
                 AND m.id = memory_embeddings.memory_id
                 AND (memory_embeddings.revision_id = '' OR memory_embeddings.revision_id = m.current_revision)
                 AND json_extract(r.metadata, '$.privacy') != 'sealed'
                 AND json_extract(r.metadata, '$.status') != 'archived'
           )",
        params![workspace_id],
    )
    .map_err(|e| VectorError::StorageError(e.to_string()))?;
    Ok(())
}

pub fn list_embeddings_for_workspace(
    conn: &Connection,
    workspace_id: &str,
    model: &str,
) -> Result<Vec<(String, usize, EmbeddingVector)>, VectorError> {
    ensure_embeddings_table(conn)?;

    let mut stmt = conn
        .prepare(
            "SELECT memory_id, chunk_index, embedding FROM memory_embeddings
             WHERE workspace_id = ?1 AND model = ?2
             ORDER BY memory_id, chunk_index",
        )
        .map_err(|e| VectorError::StorageError(e.to_string()))?;

    let rows = stmt
        .query_map(params![workspace_id, model], |row| {
            let memory_id: String = row.get(0)?;
            let chunk_index: i64 = row.get(1)?;
            let bytes: Vec<u8> = row.get(2)?;
            Ok((memory_id, chunk_index as usize, bytes))
        })
        .map_err(|e| VectorError::StorageError(e.to_string()))?;

    let mut results = Vec::new();
    for row in rows {
        let (memory_id, chunk_index, bytes) =
            row.map_err(|e| VectorError::StorageError(e.to_string()))?;
        let vector = EmbeddingVector::from_bytes(&bytes)?;
        results.push((memory_id, chunk_index, vector));
    }

    Ok(results)
}

pub use delete_embeddings_for_memory as invalidate_embeddings;
pub use delete_embeddings_for_workspace as invalidate_workspace_embeddings;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub memory_id: String,
    pub chunk_index: usize,
    pub model: String,
    pub score: f32,
}

pub fn vector_search(
    conn: &Connection,
    workspace_id: &str,
    query_vector: &EmbeddingVector,
    model: &str,
    limit: usize,
    min_score: Option<f32>,
) -> Result<Vec<VectorSearchResult>, VectorError> {
    ensure_embeddings_table(conn)?;

    let bounded_limit = limit.clamp(1, 100);

    let mut stmt = conn
        .prepare(
            "SELECT me.memory_id, me.chunk_index, me.embedding
             FROM memory_embeddings me
             JOIN memories m ON m.workspace_id = me.workspace_id AND m.id = me.memory_id
             JOIN memory_revisions r ON r.workspace_id = m.workspace_id AND r.memory_id = m.id AND r.id = m.current_revision
             WHERE me.workspace_id = ?1
               AND me.model = ?2
               AND me.dimension = ?3
               AND (me.revision_id = '' OR me.revision_id = m.current_revision)
               AND json_extract(r.metadata, '$.status') != 'archived'
               AND json_extract(r.metadata, '$.privacy') != 'sealed'",
        )
        .map_err(|e| VectorError::StorageError(e.to_string()))?;

    let rows = stmt
        .query_map(
            params![workspace_id, model, query_vector.dimension() as i64],
            |row| {
                let memory_id: String = row.get(0)?;
                let chunk_index: i64 = row.get(1)?;
                let bytes: Vec<u8> = row.get(2)?;
                Ok((memory_id, chunk_index as usize, bytes))
            },
        )
        .map_err(|e| VectorError::StorageError(e.to_string()))?;

    let mut results = Vec::new();
    for row in rows {
        let (memory_id, chunk_index, bytes) =
            row.map_err(|e| VectorError::StorageError(e.to_string()))?;
        let candidate_vector = EmbeddingVector::from_bytes(&bytes)?;
        let sim = query_vector.cosine_similarity(&candidate_vector)?;

        if let Some(min) = min_score
            && sim < min
        {
            continue;
        }

        results.push(VectorSearchResult {
            memory_id,
            chunk_index,
            model: model.to_string(),
            score: sim,
        });
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.memory_id.cmp(&b.memory_id))
    });

    if results.len() > bounded_limit {
        results.truncate(bounded_limit);
    }

    Ok(results)
}
