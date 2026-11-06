use crate::{
    CommandError, Core, ErrorCode, content,
    document::{DocumentProvenance, JobPublishGuard},
    lifecycle::{LifecycleChange, MemoryState},
    privacy::Privacy,
    storage::{Storage, unavailable},
};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

#[cfg(test)]
fn fault(point: &str) {
    if std::env::var("ANANSI_UNIT_CRASH").ok().as_deref() == Some(point) {
        // Exit without Rust destructors, avoiding private-content crash dumps.
        std::process::exit(86);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn crash_worker() {
        let Ok(root) = std::env::var("ANANSI_UNIT_ROOT") else {
            return;
        };
        let request: Value =
            serde_json::from_str(&std::env::var("ANANSI_UNIT_REQUEST").unwrap()).unwrap();
        let _ = Core::with_storage_root(root.into()).memory_command("revise", request);
        panic!("Expected injected abort");
    }
    #[test]
    fn real_process_interruption_preserves_committed_state() {
        for point in ["before_file", "after_file", "before_commit", "after_commit"] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("storage");
            let core = Core::with_storage_root(root.clone());
            let w = core
                .workspace_command(
                    "create",
                    json!({"contractVersion":1,"name":"Crash fixture"}),
                )
                .unwrap()["workspace"]["id"]
                .clone();
            let mut r = json!({"contractVersion":1,"workspaceId":w,"draft":{"title":"Synthetic","body":"original"}});
            let first = core.memory_command("create", r.clone()).unwrap();
            r["id"] = first["memory"]["id"].clone();
            r["expectedRevision"] = first["memory"]["revisionId"].clone();
            r["draft"]["body"] = json!("committed second");
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "memory::tests::crash_worker", "--nocapture"])
                .env("ANANSI_UNIT_ROOT", &root)
                .env("ANANSI_UNIT_REQUEST", r.to_string())
                .env("ANANSI_UNIT_CRASH", point)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap();
            assert_eq!(
                status.code(),
                Some(86),
                "The intended interruption point must be reached"
            );
            let read = Core::with_storage_root(root)
                .memory_command(
                    "read",
                    json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"]}),
                )
                .unwrap();
            assert_eq!(
                read["body"],
                if point == "after_commit" {
                    "committed second"
                } else {
                    "original"
                }
            );
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryDraft {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub privacy: Privacy,
    pub event_date: Option<String>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub confidence: Option<f64>,
    #[serde(default)]
    pub source_references: Vec<String>,
    #[serde(default, deserialize_with = "project_assignment")]
    pub project_id: Option<Option<String>>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub completed: Option<bool>,
}
// Distinguish an omitted assignment (retain) from explicit null (clear).
fn project_assignment<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryMetadata {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub kind: String,
    pub revision_id: String,
    pub version: u32,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub event_date: Option<String>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub confidence: Option<f64>,
    pub status: MemoryState,
    #[serde(default)]
    pub archived_state: Option<MemoryState>,
    #[serde(default)]
    pub superseded_by: Option<String>,
    #[serde(default)]
    pub restored_from: Option<String>,
    pub privacy: Privacy,
    pub provenance: String,
    /// Exact-source provenance for imported documents (`document_import`
    /// records only). Absent for every other record, including all
    /// pre-ingestion rows, which parse with `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentProvenance>,
    pub source_references: Vec<String>,
    pub generator_model: Option<String>,
    pub validation_status: String,
    pub relationships: Vec<String>,
    pub supersedes: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub completed: Option<bool>,
    pub snippet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_heading: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_score: Option<f64>,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryDetails {
    pub contract_version: u8,
    pub memory: MemoryMetadata,
    pub body: String,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryList {
    pub contract_version: u8,
    pub memories: Vec<MemoryMetadata>,
    pub next_offset: Option<u32>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryListRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: Option<String>,
    #[serde(default)]
    pub offset: u32,
    pub limit: Option<u32>,
    pub kind: Option<String>,
    pub query: Option<String>,
    #[serde(default)]
    pub full_body: bool,
    #[serde(default)]
    pub include_archived: bool,
    #[serde(default)]
    pub include_expired: bool,
    /// Optional project membership filter (M8 Slice 4):
    /// restricts rows to revisions whose metadata
    /// `projectId` equals this id. Applies both to the
    /// current-revision listing and to the revision
    /// history of a single memory, so the collider
    /// project picker can list a project's members and
    /// the versions of one member that belong to it.
    #[serde(default)]
    pub project_id: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryReadRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: String,
    pub revision_id: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryWriteRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: Option<String>,
    pub expected_revision: Option<String>,
    pub draft: MemoryDraft,
    pub kind: Option<String>,
    pub lifecycle: Option<LifecycleChange>,
    #[serde(default)]
    pub restore_archive: bool,
    pub restore_revision: Option<String>,
    #[serde(default)]
    pub confirm_privacy_change: bool,
    /// Only `document_import` is accepted, only together with `document`
    /// and kind `document`, only on create. Provenance is verified
    /// against the body (see `validate_import`), never trusted.
    #[serde(default)]
    pub provenance: Option<String>,
    /// Exact-source provenance for an imported document. Accepted only
    /// with kind `document` on create; refused on revise.
    #[serde(default)]
    pub document: Option<DocumentProvenance>,
    /// Internal-only ingestion publish guard. Never deserialized.
    #[serde(skip)]
    pub job_guard: Option<JobPublishGuard>,
}
fn invalid() -> CommandError {
    CommandError::new(
        ErrorCode::InvalidMemory,
        "Use a title of 1–200 characters, text up to 256 KiB, valid UTC timestamps and confidence from 0 to 1. Unsupported fields are not accepted.",
    )
}
fn missing() -> CommandError {
    CommandError::new(
        ErrorCode::MemoryNotFound,
        "Memory not found in this workspace. Refresh the list and choose an existing memory.",
    )
}
fn input<T: DeserializeOwned>(value: Value) -> Result<T, CommandError> {
    if value
        .get("contractVersion")
        .and_then(Value::as_u64)
        .is_some_and(|v| v != 1)
    {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Unsupported memory contract.",
        ));
    }
    serde_json::from_value(value).map_err(|_| invalid())
}
fn stamp(value: &Option<String>) -> Result<Option<DateTime<chrono::FixedOffset>>, CommandError> {
    value
        .as_ref()
        .map(|v| {
            if v.len() > 35 || !v.ends_with('Z') {
                return Err(invalid());
            }
            DateTime::parse_from_rfc3339(v).map_err(|_| invalid())
        })
        .transpose()
}
pub(crate) fn validate(draft: &MemoryDraft) -> Result<(), CommandError> {
    draft.privacy.allow_plaintext()?;
    if draft.title.trim().is_empty()
        || draft.title.chars().count() > 200
        || draft.title.contains(['\0', '\n', '\r'])
        || draft.body.len() > content::MAX_BODY
        || draft.body.contains('\0')
        || draft.source_references.len() > 32
        || draft
            .source_references
            .iter()
            .any(|s| s.is_empty() || s.len() > 2048 || s.contains('\0'))
        || draft
            .confidence
            .is_some_and(|n| !n.is_finite() || !(0.0..=1.0).contains(&n))
    {
        return Err(invalid());
    }
    stamp(&draft.event_date)?;
    stamp(&draft.due_date)?;
    if let Some(Some(pid)) = &draft.project_id {
        id(pid)?;
    }
    let from = stamp(&draft.valid_from)?;
    let until = stamp(&draft.valid_until)?;
    if from.zip(until).is_some_and(|(a, b)| b < a) {
        return Err(invalid());
    }
    Ok(())
}
fn id(value: &str) -> Result<(), CommandError> {
    crate::workspace::validate_id(value).map_err(|_| invalid())
}
/// Provenance and document-record rules for `MemoryWriteRequest`.
/// Provenance is verified against the submitted body, never trusted:
/// `document_import` is the only accepted provenance value, it requires
/// the document record and kind `document`, only on create, the title
/// must be the file name, the media type must match the kind, and both
/// hashes/counts must recompute against the body.
fn validate_import(r: &MemoryWriteRequest) -> Result<(), CommandError> {
    if r.provenance.is_none() && r.document.is_none() {
        return Ok(());
    }
    if r.id.is_some() {
        return Err(invalid());
    }
    if r.provenance.as_deref() != Some("document_import") {
        return Err(invalid());
    }
    let doc = r.document.as_ref().ok_or_else(invalid)?;
    if r.kind.as_deref() != Some("document") {
        return Err(invalid());
    }
    if doc.file_name != r.draft.title {
        return Err(invalid());
    }
    let kind = match doc.media_type.as_str() {
        "text/plain" => "txt",
        "text/markdown" => "md",
        _ => return Err(invalid()),
    };
    if doc.media_type != DocumentProvenance::media_type_for(kind) {
        return Err(invalid());
    }
    crate::document::validate_document_source(
        &doc.file_name,
        kind,
        doc.bom,
        doc.raw_bytes,
        &doc.raw_hash,
        &doc.text_hash,
        doc.text_chars,
        &r.draft.body,
    )
    .map_err(|_| invalid())
}
impl Core {
    pub fn memory_command(&self, operation: &str, value: Value) -> Result<Value, CommandError> {
        match operation {
            "create" | "revise" => {
                let r: MemoryWriteRequest = input(value)?;
                id(&r.workspace_id)?;
                validate(&r.draft)?;
                validate_import(&r)?;
                if operation == "create" {
                    if r.id.is_some() || r.expected_revision.is_some() {
                        return Err(invalid());
                    }
                } else {
                    id(r.id.as_deref().ok_or_else(invalid)?)?;
                    id(r.expected_revision.as_deref().ok_or_else(invalid)?)?;
                }
                serde_json::to_value(self.storage()?.memory_write(r)?).map_err(|_| unavailable())
            }
            "read" => {
                let r: MemoryReadRequest = input(value)?;
                id(&r.workspace_id)?;
                id(&r.id)?;
                if let Some(rev) = &r.revision_id {
                    id(rev)?;
                }
                serde_json::to_value(self.storage()?.memory_read(r)?).map_err(|_| unavailable())
            }
            "list" => {
                let r: MemoryListRequest = input(value)?;
                if let Some(query) = &r.query {
                    if r.full_body {
                        crate::search::validate_body_query(query)?;
                    } else {
                        crate::search::query(query)?;
                    }
                    if r.id.is_some() {
                        return Err(crate::search::invalid());
                    }
                }
                id(&r.workspace_id)?;
                if let Some(record) = &r.id {
                    id(record)?;
                }
                if r.kind
                    .as_deref()
                    .is_some_and(|k| !crate::entities::is_valid_kind(k))
                {
                    return Err(invalid());
                }
                if r.offset > 1_000_000 || !(1..=100).contains(&r.limit.unwrap_or(50)) {
                    return Err(invalid());
                }
                let result = if r.full_body {
                    self.storage()?.body_search(r)?
                } else {
                    self.storage()?.memory_list(r)?
                };
                serde_json::to_value(result).map_err(|_| unavailable())
            }
            "reindex" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct ReindexRequest {
                    workspace_id: String,
                }
                let r: ReindexRequest = input(value)?;
                id(&r.workspace_id)?;
                let count = self.storage()?.rebuild_search_index(&r.workspace_id)?;
                serde_json::to_value(serde_json::json!({
                    "contractVersion": 1,
                    "workspaceId": r.workspace_id,
                    "indexedCount": count
                }))
                .map_err(|_| unavailable())
            }
            _ => Err(CommandError::new(
                ErrorCode::NotFound,
                "Memory command not found.",
            )),
        }
    }
}
fn metadata(
    connection: &Connection,
    workspace: &str,
    record: &str,
    revision: Option<&str>,
) -> Result<(MemoryMetadata, String), CommandError> {
    let result:Option<(String,String)>=connection.query_row("SELECT r.metadata,r.body_hash FROM memory_revisions r JOIN memories m ON m.workspace_id=r.workspace_id AND m.id=r.memory_id WHERE r.workspace_id=?1 AND r.memory_id=?2 AND r.id=COALESCE(?3,m.current_revision)",params![workspace,record,revision],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|content::integrity())?;
    let (json, hash) = result.ok_or_else(missing)?;
    let mut meta: MemoryMetadata = serde_json::from_str(&json).map_err(|_| content::integrity())?;
    if meta.workspace_id != workspace
        || meta.id != record
        || revision.is_some_and(|r| r != meta.revision_id)
    {
        return Err(content::integrity());
    }
    meta.privacy.allow_plaintext()?;
    if revision.is_none() {
        meta.relationships = Storage::record_relationship_ids(connection, workspace, record)?;
    }
    Ok((meta, hash))
}

