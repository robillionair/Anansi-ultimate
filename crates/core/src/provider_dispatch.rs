//! Application integration for the M5 provider subsystem
//! (`anansi-providers`): exposes provider configuration, authorized
//! dispatch, streaming with poll/cancel, and the disclosure record
//! through `Core::ai_command` operations:
//!
//! - `providers-catalog` — the provider catalog (no configuration).
//! - `providers-dispatch` — one unary dispatch
//!   (configuration → gates → HTTPS transport → response +
//!   disclosure).
//! - `providers-stream-start` / `providers-stream-poll` /
//!   `providers-stream-cancel` — the streamed variant, mirroring
//!   the ask run pattern.
//!
//! The runtime is process-wide and cached by configuration
//! fingerprint (see `with_runtime`): the quota ledger and the
//! single-use permit set must live across calls, or a permit could
//! authorize twice — once per call. Credentials never pass through
//! this module: configurations carry credential *references* only,
//! resolved inside the runtime by the native keyring resolver.
//!
//! Error mapping (integration note for the API owners): provider
//! configuration problems map to `InvalidRequest`, credential
//! problems to `Unauthorized`/`Forbidden`, and dispatch failures to
//! `StorageUnavailable` with the provider error code preserved at
//! the front of the message (e.g. "ProviderUnavailable: …") — the
//! message text is the providers crate's own, already redacted.

use crate::readiness::{CommandError, ErrorCode};
use anansi_providers::dispatcher::StreamOutcome;
use anansi_providers::error::{ProviderError, ProviderErrorCode};
use anansi_providers::gates::{PaidInferencePermit, ZeroCostLock};
use anansi_providers::http::CancellationToken;
use anansi_providers::routing::{DisclosureRecord, RoutingMode, RoutingRequest};
use anansi_providers::runtime::{ProviderRuntime, RuntimeConfig};
use anansi_providers::types::{CompletionRequest, StreamEvent};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Request shapes

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DispatchRequest {
    config: RuntimeConfig,
    request: CompletionRequest,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    zero_cost_lock: bool,
    #[serde(default)]
    chosen: Option<(String, String)>,
    #[serde(default)]
    permit: Option<PaidInferencePermit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunRequest {
    run_id: String,
}

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
        Some(other) => Err(CommandError::new(
            ErrorCode::InvalidRequest,
            &format!("Unknown routing mode: {other}"),
        )),
    }
}

fn routing_request(req: &DispatchRequest) -> Result<RoutingRequest, CommandError> {
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

// ---------------------------------------------------------------------------
// Runtime cache: one runtime per active configuration, so the quota
// ledger and the single-use permit set persist across operations.

struct RuntimeSlot {
    fingerprint: u64,
    runtime: ProviderRuntime,
}

fn runtime_slot() -> &'static Mutex<Option<RuntimeSlot>> {
    static SLOT: OnceLock<Mutex<Option<RuntimeSlot>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

fn fingerprint(config: &RuntimeConfig) -> u64 {
    let mut hasher = DefaultHasher::new();
    serde_json::to_string(config)
        .unwrap_or_default()
        .hash(&mut hasher);
    hasher.finish()
}

fn with_runtime<T>(
    config: &RuntimeConfig,
    f: impl FnOnce(&mut ProviderRuntime) -> Result<T, ProviderError>,
) -> Result<T, CommandError> {
    let mut guard = runtime_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fp = fingerprint(config);
    let stale = guard
        .as_ref()
        .map(|slot| slot.fingerprint != fp)
        .unwrap_or(true);
    if stale {
        let runtime = ProviderRuntime::production(config.clone()).map_err(to_command_error)?;
        *guard = Some(RuntimeSlot {
            fingerprint: fp,
            runtime,
        });
    }
    let slot = guard.as_mut().expect("runtime slot populated above");
    f(&mut slot.runtime).map_err(to_command_error)
}

fn to_command_error(e: ProviderError) -> CommandError {
    let code = match e.code {
        ProviderErrorCode::InvalidConfiguration
        | ProviderErrorCode::EndpointRejected
        | ProviderErrorCode::Unsupported => ErrorCode::InvalidRequest,
        ProviderErrorCode::AuthFailed => ErrorCode::Unauthorized,
        ProviderErrorCode::PermissionRefused => ErrorCode::Forbidden,
        _ => ErrorCode::StorageUnavailable,
    };
    CommandError::new(code, &format!("{}: {}", e.code_name(), e.message))
}

// ---------------------------------------------------------------------------
// Stream run registry (mirrors the ask-stream pattern).

struct StreamRun {
    outcome: StreamOutcome,
    cancel: CancellationToken,
    done: bool,
}

fn stream_runs() -> &'static Mutex<HashMap<String, StreamRun>> {
    static RUNS: OnceLock<Mutex<HashMap<String, StreamRun>>> = OnceLock::new();
    RUNS.get_or_init(|| Mutex::new(HashMap::new()))
}

