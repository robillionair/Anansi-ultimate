//! Application integration for the M5 provider subsystem, with the
//! authorization boundary repaired:
//!
//! - **Policy comes from backend state, never from dispatch JSON.**
//!   Configuration is installed by `providers-configure`; paid
//!   authorization is issued by `providers-authorize`. The dispatch
//!   and stream operations accept only a completion request, routing
//!   choices, and (optionally) the *id* of a backend-issued permit.
//!   Locality is **derived** from the validated endpoint at
//!   configure time — a configuration whose declared locality
//!   contradicts the derivation is rejected, so a cloud endpoint
//!   cannot masquerade as Local, and model cost classes exist only
//!   inside the trusted configuration, never in a dispatch request.
//! - **Caller binding.** Every operation runs under a
//!   [`CallerScope`] (session + workspace). The API layer stamps the
//!   session from the authenticated bearer token (overwriting any
//!   client-supplied value — see `crates/api`); in-process host
//!   calls use the host session. Permits are bound to the issuing
//!   session; stream runs are bound to the starting session and
//!   workspace.
//! - **Logical workspace scoping.** The workspace in the scope is
//!   a logical workspace id, validated by `Core` through the
//!   existing storage registry — several workspaces share one
//!   storage root, and provider state (configuration, permits,
//!   grants, accounting, runs) is isolated per workspace, not per
//!   root.
//! - **Owner authorization.** `providers-configure` and
//!   `providers-authorize` additionally require the owner: the
//!   host session (the desktop process — the vault's Desktop
//!   transport), or an API session holding a live grant from
//!   `providers-grant-session` (host-only, revocable, expiring).
//!   Moving policy into a separate endpoint would establish
//!   nothing on its own; the check is on who calls.
//! - **Lock discipline.** The backend mutex is held only for
//!   state surgery. Authorization decisions (model universe,
//!   permit, runtime resolution, stream-cap pre-check) are taken
//!   atomically under the lock; the network dispatch itself runs
//!   unlocked on an `Arc<ProviderRuntime>`, so poll/cancel and
//!   other workspaces progress during a slow request.
//! - **Run capabilities.** `providers-stream-start` returns an
//!   unpredictable 128-bit capability next to the run id. Poll and
//!   cancel require both, compared in constant time; any mismatch —
//!   wrong capability, wrong session, wrong workspace — is answered
//!   `NotFound`, so run existence is not an oracle.
//! - **Accounting survives configuration changes.** The permit set
//!   and the quota ledger live in the per-workspace backend state,
//!   created once and shared into every runtime built from it
//!   (`ProviderRuntime::with_shared_accounting`). Configuring B
//!   after A and back cannot resurrect a consumed permit or reset
//!   usage. Budgets come from the trusted configuration and are
//!   enforced with reservations inside the providers crate.
//! - **Bounded streams.** At most [`MAX_RETAINED_STREAMS`] runs are
//!   retained per workspace; runs expire after [`STREAM_TTL`]
//!   (cancelled, then dropped) and finished runs are removed by
//!   their terminal poll. Expired permits are swept on every
//!   operation.
//! - **Strict request bounds** are enforced before any state change
//!   or network traffic (see `validate_*_bounds`); the same bounds
//!   are published in the generated contracts
//!   (`crates/contracts`, `packages/contracts`).
//!
//! Credentials never pass through this module: configurations carry
//! credential *references* only, resolved inside the runtime by the
//! native keyring resolver.

use crate::readiness::{CommandError, ErrorCode};
use anansi_providers::dispatcher::StreamOutcome;
use anansi_providers::error::{ProviderError, ProviderErrorCode};
use anansi_providers::gates::{PaidInferencePermit, QuotaLedger, ZeroCostLock};
use anansi_providers::http::CancellationToken;
use anansi_providers::routing::{DisclosureRecord, RoutingMode, RoutingRequest};
use anansi_providers::runtime::{LocalityConfig, ProviderRuntime, RuntimeConfig};
use anansi_providers::types::{CompletionRequest, CostClass, StreamEvent, Usage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Bounds (mirrored in the generated contracts)

pub const MAX_PROVIDERS: usize = 32;
pub const MAX_MODELS_PER_PROVIDER: usize = 128;
pub const MAX_BASE_URL_CHARS: usize = 2048;
pub const MAX_MESSAGES: usize = 256;
pub const MAX_SEGMENTS_PER_MESSAGE: usize = 64;
pub const MAX_SEGMENT_CHARS: usize = 32_768;
pub const MAX_TOTAL_MESSAGE_CHARS: usize = 262_144;
pub const MAX_OUTPUT_TOKENS: u32 = 1_000_000;
pub const MAX_PERMIT_TOKENS: u64 = 10_000_000;
pub const MIN_PERMIT_TTL_SECONDS: u64 = 30;
pub const MAX_PERMIT_TTL_SECONDS: u64 = 3_600;
pub const DEFAULT_PERMIT_TTL_SECONDS: u64 = 600;
pub const MAX_RETAINED_STREAMS: usize = 32;
pub const STREAM_TTL: Duration = Duration::from_secs(600);

// ---------------------------------------------------------------------------
// Caller scope

/// Who is calling, as established by the transport layer (the API
/// stamps the session from the authenticated token; the workspace
/// is derived by `Core` from its own storage, never from request
/// JSON).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerScope {
    pub session: String,
    pub workspace: String,
    /// Storage root + logical workspace id, when the caller has
    /// them (the ai_command arm always does). Used to persist the
    /// trusted configuration transactionally with its in-memory
    /// install; `None` for embedders without storage.
    pub storage_root: Option<std::path::PathBuf>,
    pub logical_workspace: Option<String>,
}

