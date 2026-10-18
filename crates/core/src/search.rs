//! Bounded, request-local title retrieval. Canonical catalogs and bodies are never indexed on disk.
use crate::{
    CommandError, ErrorCode, content,
    memory::{MemoryList, MemoryListRequest, MemoryMetadata},
    storage::{Storage, unavailable},
};
use chrono::Utc;
use rusqlite::{Connection, params};

const MAX_RECORDS: usize = 10_000;
const MAX_METADATA: usize = 32 * 1024 * 1024;

pub(crate) fn invalid() -> CommandError {
    CommandError::new(
        ErrorCode::InvalidSearch,
        "Search titles using one to eight words, at most 256 UTF-8 bytes. History search is unavailable.",
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

impl Storage {
    pub(crate) fn title_search(&self, r: MemoryListRequest) -> Result<MemoryList, CommandError> {
        let expression = query(r.query.as_deref().ok_or_else(invalid)?)?;
        let connection = self.connection(false)?.ok_or_else(unavailable)?;
        let mut index = Connection::open_in_memory().map_err(|_| unavailable())?;
        index.execute_batch("PRAGMA temp_store=MEMORY; CREATE VIRTUAL TABLE titles USING fts5(title, metadata UNINDEXED, id UNINDEXED, tokenize='unicode61 remove_diacritics 2');").map_err(|_|unavailable())?;
        // A single SELECT holds a consistent current-metadata snapshot. Apply visibility before ranking.
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
}