/// Ingestion publish guard (see `JobPublishGuard` in `document.rs`).
/// Verifies the job is still Running under this lease, then looks for an
/// existing current-revision `document` memory with the same title and
/// text hash. Returns that record's details when one exists, so the
/// caller publishes nothing. The caller runs this twice: once before
/// writing the content file (cheap path, no orphan) and once inside the
/// publish transaction (authoritative; the transaction serializes
/// concurrent publishers and cancellations).
fn guard_check(
    connection: &Connection,
    root: &std::path::Path,
    workspace: &str,
    guard: &JobPublishGuard,
) -> Result<Option<MemoryDetails>, CommandError> {
    let row: Option<(String, String)> = connection
        .query_row(
            "SELECT state, owner FROM jobs WHERE workspace_id=?1 AND id=?2",
            params![workspace, guard.job_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|_| unavailable())?;
    let Some((state, owner)) = row else {
        return Err(unavailable());
    };
    if state != "Running" || owner != guard.owner {
        return Err(CommandError::new(
            ErrorCode::RevisionConflict,
            "The import job was cancelled or its lease expired; nothing was published.",
        ));
    }
    let (Some(title), Some(text_hash)) = (&guard.dedup_title, &guard.dedup_text_hash) else {
        return Ok(None);
    };
    let found: Option<(String, String)> = connection
        .query_row(
            "SELECT m.id, m.current_revision FROM memories m JOIN memory_revisions r ON r.workspace_id=m.workspace_id AND r.memory_id=m.id AND r.id=m.current_revision WHERE m.workspace_id=?1 AND json_extract(r.metadata,'$.kind')='document' AND json_extract(r.metadata,'$.title')=?2 AND r.body_hash=?3",
            params![workspace, title, text_hash],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|_| unavailable())?;
    let Some((id, revision)) = found else {
        return Ok(None);
    };
    let (meta, hash) = metadata(connection, workspace, &id, Some(&revision))?;
    let body = content::read(root, workspace, &id, &meta.revision_id, &hash)?;
    Ok(Some(MemoryDetails {
        contract_version: 1,
        memory: meta,
        body,
    }))
}
impl Core {
    pub fn rebuild_search_index(&self, workspace_id: &str) -> Result<u32, CommandError> {
        id(workspace_id)?;
        self.storage()?.rebuild_search_index(workspace_id)
    }
}

impl Storage {
    pub(crate) fn memory_write(
        &self,
        mut r: MemoryWriteRequest,
    ) -> Result<MemoryDetails, CommandError> {
        // The public core validates inputs before any storage access. Check again for direct callers.
        id(&r.workspace_id)?;
        validate(&r.draft)?;
        validate_import(&r)?;
        if r.kind
            .as_deref()
            .is_some_and(|k| !crate::entities::is_valid_kind(k))
        {
            return Err(invalid());
        }
        let mut connection = self.connection(false)?.ok_or_else(missing)?;
        if !connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id=?1)",
                [&r.workspace_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|_| unavailable())?
        {
            return Err(missing());
        }
        let old = if let Some(record) = &r.id {
            id(record)?;
            let (old, hash) = metadata(&connection, &r.workspace_id, record, None)?;
            if r.expected_revision.as_deref() != Some(&old.revision_id) {
                return Err(CommandError::new(
                    ErrorCode::RevisionConflict,
                    "This memory changed. Your draft was kept; reopen the current revision before saving again.",
                ));
            }
            content::read(&self.root, &r.workspace_id, record, &old.revision_id, &hash)?;
            if r.restore_revision.is_none()
                && r.draft.privacy.downgrade_from(old.privacy)
                && !r.confirm_privacy_change
            {
                return Err(CommandError::new(
                    ErrorCode::InvalidMemory,
                    "Confirm the privacy classification change before saving.",
                ));
            }
            Some(old)
        } else {
            if r.expected_revision.is_some() {
                return Err(invalid());
            }
            None
        };
        if old.is_none()
            && (r.lifecycle.is_some() || r.restore_archive || r.restore_revision.is_some())
        {
            return Err(invalid());
        }
        if let Some(previous) = &old {
            if r.kind.as_ref().is_some_and(|k| *k != previous.kind) {
                return Err(invalid());
            }
            if let Some(rev) = &r.restore_revision {
                id(rev)?;
                if r.lifecycle.is_some() || r.restore_archive {
                    return Err(invalid());
                }
                let (historic, hash) =
                    metadata(&connection, &r.workspace_id, &previous.id, Some(rev))?;
                r.draft = MemoryDraft {
                    title: historic.title,
                    body: content::read(&self.root, &r.workspace_id, &previous.id, rev, &hash)?,
                    privacy: previous.privacy,
                    event_date: historic.event_date,
                    valid_from: historic.valid_from,
                    valid_until: historic.valid_until,
                    confidence: historic.confidence,
                    source_references: historic.source_references,
                    project_id: Some(historic.project_id),
                    due_date: historic.due_date,
                    completed: historic.completed,
                };
            }
        }
        let kind = old
            .as_ref()
            .map(|m| m.kind.as_str())
            .or(r.kind.as_deref())
            .unwrap_or("memory");
        crate::entities::validate_entity_fields(
            kind,
            r.draft.project_id.as_ref().and_then(|p| p.as_deref()),
            r.draft.due_date.as_deref(),
            r.draft.event_date.as_deref(),
        )?;
        if kind != "task" && (r.draft.due_date.is_some() || r.draft.completed == Some(true)) {
            return Err(invalid());
        }
        let record = old
            .as_ref()
            .map(|m| m.id.clone())
            .unwrap_or(content::random_id()?);
        if let Some(Some(pid)) = &r.draft.project_id {
            let project_exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM memories m JOIN memory_revisions r ON r.workspace_id=m.workspace_id AND r.id=m.current_revision WHERE m.workspace_id=?1 AND m.id=?2 AND json_extract(r.metadata,'$.kind')='project')",
                    params![&r.workspace_id, pid],
                    |row| row.get(0),
                )
                .map_err(|_| unavailable())?;
            if !project_exists {
                return Err(invalid());
            }
        }
        let project_id = r
            .draft
            .project_id
            .clone()
            .unwrap_or_else(|| old.as_ref().and_then(|m| m.project_id.clone()));
        let revision = content::random_id()?;
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let version = old
            .as_ref()
            .map_or(Some(1), |m| m.version.checked_add(1))
            .ok_or_else(invalid)?;
        let mut memory = MemoryMetadata {
            id: record.clone(),
            workspace_id: r.workspace_id.clone(),
            project_id,
            kind: old
                .as_ref()
                .map(|m| m.kind.clone())
                .unwrap_or_else(|| r.kind.clone().unwrap_or_else(|| "memory".into())),
            revision_id: revision.clone(),
            version,
            title: r.draft.title,
            created_at: old
                .as_ref()
                .map(|m| m.created_at.clone())
                .unwrap_or(now.clone()),
            updated_at: now,
            event_date: r.draft.event_date,
            valid_from: r.draft.valid_from,
            valid_until: r.draft.valid_until,
            confidence: r.draft.confidence,
            status: old.as_ref().map_or(MemoryState::Proposed, |m| m.status),
            archived_state: old.as_ref().and_then(|m| m.archived_state),
            superseded_by: old.as_ref().and_then(|m| m.superseded_by.clone()),
            restored_from: r.restore_revision.clone(),
            privacy: r.draft.privacy,
            provenance: old
                .as_ref()
                .map(|m| m.provenance.clone())
                .unwrap_or_else(|| {
                    r.provenance
                        .clone()
                        .unwrap_or_else(|| "user_authored".into())
                }),
            document: old
                .as_ref()
                .and_then(|m| m.document.clone())
                .or_else(|| r.document.clone()),
            source_references: r.draft.source_references,
            generator_model: old.as_ref().and_then(|m| m.generator_model.clone()),
            validation_status: old
                .as_ref()
                .map(|m| m.validation_status.clone())
                .unwrap_or_else(|| "unreviewed".into()),
            relationships: Storage::record_relationship_ids(&connection, &r.workspace_id, &record)?,
            supersedes: old.as_ref().and_then(|m| m.supersedes.clone()),
            due_date: r.draft.due_date,
            completed: r.draft.completed,
            snippet: None,
            section_heading: None,
            line_number: None,
            duplicate_of: None,
            search_score: None,
        };
        if let Some(previous) = &old {
            if r.restore_archive {
                if r.lifecycle.is_some() || previous.status != MemoryState::Archived {
                    return Err(crate::lifecycle::invalid());
                }
                memory.status = previous
                    .archived_state
                    .ok_or_else(crate::lifecycle::invalid)?;
                memory.archived_state = None;
            }
            if let Some(change) = &r.lifecycle {
                if !change.confirmed || !previous.status.allows(change.state) {
                    return Err(crate::lifecycle::invalid());
                }
                memory.status = change.state;
                if change.state == MemoryState::Archived {
                    memory.archived_state = Some(previous.status);
                }
                if change.state == MemoryState::Superseded {
                    let successor = change
                        .successor_id
                        .as_deref()
                        .ok_or_else(crate::lifecycle::invalid)?;
                    id(successor)?;
                    memory.superseded_by = Some(successor.into());
                } else if change.successor_id.is_some() {
                    return Err(invalid());
                }
            }
        }
        if memory.status == MemoryState::Temporary && memory.valid_until.is_none() {
            return Err(crate::lifecycle::invalid());
        }
        if r.lifecycle
            .as_ref()
            .is_some_and(|c| c.state == MemoryState::Temporary)
            && stamp(&memory.valid_until)?.is_none_or(|date| date.with_timezone(&Utc) <= Utc::now())
        {
            return Err(crate::lifecycle::invalid());
        }
        // Ingestion guard, cheap path: refuse a cancelled/expired job and
        // return an existing duplicate before writing any content file.
        // The authoritative check runs again inside the publish
        // transaction below. The guard is create-only.
        if let Some(guard) = &r.job_guard {
            if old.is_some() {
                return Err(invalid());
            }
            if let Some(existing) = guard_check(&connection, &self.root, &r.workspace_id, guard)? {
                return Ok(existing);
            }
        }
        #[cfg(test)]
        fault("before_file");
        let hash = content::write(
            &self.root,
            &r.workspace_id,
            &record,
            &revision,
            &r.draft.body,
        )?;
        #[cfg(test)]
        fault("after_file");
        // Do not hold the write lock during filesystem/ACL work. Recheck the
        // optimistic revision under the lock before publishing immutable bytes.
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        // Ingestion guard, authoritative path: the lease and dedup are
        // re-verified under the publication lock. A duplicate found here
        // (or a cancelled lease) means the content file just written is
        // an orphan; remove it before returning.
        if let Some(guard) = &r.job_guard {
            match guard_check(&tx, &self.root, &r.workspace_id, guard) {
                Ok(Some(existing)) => {
                    drop(tx);
                    let _ = content::remove(&self.root, &r.workspace_id, &record, &revision);
                    return Ok(existing);
                }
                Ok(None) => {}
                Err(error) => {
                    drop(tx);
                    let _ = content::remove(&self.root, &r.workspace_id, &record, &revision);
                    return Err(error);
                }
            }
        }
        if let Some(previous) = &old {
            let (latest, _) = metadata(&tx, &r.workspace_id, &record, None)?;
            if latest.revision_id != previous.revision_id {
                return Err(CommandError::new(
                    ErrorCode::RevisionConflict,
                    "This memory changed. Your draft was kept; reopen the current revision before saving again.",
                ));
            }
        }
        // Check the complete successor graph under the publication lock.
        // Archive retains edges, preventing concurrent A->B/B->A cycles.
        if let Some(successor) = &memory.superseded_by {
            let mut cursor = Some(successor.clone());
            let mut visited = std::collections::HashSet::new();
            while let Some(next) = cursor {
                if next == record || !visited.insert(next.clone()) {
                    return Err(crate::lifecycle::invalid());
                }
                let (target, _) = metadata(&tx, &r.workspace_id, &next, None)?;
                cursor = target.superseded_by;
            }
        }
        memory.relationships = Storage::record_relationship_ids(&tx, &r.workspace_id, &record)?;
        if old.is_none() {
            tx.execute(
                "INSERT INTO memories VALUES(?1,?2,?3)",
                params![r.workspace_id, record, revision],
            )
            .map_err(|_| unavailable())?;
        }
        tx.execute(
            "INSERT INTO memory_revisions VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                r.workspace_id,
                record,
                revision,
                version,
                serde_json::to_string(&memory).map_err(|_| unavailable())?,
                hash
            ],
        )
        .map_err(|_| unavailable())?;
        tx.execute(
            "UPDATE memories SET current_revision=?3 WHERE workspace_id=?1 AND id=?2",
            params![r.workspace_id, record, revision],
        )
        .map_err(|_| unavailable())?;
        crate::search::sync_index_tx(
            &tx,
            &r.workspace_id,
            &record,
            &revision,
            &memory.title,
            &r.draft.body,
            &memory.privacy,
        )?;
        #[cfg(test)]
        fault("before_commit");
        tx.commit().map_err(|_| unavailable())?;
        #[cfg(test)]
        fault("after_commit");
        Ok(MemoryDetails {
            contract_version: 1,
            memory,
            body: r.draft.body,
        })
    }
    pub(crate) fn memory_read(&self, r: MemoryReadRequest) -> Result<MemoryDetails, CommandError> {
        id(&r.workspace_id)?;
        id(&r.id)?;
        if let Some(rev) = &r.revision_id {
            id(rev)?;
        }
        let connection = self.connection(false)?.ok_or_else(missing)?;
        let (memory, hash) = metadata(
            &connection,
            &r.workspace_id,
            &r.id,
            r.revision_id.as_deref(),
        )?;
        let body = content::read(
            &self.root,
            &r.workspace_id,
            &r.id,
            &memory.revision_id,
            &hash,
        )?;
        Ok(MemoryDetails {
            contract_version: 1,
            memory,
            body,
        })
    }
    pub(crate) fn memory_list(&self, r: MemoryListRequest) -> Result<MemoryList, CommandError> {
        if r.query.is_some() {
            if r.full_body {
                return self.body_search(r);
            } else {
                return self.title_search(r);
            }
        }
        let connection = self.connection(false)?.ok_or_else(missing)?;
        let limit = r.limit.unwrap_or(50);
        // Sealed rows are excluded in SQL, not merely
        // filtered after the read: a workspace containing
        // a Sealed memory must still list its other
        // memories (the collider picker depends on this),
        // and Sealed content is never disclosed. The
        // allow_plaintext check below stays as a
        // backstop for any row that slips through.
        let mut statement=connection.prepare("SELECT r.metadata FROM memory_revisions r JOIN memories m ON m.workspace_id=r.workspace_id AND m.id=r.memory_id WHERE r.workspace_id=?1 AND json_extract(r.metadata,'$.privacy')!='sealed' AND ((?2 IS NULL AND r.id=m.current_revision) OR r.memory_id=?2) AND (?2 IS NOT NULL OR ((?5 IS NULL OR json_extract(r.metadata,'$.kind')=?5) AND (?6 OR json_extract(r.metadata,'$.status')!='archived') AND (?7 OR json_extract(r.metadata,'$.validUntil') IS NULL OR julianday(json_extract(r.metadata,'$.validUntil'))>julianday(?8)))) AND (?9 IS NULL OR json_extract(r.metadata,'$.projectId')=?9) ORDER BY r.memory_id,r.version DESC LIMIT ?3 OFFSET ?4").map_err(|_|unavailable())?;
        let rows = statement
            .query_map(
                params![
                    r.workspace_id,
                    r.id,
                    limit + 1,
                    r.offset,
                    r.kind,
                    r.include_archived,
                    r.include_expired,
                    Utc::now().to_rfc3339(),
                    r.project_id,
                ],
                |row| row.get::<_, String>(0),
            )
            .map_err(|_| unavailable())?;
        let mut memories = vec![];
        for row in rows {
            let meta: MemoryMetadata =
                serde_json::from_str(&row.map_err(|_| content::integrity())?)
                    .map_err(|_| content::integrity())?;
            meta.privacy.allow_plaintext()?;
            memories.push(meta);
        }
        let next = if memories.len() > limit as usize {
            memories.pop();
            Some(r.offset + limit)
        } else {
            None
        };
        Ok(MemoryList {
            contract_version: 1,
            memories,
            next_offset: next,
        })
    }
}
