// ---------------------------------------------------------------------------
// SwarmTank v2 Checkpoint 2: genuinely AI-driven simulation mode.
//
// The seeded simulator stays as Quick simulation (crates/core/src/
// swarmtank.rs). This module adds AI simulation: distinct synthetic
// agents (one per Quick persona) with individual goals, constraints
// and bounded memory, run round by round through the existing
// provider layer (`providers-dispatch`, synchronous), with server-side
// caps on rounds, calls and tokens, cancellation between calls and
// between rounds, and a versioned `swarmtank-ai-run` artifact (format
// v1, fail-closed content hash) so a completed run reopens with no
// provider call.
//
// Architecture: `swarmtank-ai-start` validates the request, resolves
// the model from the configured chat models (local-first default),
// checks the Zero-Cost Lock up front (a cloud model without an
// explicit permit refuses before any call), builds the agents, and
// spawns a background thread that runs the simulation. The run lives
// in a process-local registry (mirroring ideas.rs) so `swarmtank-ai-
// poll` can report live progress and `swarmtank-ai-cancel` can stop
// it. The thread saves the artifact when the run reaches a terminal
// state (complete, cancelled, budget-exhausted, or failed with at
// least one recorded round).
//
// A malformed model output is never retried into a different action:
// the call is recorded as failed in the run history with the raw
// output (truncated), that agent's round is recorded as failed, and
// the run continues with the remaining agents. A failed provider call
// is recorded the same way. Budget exhaustion ends the run in a clear
// "budget-exhausted" state, not an error and not a silent stop.
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::provider_dispatch::{configured_chat_models, providers_command, CallerScope};
use crate::swarmtank::{generate_personas, generate_personas_for_market, SwarmtankInput};
use crate::swarmtank_market::{validate_market, SwarmtankMarket};
use crate::{CommandError, ErrorCode};

// ---------------------------------------------------------------------------
// Caps (server-side, enforced by the runner)
// ---------------------------------------------------------------------------

/// Default agents for the first end-to-end run (small enough to test on
/// ordinary hardware).
pub const DEFAULT_AI_AGENTS: usize = 3;
/// Default rounds for the first end-to-end run.
pub const DEFAULT_AI_ROUNDS: u32 = 2;
pub const MIN_AI_AGENTS: usize = 1;
pub const MAX_AI_AGENTS: usize = 10;
pub const MIN_AI_ROUNDS: u32 = 1;
pub const MAX_AI_ROUNDS: u32 = 10;
/// Hard cap on provider calls in one run (agents x rounds, plus headroom).
pub const MAX_AI_CALLS: u32 = 100;
pub const MIN_AI_OUTPUT_TOKENS: u32 = 64;
pub const MAX_AI_OUTPUT_TOKENS: u32 = 2048;
pub const DEFAULT_AI_OUTPUT_TOKENS: u32 = 512;
pub const MIN_AI_TOTAL_TOKENS: u64 = 1_000;
pub const MAX_AI_TOTAL_TOKENS: u64 = 200_000;
pub const DEFAULT_AI_TOTAL_TOKENS: u64 = 20_000;
/// Bounded memory: the per-agent window keeps the last N relevant events
/// plus selected prior interactions, capped by count and by characters.
pub const MEMORY_MAX_EVENTS: usize = 5;
pub const MEMORY_MAX_CHARS: usize = 4_000;
/// Raw model output kept on a failed (malformed) call, truncated.
pub const MAX_RAW_OUTPUT_CHARS: usize = 500;
/// A run is evicted from the registry after this long; a saved artifact
/// is the durable record, the registry is only the live view.
const RUN_TTL: Duration = Duration::from_secs(600);
const MAX_RUNS_PER_WORKSPACE: usize = 32;

/// The four validated actions an agent may submit in a round.
pub const AI_ACTIONS: [&str; 4] = ["request_evidence", "defer", "recommend", "reject"];

