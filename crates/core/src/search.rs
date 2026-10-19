//! Bounded, rebuildable full-body and title retrieval.
//! Canonical catalogs and Markdown bodies remain authoritative; the FTS5 index is rebuildable.
//! Sealed memories are strictly excluded from FTS5 tables to preserve the privacy boundary.

use crate::{
    CommandError, ErrorCode, content,
    memory::{MemoryList, MemoryListRequest, MemoryMetadata},
    privacy::Privacy,
    storage::{Storage, unavailable},
    vault::policy::SearchBoundary,
};
use chrono::Utc;
use rusqlite::{Connection, Transaction, params};

const MAX_RECORDS: usize = 10_000;
const MAX_METADATA: usize = 32 * 1024 * 1024;
const MAX_BODY_QUERY_BYTES: usize = 512;
const MAX_BODY_QUERY_TERMS: usize = 16;

pub(crate) fn invalid() -> CommandError {
    CommandError::new(
        ErrorCode::InvalidSearch,
        "Search titles using one to eight words, at most 256 UTF-8 bytes. History search is unavailable.",
    )
}

pub(crate) fn invalid_body_search() -> CommandError {
    CommandError::new(
        ErrorCode::InvalidSearch,
        "Search using 1 to 16 terms or exact phrases in quotes, up to 512 UTF-8 bytes.",
    )
}

pub(crate) fn query(text: &str) -> Result<String, CommandError> {
    if text.len() > 256 {
        return Err(invalid());
    }
    let words: Vec<_> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .take(9)
        .collect();
    if words.is_empty() || words.len() > 8 {
        return Err(invalid());
    }
    Ok(words
        .iter()
        .map(|word| format!("\"{word}\""))
        .collect::<Vec<_>>()
        .join(" AND "))
}

pub(crate) fn validate_body_query(text: &str) -> Result<(), CommandError> {
    parse_body_query(text).map(|_| ())
}

pub(crate) fn parse_body_query(text: &str) -> Result<(String, Vec<String>), CommandError> {
    if text.len() > MAX_BODY_QUERY_BYTES {
        return Err(invalid_body_search());
    }
    let mut tokens = Vec::new();
    let mut literal_terms = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c == '"' {
            chars.next();
            let mut phrase = String::new();
            while let Some(&p) = chars.peek() {
                chars.next();
                if p == '"' {
                    break;
                }
                phrase.push(p);
            }
            let phrase_words: Vec<_> = phrase
                .split(|ch: char| !ch.is_alphanumeric())
                .filter(|s| !s.is_empty())
                .collect();
            if !phrase_words.is_empty() {
                let clean_phrase = phrase_words.join(" ");
                tokens.push(format!("\"{}\"", clean_phrase));
                literal_terms.push(clean_phrase);
            }
        } else {
            let mut word = String::new();
            let mut has_wildcard = false;
            while let Some(&w) = chars.peek() {
                if w.is_whitespace() || w == '"' {
                    break;
                }
                chars.next();
                if w == '*' {
                    has_wildcard = true;
                    break;
                } else if w.is_alphanumeric() {
                    word.push(w);
                }
            }
            if !word.is_empty() {
                if has_wildcard {
                    tokens.push(format!("\"{}\"*", word));
                } else {
                    tokens.push(format!("\"{}\"", word));
                }
                literal_terms.push(word);
            }
        }
        if tokens.len() > MAX_BODY_QUERY_TERMS {
            return Err(invalid_body_search());
        }
    }

    if tokens.is_empty() {
        return Err(invalid_body_search());
    }
    let expression = tokens.join(" AND ");
    Ok((expression, literal_terms))
}

pub(crate) fn ensure_fts_table(conn: &Connection) -> Result<(), CommandError> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS memory_body_fts USING fts5(
            workspace_id UNINDEXED,
            memory_id UNINDEXED,
            revision_id UNINDEXED,
            title,
            body,
            tokenize='unicode61 remove_diacritics 2'
        );",
    )
    .map_err(|_| unavailable())
}

