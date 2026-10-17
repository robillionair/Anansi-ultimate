use crate::{
    CommandError, Core, ErrorCode, content,
    storage::{Storage, unavailable},
    workspace::validate_id,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

pub const SUPPORTED_PREDICATES: &[&str] = &[
    "relates_to",
    "belongs_to_project",
    "assigned_to",
    "decided_by",
    "documents",
    "depends_on",
    "references",
];

fn input<T: DeserializeOwned>(value: Value) -> Result<T, CommandError> {
    if value
        .get("contractVersion")
        .and_then(Value::as_u64)
        .is_some_and(|v| v != 1)
    {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "This contract version is not supported.",
        ));
    }
    serde_json::from_value(value).map_err(|_| {
        CommandError::new(
            ErrorCode::InvalidRequest,
            "Use the supported relationship request.",
        )
    })
}

pub fn validate_predicate(predicate: &str) -> Result<(), CommandError> {
    if predicate.is_empty()
        || predicate.len() > 64
        || !predicate
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    {
        return Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "Predicate must be 1–64 ASCII alphanumeric, underscore or hyphen characters.",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelationshipDraft {
    pub source_id: String,
    pub target_id: String,
    pub predicate: String,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelationshipRecord {
    pub id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub target_id: String,
    pub predicate: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
    pub archived: bool,
    pub provenance: String,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelationshipDetails {
    pub contract_version: u8,
    pub relationship: RelationshipRecord,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelationshipList {
    pub contract_version: u8,
    pub relationships: Vec<RelationshipRecord>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateRelationshipRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub draft: RelationshipDraft,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteRelationshipRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: String,
    pub expected_version: u32,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListRelationshipsRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub record_id: Option<String>,
    pub predicate: Option<String>,
    #[serde(default)]
    pub include_history: bool,
}

impl Storage {
    pub(crate) fn create_relationship(
        &self,
        req: CreateRelationshipRequest,
    ) -> Result<RelationshipDetails, CommandError> {
        validate_id(&req.workspace_id)?;
        validate_id(&req.draft.source_id)?;
        validate_id(&req.draft.target_id)?;
        validate_predicate(&req.draft.predicate)?;
        if req.draft.source_id == req.draft.target_id {
            return Err(invalid());
        }
        let metadata = req.draft.metadata.unwrap_or_else(|| serde_json::json!({}));
        if !metadata.is_object() || metadata.to_string().len() > 8192 {
            return Err(invalid());
        }
        let mut connection = self.connection(false)?.ok_or_else(not_found)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        for endpoint in [&req.draft.source_id, &req.draft.target_id] {
            let raw: Option<String> = tx.query_row("SELECT r.metadata FROM memories m JOIN memory_revisions r ON r.workspace_id=m.workspace_id AND r.id=m.current_revision WHERE m.workspace_id=?1 AND m.id=?2",params![req.workspace_id,endpoint],|r|r.get(0)).optional().map_err(|_| unavailable())?;
            let meta: crate::memory::MemoryMetadata =
                serde_json::from_str(&raw.ok_or_else(not_found)?)
                    .map_err(|_| content::integrity())?;
            meta.privacy.allow_plaintext()?;
        }
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let record = RelationshipRecord {
            id: content::random_id()?,
            workspace_id: req.workspace_id,
            source_id: req.draft.source_id,
            target_id: req.draft.target_id,
            predicate: req.draft.predicate,
            created_at: now.clone(),
            updated_at: now,
            version: 1,
            archived: false,
            provenance: "user_authored".into(),
            metadata,
        };
        tx.execute("INSERT INTO relationships(workspace_id,id,source_id,target_id,predicate,metadata,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![record.workspace_id,record.id,record.source_id,record.target_id,record.predicate,record.metadata.to_string(),record.created_at]).map_err(|_| unavailable())?;
        append_revision(&tx, &record)?;
        tx.commit().map_err(|_| unavailable())?;
        Ok(RelationshipDetails {
            contract_version: 1,
            relationship: record,
        })
    }

    pub(crate) fn change_relationship(
        &self,
        req: DeleteRelationshipRequest,
        archived: bool,
    ) -> Result<RelationshipDetails, CommandError> {
        validate_id(&req.workspace_id)?;
        validate_id(&req.id)?;
        let mut connection = self.connection(false)?.ok_or_else(not_found)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        let raw: Option<String> = tx.query_row("SELECT snapshot FROM relationship_revisions WHERE workspace_id=?1 AND relationship_id=?2 ORDER BY version DESC LIMIT 1",params![req.workspace_id,req.id],|r|r.get(0)).optional().map_err(|_| unavailable())?;
        let mut record: RelationshipRecord =
            serde_json::from_str(&raw.ok_or_else(not_found)?).map_err(|_| content::integrity())?;
        if record.version != req.expected_version {
            return Err(CommandError::new(
                ErrorCode::RevisionConflict,
                "This relationship changed. Refresh before trying again.",
            ));
        }
        if record.archived == archived {
            return Err(invalid());
        }
        record.version = record.version.checked_add(1).ok_or_else(invalid)?;
        record.archived = archived;
        record.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        // Authorship of this action is explicit; the initial migrated revision retains its unknown legacy origin.
        record.provenance = "user_authored".into();
        tx.execute(
            "UPDATE relationships SET archived=?3,version=?4 WHERE workspace_id=?1 AND id=?2",
            params![req.workspace_id, req.id, archived, record.version],
        )
        .map_err(|_| unavailable())?;
        append_revision(&tx, &record)?;
        tx.commit().map_err(|_| unavailable())?;
        Ok(RelationshipDetails {
            contract_version: 1,
            relationship: record,
        })
    }

    pub(crate) fn list_relationships(
        &self,
        req: ListRelationshipsRequest,
    ) -> Result<RelationshipList, CommandError> {
        validate_id(&req.workspace_id)?;
        if let Some(id) = &req.record_id {
            validate_id(id)?;
        }
        if let Some(p) = &req.predicate {
            validate_predicate(p)?;
        }
        let connection = self.connection(false)?.ok_or_else(not_found)?;
        // History is scoped and bounded like current graph queries. No endpoint bodies are returned.
        let mut statement=connection.prepare("SELECT v.snapshot FROM relationship_revisions v JOIN relationships r ON r.workspace_id=v.workspace_id AND r.id=v.relationship_id WHERE r.workspace_id=?1 AND (?2 IS NULL OR r.source_id=?2 OR r.target_id=?2) AND (?3 IS NULL OR r.predicate=?3) AND (?4 OR (r.archived=0 AND r.version=v.version)) ORDER BY v.relationship_id,v.version DESC LIMIT 200").map_err(|_| unavailable())?;
        let rows = statement
            .query_map(
                params![
                    req.workspace_id,
                    req.record_id,
                    req.predicate,
                    req.include_history
                ],
                |r| r.get::<_, String>(0),
            )
            .map_err(|_| unavailable())?;
        let mut relationships = Vec::new();
        for raw in rows {
            relationships.push(
                serde_json::from_str(&raw.map_err(|_| unavailable())?)
                    .map_err(|_| content::integrity())?,
            );
        }
        Ok(RelationshipList {
            contract_version: 1,
            relationships,
        })
    }

    pub(crate) fn record_relationship_ids(
        connection: &Connection,
        workspace_id: &str,
        record_id: &str,
    ) -> Result<Vec<String>, CommandError> {
        let mut statement=connection.prepare("SELECT id FROM relationships WHERE workspace_id=?1 AND archived=0 AND (source_id=?2 OR target_id=?2) ORDER BY created_at DESC,id").map_err(|_| unavailable())?;
        let rows = statement
            .query_map(params![workspace_id, record_id], |r| r.get(0))
            .map_err(|_| unavailable())?;
        rows.map(|r| r.map_err(|_| unavailable())).collect()
    }
}
fn invalid() -> CommandError {
    CommandError::new(
        ErrorCode::InvalidRequest,
        "Use valid distinct endpoints, bounded object metadata and an applicable relationship action.",
    )
}
fn not_found() -> CommandError {
    CommandError::new(
        ErrorCode::NotFound,
        "Relationship or endpoints not found in this workspace.",
    )
}
fn append_revision(
    connection: &Connection,
    record: &RelationshipRecord,
) -> Result<(), CommandError> {
    connection
        .execute(
            "INSERT INTO relationship_revisions VALUES(?1,?2,?3,?4)",
            params![
                record.workspace_id,
                record.id,
                record.version,
                serde_json::to_string(record).map_err(|_| unavailable())?
            ],
        )
        .map_err(|_| unavailable())?;
    Ok(())
}

impl Core {
    pub fn relationship_command(
        &self,
        action: &str,
        request: Value,
    ) -> Result<Value, CommandError> {
        match action {
            "create" => {
                let r: CreateRelationshipRequest = input(request)?;
                serde_json::to_value(self.storage()?.create_relationship(r)?)
                    .map_err(|_| unavailable())
            }
            "delete" | "restore" => {
                let r: DeleteRelationshipRequest = input(request)?;
                serde_json::to_value(self.storage()?.change_relationship(r, action == "delete")?)
                    .map_err(|_| unavailable())
            }
            "list" => {
                let r: ListRelationshipsRequest = input(request)?;
                serde_json::to_value(self.storage()?.list_relationships(r)?)
                    .map_err(|_| unavailable())
            }
            _ => Err(CommandError::new(
                ErrorCode::NotFound,
                "Relationship command not found.",
            )),
        }
    }
}
