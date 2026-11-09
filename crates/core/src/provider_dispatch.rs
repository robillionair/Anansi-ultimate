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
use anansi_providers::gates::{Budget, PaidInferencePermit, QuotaLedger, ZeroCostLock};
use anansi_providers::http::CancellationToken;
use anansi_providers::routing::{DisclosureRecord, RoutingMode, RoutingRequest};
use anansi_providers::runtime::{LocalityConfig, ProviderRuntime, RuntimeConfig, RuntimeProvider};
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

/// Atomic add/update of ONE provider entry (Connect AI slice):
/// the merged configuration keeps every other provider, the
/// existing budget, and the shared usage accounting. The request
/// carries no budget field by design — this operation can never
/// move the budget.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderConfigureAddRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub provider: RuntimeProvider,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigureAddResponse {
    pub contract_version: u8,
    pub providers: Vec<String>,
    pub added: String,
    pub updated: bool,
}

/// Host-generated credential reference (Connect AI slice): the
/// reference is a keyring account label, never a secret.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCredentialReferenceRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub provider: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredentialReferenceResponse {
    pub contract_version: u8,
    pub provider: String,
    pub reference: String,
}

/// One configured provider as a paired browser may see it
/// (Connect AI slice): references and cost classes only — no
/// secret, no usage detail beyond the verified flag.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatusModel {
    pub id: String,
    /// "free" | "paid" | "unknown" — the configured cost class.
    pub cost: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatusEntry {
    pub provider: String,
    pub display_name: String,
    pub base_url_host: String,
    pub locality: String,
    pub enabled: bool,
    pub models: Vec<ProviderStatusModel>,
    pub credential_reference: Option<String>,
    /// A credential exists in the OS store for the reference.
    pub credential_stored: bool,
    /// A dispatch has succeeded for THIS saved connection — the
    /// exact endpoint and credential reference recorded below —
    /// since the app started. Changing the endpoint, replacing
    /// the key (even under the same reference), or removing the
    /// provider clears it until the new connection succeeds.
    /// Session-scoped: a restart clears it, as the UI states.
    pub connection_verified: bool,
}

/// A stored credential no saved connection references: a key
/// minted during setup whose draft was discarded or abandoned.
/// Labels only — the reference and the provider it was minted
/// for — never secret material.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatusOrphanedCredential {
    pub provider: String,
    pub reference: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatusResponse {
    pub contract_version: u8,
    pub providers: Vec<ProviderStatusEntry>,
    pub orphaned_credentials: Vec<ProviderStatusOrphanedCredential>,
    pub budget: Budget,
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

/// The approved target a dispatch must still match, when the caller
/// bound the dispatch to a reviewed approval (SOMNIA compaction).
/// It is a binding the server verifies against backend state, never
/// configuration the caller asserts: the endpoint, locality, and cost
/// are re-derived from the live configuration in phase 1 and the
/// dispatch is refused before any traffic when they differ.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpectedDispatchTarget {
    pub provider: String,
    pub model: String,
    pub endpoint: String,
    pub locality: String,
    pub cost: CostClass,
}

/// Dispatch and stream-start share one request shape. It carries
/// **no** configuration, locality, cost or credential material:
/// policy is resolved from backend state only. The one exception is
/// `expected_target`: a caller-supplied approval binding that phase 1
/// verifies against backend state and never trusts as configuration.
/// The binding is per-op: on unary dispatch it requires explicit
/// model selection (`chooseModel` plus a `chosen` that matches the
/// expected provider/model), because every other routing mode
/// ignores `chosen` and lets the router pick a different model; on
/// stream-start the field is refused outright, because streaming
/// does not enforce the binding.
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
    /// The approved target this dispatch must still match. Absent
    /// for callers without an approval binding.
    #[serde(default)]
    pub expected_target: Option<ExpectedDispatchTarget>,
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
    /// Verification binding (Connect AI repair): the provider
    /// that established this stream, and the connection
    /// fingerprints snapshotted at stream-start. When polling
    /// observes the terminal Completed event, verification is
    /// recorded only if the saved connection still matches the
    /// snapshot — a stream completing after a change verifies
    /// nothing.
    verified_provider: String,
    verified_snapshot: std::collections::HashMap<String, String>,
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
    /// Connection verification (Connect AI repair): provider id
    /// -> fingerprint of the saved connection whose dispatch
    /// last succeeded. Bound to the connection, never to the
    /// provider id alone, and never derived from usage history.
    /// Session-scoped, in-memory only; a restart clears it.
    verified: HashMap<String, String>,
    /// Credential references minted by `providers-credential-
    /// reference` this session (reference -> provider), backed
    /// by the persisted ledger file. The keyring cannot be
    /// enumerated, so this is the only way an abandoned setup
    /// key can be found again for cleanup.
    minted_references: HashMap<String, String>,
    /// Whether `minted_references` has been loaded from disk
    /// (false until the first load, so an empty ledger is not
    /// re-read on every status call).
    minted_loaded: bool,
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

