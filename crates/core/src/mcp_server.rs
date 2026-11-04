//! MCP *server* side (MASTER_SPEC §68, Milestone 13, Slice 3, as revised).
//!
//! ANANSI itself answers MCP requests over NDJSON stdio, exposing exactly
//! three approved read-only knowledge tools — `memory.search`, `memory.get`,
//! `projects.search`. Everything else §68 lists as possible
//! (`notes.search`, `products.get`, `experiments.list`, and the write tool
//! `ideas.create`) is out of this slice; the allowlist below is the whole
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
}

/// The three approved tools, in `tools/list` order.
pub fn tool_names() -> Vec<String> {
    vec![
        "memory.search".to_string(),
        "memory.get".to_string(),
        "projects.search".to_string(),
    ]
}

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
            name: "projects.search".to_string(),
            description: "Search active shareable project titles, summaries, and stage metadata.".to_string(),
            input_schema: McpToolInputSchema {
                schema_type: "object".to_string(),
                properties: project_properties,
                required: project_required,
            },
        },
    ]
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
    let limit = match arguments.get("limit") {
        None => DEFAULT_LIMIT,
        Some(value) => {
            let n = value.as_u64().filter(|n| *n >= 1 && *n <= u64::from(MAX_LIMIT));
            match n {
                Some(n) => n as u32,
                None => return Err(invalid("limit must be an integer between 1 and 50.")),
            }
        }
    };
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
    let id = arguments
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| invalid("id is required and must be a string."))?;
    crate::workspace::validate_id(id)
        .map_err(|_| invalid("id must be a 32-character lowercase hex string."))?;
    Ok(id.to_string())
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
        "memory.search" => call_search(&arguments, backend, options, false),
        "projects.search" => call_search(&arguments, backend, options, true),
        "memory.get" => call_get(&arguments, backend, options),
        _ => unreachable!("allowlist checked above"),
    };
    match result {
        Ok(tool_value) => result_response(id, tool_value),
        Err((code, message)) => error_response(id, code, &message),
    }
}

fn call_search<B: McpServerBackend>(
    arguments: &Value,
    backend: &B,
    options: &McpServerOptions,
    projects_only: bool,
) -> Result<Value, (i64, String)> {
    let (query, limit) = parse_query(arguments)?;
    let kind = if projects_only {
        None
    } else {
        parse_kind(arguments)?
    };
    let hits = if projects_only {
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
        if projects_only {
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
    }

    impl FakeBackend {
        fn new() -> Self {
            Self {
                calls: RefCell::new(vec![]),
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
            let (title, privacy, body) = match id {
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" => ("Sealed note", Privacy::Sealed, "secret body"),
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" => ("Visible note", Privacy::Standard, "visible body"),
                "cccccccccccccccccccccccccccccccc" => ("Private note", Privacy::Private, "private body"),
                _ => return Err(CommandError::new(ErrorCode::NotFound, "missing")),
            };
            Ok(MemoryDetails {
                contract_version: 1,
                memory: meta(id, title, "note", MemoryState::Durable, privacy),
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
    fn tools_list_returns_exactly_three_tools() {
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
        assert_eq!(names, vec!["memory.search", "memory.get", "projects.search"]);
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
}
