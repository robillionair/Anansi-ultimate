use crate::{
    CommandError, Core, ErrorCode, content,
    storage::{Storage, unavailable},
    workspace::validate_id,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
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
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListRelationshipsRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub record_id: Option<String>,
    pub predicate: Option<String>,
}

impl Storage {
    pub(crate) fn create_relationship(
        &self,
        req: CreateRelationshipRequest,
    ) -> Result<RelationshipDetails, CommandError> {
        validate_id(&req.workspace_id)?;
        validate_id(&req.draft.source_id)?;
        validate_id(&req.draft.target_id)?;
        if req.draft.source_id == req.draft.target_id {
            return Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "Self-referencing relationships are rejected.",
            ));
        }
        validate_predicate(&req.draft.predicate)?;

        let connection = self.connection(false)?.ok_or_else(|| {
            CommandError::new(ErrorCode::NotFound, "Workspace storage not found.")
        })?;

        // Ensure both endpoints exist in the specified workspace
        let source_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM memories WHERE workspace_id=?1 AND id=?2)",
                params![&req.workspace_id, &req.draft.source_id],
                |row| row.get(0),
            )
            .map_err(|_| unavailable())?;
        let target_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM memories WHERE workspace_id=?1 AND id=?2)",
                params![&req.workspace_id, &req.draft.target_id],
                |row| row.get(0),
            )
            .map_err(|_| unavailable())?;

        if !source_exists || !target_exists {
            return Err(CommandError::new(
                ErrorCode::NotFound,
                "Source or target record not found in this workspace.",
            ));
        }

        let rel_id = content::random_id()?;
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let meta_val = req
            .draft
            .metadata
            .unwrap_or_else(|| Value::Object(Default::default()));
        let meta_str = serde_json::to_string(&meta_val).map_err(|_| unavailable())?;

        connection
            .execute(
                "INSERT INTO relationships(workspace_id, id, source_id, target_id, predicate, metadata, created_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &req.workspace_id,
                    &rel_id,
                    &req.draft.source_id,
                    &req.draft.target_id,
                    &req.draft.predicate,
                    &meta_str,
                    &now
                ],
            )
            .map_err(|_| unavailable())?;

        Ok(RelationshipDetails {
            contract_version: 1,
            relationship: RelationshipRecord {
                id: rel_id,
                workspace_id: req.workspace_id,
                source_id: req.draft.source_id,
                target_id: req.draft.target_id,
                predicate: req.draft.predicate,
                created_at: now,
                metadata: meta_val,
            },
        })
    }

    pub(crate) fn delete_relationship(
        &self,
        req: DeleteRelationshipRequest,
    ) -> Result<RelationshipDetails, CommandError> {
        validate_id(&req.workspace_id)?;
        validate_id(&req.id)?;

        let connection = self.connection(false)?.ok_or_else(|| {
            CommandError::new(ErrorCode::NotFound, "Workspace storage not found.")
        })?;

        let record_opt: Option<(String, String, String, String, String)> = connection
            .query_row(
                "SELECT source_id, target_id, predicate, metadata, created_at FROM relationships WHERE workspace_id=?1 AND id=?2",
                params![&req.workspace_id, &req.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(|_| unavailable())?;

        let (source_id, target_id, predicate, meta_str, created_at) =
            record_opt.ok_or_else(|| {
                CommandError::new(
                    ErrorCode::NotFound,
                    "Relationship not found in this workspace.",
                )
            })?;

        connection
            .execute(
                "DELETE FROM relationships WHERE workspace_id=?1 AND id=?2",
                params![&req.workspace_id, &req.id],
            )
            .map_err(|_| unavailable())?;

        let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));

        Ok(RelationshipDetails {
            contract_version: 1,
            relationship: RelationshipRecord {
                id: req.id,
                workspace_id: req.workspace_id,
                source_id,
                target_id,
                predicate,
                created_at,
                metadata,
            },
        })
    }

    pub(crate) fn list_relationships(
        &self,
        req: ListRelationshipsRequest,
    ) -> Result<RelationshipList, CommandError> {
        validate_id(&req.workspace_id)?;
        if let Some(rec) = &req.record_id {
            validate_id(rec)?;
        }
        if let Some(pred) = &req.predicate {
            validate_predicate(pred)?;
        }

        let connection = self.connection(false)?.ok_or_else(|| {
            CommandError::new(ErrorCode::NotFound, "Workspace storage not found.")
        })?;

        let mut statement = connection
            .prepare(
                "SELECT id, source_id, target_id, predicate, metadata, created_at FROM relationships WHERE workspace_id=?1 AND (?2 IS NULL OR source_id=?2 OR target_id=?2) AND (?3 IS NULL OR predicate=?3) ORDER BY created_at DESC LIMIT 200",
            )
            .map_err(|_| unavailable())?;

        let rows = statement
            .query_map(
                params![&req.workspace_id, &req.record_id, &req.predicate],
                |row| {
                    let id: String = row.get(0)?;
                    let source_id: String = row.get(1)?;
                    let target_id: String = row.get(2)?;
                    let predicate: String = row.get(3)?;
                    let meta_str: String = row.get(4)?;
                    let created_at: String = row.get(5)?;
                    let metadata = serde_json::from_str(&meta_str)
                        .unwrap_or(Value::Object(Default::default()));
                    Ok(RelationshipRecord {
                        id,
                        workspace_id: req.workspace_id.clone(),
                        source_id,
                        target_id,
                        predicate,
                        created_at,
                        metadata,
                    })
                },
            )
            .map_err(|_| unavailable())?;

        let mut relationships = Vec::new();
        for r in rows {
            relationships.push(r.map_err(|_| unavailable())?);
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
        let mut statement = connection
            .prepare(
                "SELECT id FROM relationships WHERE workspace_id=?1 AND (source_id=?2 OR target_id=?2) ORDER BY created_at DESC",
            )
            .map_err(|_| unavailable())?;

        let rows = statement
            .query_map(params![workspace_id, record_id], |row| row.get(0))
            .map_err(|_| unavailable())?;

        let mut ids = Vec::new();
        for r in rows {
            ids.push(r.map_err(|_| unavailable())?);
        }
        Ok(ids)
    }
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
            "delete" => {
                let r: DeleteRelationshipRequest = input(request)?;
                serde_json::to_value(self.storage()?.delete_relationship(r)?)
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
