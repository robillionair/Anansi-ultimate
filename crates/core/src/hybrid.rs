//! Hybrid retrieval combining SQLite FTS5 lexical BM25 ranking and vector cosine ranking
//! using Reciprocal Rank Fusion (RRF).
//! Fail-closed Sealed memory exclusion and metadata/kind/archive filtering are preserved.

use crate::search::{ensure_fts_table, locate_evidence, parse_body_query};
use crate::vector::{EmbeddingVector, vector_search};
use crate::{CommandError, ErrorCode};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

fn default_limit() -> usize {
    10
}
fn default_weight() -> f32 {
    1.0
}
fn default_rrf_k() -> f32 {
    60.0
}

use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HybridSearchRequest {
    pub workspace_id: String,
    pub query: Option<String>,
    pub query_vector: Option<EmbeddingVector>,
    pub model: Option<String>,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HybridSearchResult {
    pub memory_id: String,
    pub title: String,
    pub kind: String,
    pub rrf_score: f32,
    pub lexical_rank: Option<usize>,
    pub semantic_rank: Option<usize>,
    pub snippet: Option<String>,
    pub section_heading: Option<String>,
    pub line_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HybridSearchResponse {
    pub contract_version: u8,
    pub results: Vec<HybridSearchResult>,
}

struct LexicalHit {
    rank: usize,
    snippet: Option<String>,
    section_heading: Option<String>,
    line_number: Option<u32>,
}

pub fn hybrid_search(
    conn: &Connection,
    request: &HybridSearchRequest,
) -> Result<Vec<HybridSearchResult>, CommandError> {
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

    let mut lexical_hits: HashMap<String, LexicalHit> = HashMap::new();
    let mut semantic_hits: HashMap<String, usize> = HashMap::new();
    let mut candidate_ids: HashSet<String> = HashSet::new();

    // 1. Lexical retrieval via SQLite FTS5 (if query string provided)
    if let Some(query_str) = &request.query {
        let trimmed = query_str.trim();
        if !trimmed.is_empty() {
            let (expression, terms) = parse_body_query(trimmed)?;
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
                let (heading, line_num, snippet) = locate_evidence(&body, &terms);

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
        }
    }

    // 2. Semantic retrieval via vector_search
    let query_vector = if let Some(q_vec) = &request.query_vector {
        Some(q_vec.clone())
    } else if let (Some(model), Some(query_str)) = (&request.model, &request.query) {
        match crate::semantic::generate_query_vector(None, model, query_str) {
            Ok(vec) => Some(vec),
            Err(e) => {
                if request.lexical_weight == 0.0 {
                    return Err(CommandError::new(
                        ErrorCode::StorageUnavailable,
                        &format!("Semantic query vector generation failed: {e}"),
                    ));
                }
                // When lexical weight > 0, fallback to lexical-only without claiming semantic match
                None
            }
        }
    } else {
        None
    };

    if let (Some(q_vec), Some(model)) = (&query_vector, &request.model) {
        let vec_results = vector_search(conn, &request.workspace_id, q_vec, model, 100, None)
            .map_err(|_| crate::storage::unavailable())?;
        for (current_rank, hit) in (1usize..).zip(vec_results) {
            candidate_ids.insert(hit.memory_id.clone());
            semantic_hits.insert(hit.memory_id, current_rank);
        }
    }

    // 3. Fusion & Filter pass
    let rrf_k = request.rrf_k.max(1.0);
    let mut results: Vec<HybridSearchResult> = Vec::new();

    for memory_id in candidate_ids {
        // Query memory metadata and verify privacy & archive boundaries
        let meta_res = conn.query_row(
            "SELECT r.metadata
             FROM memory_revisions r
             JOIN memories m ON m.workspace_id = r.workspace_id AND m.id = r.memory_id AND m.current_revision = r.id
             WHERE m.workspace_id = ?1 AND m.id = ?2",
            params![request.workspace_id, memory_id],
            |row| row.get::<_, String>(0),
        );

        let json_str = meta_res.map_err(|_| crate::storage::unavailable())?;
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

        // Kind filter
        if let Some(filter_kind) = &request.kind
            && !kind.eq_ignore_ascii_case(filter_kind)
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
        if let Some(&s_rank) = sem {
            rrf_score += request.semantic_weight / (rrf_k + s_rank as f32);
            semantic_rank = Some(s_rank);
        }

        results.push(HybridSearchResult {
            memory_id,
            title,
            kind,
            rrf_score,
            lexical_rank,
            semantic_rank,
            snippet,
            section_heading,
            line_number,
        });
    }

    // 4. Stable descending sort by RRF score, breaking ties by memory_id
    results.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.memory_id.cmp(&b.memory_id))
    });

    let bounded_limit = request.limit.clamp(1, 100);
    if results.len() > bounded_limit {
        results.truncate(bounded_limit);
    }

    Ok(results)
}
