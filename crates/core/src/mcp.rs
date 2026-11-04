//! MCP client configuration, registry, and one-shot stdio probing.
//!
//! Slice 1 (MASTER_SPEC §68, Milestone 13) covers the MCP *client*
//! side only: registering servers, listing them, and probing one
//! server for its tools and prompts. It deliberately does NOT
//! implement `tools/call`, resources, or the MCP server side, and it
//! never keeps a persistent connection: a probe spawns the
//! configured local command, runs the handshake plus list calls,
//! then kills the child. No auto-connect, no polling, no probe on
//! view load — a probe runs only when the user asks for one server.
//!
//! Protocol negotiation (spec verified before implementation):
//! the deployed 2025-11-25 revision uses `initialize` +
//! `notifications/initialized` + `ping`; the current stable
//! 2026-07-28 revision is stateless, removes both `initialize`
//! and `ping`, and requires `server/discover`. The probe tries
//! `initialize` first and falls back to `server/discover` only on
//! a method-not-found error, and it reports the health method
//! actually used — a ping is never fabricated for a stateless
//! server.
//!
//! Transports: stdio is fully implemented. SSE / Streamable HTTP
//! servers can be registered and listed, but a probe is explicitly
//! refused: the workspace has no POST HTTP client, and §68
//! (default external access disabled) plus §71 (SSRF, DNS
//! rebinding) require remote access to arrive behind explicit
//! per-server authorization in a later slice.
//!
//! Safety: the spawned child never runs through a shell, and its
//! environment is cleared and rebuilt from the configured env vars
//! only — the API server process environment (which may hold proxy
//! credentials) is never inherited. One Windows exception, added by
//! the Slice 1 repair: on Windows the CRT and runtimes such as
//! Node.js need `SystemRoot`/`WINDIR` (and `PATH` when not
//! configured) to boot at all — wiping them aborts a Node MCP
//! server before it runs any script. Those are system paths, not
//! credentials, and a configured value always wins over a preserved
//! one. Server descriptions and schemas
//! are untrusted text: they are validated, capped, and returned for
//! display, never executed and never forwarded to a provider.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::readiness::{CommandError, ErrorCode};
use crate::storage;

/// Protocol version offered to legacy (pre-2026-07-28) servers.
pub const LEGACY_PROTOCOL_VERSION: &str = "2025-11-25";
/// Protocol version used for stateless (2026-07-28) servers.
pub const STATELESS_PROTOCOL_VERSION: &str = "2026-07-28";
/// Whole-probe budget: spawn, handshake, lists, and health check.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
/// Longest single NDJSON line accepted from a server.
pub const MAX_LINE_BYTES: usize = 1024 * 1024;
/// Tools/prompts kept per probe; the rest set `truncated`.
pub const MAX_LISTED: usize = 100;
pub const MAX_SERVERS_PER_WORKSPACE: usize = 64;

