//! MCP *server* side (MASTER_SPEC §68, Milestone 13, Slice 5).
//!
//! ANANSI itself answers MCP requests over NDJSON stdio, exposing exactly
//! seven approved knowledge tools — `memory.search`, `memory.get`,
//! `notes.search`, `projects.search`, `experiments.list`, `products.get`,
//! and the mutation-gated `ideas.create`. The allowlist below is the whole
//! permission set, enforced in dispatch before any backend call.
//!
//! Security boundary, in three layers:
//! 1. Disabled by default: the server runs only when the operator launches
//!    `anansi-api --mcp-server` with a workspace id and a pairing root.
//!    No flag, no server.
//! 2. Pairing-token gate: the client must present a bearer token in
//!    `initialize` params (`_meta.anansiMcpToken`, an ANANSI extension on
//!    MCP's sanctioned `_meta` point — not part of the MCP spec) that
//!    matches a token in a LIVE pairing session file
//!    (`<pairing-root>/session-*/pairing.json`, the same files the HTTP
//!    API's session pairing writes). Pairing sessions delete their files
//!    on shutdown, so a token stops working when its session ends, and a
//!    session started after the server also works, because tokens are
//!    re-read on every `initialize`. Until a valid `initialize` succeeds,
//!    every request except `initialize` is refused with `-32001`. The
//!    token is never logged or echoed. There is no separate server token:
//!    the pairing token is the bearer token, so no second credential
//!    inventory exists.
//! 3. Strict exposure boundary: sealed AND private memories are never
//!    exposed. The storage layer already excludes sealed rows from lists
//!    and refuses sealed reads; this module additionally drops any search
//!    hit whose privacy is sealed or private, and refuses any `get` result
//!    whose privacy is sealed or private, even if a backend ever returned
//!    one. Only memories explicitly marked standard or public leave the
//!    process. A `get` on a missing, sealed, or private id produces the
//!    identical `Memory not found.` result, so the server is never an
//!    existence oracle for protected records. Stage derivation likewise
//!    ignores sealed artifacts.
//!
//! The engine is transport-agnostic (`serve` over `BufRead`/`Write`) so unit
//! tests drive it over in-memory buffers and the binary wires it to stdin /
//! stdout. One stdio process serves one client for its lifetime; there are
//! no sessions, no polling, no persistent connections. Search discovers
//! titles and snippets only (the core's validated title-search path);
//! `memory.get` is the only path that returns a body, and only for
//! standard/public records. `projects.search` additionally reports stage
//! metadata derived with the same `derive_pipeline` the Dashboard mirrors.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::lifecycle::MemoryState;
use crate::memory::{MemoryDetails, MemoryList, MemoryMetadata};
use crate::pipeline::{PipelineStatus, derive_pipeline};
use crate::privacy::Privacy;
use crate::readiness::{CommandError, ErrorCode};

/// Server name reported in `initialize` responses, per the revised spec.
pub const SERVER_NAME: &str = "anansi-server";
/// Protocol revisions this server negotiates (the same two the Slice 1
/// client negotiates). Anything else is a `-32602`.
pub const SUPPORTED_PROTOCOL_VERSIONS: [&str; 2] = ["2025-11-25", "2026-07-28"];
/// Maximum accepted request line, mirroring the Slice 1 client channel.
pub const MAX_LINE_BYTES: usize = 1024 * 1024;
/// Maximum query length in characters.
pub const MAX_QUERY_CHARS: usize = 500;
/// Default and maximum result limits.
pub const DEFAULT_LIMIT: u32 = 10;
pub const MAX_LIMIT: u32 = 50;
/// Tool-level "not found" text, identical for missing, sealed, and private ids.
pub const NOT_FOUND_TEXT: &str = "Memory not found.";

/// Capability surface exposed through the contracts (never carries a token).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpServerInfo {
    pub contract_version: u8,
    pub name: String,
    pub version: String,
    pub protocol_versions: Vec<String>,
    pub tools: Vec<String>,
}

/// One tool definition, as returned by `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpServerToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: McpToolInputSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: BTreeMap<String, McpSchemaProperty>,
    #[serde(default)]
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpSchemaProperty {
    #[serde(rename = "type")]
    pub property_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<u32>,
}

/// Launch-time server configuration (a contract DTO; never carries a token).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub contract_version: u8,
    pub enabled: bool,
    pub workspace_id: String,
    pub tools: Vec<String>,
    pub auth_method: String,
}

/// Runtime options for `serve`. Distinct from the contract DTO above.
#[derive(Debug, Clone)]
pub struct McpServerOptions {
    pub workspace_id: String,
    pub pairing_root: PathBuf,
}

/// Data access behind the protocol engine. The server layer owns policy
/// (privacy drops, kind filter, active-project filtering, allowlist,
/// stage derivation); backends own reads.
pub trait McpServerBackend {
    fn search_memories(
        &self,
        workspace_id: &str,
        query: &str,
        limit: u32,
        kind: Option<&str>,
    ) -> Result<Vec<MemoryMetadata>, CommandError>;
    fn read_memory(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<MemoryDetails, CommandError>;
    fn search_projects(
        &self,
        workspace_id: &str,
        query: &str,
        limit: u32,
    ) -> Result<Vec<MemoryMetadata>, CommandError>;
    fn project_artifact_types(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Vec<String>, CommandError>;
    fn project_artifacts(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Vec<ArtifactSummary>, CommandError>;
    fn create_idea_artifact(
        &self,
        workspace_id: &str,
        project_id: &str,
        title: &str,
        body: &str,
        privacy: &str,
    ) -> Result<String, CommandError>;
}

/// One artifact's summary metadata for the project tools. Bodies are
/// never carried here: summaries only, by construction.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactSummary {
    pub artifact_id: String,
    pub artifact_type: String,
    pub version: u32,
    pub title: String,
    pub timestamp: String,
    pub privacy: String,
}

/// Production backend: the same validated `Core::memory_command` /
/// `Core::artifacts_command` entries the HTTP API uses, so the server
/// inherits id/query validation and the storage layer's sealed handling
/// instead of reimplementing them.
pub struct CoreMcpServerBackend<'a> {
    core: &'a crate::Core,
}

impl<'a> CoreMcpServerBackend<'a> {
    pub fn new(core: &'a crate::Core) -> Self {
        Self { core }
    }
}

impl McpServerBackend for CoreMcpServerBackend<'_> {
    fn search_memories(
        &self,
        workspace_id: &str,
        query: &str,
        limit: u32,
        kind: Option<&str>,
    ) -> Result<Vec<MemoryMetadata>, CommandError> {
        let value = self.core.memory_command(
            "list",
            json!({"contractVersion": 1, "workspaceId": workspace_id, "query": query, "limit": limit, "kind": kind}),
        )?;
        let list: MemoryList = serde_json::from_value(value)
            .map_err(|_| CommandError::new(ErrorCode::StorageUnavailable, "Search failed."))?;
        Ok(list.memories)
    }