/// Verify an approval binding against the live configuration, in the
/// same locked section that acquires the runtime (mid-run design §2
/// D1–D3). Reads `state.config` directly — it must NOT call
/// `configured_model_target`, which takes the backend lock itself.
/// A mismatch refuses before the runtime is acquired and before any
/// traffic, so a configuration change between two dispatches can
/// never redirect the later one.
fn enforce_expected_target(
    state: &BackendState,
    expected: &ExpectedDispatchTarget,
    chosen: Option<&(String, String)>,
    mode: RoutingMode,
) -> Result<(), CommandError> {
    // The router honors `chosen` only in ChooseModel mode; in every
    // other mode it ignores the field and orders an automatic chain,
    // so an expected target without explicit selection binds nothing.
    if mode != RoutingMode::ChooseModel {
        return Err(invalid(
            "The expected dispatch target requires explicit model selection (chooseModel); automatic routing is not allowed.",
        ));
    }
    let Some((provider, model)) = chosen else {
        return Err(invalid(
            "The expected dispatch target requires a chosen provider and model; automatic routing is not allowed.",
        ));
    };
    if provider != &expected.provider || model != &expected.model {
        return Err(invalid(
            "The expected dispatch target does not match the chosen provider and model.",
        ));
    }
    let config = state.config.as_ref().ok_or_else(|| {
        CommandError::new(
            ErrorCode::InvalidRequest,
            "Providers are not configured for this workspace.",
        )
    })?;
    let Some(target) = target_in_config(config, &expected.provider, &expected.model) else {
        return Err(invalid(
            "The approved provider target is no longer configured for this workspace; review the candidate again before starting a new run.",
        ));
    };
    if target.endpoint != expected.endpoint
        || catalog_locality(&expected.provider) != expected.locality
        || target.cost != expected.cost
    {
        return Err(invalid(
            "The approved provider target changed; review the candidate again before starting a new run.",
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

/// The full trusted target record for one configured chat model:
/// its cost class and its effective endpoint — the configured
/// `baseUrl`, or the catalog default when the configuration leaves
/// it blank. Used by the SOMNIA compaction resolve op: the approval
/// binds the endpoint (and cost) of the resolved target, so a
/// configuration change after approval requires a fresh review.
pub struct ConfiguredModelTarget {
    pub cost: CostClass,
    pub endpoint: String,
}

/// The catalog's locality enum is private; the Debug form is
/// "Local" / "Cloud" (the same rendering providers-catalog uses).
/// One derivation, shared by the compaction target resolution and
/// the expected-target check, so the two can never disagree about
/// what a target's locality is.
pub(crate) fn catalog_locality(provider: &str) -> String {
    anansi_providers::catalog::catalog()
        .iter()
        .find(|e| e.id == provider)
        .map(|e| {
            if format!("{:?}", e.locality) == "Local" {
                "local".to_string()
            } else {
                "cloud".to_string()
            }
        })
        .unwrap_or_else(|| "cloud".to_string())
}

/// The cost class and effective endpoint of one configured chat
/// model, read from an already-held configuration. Shared by
/// `configured_model_target` (which takes the backend lock) and the
/// expected-target check (which runs inside the locked phase 1 and
/// must not re-lock).
fn target_in_config(
    config: &RuntimeConfig,
    provider: &str,
    model: &str,
) -> Option<ConfiguredModelTarget> {
    config
        .providers
        .iter()
        .filter(|p| p.config.enabled && p.config.provider == provider)
        .find_map(|p| {
            let m = p
                .models
                .iter()
                .find(|m| m.id == model && m.capabilities.chat)?;
            let endpoint = p.config.base_url.trim().to_string();
            let endpoint = if endpoint.is_empty() {
                anansi_providers::catalog::catalog()
                    .iter()
                    .find(|e| e.id == provider)
                    .and_then(|e| e.base_url.map(|u| u.to_string()))
                    .unwrap_or_default()
            } else {
                endpoint
            };
            Some(ConfiguredModelTarget {
                cost: m.cost,
                endpoint,
            })
        })
}

pub fn configured_model_target(
    scope: &CallerScope,
    provider: &str,
    model: &str,
) -> Option<ConfiguredModelTarget> {
    with_backend(scope, |state| {
        Ok(state
            .config
            .as_ref()
            .and_then(|config| target_in_config(config, provider, model)))
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
/// Install a parsed configuration into backend state held by the
/// caller (the lock is already held — this is the shared core of
/// `install_persisted_config` and the configure-add lazy load).
fn install_config_locked(
    state: &mut BackendState,
    config: RuntimeConfig,
) -> Result<bool, CommandError> {
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
}

/// Lazy-load the persisted configuration file into backend state
/// (restart case), inside the caller's `with_backend` closure.
/// Returns true when a file was loaded; false when state already
/// had a configuration or no file exists.
fn load_persisted_config_locked(
    state: &mut BackendState,
    scope: &CallerScope,
) -> Result<bool, CommandError> {
    if state.config.is_some() {
        return Ok(false);
    }
    let (Some(root), Some(wid)) = (&scope.storage_root, &scope.logical_workspace) else {
        return Ok(false);
    };
    let Some(json) = load_config_file(root, wid)? else {
        return Ok(false);
    };
    let config: RuntimeConfig = serde_json::from_str(&json).map_err(|_| {
        invalid("The persisted provider configuration is unreadable; reconfigure providers.")
    })?;
    validate_config_bounds(&config)?;
    enforce_derived_locality(&config)?;
    install_config_locked(state, config)
}

/// The identity of a saved connection for verification purposes
/// (Connect AI repair): the provider, the endpoint, the credential
/// reference, and the endpoint-security flags. The model list is
/// deliberately excluded — changing which models are offered does
/// not change the connection. A key rotated under the same
/// reference is invisible to this fingerprint, so the credential
/// set/remove operations invalidate explicitly (see below).
fn connection_fingerprint(p: &RuntimeProvider) -> String {
    let reference = p
        .config
        .credential
        .as_ref()
        .map(|c| c.reference.as_str())
        .unwrap_or("");
    fingerprint(&format!(
        "{}|{}|{}|{}|{}",
        p.config.provider,
        p.config.base_url,
        reference,
        p.config.custom_endpoint,
        p.config.allow_insecure_custom
    ))
}

fn snapshot_fingerprints(config: &RuntimeConfig) -> HashMap<String, String> {
    config
        .providers
        .iter()
        .map(|p| (p.config.provider.clone(), connection_fingerprint(p)))
        .collect()
}

/// Drop verification for every provider whose saved connection no
/// longer matches the fingerprint it verified under — changed
/// endpoint, changed credential reference, or a provider that is
/// gone. Unchanged providers keep their verification.
fn invalidate_changed_verifications(state: &mut BackendState, new: &RuntimeConfig) {
    let new_fps = snapshot_fingerprints(new);
    state
        .verified
        .retain(|provider, fp| new_fps.get(provider).map(|nf| nf == fp).unwrap_or(false));
}

/// Drop verification for every saved provider using this
/// credential reference — key rotation (set) or removal. The
/// caller loads the persisted config first when state has none.
fn invalidate_verification_for_reference(state: &mut BackendState, reference: &str) {
    let doomed: Vec<String> = state
        .config
        .iter()
        .flat_map(|c| c.providers.iter())
        .filter(|p| {
            p.config
                .credential
                .as_ref()
                .map(|cr| cr.reference == reference)
                .unwrap_or(false)
        })
        .map(|p| p.config.provider.clone())
        .collect();
    for id in doomed {
        state.verified.remove(&id);
    }
}

/// Record verification for the provider that actually served a
/// dispatch — but only when the saved connection still matches
/// the fingerprint snapshotted in phase 1. A delayed success
/// from the old connection finds a changed fingerprint and
/// records nothing. Usage and budgets are untouched: the runtime
/// settles the ledger independently.
fn record_verification_if_unchanged(
    state: &mut BackendState,
    provider: &str,
    snapshot: &HashMap<String, String>,
) {
    let Some(config) = &state.config else {
        return;
    };
    let Some(p) = config
        .providers
        .iter()
        .find(|p| p.config.provider == provider)
    else {
        return;
    };
    let current = connection_fingerprint(p);
    if snapshot.get(provider).map(|s| *s == current).unwrap_or(false) {
        state.verified.insert(provider.to_string(), current);
    }
}

/// Whether any saved connection references this credential.
/// The caller loads the persisted config first when state has
/// none; a removal decision must never rest on stale state.
fn referenced_credential(state: &BackendState, reference: &str) -> bool {
    state
        .config
        .iter()
        .flat_map(|c| c.providers.iter())
        .any(|p| {
            p.config
                .credential
                .as_ref()
                .map(|cr| cr.reference == reference)
                .unwrap_or(false)
        })
}

/// Path of the minted-reference ledger: the references this
/// backend minted for the workspace, mapped to the provider each
/// was minted for. Labels only — never secrets. Beside the
/// provider-config file, same sanitization, same atomic write.
fn minted_file_path(root: &std::path::Path, workspace_id: &str) -> std::path::PathBuf {
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
    root.join("provider-config").join(format!("{safe}.minted.json"))
}

/// Read the minted-reference ledger: missing file means no
/// references were ever minted. A corrupt file fails closed
/// (StorageUnavailable) rather than silently dropping orphans.
fn load_minted_file(
    root: &std::path::Path,
    workspace_id: &str,
) -> Result<HashMap<String, String>, CommandError> {
    let path = minted_file_path(root, workspace_id);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    crate::storage::reject_links(&path).map_err(|_| crate::storage::unavailable())?;
    let text = std::fs::read_to_string(&path).map_err(|_| crate::storage::unavailable())?;
    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| crate::storage::unavailable())?;
    let obj = parsed.as_object().ok_or_else(crate::storage::unavailable)?;
    let mut out = HashMap::new();
    for (reference, provider) in obj {
        let provider = provider.as_str().ok_or_else(crate::storage::unavailable)?;
        if reference.is_empty()
            || reference.len() > 128
            || reference.chars().any(|c| c.is_control() || c.is_whitespace())
            || provider.is_empty()
            || provider.len() > 64
        {
            return Err(crate::storage::unavailable());
        }
        out.insert(reference.clone(), provider.to_string());
    }
    Ok(out)
}

/// Persist the minted-reference ledger atomically (same hardened
/// pattern as the config file).
fn persist_minted_file(
    root: &std::path::Path,
    workspace_id: &str,
    minted: &HashMap<String, String>,
) -> Result<(), CommandError> {
    let dir = root.join("provider-config");
    std::fs::create_dir_all(&dir).map_err(|_| crate::storage::unavailable())?;
    crate::storage::reject_links(&dir).map_err(|_| crate::storage::unavailable())?;
    crate::storage::private_permissions(&dir, true, false)
        .map_err(|_| crate::storage::unavailable())?;
    let path = minted_file_path(root, workspace_id);
    if path.symlink_metadata().is_ok() {
        crate::storage::reject_links(&path).map_err(|_| crate::storage::unavailable())?;
    }
    let json = serde_json::to_string(minted).unwrap_or_default();
    persist_text_file_atomic(&dir, &path, &json)
}

/// Atomically write `text` to `path` inside `dir`: a unique
/// create-new temporary file (never following a planted file),
/// fsync before an atomic rename, directory synced afterwards.
/// Shared by the config file and the minted-reference ledger so
/// the two persisted artifacts cannot drift apart in handling.
fn persist_text_file_atomic(
    dir: &std::path::Path,
    path: &std::path::Path,
    text: &str,
) -> Result<(), CommandError> {
    let unavailable = || crate::storage::unavailable();
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
        file.write_all(text.as_bytes()).map_err(|_| unavailable())?;
        file.sync_all().map_err(|_| unavailable())?;
        drop(file);
        crate::storage::private_permissions(&tmp_path, false, false)
            .map_err(|_| unavailable())?;
        crate::storage::reject_links(&tmp_path).map_err(|_| unavailable())?;
        std::fs::rename(&tmp_path, path).map_err(|_| unavailable())?;
        // Best-effort directory sync for durability of the rename
        // itself (unsupported on some platforms — the rename is
        // already atomic there).
        if let Ok(d) = std::fs::File::open(dir) {
            let _ = d.sync_all();
        }
        Ok(())
    })();
    if outcome.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    outcome
}

/// Lazy-load the minted-reference ledger into backend state
/// (first use in this process), inside the caller's
/// `with_backend` closure. No storage root means no ledger.
fn load_minted_locked(
    state: &mut BackendState,
    scope: &CallerScope,
) -> Result<(), CommandError> {
    if state.minted_loaded {
        return Ok(());
    }
    if let (Some(root), Some(wid)) = (&scope.storage_root, &scope.logical_workspace) {
        state.minted_references = load_minted_file(root, wid)?;
    }
    state.minted_loaded = true;
    Ok(())
}

/// Drop minted entries that are neither stored in the keyring
/// nor referenced by a saved connection: a minted-but-never-used
/// reference, or one whose key was deleted elsewhere. Keeps the
/// ledger bounded; called before minting a fresh reference.
fn prune_minted_locked(state: &mut BackendState) {
    let resolver = shared_resolver();
    let referenced: std::collections::HashSet<String> = state
        .config
        .iter()
        .flat_map(|c| c.providers.iter())
        .filter_map(|p| p.config.credential.as_ref().map(|cr| cr.reference.clone()))
        .collect();
    state.minted_references.retain(|reference, _| {
        resolver.exists(reference) || referenced.contains(reference)
    });
}

pub fn install_persisted_config(scope: &CallerScope, json: &str) -> Result<bool, CommandError> {
    let config: RuntimeConfig = serde_json::from_str(json).map_err(|_| {
        invalid("The persisted provider configuration is unreadable; reconfigure providers.")
    })?;
    validate_config_bounds(&config)?;
    enforce_derived_locality(&config)?;
    with_backend(scope, |state| install_config_locked(state, config))
}

/// Generate a host-side credential reference for a provider:
/// `{provider}-{16 lowercase hex}` from OS randomness. The
/// reference is a keyring account label, never a secret.
fn generate_credential_reference(provider: &str) -> String {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("OS randomness for credential references");
    format!("{provider}-{:016x}", u64::from_le_bytes(bytes))
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
    persist_text_file_atomic(&dir, &path, json)
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
                // A changed endpoint or credential clears that
                // provider's verification (Connect AI repair);
                // unchanged providers keep theirs.
                invalidate_changed_verifications(state, &req.config);
                state.config = Some(req.config);
                state.runtime = Some((fp, Arc::new(runtime)));
                Ok(())
            })?;
            to_value(&ProviderConfigureResponse {
                contract_version: 1,
                providers,
            })
        }
        "providers-configure-add" => {
            let req: ProviderConfigureAddRequest = parse(value, "provider configuration add")?;
            require_contract_v1(req.contract_version)?;
            // The entry is validated as part of the merged config
            // below (bounds, derived locality, runtime build), so
            // a bad entry fails exactly like a bad full configure.
            let added_id = req.provider.config.provider.clone();
            if added_id.is_empty() || added_id.len() > 64 {
                return Err(invalid("Name the provider to add."));
            }
            let (providers, updated) = with_backend(scope, |state| {
                require_owner_recording(state, scope)?;
                // Restart case: bring the persisted configuration
                // into memory first, so the add merges with what
                // is on disk rather than overwriting it with a
                // single-provider config.
                load_persisted_config_locked(state, scope)?;
                let mut merged = state.config.clone().unwrap_or_default();
                let updated = if let Some(slot) = merged
                    .providers
                    .iter_mut()
                    .find(|p| p.config.provider == added_id)
                {
                    *slot = req.provider.clone();
                    true
                } else {
                    merged.providers.push(req.provider.clone());
                    false
                };
                validate_config_bounds(&merged)?;
                enforce_derived_locality(&merged)?;
                // Validate by building the runtime now, so a bad
                // merged configuration fails at add time — with
                // the workspace's shared accounting wired in.
                let runtime = production_runtime(merged.clone())
                    .map_err(to_command_error)?
                    .with_shared_accounting(state.used_permits.clone(), state.ledger.clone());
                let serialized = serde_json::to_string(&merged).unwrap_or_default();
                // Persist BEFORE installing, under this same
                // backend lock: if persistence fails, the
                // operation fails and memory stays exactly as it
                // was — the previous providers, budget, and
                // usage accounting are untouched.
                if let (Some(root), Some(wid)) = (&scope.storage_root, &scope.logical_workspace) {
                    persist_config_file(root, wid, &serialized)?;
                }
                let fp = fingerprint(&serialized);
                // A changed endpoint or credential clears that
                // provider's verification (Connect AI repair);
                // unchanged providers keep theirs.
                invalidate_changed_verifications(state, &merged);
                state.config = Some(merged.clone());
                state.runtime = Some((fp, Arc::new(runtime)));
                let providers: Vec<String> = merged
                    .providers
                    .iter()
                    .map(|p| p.config.provider.clone())
                    .collect();
                Ok((providers, updated))
            })?;
            to_value(&ProviderConfigureAddResponse {
                contract_version: 1,
                providers,
                added: added_id,
                updated,
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
            //
            // When the caller bound this dispatch to a reviewed
            // approval, phase 1 also verifies the approved target
            // against the live configuration in this same locked
            // section, before acquiring the runtime: a target that
            // changed after approval is refused here, before any
            // traffic, and a configuration change between two
            // dispatches can never redirect the later one.
            let (permit, runtime, snapshot) = with_backend(scope, |state| {
                enforce_configured_model(state, &req.request)?;
                if let Some(expected) = &req.expected_target {
                    enforce_expected_target(state, expected, req.chosen.as_ref(), routing.mode)?;
                }
                let permit = take_permit(state, scope, &req.permit_id)?;
                let runtime = runtime_for(state)?;
                // Verification binding (Connect AI repair): the
                // connection fingerprints as they stand before
                // the network call. A success records verification
                // only if the saved connection still matches this
                // snapshot when the call lands.
                let snapshot = state
                    .config
                    .as_ref()
                    .map(snapshot_fingerprints)
                    .unwrap_or_default();
                Ok((permit, runtime, snapshot))
            })?;
            let outcome = runtime
                .dispatch(&req.request, routing, permit, &CancellationToken::new())
                .map_err(to_command_error)?;
            // The serving provider is the candidate that actually
            // succeeded (`CompletionResponse.provider`); record
            // verification only for the connection it used, and
            // only if that connection is unchanged since phase 1.
            // Usage and budgets are untouched — the runtime
            // settles the ledger independently.
            let served = outcome.response.provider.clone();
            let _ = with_backend(scope, |state| {
                record_verification_if_unchanged(state, &served, &snapshot);
                Ok(())
            });
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
            // Stream-start does not enforce an expected target, so the
            // field is refused here — before traffic and before the
            // locked phase — rather than accepted and silently ignored.
            // No existing caller sends it (collider, forge, ideas,
            // opportunities); a future approval-bound stream must
            // enforce it the same way unary dispatch does.
            if req.expected_target.is_some() {
                return Err(invalid(
                    "expectedTarget is not supported on stream-start; use a unary dispatch for an approval-bound target.",
                ));
            }
            let routing = routing_request(&req)?;
            // Locked phase: policy resolution (model, cap
            // pre-check, permit, runtime). Unlocked: stream
            // establishment (a network call). Locked again: run
            // registration — if the retained cap filled meanwhile,
            // the just-established stream is cancelled and the
            // start refused rather than exceeding the bound.
            let (permit, runtime, snapshot) = with_backend(scope, |state| {
                enforce_configured_model(state, &req.request)?;
                if state.streams.len() >= MAX_RETAINED_STREAMS {
                    return Err(invalid(
                        "Too many retained provider streams for this workspace.",
                    ));
                }
                let permit = take_permit(state, scope, &req.permit_id)?;
                let runtime = runtime_for(state)?;
                // Verification binding (Connect AI repair): same
                // snapshot as unary dispatch, checked when the
                // stream completes.
                let snapshot = state
                    .config
                    .as_ref()
                    .map(snapshot_fingerprints)
                    .unwrap_or_default();
                Ok((permit, runtime, snapshot))
            })?;
            let cancel = CancellationToken::new();
            let outcome = runtime
                .dispatch_stream(&req.request, routing, permit, &cancel)
                .map_err(to_command_error)?;
            let disclosure = outcome.disclosure.clone();
            // The disclosure names the provider that established
            // this stream (built from the successful candidate).
            let verified_provider = disclosure.provider.clone();
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
                        verified_provider,
                        verified_snapshot: snapshot,
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
                // Verification binding (Connect AI repair): set
                // only when this poll first observes the terminal
                // Completed event — a cancelled or errored stream
                // never verifies, and a stream completing after a
                // configuration change verifies nothing.
                let mut completed_now = false;
                let mut verified_provider = String::new();
                let mut verified_snapshot: HashMap<String, String> = HashMap::new();
                if !run.done {
                    loop {
                        match run.outcome.events.try_recv() {
                            Ok(event) => {
                                let terminal =
                                    matches!(event, Ok(StreamEvent::Completed { .. }) | Err(_));
                                if matches!(&event, Ok(StreamEvent::Completed { .. })) {
                                    completed_now = true;
                                    verified_provider = run.verified_provider.clone();
                                    verified_snapshot = run.verified_snapshot.clone();
                                }
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
                if completed_now {
                    record_verification_if_unchanged(state, &verified_provider, &verified_snapshot);
                }
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
        "providers-credential-reference" => {
            let req: ProviderCredentialReferenceRequest =
                parse(value, "provider credential reference")?;
            require_contract_v1(req.contract_version)?;
            if req.provider.is_empty() || req.provider.len() > 64 {
                return Err(invalid("Name the provider for this credential."));
            }
            // Owner-only: the reference names the keyring account
            // the host prompt will store under, so a browser must
            // not mint labels the host never approved. A refusal
            // is recorded for the host's approval queue.
            with_backend(scope, |state| require_owner_recording(state, scope))?;
            let resolver = shared_resolver();
            // Bounded retries on collision: a reference that
            // already has a stored credential must never be
            // handed out as "fresh".
            let mut reference = String::new();
            for _ in 0..8 {
                let candidate = generate_credential_reference(&req.provider);
                if !resolver.exists(&candidate) {
                    reference = candidate;
                    break;
                }
            }
            if reference.is_empty() {
                return Err(crate::storage::unavailable());
            }
            // Record the mint in the persisted ledger (Connect AI
            // repair): the keyring cannot be enumerated, so
            // without this record an abandoned setup key could
            // never be found again for cleanup. A persist failure
            // fails the mint — nothing was stored yet, so a retry
            // is safe.
            with_backend(scope, |state| {
                load_persisted_config_locked(state, scope)?;
                load_minted_locked(state, scope)?;
                prune_minted_locked(state);
                state
                    .minted_references
                    .insert(reference.clone(), req.provider.clone());
                if let (Some(root), Some(wid)) = (&scope.storage_root, &scope.logical_workspace) {
                    persist_minted_file(root, wid, &state.minted_references)?;
                }
                Ok(())
            })?;
            to_value(&ProviderCredentialReferenceResponse {
                contract_version: 1,
                provider: req.provider,
                reference,
            })
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
            // Key rotation under the same reference is invisible
            // to the connection fingerprint, so invalidate
            // explicitly (Connect AI repair). Best effort: the key
            // is already stored; a failure here must not report
            // the set itself as failed.
            let _ = with_backend(scope, |state| {
                load_persisted_config_locked(state, scope)?;
                invalidate_verification_for_reference(state, &req.reference);
                Ok(())
            });
            to_value(&ProviderCredentialSetResponse {
                contract_version: 1,
                stored: true,
            })
        }
        "providers-credential-remove" => {
            let req: ProviderCredentialRequest = parse(value, "provider credential")?;
            require_contract_v1(req.contract_version)?;
            crate::provider_credentials::validate_reference(&req.reference)?;
            // Guard (Connect AI repair): a credential referenced
            // by a saved connection must never be deleted — while
            // editing that provider, or while another setup
            // overlaps it. The check runs against the persisted
            // config when state has none, so a restart cannot
            // bypass it.
            with_backend(scope, |state| {
                require_owner_recording(state, scope)?;
                load_persisted_config_locked(state, scope)?;
                if referenced_credential(state, &req.reference) {
                    return Err(invalid(
                        "This credential is in use by a saved connection. Save the connection under a different credential first, or remove the provider.",
                    ));
                }
                Ok(())
            })?;
            let resolver = shared_resolver();
            resolver.delete(&req.reference).map_err(to_command_error)?;
            // The key is gone: drop the ledger entry (best-effort
            // persist — a stale label for a deleted key is
            // harmless; it is pruned on the next mint) and
            // invalidate any verification bound to it.
            let _ = with_backend(scope, |state| {
                load_minted_locked(state, scope)?;
                state.minted_references.remove(&req.reference);
                if let (Some(root), Some(wid)) = (&scope.storage_root, &scope.logical_workspace) {
                    let _ = persist_minted_file(root, wid, &state.minted_references);
                }
                load_persisted_config_locked(state, scope)?;
                invalidate_verification_for_reference(state, &req.reference);
                Ok(())
            });
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
        "providers-status" => {
            // Browser-readable setup status (Connect AI slice):
            // no owner check, like `providers-usage`. A paired
            // browser may see what is configured, whether a
            // credential is stored, and whether the connection
            // has been verified in this app session — and is
            // directed to the desktop to enter credentials. No
            // secret ever leaves the keyring.
            let req: ProviderWorkspaceRequest = parse(value, "provider status")?;
            require_contract_v1(req.contract_version)?;
            with_backend(scope, |state| {
                // Lazy-load persisted state: the config (a restart
                // must not hide saved providers) and the
                // minted-reference ledger (a restart must not hide
                // abandoned setup keys).
                load_persisted_config_locked(state, scope)?;
                load_minted_locked(state, scope)?;
                let config = state.config.clone().unwrap_or_default();
                let resolver = shared_resolver();
                let catalog = anansi_providers::catalog::catalog();
                let providers = config
                    .providers
                    .iter()
                    .map(|p| {
                        let reference = p.config.credential.as_ref().map(|c| c.reference.clone());
                        let credential_stored = reference
                            .as_ref()
                            .map(|r| resolver.exists(r))
                            .unwrap_or(false);
                        let base_url_host = p.config.validate().map(|e| e.host).unwrap_or_default();
                        // Verification is bound to THIS saved
                        // connection (Connect AI repair): the
                        // stored fingerprint must equal the
                        // current connection's fingerprint. Usage
                        // history is never consulted here.
                        let fingerprint = connection_fingerprint(p);
                        let connection_verified = state
                            .verified
                            .get(&p.config.provider)
                            .map(|v| *v == fingerprint)
                            .unwrap_or(false);
                        ProviderStatusEntry {
                            provider: p.config.provider.clone(),
                            display_name: catalog
                                .iter()
                                .find(|e| e.id == p.config.provider)
                                .map(|e| e.display_name.to_string())
                                .unwrap_or_else(|| p.config.provider.clone()),
                            base_url_host,
                            locality: format!("{:?}", p.locality),
                            enabled: p.config.enabled,
                            models: p
                                .models
                                .iter()
                                .map(|m| ProviderStatusModel {
                                    id: m.id.clone(),
                                    cost: format!("{:?}", m.cost).to_lowercase(),
                                })
                                .collect(),
                            credential_reference: reference,
                            credential_stored,
                            connection_verified,
                        }
                    })
                    .collect();
                // Orphaned credentials (Connect AI repair):
                // minted references that are stored in the keyring
                // but no saved connection references — keys left
                // behind by a discarded or abandoned setup. Labels
                // only; the keyring cannot be enumerated, so only
                // references this backend minted can be listed.
                let referenced: std::collections::HashSet<String> = config
                    .providers
                    .iter()
                    .filter_map(|p| p.config.credential.as_ref().map(|c| c.reference.clone()))
                    .collect();
                let mut orphaned_credentials: Vec<ProviderStatusOrphanedCredential> = state
                    .minted_references
                    .iter()
                    .filter(|(reference, _)| {
                        resolver.exists(reference) && !referenced.contains(*reference)
                    })
                    .map(|(reference, provider)| ProviderStatusOrphanedCredential {
                        provider: provider.clone(),
                        reference: reference.clone(),
                    })
                    .collect();
                orphaned_credentials.sort_by(|a, b| a.reference.cmp(&b.reference));
                to_value(&ProviderStatusResponse {
                    contract_version: 1,
                    providers,
                    orphaned_credentials,
                    budget: config.budget,
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
    use anansi_providers::config::ProviderConfig;
    use anansi_providers::secrets::CredentialRef;
    use anansi_providers::types::ModelInfo;

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
            verified_provider: "ollama".into(),
            verified_snapshot: HashMap::new(),
        }
    }

    #[test]
    fn minted_ledger_roundtrips_and_fails_closed_on_corruption() {
        // Connect AI repair: the ledger is the only record of
        // abandoned setup keys, so its read/write path is tested
        // directly (the integration restart test shares the
        // process-wide backend, so it cannot prove the load).
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut minted = HashMap::new();
        minted.insert("openai-0123456789abcdef".to_string(), "openai".to_string());
        minted.insert("gemini-0123456789abcdef".to_string(), "gemini".to_string());
        persist_minted_file(root, "ws-main", &minted).unwrap();
        assert_eq!(load_minted_file(root, "ws-main").unwrap(), minted);
        assert_eq!(
            load_minted_file(root, "ws-never-minted").unwrap(),
            HashMap::new(),
            "a workspace that never minted has an empty ledger"
        );
        let path = minted_file_path(root, "ws-main");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("sk-"), "ledger holds labels only");
        std::fs::write(&path, "{not json").unwrap();
        assert!(
            load_minted_file(root, "ws-main").is_err(),
            "a corrupt ledger must fail closed, never silently drop orphans"
        );
    }

    #[test]
    fn connection_fingerprint_ignores_models_but_not_endpoint_or_key() {
        // The fingerprint binds the connection, not the model
        // list: an endpoint or credential change clears
        // verification, a model-list change does not.
        let base = RuntimeProvider {
            config: ProviderConfig {
                provider: "openai".into(),
                base_url: "https://api.openai.com/v1".into(),
                credential: Some(CredentialRef::new("openai", "openai-aaaa")),
                enabled: true,
                custom_endpoint: false,
                allow_insecure_custom: false,
            },
            locality: LocalityConfig::Cloud,
            models: vec![ModelInfo {
                id: "gpt-x".into(),
                provider: "openai".into(),
                capabilities: Default::default(),
                context_window: Some(8192),
                cost: CostClass::Free,
                speed: anansi_providers::types::SpeedTier::Balanced,
                price_input_micro_usd_per_mtok: None,
                price_output_micro_usd_per_mtok: None,
            }],
        };
        let same = base.clone();
        let mut model_changed = base.clone();
        model_changed.models[0].id = "gpt-y".into();
        let mut endpoint_changed = base.clone();
        endpoint_changed.config.base_url = "https://other.example/v1".into();
        let mut credential_changed = base.clone();
        credential_changed.config.credential =
            Some(CredentialRef::new("openai", "openai-bbbb"));
        assert_eq!(connection_fingerprint(&base), connection_fingerprint(&same));
        assert_eq!(
            connection_fingerprint(&base),
            connection_fingerprint(&model_changed),
            "a model-list change is not a connection change"
        );
        assert_ne!(
            connection_fingerprint(&base),
            connection_fingerprint(&endpoint_changed)
        );
        assert_ne!(
            connection_fingerprint(&base),
            connection_fingerprint(&credential_changed)
        );
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