fn invalid(message: &str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn unavailable() -> CommandError {
    crate::storage::unavailable()
}

// ---------------------------------------------------------------- DTOs

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpServerDraft {
    pub name: String,
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Stored server configuration. Env VALUES live only in this file;
/// they are never returned to API clients (see `McpServer`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Public server view: env values redacted to their keys.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub contract_version: u8,
    pub id: String,
    pub name: String,
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env_keys: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
}

impl From<&McpServerConfig> for McpServer {
    fn from(config: &McpServerConfig) -> Self {
        Self {
            contract_version: 1,
            id: config.id.clone(),
            name: config.name.clone(),
            transport: config.transport.clone(),
            command: config.command.clone(),
            args: config.args.clone(),
            env_keys: config.env.keys().cloned().collect(),
            url: config.url.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpServerList {
    pub contract_version: u8,
    pub servers: Vec<McpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpToolAnnotations {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub read_only_hint: Option<bool>,
    #[serde(default)]
    pub destructive_hint: Option<bool>,
    #[serde(default)]
    pub idempotent_hint: Option<bool>,
    #[serde(default)]
    pub open_world_hint: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub annotations: Option<McpToolAnnotations>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpPromptArgument {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpPrompt {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub arguments: Vec<McpPromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpProbeResult {
    pub contract_version: u8,
    pub server_id: String,
    pub healthy: bool,
    #[serde(default)]
    pub protocol_version: Option<String>,
    #[serde(default)]
    pub health_method: Option<String>,
    #[serde(default)]
    pub server_name: Option<String>,
    #[serde(default)]
    pub tools: Vec<McpTool>,
    #[serde(default)]
    pub prompts: Vec<McpPrompt>,
    pub truncated: bool,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpRemoveResult {
    pub contract_version: u8,
    pub removed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpListRequest {
    pub contract_version: u8,
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpRegisterRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub draft: McpServerDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpRemoveRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpProbeRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: String,
}

/// One content part of a tool result. Server text is data for
/// display, never executed and never forwarded to a provider.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub text: Option<String>,
}

/// Structured result of one completed tool dispatch. A tool-level
/// `isError` is a completed invocation with status Error, not a
/// transport failure; transport failures return `CommandError`
/// instead and never produce this value.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallResult {
    pub contract_version: u8,
    pub server_id: String,
    pub tool_name: String,
    pub is_error: bool,
    pub text: String,
    #[serde(default)]
    pub content: Vec<McpToolCallContent>,
    #[serde(default)]
    pub structured_content: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpFile {
    version: u8,
    #[serde(default)]
    servers: Vec<McpServerConfig>,
}

// ------------------------------------------------------------ validation

fn has_control(text: &str) -> bool {
    text.chars().any(|c| c.is_control())
}

fn valid_env_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn validate_draft(draft: &McpServerDraft) -> Result<(), CommandError> {
    let name = draft.name.trim();
    if name.is_empty() || name.chars().count() > 100 || has_control(&draft.name) {
        return Err(invalid(
            "Server name must be 1 to 100 characters without control characters.",
        ));
    }
    match draft.transport.as_str() {
        "stdio" => {
            let command = draft.command.as_deref().unwrap_or("").trim();
            if command.is_empty() || has_control(command) {
                return Err(invalid(
                    "A stdio server requires a command without control characters.",
                ));
            }
            if draft.args.len() > 64
                || draft
                    .args
                    .iter()
                    .any(|a| a.chars().count() > 1024 || has_control(a))
            {
                return Err(invalid(
                    "Arguments are limited to 64 entries of 1024 characters without control characters.",
                ));
            }
            if draft.url.is_some() {
                return Err(invalid("A stdio server takes no URL."));
            }
        }
        "sse" => {
            if draft.command.as_deref().map(|c| !c.trim().is_empty()).unwrap_or(false)
                || !draft.args.is_empty()
                || !draft.env.is_empty()
            {
                return Err(invalid(
                    "An SSE server takes a URL only, not a command, arguments, or environment.",
                ));
            }
            let url = draft.url.as_deref().unwrap_or("");
            let parsed = url::Url::parse(url).map_err(|_| {
                invalid("An SSE server requires an http(s) URL.")
            })?;
            if parsed.scheme() != "http" && parsed.scheme() != "https" {
                return Err(invalid("An SSE server requires an http(s) URL."));
            }
        }
        _ => {
            return Err(invalid(
                "Transport must be 'stdio' or 'sse'.",
            ));
        }
    }
    if draft.env.len() > 50
        || draft.env.iter().any(|(k, v)| {
            !valid_env_key(k) || v.chars().count() > 4096 || has_control(v)
        })
    {
        return Err(invalid(
            "Environment is limited to 50 entries with alphanumeric/underscore keys and values up to 4096 characters.",
        ));
    }
    Ok(())
}

// -------------------------------------------------------------- file I/O

fn safe_workspace_id(workspace_id: &str) -> String {
    workspace_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn mcp_config_path(root: &Path, workspace_id: &str) -> PathBuf {
    root.join("mcp-servers")
        .join(format!("{}.json", safe_workspace_id(workspace_id)))
}

fn load_mcp_file(root: &Path, workspace_id: &str) -> Result<McpFile, CommandError> {
    let dir = root.join("mcp-servers");
    if dir.symlink_metadata().is_ok() {
        storage::reject_links(&dir).map_err(|_| unavailable())?;
    }
    let path = mcp_config_path(root, workspace_id);
    if !path.exists() {
        return Ok(McpFile {
            version: 1,
            servers: Vec::new(),
        });
    }
    storage::reject_links(&path).map_err(|_| unavailable())?;
    let text = std::fs::read_to_string(&path).map_err(|_| unavailable())?;
    let file: McpFile = serde_json::from_str(&text)
        .map_err(|_| CommandError::new(ErrorCode::CorruptStorage, "MCP registry is corrupt."))?;
    if file.version != 1 {
        return Err(CommandError::new(
            ErrorCode::CorruptStorage,
            "MCP registry version is not supported.",
        ));
    }
    for server in &file.servers {
        let draft = McpServerDraft {
            name: server.name.clone(),
            transport: server.transport.clone(),
            command: server.command.clone(),
            args: server.args.clone(),
            env: server.env.clone(),
            url: server.url.clone(),
        };
        validate_draft(&draft).map_err(|_| {
            CommandError::new(ErrorCode::CorruptStorage, "MCP registry entry is invalid.")
        })?;
    }
    Ok(file)
}

fn persist_mcp_file(root: &Path, workspace_id: &str, file: &McpFile) -> Result<(), CommandError> {
    let dir = root.join("mcp-servers");
    std::fs::create_dir_all(&dir).map_err(|_| unavailable())?;
    storage::reject_links(&dir).map_err(|_| unavailable())?;
    storage::private_permissions(&dir, true, false).map_err(|_| unavailable())?;
    let path = mcp_config_path(root, workspace_id);
    if path.symlink_metadata().is_ok() {
        storage::reject_links(&path).map_err(|_| unavailable())?;
    }
    let json = serde_json::to_string_pretty(file).map_err(|_| unavailable())?;
    let mut tmp = None;
    for attempt in 0..8 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let candidate = dir.join(format!(
            ".mcp-{}-{}-{}.tmp",
            std::process::id(),
            nanos,
            attempt
        ));
        let opened = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate);
        match opened {
            Ok(mut f) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = f.set_permissions(std::fs::Permissions::from_mode(0o600));
                }
                if f.write_all(json.as_bytes()).and_then(|_| f.sync_all()).is_ok() {
                    tmp = Some(candidate);
                    break;
                }
                let _ = std::fs::remove_file(&candidate);
                return Err(unavailable());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(unavailable()),
        }
    }
    let tmp = tmp.ok_or_else(unavailable)?;
    std::fs::rename(&tmp, &path).map_err(|_| {
        let _ = std::fs::remove_file(&tmp);
        unavailable()
    })?;
    if let Ok(dir_file) = std::fs::File::open(&dir) {
        let _ = dir_file.sync_all();
    }
    Ok(())
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn new_server_id(existing: &[McpServerConfig]) -> String {
    for _ in 0..64 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        use std::hash::Hasher;
        hash.write_u128(nanos);
        hash.write_u32(std::process::id());
        hash.write_u64(counter);
        let id = format!("mcp-{:016x}", hash.finish());
        if !existing.iter().any(|s| s.id == id) {
            return id;
        }
    }
    // Practically unreachable; the caller treats a duplicate as a
    // registration failure rather than overwriting a server.
    String::new()
}

fn public_list(file: &McpFile) -> McpServerList {
    McpServerList {
        contract_version: 1,
        servers: file.servers.iter().map(McpServer::from).collect(),
    }
}

// ------------------------------------------------------- JSON-RPC channel

/// Minimal JSON-RPC 2.0 channel: one request, one matching response.
/// The probe is generic over this trait so unit tests can replay
/// canned transcripts without spawning a process.
trait McpChannel {
    /// Send a request and return its `result`, or the server error.
    fn request(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String>;
    /// Send a notification (no response expected).
    fn notify(&mut self, method: &str);
}

fn parse_response(line: &str, expected_id: u64) -> Result<serde_json::Value, String> {
    let value: serde_json::Value =
        serde_json::from_str(line).map_err(|_| "Server response is not JSON.".to_string())?;
    if value.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
        return Err("Server response is not JSON-RPC 2.0.".to_string());
    }
    if value.get("id").and_then(|v| v.as_u64()) != Some(expected_id) {
        return Err("Server response id does not match the request.".to_string());
    }
    if let Some(error) = value.get("error") {
        let code = error
            .get("code")
            .and_then(|c| c.as_i64())
            .unwrap_or(0);
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Server error {code}: {message}"));
    }
    value
        .get("result")
        .cloned()
        .ok_or_else(|| "Server response has neither result nor error.".to_string())
}

/// Distinguish a dead channel from a slow server: when the reader
/// thread ends (child exited, stdout EOF) the receiver disconnects,
/// and that is a crash report, not a timeout. Previously both cases
/// reported a timeout, masking a startup crash as slowness.
fn channel_recv_error(err: mpsc::RecvTimeoutError) -> String {
    match err {
        mpsc::RecvTimeoutError::Timeout => "Probe timed out waiting for the server.".to_string(),
        mpsc::RecvTimeoutError::Disconnected => {
            "Server exited or closed connection before replying.".to_string()
        }
    }
}

/// Real stdio transport: a reader thread forwards NDJSON lines to a
/// channel; requests wait on it with the probe's remaining budget.
struct StdioChannel {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<Result<String, String>>,
    next_id: u64,
    deadline: Instant,
}

impl StdioChannel {
    /// Parent-environment entries a cleared child still needs on
    /// Windows: the CRT and runtimes such as Node.js locate the
    /// system directory through `SystemRoot`/`WINDIR` and resolve
    /// helpers through `PATH`. Without them a Node MCP server on
    /// Windows aborts on startup (exit code 134,
    /// `ncrypto::CSPRNG` assertion) before running any script.
    /// Pure over its inputs so it is unit-testable on any
    /// platform; only called from `spawn` on Windows. A key the
    /// server configuration already supplies is never overridden:
    /// configured values always win (comparison is
    /// case-insensitive because Windows env names are).
    fn preserved_env_entries(
        wanted: &[&str],
        parent: &[(String, String)],
        configured: &BTreeMap<String, String>,
    ) -> Vec<(String, String)> {
        let configured_lower: Vec<String> = configured.keys().map(|k| k.to_lowercase()).collect();
        wanted
            .iter()
            .filter_map(|name| {
                if configured_lower.iter().any(|k| k == &name.to_lowercase()) {
                    return None;
                }
                parent
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == name.to_lowercase())
                    .cloned()
            })
            .collect()
    }

    fn spawn(server: &McpServerConfig) -> Result<Self, String> {
        let command = server.command.clone().unwrap_or_default();
        let mut cmd = Command::new(&command);
        cmd.args(&server.args).env_clear();
        // Windows only: restore the system paths a child runtime
        // needs to boot, then apply the configured env on top so
        // explicit configuration always wins. On other platforms
        // the child keeps exactly the configured environment.
        #[cfg(windows)]
        {
            let parent: Vec<(String, String)> = std::env::vars().collect();
            let preserved =
                Self::preserved_env_entries(&["SystemRoot", "WINDIR", "PATH"], &parent, &server.env);
            cmd.envs(preserved);
        }
        let mut child = cmd
            .envs(&server.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Could not start '{command}': {e}"))?;
        let stdin = child.stdin.take().ok_or("Server stdin unavailable.")?;
        let stdout = child.stdout.take().ok_or("Server stdout unavailable.")?;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        if line.len() > MAX_LINE_BYTES {
                            let _ = tx.send(Err("Server response line is too long.".to_string()));
                            break;
                        }
                        if tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            lines: rx,
            next_id: 1,
            deadline: Instant::now() + PROBE_TIMEOUT,
        })
    }

    fn remaining(&self) -> Result<Duration, String> {
        self.deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| "Probe timed out.".to_string())
    }

    fn shutdown(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl McpChannel for StdioChannel {
    fn request(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let line = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .map_err(|_| "Request could not be encoded.".to_string())?;
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|e| format!("Could not write to server: {e}"))?;
        let remaining = self.remaining()?;
        let line = self
            .lines
            .recv_timeout(remaining)
            .map_err(channel_recv_error)??;
        parse_response(&line, id)
    }

    fn notify(&mut self, method: &str) {
        if let Ok(line) = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
        })) {
            let _ = self.stdin.write_all(line.as_bytes());
            let _ = self.stdin.write_all(b"\n");
            let _ = self.stdin.flush();
        }
    }
}

impl Drop for StdioChannel {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ------------------------------------------------------------- probe logic

fn parse_tool(value: &serde_json::Value) -> Option<McpTool> {
    let name = value.get("name")?.as_str()?.to_string();
    if name.is_empty() || name.chars().count() > 256 {
        return None;
    }
    let description = value
        .get("description")
        .and_then(|d| d.as_str())
        .map(|d| d.chars().take(2000).collect());
    let input_schema = value.get("inputSchema").cloned();
    let annotations = value.get("annotations").map(|a| McpToolAnnotations {
        title: a
            .get("title")
            .and_then(|t| t.as_str())
            .map(|t| t.chars().take(256).collect()),
        read_only_hint: a.get("readOnlyHint").and_then(|v| v.as_bool()),
        destructive_hint: a.get("destructiveHint").and_then(|v| v.as_bool()),
        idempotent_hint: a.get("idempotentHint").and_then(|v| v.as_bool()),
        open_world_hint: a.get("openWorldHint").and_then(|v| v.as_bool()),
    });
    Some(McpTool {
        name,
        description,
        input_schema,
        annotations,
    })
}

fn parse_prompt(value: &serde_json::Value) -> Option<McpPrompt> {
    let name = value.get("name")?.as_str()?.to_string();
    if name.is_empty() || name.chars().count() > 256 {
        return None;
    }
    let description = value
        .get("description")
        .and_then(|d| d.as_str())
        .map(|d| d.chars().take(2000).collect());
    let mut arguments = Vec::new();
    if let Some(list) = value.get("arguments").and_then(|a| a.as_array()) {
        for arg in list {
            let arg_name = arg.get("name")?.as_str()?;
            arguments.push(McpPromptArgument {
                name: arg_name.to_string(),
                description: arg
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(|d| d.chars().take(2000).collect()),
                required: arg.get("required").and_then(|v| v.as_bool()),
            });
        }
    }
    Some(McpPrompt {
        name,
        description,
        arguments,
    })
}

fn parse_list<T>(
    result: &serde_json::Value,
    key: &str,
    parse: fn(&serde_json::Value) -> Option<T>,
) -> (Vec<T>, bool) {
    let items = result
        .get(key)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let truncated = items.len() > MAX_LISTED;
    let kept = items
        .iter()
        .take(MAX_LISTED)
        .filter_map(parse)
        .collect();
    (kept, truncated)
}

fn client_info() -> serde_json::Value {
    serde_json::json!({
        "name": "ANANSI",
        "version": env!("CARGO_PKG_VERSION"),
    })
}

/// Negotiated handshake, shared by probing and tool calls so both
/// speak exactly the same protocol selection. Legacy `initialize`
/// first; `server/discover` only on method-not-found; any other
/// initialize failure is the verdict. Error strings are the ones
/// probing has always reported.
struct Handshake {
    stateless: bool,
    protocol_version: Option<String>,
    server_name: Option<String>,
}

fn handshake<C: McpChannel>(channel: &mut C) -> Result<Handshake, String> {
    // Legacy path first: what deployed servers speak.
    match channel.request(
        "initialize",
        serde_json::json!({
            "protocolVersion": LEGACY_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": client_info(),
        }),
    ) {
        Ok(init) => {
            channel.notify("notifications/initialized");
            Ok(Handshake {
                stateless: false,
                protocol_version: init
                    .get("protocolVersion")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| Some(LEGACY_PROTOCOL_VERSION.to_string())),
                server_name: init
                    .get("serverInfo")
                    .and_then(|i| i.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string()),
            })
        }
        Err(message) => {
            // Only a method-not-found error justifies the stateless
            // fallback; any other initialize failure is the verdict.
            if !message.contains("Server error -32601") {
                return Err(format!("Handshake failed: {message}"));
            }
            match channel.request(
                "server/discover",
                serde_json::json!({
                    "_meta": {
                        "io.modelcontextprotocol/protocolVersion": STATELESS_PROTOCOL_VERSION,
                        "io.modelcontextprotocol/clientCapabilities": {},
                        "io.modelcontextprotocol/clientInfo": client_info(),
                    },
                }),
            ) {
                Ok(discover) => Ok(Handshake {
                    stateless: true,
                    protocol_version: Some(STATELESS_PROTOCOL_VERSION.to_string()),
                    server_name: discover
                        .get("serverInfo")
                        .and_then(|i| i.get("name"))
                        .and_then(|n| n.as_str())
                        .or_else(|| discover.get("serverName").and_then(|n| n.as_str()))
                        .map(|s| s.to_string()),
                }),
                Err(message) => Err(format!("Discovery failed: {message}")),
            }
        }
    }
}

/// Params for list calls: stateless servers require the protocol
/// version in `_meta` on every request; legacy servers take {}.
fn stateless_meta(stateless: bool) -> serde_json::Value {
    if stateless {
        serde_json::json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": STATELESS_PROTOCOL_VERSION,
                "io.modelcontextprotocol/clientCapabilities": {},
            },
        })
    } else {
        serde_json::json!({})
    }
}