pub(crate) fn sync_index_tx(
    tx: &Transaction,
    workspace_id: &str,
    record: &str,
    revision: &str,
    title: &str,
    body: &str,
    privacy: &Privacy,
) -> Result<(), CommandError> {
    ensure_fts_table(tx)?;
    SearchBoundary::invalidate_record(tx, workspace_id, record).map_err(|_| unavailable())?;
    let _ = crate::vector::invalidate_embeddings(tx, workspace_id, record);
    if SearchBoundary::may_index(*privacy) {
        tx.execute(
            "INSERT INTO memory_body_fts(workspace_id, memory_id, revision_id, title, body)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![workspace_id, record, revision, title, body],
        )
        .map_err(|_| unavailable())?;
    }
    Ok(())
}

pub(crate) fn locate_evidence(
    body: &str,
    terms: &[String],
) -> (Option<String>, Option<u32>, String) {
    let lines: Vec<&str> = body.lines().collect();
    let mut current_heading: Option<String> = None;
    let mut matched_line: Option<usize> = None;
    let mut matched_heading: Option<String> = None;

    let lower_terms: Vec<String> = terms.iter().map(|t| t.to_lowercase()).collect();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            current_heading = Some(trimmed.to_string());
        }
        let lower_line = line.to_lowercase();
        let matches = lower_terms.iter().any(|term| {
            if term.contains(' ') {
                let parts: Vec<&str> = term.split_whitespace().collect();
                !parts.is_empty() && parts.iter().all(|p| lower_line.contains(p))
            } else if term.len() <= 1 {
                false
            } else {
                lower_line.contains(term)
            }
        });
        if matches {
            matched_line = Some(idx + 1);
            matched_heading = current_heading.clone();
            break;
        }
    }

    let snippet = if let Some(line_num) = matched_line {
        let idx = line_num - 1;
        let start = idx.saturating_sub(1);
        let end = (idx + 2).min(lines.len());
        let joined = lines[start..end].join(" ");
        let chars: Vec<char> = joined.chars().collect();
        if chars.len() > 160 {
            chars[..160].iter().collect::<String>() + "..."
        } else {
            joined
        }
    } else {
        let chars: Vec<char> = body.chars().collect();
        if chars.len() > 160 {
            chars[..160].iter().collect::<String>() + "..."
        } else {
            body.to_string()
        }
    };

    (matched_heading, matched_line.map(|l| l as u32), snippet)
}

