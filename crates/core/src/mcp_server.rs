//! MCP *server* side (MASTER_SPEC §68, Milestone 13, Slice 3).
//!
//! The mirror image of the Slice 1/2 client: ANANSI itself answers MCP
//! requests over NDJSON stdio, exposing exactly three approved read-only
//! knowledge tools — `memory.search`, `memory.get`, `projects.search`.
//! Everything else §68 lists as possible (`notes.search`, `products.get`,
//! `experiments.list`, and the write tool `ideas.create`) is out of this
//! slice; the allowlist below is the whole permission set, enforced in
//! dispatch before any backend call.
//!
//! Security boundary, in three layers:
//! 1. Disabled by default: the server runs only when the operator launches
//!    `anansi-api --mcp-server` with a workspace id AND sets
//!    `ANANSI_MCP_SERVER_TOKEN`. No flag, no server; flag without a token,
//!    the process refuses to start.
//! 2. Token gate: until a valid `initialize` presents the token (in
//!    `params._meta.anansiMcpToken`, an ANANSI extension on MCP's sanctioned
//!    `_meta` point — not part of the MCP spec), every request except
//!    `initialize` is refused with `-32001`. The token is accepted only
//!    from the environment, never from a CLI argument, and is never logged
//!    or echoed.
//! 3. Sealed boundary: the storage layer already excludes sealed rows from
//!    lists and refuses sealed reads; this module additionally drops any
//!    sealed hit from search results and refuses any sealed `get` result,
//!    even if a backend ever returned one. A `get` on a missing id and on
//!    a sealed id produce the identical `Memory not found.` result, so the
//!    server is never an existence oracle for sealed records.
//!
//! The engine is transport-agnostic (`serve` over `BufRead`/`Write`) so unit
//! tests drive it over in-memory buffers and the binary wires it to stdin /
//! stdout. One stdio process serves one client for its lifetime; there are
//! no sessions, no polling, no persistent connections. Search discovers
//! titles and snippets only (the core's validated title-search path);
//! `memory.get` is the only path that returns a body, and only for
//! non-sealed plaintext records.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::lifecycle::MemoryState;
use crate::memory::{MemoryDetails, MemoryList, MemoryMetadata};
use crate::privacy::Privacy;
use crate::readiness::{CommandError, ErrorCode};

/// Server name reported in `initialize` responses.
pub const SERVER_NAME: &str = "ANANSI";
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
/// Tool-level "not found" text, identical for missing and sealed ids.
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
}

/// Runtime options for `serve`. Distinct from the contract DTO above.
#[derive(Debug, Clone)]
pub struct McpServerOptions {
    pub workspace_id: String,
    pub token: String,
}