/// Probe one server over an already-open channel. Returns the probe
/// result; transport failures surface as `healthy: false` with the
/// real error, never as a fabricated healthy status.
fn probe_channel<C: McpChannel>(channel: &mut C, server_id: &str) -> McpProbeResult {
    let mut result = McpProbeResult {
        contract_version: 1,
        server_id: server_id.to_string(),
        healthy: false,
        protocol_version: None,
        health_method: None,
        server_name: None,
        tools: Vec::new(),
        prompts: Vec::new(),
        truncated: false,
        error: None,
    };
    let handshake = match handshake(channel) {
        Ok(handshake) => handshake,
        Err(message) => {
            result.error = Some(message);
            return result;
        }
    };
    let stateless = handshake.stateless;
    result.protocol_version = handshake.protocol_version;
    result.server_name = handshake.server_name;
    match channel.request("tools/list", stateless_meta(stateless)) {
        Ok(list) => {
            let (tools, truncated) = parse_list(&list, "tools", parse_tool);
            result.tools = tools;
            result.truncated = truncated;
        }
        Err(message) => {
            result.error = Some(format!("tools/list failed: {message}"));
            return result;
        }
    }
    match channel.request("prompts/list", stateless_meta(stateless)) {
        Ok(list) => {
            let (prompts, truncated) = parse_list(&list, "prompts", parse_prompt);
            result.prompts = prompts;
            result.truncated = result.truncated || truncated;
        }
        Err(message) => {
            result.error = Some(format!("prompts/list failed: {message}"));
            return result;
        }
    }
    if stateless {
        // 2026-07-28 removed ping: discovery success IS the health
        // signal. Never report a ping that never happened.
        result.healthy = true;
        result.health_method = Some("discover".to_string());
    } else {
        match channel.request("ping", serde_json::json!({})) {
            Ok(_) => {
                result.healthy = true;
                result.health_method = Some("ping".to_string());
            }
            Err(message) => {
                result.error = Some(format!("Ping failed: {message}"));
            }
        }
    }
    result
}

