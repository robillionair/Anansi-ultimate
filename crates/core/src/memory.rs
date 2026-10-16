use crate::{
    CommandError, Core, ErrorCode, content,
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
    pub status: String,
    pub privacy: Privacy,
    pub provenance: String,
    pub source_references: Vec<String>,
    pub generator_model: Option<String>,
    pub validation_status: String,
    pub relationships: Vec<String>,
    pub supersedes: Option<String>,
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
    #[serde(default)]
    pub confirm_privacy_change: bool,
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
fn validate(draft: &MemoryDraft) -> Result<(), CommandError> {
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
impl Core {
    pub fn memory_command(&self, operation: &str, value: Value) -> Result<Value, CommandError> {
        match operation {
            "create" | "revise" => {
                let r: MemoryWriteRequest = input(value)?;
                id(&r.workspace_id)?;
                validate(&r.draft)?;
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
                id(&r.workspace_id)?;
                if let Some(record) = &r.id {
                    id(record)?;
                }
                if r.offset > 1_000_000 || !(1..=100).contains(&r.limit.unwrap_or(50)) {
                    return Err(invalid());
                }
                serde_json::to_value(self.storage()?.memory_list(r)?).map_err(|_| unavailable())
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
    let meta: MemoryMetadata = serde_json::from_str(&json).map_err(|_| content::integrity())?;
    if meta.workspace_id != workspace
        || meta.id != record
        || revision.is_some_and(|r| r != meta.revision_id)
    {
        return Err(content::integrity());
    }
    meta.privacy.allow_plaintext()?;
    Ok((meta, hash))
}
impl Storage {
    pub(crate) fn memory_write(
        &self,
        r: MemoryWriteRequest,
    ) -> Result<MemoryDetails, CommandError> {
        // The public core validates inputs before any storage access. Check again for direct callers.
        id(&r.workspace_id)?;
        validate(&r.draft)?;
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
            if r.draft.privacy.downgrade_from(old.privacy) && !r.confirm_privacy_change {
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
        let record = old
            .as_ref()
            .map(|m| m.id.clone())
            .unwrap_or(content::random_id()?);
        let revision = content::random_id()?;
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let version = old
            .as_ref()
            .map_or(Some(1), |m| m.version.checked_add(1))
            .ok_or_else(invalid)?;
        let memory = MemoryMetadata {
            id: record.clone(),
            workspace_id: r.workspace_id.clone(),
            project_id: None,
            kind: "memory".into(),
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
            status: "proposed".into(),
            privacy: r.draft.privacy,
            provenance: "user_authored".into(),
            source_references: r.draft.source_references,
            generator_model: None,
            validation_status: "unreviewed".into(),
            relationships: vec![],
            supersedes: None,
        };
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
        if let Some(previous) = &old {
            let (latest, _) = metadata(&tx, &r.workspace_id, &record, None)?;
            if latest.revision_id != previous.revision_id {
                return Err(CommandError::new(
                    ErrorCode::RevisionConflict,
                    "This memory changed. Your draft was kept; reopen the current revision before saving again.",
                ));
            }
        }
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
        let connection = self.connection(false)?.ok_or_else(missing)?;
        let limit = r.limit.unwrap_or(50);
        let mut statement=connection.prepare("SELECT r.metadata FROM memory_revisions r JOIN memories m ON m.workspace_id=r.workspace_id AND m.id=r.memory_id WHERE r.workspace_id=?1 AND ((?2 IS NULL AND r.id=m.current_revision) OR r.memory_id=?2) ORDER BY r.memory_id,r.version DESC LIMIT ?3 OFFSET ?4").map_err(|_|unavailable())?;
        let rows = statement
            .query_map(params![r.workspace_id, r.id, limit + 1, r.offset], |row| {
                row.get::<_, String>(0)
            })
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