// ---------------------------------------------------------------------------
// Stored run document (artifact body, format v1)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiAgent {
    pub id: String,
    pub persona_id: String,
    pub name: String,
    pub role: String,
    pub goal: String,
    pub constraints: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiEvent {
    pub id: String,
    pub round: u32,
    pub kind: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiMemoryWindow {
    pub max_events: usize,
    pub max_chars: usize,
    pub observed_event_ids: Vec<String>,
    pub observed_interaction_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiAction {
    pub agent_id: String,
    pub round: u32,
    /// The validated action, or null when the call failed (no valid
    /// action was produced). A null action with status "failed" is a
    /// recorded failure, never a silent drop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_interaction_ref: Option<String>,
    /// "ok" | "failed"
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<AiUsage>,
    pub memory: AiMemoryWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiRound {
    pub round: u32,
    pub actions: Vec<AiAction>,
    pub events_due: Vec<String>,
    /// ISO date (YYYY-MM-DD) this round simulates, when the run has a
    /// time dimension (an end date). Absent on runs without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiCaps {
    pub max_agents: usize,
    pub max_rounds: u32,
    pub max_calls: u32,
    pub max_output_tokens: u32,
    pub max_total_tokens: u64,
    pub memory_max_events: usize,
    pub memory_max_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankAiRunDoc {
    pub format: String,
    pub version: u32,
    pub run_id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub input: SwarmtankInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market: Option<SwarmtankMarket>,
    pub seed: String,
    pub agents: Vec<AiAgent>,
    pub scheduled_events: Vec<AiEvent>,
    pub rounds: Vec<AiRound>,
    /// "complete" | "cancelled" | "budget-exhausted" | "failed"
    pub status: String,
    pub model_provider: String,
    pub model: String,
    /// "local" | "cloud"
    pub locality: String,
    pub caps: AiCaps,
    pub calls_made: u32,
    pub tokens_used: u64,
    pub assumptions: Vec<String>,
    pub content_hash: String,
    pub created_ms: u64,
    /// ISO date (YYYY-MM-DD) the run starts from (its creation date).
    pub start_date: String,
    /// ISO date the run simulates toward, when the user chose one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    /// Step size in days between rounds.
    pub step_days: u32,
    /// True when the horizon forced a coarser step than requested.
    pub step_coarsened: bool,
    /// Branch lineage: the parent run and round this run continues from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_round: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_changes: Option<BranchChanges>,
}

/// What changed on a what-if branch, recorded with the run.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BranchChanges {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_amount: Option<f64>,
    #[serde(default)]
    pub new_event_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Request / response DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankAiEventRequest {
    pub round: u32,
    pub kind: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankAiStartRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub input: SwarmtankInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market: Option<SwarmtankMarket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_total_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permit_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_events: Option<Vec<SwarmtankAiEventRequest>>,
    /// Future end date (YYYY-MM-DD) the run simulates toward.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    /// Step size in days between rounds (1, 7, 30 or 90).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankAiStartResponse {
    pub contract_version: u8,
    pub run_id: String,
    pub provider: String,
    pub model: String,
    pub locality: String,
    pub estimated_calls: u32,
    pub estimated_tokens: u64,
    pub step_count: u32,
    pub effective_step_days: u32,
    pub step_coarsened: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankAiPollRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankAiPollResponse {
    pub contract_version: u8,
    pub run_id: String,
    /// "running" | "complete" | "cancelled" | "budget-exhausted" | "failed"
    pub status: String,
    pub agents: Vec<AiAgent>,
    pub rounds: Vec<AiRound>,
    pub calls_made: u32,
    pub tokens_used: u64,
    pub provider: String,
    pub model: String,
    pub locality: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankAiCancelRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankAiCancelResponse {
    pub contract_version: u8,
    pub run_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankAiEstimateRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub input: SwarmtankInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market: Option<SwarmtankMarket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_total_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_days: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_events: Option<Vec<SwarmtankAiEventRequest>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankAiEstimateResponse {
    pub contract_version: u8,
    pub provider: String,
    pub model: String,
    pub locality: String,
    pub estimated_calls: u32,
    pub estimated_tokens: u64,
    pub step_count: u32,
    pub effective_step_days: u32,
    pub step_coarsened: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankAiBranchRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub parent_artifact_id: String,
    pub parent_round: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_events: Option<Vec<SwarmtankAiEventRequest>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_total_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permit_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankAiBranchResponse {
    pub contract_version: u8,
    pub run_id: String,
    pub parent_run_id: String,
    pub parent_round: u32,
    pub provider: String,
    pub model: String,
    pub locality: String,
    pub estimated_calls: u32,
    pub estimated_tokens: u64,
    pub step_count: u32,
    pub effective_step_days: u32,
    pub step_coarsened: bool,
}

// ---------------------------------------------------------------------------
// Run registry (process-local, like ideas.rs)
// ---------------------------------------------------------------------------

struct AiRun {
    session: String,
    status: String,
    agents: Vec<AiAgent>,
    rounds: Vec<AiRound>,
    calls_made: u32,
    tokens_used: u64,
    provider: String,
    model: String,
    locality: String,
    cancel: Arc<AtomicBool>,
    artifact_id: Option<String>,
    artifact_version: Option<u32>,
    error: Option<String>,
    created: Instant,
}

static AI_RUNS: Mutex<Option<HashMap<String, AiRun>>> = Mutex::new(None);

fn with_ai_runs<T>(f: impl FnOnce(&mut HashMap<String, AiRun>) -> T) -> T {
    let mut guard = AI_RUNS.lock().unwrap_or_else(|p| p.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

fn ai_run_key(scope: &CallerScope, run_id: &str) -> String {
    format!("{}/{}", scope.workspace, run_id)
}

fn new_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("swarmtank-ai-{nanos:x}")
}

fn collect_ai_evictions(workspace_key: Option<&str>) {
    with_ai_runs(|map| {
        let expired: Vec<String> = map
            .iter()
            .filter(|(_, r)| r.created.elapsed() >= RUN_TTL)
            .map(|(k, _)| k.clone())
            .collect();
        for key in expired {
            map.remove(&key);
        }
        if let Some(workspace) = workspace_key {
            let prefix = format!("{workspace}/");
            loop {
                let count = map.keys().filter(|k| k.starts_with(&prefix)).count();
                if count < MAX_RUNS_PER_WORKSPACE {
                    break;
                }
                let oldest = map
                    .iter()
                    .filter(|(k, _)| k.starts_with(&prefix))
                    .min_by_key(|(_, r)| r.created)
                    .map(|(k, _)| k.clone());
                match oldest {
                    Some(k) => {
                        map.remove(&k);
                    }
                    None => break,
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

fn invalid(message: &str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn not_found() -> CommandError {
    CommandError::new(ErrorCode::NotFound, "AI run not found.")
}

pub fn swarmtank_ai_command(
    operation: &str,
    value: Value,
    scope: &CallerScope,
) -> Result<Value, CommandError> {
    match operation {
        "swarmtank-ai-estimate" => estimate_op(value, scope),
        "swarmtank-ai-start" => start_op(value, scope),
        "swarmtank-ai-branch" => branch_op(value, scope),
        "swarmtank-ai-poll" => poll_op(value, scope),
        "swarmtank-ai-cancel" => cancel_op(value, scope),
        _ => Err(invalid("Use a supported SwarmTank AI request.")),
    }
}

// ---------------------------------------------------------------------------
// Dates (Howard Hinnant's civil calendar algorithms; no date crate)
// ---------------------------------------------------------------------------

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + (d as u64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}

fn parse_iso_date(text: &str) -> Option<(i64, u32, u32)> {
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i64 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Reject impossible dates (e.g. 2026-02-30) by round-tripping.
    let (ry, rm, rd) = civil_from_days(days_from_civil(y, m, d));
    if ry != y || rm != m || rd != d {
        return None;
    }
    Some((y, m, d))
}

fn format_iso_date(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn today_days() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs.div_euclid(86400)
}

/// The time plan for a run: the start date (today), the end date when
/// the user chose one, the step size in days, how many rounds the
/// horizon needs, and whether the horizon forced a coarser step than
/// requested. A longer horizon uses coarser steps so the run stays
/// within its round budget; the caller is told it happened.
struct TimePlan {
    start_days: i64,
    start_date: String,
    end_date: Option<String>,
    step_days: u32,
    step_count: u32,
    step_coarsened: bool,
}

fn resolve_time_plan(
    end_date: &Option<String>,
    step_days: Option<u32>,
    max_rounds: u32,
) -> Result<TimePlan, CommandError> {
    let step = step_days.unwrap_or(30);
    if ![1, 7, 30, 90].contains(&step) {
        return Err(invalid("Step size must be 1, 7, 30 or 90 days."));
    }
    let start_days = today_days();
    let start_date = format_iso_date(start_days);
    let Some(end_text) = end_date else {
        // No end date: one round per step, up to the round budget.
        return Ok(TimePlan {
            start_days,
            start_date,
            end_date: None,
            step_days: step,
            step_count: max_rounds,
            step_coarsened: false,
        });
    };
    let (y, m, d) = parse_iso_date(end_text)
        .ok_or_else(|| invalid("End date must be a valid YYYY-MM-DD date."))?;
    let end_days = days_from_civil(y, m, d);
    if end_days <= start_days {
        return Err(invalid("End date must be in the future."));
    }
    let horizon = (end_days - start_days) as u32;
    let mut effective = step;
    let mut count = horizon.div_ceil(effective);
    let mut coarsened = false;
    if count > max_rounds {
        // Coarsen the step so the horizon fits the round budget.
        effective = horizon.div_ceil(max_rounds);
        count = horizon.div_ceil(effective);
        coarsened = true;
    }
    Ok(TimePlan {
        start_days,
        start_date,
        end_date: Some(end_text.clone()),
        step_days: effective,
        step_count: count,
        step_coarsened: coarsened,
    })
}

/// The ISO date a round simulates. Round N lands N steps after the
/// start date, clamped to the end date for the last round.
fn round_date(plan: &TimePlan, round: u32) -> Option<String> {
    if plan.end_date.is_none() {
        return None;
    }
    let days = plan.start_days + (round as i64) * (plan.step_days as i64);
    let end_days = plan
        .end_date
        .as_ref()
        .and_then(|t| parse_iso_date(t))
        .map(|(y, m, d)| days_from_civil(y, m, d));
    let days = match end_days {
        Some(end) if days > end => end,
        _ => days,
    };
    Some(format_iso_date(days))
}

// ---------------------------------------------------------------------------
// Model resolution + Zero-Cost Lock (up front, before any call)
// ---------------------------------------------------------------------------

/// Resolve the model the run will use: the first configured chat model
/// in configuration order (configured local models are the default).
/// Returns (provider, model, locality) where locality is "local" or
/// "cloud", looked up in the provider catalog. An unknown provider is
/// treated as cloud (fail-closed for cost).
fn resolve_model(scope: &CallerScope) -> Result<(String, String, String), CommandError> {
    let configured = configured_chat_models(scope);
    let (provider, model) = configured.into_iter().next().ok_or_else(|| {
        invalid("No chat model is configured for this workspace; configure a provider first.")
    })?;
    // The catalog's locality enum is private; the Debug form is
    // "Local" / "Cloud" (the same rendering providers-catalog uses).
    let locality = anansi_providers::catalog::catalog()
        .iter()
        .find(|e| e.id == provider)
        .map(|e| {
            if format!("{:?}", e.locality) == "Local" {
                "local".to_string()
            } else {
                "cloud".to_string()
            }
        })
        .unwrap_or_else(|| "cloud".to_string());
    Ok((provider, model, locality))
}

fn resolve_caps(
    agents: Option<usize>,
    rounds: Option<u32>,
    max_output_tokens: Option<u32>,
    max_total_tokens: Option<u64>,
) -> Result<AiCaps, CommandError> {
    let max_agents = agents.unwrap_or(DEFAULT_AI_AGENTS);
    if !(MIN_AI_AGENTS..=MAX_AI_AGENTS).contains(&max_agents) {
        return Err(invalid("Agent count must be between 1 and 10."));
    }
    let max_rounds = rounds.unwrap_or(DEFAULT_AI_ROUNDS);
    if !(MIN_AI_ROUNDS..=MAX_AI_ROUNDS).contains(&max_rounds) {
        return Err(invalid("Round count must be between 1 and 10."));
    }
    let max_calls = (max_agents as u32)
        .saturating_mul(max_rounds)
        .min(MAX_AI_CALLS);
    let max_output = max_output_tokens.unwrap_or(DEFAULT_AI_OUTPUT_TOKENS);
    if !(MIN_AI_OUTPUT_TOKENS..=MAX_AI_OUTPUT_TOKENS).contains(&max_output) {
        return Err(invalid("Max output tokens must be between 64 and 2048."));
    }
    let max_total = max_total_tokens.unwrap_or(DEFAULT_AI_TOTAL_TOKENS);
    if !(MIN_AI_TOTAL_TOKENS..=MAX_AI_TOTAL_TOKENS).contains(&max_total) {
        return Err(invalid("Max total tokens must be between 1000 and 200000."));
    }
    Ok(AiCaps {
        max_agents,
        max_rounds,
        max_calls,
        max_output_tokens: max_output,
        max_total_tokens: max_total,
        memory_max_events: MEMORY_MAX_EVENTS,
        memory_max_chars: MEMORY_MAX_CHARS,
    })
}

fn validate_events(
    events: &Option<Vec<SwarmtankAiEventRequest>>,
    max_rounds: u32,
) -> Result<Vec<AiEvent>, CommandError> {
    let mut out = Vec::new();
    let Some(list) = events else {
        return Ok(out);
    };
    if list.len() > 32 {
        return Err(invalid("At most 32 scheduled events per run."));
    }
    for (i, e) in list.iter().enumerate() {
        if e.round < 1 || e.round > max_rounds {
            return Err(invalid(
                "A scheduled event names a round outside the run.",
            ));
        }
        if !["price_change", "competitor_launch", "scenario"].contains(&e.kind.as_str()) {
            return Err(invalid(
                "A scheduled event kind must be price_change, competitor_launch or scenario.",
            ));
        }
        if e.description.trim().is_empty() || e.description.chars().count() > 1000 {
            return Err(invalid(
                "A scheduled event description must be 1 to 1000 characters.",
            ));
        }
        out.push(AiEvent {
            id: format!("event-{}", i + 1),
            round: e.round,
            kind: e.kind.clone(),
            description: e.description.trim().to_string(),
            payload: e.payload.clone(),
        });
    }
    Ok(out)
}

/// Build the agents from the Quick personas (market-shaped when a market
/// is given). Each agent embodies one persona: its goal comes from the
/// persona's needs and priorities, its constraints from the purchase
/// constraints, budget, price sensitivity and required trust signals.
/// Geography shapes which buyers appear, never a personality trait.
fn build_agents(
    seed: &str,
    preset: u32,
    market: &Option<SwarmtankMarket>,
    count: usize,
) -> Result<Vec<AiAgent>, CommandError> {
    let (personas, _) = match market {
        Some(m) => generate_personas_for_market(seed, preset, m),
        None => generate_personas(seed, preset),
    }
    .map_err(|e| invalid(&e))?;
    let mut agents = Vec::new();
    for (i, persona) in personas.iter().take(count).enumerate() {
        let goal = format!(
            "As a {}, decide whether this product is worth buying for a {} company. Your needs: {}. Your priorities: {}.",
            persona.role,
            persona.company_size,
            persona.needs.join(", "),
            persona.priorities.join(", "),
        );
        let mut constraints: Vec<String> = persona.purchase_constraints.clone();
        constraints.push(format!(
            "Approximate budget: {:.0} (price sensitivity {}).",
            persona.approximate_budget, persona.price_sensitivity,
        ));
        constraints.push(format!(
            "Required trust signals: {}.",
            persona.required_trust_signals.join(", ")
        ));
        constraints.push(format!(
            "Expected implementation effort: {}.",
            persona.expected_implementation_effort
        ));
        agents.push(AiAgent {
            id: format!("agent-{}", i + 1),
            persona_id: persona.id.clone(),
            name: persona.name.clone(),
            role: persona.role.clone(),
            goal,
            constraints,
            language: persona.language.clone(),
        });
    }
    Ok(agents)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn content_hash(doc: &SwarmtankAiRunDoc) -> String {
    let mut copy = doc.clone();
    copy.content_hash = String::new();
    let body = serde_json::to_string(&copy).unwrap_or_default();
    crate::content::hash(body.as_bytes())
}

// ---------------------------------------------------------------------------
// Estimate (no run, no provider call)
// ---------------------------------------------------------------------------

fn estimate_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SwarmtankAiEstimateRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank AI request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    crate::swarmtank::validate_input(&req.input).map_err(|e| invalid(&e))?;
    if let Some(market) = &req.market {
        validate_market(market).map_err(|e| invalid(&e))?;
    }
    let seed = crate::swarmtank::resolve_seed(req.seed).map_err(|e| e)?;
    let caps = resolve_caps(req.agents, req.rounds, req.max_output_tokens, req.max_total_tokens)?;
    let plan = resolve_time_plan(&req.end_date, req.step_days, caps.max_rounds)?;
    let rounds_to_run = if req.end_date.is_some() {
        plan.step_count
    } else {
        caps.max_rounds
    };
    let (provider, model, locality) = resolve_model(scope)?;
    let agents = build_agents(&seed, 20, &req.market, caps.max_agents)?;
    // Measure a real first-round prompt for the input estimate, so the
    // number shown before Start is the prompt the model will actually
    // see, not a guessed constant.
    let events: Vec<AiEvent> = Vec::new();
    let prompt = build_prompt(&agents[0], &req.input, &req.market, 1, rounds_to_run, &events, &[]);
    let estimated_input_tokens = (prompt.chars().count() / 4) as u64;
    let estimated_calls = (caps.max_agents as u32).saturating_mul(rounds_to_run);
    let estimated_tokens =
        estimated_calls as u64 * (estimated_input_tokens + caps.max_output_tokens as u64);
    serde_json::to_value(SwarmtankAiEstimateResponse {
        contract_version: 1,
        provider,
        model,
        locality,
        estimated_calls,
        estimated_tokens,
        step_count: plan.step_count,
        effective_step_days: plan.step_days,
        step_coarsened: plan.step_coarsened,
    })
    .map_err(|_| crate::storage::unavailable())
}

// ---------------------------------------------------------------------------
// Start
// ---------------------------------------------------------------------------

fn start_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SwarmtankAiStartRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank AI request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let (workspace, _) =
        crate::swarmtank::require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    crate::swarmtank::validate_input(&req.input).map_err(|e| invalid(&e))?;
    if let Some(market) = &req.market {
        validate_market(market).map_err(|e| invalid(&e))?;
    }
    let seed = crate::swarmtank::resolve_seed(req.seed).map_err(|e| e)?;
    let caps = resolve_caps(req.agents, req.rounds, req.max_output_tokens, req.max_total_tokens)?;
    let plan = resolve_time_plan(&req.end_date, req.step_days, caps.max_rounds)?;
    let rounds_to_run = if req.end_date.is_some() {
        plan.step_count
    } else {
        caps.max_rounds
    };
    let events = validate_events(&req.scheduled_events, rounds_to_run)?;
    let (provider, model, locality) = resolve_model(scope)?;
    // Zero-Cost Lock, up front: a cloud model without an explicit
    // permit refuses before any call is made.
    let local_only = req.local_only.unwrap_or_else(|| locality == "local");
    if local_only && locality == "cloud" {
        return Err(invalid(
            "Local-only was requested but the configured model is not local.",
        ));
    }
    if locality == "cloud" && req.permit_id.is_none() {
        return Err(invalid(
            "This model requires explicit authorization. Authorize it first (providers-authorize), then start the run with the permit.",
        ));
    }
    let agents = build_agents(&seed, 20, &req.market, caps.max_agents)?;
    let prompt = build_prompt(&agents[0], &req.input, &req.market, 1, rounds_to_run, &events, &[]);
    let estimated_input_tokens = (prompt.chars().count() / 4) as u64;
    let estimated_calls = (caps.max_agents as u32).saturating_mul(rounds_to_run);
    let estimated_tokens =
        estimated_calls as u64 * (estimated_input_tokens + caps.max_output_tokens as u64);
    let step_count = plan.step_count;
    let effective_step_days = plan.step_days;
    let step_coarsened = plan.step_coarsened;
    let run_id = new_run_id();
    collect_ai_evictions(Some(&scope.workspace));
    let cancel = Arc::new(AtomicBool::new(false));
    let run = AiRun {
        session: scope.session.clone(),
        status: "running".to_string(),
        agents: agents.clone(),
        rounds: Vec::new(),
        calls_made: 0,
        tokens_used: 0,
        provider: provider.clone(),
        model: model.clone(),
        locality: locality.clone(),
        cancel: cancel.clone(),
        artifact_id: None,
        artifact_version: None,
        error: None,
        created: Instant::now(),
    };
    with_ai_runs(|map| {
        map.insert(ai_run_key(scope, &run_id), run);
    });
    // The thread owns the run's config; the registry is the live view
    // the UI polls. The thread updates the registry after every agent
    // call and saves the artifact at a terminal state.
    let thread_scope = scope.clone();
    let thread_run_id = run_id.clone();
    let thread_config = AiRunConfig {
        workspace_id: workspace,
        project_id: req.project_id.clone(),
        input: req.input.clone(),
        market: req.market.clone(),
        seed,
        agents,
        events,
        caps: caps.clone(),
        provider: provider.clone(),
        model: model.clone(),
        locality: locality.clone(),
        local_only,
        permit_id: req.permit_id.clone(),
        cancel,
        created_ms: now_ms(),
        plan,
        rounds: rounds_to_run,
        resume_rounds: Vec::new(),
        parent_run_id: None,
        parent_artifact_id: None,
        parent_round: None,
        branch_changes: None,
    };
    std::thread::spawn(move || {
        run_ai_simulation(thread_run_id, thread_scope, thread_config);
    });
    serde_json::to_value(SwarmtankAiStartResponse {
        contract_version: 1,
        run_id,
        provider,
        model,
        locality,
        estimated_calls,
        estimated_tokens,
        step_count,
        effective_step_days,
        step_coarsened,
    })
    .map_err(|_| crate::storage::unavailable())
}


// ---------------------------------------------------------------------------
// Branch: continue a saved run from one of its rounds with a changed
// price and/or new scheduled events. The branch is stored as a child
// of the original run (branch id + parent round); its rounds are the
// parent's rounds up to the branch point, then new rounds run with the
// changed price/events. The comparison shows the original timeline
// beside the branch; both are labelled conditional simulation results.
// ---------------------------------------------------------------------------

fn branch_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SwarmtankAiBranchRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank AI request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let (workspace, storage) =
        crate::swarmtank::require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    // Read the parent run artifact (latest version) and parse it.
    let details = storage
        .artifact_read(&workspace, &req.parent_artifact_id, None)
        .map_err(|_| invalid("The parent AI run was not found."))?;
    let parent: SwarmtankAiRunDoc = serde_json::from_str(&details.body)
        .map_err(|_| invalid("The parent AI run body is not readable."))?;
    if parent.format != "anansi-swarmtank-ai-run" || parent.version != 1 {
        return Err(invalid("The parent artifact is not a SwarmTank AI run."));
    }
    if details.artifact.artifact_type != "swarmtank-ai-run" {
        return Err(invalid("The parent artifact is not a SwarmTank AI run."));
    }
    if req.parent_round < 1 || req.parent_round as usize > parent.rounds.len() {
        return Err(invalid(
            "The parent round must be one of the parent run's completed rounds.",
        ));
    }
    // The branch input: the parent's input, with the price changed when
    // the user asked for one.
    let mut input = parent.input.clone();
    if let Some(price) = req.price_amount {
        if !(0.0..=1_000_000.0).contains(&price) {
            return Err(invalid("The branch price must be between 0 and 1000000."));
        }
        input.price_amount = Some(price);
        input.pricing = format!("${price} per month");
    }
    // New events must fall due after the branch point.
    let total_rounds = req.rounds.unwrap_or(parent.caps.max_rounds);
    if total_rounds <= req.parent_round {
        return Err(invalid(
            "The branch must continue past the parent round: choose more rounds than the branch point.",
        ));
    }
    if total_rounds > MAX_AI_ROUNDS {
        return Err(invalid("Round count must be between 1 and 10."));
    }
    let mut events = parent.scheduled_events.clone();
    let mut new_event_ids: Vec<String> = Vec::new();
    if let Some(new) = &req.new_events {
        for (i, e) in new.iter().enumerate() {
            if e.round <= req.parent_round || e.round > total_rounds {
                return Err(invalid(
                    "A new branch event must fall due after the branch point and within the run.",
                ));
            }
            if !["price_change", "competitor_launch", "scenario"].contains(&e.kind.as_str()) {
                return Err(invalid(
                    "A scheduled event kind must be price_change, competitor_launch or scenario.",
                ));
            }
            if e.description.trim().is_empty() || e.description.chars().count() > 1000 {
                return Err(invalid(
                    "A scheduled event description must be 1 to 1000 characters.",
                ));
            }
            let id = format!("event-{}", events.len() + i + 1);
            new_event_ids.push(id.clone());
            events.push(AiEvent {
                id,
                round: e.round,
                kind: e.kind.clone(),
                description: e.description.trim().to_string(),
                payload: e.payload.clone(),
            });
        }
    }
    // The branch reuses the parent's time plan and agents; only the
    // price and the events change.
    let (sy, sm, sd) = parse_iso_date(&parent.start_date)
        .ok_or_else(|| invalid("The parent run has an unreadable start date."))?;
    let plan = TimePlan {
        start_days: days_from_civil(sy, sm, sd),
        start_date: parent.start_date.clone(),
        end_date: parent.end_date.clone(),
        step_days: parent.step_days,
        step_count: total_rounds,
        step_coarsened: parent.step_coarsened,
    };
    let caps = AiCaps {
        max_agents: parent.caps.max_agents,
        max_rounds: total_rounds,
        max_calls: (parent.caps.max_agents as u32).saturating_mul(total_rounds),
        max_output_tokens: req
            .max_output_tokens
            .unwrap_or(parent.caps.max_output_tokens),
        max_total_tokens: req
            .max_total_tokens
            .unwrap_or(parent.caps.max_total_tokens),
        memory_max_events: parent.caps.memory_max_events,
        memory_max_chars: parent.caps.memory_max_chars,
    };
    if !(MIN_AI_OUTPUT_TOKENS..=MAX_AI_OUTPUT_TOKENS).contains(&caps.max_output_tokens) {
        return Err(invalid("Max output tokens must be between 64 and 2048."));
    }
    if !(MIN_AI_TOTAL_TOKENS..=MAX_AI_TOTAL_TOKENS).contains(&caps.max_total_tokens) {
        return Err(invalid("Max total tokens must be between 1000 and 200000."));
    }
    let (provider, model, locality) = resolve_model(scope)?;
    let local_only = req.local_only.unwrap_or_else(|| locality == "local");
    if local_only && locality == "cloud" {
        return Err(invalid(
            "Local-only was requested but the configured model is not local.",
        ));
    }
    if locality == "cloud" && req.permit_id.is_none() {
        return Err(invalid(
            "This model requires explicit authorization. Authorize it first (providers-authorize), then start the run with the permit.",
        ));
    }
    let resume_rounds: Vec<AiRound> = parent
        .rounds
        .iter()
        .take(req.parent_round as usize)
        .cloned()
        .collect();
    let new_rounds = total_rounds - req.parent_round;
    let estimated_calls = (caps.max_agents as u32).saturating_mul(new_rounds);
    // Measure a real prompt for the input estimate (first new round).
    let first_new_round = req.parent_round + 1;
    let events_due: Vec<AiEvent> = events
        .iter()
        .filter(|e| e.round == first_new_round)
        .cloned()
        .collect();
    let prior: Vec<String> = resume_rounds
        .iter()
        .flat_map(|r| r.actions.iter())
        .rev()
        .take(caps.memory_max_events)
        .map(|a| {
            format!(
                "round-{}-{}: {}",
                a.round,
                a.agent_id,
                a.rationale.clone().unwrap_or_default()
            )
        })
        .collect();
    let prompt = build_prompt(
        &parent.agents[0],
        &input,
        &parent.market,
        first_new_round,
        total_rounds,
        &events_due,
        &prior,
    );
    let estimated_input_tokens = (prompt.chars().count() / 4) as u64;
    let estimated_tokens =
        estimated_calls as u64 * (estimated_input_tokens + caps.max_output_tokens as u64);
    let step_count = plan.step_count;
    let effective_step_days = plan.step_days;
    let step_coarsened = plan.step_coarsened;
    let run_id = new_run_id();
    collect_ai_evictions(Some(&scope.workspace));
    let cancel = Arc::new(AtomicBool::new(false));
    let run = AiRun {
        session: scope.session.clone(),
        status: "running".to_string(),
        agents: parent.agents.clone(),
        rounds: resume_rounds.clone(),
        calls_made: 0,
        tokens_used: 0,
        provider: provider.clone(),
        model: model.clone(),
        locality: locality.clone(),
        cancel: cancel.clone(),
        artifact_id: None,
        artifact_version: None,
        error: None,
        created: Instant::now(),
    };
    with_ai_runs(|map| {
        map.insert(ai_run_key(scope, &run_id), run);
    });
    let thread_scope = scope.clone();
    let thread_run_id = run_id.clone();
    let thread_config = AiRunConfig {
        workspace_id: workspace,
        project_id: req.project_id.clone(),
        input,
        market: parent.market.clone(),
        seed: parent.seed.clone(),
        agents: parent.agents.clone(),
        events,
        caps,
        provider: provider.clone(),
        model: model.clone(),
        locality: locality.clone(),
        local_only,
        permit_id: req.permit_id.clone(),
        cancel,
        created_ms: now_ms(),
        plan,
        rounds: total_rounds,
        resume_rounds,
        parent_run_id: Some(parent.run_id.clone()),
        parent_artifact_id: Some(req.parent_artifact_id.clone()),
        parent_round: Some(req.parent_round),
        branch_changes: Some(BranchChanges {
            price_amount: req.price_amount,
            new_event_ids,
        }),
    };
    std::thread::spawn(move || {
        run_ai_simulation(thread_run_id, thread_scope, thread_config);
    });
    serde_json::to_value(SwarmtankAiBranchResponse {
        contract_version: 1,
        run_id,
        parent_run_id: parent.run_id,
        parent_round: req.parent_round,
        provider,
        model,
        locality,
        estimated_calls,
        estimated_tokens,
        step_count,
        effective_step_days,
        step_coarsened,
    })
    .map_err(|_| crate::storage::unavailable())
}

// ---------------------------------------------------------------------------
// Poll / cancel
// ---------------------------------------------------------------------------

fn poll_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SwarmtankAiPollRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank AI request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let key = ai_run_key(scope, &req.run_id);
    let snapshot = with_ai_runs(|map| {
        map.get(&key)
            .filter(|r| r.session == scope.session)
            .map(|r| {
                (
                    r.status.clone(),
                    r.agents.clone(),
                    r.rounds.clone(),
                    r.calls_made,
                    r.tokens_used,
                    r.provider.clone(),
                    r.model.clone(),
                    r.locality.clone(),
                    r.artifact_id.clone(),
                    r.artifact_version,
                    r.error.clone(),
                )
            })
    });
    let Some((status, agents, rounds, calls_made, tokens_used, provider, model, locality, artifact_id, artifact_version, error)) =
        snapshot
    else {
        return Err(not_found());
    };
    serde_json::to_value(SwarmtankAiPollResponse {
        contract_version: 1,
        run_id: req.run_id,
        status,
        agents,
        rounds,
        calls_made,
        tokens_used,
        provider,
        model,
        locality,
        artifact_id,
        artifact_version,
        error,
    })
    .map_err(|_| crate::storage::unavailable())
}

fn cancel_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SwarmtankAiCancelRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank AI request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let key = ai_run_key(scope, &req.run_id);
    let status = with_ai_runs(|map| {
        let Some(run) = map.get_mut(&key).filter(|r| r.session == scope.session) else {
            return None;
        };
        if run.status == "running" {
            run.cancel.store(true, Ordering::SeqCst);
            run.status = "cancelling".to_string();
        }
        Some(run.status.clone())
    });
    let Some(status) = status else {
        return Err(not_found());
    };
    serde_json::to_value(SwarmtankAiCancelResponse {
        contract_version: 1,
        run_id: req.run_id,
        status,
    })
    .map_err(|_| crate::storage::unavailable())
}

// ---------------------------------------------------------------------------
// Prompt + action validation
// ---------------------------------------------------------------------------

struct AiRunConfig {
    workspace_id: String,
    project_id: String,
    input: SwarmtankInput,
    market: Option<SwarmtankMarket>,
    seed: String,
    agents: Vec<AiAgent>,
    events: Vec<AiEvent>,
    caps: AiCaps,
    provider: String,
    model: String,
    locality: String,
    local_only: bool,
    permit_id: Option<String>,
    cancel: Arc<AtomicBool>,
    created_ms: u64,
    plan: TimePlan,
    /// Rounds this run will produce in total (its own new rounds plus
    /// any resumed rounds from a parent branch).
    rounds: u32,
    /// Rounds copied from a parent run (a branch continues from them).
    resume_rounds: Vec<AiRound>,
    parent_run_id: Option<String>,
    parent_artifact_id: Option<String>,
    parent_round: Option<u32>,
    branch_changes: Option<BranchChanges>,
}

/// Build the prompt for one agent in one round. The agent sees its own
/// goal and constraints, the product and market, the events due this
/// round, and a bounded window of prior interactions — never the full
/// history and never another agent's private goals.
fn build_prompt(
    agent: &AiAgent,
    input: &SwarmtankInput,
    market: &Option<SwarmtankMarket>,
    round: u32,
    total_rounds: u32,
    events_due: &[AiEvent],
    prior: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("You are a simulated buyer persona in a SwarmTank scenario-planning simulation. You are NOT a real customer; your answer is a conditional simulation result, not measured demand.\n\n");
    out.push_str(&format!(
        "Your persona:\n- Name: {}\n- Role: {}\n- Goal: {}\n- Constraints: {}\n",
        agent.name,
        agent.role,
        agent.goal,
        agent.constraints.join("; "),
    ));
    if let Some(language) = &agent.language {
        out.push_str(&format!("- Language: {language}\n"));
    }
    out.push_str(&format!(
        "\nProduct under review:\n- Description: {}\n- Price: {}",
        input.description, input.pricing,
    ));
    if let Some(amount) = input.price_amount {
        out.push_str(&format!(" (amount {amount})"));
    }
    out.push('\n');
    if !input.segment.is_empty() {
        out.push_str(&format!("- Segment: {}\n", input.segment));
    }
    if !input.industry.is_empty() {
        out.push_str(&format!("- Industry: {}\n", input.industry));
    }
    if !input.features.is_empty() {
        out.push_str(&format!("- Features: {}\n", input.features.join(", ")));
    }
    if !input.value_proposition.is_empty() {
        out.push_str(&format!("- Value proposition: {}\n", input.value_proposition));
    }
    if let Some(market) = market {
        out.push_str(&format!(
            "\nMarket: {} ({}){} — this run is labelled {}.\n",
            market.territory_name.as_deref().unwrap_or(&market.territory_code.clone().unwrap_or_default()),
            market.territory_code.as_deref().unwrap_or(""),
            market
                .profile_id
                .as_ref()
                .map(|id| format!(" — researched profile {id} v{}", market.profile_version.unwrap_or(0)))
                .unwrap_or_else(|| " — Custom assumptions".to_string()),
            if market.custom { "Custom assumptions" } else { "a researched profile" },
        ));
    }
    out.push_str(&format!(
        "\nRound {round} of {total_rounds}. Scenario events due this round:\n"
    ));
    if events_due.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for event in events_due {
            out.push_str(&format!("- [{}] {}: {}\n", event.id, event.kind, event.description));
        }
    }
    out.push_str("\nWhat you observed before (bounded memory — selected prior interactions, not the full history):\n");
    if prior.is_empty() {
        out.push_str("- (nothing yet — this is the first round)\n");
    } else {
        for item in prior {
            out.push_str(&format!("- {item}\n"));
        }
    }
    out.push_str("\nReply with ONLY a JSON object, no other text:\n{\"action\": \"recommend\" | \"defer\" | \"request_evidence\" | \"reject\", \"rationale\": \"one or two sentences\", \"evidenceRef\": \"an event id or interaction id you cite, or omit\", \"priorInteractionRef\": \"an interaction id, or omit\"}\n");
    out
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ModelAction {
    action: String,
    rationale: String,
    #[serde(default)]
    evidence_ref: Option<String>,
    #[serde(default)]
    prior_interaction_ref: Option<String>,
}

/// Validate a model output. Returns the validated action, or an error
/// describing why it is malformed. A malformed output is never retried
/// into a different action.
fn validate_model_output(
    raw: &str,
    known_refs: &[String],
) -> Result<ModelAction, String> {
    let parsed: ModelAction = serde_json::from_str(raw).map_err(|e| {
        format!("Model output is not valid JSON: {e}")
    })?;
    if !AI_ACTIONS.contains(&parsed.action.as_str()) {
        return Err(format!(
            "Model action '{}' is not one of request_evidence, defer, recommend, reject.",
            parsed.action
        ));
    }
    if parsed.rationale.trim().is_empty() || parsed.rationale.chars().count() > 2000 {
        return Err("Model rationale must be 1 to 2000 characters.".to_string());
    }
    for (name, value) in [
        ("evidenceRef", &parsed.evidence_ref),
        ("priorInteractionRef", &parsed.prior_interaction_ref),
    ] {
        if let Some(value) = value {
            if !known_refs.iter().any(|r| r == value) {
                return Err(format!(
                    "Model {name} '{value}' does not name an event or interaction in this run."
                ));
            }
        }
    }
    Ok(parsed)
}

fn truncate_raw(raw: &str) -> String {
    let mut out: String = raw.chars().take(MAX_RAW_OUTPUT_CHARS).collect();
    if raw.chars().count() > MAX_RAW_OUTPUT_CHARS {
        out.push_str("…");
    }
    out
}

/// The bounded memory window for one agent in one round: the most
/// recent prior actions (as short interaction lines) plus the events
/// due this round, capped by count and by characters. The snapshot
/// records exactly which ids the agent observed.
fn memory_window(
    prior_actions: &[AiAction],
    events_due: &[AiEvent],
    caps: &AiCaps,
) -> (Vec<String>, AiMemoryWindow) {
    let mut observed_event_ids: Vec<String> = events_due.iter().map(|e| e.id.clone()).collect();
    let mut observed_interaction_ids: Vec<String> = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    // Most recent prior actions first, up to the event cap.
    let mut taken = 0usize;
    for action in prior_actions.iter().rev() {
        if taken >= caps.memory_max_events {
            break;
        }
        let id = format!("round-{}-{}", action.round, action.agent_id);
        let line = match (&action.action, &action.rationale) {
            (Some(a), Some(r)) => format!("{id}: {} — {}", a, r),
            _ => format!("{id}: (no valid action recorded)"),
        };
        lines.push(line);
        observed_interaction_ids.push(id);
        taken += 1;
    }
    lines.reverse();
    // Cap the whole window by characters (events are already in the
    // prompt separately; the prior-interaction text is what is capped).
    let mut chars = 0usize;
    let mut kept: Vec<String> = Vec::new();
    for line in lines {
        let len = line.chars().count();
        if chars + len > caps.memory_max_chars && !kept.is_empty() {
            break;
        }
        chars += len;
        kept.push(line);
    }
    observed_event_ids.truncate(caps.memory_max_events);
    (
        kept,
        AiMemoryWindow {
            max_events: caps.memory_max_events,
            max_chars: caps.memory_max_chars,
            observed_event_ids,
            observed_interaction_ids,
        },
    )
}

// ---------------------------------------------------------------------------
// The runner (background thread)
// ---------------------------------------------------------------------------

fn run_ai_simulation(run_id: String, scope: CallerScope, config: AiRunConfig) {
    let key = ai_run_key(&scope, &run_id);
    // A branch resumes from its parent's rounds: they are the run's
    // history, and the agents' memory is seeded from them.
    let mut rounds: Vec<AiRound> = config.resume_rounds.clone();
    let mut calls_made: u32 = 0;
    let mut tokens_used: u64 = 0;
    let mut status = "complete".to_string();
    let error: Option<String> = None;
    let mut prior_actions: Vec<AiAction> = config
        .resume_rounds
        .iter()
        .flat_map(|r| r.actions.clone())
        .collect();
    let start_round = (config.resume_rounds.len() as u32) + 1;

    'rounds: for round in start_round..=config.rounds {
        if config.cancel.load(Ordering::SeqCst) {
            status = "cancelled".to_string();
            break 'rounds;
        }
        let events_due: Vec<AiEvent> = config
            .events
            .iter()
            .filter(|e| e.round == round)
            .cloned()
            .collect();
        let event_ids: Vec<String> = events_due.iter().map(|e| e.id.clone()).collect();
        let mut actions: Vec<AiAction> = Vec::new();
        for agent in &config.agents {
            if config.cancel.load(Ordering::SeqCst) {
                status = "cancelled".to_string();
                break 'rounds;
            }
            // Budget, checked before every call: exhaustion ends the
            // run in its own terminal state, not an error.
            if calls_made >= config.caps.max_calls {
                status = "budget-exhausted".to_string();
                break 'rounds;
            }
            let (prior_lines, memory) =
                memory_window(&prior_actions, &events_due, &config.caps);
            let prompt = build_prompt(
                agent,
                &config.input,
                &config.market,
                round,
                config.rounds,
                &events_due,
                &prior_lines,
            );
            let estimated_input = (prompt.chars().count() / 4) as u64;
            let estimated_call =
                estimated_input + config.caps.max_output_tokens as u64;
            if tokens_used.saturating_add(estimated_call) > config.caps.max_total_tokens {
                status = "budget-exhausted".to_string();
                break 'rounds;
            }
            let action = dispatch_agent_call(
                &scope,
                &config,
                agent,
                round,
                &prompt,
                &memory,
                &known_refs(&config.events, &prior_actions),
            );
            match &action.usage {
                Some(usage) => {
                    tokens_used = tokens_used.saturating_add(
                        usage.total_tokens.unwrap_or_else(|| {
                            usage
                                .prompt_tokens
                                .unwrap_or(0)
                                .saturating_add(usage.completion_tokens.unwrap_or(0))
                        }),
                    );
                }
                None => {
                    tokens_used = tokens_used.saturating_add(estimated_call);
                }
            }
            calls_made += 1;
            if action.status == "failed" && action.failure.as_deref() == Some("budget") {
                status = "budget-exhausted".to_string();
            }
            prior_actions.push(action.clone());
            actions.push(action);
            // Live progress: the UI polls the registry after every call.
            let round_snapshot = AiRound {
                round,
                actions: actions.clone(),
                events_due: event_ids.clone(),
                date: round_date(&config.plan, round),
            };
            let mut all_rounds = rounds.clone();
            all_rounds.push(round_snapshot);
            let calls = calls_made;
            let tokens = tokens_used;
            with_ai_runs(|map| {
                if let Some(run) = map.get_mut(&key) {
                    run.rounds = all_rounds;
                    run.calls_made = calls;
                    run.tokens_used = tokens;
                }
            });
            if status == "budget-exhausted" {
                break 'rounds;
            }
        }
        rounds.push(AiRound {
            round,
            actions,
            events_due: event_ids,
            date: round_date(&config.plan, round),
        });
    }

    if status == "complete" && rounds.len() as u32 != config.rounds {
        // The loop only ends early on cancel / budget-exhaustion;
        // a complete run has every round.
        status = "complete".to_string();
    }

    // Save the artifact at a terminal state. A run that failed before
    // any round was recorded is still saved when it has at least one
    // round; a run with no rounds at all is a start failure, and the
    // registry (with its error) is the record.
    let (artifact_id, artifact_version) = save_ai_run(
        &scope,
        &run_id,
        &config,
        &rounds,
        &status,
        calls_made,
        tokens_used,
        error.clone(),
    );
    with_ai_runs(|map| {
        if let Some(run) = map.get_mut(&key) {
            run.status = status.clone();
            run.rounds = rounds;
            run.calls_made = calls_made;
            run.tokens_used = tokens_used;
            run.artifact_id = artifact_id;
            run.artifact_version = artifact_version;
            run.error = error;
        }
    });
}

fn known_refs(events: &[AiEvent], prior_actions: &[AiAction]) -> Vec<String> {
    let mut refs: Vec<String> = events.iter().map(|e| e.id.clone()).collect();
    for action in prior_actions {
        refs.push(format!("round-{}-{}", action.round, action.agent_id));
    }
    refs
}

/// One provider call for one agent in one round. Dispatch errors and
/// malformed outputs are recorded as failed actions, never retried.
fn dispatch_agent_call(
    scope: &CallerScope,
    config: &AiRunConfig,
    agent: &AiAgent,
    round: u32,
    prompt: &str,
    memory: &AiMemoryWindow,
    known_refs: &[String],
) -> AiAction {
    let model_label = format!("{}/{}", config.provider, config.model);
    let dispatch = json!({
        "contractVersion": 1,
        "workspaceId": config.workspace_id,
        "request": {
            "model": config.model.clone(),
            "messages": [{ "role": "user", "segments": [{ "text": prompt, "privacy": "standard" }] }],
            "maxOutputTokens": config.caps.max_output_tokens,
            "localOnly": config.local_only,
            "explicitCloudAuthorization": config.permit_id.is_some(),
        },
        "mode": "auto",
        "zeroCostLock": true,
        "chosen": [config.provider.clone(), config.model.clone()],
        "permitId": config.permit_id.clone(),
    });
    let result = providers_command("providers-dispatch", dispatch, scope);
    let response = match result {
        Ok(value) => value,
        Err(e) => {
            return AiAction {
                agent_id: agent.id.clone(),
                round,
                action: None,
                rationale: None,
                evidence_ref: None,
                prior_interaction_ref: None,
                status: "failed".to_string(),
                failure: Some(format!("Provider call failed: {}", e.message)),
                raw_output: None,
                model: Some(model_label),
                usage: None,
                memory: memory.clone(),
            };
        }
    };
    // ProviderDispatchResponse carries Serialize only, so read the
    // fields from the Value instead of deserializing the struct.
    let raw = match response
        .get("response")
        .and_then(|r| r.get("text"))
        .and_then(|t| t.as_str())
    {
        Some(text) => text.to_string(),
        None => {
            return AiAction {
                agent_id: agent.id.clone(),
                round,
                action: None,
                rationale: None,
                evidence_ref: None,
                prior_interaction_ref: None,
                status: "failed".to_string(),
                failure: Some(
                    "Provider returned a response the core could not read.".to_string(),
                ),
                raw_output: None,
                model: Some(model_label),
                usage: None,
                memory: memory.clone(),
            };
        }
    };
    let usage = response
        .get("response")
        .and_then(|r| r.get("usage"))
        .map(|u| AiUsage {
            prompt_tokens: u.get("inputTokens").and_then(|v| v.as_u64()),
            completion_tokens: u.get("outputTokens").and_then(|v| v.as_u64()),
            total_tokens: None,
        })
        .map(|mut u| {
            if let (Some(p), Some(c)) = (u.prompt_tokens, u.completion_tokens) {
                u.total_tokens = Some(p.saturating_add(c));
            }
            u
        });
    match validate_model_output(&raw, known_refs) {
        Ok(valid) => AiAction {
            agent_id: agent.id.clone(),
            round,
            action: Some(valid.action),
            rationale: Some(valid.rationale),
            evidence_ref: valid.evidence_ref,
            prior_interaction_ref: valid.prior_interaction_ref,
            status: "ok".to_string(),
            failure: None,
            raw_output: None,
            model: Some(model_label),
            usage,
            memory: memory.clone(),
        },
        Err(why) => AiAction {
            agent_id: agent.id.clone(),
            round,
            action: None,
            rationale: None,
            evidence_ref: None,
            prior_interaction_ref: None,
            status: "failed".to_string(),
            failure: Some(format!("Malformed model output: {why}")),
            raw_output: Some(truncate_raw(&raw)),
            model: Some(model_label),
            usage,
            memory: memory.clone(),
        },
    }
}

fn save_ai_run(
    scope: &CallerScope,
    run_id: &str,
    config: &AiRunConfig,
    rounds: &[AiRound],
    status: &str,
    calls_made: u32,
    tokens_used: u64,
    _error: Option<String>,
) -> (Option<String>, Option<u32>) {
    let mut doc = SwarmtankAiRunDoc {
        format: "anansi-swarmtank-ai-run".to_string(),
        version: 1,
        run_id: run_id.to_string(),
        workspace_id: config.workspace_id.clone(),
        project_id: config.project_id.clone(),
        input: config.input.clone(),
        market: config.market.clone(),
        seed: config.seed.clone(),
        agents: config.agents.clone(),
        scheduled_events: config.events.clone(),
        rounds: rounds.to_vec(),
        status: status.to_string(),
        model_provider: config.provider.clone(),
        model: config.model.clone(),
        locality: config.locality.clone(),
        caps: config.caps.clone(),
        calls_made,
        tokens_used,
        assumptions: vec![
            "Outcomes are conditional simulation results from synthetic agents, not measured demand.".to_string(),
            "Confidence is an internal model indicator, not a calibrated probability.".to_string(),
        ],
        content_hash: String::new(),
        created_ms: config.created_ms,
        start_date: config.plan.start_date.clone(),
        end_date: config.plan.end_date.clone(),
        step_days: config.plan.step_days,
        step_coarsened: config.plan.step_coarsened,
        parent_run_id: config.parent_run_id.clone(),
        parent_artifact_id: config.parent_artifact_id.clone(),
        parent_round: config.parent_round,
        branch_changes: config.branch_changes.clone(),
    };
    doc.content_hash = content_hash(&doc);
    let body = match serde_json::to_string(&doc) {
        Ok(b) => b,
        Err(_) => return (None, None),
    };
    let storage = match crate::swarmtank::storage(scope) {
        Ok(s) => s,
        Err(_) => return (None, None),
    };
    let title = {
        let excerpt: String = config
            .input
            .description
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(120)
            .collect();
        match (&config.parent_run_id, config.parent_round) {
            (Some(parent), Some(round)) => format!(
                "SwarmTank AI run ({status}, branch of {parent} from round {round}): {excerpt}"
            ),
            _ => format!("SwarmTank AI run ({status}): {excerpt}"),
        }
    };
    let details = storage.artifact_register_generated(
        &crate::artifacts::ArtifactRegisterRequest {
            contract_version: 1,
            workspace_id: config.workspace_id.clone(),
            project_id: config.project_id.clone(),
            artifact_type: "swarmtank-ai-run".to_string(),
            title,
            body,
            source_references: vec![],
            privacy: "private".to_string(),
        },
        json!({
            "kind": "ai-simulation",
            "generator": "anansi-swarmtank-ai",
            "seed": config.seed,
            "status": status,
        }),
        "validated",
    );
    match details {
        Ok(d) => (Some(d.artifact.artifact_id), Some(d.artifact.version)),
        Err(_) => (None, None),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_reject_out_of_bounds() {
        assert!(resolve_caps(Some(0), None, None, None).is_err());
        assert!(resolve_caps(Some(11), None, None, None).is_err());
        assert!(resolve_caps(None, Some(0), None, None).is_err());
        assert!(resolve_caps(None, Some(11), None, None).is_err());
        assert!(resolve_caps(None, None, Some(10), None).is_err());
        assert!(resolve_caps(None, None, Some(4096), None).is_err());
        assert!(resolve_caps(None, None, None, Some(10)).is_err());
        let caps = resolve_caps(None, None, None, None).unwrap();
        assert_eq!(caps.max_agents, DEFAULT_AI_AGENTS);
        assert_eq!(caps.max_rounds, DEFAULT_AI_ROUNDS);
        assert_eq!(caps.max_calls, 6);
    }

    #[test]
    fn model_output_validation_accepts_a_valid_action() {
        let refs = vec!["event-1".to_string(), "round-1-agent-1".to_string()];
        let raw = r#"{"action":"recommend","rationale":"The price fits and the value is clear.","evidenceRef":"event-1"}"#;
        let valid = validate_model_output(raw, &refs).unwrap();
        assert_eq!(valid.action, "recommend");
        assert_eq!(valid.evidence_ref.as_deref(), Some("event-1"));
    }

    #[test]
    fn model_output_validation_rejects_unknown_actions_and_refs() {
        let refs = vec!["event-1".to_string()];
        assert!(validate_model_output(
            r#"{"action":"buy","rationale":"x"}"#,
            &refs
        )
        .is_err());
        assert!(validate_model_output(
            r#"{"action":"recommend","rationale":"x","evidenceRef":"event-99"}"#,
            &refs
        )
        .is_err());
        // Not JSON at all.
        assert!(validate_model_output("not json", &refs).is_err());
        // Empty rationale.
        assert!(validate_model_output(
            r#"{"action":"defer","rationale":"  "}"#,
            &refs
        )
        .is_err());
        // Unknown fields are refused (strict schema).
        assert!(validate_model_output(
            r#"{"action":"defer","rationale":"x","surprise":true}"#,
            &refs
        )
        .is_err());
    }

    #[test]
    fn memory_window_is_bounded_by_count_and_chars() {
        let caps = AiCaps {
            max_agents: 3,
            max_rounds: 2,
            max_calls: 6,
            max_output_tokens: 512,
            max_total_tokens: 20_000,
            memory_max_events: 2,
            memory_max_chars: 40,
        };
        let mut prior: Vec<AiAction> = Vec::new();
        for i in 1..=4 {
            prior.push(AiAction {
                agent_id: format!("agent-{i}"),
                round: 1,
                action: Some("recommend".to_string()),
                rationale: Some(format!("Reason number {i} is quite long indeed.")),
                evidence_ref: None,
                prior_interaction_ref: None,
                status: "ok".to_string(),
                failure: None,
                raw_output: None,
                model: None,
                usage: None,
                memory: AiMemoryWindow {
                    max_events: 5,
                    max_chars: 4000,
                    observed_event_ids: vec![],
                    observed_interaction_ids: vec![],
                },
            });
        }
        let (lines, window) = memory_window(&prior, &[], &caps);
        // At most 2 prior interactions (the most recent), and the
        // character cap keeps at most one long line.
        assert!(window.observed_interaction_ids.len() <= 2);
        assert!(lines.len() <= 2);
        assert!(window.max_events == 2 && window.max_chars == 40);
    }

    #[test]
    fn iso_dates_round_trip_including_leap_years() {
        // 2028 is a leap year: Feb 29 exists.
        assert_eq!(parse_iso_date("2028-02-29"), Some((2028, 2, 29)));
        assert_eq!(parse_iso_date("2026-02-29"), None);
        assert_eq!(parse_iso_date("2026-13-01"), None);
        assert_eq!(parse_iso_date("not-a-date"), None);
        let days = days_from_civil(2026, 11, 6);
        assert_eq!(civil_from_days(days), (2026, 11, 6));
        assert_eq!(format_iso_date(days), "2026-11-06");
        assert_eq!(format_iso_date(days + 30), "2026-12-06");
        assert_eq!(format_iso_date(days_from_civil(2026, 12, 31) + 1), "2027-01-01");
    }

    #[test]
    fn time_plan_coarsens_a_long_horizon_to_fit_the_round_budget() {
        // No end date: the plan spans the round budget.
        let plan = resolve_time_plan(&None, Some(7), 2).unwrap();
        assert_eq!(plan.step_count, 2);
        assert_eq!(plan.step_days, 7);
        assert!(!plan.step_coarsened);
        // A 60-day horizon at 7-day steps needs 9 rounds; the budget
        // is 2, so the step coarsens to 30 days and 2 steps.
        let start = format_iso_date(today_days());
        let end = format_iso_date(today_days() + 60);
        let _ = start;
        let plan = resolve_time_plan(&Some(end), Some(7), 2).unwrap();
        assert_eq!(plan.step_count, 2);
        assert_eq!(plan.step_days, 30);
        assert!(plan.step_coarsened);
        // A 14-day horizon at 7-day steps fits: 2 steps, no coarsening.
        let end = format_iso_date(today_days() + 14);
        let plan = resolve_time_plan(&Some(end), Some(7), 2).unwrap();
        assert_eq!(plan.step_count, 2);
        assert_eq!(plan.step_days, 7);
        assert!(!plan.step_coarsened);
        // A past end date is refused.
        let past = format_iso_date(today_days() - 1);
        assert!(resolve_time_plan(&Some(past), Some(7), 2).is_err());
        // An invalid step size is refused.
        assert!(resolve_time_plan(&None, Some(5), 2).is_err());
        // Round dates land on the step grid and clamp to the end date.
        let end = format_iso_date(today_days() + 10);
        let plan = resolve_time_plan(&Some(end.clone()), Some(7), 2).unwrap();
        assert_eq!(round_date(&plan, 1), Some(format_iso_date(today_days() + 7)));
        assert_eq!(round_date(&plan, 2), Some(end));
    }

    #[test]
    fn events_are_validated_against_the_round_budget() {
        let caps = resolve_caps(None, Some(2), None, None).unwrap();
        let bad_round = vec![SwarmtankAiEventRequest {
            round: 3,
            kind: "price_change".to_string(),
            description: "Raise the price.".to_string(),
            payload: None,
        }];
        assert!(validate_events(&Some(bad_round), caps.max_rounds).is_err());
        let bad_kind = vec![SwarmtankAiEventRequest {
            round: 1,
            kind: "alien_invasion".to_string(),
            description: "Something.".to_string(),
            payload: None,
        }];
        assert!(validate_events(&Some(bad_kind), caps.max_rounds).is_err());
        let ok = vec![SwarmtankAiEventRequest {
            round: 2,
            kind: "competitor_launch".to_string(),
            description: "A competitor launches a cheaper tier.".to_string(),
            payload: None,
        }];
        let events = validate_events(&Some(ok), caps.max_rounds).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "event-1");
    }
}