fn probe_server(server: &McpServerConfig) -> McpProbeResult {
    if server.transport == "sse" {
        return McpProbeResult {
            contract_version: 1,
            server_id: server.id.clone(),
            healthy: false,
            protocol_version: None,
            health_method: None,
            server_name: None,
            tools: Vec::new(),
            prompts: Vec::new(),
            truncated: false,
            error: Some(
                "Remote MCP servers cannot be probed in this slice: external network access is disabled by default (MASTER_SPEC §68). Only local stdio servers can be connected."
                    .to_string(),
            ),
        };
    }
    let mut channel = match StdioChannel::spawn(server) {
        Ok(channel) => channel,
        Err(message) => {
            return McpProbeResult {
                contract_version: 1,
                server_id: server.id.clone(),
                healthy: false,
                protocol_version: None,
                health_method: None,
                server_name: None,
                tools: Vec::new(),
                prompts: Vec::new(),
                truncated: false,
                error: Some(message),
            };
        }
    };
    probe_channel(&mut channel, &server.id)
}

// ------------------------------------------------------- tool invocation

/// Conservative JSON Schema subset for tool arguments. Enforced,
/// and only this: arguments must be an object; every `required`
/// property must be present (presence, exactly JSON Schema
/// semantics); present properties are type-checked; `enum`
/// membership is checked. Not enforced, by design:
/// `additionalProperties`, string/number bounds, nested schemas,
/// `format`. A tool with no usable schema is called unvalidated
/// rather than refused — the UI says so instead of pretending.
fn validate_tool_arguments(
    schema: Option<&serde_json::Value>,
    arguments: &serde_json::Value,
) -> Result<(), String> {
    let args = arguments
        .as_object()
        .ok_or_else(|| "Tool arguments must be a JSON object.".to_string())?;
    let Some(schema) = schema.and_then(|s| s.as_object()) else {
        return Ok(());
    };
    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
        for name in required.iter().filter_map(|r| r.as_str()) {
            if !args.contains_key(name) {
                return Err(format!("Missing required parameter: {name}"));
            }
        }
    }
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        for (name, value) in args {
            let Some(property) = properties.get(name) else {
                continue;
            };
            if let Some(allowed) = property.get("enum").and_then(|e| e.as_array()) {
                if !allowed.contains(value) {
                    return Err(format!(
                        "Parameter '{name}' must be one of the allowed values."
                    ));
                }
            }
            let Some(kind) = property.get("type").and_then(|t| t.as_str()) else {
                continue;
            };
            let ok = match kind {
                "string" => value.is_string(),
                "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
                "number" => value.is_number(),
                "boolean" => value.is_boolean(),
                "array" => value.is_array(),
                "object" => value.is_object(),
                _ => true,
            };
            if !ok {
                return Err(format!("Parameter '{name}' must be of type {kind}."));
            }
        }
    }
    Ok(())
}

