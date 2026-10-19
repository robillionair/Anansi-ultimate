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
    ensure_fts_table(conn)?;

    let mut lexical_hits: HashMap<String, LexicalHit> = HashMap::new();
    let mut semantic_hits: HashMap<String, usize> = HashMap::new();
    let mut candidate_ids: HashSet<String> = HashSet::new();

    // 1. Lexical retrieval via SQLite FTS5 (if query string provided)
    if let Some(query_str) = &request.query {
        let trimmed = query_str.trim();
        if !trimmed.is_empty()
            && let Ok((expression, terms)) = parse_body_query(trimmed)
        {
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

            for (current_rank, row) in (1usize..).zip(rows.flatten()) {
                let (memory_id, body) = row;
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

    // 2. Semantic retrieval via vector_search (if query vector and model provided)
    if let (Some(q_vec), Some(model)) = (&request.query_vector, &request.model)
        && let Ok(vec_results) = vector_search(conn, &request.workspace_id, q_vec, model, 100, None)
    {
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

        let (title, kind, is_archived, privacy) = if let Ok(json_str) = meta_res {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let title = val["title"].as_str().unwrap_or("").to_string();
                let kind = val["kind"].as_str().unwrap_or("note").to_string();
                let is_archived = val["status"].as_str() == Some("archived");
                let privacy = val["privacy"].as_str().unwrap_or("standard").to_string();
                (title, kind, is_archived, privacy)
            } else {
                continue;
            }
        } else {
            let fallback = conn.query_row(
                "SELECT title, kind, archived, privacy
                 FROM memories
                 WHERE workspace_id = ?1 AND id = ?2",
                params![request.workspace_id, memory_id],
                |row| {
                    let title: String = row.get(0)?;
                    let kind: String = row.get(1)?;
                    let archived: i64 = row.get(2)?;
                    let privacy: String = row.get(3)?;
                    Ok((title, kind, archived != 0, privacy))
                },
            );
            match fallback {
                Ok(data) => data,
                Err(_) => continue,
            }
        };

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