/// Data access behind the protocol engine. The server layer owns policy
/// (sealed drops, active-project filtering, allowlist); backends own reads.
pub trait McpServerBackend {
    fn search_memories(
        &self,
        workspace_id: &str,
        query: &str,
        limit: u32,
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
}

/// Production backend: the same validated `Core::memory_command` entry the
/// HTTP API uses, so the server inherits id/query validation and the
/// storage layer's sealed handling instead of reimplementing them.
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
    ) -> Result<Vec<MemoryMetadata>, CommandError> {
        let value = self.core.memory_command(
            "list",
            json!({"contractVersion": 1, "workspaceId": workspace_id, "query": query, "limit": limit}),
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

fn query_limit_schema() -> (BTreeMap<String, McpSchemaProperty>, Vec<String>) {
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
    (properties, vec!["query".to_string()])
}

pub fn tool_definitions() -> Vec<McpServerToolDef> {
    let (search_properties, search_required) = query_limit_schema();
    let (project_properties, project_required) = query_limit_schema();
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
            description: "Search workspace memory titles and snippets.".to_string(),
            input_schema: McpToolInputSchema {
                schema_type: "object".to_string(),
                properties: search_properties,
                required: search_required,
            },
        },
        McpServerToolDef {
            name: "memory.get".to_string(),
            description: "Retrieve one non-sealed memory by ID, including its body.".to_string(),
            input_schema: McpToolInputSchema {
                schema_type: "object".to_string(),
                properties: id_properties,
                required: vec!["id".to_string()],
            },
        },
        McpServerToolDef {
            name: "projects.search".to_string(),
            description: "Search active project titles and summaries.".to_string(),
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

/// Run the server loop until stdin closes. Returns Ok on clean EOF; a
/// malformed workspace id or empty token fails before reading anything.
pub fn serve<B: McpServerBackend>(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    backend: &B,
    options: &McpServerOptions,
) -> Result<(), CommandError> {
    crate::workspace::validate_id(&options.workspace_id).map_err(|_| {
        CommandError::new(ErrorCode::InvalidWorkspace, "Invalid workspace id.")
    })?;
    if options.token.is_empty() {
        return Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "MCP server token must not be empty.",
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

#[allow(clippy::too_many_arguments)]
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
        let token = params
            .get("_meta")
            .and_then(|m| m.get("anansiMcpToken"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !tokens_equal(token, &options.token) {
            return error_response(
                id,
                -32001,
                "Unauthorized: invalid or missing MCP server token.",
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
            "Unauthorized: initialize with a valid token first.",
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
    let hits = if projects_only {
        backend.search_projects(&options.workspace_id, &query, limit)
    } else {
        backend.search_memories(&options.workspace_id, &query, limit)
    }
    .map_err(|_| (-32603, "Tool execution failed.".to_string()))?;
    let mut rendered = Vec::new();
    for meta in &hits {
        // Layer 3 of the sealed boundary: never emit a sealed row, even if
        // a backend returned one.
        if meta.privacy == Privacy::Sealed {
            continue;
        }
        if projects_only && !is_active_project(meta.status) {
            continue;
        }
        rendered.push(search_hit(meta));
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
    // Missing and sealed produce the identical result: no existence oracle.
    let details = match found {
        Ok(details) if details.memory.privacy != Privacy::Sealed => details,
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

    const TOKEN: &str = "slice3-test-token";
    const WORKSPACE: &str = "0123456789abcdef0123456789abcdef";

    fn options() -> McpServerOptions {
        McpServerOptions {
            workspace_id: WORKSPACE.to_string(),
            token: TOKEN.to_string(),
        }
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
        ) -> Result<Vec<MemoryMetadata>, CommandError> {
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
            Ok(vec![meta(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "Visible note",
                "note",
                MemoryState::Durable,
                Privacy::Private,
            )])
        }

        fn read_memory(
            &self,
            _workspace_id: &str,
            id: &str,
        ) -> Result<MemoryDetails, CommandError> {
            self.calls.borrow_mut().push("get".to_string());
            if id == "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" {
                return Ok(MemoryDetails {
                    contract_version: 1,
                    memory: meta(
                        id,
                        "Sealed note",
                        "note",
                        MemoryState::Durable,
                        Privacy::Sealed,
                    ),
                    body: "secret body".to_string(),
                });
            }
            if id == "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" {
                return Ok(MemoryDetails {
                    contract_version: 1,
                    memory: meta(
                        id,
                        "Visible note",
                        "note",
                        MemoryState::Durable,
                        Privacy::Private,
                    ),
                    body: "visible body".to_string(),
                });
            }
            Err(CommandError::new(ErrorCode::NotFound, "missing"))
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
                    "cccccccccccccccccccccccccccccccc",
                    "Active project",
                    "project",
                    MemoryState::Durable,
                    Privacy::Private,
                ),
                meta(
                    "dddddddddddddddddddddddddddddddd",
                    "Old project",
                    "project",
                    MemoryState::Superseded,
                    Privacy::Private,
                ),
                meta(
                    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                    "Archived project",
                    "project",
                    MemoryState::Archived,
                    Privacy::Private,
                ),
            ])
        }
    }

    /// Drive `serve` over one scripted input and return parsed output lines.
    fn run(input: &str, backend: &FakeBackend) -> Vec<Value> {
        let mut reader = Cursor::new(input.as_bytes().to_vec());
        let mut out: Vec<u8> = vec![];
        serve(&mut reader, &mut out, backend, &options()).unwrap();
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
    fn handshake_accepts_both_protocol_versions() {
        for version in SUPPORTED_PROTOCOL_VERSIONS {
            let backend = FakeBackend::new();
            let out = run(&format!("{}\n", initialize(version, Some(TOKEN))), &backend);
            assert_eq!(out.len(), 1);
            assert_eq!(out[0]["result"]["protocolVersion"], version);
            assert_eq!(out[0]["result"]["serverInfo"]["name"], "ANANSI");
            assert!(out[0]["result"]["capabilities"]["tools"].is_object());
        }
    }

    #[test]
    fn handshake_rejects_unsupported_and_missing_versions() {
        let backend = FakeBackend::new();
        let out = run(
            &format!(
                "{}\n{}\n",
                initialize("2024-01-01", Some(TOKEN)),
                json!({"jsonrpc": "2.0", "id": 2, "method": "initialize", "params": {"capabilities": {}, "clientInfo": {}}}),
            ),
            &backend,
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["error"]["code"], -32602);
        assert_eq!(out[1]["error"]["code"], -32602);
    }

    #[test]
    fn auth_gate_refuses_then_accepts() {
        let backend = FakeBackend::new();
        let out = run(
            &format!(
                "{}\n{}\n{}\n{}\n",
                json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
                initialize("2025-11-25", Some("wrong-token")),
                initialize("2025-11-25", None),
                initialize("2025-11-25", Some(TOKEN)),
            ),
            &backend,
        );
        assert_eq!(out.len(), 4);
        assert_eq!(out[0]["error"]["code"], -32001);
        assert_eq!(out[1]["error"]["code"], -32001);
        assert_eq!(out[2]["error"]["code"], -32001);
        assert!(out[3]["result"]["serverInfo"].is_object());
    }

    #[test]
    fn tools_list_returns_exactly_three_tools() {
        let backend = FakeBackend::new();
        let out = run(
            &authed_script(
                &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}).to_string(),
            ),
            &backend,
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
    fn tool_calls_cover_all_three_tools() {
        let backend = FakeBackend::new();
        let script = authed_script(&format!(
            "{}\n{}\n{}",
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "memory.search", "arguments": {"query": "visible"}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "memory.get", "arguments": {"id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "projects.search", "arguments": {"query": "project"}}}),
        ));
        let out = run(&script, &backend);
        assert_eq!(out.len(), 4);
        let search = &out[1]["result"];
        assert_eq!(search["isError"], false);
        assert!(search["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Visible note"));
        let get = &out[2]["result"];
        assert_eq!(get["structuredContent"]["body"], "visible body");
        let projects = &out[3]["result"];
        let titles: Vec<&str> = projects["structuredContent"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Active project"]);
    }

    #[test]
    fn unknown_tool_refused_without_backend_call() {
        let backend = FakeBackend::new();
        let out = run(
            &authed_script(
                &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "ideas.create", "arguments": {}}}).to_string(),
            ),
            &backend,
        );
        assert_eq!(out[1]["error"]["code"], -32602);
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn malformed_unknown_and_notification_handling() {
        let backend = FakeBackend::new();
        let out = run(
            &format!(
                "not json\n{}\n{}\n{}\n",
                json!({"jsonrpc": "2.0", "id": 1, "method": "resources/list", "params": {}}),
                json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}),
                json!({"jsonrpc": "2.0", "id": 2, "method": "ping", "params": {}}),
            ),
            &backend,
        );
        // Malformed line -> -32700; unknown method -> -32601 even before
        // auth (method dispatch happens after the auth check, so the
        // unknown method here is refused as unauthorized first).
        assert_eq!(out.len(), 3);
        assert_eq!(out[0]["error"]["code"], -32700);
        assert_eq!(out[0]["id"], Value::Null);
        assert_eq!(out[1]["error"]["code"], -32001);
        assert_eq!(out[2]["error"]["code"], -32001);
    }

    #[test]
    fn ping_after_auth_returns_empty_result() {
        let backend = FakeBackend::new();
        let out = run(
            &authed_script(
                &json!({"jsonrpc": "2.0", "id": 2, "method": "ping", "params": {}}).to_string(),
            ),
            &backend,
        );
        assert_eq!(out[1]["result"], json!({}));
    }

    #[test]
    fn invalid_tool_arguments_rejected() {
        let backend = FakeBackend::new();
        let cases = [
            json!({"name": "memory.search", "arguments": {}}),
            json!({"name": "memory.search", "arguments": {"query": "   "}}),
            json!({"name": "memory.search", "arguments": {"query": "x".repeat(501)}}),
            json!({"name": "memory.search", "arguments": {"query": "x", "limit": 0}}),
            json!({"name": "memory.search", "arguments": {"query": "x", "limit": 51}}),
            json!({"name": "memory.search", "arguments": {"query": "x", "limit": 1.5}}),
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
        let out = run(&script, &backend);
        assert_eq!(out.len(), 1 + cases.len());
        for response in &out[1..] {
            assert_eq!(response["error"]["code"], -32602);
        }
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn sealed_rows_never_leak_and_get_matches_missing() {
        let backend = FakeBackend::new();
        let script = authed_script(&format!(
            "{}\n{}\n{}\n",
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "memory.search", "arguments": {"query": "sealed-only"}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "memory.get", "arguments": {"id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "memory.get", "arguments": {"id": "ffffffffffffffffffffffffffffffff"}}}),
        ));
        let out = run(&script, &backend);
        assert_eq!(out[1]["result"]["structuredContent"], json!([]));
        assert_eq!(out[2]["result"]["isError"], true);
        assert_eq!(out[2]["result"]["content"][0]["text"], NOT_FOUND_TEXT);
        assert_eq!(out[3]["result"], out[2]["result"]);
    }

    #[test]
    fn serve_rejects_bad_workspace_and_empty_token_before_reading() {
        let backend = FakeBackend::new();
        let mut reader = Cursor::new(b"{}\n".to_vec());
        let mut out: Vec<u8> = vec![];
        let bad = McpServerOptions {
            workspace_id: "not-a-workspace".to_string(),
            token: TOKEN.to_string(),
        };
        assert!(serve(&mut reader, &mut out, &backend, &bad).is_err());
        let empty = McpServerOptions {
            workspace_id: WORKSPACE.to_string(),
            token: String::new(),
        };
        assert!(serve(&mut reader, &mut out, &backend, &empty).is_err());
    }

    #[test]
    fn component_real_backend_round_trip() {
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
                json!({"contractVersion": 1, "workspaceId": workspace, "kind": "note", "draft": {"title": "Component note", "body": "Component body."}}),
            )
            .unwrap();
        let note_id = created["memory"]["id"].as_str().unwrap().to_string();
        core.memory_command(
            "create",
            json!({"contractVersion": 1, "workspaceId": workspace, "kind": "project", "draft": {"title": "Component project", "body": "Project body."}}),
        )
        .unwrap();
        let backend = CoreMcpServerBackend::new(&core);
        let options = McpServerOptions {
            workspace_id: workspace.clone(),
            token: TOKEN.to_string(),
        };
        let script = format!(
            "{}\n{}\n{}\n{}\n",
            initialize("2026-07-28", Some(TOKEN)),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "memory.search", "arguments": {"query": "Component"}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "memory.get", "arguments": {"id": note_id}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "projects.search", "arguments": {"query": "Component"}}}),
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
        assert_eq!(lines[0]["result"]["protocolVersion"], "2026-07-28");
        let search = &lines[1]["result"]["structuredContent"];
        assert!(search
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["title"] == "Component note"));
        assert_eq!(lines[2]["result"]["structuredContent"]["body"], "Component body.");
        let projects = &lines[3]["result"]["structuredContent"];
        assert!(projects
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["title"] == "Component project"));
    }
}