/// One tool dispatch over an already-open channel: handshake,
/// fresh `tools/list`, schema validation against the schema the
/// server just offered (never a client-supplied one), then
/// `tools/call`. The channel — and its 30s budget and child
/// teardown — is the caller's.
fn call_tool_channel<C: McpChannel>(
    channel: &mut C,
    server_id: &str,
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<McpToolCallResult, String> {
    let handshake = handshake(channel)?;
    let list = channel
        .request("tools/list", stateless_meta(handshake.stateless))
        .map_err(|message| format!("tools/list failed: {message}"))?;
    let (tools, _) = parse_list(&list, "tools", parse_tool);
    let tool = tools
        .iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| format!("Tool '{tool_name}' was not offered by this server."))?;
    validate_tool_arguments(tool.input_schema.as_ref(), arguments)?;
    let mut params = serde_json::json!({
        "name": tool_name,
        "arguments": arguments,
    });
    if handshake.stateless {
        params["_meta"] = serde_json::json!({
            "io.modelcontextprotocol/protocolVersion": STATELESS_PROTOCOL_VERSION,
            "io.modelcontextprotocol/clientCapabilities": {},
        });
    }
    let outcome = channel.request("tools/call", params)?;
    let mut content = Vec::new();
    let mut texts = Vec::new();
    if let Some(parts) = outcome.get("content").and_then(|c| c.as_array()) {
        for part in parts {
            let kind = part
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("text")
                .to_string();
            let text = part.get("text").and_then(|t| t.as_str()).map(|s| s.to_string());
            if let Some(text) = &text {
                texts.push(text.clone());
            }
            content.push(McpToolCallContent {
                content_type: kind,
                text,
            });
        }
    }
    Ok(McpToolCallResult {
        contract_version: 1,
        server_id: server_id.to_string(),
        tool_name: tool_name.to_string(),
        is_error: outcome
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        text: texts.join("\n"),
        content,
        structured_content: outcome.get("structuredContent").cloned(),
    })
}