fn calculate_recency_score(updated_at: &str, event_date: Option<&str>) -> f64 {
    let now = Utc::now();
    let ref_date = if let Some(ed) = event_date {
        chrono::DateTime::parse_from_rfc3339(ed)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    } else {
        None
    };
    let target_time = ref_date.unwrap_or_else(|| {
        chrono::DateTime::parse_from_rfc3339(updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(now)
    });
    let days_diff = (now - target_time).num_days().abs() as f64;
    1.0 / (1.0 + 0.05 * days_diff)
}

impl Storage {
    pub fn rebuild_search_index(&self, workspace_id: &str) -> Result<u32, CommandError> {
        crate::workspace::validate_id(workspace_id)?;
        let mut conn = self.connection(false)?.ok_or_else(unavailable)?;
        ensure_fts_table(&conn)?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        SearchBoundary::invalidate_workspace(&tx, workspace_id).map_err(|_| unavailable())?;

        let mut stmt = tx
            .prepare(
                "SELECT m.id, r.id, r.body_hash, r.metadata
                 FROM memories m
                 JOIN memory_revisions r ON r.workspace_id = m.workspace_id
                                        AND r.memory_id = m.id
                                        AND r.id = m.current_revision
                 WHERE m.workspace_id = ?1
                   AND json_extract(r.metadata, '$.privacy') != 'sealed'
                 ORDER BY m.id",
            )
            .map_err(|_| unavailable())?;

        let rows = stmt
            .query_map(params![workspace_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|_| unavailable())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| unavailable())?;
        drop(stmt);

        if rows.len() > MAX_RECORDS {
            return Err(CommandError::new(
                ErrorCode::InvalidSearch,
                "Body search exceeds 10,000 records. Narrow the record type and try again; no partial results were returned.",
            ));
        }

        let mut indexed_count = 0u32;
        for (memory_id, revision_id, body_hash, metadata_str) in rows {
            let metadata: MemoryMetadata =
                serde_json::from_str(&metadata_str).map_err(|_| content::integrity())?;
            if metadata.privacy == Privacy::Sealed {
                continue;
            }
            if metadata.workspace_id != workspace_id
                || metadata.id != memory_id
                || metadata.revision_id != revision_id
            {
                return Err(content::integrity());
            }
            metadata.privacy.allow_plaintext()?;
            let body = content::read(
                &self.root,
                workspace_id,
                &memory_id,
                &revision_id,
                &body_hash,
            )?;
            tx.execute(
                "INSERT INTO memory_body_fts(workspace_id, memory_id, revision_id, title, body)
                 VALUES(?1, ?2, ?3, ?4, ?5)",
                params![workspace_id, memory_id, revision_id, metadata.title, body],
            )
            .map_err(|_| unavailable())?;
            indexed_count += 1;
        }

        tx.commit().map_err(|_| unavailable())?;
        Ok(indexed_count)
    }

    pub(crate) fn body_search(&self, r: MemoryListRequest) -> Result<MemoryList, CommandError> {
        let (expression, terms) =
            parse_body_query(r.query.as_deref().ok_or_else(invalid_body_search)?)?;
        let conn = self.connection(false)?.ok_or_else(unavailable)?;
        ensure_fts_table(&conn)?;

        let fts_count: i64 = conn
            .query_row(
                "SELECT COUNT(1) FROM memory_body_fts WHERE workspace_id = ?1",
                params![&r.workspace_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let conn = if fts_count == 0 {
            let raw_count: i64 = conn
                .query_row(
                    "SELECT COUNT(1) FROM memories m
                     JOIN memory_revisions r ON r.workspace_id = m.workspace_id AND r.id = m.current_revision
                     WHERE m.workspace_id = ?1 AND json_extract(r.metadata, '$.privacy') != 'sealed'",
                    params![&r.workspace_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            drop(conn);
            if raw_count > 0 {
                self.rebuild_search_index(&r.workspace_id)?;
            }
            self.connection(false)?.ok_or_else(unavailable)?
        } else {
            conn
        };

        let mut stmt = conn
            .prepare(
                "SELECT
                    r.metadata,
                    m.id,
                    r.id,
                    r.body_hash,
                    bm25(memory_body_fts)
                 FROM memory_body_fts
                 JOIN memories m ON m.workspace_id = memory_body_fts.workspace_id
                                AND m.id = memory_body_fts.memory_id
                                AND m.current_revision = memory_body_fts.revision_id
                 JOIN memory_revisions r ON r.workspace_id = m.workspace_id
                                        AND r.memory_id = m.id
                                        AND r.id = m.current_revision
                 WHERE memory_body_fts.workspace_id = ?1
                   AND memory_body_fts MATCH ?2
                   AND json_extract(r.metadata, '$.privacy') != 'sealed'
                   AND (?3 IS NULL OR m.id = ?3)
                   AND (?4 IS NULL OR json_extract(r.metadata, '$.kind') = ?4)
                   AND (?5 OR json_extract(r.metadata, '$.status') != 'archived')
                   AND (?6 OR json_extract(r.metadata, '$.validUntil') IS NULL
                           OR julianday(json_extract(r.metadata, '$.validUntil')) > julianday(?7))
                 ORDER BY bm25(memory_body_fts), m.id
                 LIMIT ?8",
            )
            .map_err(|_| unavailable())?;

        let rows = stmt
            .query_map(
                params![
                    r.workspace_id,
                    expression,
                    r.id,
                    r.kind,
                    r.include_archived,
                    r.include_expired,
                    Utc::now().to_rfc3339(),
                    (MAX_RECORDS + 1) as i64
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, f64>(4)?,
                    ))
                },
            )
            .map_err(|_| unavailable())?;

        let mut candidate_list = Vec::new();
        let mut count = 0usize;

        for row in rows {
            count += 1;
            if count > MAX_RECORDS {
                return Err(CommandError::new(
                    ErrorCode::InvalidSearch,
                    "Body search exceeds 10,000 records. Narrow the record type and try again; no partial results were returned.",
                ));
            }
            let (meta_json, mem_id, rev_id, body_hash, bm25_val) =
                row.map_err(|_| content::integrity())?;
            let mut metadata: MemoryMetadata =
                serde_json::from_str(&meta_json).map_err(|_| content::integrity())?;
            if metadata.workspace_id != r.workspace_id
                || metadata.id != mem_id
                || metadata.revision_id != rev_id
            {
                return Err(content::integrity());
            }
            metadata.privacy.allow_plaintext()?;

            let recency_factor =
                calculate_recency_score(&metadata.updated_at, metadata.event_date.as_deref());
            let lexical_score = -bm25_val;
            let search_score = lexical_score * recency_factor;
            metadata.search_score = Some(search_score);

            candidate_list.push((metadata, body_hash));
        }

        candidate_list.sort_by(|(a, _), (b, _)| {
            b.search_score
                .partial_cmp(&a.search_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut seen_hashes: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (metadata, body_hash) in &mut candidate_list {
            if let Some(orig_id) = seen_hashes.get(body_hash) {
                metadata.duplicate_of = Some(orig_id.clone());
            } else {
                seen_hashes.insert(body_hash.clone(), metadata.id.clone());
            }
        }

        let limit = r.limit.unwrap_or(50) as usize;
        let offset = r.offset as usize;
        let total_matches = candidate_list.len();

        let next_offset = if total_matches > offset.saturating_add(limit) {
            Some(r.offset.saturating_add(limit as u32))
        } else {
            None
        };

        let page_items: Vec<_> = candidate_list
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        let mut memories = Vec::with_capacity(page_items.len());
        for (mut metadata, body_hash) in page_items {
            let body = content::read(
                &self.root,
                &r.workspace_id,
                &metadata.id,
                &metadata.revision_id,
                &body_hash,
            )?;
            let (section_heading, line_number, snippet) = locate_evidence(&body, &terms);
            metadata.snippet = Some(snippet);
            metadata.section_heading = section_heading;
            metadata.line_number = line_number;
            memories.push(metadata);
        }

        Ok(MemoryList {
            contract_version: 1,
            memories,
            next_offset,
        })
    }

    pub(crate) fn title_search(&self, r: MemoryListRequest) -> Result<MemoryList, CommandError> {
        let expression = query(r.query.as_deref().ok_or_else(invalid)?)?;
        let connection = self.connection(false)?.ok_or_else(unavailable)?;
        let mut index = Connection::open_in_memory().map_err(|_| unavailable())?;
        index.execute_batch("PRAGMA temp_store=MEMORY; CREATE VIRTUAL TABLE titles USING fts5(title, metadata UNINDEXED, id UNINDEXED, tokenize='unicode61 remove_diacritics 2');").map_err(|_|unavailable())?;
        let mut statement = connection.prepare("SELECT r.metadata,m.id,r.id FROM memory_revisions r JOIN memories m ON m.workspace_id=r.workspace_id AND m.id=r.memory_id AND m.current_revision=r.id WHERE r.workspace_id=?1 AND json_extract(r.metadata,'$.privacy')!='sealed' AND (?2 IS NULL OR json_extract(r.metadata,'$.kind')=?2) AND (?3 OR json_extract(r.metadata,'$.status')!='archived') AND (?4 OR json_extract(r.metadata,'$.validUntil') IS NULL OR julianday(json_extract(r.metadata,'$.validUntil'))>julianday(?5)) ORDER BY m.id LIMIT ?6").map_err(|_|content::integrity())?;
        let rows = statement
            .query_map(
                params![
                    r.workspace_id,
                    r.kind,
                    r.include_archived,
                    r.include_expired,
                    Utc::now().to_rfc3339(),
                    (MAX_RECORDS + 1) as i64
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .map_err(|_| content::integrity())?;
        let transaction = index.transaction().map_err(|_| unavailable())?;
        let mut size = 0usize;
        for (count, row) in rows.enumerate() {
            let (json, id, revision) = row.map_err(|_| content::integrity())?;
            size = size.saturating_add(json.len());
            if count >= MAX_RECORDS || size > MAX_METADATA {
                return Err(CommandError::new(
                    ErrorCode::InvalidSearch,
                    "Title search exceeds 10,000 records or 32 MiB of metadata. Narrow the record type and try again; no partial results were returned.",
                ));
            }
            let metadata: MemoryMetadata =
                serde_json::from_str(&json).map_err(|_| content::integrity())?;
            if metadata.workspace_id != r.workspace_id
                || metadata.id != id
                || metadata.revision_id != revision
            {
                return Err(content::integrity());
            }
            metadata.privacy.allow_plaintext()?;
            transaction
                .execute(
                    "INSERT INTO titles(title,metadata,id) VALUES(?1,?2,?3)",
                    params![metadata.title, json, id],
                )
                .map_err(|_| unavailable())?;
        }
        transaction.commit().map_err(|_| unavailable())?;
        let limit = r.limit.unwrap_or(50);
        let mut matched = index.prepare("SELECT metadata FROM titles WHERE titles MATCH ?1 ORDER BY rank,id LIMIT ?2 OFFSET ?3").map_err(|_|unavailable())?;
        let rows = matched
            .query_map(params![expression, limit + 1, r.offset], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|_| unavailable())?;
        let mut memories = Vec::new();
        for row in rows {
            memories.push(
                serde_json::from_str::<MemoryMetadata>(&row.map_err(|_| unavailable())?)
                    .map_err(|_| content::integrity())?,
            );
        }
        let next_offset = if memories.len() > limit as usize {
            memories.pop();
            Some(r.offset + limit)
        } else {
            None
        };
        Ok(MemoryList {
            contract_version: 1,
            memories,
            next_offset,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literal_query_and_bounds() {
        assert_eq!(
            query("café OR 世界*").unwrap(),
            "\"café\" AND \"OR\" AND \"世界\""
        );
        for text in ["", "* () :", "1 2 3 4 5 6 7 8 9", &"é".repeat(129)] {
            assert!(query(text).is_err());
        }
    }

    #[test]
    fn body_query_parser_supports_phrases_and_prefixes() {
        let (expr, terms) = parse_body_query("\"quantum computing\" algorithm* fast").unwrap();
        assert_eq!(
            expr,
            "\"quantum computing\" AND \"algorithm\"* AND \"fast\""
        );
        assert_eq!(terms, vec!["quantum computing", "algorithm", "fast"]);

        let (expr2, terms2) = parse_body_query("single").unwrap();
        assert_eq!(expr2, "\"single\"");
        assert_eq!(terms2, vec!["single"]);

        assert!(parse_body_query("").is_err());
        assert!(parse_body_query("!@#$%^&*()").is_err());
        assert!(parse_body_query(&"a ".repeat(17)).is_err());
        assert!(parse_body_query(&"a".repeat(513)).is_err());
    }

    #[test]
    fn evidence_location_finds_headings_and_lines() {
        let body =
            "# Top Heading\nFirst line.\n\n## Sub Section\nTarget content is here.\nAnother line.";
        let (heading, line, snippet) = locate_evidence(body, &["target".to_string()]);
        assert_eq!(heading, Some("## Sub Section".to_string()));
        assert_eq!(line, Some(5));
        assert!(snippet.contains("Target content is here"));
    }
}