    fn read_memory(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<MemoryDetails, CommandError> {
        let value = self.core.memory_command(
            "read",
            json!({"contractVersion": 1, "workspaceId": workspace_id, "id": id}),
        )?;
        serde_json::from_value(value)
            .map_err(|_| CommandError::new(ErrorCode::StorageUnavailable, "Read failed."))
    }

    fn search_projects(
        &self,
        workspace_id: &str,
        query: &str,
        limit: u32,
    ) -> Result<Vec<MemoryMetadata>, CommandError> {
        let value = self.core.memory_command(
            "list",
            json!({"contractVersion": 1, "workspaceId": workspace_id, "kind": "project", "query": query, "limit": limit}),
        )?;
        let list: MemoryList = serde_json::from_value(value)
            .map_err(|_| CommandError::new(ErrorCode::StorageUnavailable, "Search failed."))?;
        Ok(list.memories)
    }

    fn project_artifact_types(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Vec<String>, CommandError> {
        let value = self.core.artifacts_command(
            "list",
            json!({"contractVersion": 1, "workspaceId": workspace_id, "projectId": project_id}),
        )?;
        let failed = || CommandError::new(ErrorCode::StorageUnavailable, "Search failed.");
        let artifacts = value
            .get("artifacts")
            .and_then(|a| a.as_array())
            .ok_or_else(failed)?;
        Ok(artifacts
            .iter()
            .filter_map(|a| {
                // Sealed artifacts never contribute to stage metadata.
                if a.get("privacy").and_then(|p| p.as_str()) == Some("sealed") {
                    return None;
                }
                a.get("type").and_then(|t| t.as_str()).map(|t| t.to_string())
            })
            .collect())
    }

    fn project_artifacts(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Vec<ArtifactSummary>, CommandError> {
        let value = self.core.artifacts_command(
            "list",
            json!({"contractVersion": 1, "workspaceId": workspace_id, "projectId": project_id}),
        )?;
        let failed = || CommandError::new(ErrorCode::StorageUnavailable, "Search failed.");
        let artifacts = value
            .get("artifacts")
            .and_then(|a| a.as_array())
            .ok_or_else(failed)?;
        // Malformed rows are skipped, never fatal: one corrupt artifact
        // must not hide every well-formed summary.
        Ok(artifacts
            .iter()
            .filter_map(|a| {
                Some(ArtifactSummary {
                    artifact_id: a.get("artifactId")?.as_str()?.to_string(),
                    artifact_type: a.get("type")?.as_str()?.to_string(),
                    version: a.get("version")?.as_u64()? as u32,
                    title: a.get("title")?.as_str()?.to_string(),
                    timestamp: a.get("timestamp")?.as_str()?.to_string(),
                    privacy: a.get("privacy").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                })
            })
            .collect())
    }

    fn create_idea_artifact(
        &self,
        workspace_id: &str,
        project_id: &str,
        title: &str,
        body: &str,
        privacy: &str,
    ) -> Result<String, CommandError> {
        // The assignment's `artifacts_command("create", ...)` does not
        // exist: `register` is the validated creation operation, and
        // the ideas module is provider-generation runs, not manual
        // creation. This is the creation path, with type pinned to
        // "idea" by the server layer, never by the caller.
        let value = self.core.artifacts_command(
            "register",
            json!({"contractVersion": 1, "workspaceId": workspace_id, "projectId": project_id, "type": "idea", "title": title, "body": body, "privacy": privacy}),
        )?;
        value
            .get("artifact")
            .and_then(|a| a.get("artifactId"))
            .and_then(|id| id.as_str())
            .map(|id| id.to_string())
            .ok_or_else(|| CommandError::new(ErrorCode::StorageUnavailable, "Creation failed."))
    }
}

/// The seven approved tools, in `tools/list` order.
pub fn tool_names() -> Vec<String> {
    vec![
        "memory.search".to_string(),
        "memory.get".to_string(),
        "notes.search".to_string(),
        "projects.search".to_string(),
        "experiments.list".to_string(),
        "products.get".to_string(),
        "ideas.create".to_string(),
    ]
}

/// Artifact types Product Forge registers for a project. `products.get`
/// summarizes exactly these: the product's definition, package,
/// deliverable, and review — not every artifact a project happens to
/// hold. There is no product entity in ANANSI; this set is the
/// documented derivation.
pub const PRODUCT_ARTIFACT_TYPES: [&str; 4] = [
    "specification",
    "source-package",
    "deliverable",
    "source-review",
];

/// The one artifact type LEXI-COLLIDER saves: `ai/collider-save`
/// registers `artifact_type: "experiment"` (crates/core/src/collider.rs).
/// SwarmTank `scenario` artifacts are not collider experiments.
pub const EXPERIMENT_ARTIFACT_TYPE: &str = "experiment";

pub fn server_info() -> McpServerInfo {
    McpServerInfo {
        contract_version: 1,
        name: SERVER_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_versions: SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .map(|v| v.to_string())
            .collect(),
        tools: tool_names(),
    }
}

fn search_schema(with_kind: bool) -> (BTreeMap<String, McpSchemaProperty>, Vec<String>) {
    let mut properties = BTreeMap::new();
    properties.insert(
        "query".to_string(),
        McpSchemaProperty {
            property_type: "string".to_string(),
            description: Some("Title search query.".to_string()),
            minimum: None,
            maximum: None,
        },
    );
    properties.insert(
        "limit".to_string(),
        McpSchemaProperty {
            property_type: "integer".to_string(),
            description: Some("Maximum results (default 10).".to_string()),
            minimum: Some(1),
            maximum: Some(MAX_LIMIT),
        },
    );
    if with_kind {
        properties.insert(
            "kind".to_string(),
            McpSchemaProperty {
                property_type: "string".to_string(),
                description: Some("Only memories of this kind.".to_string()),
                minimum: None,
                maximum: None,
            },
        );
    }
    (properties, vec!["query".to_string()])
}

pub fn tool_definitions() -> Vec<McpServerToolDef> {
    let (search_properties, search_required) = search_schema(true);
    let (project_properties, project_required) = search_schema(false);
    let mut id_properties = BTreeMap::new();
    id_properties.insert(
        "id".to_string(),
        McpSchemaProperty {
            property_type: "string".to_string(),
            description: Some("32-character lowercase hex memory ID.".to_string()),
            minimum: None,
            maximum: None,
        },
    );
    let mut project_id_properties = BTreeMap::new();
    project_id_properties.insert(
        "projectId".to_string(),
        McpSchemaProperty {
            property_type: "string".to_string(),
            description: Some("32-character lowercase hex project memory ID.".to_string()),
            minimum: None,
            maximum: None,
        },
    );
    project_id_properties.insert(
        "limit".to_string(),
        McpSchemaProperty {
            property_type: "integer".to_string(),
            description: Some("Maximum results (default 10).".to_string()),
            minimum: Some(1),
            maximum: Some(MAX_LIMIT),
        },
    );
    vec![
        McpServerToolDef {
            name: "memory.search".to_string(),
            description: "Search shareable (standard or public) workspace memory titles and snippets, optionally filtered by kind.".to_string(),
            input_schema: McpToolInputSchema {
                schema_type: "object".to_string(),
                properties: search_properties,
                required: search_required,
            },
        },
        McpServerToolDef {
            name: "memory.get".to_string(),
            description: "Retrieve one shareable (standard or public) memory by ID, including its body.".to_string(),
            input_schema: McpToolInputSchema {
                schema_type: "object".to_string(),
                properties: id_properties,
                required: vec!["id".to_string()],
            },
        },
        McpServerToolDef {
            name: "notes.search".to_string(),
            description: "Search shareable (standard or public) note titles and snippets. Same as memory.search with kind fixed to note.".to_string(),
            input_schema: McpToolInputSchema {
                schema_type: "object".to_string(),
                properties: search_schema(false).0,
                required: search_schema(false).1,
            },
        },
        McpServerToolDef {
            name: "projects.search".to_string(),
            description: "Search active shareable project titles, summaries, and stage metadata.".to_string(),
            input_schema: McpToolInputSchema {
                schema_type: "object".to_string(),
                properties: project_properties,
                required: project_required,
            },
        },
        McpServerToolDef {
            name: "experiments.list".to_string(),
            description: "List LEXI-COLLIDER collision experiment summaries (artifact type experiment) for an active shareable project. Sealed and private artifacts are excluded; bodies are never returned.".to_string(),
            input_schema: McpToolInputSchema {
                schema_type: "object".to_string(),
                properties: project_id_properties.clone(),
                required: vec!["projectId".to_string()],
            },
        },
        McpServerToolDef {
            name: "products.get".to_string(),
            description: "Retrieve Product Forge summary metadata (specification, source-package, deliverable, source-review artifacts) for an active shareable project. Sealed and private artifacts are excluded; bodies are never returned.".to_string(),
            input_schema: McpToolInputSchema {
                schema_type: "object".to_string(),
                properties: project_id_properties,
                required: vec!["projectId".to_string()],
            },
        },
        McpServerToolDef {
            name: "ideas.create".to_string(),
            description: "Create one idea artifact bound strictly to an active shareable project. The project must exist, be active, non-expired, and standard or public; anything else returns 'Project not found.'. The artifact's privacy mirrors the project's. The artifact schema has no tags column, so tags, when supplied, are appended to the stored body as a final 'Tags: ...' line with the description verbatim before it.".to_string(),
            input_schema: McpToolInputSchema {
                schema_type: "object".to_string(),
                properties: idea_properties(),
                required: vec!["projectId".to_string(), "title".to_string(), "description".to_string()],
            },
        },
    ]
}

fn idea_properties() -> BTreeMap<String, McpSchemaProperty> {
    let mut properties = BTreeMap::new();
    properties.insert(
        "projectId".to_string(),
        McpSchemaProperty {
            property_type: "string".to_string(),
            description: Some("32-character lowercase hex project memory ID.".to_string()),
            minimum: None,
            maximum: None,
        },
    );
    properties.insert(
        "title".to_string(),
        McpSchemaProperty {
            property_type: "string".to_string(),
            description: Some("Idea title, 1 to 120 characters.".to_string()),
            minimum: None,
            maximum: None,
        },
    );
    properties.insert(
        "description".to_string(),
        McpSchemaProperty {
            property_type: "string".to_string(),
            description: Some("Idea description, stored verbatim as the artifact body.".to_string()),
            minimum: None,
            maximum: None,
        },
    );
    properties.insert(
        "tags".to_string(),
        McpSchemaProperty {
            property_type: "array".to_string(),
            description: Some("Optional tags, at most 32 of up to 64 characters each; appended to the body as a final 'Tags: ...' line.".to_string()),
            minimum: None,
            maximum: None,
        },
    );
    properties
}

/// Byte-wise comparison without early exit on first difference, so a token
/// guess cannot be timed character by character.
fn tokens_equal(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// Tokens of currently live pairing sessions: every
/// `<pairing-root>/session-*/pairing.json` token. Unreadable or malformed
/// files are skipped, never fatal — a session that cannot be read cannot
/// authenticate, and an empty result means no live session at all.
pub fn live_pairing_tokens(pairing_root: &std::path::Path) -> Vec<String> {
    let mut tokens = Vec::new();
    let Ok(entries) = fs::read_dir(pairing_root) else {
        return tokens;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with("session-") {
            continue;
        }
        let Ok(text) = fs::read_to_string(entry.path().join("pairing.json")) else {
            continue;
        };
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            if let Some(token) = value.get("token").and_then(|t| t.as_str()) {
                tokens.push(token.to_string());
            }
        }
    }
    tokens
}

/// A memory is shareable through the server only when explicitly marked
/// standard or public. Sealed and private records never leave the process.
fn is_shareable(privacy: Privacy) -> bool {
    matches!(privacy, Privacy::Standard | Privacy::Public)
}

fn error_response(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn result_response(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn tool_result(text: &str, structured: Value, is_error: bool) -> Value {
    json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": is_error,
    })
}

fn invalid(message: &str) -> (i64, String) {
    (-32602, message.to_string())
}

fn parse_limit(arguments: &Value) -> Result<u32, (i64, String)> {
    match arguments.get("limit") {
        None => Ok(DEFAULT_LIMIT),
        Some(value) => {
            let n = value.as_u64().filter(|n| *n >= 1 && *n <= u64::from(MAX_LIMIT));
            match n {
                Some(n) => Ok(n as u32),
                None => Err(invalid("limit must be an integer between 1 and 50.")),
            }
        }
    }
}

fn parse_query(arguments: &Value) -> Result<(String, u32), (i64, String)> {
    let query = arguments
        .get("query")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .ok_or_else(|| invalid("query is required and must be a non-empty string."))?;
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(invalid("query must be at most 500 characters."));
    }
    let limit = parse_limit(arguments)?;
    Ok((query.to_string(), limit))
}

fn parse_kind(arguments: &Value) -> Result<Option<String>, (i64, String)> {
    let Some(kind) = arguments.get("kind") else {
        return Ok(None);
    };
    let kind = kind
        .as_str()
        .filter(|k| crate::entities::is_valid_kind(k))
        .ok_or_else(|| invalid("kind must be a supported memory kind."))?;
    Ok(Some(kind.to_string()))
}

fn parse_id(arguments: &Value) -> Result<String, (i64, String)> {
    parse_id_field(arguments, "id")
}

fn parse_id_field(arguments: &Value, field: &str) -> Result<String, (i64, String)> {
    let id = arguments
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| invalid(&format!("{field} is required and must be a string.")))?;
    crate::workspace::validate_id(id)
        .map_err(|_| invalid(&format!("{field} must be a 32-character lowercase hex string.")))?;
    Ok(id.to_string())
}

/// Artifact privacy arrives as a storage string, not the enum: only
/// the two shareable spellings pass.
fn is_shareable_str(privacy: &str) -> bool {
    matches!(privacy, "standard" | "public")
}

/// A project whose validity window has closed is not active. An
/// unparseable expiry is treated as expired: fail closed on corrupt
/// data rather than serving a record whose lifetime is unknown.
fn is_expired(valid_until: &Option<String>) -> bool {
    let Some(until) = valid_until else {
        return false;
    };
    match chrono::DateTime::parse_from_rfc3339(until) {
        Ok(end) => end < chrono::Utc::now(),
        Err(_) => true,
    }
}

fn search_hit(meta: &MemoryMetadata) -> Value {
    json!({
        "id": meta.id,
        "title": meta.title,
        "snippet": meta.snippet,
        "kind": meta.kind,
        "status": meta.status,
    })
}

fn is_active_project(status: MemoryState) -> bool {
    !matches!(status, MemoryState::Superseded | MemoryState::Archived)
}

fn stage_metadata(artifact_types: &[String]) -> Value {
    let refs: Vec<&str> = artifact_types.iter().map(|t| t.as_str()).collect();
    let derivation = derive_pipeline(&refs);
    let stages: Vec<Value> = derivation
        .stages
        .iter()
        .map(|s| {
            let status = match s.status {
                PipelineStatus::Complete => "complete",
                PipelineStatus::Current => "current",
                PipelineStatus::Available => "available",
                PipelineStatus::Awaiting => "awaiting",
            };
            json!({"id": s.stage.id(), "label": s.stage.label(), "status": status})
        })
        .collect();
    json!({
        "stage": derivation.current.map(|s| s.id()),
        "stageLabel": derivation.current.map(|s| s.label()),
        "stages": stages,
    })
}

/// Run the server loop until stdin closes. Returns Ok on clean EOF; a
/// malformed workspace id or a pairing root with no live session fails
/// before reading anything.
pub fn serve<B: McpServerBackend>(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    backend: &B,
    options: &McpServerOptions,
) -> Result<(), CommandError> {
    crate::workspace::validate_id(&options.workspace_id).map_err(|_| {
        CommandError::new(ErrorCode::InvalidWorkspace, "Invalid workspace id.")
    })?;
    if live_pairing_tokens(&options.pairing_root).is_empty() {
        return Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "MCP server requires a live pairing session.",
        ));
    }
    let mut authenticated = false;
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line).map_err(|_| {
            CommandError::new(ErrorCode::StorageUnavailable, "Failed to read request.")
        })?;
        if read == 0 {
            return Ok(());
        }
        if line.len() > MAX_LINE_BYTES {
            write_value(
                writer,
                &error_response(&Value::Null, -32700, "Request line is too long."),
            )?;
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => {
                write_value(
                    writer,
                    &error_response(&Value::Null, -32700, "Parse error."),
                )?;
                continue;
            }
        };
        let object = match message.as_object() {
            Some(object) => object,
            None => {
                write_value(
                    writer,
                    &error_response(&Value::Null, -32600, "Invalid request."),
                )?;
                continue;
            }
        };
        // A message without an `id` is a notification: never answered,
        // even before authentication.
        let Some(id) = object.get("id") else {
            continue;
        };
        let id = id.clone();
        let method = match object.get("method").and_then(|v| v.as_str()) {
            Some(method) => method,
            None => {
                write_value(
                    writer,
                    &error_response(&id, -32600, "Invalid request."),
                )?;
                continue;
            }
        };
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        let response = handle_request(method, &params, &id, backend, options, &mut authenticated);
        write_value(writer, &response)?;
    }
}

