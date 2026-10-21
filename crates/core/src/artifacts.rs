//! The versioned artifact registry (MASTER_SPEC §9, M6 Slice 1).
//!
//! §9's universal project model already has its identity in the
//! Memory OS: a project is a memory with `kind = 'project'`.
//! This module attaches the artifact registry to that identity —
//! no parallel project system. An artifact belongs to exactly
//! one project (project-optional artifacts do not exist), and
//! every artifact version carries the §9 fields: unique ID,
//! project ID, type, version, timestamp, source references,
//! generator/model metadata, validation status and privacy
//! classification.
//!
//! Versions are immutable: registration inserts version 1 and
//! revision inserts a new row; nothing updates or deletes a
//! stored version. Revision is guarded by an expected-version
//! check (stale writes are refused with `RevisionConflict`, the
//! memory-revision rule). Every read re-verifies the stored
//! SHA-256 over the stored body; a mismatch is refused with
//! `ContentIntegrity` and the body is never returned.
//!
//! Privacy is fail-closed, mirroring the semantic pipeline's
//! Sealed posture: `sealed` registration is refused with
//! `VaultUnavailable` — sealed content belongs to the vault,
//! and this registry has no vault integration yet.

use crate::content;
use crate::lifecycle::MemoryState;
use crate::readiness::{CommandError, Core, ErrorCode};
use crate::storage::Storage;
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Manual artifact types for this slice, from §9's list of what
/// a project may contain.
pub const ARTIFACT_TYPES: [&str; 13] = [
    "research",
    "source",
    "idea",
    "concept",
    "experiment",
    "prototype",
    "generated-file",
    "simulation",
    "agent-test",
    "decision",
    "launch-material",
    "feedback",
    "note",
];