/// Small stable fingerprint (FNV-1a 64) for scope labels. These
/// labels are never secrets and never leave the process; they only
/// need to be stable and collision-resistant enough to separate
/// workspaces/sessions.
pub fn fingerprint(text: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Session label for in-process host calls (the desktop app —
/// the owner's own process, the Desktop transport in vault terms).
/// API requests always carry a stamped fingerprint instead, and
/// the stamp cannot be removed or forged by a browser client
/// because the API layer inserts it unconditionally.
pub const HOST_SESSION: &str = "host-session";

fn random_capability(prefix: &str) -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("OS randomness for capabilities");
    let mut out = String::with_capacity(prefix.len() + 32);
    out.push_str(prefix);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn constant_time_eq(a: &str, b: &str) -> bool {
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

// ---------------------------------------------------------------------------
// Contract DTOs (generated schemas live in packages/contracts)

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCatalogEntry {
    pub id: String,
    pub display_name: String,
    pub base_url: Option<String>,
    pub docs_url: Option<String>,
    /// "Local" or "Cloud" (the catalog's declared locality; a
    /// configuration's locality is always re-derived from the
    /// endpoint at configure time).
    pub locality: String,
    /// "Ready", "CatalogOnly" or "Deferred".
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCatalogResponse {
    pub contract_version: u8,
    pub providers: Vec<ProviderCatalogEntry>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderConfigureRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    /// Logical workspace (validated by `Core` against the storage
    /// registry before this DTO is ever parsed).
    pub workspace_id: String,
    pub config: RuntimeConfig,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigureResponse {
    pub contract_version: u8,
    pub providers: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderAuthorizeRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub provider: String,
    pub model: String,
    #[schemars(range(min = 1, max = 10_000_000))]
    pub max_estimated_total_tokens: u64,
    #[serde(default)]
    pub allow_unknown_cost: bool,
    #[schemars(range(min = 30, max = 3_600))]
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAuthorizeResponse {
    pub contract_version: u8,
    pub permit_id: String,
    pub expires_at_ms: u64,
}

/// Dispatch and stream-start share one request shape. It carries
/// **no** configuration, locality, cost or credential material:
/// policy is resolved from backend state only.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderDispatchRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub request: CompletionRequest,
    pub mode: Option<String>,
    #[serde(default)]
    pub zero_cost_lock: bool,
    pub chosen: Option<(String, String)>,
    /// The id of a permit issued by `providers-authorize` to this
    /// session. The permit itself (scope, caps, expiry) lives in
    /// backend state; presenting an id never creates authorization.
    pub permit_id: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderResponseBody {
    pub text: String,
    pub model: String,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDispatchResponse {
    pub contract_version: u8,
    pub response: ProviderResponseBody,
    pub disclosure: DisclosureRecord,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStreamStartResponse {
    pub contract_version: u8,
    pub run_id: String,
    /// Unpredictable per-run capability; poll/cancel require it.
    pub capability: String,
    pub disclosure: DisclosureRecord,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderRunRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub run_id: String,
    pub capability: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderWorkspaceRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderGrantRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    /// The session label to grant (for API sessions, the token
    /// fingerprint the API stamps — the host knows it from the
    /// pairing record; the label is not a secret).
    pub session: String,
    #[schemars(range(min = 30, max = 3_600))]
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCredentialRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub provider: String,
    /// The keyring account label. A reference, never a secret:
    /// the secret itself only ever moves between the native
    /// prompt and the OS credential store.
    pub reference: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredentialSetResponse {
    pub contract_version: u8,
    pub stored: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredentialRemoveResponse {
    pub contract_version: u8,
    pub removed: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredentialStatusResponse {
    pub contract_version: u8,
    pub present: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPendingAccess {
    pub session: String,
    pub first_seen_ms: u64,
    pub last_seen_ms: u64,
    pub attempts: u32,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderGrantInfo {
    pub session: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSessionAccessResponse {
    pub contract_version: u8,
    pub pending: Vec<ProviderPendingAccess>,
    pub grants: Vec<ProviderGrantInfo>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStreamEvent {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPollResponse {
    pub contract_version: u8,
    pub events: Vec<ProviderStreamEvent>,
    pub done: bool,
    pub disclosure: DisclosureRecord,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageEntry {
    pub provider: String,
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageResponse {
    pub contract_version: u8,
    pub total_requests: u64,
    pub reserved_requests: u64,
    pub reserved_tokens: u64,
    pub providers: Vec<ProviderUsageEntry>,
}

// ---------------------------------------------------------------------------
// Backend state (per workspace; accounting created once, never reset)

struct PermitRecord {
    permit: PaidInferencePermit,
    session: String,
    expires_at: Instant,
}

struct StreamRun {
    outcome: StreamOutcome,
    cancel: CancellationToken,
    capability: String,
    session: String,
    started: Instant,
    done: bool,
}

/// A browser session that attempted an owner operation and was
/// refused: the host's approval queue. Bounded and swept (see
/// `record_pending_access`).
#[derive(Debug, Clone)]
struct PendingAccess {
    session: String,
    first_seen_ms: u64,
    last_seen_ms: u64,
    attempts: u32,
}

#[derive(Default)]
struct BackendState {
    pending_access: Vec<PendingAccess>,
    config: Option<RuntimeConfig>,
    /// Owner-authorization grants: session label -> expiry. Only
    /// the host can create or revoke these; they are what lets a
    /// browser (API) session configure providers or issue permits.
    grants: HashMap<String, Instant>,
    permits: HashMap<String, PermitRecord>,
    used_permits: Arc<Mutex<HashSet<String>>>,
    ledger: Arc<Mutex<QuotaLedger>>,
    runtime: Option<(String, Arc<ProviderRuntime>)>,
    streams: HashMap<String, StreamRun>,
    stream_seq: u64,
}

/// Backend key for a logical workspace that has been validated
/// against the storage registry. Validation is cached (positive
/// results only): workspace ids are created through the domain
/// operations and never deleted in this codebase, so a validated
/// (root, workspace) pair stays valid for the process. Without the
/// cache every operation would pay the registry's per-open cost —
/// on Windows `Storage::list()` spawns `powershell.exe` twice per
/// connection open (root + database permission verification),
/// which made even stream polling take ~1.5 s there while Linux
/// (a local metadata check) never noticed.
pub fn validated_workspace_key(
    root_fingerprint: &str,
    workspace_id: &str,
    list_ids: impl FnOnce() -> Result<Vec<String>, CommandError>,
) -> Result<String, CommandError> {
    let key = addressed_workspace_key(root_fingerprint, workspace_id);
    let cache = VALIDATED_WORKSPACES.get_or_init(|| Mutex::new(std::collections::HashSet::new()));
    if cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains(&key)
    {
        return Ok(key);
    }
    let ids = list_ids()?;
    if !ids.iter().any(|id| id == workspace_id) {
        return Err(CommandError::new(ErrorCode::NotFound, "Unknown workspace."));
    }
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key.clone());
    Ok(key)
}

/// The backend key for a workspace id WITHOUT registry
/// validation. Poll/cancel use this: they create nothing and
/// authorize purely through the run record (existence in that
/// workspace's backend + session + capability), so an invented
/// workspace id simply finds no run — the same `NotFound` a wrong
/// capability produces. Skipping the registry read keeps
/// poll/cancel at mutex-plus-channel cost on every platform.
pub fn addressed_workspace_key(root_fingerprint: &str, workspace_id: &str) -> String {
    format!("{root_fingerprint}/{workspace_id}")
}

static VALIDATED_WORKSPACES: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();

fn backends() -> &'static Mutex<HashMap<String, BackendState>> {
    static BACKENDS: OnceLock<Mutex<HashMap<String, BackendState>>> = OnceLock::new();
    BACKENDS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Like [`with_backend`] but never creates state: used by
/// poll/cancel, which must not conjure a backend for a workspace
/// that was never configured (and which answer the run-binding
/// `NotFound` when there is nothing to find).
fn with_backend_existing<T>(
    scope: &CallerScope,
    f: impl FnOnce(&mut BackendState) -> Result<T, CommandError>,
) -> Result<T, CommandError> {
    let mut guard = backends().lock().unwrap_or_else(|e| e.into_inner());
    let Some(state) = guard.get_mut(&scope.workspace) else {
        return Err(CommandError::new(
            ErrorCode::NotFound,
            "Unknown provider stream run.",
        ));
    };
    sweep(state);
    f(state)
}

fn with_backend<T>(
    scope: &CallerScope,
    f: impl FnOnce(&mut BackendState) -> Result<T, CommandError>,
) -> Result<T, CommandError> {
    let mut guard = backends()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let state = guard.entry(scope.workspace.clone()).or_default();
    sweep(state);
    f(state)
}

/// Expiry for permits and streams, run on every operation.
fn sweep(state: &mut BackendState) {
    let now = Instant::now();
    state.permits.retain(|_, p| p.expires_at > now);
    state.grants.retain(|_, expiry| *expiry > now);
    let expired: Vec<String> = state
        .streams
        .iter()
        .filter(|(_, run)| now.duration_since(run.started) > STREAM_TTL)
        .map(|(id, _)| id.clone())
        .collect();
    for id in expired {
        if let Some(run) = state.streams.remove(&id) {
            run.cancel.cancel();
        }
    }
}

fn runtime_for(state: &mut BackendState) -> Result<Arc<ProviderRuntime>, CommandError> {
    let config = state.config.clone().ok_or_else(|| {
        CommandError::new(
            ErrorCode::InvalidRequest,
            "Providers are not configured for this workspace.",
        )
    })?;
    let fp = fingerprint(&serde_json::to_string(&config).unwrap_or_default());
    let stale = state
        .runtime
        .as_ref()
        .map(|(cached, _)| *cached != fp)
        .unwrap_or(true);
    if stale {
        let runtime = ProviderRuntime::production(config)
            .map_err(to_command_error)?
            .with_shared_accounting(state.used_permits.clone(), state.ledger.clone());
        state.runtime = Some((fp, Arc::new(runtime)));
    }
    Ok(state.runtime.as_ref().expect("runtime populated").1.clone())
}

/// Owner authorization for the policy-setting operations
/// (configure / authorize). Mirrors the vault's Desktop/Browser
/// transport split: the host process is the owner; an API session
/// must hold a live host-issued grant. An endpoint move alone —
/// caller JSON arriving at `providers-configure` instead of
/// `providers-dispatch` — establishes nothing, so the check is on
/// WHO calls, not WHERE the policy sits.
const PENDING_ACCESS_CAP: usize = 16;
const PENDING_ACCESS_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// Record (or refresh) a refused browser session in the host's
/// approval queue. Host attempts are never queued (the host needs
/// no approval), and the queue is bounded with stale entries
/// swept — it is a signal for the owner, not an audit log.
fn record_pending_access(state: &mut BackendState, session: &str) {
    if session == HOST_SESSION {
        return;
    }
    let now = now_ms();
    state
        .pending_access
        .retain(|p| now.saturating_sub(p.last_seen_ms) <= PENDING_ACCESS_TTL_MS);
    if let Some(entry) = state
        .pending_access
        .iter_mut()
        .find(|p| p.session == session)
    {
        entry.last_seen_ms = now;
        entry.attempts = entry.attempts.saturating_add(1);
        return;
    }
    if state.pending_access.len() >= PENDING_ACCESS_CAP {
        state.pending_access.remove(0);
    }
    state.pending_access.push(PendingAccess {
        session: session.to_string(),
        first_seen_ms: now,
        last_seen_ms: now,
        attempts: 1,
    });
}

/// `require_owner`, additionally recording refused browser
/// attempts in the approval queue.
fn require_owner_recording(
    state: &mut BackendState,
    scope: &CallerScope,
) -> Result<(), CommandError> {
    let result = require_owner(state, scope);
    if result.is_err() {
        record_pending_access(state, &scope.session);
    }
    result
}

fn require_owner(state: &BackendState, scope: &CallerScope) -> Result<(), CommandError> {
    if scope.session == HOST_SESSION {
        return Ok(());
    }
    match state.grants.get(&scope.session) {
        Some(expiry) if *expiry > Instant::now() => Ok(()),
        _ => Err(CommandError::new(
            ErrorCode::Forbidden,
            "This operation requires owner authorization (host or host-granted session).",
        )),
    }
}

fn take_permit(
    state: &mut BackendState,
    scope: &CallerScope,
    permit_id: &Option<String>,
) -> Result<Option<PaidInferencePermit>, CommandError> {
    let Some(id) = permit_id else {
        return Ok(None);
    };
    let record = state.permits.get(id).ok_or_else(|| {
        CommandError::new(
            ErrorCode::Forbidden,
            "The permit is not available to this caller.",
        )
    })?;
    if record.session != scope.session || record.expires_at <= Instant::now() {
        return Err(CommandError::new(
            ErrorCode::Forbidden,
            "The permit is not available to this caller.",
        ));
    }
    Ok(Some(record.permit.clone()))
}

// ---------------------------------------------------------------------------
// Bounds validation (before any state change or traffic)

fn invalid(message: &str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn validate_config_bounds(config: &RuntimeConfig) -> Result<(), CommandError> {
    if config.providers.len() > MAX_PROVIDERS {
        return Err(invalid("Too many providers in configuration."));
    }
    for provider in &config.providers {
        if provider.config.base_url.chars().count() > MAX_BASE_URL_CHARS {
            return Err(invalid("Provider base URL is too long."));
        }
        if provider.models.len() > MAX_MODELS_PER_PROVIDER {
            return Err(invalid("Too many models for one provider."));
        }
    }
    Ok(())
}

fn validate_request_bounds(request: &CompletionRequest) -> Result<(), CommandError> {
    if request.messages.len() > MAX_MESSAGES {
        return Err(invalid("Too many messages in the request."));
    }
    let mut total_chars = 0usize;
    for message in &request.messages {
        if message.segments.len() > MAX_SEGMENTS_PER_MESSAGE {
            return Err(invalid("Too many segments in one message."));
        }
        for segment in &message.segments {
            let chars = segment.text.chars().count();
            if chars > MAX_SEGMENT_CHARS {
                return Err(invalid("A message segment is too long."));
            }
            total_chars += chars;
        }
    }
    if total_chars > MAX_TOTAL_MESSAGE_CHARS {
        return Err(invalid("The request is too large."));
    }
    if request
        .max_output_tokens
        .map(|max| max > MAX_OUTPUT_TOKENS)
        .unwrap_or(false)
    {
        return Err(invalid("maxOutputTokens exceeds the bound."));
    }
    Ok(())
}

/// Derive locality from the validated endpoint and refuse a
/// configuration whose declared locality contradicts it. This is
/// the check that makes "cloud endpoint declared Local" impossible
/// rather than merely discouraged.
fn enforce_derived_locality(config: &RuntimeConfig) -> Result<(), CommandError> {
    for provider in &config.providers {
        let endpoint = provider.config.validate().map_err(to_command_error)?;
        let derived = if endpoint.loopback {
            LocalityConfig::Local
        } else {
            LocalityConfig::Cloud
        };
        if provider.locality != derived {
            return Err(invalid(&format!(
                "Provider {} declares locality {:?} but its endpoint is {}; locality is derived, not declared.",
                provider.config.provider,
                provider.locality,
                if endpoint.loopback {
                    "loopback (Local)"
                } else {
                    "non-loopback (Cloud)"
                },
            )));
        }
    }
    Ok(())
}

/// The trusted configuration defines the model universe: a request
/// naming a model no configured provider offers is refused before
/// any gate or network work, in every routing mode. (In Auto modes
/// the router picks the model from the configuration; the request's
/// model string must never reach the wire unless the configuration
/// sanctions it.)
fn enforce_configured_model(
    state: &BackendState,
    request: &CompletionRequest,
) -> Result<(), CommandError> {
    let config = state.config.as_ref().ok_or_else(|| {
        CommandError::new(
            ErrorCode::InvalidRequest,
            "Providers are not configured for this workspace.",
        )
    })?;
    let configured = config
        .providers
        .iter()
        .flat_map(|p| p.models.iter())
        .any(|m| m.id == request.model);
    if !configured {
        return Err(invalid(
            "The requested model is not configured for this workspace.",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared helpers

fn parse_mode(mode: &Option<String>) -> Result<RoutingMode, CommandError> {
    match mode.as_deref() {
        None | Some("auto") => Ok(RoutingMode::Auto),
        Some("autoFree") => Ok(RoutingMode::AutoFree),
        Some("private") => Ok(RoutingMode::Private),
        Some("fast") => Ok(RoutingMode::Fast),
        Some("deep") => Ok(RoutingMode::Deep),
        Some("coding") => Ok(RoutingMode::Coding),
        Some("vision") => Ok(RoutingMode::Vision),
        Some("chooseModel") => Ok(RoutingMode::ChooseModel),
        Some(other) => Err(invalid(&format!("Unknown routing mode: {other}"))),
    }
}

fn routing_request(req: &ProviderDispatchRequest) -> Result<RoutingRequest, CommandError> {
    Ok(RoutingRequest {
        mode: parse_mode(&req.mode)?,
        lock: if req.zero_cost_lock {
            ZeroCostLock::on()
        } else {
            ZeroCostLock::off()
        },
        chosen: req.chosen.clone(),
        ..Default::default()
    })
}

fn to_command_error(e: ProviderError) -> CommandError {
    let code = match e.code {
        ProviderErrorCode::InvalidConfiguration
        | ProviderErrorCode::EndpointRejected
        | ProviderErrorCode::Unsupported => ErrorCode::InvalidRequest,
        ProviderErrorCode::AuthFailed => ErrorCode::Unauthorized,
        ProviderErrorCode::PermissionRefused | ProviderErrorCode::CostRefused => {
            ErrorCode::Forbidden
        }
        _ => ErrorCode::StorageUnavailable,
    };
    CommandError::new(code, &format!("{}: {}", e.code_name(), e.message))
}

fn event_dto(event: &Result<StreamEvent, ProviderError>) -> ProviderStreamEvent {
    match event {
        Ok(StreamEvent::Token(text)) => ProviderStreamEvent {
            kind: "token".into(),
            text: Some(text.clone()),
            code: None,
            message: None,
            input_tokens: None,
            output_tokens: None,
            finish_reason: None,
        },
        Ok(StreamEvent::Usage(u)) => ProviderStreamEvent {
            kind: "usage".into(),
            text: None,
            code: None,
            message: None,
            input_tokens: Some(u.input_tokens),
            output_tokens: Some(u.output_tokens),
            finish_reason: None,
        },
        Ok(StreamEvent::Completed { finish_reason }) => ProviderStreamEvent {
            kind: "completed".into(),
            text: None,
            code: None,
            message: None,
            input_tokens: None,
            output_tokens: None,
            finish_reason: finish_reason.clone(),
        },
        Err(e) => ProviderStreamEvent {
            kind: "error".into(),
            text: None,
            code: Some(e.code_name().to_string()),
            message: Some(e.message.clone()),
            input_tokens: None,
            output_tokens: None,
            finish_reason: None,
        },
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn default_contract_version() -> u8 {
    1
}

fn require_contract_v1(version: u8) -> Result<(), CommandError> {
    if version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use a supported contract version.",
        ));
    }
    Ok(())
}

fn parse<T: serde::de::DeserializeOwned>(value: Value, what: &str) -> Result<T, CommandError> {
    serde_json::from_value(value).map_err(|_| {
        CommandError::new(
            ErrorCode::InvalidRequest,
            &format!("Use a supported {what} request."),
        )
    })
}

fn to_value<T: Serialize>(value: &T) -> Result<Value, CommandError> {
    serde_json::to_value(value).map_err(|_| crate::storage::unavailable())
}

// ---------------------------------------------------------------------------
// Configuration persistence
//
// The trusted configuration (provider endpoints, models, budget —
// credential REFERENCES only, never secrets) persists per logical
// workspace as one JSON file under the storage root, so a restart
// does not silently drop the owner's provider setup. Permits,
// grants, stream runs and usage totals deliberately do NOT
// persist: they are session-bound security/accounting state and a
// restart correctly resets them. Loading re-runs the full
// configure-time validation (bounds + derived locality + runtime
// build), so a tampered file cannot smuggle in a masquerade the
// configure endpoint would have rejected.

fn config_file_path(root: &std::path::Path, workspace_id: &str) -> std::path::PathBuf {
    // Workspace ids are generated by the storage layer; sanitize
    // anyway before using one as a file name.
    let safe: String = workspace_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect();
    root.join("provider-config").join(format!("{safe}.json"))
}

/// The process-wide keyring resolver, shared by every runtime
/// this process builds and by the credential operations: one
/// entry cache, and (under the keyring mock builder in tests)
/// one visible store.
fn shared_resolver() -> Arc<anansi_providers::keyring_resolver::KeyringSecretResolver> {
    static RESOLVER: std::sync::OnceLock<
        Arc<anansi_providers::keyring_resolver::KeyringSecretResolver>,
    > = std::sync::OnceLock::new();
    RESOLVER
        .get_or_init(|| Arc::new(anansi_providers::keyring_resolver::KeyringSecretResolver::new()))
        .clone()
}

fn production_runtime(
    config: RuntimeConfig,
) -> Result<ProviderRuntime, anansi_providers::error::ProviderError> {
    ProviderRuntime::new(
        config,
        anansi_providers::http::RustlsTransport::new(),
        Box::new(shared_resolver()),
    )
}

/// Whether this workspace currently has a configuration installed
/// in the in-memory backend.
pub fn has_config(scope: &CallerScope) -> bool {
    with_backend(scope, |state| Ok(state.config.is_some())).unwrap_or(false)
}

/// The workspace's configured chat-capable models, in
/// configuration order, as (provider, model) pairs. Used by
/// higher-level workflows (idea generation) to resolve a
/// default model from the same trusted configuration the
/// dispatcher enforces — never from caller-asserted policy.
pub fn configured_chat_models(scope: &CallerScope) -> Vec<(String, String)> {
    with_backend(scope, |state| {
        let mut out = Vec::new();
        if let Some(config) = &state.config {
            for provider in &config.providers {
                if !provider.config.enabled {
                    continue;
                }
                for model in &provider.models {
                    if model.capabilities.chat {
                        out.push((model.provider.clone(), model.id.clone()));
                    }
                }
            }
        }
        Ok(out)
    })
    .unwrap_or_default()
}

/// The cost class of one configured chat model, from the trusted
/// configuration — never from a request. Used by the SOMNIA
/// compaction free-only gate: a comparison runs free models only,
/// because one permit authorizes one dispatch chain.
pub fn configured_model_cost(
    scope: &CallerScope,
    provider: &str,
    model: &str,
) -> Option<CostClass> {
    with_backend(scope, |state| {
        let found = state.config.as_ref().and_then(|config| {
            config
                .providers
                .iter()
                .filter(|p| p.config.enabled && p.config.provider == provider)
                .flat_map(|p| p.models.iter())
                .find(|m| m.id == model && m.capabilities.chat)
                .map(|m| m.cost)
        });
        Ok(found)
    })
    .unwrap_or_default()
}

/// The installed configuration as JSON (for persistence after a
/// successful configure).
pub fn configured_config_json(scope: &CallerScope) -> Option<String> {
    with_backend(scope, |state| {
        Ok(state
            .config
            .as_ref()
            .and_then(|c| serde_json::to_string(c).ok()))
    })
    .ok()
    .flatten()
}

/// Install a persisted configuration if none is installed yet.
/// Returns Ok(false) when a configuration is already present (the
/// in-memory one wins). Validation failures are errors: a bad
/// persisted file must be visible, not silently ignored.
pub fn install_persisted_config(scope: &CallerScope, json: &str) -> Result<bool, CommandError> {
    let config: RuntimeConfig = serde_json::from_str(json).map_err(|_| {
        invalid("The persisted provider configuration is unreadable; reconfigure providers.")
    })?;
    validate_config_bounds(&config)?;
    enforce_derived_locality(&config)?;
    with_backend(scope, |state| {
        if state.config.is_some() {
            return Ok(false);
        }
        let runtime = production_runtime(config.clone())
            .map_err(to_command_error)?
            .with_shared_accounting(state.used_permits.clone(), state.ledger.clone());
        let fp = fingerprint(&serde_json::to_string(&config).unwrap_or_default());
        state.config = Some(config);
        state.runtime = Some((fp, Arc::new(runtime)));
        Ok(true)
    })
}

/// Read a persisted configuration file, if one exists.
pub fn load_config_file(
    root: &std::path::Path,
    workspace_id: &str,
) -> Result<Option<String>, CommandError> {
    let dir = root.join("provider-config");
    if dir.symlink_metadata().is_ok() {
        crate::storage::reject_links(&dir).map_err(|_| crate::storage::unavailable())?;
    }
    let path = config_file_path(root, workspace_id);
    if !path.exists() {
        return Ok(None);
    }
    crate::storage::reject_links(&path).map_err(|_| crate::storage::unavailable())?;
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(crate::storage::unavailable()),
    }
}

/// Persist a configuration atomically with hardened file
/// handling: the directory and any existing destination are
/// link-checked, the temporary file has an unpredictable name and
/// is created with create-new semantics (never following or
/// truncating a pre-planted file), the payload is fsynced before
/// an atomic rename, and the directory is synced afterwards.
/// Callers must treat an error as "not persisted" and keep
/// memory/disk consistent (configure persists BEFORE installing
/// into backend state, under the backend lock).
pub fn persist_config_file(
    root: &std::path::Path,
    workspace_id: &str,
    json: &str,
) -> Result<(), CommandError> {
    let unavailable = || crate::storage::unavailable();
    let dir = root.join("provider-config");
    std::fs::create_dir_all(&dir).map_err(|_| unavailable())?;
    crate::storage::reject_links(&dir).map_err(|_| unavailable())?;
    crate::storage::private_permissions(&dir, true, false).map_err(|_| unavailable())?;
    let path = config_file_path(root, workspace_id);
    if path.symlink_metadata().is_ok() {
        // An existing destination must be a regular file we own,
        // never a link/reparse point a rename would silently
        // replace the target of.
        crate::storage::reject_links(&path).map_err(|_| unavailable())?;
    }
    // Unique create-new temporary file (a planted file at a
    // predictable name must never be followed).
    let mut tmp = None;
    for _ in 0..8 {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).map_err(|_| unavailable())?;
        let candidate = dir.join(format!(".tmp-{:016x}", u64::from_le_bytes(bytes)));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                tmp = Some((candidate, file));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(unavailable()),
        }
    }
    let Some((tmp_path, mut file)) = tmp else {
        return Err(unavailable());
    };
    let outcome = (|| -> Result<(), CommandError> {
        use std::io::Write;
        file.write_all(json.as_bytes()).map_err(|_| unavailable())?;
        file.sync_all().map_err(|_| unavailable())?;
        drop(file);
        crate::storage::private_permissions(&tmp_path, false, false).map_err(|_| unavailable())?;
        crate::storage::reject_links(&tmp_path).map_err(|_| unavailable())?;
        std::fs::rename(&tmp_path, &path).map_err(|_| unavailable())?;
        // Best-effort directory sync for durability of the rename
        // itself (unsupported on some platforms — the rename is
        // already atomic there).
        if let Ok(d) = std::fs::File::open(&dir) {
            let _ = d.sync_all();
        }
        Ok(())
    })();
    if outcome.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    outcome
}

// ---------------------------------------------------------------------------
// Operations

pub fn providers_command(
    operation: &str,
    value: Value,
    scope: &CallerScope,
) -> Result<Value, CommandError> {
    if operation.starts_with("ideas-") {
        return crate::ideas::ideas_command(operation, value, scope);
    }
    if operation.starts_with("collider-") {
        return crate::collider::collider_command(operation, value, scope);
    }
    if operation.starts_with("swarmtank-") {
        return crate::swarmtank::swarmtank_command(operation, value, scope);
    }
    if operation.starts_with("somnia-") {
        return crate::somnia::somnia_command(operation, value, scope);
    }
    if operation.starts_with("opportunities-") {
        return crate::opportunities::opportunities_command(operation, value, scope);
    }
    if operation.starts_with("forge-") {
        return crate::forge::forge_command(operation, value, scope);
    }
    match operation {
        "providers-catalog" => to_value(&ProviderCatalogResponse {
            contract_version: 1,
            providers: anansi_providers::catalog::catalog()
                .iter()
                .map(|e| ProviderCatalogEntry {
                    id: e.id.to_string(),
                    display_name: e.display_name.to_string(),
                    base_url: e.base_url.map(str::to_string),
                    docs_url: Some(e.docs_url.to_string()),
                    locality: format!("{:?}", e.locality),
                    status: format!("{:?}", e.status),
                })
                .collect(),
        }),
        "providers-configure" => {
            let req: ProviderConfigureRequest = parse(value, "provider configuration")?;
            require_contract_v1(req.contract_version)?;
            validate_config_bounds(&req.config)?;
            enforce_derived_locality(&req.config)?;
            let providers: Vec<String> = req
                .config
                .providers
                .iter()
                .map(|p| p.config.provider.clone())
                .collect();
            with_backend(scope, |state| {
                require_owner_recording(state, scope)?;
                // Validate by building the runtime now, so a bad
                // configuration fails at configure time — with the
                // workspace's shared accounting wired in.
                let runtime = production_runtime(req.config.clone())
                    .map_err(to_command_error)?
                    .with_shared_accounting(state.used_permits.clone(), state.ledger.clone());
                let serialized = serde_json::to_string(&req.config).unwrap_or_default();
                // Persist BEFORE installing, under this same
                // backend lock: if persistence fails, the
                // operation fails and memory stays exactly as it
                // was — memory and disk can never diverge, and
                // concurrent configures serialize whole.
                if let (Some(root), Some(wid)) = (&scope.storage_root, &scope.logical_workspace) {
                    persist_config_file(root, wid, &serialized)?;
                }
                let fp = fingerprint(&serialized);
                state.config = Some(req.config);
                state.runtime = Some((fp, Arc::new(runtime)));
                Ok(())
            })?;
            to_value(&ProviderConfigureResponse {
                contract_version: 1,
                providers,
            })
        }
        "providers-authorize" => {
            let req: ProviderAuthorizeRequest = parse(value, "provider authorization")?;
            require_contract_v1(req.contract_version)?;
            if req.max_estimated_total_tokens == 0
                || req.max_estimated_total_tokens > MAX_PERMIT_TOKENS
            {
                return Err(invalid("Permit token cap is out of bounds."));
            }
            let ttl = req.ttl_seconds.unwrap_or(DEFAULT_PERMIT_TTL_SECONDS);
            if !(MIN_PERMIT_TTL_SECONDS..=MAX_PERMIT_TTL_SECONDS).contains(&ttl) {
                return Err(invalid("Permit TTL is out of bounds."));
            }
            with_backend(scope, |state| require_owner_recording(state, scope))?;
            let permit_id = random_capability("permit-");
            let permit = PaidInferencePermit {
                id: permit_id.clone(),
                provider: req.provider,
                model: req.model,
                max_estimated_total_tokens: req.max_estimated_total_tokens,
                allow_unknown_cost: req.allow_unknown_cost,
            };
            let expires_at_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64 + ttl * 1000)
                .unwrap_or(0);
            with_backend(scope, |state| {
                state.permits.insert(
                    permit_id.clone(),
                    PermitRecord {
                        permit,
                        session: scope.session.clone(),
                        expires_at: Instant::now() + Duration::from_secs(ttl),
                    },
                );
                Ok(())
            })?;
            to_value(&ProviderAuthorizeResponse {
                contract_version: 1,
                permit_id,
                expires_at_ms,
            })
        }
        "providers-dispatch" => {
            let req: ProviderDispatchRequest = parse(value, "provider dispatch")?;
            require_contract_v1(req.contract_version)?;
            validate_request_bounds(&req.request)?;
            let routing = routing_request(&req)?;
            // Phase 1 (locked): resolve policy — model universe,
            // permit, runtime. Phase 2 (UNLOCKED): the network
            // dispatch itself. The backend mutex is never held
            // across a network call, so poll/cancel and other
            // workspaces progress during a slow request; the
            // authorization decisions were taken atomically in
            // phase 1 and the accounting lives behind its own
            // locks inside the runtime.
            let (permit, runtime) = with_backend(scope, |state| {
                enforce_configured_model(state, &req.request)?;
                let permit = take_permit(state, scope, &req.permit_id)?;
                let runtime = runtime_for(state)?;
                Ok((permit, runtime))
            })?;
            let outcome = runtime
                .dispatch(&req.request, routing, permit, &CancellationToken::new())
                .map_err(to_command_error)?;
            to_value(&ProviderDispatchResponse {
                contract_version: 1,
                response: ProviderResponseBody {
                    text: outcome.response.text,
                    model: outcome.response.model,
                    finish_reason: outcome.response.finish_reason,
                    usage: outcome.response.usage,
                },
                disclosure: outcome.disclosure,
            })
        }
        "providers-stream-start" => {
            let req: ProviderDispatchRequest = parse(value, "provider dispatch")?;
            require_contract_v1(req.contract_version)?;
            validate_request_bounds(&req.request)?;
            let routing = routing_request(&req)?;
            // Locked phase: policy resolution (model, cap
            // pre-check, permit, runtime). Unlocked: stream
            // establishment (a network call). Locked again: run
            // registration — if the retained cap filled meanwhile,
            // the just-established stream is cancelled and the
            // start refused rather than exceeding the bound.
            let (permit, runtime) = with_backend(scope, |state| {
                enforce_configured_model(state, &req.request)?;
                if state.streams.len() >= MAX_RETAINED_STREAMS {
                    return Err(invalid(
                        "Too many retained provider streams for this workspace.",
                    ));
                }
                let permit = take_permit(state, scope, &req.permit_id)?;
                let runtime = runtime_for(state)?;
                Ok((permit, runtime))
            })?;
            let cancel = CancellationToken::new();
            let outcome = runtime
                .dispatch_stream(&req.request, routing, permit, &cancel)
                .map_err(to_command_error)?;
            let disclosure = outcome.disclosure.clone();
            let capability = random_capability("cap-");
            let started = with_backend(scope, |state| {
                if state.streams.len() >= MAX_RETAINED_STREAMS {
                    cancel.cancel();
                    return Err(invalid(
                        "Too many retained provider streams for this workspace.",
                    ));
                }
                state.stream_seq += 1;
                let run_id = format!("provider-run-{}", state.stream_seq);
                state.streams.insert(
                    run_id.clone(),
                    StreamRun {
                        outcome,
                        cancel: cancel.clone(),
                        capability: capability.clone(),
                        session: scope.session.clone(),
                        started: Instant::now(),
                        done: false,
                    },
                );
                Ok(ProviderStreamStartResponse {
                    contract_version: 1,
                    run_id,
                    capability,
                    disclosure,
                })
            })?;
            to_value(&started)
        }
        "providers-stream-poll" | "providers-stream-cancel" => {
            let req: ProviderRunRequest = parse(value, "provider stream run")?;
            require_contract_v1(req.contract_version)?;
            with_backend_existing(scope, |state| {
                // Binding check: right run, right session, right
                // capability — anything else is indistinguishable
                // from an unknown run.
                let bound = state.streams.get(&req.run_id).map(|run| {
                    run.session == scope.session
                        && constant_time_eq(&run.capability, &req.capability)
                });
                if bound != Some(true) {
                    return Err(CommandError::new(
                        ErrorCode::NotFound,
                        "Unknown provider stream run.",
                    ));
                }
                if operation == "providers-stream-cancel" {
                    let run = state.streams.get(&req.run_id).expect("checked above");
                    run.cancel.cancel();
                    return to_value(&serde_json::json!({ "cancelled": true }));
                }
                let run = state.streams.get_mut(&req.run_id).expect("checked above");
                let mut events = Vec::new();
                if !run.done {
                    loop {
                        match run.outcome.events.try_recv() {
                            Ok(event) => {
                                let terminal =
                                    matches!(event, Ok(StreamEvent::Completed { .. }) | Err(_));
                                // D1: the run disclosure is built at stream
                                // start with usage None, and the settle
                                // thread records reported usage only in
                                // the ledger. Carry each reported Usage
                                // event into the disclosure, so a
                                // completed run's disclosure agrees with
                                // the ledger instead of staying null.
                                if let Ok(StreamEvent::Usage(u)) = &event {
                                    run.outcome.disclosure.usage = Some(*u);
                                }
                                events.push(event_dto(&event));
                                if terminal {
                                    run.done = true;
                                    break;
                                }
                                if events.len() >= 256 {
                                    break;
                                }
                            }
                            Err(std::sync::mpsc::TryRecvError::Empty) => break,
                            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                run.done = true;
                                break;
                            }
                        }
                    }
                }
                let done = run.done;
                let disclosure = run.outcome.disclosure.clone();
                if done {
                    state.streams.remove(&req.run_id);
                }
                to_value(&ProviderPollResponse {
                    contract_version: 1,
                    events,
                    done,
                    disclosure,
                })
            })
        }
        "providers-grant-session" | "providers-revoke-session" => {
            // Host-only: an API session can never grant itself
            // owner authorization.
            if scope.session != HOST_SESSION {
                return Err(CommandError::new(
                    ErrorCode::Forbidden,
                    "Only the host can manage owner-authorization grants.",
                ));
            }
            let req: ProviderGrantRequest = parse(value, "provider grant")?;
            require_contract_v1(req.contract_version)?;
            let granting = operation == "providers-grant-session";
            with_backend(scope, |state| {
                if granting {
                    let ttl = req.ttl_seconds.unwrap_or(DEFAULT_PERMIT_TTL_SECONDS);
                    if !(MIN_PERMIT_TTL_SECONDS..=MAX_PERMIT_TTL_SECONDS).contains(&ttl) {
                        return Err(invalid("Grant TTL is out of bounds."));
                    }
                    state.grants.insert(
                        req.session.clone(),
                        Instant::now() + Duration::from_secs(ttl),
                    );
                    state.pending_access.retain(|p| p.session != req.session);
                } else {
                    state.grants.remove(&req.session);
                }
                Ok(())
            })?;
            to_value(&serde_json::json!({ "granted": granting }))
        }
        "providers-credential-set" => {
            let req: ProviderCredentialRequest = parse(value, "provider credential")?;
            require_contract_v1(req.contract_version)?;
            crate::provider_credentials::validate_reference(&req.reference)?;
            if req.provider.is_empty() || req.provider.len() > 64 {
                return Err(invalid("Name the provider for this credential."));
            }
            // Owner authorization first (recorded for the host's
            // approval queue); the prompt runs AFTER the backend
            // lock is released — a human may take minutes, and no
            // workspace state is touched by this operation at all.
            with_backend(scope, |state| require_owner_recording(state, scope))?;
            let key =
                crate::provider_credentials::prompt_for_api_key(&req.provider, &req.reference)?;
            let resolver = shared_resolver();
            resolver
                .store_api_key(&req.reference, &key)
                .map_err(to_command_error)?;
            to_value(&ProviderCredentialSetResponse {
                contract_version: 1,
                stored: true,
            })
        }
        "providers-credential-remove" => {
            let req: ProviderCredentialRequest = parse(value, "provider credential")?;
            require_contract_v1(req.contract_version)?;
            crate::provider_credentials::validate_reference(&req.reference)?;
            with_backend(scope, |state| require_owner_recording(state, scope))?;
            let resolver = shared_resolver();
            resolver.delete(&req.reference).map_err(to_command_error)?;
            to_value(&ProviderCredentialRemoveResponse {
                contract_version: 1,
                removed: true,
            })
        }
        "providers-credential-status" => {
            let req: ProviderCredentialRequest = parse(value, "provider credential")?;
            require_contract_v1(req.contract_version)?;
            crate::provider_credentials::validate_reference(&req.reference)?;
            with_backend(scope, |state| require_owner(state, scope))?;
            let resolver = shared_resolver();
            to_value(&ProviderCredentialStatusResponse {
                contract_version: 1,
                present: resolver.exists(&req.reference),
            })
        }
        "providers-session-access" => {
            let req: ProviderWorkspaceRequest = parse(value, "provider session access")?;
            require_contract_v1(req.contract_version)?;
            with_backend(scope, |state| {
                require_owner(state, scope)?;
                let now = now_ms();
                let now_instant = Instant::now();
                Ok(ProviderSessionAccessResponse {
                    contract_version: 1,
                    pending: state
                        .pending_access
                        .iter()
                        .map(|p| ProviderPendingAccess {
                            session: p.session.clone(),
                            first_seen_ms: p.first_seen_ms,
                            last_seen_ms: p.last_seen_ms,
                            attempts: p.attempts,
                        })
                        .collect(),
                    grants: state
                        .grants
                        .iter()
                        .filter(|(_, expiry)| **expiry > now_instant)
                        .map(|(session, expiry)| ProviderGrantInfo {
                            session: session.clone(),
                            expires_at_ms: now
                                + expiry.duration_since(now_instant).as_millis() as u64,
                        })
                        .collect(),
                })
            })
            .and_then(|resp| to_value(&resp))
        }
        "providers-usage" => {
            let req: ProviderWorkspaceRequest = parse(value, "provider usage")?;
            require_contract_v1(req.contract_version)?;
            with_backend(scope, |state| {
                let ledger = state
                    .ledger
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let (reserved_requests, reserved_tokens) = ledger.reserved();
                to_value(&ProviderUsageResponse {
                    contract_version: 1,
                    total_requests: ledger.total_requests(),
                    reserved_requests,
                    reserved_tokens,
                    providers: ledger
                        .totals_snapshot()
                        .into_iter()
                        .map(|(provider, totals)| ProviderUsageEntry {
                            provider,
                            requests: totals.requests,
                            input_tokens: totals.input_tokens,
                            output_tokens: totals.output_tokens,
                        })
                        .collect(),
                })
            })
        }
        _ => Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "Unknown provider operation.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disclosure() -> DisclosureRecord {
        serde_json::from_value(serde_json::json!({
            "provider": "ollama", "model": "llama-x", "endpointHost": "127.0.0.1",
            "local": true, "routingMode": "auto", "routingReason": "test",
            "messageCount": 1, "segmentCount": 1, "redactionCount": 0,
            "estimatedInputTokens": 1, "costClass": "free", "usage": null,
            "attempts": []
        }))
        .unwrap()
    }

    fn run(started: Instant, cancel: &CancellationToken) -> StreamRun {
        let (_tx, rx) = std::sync::mpsc::channel();
        StreamRun {
            outcome: StreamOutcome {
                events: rx,
                disclosure: disclosure(),
            },
            cancel: cancel.clone(),
            capability: "cap-test".into(),
            session: "s1".into(),
            started,
            done: false,
        }
    }

    #[test]
    fn sweep_cancels_and_drops_expired_streams() {
        let mut state = BackendState::default();
        let cancel = CancellationToken::new();
        state.streams.insert(
            "old".into(),
            run(
                Instant::now() - STREAM_TTL - Duration::from_secs(1),
                &cancel,
            ),
        );
        let fresh_cancel = CancellationToken::new();
        state
            .streams
            .insert("fresh".into(), run(Instant::now(), &fresh_cancel));
        sweep(&mut state);
        assert!(!state.streams.contains_key("old"), "expired run dropped");
        assert!(cancel.is_cancelled(), "expired run is cancelled");
        assert!(state.streams.contains_key("fresh"));
        assert!(!fresh_cancel.is_cancelled());
    }

    #[test]
    fn sweep_drops_expired_permits() {
        let mut state = BackendState::default();
        state.permits.insert(
            "permit-old".into(),
            PermitRecord {
                permit: PaidInferencePermit {
                    id: "permit-old".into(),
                    provider: "ollama".into(),
                    model: "llama-x".into(),
                    max_estimated_total_tokens: 10,
                    allow_unknown_cost: false,
                },
                session: "s1".into(),
                expires_at: Instant::now() - Duration::from_secs(1),
            },
        );
        sweep(&mut state);
        assert!(state.permits.is_empty());
    }

    #[test]
    fn workspace_validation_is_cached_for_positive_results_only() {
        use std::cell::Cell;
        let calls = Cell::new(0);
        let list = || {
            calls.set(calls.get() + 1);
            Ok(vec!["ws-a".to_string()])
        };
        // First resolution consults the registry...
        let key = validated_workspace_key("root-x", "ws-a", list).unwrap();
        assert_eq!(key, "root-x/ws-a");
        assert_eq!(calls.get(), 1);
        // ...the second is served from the cache.
        let list2 = || {
            calls.set(calls.get() + 1);
            Ok(vec!["ws-a".to_string()])
        };
        validated_workspace_key("root-x", "ws-a", list2).unwrap();
        assert_eq!(
            calls.get(),
            1,
            "cached validation must not re-read the registry"
        );
        // Unknown ids are refused and NOT cached (each attempt
        // re-consults the registry, so a workspace created later
        // becomes visible immediately).
        let list3 = || {
            calls.set(calls.get() + 1);
            Ok(vec!["ws-a".to_string()])
        };
        let err = validated_workspace_key("root-x", "ws-nope", list3).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn capabilities_are_unpredictable_and_compared_exactly() {
        let a = random_capability("cap-");
        let b = random_capability("cap-");
        assert_ne!(a, b);
        assert_eq!(a.len(), "cap-".len() + 32);
        assert!(constant_time_eq(&a, &a));
        assert!(!constant_time_eq(&a, &b));
        assert!(!constant_time_eq(&a, "cap-short"));
    }
}