fn write_value(writer: &mut impl Write, value: &Value) -> Result<(), CommandError> {
    let text = serde_json::to_string(value)
        .map_err(|_| CommandError::new(ErrorCode::StorageUnavailable, "Failed to encode response."))?;
    writer.write_all(text.as_bytes()).map_err(|_| {
        CommandError::new(ErrorCode::StorageUnavailable, "Failed to write response.")
    })?;
    writer.write_all(b"\n").map_err(|_| {
        CommandError::new(ErrorCode::StorageUnavailable, "Failed to write response.")
    })?;
    writer.flush().map_err(|_| {
        CommandError::new(ErrorCode::StorageUnavailable, "Failed to flush response.")
    })?;
    Ok(())
}

fn handle_request<B: McpServerBackend>(
    method: &str,
    params: &Value,
    id: &Value,
    backend: &B,
    options: &McpServerOptions,
    authenticated: &mut bool,
) -> Value {
    if method == "initialize" {
        let version = params
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !SUPPORTED_PROTOCOL_VERSIONS.contains(&version) {
            return error_response(id, -32602, "Unsupported protocol version.");
        }
        let presented = params
            .get("_meta")
            .and_then(|m| m.get("anansiMcpToken"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // Tokens are re-read on every initialize: a session started after
        // the server works, and an ended session's token stops working.
        let live = live_pairing_tokens(&options.pairing_root);
        if !live.iter().any(|t| tokens_equal(presented, t)) {
            return error_response(
                id,
                -32001,
                "Unauthorized: invalid or missing pairing token.",
            );
        }
        *authenticated = true;
        let info = server_info();
        return result_response(
            id,
            json!({
                "protocolVersion": version,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": info.name, "version": info.version},
            }),
        );
    }
    if !*authenticated {
        return error_response(
            id,
            -32001,
            "Unauthorized: initialize with a valid pairing token first.",
        );
    }
    match method {
        "tools/list" => result_response(
            id,
            json!({"tools": tool_definitions()}),
        ),
        "ping" => result_response(id, json!({})),
        "tools/call" => handle_tool_call(params, id, backend, options),
        _ => error_response(id, -32601, "Method not found."),
    }
}

fn handle_tool_call<B: McpServerBackend>(
    params: &Value,
    id: &Value,
    backend: &B,
    options: &McpServerOptions,
) -> Value {
    let object = match params.as_object() {
        Some(object) => object,
        None => return error_response(id, -32602, "params must be an object."),
    };
    let name = match object.get("name").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => return error_response(id, -32602, "name is required and must be a string."),
    };
    // The allowlist is enforced before any backend call.
    if !tool_names().iter().any(|t| t == name) {
        return error_response(id, -32602, &format!("Unknown tool '{name}'."));
    }
    let arguments = object
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let arguments = match arguments.as_object() {
        Some(_) => arguments,
        None => return error_response(id, -32602, "arguments must be an object."),
    };
    let result = match name {
        "memory.search" => call_search(&arguments, backend, options, SearchMode::Memories),
        "notes.search" => call_search(&arguments, backend, options, SearchMode::Notes),
        "projects.search" => call_search(&arguments, backend, options, SearchMode::Projects),
        "experiments.list" => {
            call_project_tool(&arguments, backend, options, ProjectToolKind::Experiments)
        }
        "products.get" => {
            call_project_tool(&arguments, backend, options, ProjectToolKind::Products)
        }
        "ideas.create" => call_ideas_create(&arguments, backend, options),
        "memory.get" => call_get(&arguments, backend, options),
        _ => unreachable!("allowlist checked above"),
    };
    match result {
        Ok(tool_value) => result_response(id, tool_value),
        Err((code, message)) => error_response(id, code, &message),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchMode {
    Memories,
    Notes,
    Projects,
}

fn call_search<B: McpServerBackend>(
    arguments: &Value,
    backend: &B,
    options: &McpServerOptions,
    mode: SearchMode,
) -> Result<Value, (i64, String)> {
    let (query, limit) = parse_query(arguments)?;
    let kind = match mode {
        SearchMode::Projects => None,
        SearchMode::Memories => parse_kind(arguments)?,
        // notes.search fixes kind to note. A contradictory kind
        // argument is a caller error, not something to ignore.
        SearchMode::Notes => match parse_kind(arguments)? {
            None => Some("note".to_string()),
            Some(kind) if kind == "note" => Some(kind),
            Some(_) => {
                return Err(invalid(
                    "notes.search searches notes only; kind must be \"note\" or absent.",
                ));
            }
        },
    };
    let hits = if mode == SearchMode::Projects {
        backend.search_projects(&options.workspace_id, &query, limit)
    } else {
        backend.search_memories(&options.workspace_id, &query, limit, kind.as_deref())
    }
    .map_err(|_| (-32603, "Tool execution failed.".to_string()))?;
    let mut rendered = Vec::new();
    for meta in &hits {
        // Strict exposure boundary: sealed AND private rows never leave
        // the process, even if a backend returned them.
        if !is_shareable(meta.privacy) {
            continue;
        }
        if mode == SearchMode::Projects {
            if !is_active_project(meta.status) {
                continue;
            }
            let types = backend
                .project_artifact_types(&options.workspace_id, &meta.id)
                .map_err(|_| (-32603, "Tool execution failed.".to_string()))?;
            let mut hit = search_hit(meta);
            let stage = stage_metadata(&types);
            hit["stage"] = stage["stage"].clone();
            hit["stageLabel"] = stage["stageLabel"].clone();
            hit["stages"] = stage["stages"].clone();
            rendered.push(hit);
        } else {
            // Kind is enforced here as well as in the backend, so the
            // filter holds even if a backend ignores it.
            if let Some(kind) = &kind {
                if meta.kind != *kind {
                    continue;
                }
            }
            rendered.push(search_hit(meta));
        }
    }
    let array = Value::Array(rendered);
    Ok(tool_result(
        &serde_json::to_string(&array).unwrap_or_else(|_| "[]".to_string()),
        array,
        false,
    ))
}

const PROJECT_NOT_FOUND_TEXT: &str = "Project not found.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectToolKind {
    Experiments,
    Products,
}

fn call_project_tool<B: McpServerBackend>(
    arguments: &Value,
    backend: &B,
    options: &McpServerOptions,
    kind: ProjectToolKind,
) -> Result<Value, (i64, String)> {
    let project_id = parse_id_field(arguments, "projectId")?;
    let limit = parse_limit(arguments)?;
    let not_found = || {
        tool_result(
            PROJECT_NOT_FOUND_TEXT,
            json!({"error": PROJECT_NOT_FOUND_TEXT}),
            true,
        )
    };
    // Any read failure maps to the same not-found result: no oracle
    // distinguishing unknown, sealed, private, inactive, and expired
    // projects. This mirrors memory.get's treatment of protected rows.
    let meta = match backend.read_memory(&options.workspace_id, &project_id) {
        Ok(details) => details.memory,
        Err(_) => return Ok(not_found()),
    };
    if meta.kind != "project"
        || !is_active_project(meta.status)
        || !is_shareable(meta.privacy)
        || is_expired(&meta.valid_until)
    {
        return Ok(not_found());
    }
    let wanted: &[&str] = match kind {
        ProjectToolKind::Experiments => &[EXPERIMENT_ARTIFACT_TYPE],
        ProjectToolKind::Products => &PRODUCT_ARTIFACT_TYPES,
    };
    let artifacts = backend
        .project_artifacts(&options.workspace_id, &project_id)
        .map_err(|_| (-32603, "Tool execution failed.".to_string()))?;
    let mut rendered = Vec::new();
    for artifact in &artifacts {
        if !wanted.contains(&artifact.artifact_type.as_str()) {
            continue;
        }
        // Sealed AND private artifacts never leave the process, even
        // if a backend returned them.
        if !is_shareable_str(&artifact.privacy) {
            continue;
        }
        rendered.push(json!({
            "artifactId": artifact.artifact_id,
            "type": artifact.artifact_type,
            "title": artifact.title,
            "version": artifact.version,
            "timestamp": artifact.timestamp,
        }));
        if rendered.len() >= limit as usize {
            break;
        }
    }
    let value = match kind {
        ProjectToolKind::Experiments => json!({
            "projectId": project_id,
            "projectTitle": meta.title,
            "experiments": rendered,
        }),
        ProjectToolKind::Products => json!({
            "projectId": project_id,
            "projectTitle": meta.title,
            "products": rendered,
        }),
    };
    Ok(tool_result(
        &serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string()),
        value,
        false,
    ))
}

/// The mutation gate for `ideas.create`: argument validation, then the
/// same project gate as the read-only project tools, then a single
/// `register` of a type-`idea` artifact bound to the validated project.
/// Project gate failures return the identical not-found result as the
/// read tools (no oracle); register *validation* failures surface as
/// -32602, anything else as -32603.
fn call_ideas_create<B: McpServerBackend>(
    arguments: &Value,
    backend: &B,
    options: &McpServerOptions,
) -> Result<Value, (i64, String)> {
    let project_id = parse_id_field(arguments, "projectId")?;
    let title = arguments
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|t| !t.is_empty() && t.chars().count() <= 120)
        .ok_or_else(|| invalid("title is required and must be 1 to 120 characters."))?;
    let description = arguments
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .ok_or_else(|| invalid("description is required and must be a non-empty string."))?;
    let tags = match arguments.get("tags") {
        None => Vec::new(),
        Some(value) => {
            let items = value
                .as_array()
                .ok_or_else(|| invalid("tags must be an array of strings."))?;
            if items.len() > 32 {
                return Err(invalid("tags must contain at most 32 entries."));
            }
            let mut tags = Vec::new();
            for item in items {
                let tag = item
                    .as_str()
                    .map(str::trim)
                    .filter(|t| !t.is_empty() && t.chars().count() <= 64)
                    .ok_or_else(|| invalid("each tag must be 1 to 64 characters."))?;
                tags.push(tag.to_string());
            }
            tags
        }
    };
    let not_found = || {
        tool_result(
            PROJECT_NOT_FOUND_TEXT,
            json!({"error": PROJECT_NOT_FOUND_TEXT}),
            true,
        )
    };
    let meta = match backend.read_memory(&options.workspace_id, &project_id) {
        Ok(details) => details.memory,
        Err(_) => return Ok(not_found()),
    };
    if meta.kind != "project"
        || !is_active_project(meta.status)
        || !is_shareable(meta.privacy)
        || is_expired(&meta.valid_until)
    {
        return Ok(not_found());
    }
    // The artifact is visible exactly where its project is visible.
    let privacy = match meta.privacy {
        Privacy::Standard => "standard",
        Privacy::Public => "public",
        _ => unreachable!("project gate passed above"),
    };
    // No tags column exists on artifacts: tags ride as a final body
    // line, with the description verbatim before it.
    let mut body = description.to_string();
    if !tags.is_empty() {
        body.push_str("\n\nTags: ");
        body.push_str(&tags.join(", "));
    }
    let artifact_id = backend
        .create_idea_artifact(&options.workspace_id, &project_id, title, &body, privacy)
        .map_err(|err| {
            if err.code == ErrorCode::InvalidRequest {
                (-32602, err.message)
            } else {
                (-32603, "Tool execution failed.".to_string())
            }
        })?;
    let value = json!({
        "projectId": project_id,
        "projectTitle": meta.title,
        "artifactId": artifact_id,
        "type": "idea",
        "title": title,
        "tags": tags,
    });
    Ok(tool_result(
        &serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string()),
        value,
        false,
    ))
}