static RUN_COUNTER: AtomicU64 = AtomicU64::new(1);

fn event_json(event: &Result<StreamEvent, ProviderError>) -> Value {
    match event {
        Ok(StreamEvent::Token(text)) => json!({ "kind": "token", "text": text }),
        Ok(StreamEvent::Usage(u)) => json!({
            "kind": "usage",
            "inputTokens": u.input_tokens,
            "outputTokens": u.output_tokens,
        }),
        Ok(StreamEvent::Completed { finish_reason }) => json!({
            "kind": "completed",
            "finishReason": finish_reason,
        }),
        Err(e) => json!({
            "kind": "error",
            "code": e.code_name(),
            "message": e.message,
        }),
    }
}

// ---------------------------------------------------------------------------
// Operations

pub fn providers_command(operation: &str, value: Value) -> Result<Value, CommandError> {
    match operation {
        "providers-catalog" => Ok(json!({
            "providers": anansi_providers::catalog::catalog()
                .iter()
                .map(|e| json!({
                    "id": e.id,
                    "displayName": e.display_name,
                    "baseUrl": e.base_url,
                    "docsUrl": e.docs_url,
                    "locality": format!("{:?}", e.locality),
                    "status": format!("{:?}", e.status),
                }))
                .collect::<Vec<_>>(),
        })),
        "providers-dispatch" => {
            let req: DispatchRequest = serde_json::from_value(value).map_err(|_| {
                CommandError::new(
                    ErrorCode::InvalidRequest,
                    "Use a supported provider dispatch request.",
                )
            })?;
            let routing = routing_request(&req)?;
            let outcome = with_runtime(&req.config, |runtime| {
                runtime.dispatch(
                    &req.request,
                    routing,
                    req.permit.clone(),
                    &CancellationToken::new(),
                )
            })?;
            Ok(json!({
                "response": {
                    "text": outcome.response.text,
                    "model": outcome.response.model,
                    "finishReason": outcome.response.finish_reason,
                    "usage": outcome.response.usage.map(|u| json!({
                        "inputTokens": u.input_tokens,
                        "outputTokens": u.output_tokens,
                    })),
                },
                "disclosure": outcome.disclosure,
            }))
        }
        "providers-stream-start" => {
            let req: DispatchRequest = serde_json::from_value(value).map_err(|_| {
                CommandError::new(
                    ErrorCode::InvalidRequest,
                    "Use a supported provider dispatch request.",
                )
            })?;
            let routing = routing_request(&req)?;
            let cancel = CancellationToken::new();
            let run_cancel = cancel.clone();
            let outcome = with_runtime(&req.config, |runtime| {
                runtime.dispatch_stream(&req.request, routing, req.permit.clone(), &cancel)
            })?;
            let disclosure: DisclosureRecord = outcome.disclosure.clone();
            let run_id = format!(
                "provider-run-{}",
                RUN_COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            stream_runs()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(
                    run_id.clone(),
                    StreamRun {
                        outcome,
                        cancel: run_cancel,
                        done: false,
                    },
                );
            Ok(json!({ "runId": run_id, "disclosure": disclosure }))
        }
        "providers-stream-poll" => {
            let req: RunRequest = serde_json::from_value(value).map_err(|_| {
                CommandError::new(ErrorCode::InvalidRequest, "Use a supported run request.")
            })?;
            let mut runs = stream_runs()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let run = runs.get_mut(&req.run_id).ok_or_else(|| {
                CommandError::new(ErrorCode::NotFound, "Unknown provider stream run.")
            })?;
            let mut events = Vec::new();
            if !run.done {
                loop {
                    match run.outcome.events.try_recv() {
                        Ok(event) => {
                            let terminal =
                                matches!(event, Ok(StreamEvent::Completed { .. }) | Err(_));
                            events.push(event_json(&event));
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
                runs.remove(&req.run_id);
            }
            Ok(json!({
                "events": events,
                "done": done,
                "disclosure": disclosure,
            }))
        }
        "providers-stream-cancel" => {
            let req: RunRequest = serde_json::from_value(value).map_err(|_| {
                CommandError::new(ErrorCode::InvalidRequest, "Use a supported run request.")
            })?;
            let runs = stream_runs()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let run = runs.get(&req.run_id).ok_or_else(|| {
                CommandError::new(ErrorCode::NotFound, "Unknown provider stream run.")
            })?;
            run.cancel.cancel();
            Ok(json!({ "cancelled": true }))
        }
        _ => Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "Unknown provider operation.",
        )),
    }
}
