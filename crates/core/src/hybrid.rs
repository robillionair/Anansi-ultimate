//! Hybrid retrieval combining SQLite FTS5 lexical BM25 ranking, vector cosine ranking
//! using Reciprocal Rank Fusion (RRF), and a distinct post-fusion reranking stage
//! accounting for temporal recency, epistemic lifecycle status, and confidence.
//! Fail-closed Sealed memory exclusion and metadata/kind/archive/expiry filtering are preserved.

use crate::content;
use crate::context::chunk_markdown_sections;
use crate::search::{ensure_fts_table, locate_evidence, parse_body_query};
use crate::vector::{EmbeddingVector, ensure_embeddings_table, vector_search};
use crate::{CommandError, ErrorCode};
use chrono::Utc;
use rusqlite::{Connection, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

fn default_limit() -> usize {
    10
}
fn default_weight() -> f32 {
    1.0
}
fn default_rrf_k() -> f32 {
    60.0
}

pub const DEFAULT_SEMANTIC_MIN_SCORE: f32 = 0.25;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HybridSearchRequest {
    pub workspace_id: String,
    pub query: Option<String>,
    pub query_vector: Option<EmbeddingVector>,
    pub model: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_weight")]
    pub lexical_weight: f32,
    #[serde(default = "default_weight")]
    pub semantic_weight: f32,
    #[serde(default = "default_rrf_k")]
    pub rrf_k: f32,
    pub kind: Option<String>,
    #[serde(default)]
    pub include_archived: bool,
    #[serde(default)]
    pub include_expired: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HybridSearchResult {
    pub memory_id: String,
    pub title: String,
    pub kind: String,
    pub rrf_score: f32,
    #[serde(default)]
    pub rerank_score: Option<f32>,
    pub lexical_rank: Option<usize>,
    pub semantic_rank: Option<usize>,
    #[serde(default)]
    pub cosine_similarity: Option<f32>,
    #[serde(default)]
    pub rerank_provenance: Option<String>,
    pub snippet: Option<String>,
    pub section_heading: Option<String>,
    pub line_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HybridSearchResponse {
    pub contract_version: u8,
    pub results: Vec<HybridSearchResult>,
    #[serde(default)]
    pub lexical_only: bool,
    #[serde(default)]
    pub fallback_reason: Option<String>,
    #[serde(default)]
    pub embedding_model: Option<String>,
}

struct LexicalHit {
    rank: usize,
    snippet: Option<String>,
    section_heading: Option<String>,
    line_number: Option<u32>,
}

struct SemanticHit {
    rank: usize,
    chunk_index: usize,
    score: f32,
}

fn calculate_recency_multiplier(updated_at: Option<&str>, event_date: Option<&str>) -> f32 {
    let now = Utc::now();
    let parsed = event_date
        .and_then(|ed| chrono::DateTime::parse_from_rfc3339(ed).ok())
        .or_else(|| updated_at.and_then(|ua| chrono::DateTime::parse_from_rfc3339(ua).ok()))
        .map(|dt| dt.with_timezone(&Utc));

    let Some(target_time) = parsed else {
        return 1.0;
    };
    let days_diff = (now - target_time).num_days().max(0) as f32;
    1.0 / (1.0 + 0.005 * days_diff)
}

fn calculate_lifecycle_multiplier(status: &str, validation_status: Option<&str>) -> f32 {
    let base = match status {
        "superseded" | "contradicted" => 0.35,
        "archived" => 0.25,
        "validated" => 1.10,
        _ => 1.0,
    };
    let val_adj = match validation_status {
        Some("verified") => 1.05,
        Some("contested") => 0.75,
        _ => 1.0,
    };
    base * val_adj
}

pub fn hybrid_search(
    conn: &Connection,
    request: &HybridSearchRequest,
) -> Result<Vec<HybridSearchResult>, CommandError> {
    let response = hybrid_search_with_status(conn, None, request)?;
    Ok(response.results)
}

pub fn hybrid_search_with_status(
    conn: &Connection,
    storage_root: Option<&Path>,
    request: &HybridSearchRequest,
) -> Result<HybridSearchResponse, CommandError> {
    if !request.lexical_weight.is_finite()
        || !request.semantic_weight.is_finite()
        || !request.rrf_k.is_finite()
        || request.lexical_weight < 0.0
        || request.semantic_weight < 0.0
        || request.rrf_k < 1.0
    {
        return Err(CommandError::new(
            ErrorCode::InvalidSearch,
            "Invalid retrieval weights or rank constant.",
        ));
    }
    ensure_fts_table(conn)?;
    ensure_embeddings_table(conn)
        .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

    let mut lexical_hits: HashMap<String, LexicalHit> = HashMap::new();
    let mut semantic_hits: HashMap<String, SemanticHit> = HashMap::new();
    let mut candidate_ids: HashSet<String> = HashSet::new();

    // 1. Lexical retrieval via SQLite FTS5 (if query string provided and lexical_weight > 0)
    if let Some(query_str) = &request.query {
        let trimmed = query_str.trim();
        if !trimmed.is_empty() && request.lexical_weight > 0.0 {
            let (expression, parsed_terms) = parse_body_query(trimmed)?;
            let mut stmt = conn
                .prepare(
                    "SELECT memory_id, body
                     FROM memory_body_fts
                     WHERE workspace_id = ?1 AND memory_body_fts MATCH ?2
                     ORDER BY bm25(memory_body_fts), memory_id
                     LIMIT 100",
                )
                .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

            let rows = stmt
                .query_map(params![request.workspace_id, expression], |row| {
                    let memory_id: String = row.get(0)?;
                    let body: String = row.get(1)?;
                    Ok((memory_id, body))
                })
                .map_err(|e| CommandError::new(ErrorCode::StorageUnavailable, &e.to_string()))?;

            for (current_rank, row) in (1usize..).zip(rows) {
                let (memory_id, body) = row.map_err(|_| crate::storage::unavailable())?;
                let (heading, line_num, snippet) = locate_evidence(&body, &parsed_terms);

                candidate_ids.insert(memory_id.clone());
                lexical_hits.insert(
                    memory_id,
                    LexicalHit {
                        rank: current_rank,
                        snippet: Some(snippet),
                        section_heading: heading,
                        line_number: line_num,
                    },
                );
            }
        } else if !trimmed.is_empty() {
            // Still validate query bounds even when lexical_weight == 0.0
            let _ = parse_body_query(trimmed)?;
        }
    }

    // 2. Semantic retrieval via vector_search
    let clean_model = request
        .model
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_owned);

    let mut lexical_only = false;
    let mut fallback_reason: Option<String> = None;

    let query_vector: Option<EmbeddingVector> = if request.semantic_weight == 0.0 {
        lexical_only = true;
        fallback_reason = Some("Semantic weight is 0; showing lexical FTS5 results only.".into());
        None
    } else if let Some(q_vec) = &request.query_vector {
        Some(q_vec.clone())
    } else if let (Some(model), Some(query_str)) = (&clean_model, &request.query) {
        if query_str.trim().is_empty() {
            lexical_only = true;
            fallback_reason = Some("Empty query; showing lexical results only.".into());
            None
        } else {
            // Check whether workspace has any indexed embeddings for this model first
            let indexed_count: i64 = conn
                .query_row(
                    "SELECT COUNT(1) FROM memory_embeddings me
                     JOIN memories m ON m.workspace_id = me.workspace_id AND m.id = me.memory_id
                     JOIN memory_revisions r ON r.workspace_id = m.workspace_id AND r.memory_id = m.id AND r.id = m.current_revision
                     WHERE me.workspace_id = ?1
                       AND me.model = ?2
                       AND (me.revision_id = '' OR me.revision_id = m.current_revision)
                       AND json_extract(r.metadata, '$.status') != 'archived'
                       AND json_extract(r.metadata, '$.privacy') != 'sealed'",
                    params![request.workspace_id, model],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            if indexed_count == 0 {
                lexical_only = true;
                fallback_reason = Some(format!(
                    "No workspace embeddings are indexed for model '{model}'. Index workspace embeddings to enable semantic matching; showing lexical FTS5 results only."
                ));
                None
            } else {
                match crate::semantic::generate_query_vector(
                    request.endpoint.as_deref(),
                    model,
                    query_str,
                ) {
                    Ok(vec) => Some(vec),
                    Err(e) => {
                        if request.lexical_weight == 0.0 {
                            return Err(CommandError::new(
                                ErrorCode::StorageUnavailable,
                                &format!("Semantic query vector generation failed: {e}"),
                            ));
                        }
                        lexical_only = true;
                        fallback_reason = Some(format!(
                            "Local embedding runtime unavailable for model '{model}' ({e}); showing lexical FTS5 results only."
                        ));
                        None
                    }
                }
            }
        }
    } else {
        lexical_only = true;
        fallback_reason =
            Some("No embedding model selected; showing lexical FTS5 results only.".into());
        None
    };

    if let (Some(q_vec), Some(model)) = (&query_vector, &clean_model) {
        let min_score = if request.query_vector.is_some() {
            Some(0.01)
        } else {
            Some(DEFAULT_SEMANTIC_MIN_SCORE)
        };
        let vec_results = vector_search(conn, &request.workspace_id, q_vec, model, 100, min_score)
            .map_err(|_| crate::storage::unavailable())?;
        for hit in vec_results {
            candidate_ids.insert(hit.memory_id.clone());
            if !semantic_hits.contains_key(&hit.memory_id) {
                let next_rank = semantic_hits.len() + 1;
                semantic_hits.insert(
                    hit.memory_id,
                    SemanticHit {
                        rank: next_rank,
                        chunk_index: hit.chunk_index,
                        score: hit.score,
                    },
                );
            }
        }
    }

    // 3. Fusion, Filter, and Distinct Post-Fusion Reranking pass
    let rrf_k = request.rrf_k.max(1.0);
    let now_rfc3339 = Utc::now().to_rfc3339();
    let mut results: Vec<HybridSearchResult> = Vec::new();

    for memory_id in candidate_ids {
        let meta_res = conn.query_row(
            "SELECT r.id, r.metadata,
                    CASE WHEN EXISTS (
                        SELECT 1 FROM pragma_table_info('memory_revisions') WHERE name = 'body_hash'
                    ) THEN r.body_hash ELSE '' END
             FROM memory_revisions r
             JOIN memories m ON m.workspace_id = r.workspace_id AND m.id = r.memory_id AND m.current_revision = r.id
             WHERE m.workspace_id = ?1 AND m.id = ?2",
            params![request.workspace_id, memory_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2).unwrap_or_default(),
                ))
            },
        );

        let (revision_id, json_str, body_hash) =
            meta_res.map_err(|_| crate::storage::unavailable())?;
        let val: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|_| crate::storage::unavailable())?;
        let field = |key: &str| {
            val[key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(crate::storage::unavailable)
        };
        let title = field("title")?;
        let kind = field("kind")?;
        let status = field("status")?;
        let privacy = field("privacy")?;
        if !["private", "standard", "public", "sealed"].contains(&privacy.as_str()) {
            return Err(crate::storage::unavailable());
        }
        let is_archived = status == "archived";

        // Fail-closed Sealed memory exclusion
        if privacy.eq_ignore_ascii_case("sealed") {
            continue;
        }

        // Archive boundary
        if is_archived && !request.include_archived {
            continue;
        }

        // Temporal validity boundary (exclude expired records unless include_expired is true)
        if !request.include_expired
            && let Some(valid_until) = val["validUntil"].as_str()
            && let (Ok(until_dt), Ok(now_dt)) = (
                chrono::DateTime::parse_from_rfc3339(valid_until),
                chrono::DateTime::parse_from_rfc3339(&now_rfc3339),
            )
            && until_dt <= now_dt
        {
            continue;
        }

        // Kind filter
        if let Some(filter_kind) = &request.kind
            && !filter_kind.trim().is_empty()
            && !kind.eq_ignore_ascii_case(filter_kind.trim())
        {
            continue;
        }

        let lex = lexical_hits.get(&memory_id);
        let sem = semantic_hits.get(&memory_id);

        let mut rrf_score = 0.0f32;
        let mut lexical_rank = None;
        let mut snippet = None;
        let mut section_heading = None;
        let mut line_number = None;

        if let Some(l) = lex {
            rrf_score += request.lexical_weight / (rrf_k + l.rank as f32);
            lexical_rank = Some(l.rank);
            snippet.clone_from(&l.snippet);
            section_heading.clone_from(&l.section_heading);
            line_number = l.line_number;
        }

        let mut semantic_rank = None;
        let mut cosine_similarity = None;
        if let Some(s) = sem {
            rrf_score += request.semantic_weight / (rrf_k + s.rank as f32);
            semantic_rank = Some(s.rank);
            cosine_similarity = Some(s.score);

            // If this hit came only from semantic retrieval (e.g. paraphrase with 0 keyword overlap),
            // populate section_heading, line_number, and snippet from the matched semantic chunk!
            if snippet.is_none() {
                let body_opt = if let Some(root) = storage_root
                    && !body_hash.is_empty()
                {
                    content::read(
                        root,
                        &request.workspace_id,
                        &memory_id,
                        &revision_id,
                        &body_hash,
                    )
                    .ok()
                } else {
                    conn.query_row(
                        "SELECT body FROM memory_body_fts WHERE workspace_id = ?1 AND memory_id = ?2",
                        params![request.workspace_id, memory_id],
                        |r| r.get::<_, String>(0),
                    )
                    .ok()
                };

                if let Some(body_text) = body_opt {
                    let chunks = chunk_markdown_sections(&memory_id, &title, &kind, &body_text);
                    if let Some(chunk) = chunks.get(s.chunk_index).or_else(|| chunks.first()) {
                        section_heading = chunk.section_heading.clone();
                        line_number = chunk.line_number;
                        let chars: Vec<char> = chunk.content.chars().collect();
                        snippet = Some(if chars.len() > 160 {
                            chars[..160].iter().collect::<String>() + "..."
                        } else {
                            chunk.content.clone()
                        });
                    }
                }
            }
        }

        // Distinct Post-Fusion Reranking Stage:
        // Combines RRF score with temporal recency, epistemic lifecycle status, confidence, and cosine boost
        let recency_mult =
            calculate_recency_multiplier(val["updatedAt"].as_str(), val["eventDate"].as_str());
        let lifecycle_mult =
            calculate_lifecycle_multiplier(&status, val["validationStatus"].as_str());
        let confidence_val = val["confidence"]
            .as_f64()
            .map(|c| (c as f32).clamp(0.0, 1.0));
        let confidence_mult = confidence_val.map(|c| 0.5 + 0.5 * c).unwrap_or(1.0);
        let sim_mult = cosine_similarity
            .map(|sim| 1.0 + 0.15 * sim.clamp(0.0, 1.0))
            .unwrap_or(1.0);

        let rerank_score = rrf_score * recency_mult * lifecycle_mult * confidence_mult * sim_mult;
        let rerank_provenance = format!(
            "rrf={:.4} × recency={:.3} × lifecycle({status})={:.2} × confidence={:.2} × sim_boost={:.3}",
            rrf_score, recency_mult, lifecycle_mult, confidence_mult, sim_mult
        );

        results.push(HybridSearchResult {
            memory_id,
            title,
            kind,
            rrf_score,
            rerank_score: Some(rerank_score),
            lexical_rank,
            semantic_rank,
            cosine_similarity,
            rerank_provenance: Some(rerank_provenance),
            snippet,
            section_heading,
            line_number,
        });
    }

    // 4. Stable descending sort by post-fusion rerank_score, then rrf_score, breaking ties by memory_id
    results.sort_by(|a, b| {
        b.rerank_score
            .partial_cmp(&a.rerank_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.rrf_score
                    .partial_cmp(&a.rrf_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.memory_id.cmp(&b.memory_id))
    });

    let bounded_limit = request.limit.clamp(1, 100);
    if results.len() > bounded_limit {
        results.truncate(bounded_limit);
    }

    Ok(HybridSearchResponse {
        contract_version: 1,
        results,
        lexical_only,
        fallback_reason,
        embedding_model: if lexical_only { None } else { clean_model },
    })
}