fn call_get<B: McpServerBackend>(
    arguments: &Value,
    backend: &B,
    options: &McpServerOptions,
) -> Result<Value, (i64, String)> {
    let id = parse_id(arguments)?;
    let found = backend.read_memory(&options.workspace_id, &id);
    // Missing, sealed, and private produce the identical result: no
    // existence oracle for protected records.
    let details = match found {
        Ok(details) if is_shareable(details.memory.privacy) => details,
        _ => {
            let text = NOT_FOUND_TEXT;
            return Ok(tool_result(
                text,
                json!({"error": text}),
                true,
            ));
        }
    };
    let meta = &details.memory;
    let value = json!({
        "id": meta.id,
        "title": meta.title,
        "snippet": meta.snippet,
        "kind": meta.kind,
        "status": meta.status,
        "body": details.body,
    });
    Ok(tool_result(
        &serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string()),
        value,
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::RefCell;
    use std::io::Cursor;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const WORKSPACE: &str = "0123456789abcdef0123456789abcdef";

    /// A pairing root containing one live session with TOKEN. The caller
    /// keeps the tempdir alive for the duration of the test.
    fn paired_root() -> (tempfile::TempDir, McpServerOptions) {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("session-test");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("pairing.json"),
            json!({"endpoint": "http://127.0.0.1:1451", "token": TOKEN, "contractVersion": 1}).to_string(),
        )
        .unwrap();
        let root = temp.path().to_path_buf();
        (
            temp,
            McpServerOptions {
                workspace_id: WORKSPACE.to_string(),
                pairing_root: root,
            },
        )
    }

    fn meta(id: &str, title: &str, kind: &str, status: MemoryState, privacy: Privacy) -> MemoryMetadata {
        MemoryMetadata {
            id: id.to_string(),
            workspace_id: WORKSPACE.to_string(),
            project_id: None,
            kind: kind.to_string(),
            revision_id: "0123456789abcdef0123456789abcdef".to_string(),
            version: 1,
            title: title.to_string(),
            created_at: "2026-11-04T00:00:00Z".to_string(),
            updated_at: "2026-11-04T00:00:00Z".to_string(),
            event_date: None,
            valid_from: None,
            valid_until: None,
            confidence: None,
            status,
            archived_state: None,
            superseded_by: None,
            restored_from: None,
            privacy,
            provenance: "test".to_string(),
            source_references: vec![],
            generator_model: None,
            validation_status: "ok".to_string(),
            relationships: vec![],
            supersedes: None,
            due_date: None,
            completed: None,
            snippet: Some(format!("{title} snippet")),
            section_heading: None,
            line_number: None,
            duplicate_of: None,
            search_score: None,
        }
    }

    struct FakeBackend {
        calls: RefCell<Vec<String>>,
        created: RefCell<Vec<(String, String, String, String)>>,
    }

    impl FakeBackend {
        fn new() -> Self {
            Self {
                calls: RefCell::new(vec![]),
                created: RefCell::new(vec![]),
            }
        }
    }

    impl McpServerBackend for FakeBackend {
        fn search_memories(
            &self,
            _workspace_id: &str,
            query: &str,
            _limit: u32,
            _kind: Option<&str>,
        ) -> Result<Vec<MemoryMetadata>, CommandError> {
            // Deliberately ignores `kind`: the server-side filter is what
            // the tests assert.
            self.calls.borrow_mut().push("search".to_string());
            if query == "sealed-only" {
                return Ok(vec![meta(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "Sealed note",
                    "note",
                    MemoryState::Durable,
                    Privacy::Sealed,
                )]);
            }
            Ok(vec![
                meta(
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "Visible note",
                    "note",
                    MemoryState::Durable,
                    Privacy::Standard,
                ),
                meta(
                    "cccccccccccccccccccccccccccccccc",
                    "Private note",
                    "note",
                    MemoryState::Durable,
                    Privacy::Private,
                ),
                meta(
                    "dddddddddddddddddddddddddddddddd",
                    "Task record",
                    "task",
                    MemoryState::Durable,
                    Privacy::Standard,
                ),
            ])
        }

        fn read_memory(
            &self,
            _workspace_id: &str,
            id: &str,
        ) -> Result<MemoryDetails, CommandError> {
            self.calls.borrow_mut().push("get".to_string());
            let (title, kind, status, privacy, body) = match id {
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" => ("Sealed note", "note", MemoryState::Durable, Privacy::Sealed, "secret body"),
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" => ("Visible note", "note", MemoryState::Durable, Privacy::Standard, "visible body"),
                "cccccccccccccccccccccccccccccccc" => ("Private note", "note", MemoryState::Durable, Privacy::Private, "private body"),
                "33333333333333333333333333333333" => ("Active project", "project", MemoryState::Durable, Privacy::Standard, "project body"),
                "44444444444444444444444444444444" => ("Private project", "project", MemoryState::Durable, Privacy::Private, "project body"),
                "55555555555555555555555555555555" => ("Old project", "project", MemoryState::Superseded, Privacy::Standard, "project body"),
                "66666666666666666666666666666666" => ("Some note", "note", MemoryState::Durable, Privacy::Standard, "note body"),
                "88888888888888888888888888888888" => ("Expired project", "project", MemoryState::Durable, Privacy::Standard, "project body"),
                _ => return Err(CommandError::new(ErrorCode::NotFound, "missing")),
            };
            let mut memory = meta(id, title, kind, status, privacy);
            // The expired project: a validity window that closed in
            // 2020, so the gate must refuse it like the other
            // non-active targets.
            if id == "88888888888888888888888888888888" {
                memory.valid_until = Some("2020-01-01T00:00:00Z".to_string());
            }
            Ok(MemoryDetails {
                contract_version: 1,
                memory,
                body: body.to_string(),
            })
        }

        fn search_projects(
            &self,
            _workspace_id: &str,
            _query: &str,
            _limit: u32,
        ) -> Result<Vec<MemoryMetadata>, CommandError> {
            self.calls.borrow_mut().push("projects".to_string());
            Ok(vec![
                meta(
                    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                    "Active project",
                    "project",
                    MemoryState::Durable,
                    Privacy::Standard,
                ),
                meta(
                    "ffffffffffffffffffffffffffffffff",
                    "Private project",
                    "project",
                    MemoryState::Durable,
                    Privacy::Private,
                ),
                meta(
                    "11111111111111111111111111111111",
                    "Old project",
                    "project",
                    MemoryState::Superseded,
                    Privacy::Standard,
                ),
                meta(
                    "22222222222222222222222222222222",
                    "Archived project",
                    "project",
                    MemoryState::Archived,
                    Privacy::Standard,
                ),
            ])
        }

        fn project_artifact_types(
            &self,
            _workspace_id: &str,
            project_id: &str,
        ) -> Result<Vec<String>, CommandError> {
            self.calls
                .borrow_mut()
                .push(format!("artifact_types:{project_id}"));
            // Only the active project carries a concept: its pipeline
            // current stage is Build.
            if project_id == "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" {
                Ok(vec!["concept".to_string()])
            } else {
                Ok(vec![])
            }
        }

        fn project_artifacts(
            &self,
            _workspace_id: &str,
            project_id: &str,
        ) -> Result<Vec<ArtifactSummary>, CommandError> {
            self.calls
                .borrow_mut()
                .push(format!("artifacts:{project_id}"));
            if project_id != "33333333333333333333333333333333" {
                return Ok(vec![]);
            }
            let summary = |id: &str, artifact_type: &str, title: &str, privacy: &str| {
                ArtifactSummary {
                    artifact_id: id.to_string(),
                    artifact_type: artifact_type.to_string(),
                    version: 1,
                    title: title.to_string(),
                    timestamp: "2026-11-04T00:00:00Z".to_string(),
                    privacy: privacy.to_string(),
                }
            };
            Ok(vec![
                summary("art-experiment", "experiment", "Collision run", "standard"),
                summary("art-private-experiment", "experiment", "Hidden run", "private"),
                summary("art-sealed-experiment", "experiment", "Sealed run", "sealed"),
                summary("art-deliverable", "deliverable", "Shipped build", "standard"),
                summary("art-spec", "specification", "Product spec", "standard"),
                summary("art-scenario", "scenario", "Swarm branch", "standard"),
            ])
        }

        fn create_idea_artifact(
            &self,
            _workspace_id: &str,
            project_id: &str,
            title: &str,
            body: &str,
            privacy: &str,
        ) -> Result<String, CommandError> {
            self.created.borrow_mut().push((
                project_id.to_string(),
                title.to_string(),
                body.to_string(),
                privacy.to_string(),
            ));
            Ok("art-idea-1".to_string())
        }
    }

    /// Drive `serve` over one scripted input and return parsed output lines.
    fn run(input: &str, backend: &FakeBackend, options: &McpServerOptions) -> Vec<Value> {
        let mut reader = Cursor::new(input.as_bytes().to_vec());
        let mut out: Vec<u8> = vec![];
        serve(&mut reader, &mut out, backend, options).unwrap();
        String::from_utf8(out)
            .unwrap()
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn initialize(version: &str, token: Option<&str>) -> Value {
        let mut params = json!({"protocolVersion": version, "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}});
        if let Some(token) = token {
            params["_meta"] = json!({"anansiMcpToken": token});
        }
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": params})
    }

    fn authed_script(rest: &str) -> String {
        format!("{}\n{rest}\n", initialize("2025-11-25", Some(TOKEN)))
    }

    #[test]
    fn handshake_accepts_both_protocol_versions_and_reports_revised_name() {
        for version in SUPPORTED_PROTOCOL_VERSIONS {
            let (_temp, options) = paired_root();
            let backend = FakeBackend::new();
            let out = run(&format!("{}\n", initialize(version, Some(TOKEN))), &backend, &options);
            assert_eq!(out.len(), 1);
            assert_eq!(out[0]["result"]["protocolVersion"], version);
            assert_eq!(out[0]["result"]["serverInfo"]["name"], "anansi-server");
            assert!(out[0]["result"]["capabilities"]["tools"].is_object());
        }
    }

    #[test]
    fn handshake_rejects_unsupported_and_missing_versions() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let out = run(
            &format!(
                "{}\n{}\n",
                initialize("2024-01-01", Some(TOKEN)),
                json!({"jsonrpc": "2.0", "id": 2, "method": "initialize", "params": {"capabilities": {}, "clientInfo": {}}}),
            ),
            &backend,
            &options,
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["error"]["code"], -32602);
        assert_eq!(out[1]["error"]["code"], -32602);
    }

    #[test]
    fn pairing_gate_refuses_then_accepts() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let out = run(
            &format!(
                "{}\n{}\n{}\n{}\n",
                json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
                initialize("2025-11-25", Some("0000000000000000000000000000000000000000000000000000000000000000")),
                initialize("2025-11-25", None),
                initialize("2025-11-25", Some(TOKEN)),
            ),
            &backend,
            &options,
        );
        assert_eq!(out.len(), 4);
        assert_eq!(out[0]["error"]["code"], -32001);
        assert_eq!(out[1]["error"]["code"], -32001);
        assert_eq!(out[2]["error"]["code"], -32001);
        assert!(out[3]["result"]["serverInfo"].is_object());
    }

    #[test]
    fn tools_list_returns_exactly_seven_tools() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let out = run(
            &authed_script(
                &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}).to_string(),
            ),
            &backend,
            &options,
        );
        let names: Vec<&str> = out[1]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "memory.search",
                "memory.get",
                "notes.search",
                "projects.search",
                "experiments.list",
                "products.get",
                "ideas.create",
            ]
        );
    }

    #[test]
    fn slice4_tools_cover_notes_experiments_products_and_gating() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let script = authed_script(&format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "notes.search", "arguments": {"query": "visible"}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "notes.search", "arguments": {"query": "visible", "kind": "task"}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "experiments.list", "arguments": {"projectId": "33333333333333333333333333333333"}}}),
            json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "products.get", "arguments": {"projectId": "33333333333333333333333333333333"}}}),
            json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": {"name": "experiments.list", "arguments": {"projectId": "44444444444444444444444444444444"}}}),
            json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": "experiments.list", "arguments": {"projectId": "55555555555555555555555555555555"}}}),
            json!({"jsonrpc": "2.0", "id": 8, "method": "tools/call", "params": {"name": "experiments.list", "arguments": {"projectId": "66666666666666666666666666666666"}}}),
            json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {"name": "experiments.list", "arguments": {"projectId": "77777777777777777777777777777777"}}}),
            json!({"jsonrpc": "2.0", "id": 10, "method": "tools/call", "params": {"name": "experiments.list", "arguments": {"projectId": "not-hex"}}}),
        ));
        let out = run(&script, &backend, &options);
        assert_eq!(out.len(), 10);
        // notes.search: kind enforced server-side (fake ignores it) and
        // the private note dropped: only the standard note remains.
        let notes = &out[1]["result"]["structuredContent"];
        let titles: Vec<&str> = notes
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Visible note"]);
        // A contradictory kind is a caller error, not an ignored field.
        assert_eq!(out[2]["error"]["code"], -32602);
        // experiments.list: only the standard experiment survives —
        // private and sealed experiments dropped, and deliverable,
        // specification, and scenario rows are not experiments.
        let experiments = &out[3]["result"];
        assert_eq!(experiments["isError"], false);
        assert_eq!(experiments["structuredContent"]["projectTitle"], "Active project");
        let titles: Vec<&str> = experiments["structuredContent"]["experiments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Collision run"]);
        // products.get: the Forge set only (deliverable + specification),
        // scenario excluded, bodies never carried.
        let products = &out[4]["result"];
        assert_eq!(products["isError"], false);
        let titles: Vec<&str> = products["structuredContent"]["products"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Shipped build", "Product spec"]);
        assert!(products["structuredContent"]["products"][0].get("body").is_none());
        // Private, superseded, wrong-kind, and unknown projects all
        // produce the identical not-found result: no oracle.
        for response in &out[5..9] {
            assert_eq!(response["result"]["isError"], true);
            assert_eq!(response["result"]["content"][0]["text"], PROJECT_NOT_FOUND_TEXT);
        }
        assert_eq!(out[5]["result"], out[6]["result"]);
        assert_eq!(out[5]["result"], out[7]["result"]);
        assert_eq!(out[5]["result"], out[8]["result"]);
        // A malformed project id is a parameter error, not not-found.
        assert_eq!(out[9]["error"]["code"], -32602);
        // Artifact listing ran only for the surviving active project
        // (once per project tool call: experiments.list and products.get).
        let artifact_calls: Vec<String> = backend
            .calls
            .borrow()
            .iter()
            .filter(|c| c.starts_with("artifacts:"))
            .cloned()
            .collect();
        assert_eq!(
            artifact_calls,
            vec![
                "artifacts:33333333333333333333333333333333",
                "artifacts:33333333333333333333333333333333",
            ]
        );
    }

    #[test]
    fn slice5_ideas_create_validates_gates_and_creates_once() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let long_title = "t".repeat(121);
        let long_tag = "g".repeat(65);
        let many_tags: Vec<Value> = (0..33).map(|i| json!(format!("tag{i}"))).collect();
        let calls = vec![
            // Happy path: tags ride as a final body line.
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "33333333333333333333333333333333", "title": "Solar kiosk", "description": "A kiosk idea.", "tags": ["solar", "kiosk"]}}}),
            // Every project-gate failure is the identical not-found.
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "44444444444444444444444444444444", "title": "Hidden", "description": "No."}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "55555555555555555555555555555555", "title": "Old", "description": "No."}}}),
            json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "66666666666666666666666666666666", "title": "Note", "description": "No."}}}),
            json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "88888888888888888888888888888888", "title": "Expired", "description": "No."}}}),
            json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "99999999999999999999999999999999", "title": "Unknown", "description": "No."}}}),
            // Argument validation: all -32602, all before any write.
            json!({"jsonrpc": "2.0", "id": 8, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "not-hex", "title": "Bad id", "description": "No."}}}),
            json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "33333333333333333333333333333333", "description": "No title."}}}),
            json!({"jsonrpc": "2.0", "id": 10, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "33333333333333333333333333333333", "title": "   ", "description": "Blank title."}}}),
            json!({"jsonrpc": "2.0", "id": 11, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "33333333333333333333333333333333", "title": long_title, "description": "Overlong title."}}}),
            json!({"jsonrpc": "2.0", "id": 12, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "33333333333333333333333333333333", "title": "No desc."}}}),
            json!({"jsonrpc": "2.0", "id": 13, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "33333333333333333333333333333333", "title": "Blank desc.", "description": "  "}}}),
            json!({"jsonrpc": "2.0", "id": 14, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "33333333333333333333333333333333", "title": "Bad tags", "description": "Tags not an array.", "tags": "solar"}}}),
            json!({"jsonrpc": "2.0", "id": 15, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "33333333333333333333333333333333", "title": "Empty tag", "description": "One empty tag.", "tags": ["solar", " "]}}}),
            json!({"jsonrpc": "2.0", "id": 16, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "33333333333333333333333333333333", "title": "Long tag", "description": "One overlong tag.", "tags": [long_tag]}}}),
            json!({"jsonrpc": "2.0", "id": 17, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": "33333333333333333333333333333333", "title": "Too many", "description": "Thirty-three tags.", "tags": many_tags}}}),
        ];
        let script = authed_script(
            &calls
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let out = run(&script, &backend, &options);
        assert_eq!(out.len(), 17);
        // Happy path: artifact id, title, and tags echoed; the fake
        // recorded exactly one creation, bound to the validated
        // project, with the project's standard privacy and the tags
        // trailer after the verbatim description.
        let created = &out[1]["result"]["structuredContent"];
        assert_eq!(created["artifactId"], "art-idea-1");
        assert_eq!(created["title"], "Solar kiosk");
        assert_eq!(created["tags"], json!(["solar", "kiosk"]));
        assert_eq!(out[1]["result"]["isError"], false);
        let recorded = backend.created.borrow();
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0],
            (
                "33333333333333333333333333333333".to_string(),
                "Solar kiosk".to_string(),
                "A kiosk idea.\n\nTags: solar, kiosk".to_string(),
                "standard".to_string(),
            )
        );
        // Project-gate failures (lines 3–7): identical not-found, and
        // no creation was recorded for any of them.
        for line in &out[2..7] {
            assert_eq!(line["result"]["isError"], true);
            assert_eq!(line["result"]["content"][0]["text"], PROJECT_NOT_FOUND_TEXT);
        }
        // Argument failures (lines 8–17): all -32602.
        for line in &out[7..17] {
            assert_eq!(line["error"]["code"], -32602);
        }
        assert_eq!(recorded.len(), 1);
    }

    #[test]
    fn tool_calls_cover_all_three_tools_with_kind_and_stage() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let script = authed_script(&format!(
            "{}\n{}\n{}\n{}\n",
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "memory.search", "arguments": {"query": "visible"}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "memory.search", "arguments": {"query": "visible", "kind": "task"}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "memory.get", "arguments": {"id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}}),
            json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "projects.search", "arguments": {"query": "project"}}}),
        ));
        let out = run(&script, &backend, &options);
        assert_eq!(out.len(), 5);
        // Unfiltered search: the private note is dropped, the standard
        // note and task remain.
        let search = &out[1]["result"];
        assert_eq!(search["isError"], false);
        let titles: Vec<&str> = search["structuredContent"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Visible note", "Task record"]);
        // Kind filter is enforced server-side even though the fake
        // backend ignores it.
        let filtered = &out[2]["result"]["structuredContent"];
        let titles: Vec<&str> = filtered
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Task record"]);
        let get = &out[3]["result"];
        assert_eq!(get["structuredContent"]["body"], "visible body");
        let projects = &out[4]["result"];
        let hits = projects["structuredContent"].as_array().unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0]["title"], "Active project");
        assert_eq!(hits[0]["stage"], "build");
        assert_eq!(hits[0]["stageLabel"], "Build");
        assert_eq!(hits[0]["stages"].as_array().unwrap().len(), 7);
        // Stage lookup ran only for the surviving active project.
        let artifact_calls: Vec<String> = backend
            .calls
            .borrow()
            .iter()
            .filter(|c| c.starts_with("artifact_types:"))
            .cloned()
            .collect();
        assert_eq!(
            artifact_calls,
            vec!["artifact_types:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"]
        );
    }

    #[test]
    fn unknown_tool_refused_without_backend_call() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let out = run(
            &authed_script(
                &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {}}}).to_string(),
            ),
            &backend,
            &options,
        );
        assert_eq!(out[1]["error"]["code"], -32602);
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn malformed_unknown_and_notification_handling() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let out = run(
            &format!(
                "not json\n{}\n{}\n{}\n",
                json!({"jsonrpc": "2.0", "id": 1, "method": "resources/list", "params": {}}),
                json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}),
                json!({"jsonrpc": "2.0", "id": 2, "method": "ping", "params": {}}),
            ),
            &backend,
            &options,
        );
        // Malformed line -> -32700; anything before auth -> -32001
        // (auth is checked before method dispatch, so no method
        // inventory leaks to unauthenticated clients).
        assert_eq!(out.len(), 3);
        assert_eq!(out[0]["error"]["code"], -32700);
        assert_eq!(out[0]["id"], Value::Null);
        assert_eq!(out[1]["error"]["code"], -32001);
        assert_eq!(out[2]["error"]["code"], -32001);
    }

    #[test]
    fn ping_after_auth_returns_empty_result() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let out = run(
            &authed_script(
                &json!({"jsonrpc": "2.0", "id": 2, "method": "ping", "params": {}}).to_string(),
            ),
            &backend,
            &options,
        );
        assert_eq!(out[1]["result"], json!({}));
    }

    #[test]
    fn invalid_tool_arguments_rejected() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let cases = [
            json!({"name": "memory.search", "arguments": {}}),
            json!({"name": "memory.search", "arguments": {"query": "   "}}),
            json!({"name": "memory.search", "arguments": {"query": "x".repeat(501)}}),
            json!({"name": "memory.search", "arguments": {"query": "x", "limit": 0}}),
            json!({"name": "memory.search", "arguments": {"query": "x", "limit": 51}}),
            json!({"name": "memory.search", "arguments": {"query": "x", "limit": 1.5}}),
            json!({"name": "memory.search", "arguments": {"query": "x", "kind": "bogus"}}),
            json!({"name": "memory.search", "arguments": "not-an-object"}),
            json!({"name": "memory.get", "arguments": {"id": "not-hex"}}),
            json!({"name": "memory.get", "arguments": {}}),
        ];
        let mut script = initialize("2025-11-25", Some(TOKEN)).to_string();
        for (i, case) in cases.iter().enumerate() {
            script.push('\n');
            script.push_str(
                &json!({"jsonrpc": "2.0", "id": 10 + i, "method": "tools/call", "params": case}).to_string(),
            );
        }
        script.push('\n');
        let out = run(&script, &backend, &options);
        assert_eq!(out.len(), 1 + cases.len());
        for response in &out[1..] {
            assert_eq!(response["error"]["code"], -32602);
        }
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn sealed_and_private_rows_never_leak_and_get_matches_missing() {
        let (_temp, options) = paired_root();
        let backend = FakeBackend::new();
        let script = authed_script(&format!(
            "{}\n{}\n{}\n{}\n",
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "memory.search", "arguments": {"query": "sealed-only"}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "memory.get", "arguments": {"id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "memory.get", "arguments": {"id": "cccccccccccccccccccccccccccccccc"}}}),
            json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "memory.get", "arguments": {"id": "ffffffffffffffffffffffffffffffff"}}}),
        ));
        let out = run(&script, &backend, &options);
        assert_eq!(out[1]["result"]["structuredContent"], json!([]));
        assert_eq!(out[2]["result"]["isError"], true);
        assert_eq!(out[2]["result"]["content"][0]["text"], NOT_FOUND_TEXT);
        assert_eq!(out[3]["result"], out[2]["result"]);
        assert_eq!(out[4]["result"], out[2]["result"]);
    }

    #[test]
    fn serve_rejects_bad_workspace_and_pairing_without_live_session() {
        let backend = FakeBackend::new();
        let mut reader = Cursor::new(b"{}\n".to_vec());
        let mut out: Vec<u8> = vec![];
        let (_temp, good) = paired_root();
        let bad = McpServerOptions {
            workspace_id: "not-a-workspace".to_string(),
            pairing_root: good.pairing_root.clone(),
        };
        assert!(serve(&mut reader, &mut out, &backend, &bad).is_err());
        let empty_temp = tempfile::tempdir().unwrap();
        let empty = McpServerOptions {
            workspace_id: WORKSPACE.to_string(),
            pairing_root: empty_temp.path().to_path_buf(),
        };
        assert!(serve(&mut reader, &mut out, &backend, &empty).is_err());
        let missing = McpServerOptions {
            workspace_id: WORKSPACE.to_string(),
            pairing_root: empty_temp.path().join("does-not-exist"),
        };
        assert!(serve(&mut reader, &mut out, &backend, &missing).is_err());
    }

    #[test]
    fn component_real_backend_round_trip_with_strict_privacy() {
        let temp = tempfile::tempdir().unwrap();
        let core = crate::Core::with_storage_root(temp.path().join("storage"));
        let workspace = core
            .workspace_command("create", json!({"contractVersion": 1, "name": "Server fixture"}))
            .unwrap()["workspace"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let created = core
            .memory_command(
                "create",
                json!({"contractVersion": 1, "workspaceId": workspace, "kind": "note", "draft": {"title": "Component note", "body": "Component body.", "privacy": "standard"}}),
            )
            .unwrap();
        let note_id = created["memory"]["id"].as_str().unwrap().to_string();
        let private = core
            .memory_command(
                "create",
                json!({"contractVersion": 1, "workspaceId": workspace, "kind": "note", "draft": {"title": "Component private note", "body": "Private body.", "privacy": "private"}}),
            )
            .unwrap();
        let private_id = private["memory"]["id"].as_str().unwrap().to_string();
        core.memory_command(
            "create",
            json!({"contractVersion": 1, "workspaceId": workspace, "kind": "project", "draft": {"title": "Component project", "body": "Project body.", "privacy": "standard"}}),
        )
        .unwrap();
        let backend = CoreMcpServerBackend::new(&core);
        let (_pairing_temp, mut options) = paired_root();
        options.workspace_id = workspace.clone();
        let script = format!(
            "{}\n{}\n{}\n{}\n{}\n",
            initialize("2026-07-28", Some(TOKEN)),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "memory.search", "arguments": {"query": "Component"}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "memory.get", "arguments": {"id": note_id}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "memory.get", "arguments": {"id": private_id}}}),
            json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "projects.search", "arguments": {"query": "Component"}}}),
        );
        let mut reader = Cursor::new(script.into_bytes());
        let mut out: Vec<u8> = vec![];
        serve(&mut reader, &mut out, &backend, &options).unwrap();
        let lines: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0]["result"]["protocolVersion"], "2026-07-28");
        let search = &lines[1]["result"]["structuredContent"];
        let titles: Vec<&str> = search
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["title"].as_str().unwrap())
            .collect();
        assert!(titles.contains(&"Component note"));
        assert!(titles.contains(&"Component project"));
        assert!(!titles.contains(&"Component private note"));
        assert_eq!(lines[2]["result"]["structuredContent"]["body"], "Component body.");
        assert_eq!(lines[3]["result"]["isError"], true);
        assert_eq!(lines[3]["result"]["content"][0]["text"], NOT_FOUND_TEXT);
        let projects = &lines[4]["result"]["structuredContent"];
        let project = projects
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["title"] == "Component project")
            .unwrap();
        assert_eq!(project["stage"], "discover");
        assert_eq!(project["stages"].as_array().unwrap().len(), 7);
    }

    #[test]
    fn component_project_tools_with_real_artifacts_and_privacy() {
        let temp = tempfile::tempdir().unwrap();
        let core = crate::Core::with_storage_root(temp.path().join("storage"));
        let workspace = core
            .workspace_command("create", json!({"contractVersion": 1, "name": "Project tools fixture"}))
            .unwrap()["workspace"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let project = core
            .memory_command(
                "create",
                json!({"contractVersion": 1, "workspaceId": workspace, "kind": "project", "draft": {"title": "Fixture project", "body": "Project body.", "privacy": "standard"}}),
            )
            .unwrap();
        let project_id = project["memory"]["id"].as_str().unwrap().to_string();
        let private_project = core
            .memory_command(
                "create",
                json!({"contractVersion": 1, "workspaceId": workspace, "kind": "project", "draft": {"title": "Hidden project", "body": "Project body.", "privacy": "private"}}),
            )
            .unwrap();
        let private_project_id = private_project["memory"]["id"].as_str().unwrap().to_string();
        let note = core
            .memory_command(
                "create",
                json!({"contractVersion": 1, "workspaceId": workspace, "kind": "note", "draft": {"title": "Fixture note", "body": "Note body.", "privacy": "standard"}}),
            )
            .unwrap();
        let _note_id = note["memory"]["id"].as_str().unwrap().to_string();
        for (artifact_type, title, privacy) in [
            ("experiment", "Real collision", "standard"),
            ("experiment", "Hidden collision", "private"),
            ("deliverable", "Real build", "standard"),
            ("scenario", "Swarm branch", "standard"),
        ] {
            core.artifacts_command(
                "register",
                json!({"contractVersion": 1, "workspaceId": workspace, "projectId": project_id, "type": artifact_type, "title": title, "body": "Artifact body.", "privacy": privacy}),
            )
            .unwrap();
        }
        let backend = CoreMcpServerBackend::new(&core);
        let (_pairing_temp, mut options) = paired_root();
        options.workspace_id = workspace.clone();
        let script = format!(
            "{}\n{}\n{}\n{}\n{}\n",
            initialize("2025-11-25", Some(TOKEN)),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "notes.search", "arguments": {"query": "Fixture"}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "experiments.list", "arguments": {"projectId": project_id}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "products.get", "arguments": {"projectId": project_id}}}),
            json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "experiments.list", "arguments": {"projectId": private_project_id}}}),
        );
        let mut reader = Cursor::new(script.into_bytes());
        let mut out: Vec<u8> = vec![];
        serve(&mut reader, &mut out, &backend, &options).unwrap();
        let lines: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 5);
        // notes.search finds the standard note (kind fixed to note).
        let notes = &lines[1]["result"]["structuredContent"];
        let titles: Vec<&str> = notes
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Fixture note"]);
        // experiments.list: only the standard experiment; the private
        // experiment, the deliverable, and the scenario are excluded.
        let experiments = &lines[2]["result"];
        assert_eq!(experiments["isError"], false);
        let titles: Vec<&str> = experiments["structuredContent"]["experiments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Real collision"]);
        // products.get: only the deliverable; scenario is not a Forge
        // product artifact, and no body is carried.
        let products = &lines[3]["result"];
        let titles: Vec<&str> = products["structuredContent"]["products"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Real build"]);
        assert!(products["structuredContent"]["products"][0].get("body").is_none());
        // The private project is not-found, identically to unknown.
        assert_eq!(lines[4]["result"]["isError"], true);
        assert_eq!(lines[4]["result"]["content"][0]["text"], PROJECT_NOT_FOUND_TEXT);
    }

    #[test]
    fn component_ideas_create_with_real_artifacts_and_privacy() {
        let temp = tempfile::tempdir().unwrap();
        let core = crate::Core::with_storage_root(temp.path().join("storage"));
        let workspace = core
            .workspace_command("create", json!({"contractVersion": 1, "name": "Ideas fixture"}))
            .unwrap()["workspace"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let project = core
            .memory_command(
                "create",
                json!({"contractVersion": 1, "workspaceId": workspace, "kind": "project", "draft": {"title": "Ideas project", "body": "Project body.", "privacy": "standard"}}),
            )
            .unwrap();
        let project_id = project["memory"]["id"].as_str().unwrap().to_string();
        let private_project = core
            .memory_command(
                "create",
                json!({"contractVersion": 1, "workspaceId": workspace, "kind": "project", "draft": {"title": "Hidden project", "body": "Project body.", "privacy": "private"}}),
            )
            .unwrap();
        let private_project_id = private_project["memory"]["id"].as_str().unwrap().to_string();
        let backend = CoreMcpServerBackend::new(&core);
        let (_pairing_temp, mut options) = paired_root();
        options.workspace_id = workspace.clone();
        let script = format!(
            "{}\n{}\n{}\n{}\n",
            initialize("2025-11-25", Some(TOKEN)),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": project_id, "title": "Solar kiosk", "description": "A kiosk idea.", "tags": ["alpha", "beta"]}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": project_id, "title": "Untagged idea", "description": "No tags here."}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {"projectId": private_project_id, "title": "Hidden idea", "description": "Must not land."}}}),
        );
        let mut reader = Cursor::new(script.into_bytes());
        let mut out: Vec<u8> = vec![];
        serve(&mut reader, &mut out, &backend, &options).unwrap();
        let lines: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 4);
        // The tagged idea reports its artifact id and echoed tags.
        let first = &lines[1]["result"]["structuredContent"];
        assert_eq!(first["type"], "idea");
        assert_eq!(first["title"], "Solar kiosk");
        assert_eq!(first["tags"], json!(["alpha", "beta"]));
        let artifact_id = first["artifactId"].as_str().unwrap().to_string();
        assert_eq!(lines[2]["result"]["isError"], false);
        // The private project is not-found and creates nothing.
        assert_eq!(lines[3]["result"]["isError"], true);
        assert_eq!(lines[3]["result"]["content"][0]["text"], PROJECT_NOT_FOUND_TEXT);
        // The registry proves persistence: exactly the two ideas, with
        // the tagged body carrying the verbatim description plus the
        // tags line, and the untagged body exactly the description.
        let listed = core
            .artifacts_command(
                "list",
                json!({"contractVersion": 1, "workspaceId": workspace, "projectId": project_id}),
            )
            .unwrap();
        let artifacts = listed["artifacts"].as_array().unwrap();
        assert_eq!(artifacts.len(), 2);
        let tagged = artifacts
            .iter()
            .find(|a| a["artifactId"].as_str() == Some(artifact_id.as_str()))
            .unwrap();
        assert_eq!(tagged["type"], "idea");
        assert_eq!(tagged["privacy"], "standard");
        let read = core
            .artifacts_command(
                "read",
                json!({"contractVersion": 1, "workspaceId": workspace, "artifactId": artifact_id}),
            )
            .unwrap();
        assert_eq!(read["body"], "A kiosk idea.\n\nTags: alpha, beta");
        let untagged = artifacts
            .iter()
            .find(|a| a["title"].as_str() == Some("Untagged idea"))
            .unwrap();
        let untagged_read = core
            .artifacts_command(
                "read",
                json!({"contractVersion": 1, "workspaceId": workspace, "artifactId": untagged["artifactId"].as_str().unwrap()}),
            )
            .unwrap();
        assert_eq!(untagged_read["body"], "No tags here.");
        let hidden_listed = core
            .artifacts_command(
                "list",
                json!({"contractVersion": 1, "workspaceId": workspace, "projectId": private_project_id}),
            )
            .unwrap();
        assert_eq!(hidden_listed["artifacts"].as_array().unwrap().len(), 0);
    }
}