fn call_tool_server(
    server: &McpServerConfig,
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<McpToolCallResult, CommandError> {
    if server.transport == "sse" {
        return Err(invalid(
            "Remote MCP servers cannot be called: external network access is disabled by default (MASTER_SPEC §68). Only local stdio servers can be called.",
        ));
    }
    if tool_name.is_empty() || tool_name.chars().count() > 256 || has_control(tool_name) {
        return Err(invalid(
            "Tool name must be 1 to 256 characters without control characters.",
        ));
    }
    let mut channel = StdioChannel::spawn(server)
        .map_err(|message| CommandError::new(ErrorCode::InvalidRequest, &message))?;
    // The error enum has no execution-failure code, so every call
    // failure surfaces as InvalidRequest carrying the real message.
    call_tool_channel(&mut channel, &server.id, tool_name, arguments)
        .map_err(|message| CommandError::new(ErrorCode::InvalidRequest, &message))
}

// ------------------------------------------------------------ core command

impl crate::readiness::Core {
    pub fn mcp_command(
        &self,
        command: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, CommandError> {
        if input
            .get("contractVersion")
            .and_then(serde_json::Value::as_u64)
            != Some(1)
        {
            return Err(CommandError::new(
                ErrorCode::UnsupportedContract,
                "Use contract version 1.",
            ));
        }
        let root = self.storage_root()?;
        let workspace_id = input
            .get("workspaceId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if workspace_id.is_empty() || workspace_id.chars().count() > 256 {
            return Err(invalid("workspaceId must be 1 to 256 characters."));
        }
        match command {
            "servers-list" => {
                let _request: McpListRequest = serde_json::from_value(input)
                    .map_err(|_| invalid("Use a supported MCP list request."))?;
                let file = load_mcp_file(&root, &workspace_id)?;
                serde_json::to_value(public_list(&file)).map_err(|_| unavailable())
            }
            "servers-register" => {
                let request: McpRegisterRequest = serde_json::from_value(input)
                    .map_err(|_| invalid("Use a supported MCP register request."))?;
                validate_draft(&request.draft)?;
                let mut file = load_mcp_file(&root, &workspace_id)?;
                if file.servers.len() >= MAX_SERVERS_PER_WORKSPACE {
                    return Err(invalid("This workspace already has 64 MCP servers."));
                }
                let name = request.draft.name.trim().to_string();
                if file
                    .servers
                    .iter()
                    .any(|s| s.name.to_lowercase() == name.to_lowercase())
                {
                    return Err(invalid("A server with this name already exists."));
                }
                let id = new_server_id(&file.servers);
                if id.is_empty() {
                    return Err(unavailable());
                }
                let config = McpServerConfig {
                    id,
                    name,
                    transport: request.draft.transport,
                    command: request.draft.command.map(|c| c.trim().to_string()),
                    args: request.draft.args,
                    env: request.draft.env,
                    url: request.draft.url,
                };
                file.servers.push(config.clone());
                persist_mcp_file(&root, &workspace_id, &file)?;
                serde_json::to_value(McpServer::from(&config)).map_err(|_| unavailable())
            }
            "servers-remove" => {
                let request: McpRemoveRequest = serde_json::from_value(input)
                    .map_err(|_| invalid("Use a supported MCP remove request."))?;
                let mut file = load_mcp_file(&root, &workspace_id)?;
                let before = file.servers.len();
                file.servers.retain(|s| s.id != request.id);
                let removed = file.servers.len() < before;
                if removed {
                    persist_mcp_file(&root, &workspace_id, &file)?;
                }
                serde_json::to_value(McpRemoveResult {
                    contract_version: 1,
                    removed,
                })
                    .map_err(|_| unavailable())
            }
            "servers-probe" => {
                let request: McpProbeRequest = serde_json::from_value(input)
                    .map_err(|_| invalid("Use a supported MCP probe request."))?;
                let file = load_mcp_file(&root, &workspace_id)?;
                let server = file
                    .servers
                    .iter()
                    .find(|s| s.id == request.id)
                    .ok_or_else(|| {
                        CommandError::new(ErrorCode::NotFound, "MCP server not found.")
                    })?;
                let result = probe_server(server);
                serde_json::to_value(result).map_err(|_| unavailable())
            }
            "tools-call" => {
                let request: McpToolCallRequest = serde_json::from_value(input)
                    .map_err(|_| invalid("Use a supported MCP tool call request."))?;
                let file = load_mcp_file(&root, &workspace_id)?;
                let server = file
                    .servers
                    .iter()
                    .find(|s| s.id == request.id)
                    .ok_or_else(|| {
                        CommandError::new(ErrorCode::NotFound, "MCP server not found.")
                    })?;
                let result = call_tool_server(server, &request.tool_name, &request.arguments)?;
                serde_json::to_value(result).map_err(|_| unavailable())
            }
            _ => Err(invalid("Use a supported MCP operation.")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Canned channel: scripted responses in order, methods recorded.
    struct CannedChannel {
        responses: VecDeque<serde_json::Value>,
        methods: Vec<String>,
    }

    impl CannedChannel {
        fn new(responses: Vec<serde_json::Value>) -> Self {
            Self {
                responses: responses.into_iter().collect(),
                methods: Vec::new(),
            }
        }

        fn result(value: serde_json::Value) -> serde_json::Value {
            serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": value})
        }

        fn error(code: i64, message: &str) -> serde_json::Value {
            serde_json::json!({"jsonrpc": "2.0", "id": 1, "error": {"code": code, "message": message}})
        }
    }

    impl McpChannel for CannedChannel {
        fn request(
            &mut self,
            method: &str,
            _params: serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            self.methods.push(method.to_string());
            let envelope = self
                .responses
                .pop_front()
                .ok_or_else(|| "No scripted response left.".to_string())?;
            parse_response(&envelope.to_string(), 1)
        }

        fn notify(&mut self, _method: &str) {}
    }

    fn legacy_transcript() -> Vec<serde_json::Value> {
        vec![
            CannedChannel::result(serde_json::json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "serverInfo": {"name": "Fake MCP", "version": "1.0"},
            })),
            CannedChannel::result(serde_json::json!({
                "tools": [
                    {"name": "search", "description": "Search memory.",
                     "inputSchema": {"type": "object", "properties": {"q": {"type": "string"}}, "required": ["q"]},
                     "annotations": {"readOnlyHint": true}},
                    {"name": "write"},
                ],
            })),
            CannedChannel::result(serde_json::json!({
                "prompts": [
                    {"name": "summarize", "description": "Summarize.", "arguments": [{"name": "text", "required": true}]},
                ],
            })),
            CannedChannel::result(serde_json::json!({})),
        ]
    }

    #[test]
    fn legacy_probe_lists_tools_prompts_and_pings() {
        let mut channel = CannedChannel::new(legacy_transcript());
        let result = probe_channel(&mut channel, "mcp-1");
        assert!(result.healthy);
        assert_eq!(result.health_method.as_deref(), Some("ping"));
        assert_eq!(result.protocol_version.as_deref(), Some("2025-11-25"));
        assert_eq!(result.server_name.as_deref(), Some("Fake MCP"));
        assert_eq!(result.tools.len(), 2);
        assert_eq!(result.tools[0].name, "search");
        assert_eq!(
            result.tools[0].annotations.as_ref().unwrap().read_only_hint,
            Some(true)
        );
        assert_eq!(result.prompts.len(), 1);
        assert_eq!(result.prompts[0].arguments[0].name, "text");
        assert_eq!(
            channel.methods,
            vec!["initialize", "tools/list", "prompts/list", "ping"]
        );
    }

    #[test]
    fn stateless_probe_falls_back_to_discover_and_never_pings() {
        let mut channel = CannedChannel::new(vec![
            CannedChannel::error(-32601, "Method not found"),
            CannedChannel::result(serde_json::json!({
                "serverInfo": {"name": "Stateless MCP"},
            })),
            CannedChannel::result(serde_json::json!({"tools": []})),
            CannedChannel::result(serde_json::json!({"prompts": []})),
        ]);
        let result = probe_channel(&mut channel, "mcp-2");
        assert!(result.healthy);
        assert_eq!(result.health_method.as_deref(), Some("discover"));
        assert_eq!(result.protocol_version.as_deref(), Some("2026-07-28"));
        assert_eq!(result.server_name.as_deref(), Some("Stateless MCP"));
        assert!(!channel.methods.contains(&"ping".to_string()));
    }

    #[test]
    fn non_not_found_initialize_error_is_the_verdict() {
        let mut channel = CannedChannel::new(vec![
            CannedChannel::error(-32602, "Invalid params"),
        ]);
        let result = probe_channel(&mut channel, "mcp-3");
        assert!(!result.healthy);
        assert!(result.error.as_deref().unwrap().contains("Handshake failed"));
        assert_eq!(channel.methods, vec!["initialize"]);
    }

    #[test]
    fn malformed_response_line_is_an_error_not_a_crash() {
        assert!(parse_response("not json", 1).is_err());
        assert!(parse_response(r#"{"jsonrpc":"2.0","id":2,"result":{}}"#, 1).is_err());
        assert!(parse_response(r#"{"jsonrpc":"1.0","id":1,"result":{}}"#, 1).is_err());
    }

    #[test]
    fn tool_call_decodes_text_and_structured_content() {
        let mut channel = CannedChannel::new(vec![
            CannedChannel::result(serde_json::json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "serverInfo": {"name": "Fake MCP", "version": "1.0"},
            })),
            CannedChannel::result(serde_json::json!({
                "tools": [
                    {"name": "search", "description": "Search memory.",
                     "inputSchema": {"type": "object", "properties": {"q": {"type": "string"}}, "required": ["q"]},
                     "annotations": {"readOnlyHint": true}},
                ],
            })),
            CannedChannel::result(serde_json::json!({
                "content": [{"type": "text", "text": "Search results for \"cats\"."}],
                "structuredContent": {"count": 1},
            })),
        ]);
        let result = call_tool_channel(
            &mut channel,
            "mcp-1",
            "search",
            &serde_json::json!({"q": "cats"}),
        )
        .expect("call succeeds");
        assert_eq!(result.server_id, "mcp-1");
        assert_eq!(result.tool_name, "search");
        assert!(!result.is_error);
        assert_eq!(result.text, "Search results for \"cats\".");
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].content_type, "text");
        assert_eq!(
            result.structured_content,
            Some(serde_json::json!({"count": 1}))
        );
        assert_eq!(
            channel.methods,
            vec!["initialize", "tools/list", "tools/call"]
        );
    }

    #[test]
    fn tool_call_stateless_fallback_uses_discover() {
        let mut channel = CannedChannel::new(vec![
            CannedChannel::error(-32601, "Method not found"),
            CannedChannel::result(serde_json::json!({
                "serverInfo": {"name": "Stateless Fake", "version": "1.0"},
            })),
            CannedChannel::result(serde_json::json!({
                "tools": [
                    {"name": "search",
                     "inputSchema": {"type": "object", "properties": {"q": {"type": "string"}}, "required": ["q"]}},
                ],
            })),
            CannedChannel::result(serde_json::json!({
                "content": [{"type": "text", "text": "ok"}],
            })),
        ]);
        let result = call_tool_channel(
            &mut channel,
            "mcp-1",
            "search",
            &serde_json::json!({"q": "x"}),
        )
        .expect("stateless call succeeds");
        assert_eq!(result.text, "ok");
        assert_eq!(
            channel.methods,
            vec!["initialize", "server/discover", "tools/list", "tools/call"]
        );
    }

    #[test]
    fn tool_call_unknown_tool_is_refused_before_dispatch() {
        let mut channel = CannedChannel::new(vec![
            CannedChannel::result(serde_json::json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "serverInfo": {"name": "Fake MCP", "version": "1.0"},
            })),
            CannedChannel::result(serde_json::json!({
                "tools": [{"name": "search"}],
            })),
        ]);
        let err = call_tool_channel(
            &mut channel,
            "mcp-1",
            "nope",
            &serde_json::json!({}),
        )
        .expect_err("unknown tool must be refused");
        assert!(err.contains("was not offered"), "{err}");
        assert_eq!(channel.methods, vec!["initialize", "tools/list"]);
    }

    #[test]
    fn tool_call_is_error_is_a_completed_result() {
        let mut channel = CannedChannel::new(vec![
            CannedChannel::result(serde_json::json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "serverInfo": {"name": "Fake MCP", "version": "1.0"},
            })),
            CannedChannel::result(serde_json::json!({
                "tools": [{"name": "search"}],
            })),
            CannedChannel::result(serde_json::json!({
                "content": [{"type": "text", "text": "boom"}],
                "isError": true,
            })),
        ]);
        let result = call_tool_channel(
            &mut channel,
            "mcp-1",
            "search",
            &serde_json::json!({}),
        )
        .expect("tool-level error is still a result");
        assert!(result.is_error);
        assert_eq!(result.text, "boom");
    }

    #[test]
    fn tool_argument_validation_covers_required_types_and_enum() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "q": {"type": "string"},
                "n": {"type": "integer"},
                "mode": {"type": "string", "enum": ["fast", "deep"]},
            },
            "required": ["q"],
        });
        assert!(validate_tool_arguments(Some(&schema), &serde_json::json!({"q": "x"})).is_ok());
        assert!(validate_tool_arguments(Some(&schema), &serde_json::json!({})).is_err());
        assert!(validate_tool_arguments(
            Some(&schema),
            &serde_json::json!({"q": 3})
        )
        .is_err());
        assert!(validate_tool_arguments(
            Some(&schema),
            &serde_json::json!({"q": "x", "n": 1.5})
        )
        .is_err());
        assert!(validate_tool_arguments(
            Some(&schema),
            &serde_json::json!({"q": "x", "n": 2})
        )
        .is_ok());
        assert!(validate_tool_arguments(
            Some(&schema),
            &serde_json::json!({"q": "x", "mode": "slow"})
        )
        .is_err());
        assert!(validate_tool_arguments(
            Some(&schema),
            &serde_json::json!({"q": "x", "mode": "deep"})
        )
        .is_ok());
        assert!(validate_tool_arguments(Some(&schema), &serde_json::json!("nope")).is_err());
        assert!(validate_tool_arguments(None, &serde_json::json!({"anything": true})).is_ok());
    }

    #[test]
    fn draft_validation_refuses_bad_transports_and_fields() {
        let base = McpServerDraft {
            name: "Local".to_string(),
            transport: "stdio".to_string(),
            command: Some("node".to_string()),
            args: Vec::new(),
            env: BTreeMap::new(),
            url: None,
        };
        assert!(validate_draft(&base).is_ok());
        let mut empty_name = base.clone();
        empty_name.name = "   ".to_string();
        assert!(validate_draft(&empty_name).is_err());
        let mut no_command = base.clone();
        no_command.command = None;
        assert!(validate_draft(&no_command).is_err());
        let mut with_url = base.clone();
        with_url.url = Some("https://example.test/mcp".to_string());
        assert!(validate_draft(&with_url).is_err());
        let mut bad_transport = base.clone();
        bad_transport.transport = "tcp".to_string();
        assert!(validate_draft(&bad_transport).is_err());
        let sse = McpServerDraft {
            name: "Remote".to_string(),
            transport: "sse".to_string(),
            command: None,
            args: Vec::new(),
            env: BTreeMap::new(),
            url: Some("ftp://example.test/mcp".to_string()),
        };
        assert!(validate_draft(&sse).is_err());
        let mut sse_ok = sse.clone();
        sse_ok.url = Some("https://example.test/mcp".to_string());
        assert!(validate_draft(&sse_ok).is_ok());
        let mut bad_env = base.clone();
        bad_env.env.insert("BAD-KEY".to_string(), "v".to_string());
        assert!(validate_draft(&bad_env).is_err());
    }

    #[test]
    fn preserved_env_keeps_system_vars_and_config_wins() {
        let parent = vec![
            ("SystemRoot".to_string(), "C:\\Windows".to_string()),
            ("WINDIR".to_string(), "C:\\Windows".to_string()),
            ("PATH".to_string(), "C:\\Windows\\System32".to_string()),
            ("FAKE_TOKEN".to_string(), "must-not-leak".to_string()),
        ];
        // Configured `Path` (different case) suppresses the parent's
        // PATH: explicit configuration always wins on Windows.
        let mut configured = BTreeMap::new();
        configured.insert("Path".to_string(), "C:\\custom\\bin".to_string());
        let kept = StdioChannel::preserved_env_entries(
            &["SystemRoot", "WINDIR", "PATH"],
            &parent,
            &configured,
        );
        assert_eq!(kept.len(), 2);
        assert!(kept.contains(&("SystemRoot".to_string(), "C:\\Windows".to_string())));
        assert!(kept.contains(&("WINDIR".to_string(), "C:\\Windows".to_string())));
        assert!(!kept.iter().any(|(k, _)| k == "PATH"));
        assert!(!kept.iter().any(|(k, _)| k == "FAKE_TOKEN"));
    }

    #[test]
    fn channel_recv_error_distinguishes_disconnect_from_timeout() {
        assert_eq!(
            channel_recv_error(mpsc::RecvTimeoutError::Timeout),
            "Probe timed out waiting for the server."
        );
        assert_eq!(
            channel_recv_error(mpsc::RecvTimeoutError::Disconnected),
            "Server exited or closed connection before replying."
        );
    }

    // A child that reads one stdin line and exits without replying
    // must surface the disconnect message, not a timeout. Unix-only:
    // it relies on `sh`, which Windows test runs do not have.
    #[test]
    #[cfg(unix)]
    fn stdio_channel_reports_disconnect_when_child_exits_silently() {
        let server = McpServerConfig {
            id: "mcp-test".to_string(),
            name: "Silent".to_string(),
            transport: "stdio".to_string(),
            command: Some("sh".to_string()),
            args: vec!["-c".to_string(), "read x".to_string()],
            env: BTreeMap::new(),
            url: None,
        };
        let mut channel = StdioChannel::spawn(&server).expect("spawn silent child");
        let err = channel
            .request("initialize", serde_json::json!({}))
            .expect_err("silent child must not answer");
        assert_eq!(err, "Server exited or closed connection before replying.");
    }
}