fn invalid(message: &str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn not_found(message: &str) -> CommandError {
    CommandError::new(ErrorCode::NotFound, message)
}

fn unavailable() -> CommandError {
    crate::storage::unavailable()
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn new_artifact_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("OS randomness is required for artifact ids");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn check_id(value: &str) -> Result<(), CommandError> {
    crate::workspace::validate_id(value)
        .map_err(|_| invalid("Identifiers must use the ANANSI id format."))
}

fn parse<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, CommandError> {
    serde_json::from_value(value).map_err(|_| invalid("Use a supported artifact request."))
}

// ---------------------------------------------------------------------------
// DTOs (§9 fields are all first-class)

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactRegisterRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub source_references: Vec<String>,
    #[serde(default = "default_privacy")]
    pub privacy: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactReviseRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub artifact_id: String,
    pub expected_version: u32,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub source_references: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactListRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactReadRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub artifact_id: String,
    pub version: Option<u32>,
}

/// One artifact version's metadata — the §9 field set.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub project_id: String,
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub version: u32,
    pub title: String,
    /// The §9 timestamp: when this version was registered.
    pub timestamp: String,
    pub source_references: Vec<String>,
    /// Generator/model metadata: `{"kind":"manual"}` in this
    /// slice; generator slices record model provenance here.
    pub generator: Option<Value>,
    pub validation_status: String,
    pub privacy: String,
    pub content_hash: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDetails {
    pub contract_version: u8,
    pub artifact: ArtifactRecord,
    pub body: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactListResponse {
    pub contract_version: u8,
    pub artifacts: Vec<ArtifactRecord>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactVersionsResponse {
    pub contract_version: u8,
    pub artifact_id: String,
    pub project_id: String,
    pub versions: Vec<ArtifactRecord>,
}

fn default_contract_version() -> u8 {
    1
}

fn default_privacy() -> String {
    "private".to_string()
}

// ---------------------------------------------------------------------------
// Validation

fn validate_write(
    title: &str,
    body: &str,
    source_references: &[String],
) -> Result<(), CommandError> {
    if title.is_empty() || title.chars().count() > 200 {
        return Err(invalid("Use an artifact title of 1 to 200 characters."));
    }
    if body.len() > content::MAX_BODY {
        return Err(invalid("Artifact text is limited to 256 KiB."));
    }
    if source_references.len() > 32
        || source_references
            .iter()
            .any(|r| r.is_empty() || r.chars().count() > 512)
    {
        return Err(invalid(
            "Use at most 32 source references of up to 512 characters.",
        ));
    }
    Ok(())
}

fn validate_registration(req: &ArtifactRegisterRequest) -> Result<(), CommandError> {
    validate_write(&req.title, &req.body, &req.source_references)?;
    if !ARTIFACT_TYPES.contains(&req.artifact_type.as_str()) {
        return Err(invalid(
            "Use one of the supported artifact types for manual registration.",
        ));
    }
    match req.privacy.as_str() {
        "standard" | "private" => Ok(()),
        "sealed" => Err(CommandError::new(
            ErrorCode::VaultUnavailable,
            "Sealed artifacts belong to the vault; this registry has no vault integration yet.",
        )),
        _ => Err(invalid("Privacy must be standard or private.")),
    }
}

#[derive(Deserialize)]
pub(crate) struct ProjectMeta {
    pub(crate) kind: String,
    pub(crate) status: MemoryState,
}

/// Resolve the §9 project identity: the project memory's current
/// metadata in this workspace. Unknown id → NotFound; a memory
/// that is not a project → InvalidRequest.
pub(crate) fn project_meta(
    connection: &Connection,
    workspace_id: &str,
    project_id: &str,
) -> Result<ProjectMeta, CommandError> {
    let json: Option<String> = connection
        .query_row(
            "SELECT r.metadata FROM memory_revisions r \
             JOIN memories m ON m.workspace_id = r.workspace_id AND m.id = r.memory_id \
             WHERE r.workspace_id = ?1 AND r.memory_id = ?2 AND r.id = m.current_revision",
            params![workspace_id, project_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| unavailable())?;
    let json = json.ok_or_else(|| not_found("Project not found in this workspace."))?;
    let meta: ProjectMeta = serde_json::from_str(&json).map_err(|_| content::integrity())?;
    if meta.kind != "project" {
        return Err(invalid("The named record is not a project."));
    }
    Ok(meta)
}

pub(crate) fn require_project_writable(
    connection: &Connection,
    workspace_id: &str,
    project_id: &str,
) -> Result<(), CommandError> {
    let meta = project_meta(connection, workspace_id, project_id)?;
    if meta.status == MemoryState::Archived {
        return Err(invalid(
            "The project is archived; restore it before changing its artifacts.",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Storage

struct Row {
    record: ArtifactRecord,
    body: String,
}

fn row_from(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    let source_references: String = r.get(7)?;
    let generator: Option<String> = r.get(8)?;
    Ok(Row {
        record: ArtifactRecord {
            artifact_id: r.get(0)?,
            project_id: r.get(1)?,
            artifact_type: r.get(2)?,
            version: r.get(3)?,
            title: r.get(4)?,
            timestamp: r.get(5)?,
            content_hash: r.get(6)?,
            source_references: serde_json::from_str(&source_references).unwrap_or_default(),
            generator: generator.and_then(|g| serde_json::from_str(&g).ok()),
            validation_status: r.get(9)?,
            privacy: r.get(10)?,
        },
        body: r.get(11)?,
    })
}

const COLUMNS: &str = "artifact_id, project_id, artifact_type, version, title, created_at, \
                       content_hash, source_references, generator, validation_status, privacy, body";

impl Storage {
    pub(crate) fn artifact_register(
        &self,
        req: &ArtifactRegisterRequest,
    ) -> Result<ArtifactDetails, CommandError> {
        self.artifact_register_inner(req, serde_json::json!({ "kind": "manual" }), "unvalidated")
    }

    /// Registration for machine-generated artifacts (idea
    /// generation): the caller supplies the generator metadata
    /// and the validation status its own validation earned.
    pub(crate) fn artifact_register_generated(
        &self,
        req: &ArtifactRegisterRequest,
        generator: Value,
        validation_status: &str,
    ) -> Result<ArtifactDetails, CommandError> {
        self.artifact_register_inner(req, generator, validation_status)
    }

    fn artifact_register_inner(
        &self,
        req: &ArtifactRegisterRequest,
        generator: Value,
        validation_status: &str,
    ) -> Result<ArtifactDetails, CommandError> {
        check_id(&req.workspace_id)?;
        check_id(&req.project_id)?;
        validate_registration(req)?;
        let connection = self.connection(true)?.ok_or_else(unavailable)?;
        require_project_writable(&connection, &req.workspace_id, &req.project_id)?;
        let artifact_id = new_artifact_id();
        let hash = content::hash(req.body.as_bytes());
        connection
            .execute(
                "INSERT INTO artifact_versions (workspace_id, artifact_id, project_id, version, \
                 artifact_type, title, body, content_hash, source_references, generator, \
                 validation_status, privacy, created_at) \
                 VALUES (?1, ?2, ?3, 1, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    req.workspace_id,
                    artifact_id,
                    req.project_id,
                    req.artifact_type,
                    req.title,
                    req.body,
                    hash,
                    serde_json::to_string(&req.source_references).unwrap_or_default(),
                    generator.to_string(),
                    validation_status,
                    req.privacy,
                    now_iso(),
                ],
            )
            .map_err(|_| unavailable())?;
        self.artifact_read(&req.workspace_id, &artifact_id, Some(1))
    }

    pub(crate) fn artifact_revise(
        &self,
        req: &ArtifactReviseRequest,
    ) -> Result<ArtifactDetails, CommandError> {
        check_id(&req.workspace_id)?;
        check_id(&req.artifact_id)?;
        validate_write(&req.title, &req.body, &req.source_references)?;
        let connection = self.connection(true)?.ok_or_else(unavailable)?;
        let latest = self.latest_row(&connection, &req.workspace_id, &req.artifact_id)?;
        let latest = latest.ok_or_else(|| not_found("Artifact not found in this workspace."))?;
        require_project_writable(&connection, &req.workspace_id, &latest.record.project_id)?;
        if req.expected_version != latest.record.version {
            return Err(CommandError::new(
                ErrorCode::RevisionConflict,
                "The artifact was revised by someone else; reload it before revising again.",
            ));
        }
        let next = latest.record.version + 1;
        let hash = content::hash(req.body.as_bytes());
        connection
            .execute(
                "INSERT INTO artifact_versions (workspace_id, artifact_id, project_id, version, \
                 artifact_type, title, body, content_hash, source_references, generator, \
                 validation_status, privacy, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    req.workspace_id,
                    req.artifact_id,
                    latest.record.project_id,
                    next,
                    latest.record.artifact_type,
                    req.title,
                    req.body,
                    hash,
                    serde_json::to_string(&req.source_references).unwrap_or_default(),
                    latest.record.generator.as_ref().map(|g| g.to_string()),
                    latest.record.validation_status,
                    latest.record.privacy,
                    now_iso(),
                ],
            )
            .map_err(|_| unavailable())?;
        self.artifact_read(&req.workspace_id, &req.artifact_id, Some(next))
    }

    fn latest_row(
        &self,
        connection: &Connection,
        workspace_id: &str,
        artifact_id: &str,
    ) -> Result<Option<Row>, CommandError> {
        connection
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM artifact_versions \
                     WHERE workspace_id = ?1 AND artifact_id = ?2 \
                     ORDER BY version DESC LIMIT 1"
                ),
                params![workspace_id, artifact_id],
                row_from,
            )
            .optional()
            .map_err(|_| unavailable())
    }

    pub(crate) fn artifact_list(
        &self,
        req: &ArtifactListRequest,
    ) -> Result<ArtifactListResponse, CommandError> {
        check_id(&req.workspace_id)?;
        check_id(&req.project_id)?;
        let connection = self.connection(false)?.ok_or_else(unavailable)?;
        project_meta(&connection, &req.workspace_id, &req.project_id)?;
        let mut stmt = connection
            .prepare(&format!(
                "SELECT {COLUMNS} FROM artifact_versions a \
                 WHERE a.workspace_id = ?1 AND a.project_id = ?2 \
                 AND a.version = (SELECT MAX(version) FROM artifact_versions \
                                  WHERE workspace_id = a.workspace_id \
                                  AND artifact_id = a.artifact_id) \
                 ORDER BY a.artifact_id"
            ))
            .map_err(|_| unavailable())?;
        let rows = stmt
            .query_map(params![req.workspace_id, req.project_id], row_from)
            .map_err(|_| unavailable())?;
        let mut artifacts = Vec::new();
        for row in rows {
            artifacts.push(row.map_err(|_| unavailable())?.record);
        }
        Ok(ArtifactListResponse {
            contract_version: 1,
            artifacts,
        })
    }

    pub(crate) fn artifact_versions(
        &self,
        workspace_id: &str,
        artifact_id: &str,
    ) -> Result<ArtifactVersionsResponse, CommandError> {
        check_id(workspace_id)?;
        check_id(artifact_id)?;
        let connection = self.connection(false)?.ok_or_else(unavailable)?;
        let mut stmt = connection
            .prepare(&format!(
                "SELECT {COLUMNS} FROM artifact_versions \
                 WHERE workspace_id = ?1 AND artifact_id = ?2 ORDER BY version"
            ))
            .map_err(|_| unavailable())?;
        let rows = stmt
            .query_map(params![workspace_id, artifact_id], row_from)
            .map_err(|_| unavailable())?;
        let mut versions = Vec::new();
        for row in rows {
            versions.push(row.map_err(|_| unavailable())?.record);
        }
        let first = versions
            .first()
            .ok_or_else(|| not_found("Artifact not found in this workspace."))?;
        Ok(ArtifactVersionsResponse {
            contract_version: 1,
            artifact_id: artifact_id.to_string(),
            project_id: first.project_id.clone(),
            versions,
        })
    }

    pub(crate) fn artifact_read(
        &self,
        workspace_id: &str,
        artifact_id: &str,
        version: Option<u32>,
    ) -> Result<ArtifactDetails, CommandError> {
        check_id(workspace_id)?;
        check_id(artifact_id)?;
        let connection = self.connection(false)?.ok_or_else(unavailable)?;
        let row = match version {
            Some(v) => connection
                .query_row(
                    &format!(
                        "SELECT {COLUMNS} FROM artifact_versions \
                         WHERE workspace_id = ?1 AND artifact_id = ?2 AND version = ?3"
                    ),
                    params![workspace_id, artifact_id, v],
                    row_from,
                )
                .optional()
                .map_err(|_| unavailable())?,
            None => self.latest_row(&connection, workspace_id, artifact_id)?,
        };
        let row = row.ok_or_else(|| not_found("Artifact version not found in this workspace."))?;
        // Integrity: the stored hash must match the stored body;
        // a mismatch means the storage was tampered with and the
        // body is never returned.
        if content::hash(row.body.as_bytes()) != row.record.content_hash {
            return Err(content::integrity());
        }
        Ok(ArtifactDetails {
            contract_version: 1,
            artifact: row.record,
            body: row.body,
        })
    }
}

// ---------------------------------------------------------------------------
// Command surface

impl Core {
    pub fn artifacts_command(&self, operation: &str, value: Value) -> Result<Value, CommandError> {
        let storage = self.storage()?;
        fn to_value<T: Serialize>(v: &T) -> Result<Value, CommandError> {
            serde_json::to_value(v).map_err(|_| unavailable())
        }
        match operation {
            "register" => {
                let req: ArtifactRegisterRequest = parse(value)?;
                require_v1(req.contract_version)?;
                to_value(&storage.artifact_register(&req)?)
            }
            "revise" => {
                let req: ArtifactReviseRequest = parse(value)?;
                require_v1(req.contract_version)?;
                to_value(&storage.artifact_revise(&req)?)
            }
            "list" => {
                let req: ArtifactListRequest = parse(value)?;
                require_v1(req.contract_version)?;
                to_value(&storage.artifact_list(&req)?)
            }
            "versions" => {
                let req: ArtifactReadRequest = parse(value)?;
                require_v1(req.contract_version)?;
                to_value(&storage.artifact_versions(&req.workspace_id, &req.artifact_id)?)
            }
            "read" => {
                let req: ArtifactReadRequest = parse(value)?;
                require_v1(req.contract_version)?;
                to_value(&storage.artifact_read(
                    &req.workspace_id,
                    &req.artifact_id,
                    req.version,
                )?)
            }
            _ => Err(invalid("Use a supported artifact operation.")),
        }
    }
}

fn require_v1(version: u8) -> Result<(), CommandError> {
    if version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    Ok(())
}
