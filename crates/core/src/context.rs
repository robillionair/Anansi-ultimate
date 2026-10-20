//! Complete multi-chunk, section-aware Context Compiler.
//! Bounded token budgeting, Markdown section preservation, grounded citations,
//! deduplication, fail-closed Sealed exclusion, and full Data Disclosure Manifest.

use crate::CommandError;
use crate::ask::{DisclosureManifest, GroundedCitation, SourceMemoryReference};
use crate::content;
use crate::hybrid::{HybridSearchRequest, hybrid_search};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use schemars::JsonSchema;

pub const DEFAULT_MAX_BUDGET_CHARS: usize = 4_000;
pub const MAX_ALLOWED_BUDGET_CHARS: usize = 32_000;
pub const MAX_CHUNK_CHARS: usize = 1_000;

fn utf8_prefix(text: &str, max_bytes: usize) -> &str {
    let mut end = max_bytes.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextChunk {
    pub memory_id: String,
    pub chunk_index: usize,
    pub title: String,
    pub kind: String,
    pub section_heading: Option<String>,
    pub line_number: Option<u32>,
    pub content: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompiledContext {
    pub contract_version: u8,
    pub evidence_chunks: Vec<ContextChunk>,
    pub citations: Vec<GroundedCitation>,
    pub disclosure_manifest: DisclosureManifest,
    pub system_prompt: String,
    pub total_characters: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextCompilationRequest {
    pub workspace_id: String,
    pub user_prompt: String,
    pub max_budget_chars: Option<usize>,
    pub runtime_endpoint: Option<String>,
    pub model_tag: Option<String>,
    pub kind_filter: Option<String>,
}

pub fn chunk_markdown_sections(
    memory_id: &str,
    title: &str,
    kind: &str,
    body: &str,
) -> Vec<ContextChunk> {
    // Preserve every section byte beyond the first bounded chunk. Source line
    // offsets follow consumed newlines; splits inside a line retain that line.
    let mut complete = Vec::new();
    let mut heading: Option<String> = None;
    let mut section = String::new();
    let mut start_line = 1u32;
    let append =
        |text: &str, heading: &Option<String>, line: u32, output: &mut Vec<ContextChunk>| {
            let mut rest = text.trim_end_matches('\n');
            let mut source_line = line;
            while !rest.is_empty() {
                let piece = utf8_prefix(rest, MAX_CHUNK_CHARS);
                output.push(ContextChunk {
                    memory_id: memory_id.into(),
                    chunk_index: output.len(),
                    title: title.into(),
                    kind: kind.into(),
                    section_heading: heading.clone(),
                    line_number: Some(source_line),
                    content: piece.into(),
                    token_estimate: piece.len().div_ceil(4).max(1),
                });
                source_line += piece.bytes().filter(|byte| *byte == b'\n').count() as u32;
                rest = &rest[piece.len()..];
            }
        };
    for (index, line) in body.lines().enumerate() {
        if line.trim_start().starts_with('#') {
            append(&section, &heading, start_line, &mut complete);
            section.clear();
            heading = Some(line.trim().into());
            start_line = index as u32 + 2;
        } else {
            section.push_str(line);
            section.push('\n');
        }
    }
    append(&section, &heading, start_line, &mut complete);
    complete
}

pub fn compile_context(
    conn: &Connection,
    storage_root: &Path,
    req: &ContextCompilationRequest,
) -> Result<CompiledContext, CommandError> {
    let budget = req
        .max_budget_chars
        .unwrap_or(DEFAULT_MAX_BUDGET_CHARS)
        .clamp(500, MAX_ALLOWED_BUDGET_CHARS);

    // 1. Retrieve candidate records via hybrid retrieval
    let hybrid_req = HybridSearchRequest {
        workspace_id: req.workspace_id.clone(),
        query: Some(req.user_prompt.clone()),
        query_vector: None,
        model: req.model_tag.clone(),
        limit: 10,
        lexical_weight: 1.0,
        semantic_weight: 1.0,
        rrf_k: 60.0,
        kind: req.kind_filter.clone(),
        include_archived: false,
    };

    let hits = hybrid_search(conn, &hybrid_req)?;

    let mut evidence_chunks: Vec<ContextChunk> = Vec::new();
    let mut citations: Vec<GroundedCitation> = Vec::new();
    let mut source_memories: Vec<SourceMemoryReference> = Vec::new();
    let mut seen_memory_ids: HashSet<String> = HashSet::new();
    let mut seen_chunk_contents: HashSet<String> = HashSet::new();

    let mut current_budget_used = 0usize;

    for hit in hits {
        // Fail-closed privacy check: query record metadata
        let row_res = conn.query_row(
            "SELECT m.current_revision, json_extract(r.metadata, '$.privacy')
             FROM memories m
             JOIN memory_revisions r ON r.workspace_id = m.workspace_id AND r.memory_id = m.id AND r.id = m.current_revision
             WHERE m.workspace_id = ?1 AND m.id = ?2",
            rusqlite::params![req.workspace_id, hit.memory_id],
            |r| {
                let rev: String = r.get(0)?;
                let priv_str: String = r.get(1)?;
                Ok((rev, priv_str))
            },
        );

        let (revision_id, privacy_str) = row_res.map_err(|_| crate::storage::unavailable())?;

        if privacy_str.eq_ignore_ascii_case("sealed") {
            continue; // Fail-closed Sealed memory exclusion
        }

        let rev_row = conn.query_row(
            "SELECT body_hash FROM memory_revisions WHERE workspace_id = ?1 AND memory_id = ?2 AND id = ?3",
            rusqlite::params![req.workspace_id, hit.memory_id, revision_id],
            |r| r.get::<_, String>(0),
        );

        let body_hash = rev_row.map_err(|_| crate::storage::unavailable())?;

        let body = content::read(
            storage_root,
            &req.workspace_id,
            &hit.memory_id,
            &revision_id,
            &body_hash,
        )?;

        if !seen_memory_ids.contains(&hit.memory_id) {
            seen_memory_ids.insert(hit.memory_id.clone());
            source_memories.push(SourceMemoryReference {
                id: hit.memory_id.clone(),
                title: hit.title.clone(),
                kind: hit.kind.clone(),
            });
        }

        if current_budget_used >= budget {
            break;
        }

        // Chunk sections
        let sections = chunk_markdown_sections(&hit.memory_id, &hit.title, &hit.kind, &body);
        for mut sec in sections {
            if current_budget_used >= budget {
                break;
            }

            // Deduplicate chunks
            if seen_chunk_contents.contains(&sec.content) {
                continue;
            }

            let chunk_len = sec.content.len();
            let remaining = budget - current_budget_used;

            if chunk_len > remaining {
                if remaining < 50 {
                    // Not enough budget left for a meaningful chunk
                    break;
                }
                // Truncate cleanly to remaining budget
                let cutoff = remaining.saturating_sub(3);
                sec.content = format!("{}...", utf8_prefix(&sec.content, cutoff));
            }

            let actual_len = sec.content.len();
            seen_chunk_contents.insert(sec.content.clone());
            current_budget_used += actual_len;

            let citation_idx = citations.len() + 1;
            citations.push(GroundedCitation {
                index: citation_idx,
                memory_id: sec.memory_id.clone(),
                title: sec.title.clone(),
                kind: sec.kind.clone(),
                section_heading: sec.section_heading.clone(),
                line_number: sec.line_number,
                snippet: if sec.content.len() > 200 {
                    format!("{}...", utf8_prefix(&sec.content, 200))
                } else {
                    sec.content.clone()
                },
            });

            evidence_chunks.push(sec);
        }
    }

    // Assemble system prompt with citations
    let mut prompt_builder = String::from(
        "You are ANANSI, an intelligent, offline-first operating system assistant.\n\
         Answer the user's question accurately using ONLY the grounded workspace evidence provided below.\n\
         Cite every factual claim using [index] corresponding to the exact citation number.\n\
         If the evidence is insufficient to answer the question, state truthfully that evidence is missing.\n\n\
         --- GROUNDED EVIDENCE PACK ---\n",
    );

    for (idx, chunk) in evidence_chunks.iter().enumerate() {
        let heading_desc = chunk
            .section_heading
            .as_deref()
            .unwrap_or("General Content");
        let line_desc = chunk
            .line_number
            .map(|l| format!("(line {})", l))
            .unwrap_or_default();
        prompt_builder.push_str(&format!(
            "[{}] {} - {} {}\n{}\n\n",
            idx + 1,
            chunk.title,
            heading_desc,
            line_desc,
            chunk.content
        ));
    }

    prompt_builder.push_str(&format!("User Question: {}\nAnswer:", req.user_prompt));

    let total_chars = prompt_builder.len();
    let total_tokens = (total_chars / 4).max(1);

    let manifest = DisclosureManifest {
        runtime_kind: "localLoopback".to_string(),
        endpoint: req
            .runtime_endpoint
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:11434".to_string()),
        model_tag: req
            .model_tag
            .clone()
            .unwrap_or_else(|| "llama3.1:8b-instruct-q4_K_M".to_string()),
        source_memories,
        character_count: total_chars,
        estimated_tokens: total_tokens,
        routing_reason: "local section-aware Context Compiler".to_string(),
        compiled_prompt: prompt_builder.clone(),
    };

    Ok(CompiledContext {
        contract_version: 1,
        evidence_chunks,
        citations,
        disclosure_manifest: manifest,
        system_prompt: prompt_builder,
        total_characters: total_chars,
        total_tokens,
    })
}
