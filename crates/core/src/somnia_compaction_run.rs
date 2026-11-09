// SOMNIA compaction comparison runs (Checkpoint 2 integration).
//
// The Checkpoint 1 compactor library is pure and provider-free. This
// module wires it into the application: prepare a candidate from
// imported records, approve it (the approval binds to the reviewed
// inputs — candidate hash, questions, selected provider/model, cloud
// flag, and budget — the server recomputes the approval hash at start
// and refuses any edit), compare both histories (original vs
// compacted) against the same configured model through the existing
// provider gates, and save the complete experiment as a new artifact
// type. Comparisons run free models only: one permit authorizes one
// dispatch chain, so no single permit could cover a comparison's
// up-to-100 dispatches, and the run never uses one.
//
// Runs are backgrounded, pollable and cancellable, following the
// SwarmTank AI pattern. The registry is process-local, so an
// in-flight run never survives an API restart: after a restart it
// polls as not-found, is never silently resumed, and never
// double-dispatches. Only explicitly saved experiments persist.

use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::artifacts::{ArtifactRecord, ArtifactRegisterRequest};
use crate::provider_dispatch::{
    CallerScope, configured_chat_models, configured_model_cost, providers_command,
};
use crate::readiness::{CommandError, ErrorCode};
use anansi_providers::types::CostClass;
use crate::somnia_compaction::{
    CompactionIssue, ParsedCompactionInput, QuarantinedRecord, SomniaCompactionCandidate,
    SomniaCompactionRecord, all_retained, compact_input, parse_json_input, parse_text_input,
    validate_candidate,
};

pub const COMPACTION_EXPERIMENT_FORMAT: &str = "anansi-somnia-compaction-experiment";
pub const COMPACTION_EXPERIMENT_ARTIFACT_TYPE: &str = "somnia-compaction-experiment";
pub const COMPACTION_EXPERIMENT_VERSION: u32 = 1;

/// The frozen benchmark is 40 cases with 50 questions; one pair per
/// question, two dispatches per pair.
pub const MAX_COMPACTION_CASES: usize = 40;
pub const MAX_COMPACTION_PAIRS: usize = 50;
pub const MAX_COMPACTION_CALLS: u32 = 100;
pub const MAX_COMPACTION_QUESTIONS_PER_CASE: usize = 4;
pub const MAX_COMPACTION_QUESTION_CHARS: usize = 2000;
pub const MAX_COMPACTION_OUTPUT_TOKENS: u32 = 384;
pub const MIN_COMPACTION_TOTAL_TOKENS: u64 = 1_000;
pub const MAX_COMPACTION_TOTAL_TOKENS: u64 = 200_000;
pub const DEFAULT_COMPACTION_TOTAL_TOKENS: u64 = 20_000;

const RUN_TTL: Duration = Duration::from_secs(600);
const MAX_RUNS_PER_WORKSPACE: usize = 32;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompactionQuestion {
    pub id: String,
    pub text: String,
    pub expected_answer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompactionStartCase {
    pub case_id: String,
    pub records: Vec<SomniaCompactionRecord>,
    pub quarantined: Vec<QuarantinedRecord>,
    pub candidate_hash: String,
    pub questions: Vec<CompactionQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SomniaCompactionPrepareRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub input_text: String,
    pub input_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SomniaCompactionPrepareResponse {
    pub contract_version: u8,
    pub candidate: SomniaCompactionCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SomniaCompactionStartRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub cases: Vec<CompactionStartCase>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub cloud: Option<bool>,
    pub permit_id: Option<String>,
    pub max_total_tokens: Option<u64>,
    /// SHA-256 hex of the canonical approval JSON (design §2 D2):
    /// the reviewed cases (case id, verified candidate hash,
    /// questions), the raw approved provider/model (blank allowed),
    /// the cloud flag, and the approved budget. The server recomputes
    /// it from the request and refuses any mismatch before traffic.
    /// Absent means the run was never approved.
    pub approval_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SomniaCompactionStartResponse {
    pub contract_version: u8,
    pub run_id: String,
    pub provider: String,
    pub model: String,
    pub locality: String,
    /// Cost class of the resolved model, from the trusted
    /// configuration: "free", "paid", or "unknown".
    pub cost: String,
    pub pairs_total: usize,
    pub estimated_calls: u32,
    pub estimated_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompactionUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompactionPairResult {
    pub pair_id: String,
    pub case_id: String,
    pub question_id: String,
    pub question: String,
    pub expected_answer: Option<String>,
    pub original_prompt: String,
    pub compacted_prompt: String,
    pub original_reply: Option<String>,
    pub compacted_reply: Option<String>,
    pub original_score: String,
    pub compacted_score: String,
    pub original_usage: Option<CompactionUsage>,
    pub compacted_usage: Option<CompactionUsage>,
    pub failure: Option<String>,
    pub compaction_micros: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SomniaCompactionPollRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SomniaCompactionPollResponse {
    pub contract_version: u8,
    pub run_id: String,
    pub status: String,
    pub provider: String,
    pub model: String,
    pub locality: String,
    /// Cost class of the resolved model, from the trusted
    /// configuration: "free", "paid", or "unknown".
    pub cost: String,
    pub pairs_total: usize,
    pub pairs_done: usize,
    pub calls_made: u32,
    pub tokens_used: u64,
    pub pairs: Vec<CompactionPairResult>,
    pub error: Option<String>,
    pub compaction_micros_total: u64,
    pub total_latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SomniaCompactionCancelRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SomniaCompactionCancelResponse {
    pub contract_version: u8,
    pub run_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompactionMetrics {
    pub pairs_total: usize,
    pub pairs_done: usize,
    pub correct_original: usize,
    pub correct_compacted: usize,
    pub abstentions_original: usize,
    pub abstentions_compacted: usize,
    pub failed_pairs: usize,
    pub tokens_original: Option<u64>,
    pub tokens_compacted: Option<u64>,
    pub tokens_reported_original_pairs: usize,
    pub tokens_reported_compacted_pairs: usize,
    pub compaction_micros_total: u64,
    pub total_latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SomniaCompactionExperiment {
    pub format: String,
    pub version: u32,
    pub run_id: String,
    pub provider: String,
    pub model: String,
    pub locality: String,
    pub input_hash: String,
    pub candidate: SomniaCompactionCandidate,
    pub pairs: Vec<CompactionPairResult>,
    pub metrics: CompactionMetrics,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SomniaCompactionSaveRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub experiment: SomniaCompactionExperiment,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SomniaCompactionSaveResponse {
    pub contract_version: u8,
    pub artifact: ArtifactRecord,
}

// ---------------------------------------------------------------------------
// Registry (process-local; mirrors swarmtank_ai.rs)
// ---------------------------------------------------------------------------

struct CompactionRun {
    project_id: String,
    status: String,
    provider: String,
    model: String,
    locality: String,
    cost: String,
    pairs: Vec<CompactionPairResult>,
    pairs_total: usize,
    calls_made: u32,
    tokens_used: u64,
    compaction_micros_total: u64,
    error: Option<String>,
    cancel: Arc<AtomicBool>,
    created: Instant,
    started: Instant,
}

static COMPACTION_RUNS: Mutex<Option<HashMap<String, CompactionRun>>> = Mutex::new(None);

fn with_compaction_runs<T>(f: impl FnOnce(&mut HashMap<String, CompactionRun>) -> T) -> T {
    let mut guard = COMPACTION_RUNS.lock().unwrap_or_else(|p| p.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

fn compaction_run_key(scope: &CallerScope, run_id: &str) -> String {
    format!("{}/{}", scope.workspace, run_id)
}

fn new_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("somnia-compaction-{nanos:x}")
}

fn collect_compaction_evictions(workspace_key: Option<&str>) {
    with_compaction_runs(|map| {
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

fn invalid(message: &str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn not_found() -> CommandError {
    CommandError::new(ErrorCode::NotFound, "Compaction run not found.")
}

pub fn somnia_compaction_command(
    operation: &str,
    value: Value,
    scope: &CallerScope,
) -> Result<Value, CommandError> {
    match operation {
        "somnia-compaction-prepare" => prepare_op(value, scope),
        "somnia-compaction-start" => start_op(value, scope),
        "somnia-compaction-poll" => poll_op(value, scope),
        "somnia-compaction-cancel" => cancel_op(value, scope),
        "somnia-compaction-save" => save_op(value, scope),
        _ => Err(invalid("Use a supported SOMNIA compaction request.")),
    }
}

// ---------------------------------------------------------------------------
// Prompt rendering and scoring (pure; the exact template is fixed by the
// integration design doc §2 and recorded in every saved experiment)
// ---------------------------------------------------------------------------

/// One line per record, canonical order (the candidate's retained and
/// excluded lists are already canonically ordered).
pub fn render_history(records: &[SomniaCompactionRecord]) -> String {
    records
        .iter()
        .map(|r| {
            format!(
                "[{}] {} ({}, rev {}): {}",
                r.id, r.key, r.source, r.revision, r.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build_prompt(records: &[SomniaCompactionRecord], question: &str) -> String {
    format!(
        "History:\n{}\n\nQuestion: {}\n\nAnswer with the exact value from the history, or UNKNOWN if the history does not state it.",
        render_history(records),
        question
    )
}

/// A reply counts as an abstention when, trimmed, it is empty, equals
/// UNKNOWN (case-insensitive), or starts with UNKNOWN followed by a
/// space or punctuation.
pub fn is_abstention(reply: &str) -> bool {
    let trimmed = reply.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower == "unknown" {
        return true;
    }
    if let Some(rest) = lower.strip_prefix("unknown") {
        return rest
            .chars()
            .next()
            .map(|c| c.is_whitespace() || c.is_ascii_punctuation())
            .unwrap_or(false);
    }
    false
}

/// Score one arm: "correct" | "incorrect" | "abstained" | "failed".
/// `reply` is None only when the dispatch itself failed.
pub fn score_reply(reply: Option<&str>, expected: &Option<String>) -> &'static str {
    let Some(text) = reply else {
        return "failed";
    };
    let abstained = is_abstention(text);
    match expected {
        // Missing-fact question: the correct behaviour is to abstain.
        None => {
            if abstained {
                "correct"
            } else {
                "incorrect"
            }
        }
        Some(want) => {
            if abstained {
                "abstained"
            } else if text.trim() == want.trim() {
                "correct"
            } else {
                "incorrect"
            }
        }
    }
}

fn sha_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn experiment_hash(experiment: &SomniaCompactionExperiment) -> String {
    let mut copy = experiment.clone();
    copy.content_hash = String::new();
    let text = serde_json::to_string(&copy).unwrap_or_default();
    sha_hex(&text)
}

/// Recompute the candidate from the reviewed records + quarantine and
/// refuse unless its hash is exactly the approved one. This is the
/// server-side half of the approval binding: any edit after review
/// changes the recomputed hash and the run never starts.
pub fn verify_approved_candidate(
    records: &[SomniaCompactionRecord],
    quarantined: &[QuarantinedRecord],
    candidate_hash: &str,
) -> Result<SomniaCompactionCandidate, String> {
    let issues: Vec<CompactionIssue> = quarantined
        .iter()
        .map(|q| CompactionIssue {
            record_ref: q.record_ref.clone(),
            reason: q.reason.clone(),
        })
        .collect();
    let parsed = ParsedCompactionInput {
        records: records.to_vec(),
        issues,
    };
    let candidate = compact_input(parsed)?;
    if candidate.content_hash != candidate_hash {
        return Err(
            "The reviewed candidate no longer matches the supplied records. Review the candidate again before starting the run."
                .to_string(),
        );
    }
    Ok(candidate)
}

/// Canonical approval input, serialized in field order. The UI
/// (`SomniaCompaction.tsx`) and the e2e mirror build the identical
/// JSON string; any key-order change here must change there too.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApprovalHashCase {
    case_id: String,
    candidate_hash: String,
    questions: Vec<CompactionQuestion>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApprovalHashInput {
    cases: Vec<ApprovalHashCase>,
    provider: String,
    model: String,
    cloud: bool,
    max_total_tokens: u64,
}

/// SHA-256 hex of the canonical approval JSON: the reviewed cases
/// (case id, verified candidate hash, questions), the raw approved
/// provider/model (blank allowed — a blank model is a reference to
/// the first configured model, see `resolve_model`), the cloud
/// flag, and the approved budget. Any edit after approval changes
/// the recomputed hash and the run never starts.
fn compute_approval_hash(
    cases: &[(String, String, Vec<CompactionQuestion>)],
    provider: &str,
    model: &str,
    cloud: bool,
    max_total_tokens: u64,
) -> String {
    let input = ApprovalHashInput {
        cases: cases
            .iter()
            .map(|(case_id, candidate_hash, questions)| ApprovalHashCase {
                case_id: case_id.clone(),
                candidate_hash: candidate_hash.clone(),
                questions: questions.clone(),
            })
            .collect(),
        provider: provider.to_string(),
        model: model.to_string(),
        cloud,
        max_total_tokens,
    };
    let text = serde_json::to_string(&input).unwrap_or_default();
    sha_hex(&text)
}

fn cost_label(cost: CostClass) -> &'static str {
    match cost {
        CostClass::Free => "free",
        CostClass::Paid => "paid",
        CostClass::Unknown => "unknown",
    }
}

/// A comparison runs free models only. One permit authorizes one
/// dispatch chain (`providers` dispatcher: a permit id is consumed
/// once per candidate), so no single permit could cover a
/// comparison's up-to-100 dispatches, and the run never uses a
/// permit at all. The check runs at start, before any call, so a
/// billable target is refused with zero traffic instead of failing
/// pair by pair at dispatch time.
fn check_run_target(
    locality: &str,
    cost: CostClass,
    cloud_authorized: bool,
    permit_id: &Option<String>,
) -> Result<(), String> {
    if permit_id.is_some() {
        return Err(
            "Compaction comparisons do not use single-use permits: one permit authorizes one dispatch, and a comparison needs up to 100. Remove the permit and approve a free model instead."
                .to_string(),
        );
    }
    if cost != CostClass::Free {
        return Err(format!(
            "This comparison only runs free models: the selected model is {}, and one permit cannot authorize a multi-dispatch comparison. Approve a free model instead.",
            cost_label(cost)
        ));
    }
    if !cloud_authorized && locality == "cloud" {
        return Err(
            "Local-only was requested but the configured model is not local."
                .to_string(),
        );
    }
    Ok(())
}

fn resolve_model(
    scope: &CallerScope,
    want_provider: &Option<String>,
    want_model: &Option<String>,
) -> Result<(String, String, String), CommandError> {
    let configured = configured_chat_models(scope);
    // A blank value is the same as absent: the panel promises a
    // blank Model ID selects the first configured model, so the
    // server resolves it that way instead of refusing.
    let provider = want_provider
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let model = want_model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let (provider, model) = match (provider, model) {
        (Some(p), Some(m)) => {
            let found = configured
                .iter()
                .any(|(cp, cm)| cp == p && cm == m);
            if !found {
                return Err(invalid(
                    "The requested provider and model are not configured for this workspace.",
                ));
            }
            (p.to_string(), m.to_string())
        }
        (Some(p), None) => configured
            .iter()
            .find(|(cp, _)| cp == p)
            .cloned()
            .ok_or_else(|| {
                invalid("The requested provider has no configured chat model for this workspace.")
            })?,
        (None, None) => configured.into_iter().next().ok_or_else(|| {
            invalid("No chat model is configured for this workspace; configure a provider first.")
        })?,
        (None, Some(_)) => {
            return Err(invalid(
                "Provider and model must be given together, or neither.",
            ));
        }
    };
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

// ---------------------------------------------------------------------------
// Ops
// ---------------------------------------------------------------------------

fn prepare_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SomniaCompactionPrepareRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SOMNIA compaction request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let (_workspace, _storage) =
        crate::swarmtank::require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    if req.input_text.chars().count() > 200_000 {
        return Err(invalid(
            "Compaction input must be at most 200,000 characters.",
        ));
    }
    let parsed = match req.input_format.as_str() {
        "json" => parse_json_input(&req.input_text),
        "text" => parse_text_input(&req.input_text),
        _ => {
            return Err(invalid(
                "Compaction input format must be json or text.",
            ));
        }
    }
    .map_err(|e| invalid(&e))?;
    let candidate = compact_input(parsed).map_err(|e| invalid(&e))?;
    serde_json::to_value(SomniaCompactionPrepareResponse {
        contract_version: 1,
        candidate,
    })
    .map_err(|_| crate::storage::unavailable())
}

struct ResolvedPair {
    pair_id: String,
    case_id: String,
    question: CompactionQuestion,
    original_prompt: String,
    compacted_prompt: String,
    compaction_micros: u64,
}

struct RunConfig {
    workspace_id: String,
    pairs: Vec<ResolvedPair>,
    provider: String,
    model: String,
    local_only: bool,
    /// The user explicitly approved cloud transmission for this
    /// comparison (review + approval hash). It is NOT a permit: the
    /// run never uses a permit, and the dispatch flag records the
    /// user's consent, not an authorization token.
    cloud_authorized: bool,
    max_total_tokens: u64,
    cancel: Arc<AtomicBool>,
}

fn sort_records_canonical(records: &mut [SomniaCompactionRecord]) {
    records.sort_by(|a, b| {
        (&a.key, &a.source, a.revision, &a.id).cmp(&(&b.key, &b.source, b.revision, &b.id))
    });
}

fn validate_questions(
    case_id: &str,
    questions: &[CompactionQuestion],
) -> Result<(), CommandError> {
    if questions.is_empty()
        || questions.len() > MAX_COMPACTION_QUESTIONS_PER_CASE
    {
        return Err(invalid(&format!(
            "Case {case_id} must have 1 to {MAX_COMPACTION_QUESTIONS_PER_CASE} questions.",
        )));
    }
    let mut seen = HashSet::new();
    for q in questions {
        if q.id.trim().is_empty() || q.id.chars().count() > 64 || !seen.insert(q.id.clone()) {
            return Err(invalid(&format!(
                "Case {case_id} has a missing, over-long, or duplicate question id.",
            )));
        }
        if q.text.trim().is_empty()
            || q.text.chars().count() > MAX_COMPACTION_QUESTION_CHARS
        {
            return Err(invalid(&format!(
                "Case {case_id} has a question that must be 1 to {MAX_COMPACTION_QUESTION_CHARS} characters.",
            )));
        }
        if let Some(expected) = &q.expected_answer
            && expected.chars().count() > MAX_COMPACTION_QUESTION_CHARS
        {
            return Err(invalid(&format!(
                "Case {case_id} has an expected answer over {MAX_COMPACTION_QUESTION_CHARS} characters.",
            )));
        }
    }
    Ok(())
}

fn start_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SomniaCompactionStartRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SOMNIA compaction request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let (workspace, _) =
        crate::swarmtank::require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    if req.cases.is_empty() || req.cases.len() > MAX_COMPACTION_CASES {
        return Err(invalid(&format!(
            "A compaction run takes 1 to {MAX_COMPACTION_CASES} cases.",
        )));
    }
    let max_total = req
        .max_total_tokens
        .unwrap_or(DEFAULT_COMPACTION_TOTAL_TOKENS);
    if !(MIN_COMPACTION_TOTAL_TOKENS..=MAX_COMPACTION_TOTAL_TOKENS).contains(&max_total) {
        return Err(invalid(
            "Max total tokens must be between 1000 and 200000.",
        ));
    }
    // The raw approved values (trimmed; blank means "first
    // configured"). The approval hash is computed over these, so any
    // edit after approval invalidates it.
    let provider_raw = req.provider.as_deref().unwrap_or("").trim().to_string();
    let model_raw = req.model.as_deref().unwrap_or("").trim().to_string();
    let cloud_effective = req.cloud.unwrap_or(false);
    // Approve-then-verify: recompute every case candidate and refuse
    // unless its hash is exactly the approved one. Compaction time is
    // measured per case here, before any provider call.
    let mut resolved_pairs: Vec<ResolvedPair> = Vec::new();
    let mut compaction_micros_total: u64 = 0;
    let mut verified: Vec<(String, String, Vec<CompactionQuestion>)> = Vec::new();
    for case in &req.cases {
        validate_questions(&case.case_id, &case.questions)?;
        let started = Instant::now();
        let candidate = verify_approved_candidate(
            &case.records,
            &case.quarantined,
            &case.candidate_hash,
        )
        .map_err(|e| invalid(&e))?;
        verified.push((
            case.case_id.clone(),
            candidate.content_hash.clone(),
            case.questions.clone(),
        ));
        let micros = started.elapsed().as_micros() as u64;
        compaction_micros_total = compaction_micros_total.saturating_add(micros);
        let mut original_records = case.records.clone();
        sort_records_canonical(&mut original_records);
        let mut retained_records: Vec<SomniaCompactionRecord> = all_retained(&candidate)
            .iter()
            .map(|r| r.record.clone())
            .collect();
        sort_records_canonical(&mut retained_records);
        for question in &case.questions {
            let pair_id = format!("{}:{}", case.case_id, question.id);
            resolved_pairs.push(ResolvedPair {
                pair_id,
                case_id: case.case_id.clone(),
                question: question.clone(),
                original_prompt: build_prompt(&original_records, &question.text),
                compacted_prompt: build_prompt(&retained_records, &question.text),
                compaction_micros: micros,
            });
        }
    }
    if resolved_pairs.len() > MAX_COMPACTION_PAIRS {
        return Err(invalid(&format!(
            "A compaction run takes at most {MAX_COMPACTION_PAIRS} pairs.",
        )));
    }
    // The approval binds the reviewed cases, the selected target,
    // the cloud flag, and the budget — not just the candidate hash.
    // A missing or mismatched hash refuses before any dispatch.
    let Some(approval_hash) = req.approval_hash.as_deref() else {
        return Err(invalid(
            "Approve the reviewed inputs before starting the run.",
        ));
    };
    let expected_hash = compute_approval_hash(
        &verified,
        &provider_raw,
        &model_raw,
        cloud_effective,
        max_total,
    );
    if approval_hash != expected_hash {
        return Err(invalid(
            "The approval no longer matches the reviewed inputs. Review the candidate again before starting the run.",
        ));
    }
    let (provider, model, locality) = resolve_model(scope, &req.provider, &req.model)?;
    // Free-only, up front (design §2 D3): a comparison runs free
    // models only — one permit authorizes one dispatch chain, so no
    // permit could cover the run, and the run never uses one. The
    // Zero-Cost Lock still runs per dispatch inside authorize_cost.
    let cost = configured_model_cost(scope, &provider, &model).ok_or_else(|| {
        invalid("The selected model is not configured for this workspace.")
    })?;
    check_run_target(&locality, cost, cloud_effective, &req.permit_id)
        .map_err(|e| invalid(&e))?;
    let local_only = !cloud_effective;
    let estimated_calls = (resolved_pairs.len() as u32).saturating_mul(2);
    let estimated_tokens: u64 = resolved_pairs
        .iter()
        .map(|p| {
            (p.original_prompt.chars().count() / 4
                + p.compacted_prompt.chars().count() / 4) as u64
                + 2 * MAX_COMPACTION_OUTPUT_TOKENS as u64
        })
        .sum();
    let pairs_total = resolved_pairs.len();
    let run_id = new_run_id();
    collect_compaction_evictions(Some(&scope.workspace));
    let cancel = Arc::new(AtomicBool::new(false));
    let run = CompactionRun {
        project_id: req.project_id.clone(),
        status: "running".to_string(),
        provider: provider.clone(),
        model: model.clone(),
        locality: locality.clone(),
        cost: cost_label(cost).to_string(),
        pairs: Vec::new(),
        pairs_total: resolved_pairs.len(),
        calls_made: 0,
        tokens_used: 0,
        compaction_micros_total,
        error: None,
        cancel: cancel.clone(),
        created: Instant::now(),
        started: Instant::now(),
    };
    with_compaction_runs(|map| {
        map.insert(compaction_run_key(scope, &run_id), run);
    });
    let thread_scope = scope.clone();
    let thread_run_id = run_id.clone();
    let thread_config = RunConfig {
        workspace_id: workspace,
        pairs: resolved_pairs,
        provider: provider.clone(),
        model: model.clone(),
        local_only,
        cloud_authorized: cloud_effective,
        max_total_tokens: max_total,
        cancel,
    };
    std::thread::spawn(move || {
        run_compaction(thread_run_id, thread_scope, thread_config);
    });
    serde_json::to_value(SomniaCompactionStartResponse {
        contract_version: 1,
        run_id,
        provider,
        model,
        locality,
        cost: cost_label(cost).to_string(),
        pairs_total,
        estimated_calls,
        estimated_tokens,
    })
    .map_err(|_| crate::storage::unavailable())
}

fn read_run(
    scope: &CallerScope,
    project_id: &str,
    run_id: &str,
) -> Result<SomniaCompactionPollResponse, CommandError> {
    with_compaction_runs(|map| {
        let run = map.get(&compaction_run_key(scope, run_id)).ok_or_else(not_found)?;
        if run.project_id != project_id {
            return Err(not_found());
        }
        let latency = if run.status == "running" {
            None
        } else {
            Some(run.started.elapsed().as_millis() as u64)
        };
        Ok(SomniaCompactionPollResponse {
            contract_version: 1,
            run_id: run_id.to_string(),
            status: run.status.clone(),
            provider: run.provider.clone(),
            model: run.model.clone(),
            locality: run.locality.clone(),
            cost: run.cost.clone(),
            pairs_total: run.pairs_total,
            pairs_done: run.pairs.len(),
            calls_made: run.calls_made,
            tokens_used: run.tokens_used,
            pairs: run.pairs.clone(),
            error: run.error.clone(),
            compaction_micros_total: run.compaction_micros_total,
            total_latency_ms: latency,
        })
    })
}

fn poll_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SomniaCompactionPollRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SOMNIA compaction request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let (_workspace, _storage) =
        crate::swarmtank::require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    let response = read_run(scope, &req.project_id, &req.run_id)?;
    serde_json::to_value(response).map_err(|_| crate::storage::unavailable())
}

fn cancel_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SomniaCompactionCancelRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SOMNIA compaction request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let (_workspace, _storage) =
        crate::swarmtank::require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    let status = with_compaction_runs(|map| {
        let run = map
            .get(&compaction_run_key(scope, &req.run_id))
            .ok_or_else(not_found)?;
        if run.project_id != req.project_id {
            return Err(not_found());
        }
        run.cancel.store(true, Ordering::SeqCst);
        Ok(run.status.clone())
    })?;
    serde_json::to_value(SomniaCompactionCancelResponse {
        contract_version: 1,
        run_id: req.run_id,
        status,
    })
    .map_err(|_| crate::storage::unavailable())
}

// ---------------------------------------------------------------------------
// Runner (background thread; updates the registry after every dispatch)
// ---------------------------------------------------------------------------

struct DispatchOutcome {
    reply: Option<String>,
    usage: Option<CompactionUsage>,
    failure: Option<String>,
}

fn read_usage(response: &Value) -> Option<CompactionUsage> {
    let usage = response.get("response")?.get("usage")?;
    let input = usage.get("inputTokens").and_then(|v| v.as_u64());
    let output = usage.get("outputTokens").and_then(|v| v.as_u64());
    let total = match (input, output) {
        (Some(i), Some(o)) => Some(i.saturating_add(o)),
        _ => None,
    };
    Some(CompactionUsage {
        input_tokens: input,
        output_tokens: output,
        total_tokens: total,
    })
}

fn dispatch_pair_arm(
    scope: &CallerScope,
    config: &RunConfig,
    prompt: &str,
) -> DispatchOutcome {
    let dispatch = json!({
        "contractVersion": 1,
        "workspaceId": config.workspace_id,
        "request": {
            "model": config.model.clone(),
            "messages": [{ "role": "user", "segments": [{ "text": prompt, "privacy": "standard" }] }],
            "maxOutputTokens": MAX_COMPACTION_OUTPUT_TOKENS,
            "temperature": 0,
            "localOnly": config.local_only,
            // The user's explicit approval to send this history to
            // the cloud provider (review + approval hash), not a
            // permit — the run never uses one.
            "explicitCloudAuthorization": config.cloud_authorized,
        },
        "mode": "chooseModel",
        "zeroCostLock": true,
        "chosen": [config.provider.clone(), config.model.clone()],
    });
    // OpenRouter reasoning is disabled for compaction comparisons:
    // reasoning tokens would crowd out answer text and the comparison
    // scores exact answers, not reasoning traces.
    let dispatch = if config.provider == "openrouter" {
        let mut dispatch = dispatch;
        if let Some(request) = dispatch.get_mut("request") {
            request["disableOpenRouterReasoning"] = json!(true);
        }
        dispatch
    } else {
        dispatch
    };
    let result = providers_command("providers-dispatch", dispatch, scope);
    let response = match result {
        Ok(value) => value,
        Err(e) => {
            return DispatchOutcome {
                reply: None,
                usage: None,
                failure: Some(format!("Provider call failed: {}", e.message)),
            };
        }
    };
    // The provider must report the model that was asked for; a
    // mismatch never produces a completed pair.
    let reported_provider = response
        .get("disclosure")
        .and_then(|d| d.get("provider"))
        .and_then(|v| v.as_str());
    let reported_model = response
        .get("disclosure")
        .and_then(|d| d.get("model"))
        .and_then(|v| v.as_str());
    let response_model = response
        .get("response")
        .and_then(|r| r.get("model"))
        .and_then(|v| v.as_str());
    if let Some(p) = reported_provider
        && p != config.provider
    {
        return DispatchOutcome {
            reply: None,
            usage: read_usage(&response),
            failure: Some("The provider reported a different provider. This pair cannot be compared.".to_string()),
        };
    }
    if let Some(m) = reported_model
        && m != config.model
    {
        return DispatchOutcome {
            reply: None,
            usage: read_usage(&response),
            failure: Some("The provider reported a different model. This pair cannot be compared.".to_string()),
        };
    }
    if let Some(m) = response_model
        && m != config.model
    {
        return DispatchOutcome {
            reply: None,
            usage: read_usage(&response),
            failure: Some("The provider reported a different model. This pair cannot be compared.".to_string()),
        };
    }
    let reply = response
        .get("response")
        .and_then(|r| r.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    if reply.is_none() {
        return DispatchOutcome {
            reply: None,
            usage: read_usage(&response),
            failure: Some(
                "Provider returned a response the core could not read.".to_string(),
            ),
        };
    }
    DispatchOutcome {
        reply,
        usage: read_usage(&response),
        failure: None,
    }
}

fn run_compaction(run_id: String, scope: CallerScope, config: RunConfig) {
    let key = compaction_run_key(&scope, &run_id);
    let mut terminal_status: Option<String> = None;
    'outer: for pair in &config.pairs {
        if config.cancel.load(Ordering::SeqCst) {
            terminal_status = Some("cancelled".to_string());
            break 'outer;
        }
        // Budget is checked before every dispatch; exhaustion ends
        // the run in a clear terminal state.
        for prompt in [&pair.original_prompt, &pair.compacted_prompt] {
            let over = with_compaction_runs(|map| {
                let Some(run) = map.get(&key) else {
                    return true;
                };
                let estimate =
                    (prompt.chars().count() / 4) as u64 + MAX_COMPACTION_OUTPUT_TOKENS as u64;
                run.calls_made + 1 > MAX_COMPACTION_CALLS
                    || run.tokens_used.saturating_add(estimate) > config.max_total_tokens
            });
            if over {
                terminal_status = Some("budget-exhausted".to_string());
                break 'outer;
            }
        }
        let original = dispatch_pair_arm(&scope, &config, &pair.original_prompt);
        with_compaction_runs(|map| {
            if let Some(run) = map.get_mut(&key) {
                run.calls_made = run.calls_made.saturating_add(1);
                if let Some(u) = &original.usage
                    && let Some(t) = u.total_tokens
                {
                    run.tokens_used = run.tokens_used.saturating_add(t);
                }
            }
        });
        if config.cancel.load(Ordering::SeqCst) {
            // The original arm already dispatched; record the pair as
            // failed rather than silently dropping a paid call, then
            // stop before the compacted arm.
            let failed = CompactionPairResult {
                pair_id: pair.pair_id.clone(),
                case_id: pair.case_id.clone(),
                question_id: pair.question.id.clone(),
                question: pair.question.text.clone(),
                expected_answer: pair.question.expected_answer.clone(),
                original_prompt: pair.original_prompt.clone(),
                compacted_prompt: pair.compacted_prompt.clone(),
                original_reply: original.reply.clone(),
                compacted_reply: None,
                original_score: score_reply(
                    original.reply.as_deref(),
                    &pair.question.expected_answer,
                )
                .to_string(),
                compacted_score: "failed".to_string(),
                original_usage: original.usage,
                compacted_usage: None,
                failure: Some("Run cancelled before the compacted arm.".to_string()),
                compaction_micros: pair.compaction_micros,
            };
            with_compaction_runs(|map| {
                if let Some(run) = map.get_mut(&key) {
                    run.pairs.push(failed);
                }
            });
            terminal_status = Some("cancelled".to_string());
            break 'outer;
        }
        let compacted = dispatch_pair_arm(&scope, &config, &pair.compacted_prompt);
        with_compaction_runs(|map| {
            if let Some(run) = map.get_mut(&key) {
                run.calls_made = run.calls_made.saturating_add(1);
                if let Some(u) = &compacted.usage
                    && let Some(t) = u.total_tokens
                {
                    run.tokens_used = run.tokens_used.saturating_add(t);
                }
            }
        });
        let failure = match (&original.failure, &compacted.failure) {
            (Some(a), Some(b)) => Some(format!("Original arm: {a} Compacted arm: {b}")),
            (Some(a), None) => Some(format!("Original arm: {a}")),
            (None, Some(b)) => Some(format!("Compacted arm: {b}")),
            (None, None) => None,
        };
        let result = CompactionPairResult {
            pair_id: pair.pair_id.clone(),
            case_id: pair.case_id.clone(),
            question_id: pair.question.id.clone(),
            question: pair.question.text.clone(),
            expected_answer: pair.question.expected_answer.clone(),
            original_prompt: pair.original_prompt.clone(),
            compacted_prompt: pair.compacted_prompt.clone(),
            original_reply: original.reply.clone(),
            compacted_reply: compacted.reply.clone(),
            original_score: score_reply(
                original.reply.as_deref(),
                &pair.question.expected_answer,
            )
            .to_string(),
            compacted_score: score_reply(
                compacted.reply.as_deref(),
                &pair.question.expected_answer,
            )
            .to_string(),
            original_usage: original.usage,
            compacted_usage: compacted.usage,
            failure,
            compaction_micros: pair.compaction_micros,
        };
        with_compaction_runs(|map| {
            if let Some(run) = map.get_mut(&key) {
                run.pairs.push(result);
            }
        });
    }
    let status = terminal_status.unwrap_or_else(|| "complete".to_string());
    with_compaction_runs(|map| {
        if let Some(run) = map.get_mut(&key) {
            run.status = status;
            run.error = None;
        }
    });
}

// ---------------------------------------------------------------------------
// Save: the server revalidates everything fail-closed, exactly like
// somnia.rs::save_op recomputes the evaluation doc.
// ---------------------------------------------------------------------------

pub fn compute_metrics(
    pairs: &[CompactionPairResult],
    compaction_micros_total: u64,
    total_latency_ms: u64,
) -> CompactionMetrics {
    let mut correct_original = 0;
    let mut correct_compacted = 0;
    let mut abstentions_original = 0;
    let mut abstentions_compacted = 0;
    let mut failed_pairs = 0;
    let mut tokens_original: Option<u64> = Some(0);
    let mut tokens_compacted: Option<u64> = Some(0);
    let mut reported_original = 0;
    let mut reported_compacted = 0;
    for pair in pairs {
        if pair.original_score == "correct" {
            correct_original += 1;
        }
        if pair.compacted_score == "correct" {
            correct_compacted += 1;
        }
        // Abstention counts measure behaviour (the arm abstained),
        // independently of whether the abstention was correct.
        if let Some(reply) = &pair.original_reply
            && is_abstention(reply)
        {
            abstentions_original += 1;
        }
        if let Some(reply) = &pair.compacted_reply
            && is_abstention(reply)
        {
            abstentions_compacted += 1;
        }
        if pair.original_score == "failed" || pair.compacted_score == "failed" {
            failed_pairs += 1;
        }
        match pair
            .original_usage
            .as_ref()
            .and_then(|u| u.total_tokens)
        {
            Some(t) => {
                reported_original += 1;
                tokens_original = tokens_original.map(|sum| sum.saturating_add(t));
            }
            None => tokens_original = None,
        }
        match pair
            .compacted_usage
            .as_ref()
            .and_then(|u| u.total_tokens)
        {
            Some(t) => {
                reported_compacted += 1;
                tokens_compacted = tokens_compacted.map(|sum| sum.saturating_add(t));
            }
            None => tokens_compacted = None,
        }
    }
    CompactionMetrics {
        pairs_total: pairs.len(),
        pairs_done: pairs.len(),
        correct_original,
        correct_compacted,
        abstentions_original,
        abstentions_compacted,
        failed_pairs,
        tokens_original,
        tokens_compacted,
        tokens_reported_original_pairs: reported_original,
        tokens_reported_compacted_pairs: reported_compacted,
        compaction_micros_total,
        total_latency_ms,
    }
}

pub fn validate_experiment(experiment: &SomniaCompactionExperiment) -> Result<(), String> {
    if experiment.format != COMPACTION_EXPERIMENT_FORMAT {
        return Err("Experiment format must be anansi-somnia-compaction-experiment.".to_string());
    }
    if experiment.version != COMPACTION_EXPERIMENT_VERSION {
        return Err("Experiment version must be 1.".to_string());
    }
    if experiment.run_id.trim().is_empty() {
        return Err("Experiment run id must not be empty.".to_string());
    }
    if experiment.provider.trim().is_empty() || experiment.model.trim().is_empty() {
        return Err("Experiment provider and model must not be empty.".to_string());
    }
    if experiment.locality != "local" && experiment.locality != "cloud" {
        return Err("Experiment locality must be local or cloud.".to_string());
    }
    // The candidate must be exactly the one the records produce.
    validate_candidate(&experiment.candidate)?;
    let mut records: Vec<SomniaCompactionRecord> = all_retained(&experiment.candidate)
        .iter()
        .map(|r| r.record.clone())
        .collect();
    records.extend(
        experiment
            .candidate
            .excluded
            .iter()
            .map(|e| e.record.clone()),
    );
    let issues: Vec<CompactionIssue> = experiment
        .candidate
        .quarantined
        .iter()
        .map(|q| CompactionIssue {
            record_ref: q.record_ref.clone(),
            reason: q.reason.clone(),
        })
        .collect();
    let recomputed = compact_input(ParsedCompactionInput { records, issues })
        .map_err(|e| format!("Experiment candidate does not recompute: {e}"))?;
    if recomputed.content_hash != experiment.candidate.content_hash {
        return Err("Experiment candidate hash does not match its records.".to_string());
    }
    if recomputed.input_hash != experiment.input_hash {
        return Err("Experiment input hash does not match its candidate.".to_string());
    }
    if experiment.pairs.is_empty() || experiment.pairs.len() > MAX_COMPACTION_PAIRS {
        return Err(format!(
            "Experiment must have 1 to {MAX_COMPACTION_PAIRS} pairs.",
        ));
    }
    // Every pair's prompts must be exactly the rendered histories,
    // and every score must follow from its replies.
    let mut all_records = experiment
        .candidate
        .excluded
        .iter()
        .map(|e| e.record.clone())
        .collect::<Vec<_>>();
    all_records.extend(
        all_retained(&experiment.candidate)
            .iter()
            .map(|r| r.record.clone()),
    );
    sort_records_canonical(&mut all_records);
    let mut retained: Vec<SomniaCompactionRecord> = all_retained(&experiment.candidate)
        .iter()
        .map(|r| r.record.clone())
        .collect();
    sort_records_canonical(&mut retained);
    for pair in &experiment.pairs {
        if build_prompt(&all_records, &pair.question) != pair.original_prompt {
            return Err(format!(
                "Pair {} original prompt does not match its records.",
                pair.pair_id
            ));
        }
        if build_prompt(&retained, &pair.question) != pair.compacted_prompt {
            return Err(format!(
                "Pair {} compacted prompt does not match its retained records.",
                pair.pair_id
            ));
        }
        if score_reply(pair.original_reply.as_deref(), &pair.expected_answer)
            != pair.original_score.as_str()
        {
            return Err(format!(
                "Pair {} original score does not follow from its reply.",
                pair.pair_id
            ));
        }
        if score_reply(pair.compacted_reply.as_deref(), &pair.expected_answer)
            != pair.compacted_score.as_str()
        {
            return Err(format!(
                "Pair {} compacted score does not follow from its reply.",
                pair.pair_id
            ));
        }
        if pair.original_score == "failed" || pair.compacted_score == "failed" {
            if pair.failure.as_ref().map(|f| f.trim().is_empty()).unwrap_or(true) {
                return Err(format!(
                    "Pair {} is failed but carries no failure reason.",
                    pair.pair_id
                ));
            }
        }
    }
    let expected_metrics = compute_metrics(
        &experiment.pairs,
        experiment.metrics.compaction_micros_total,
        experiment.metrics.total_latency_ms,
    );
    if expected_metrics.pairs_total != experiment.metrics.pairs_total
        || expected_metrics.correct_original != experiment.metrics.correct_original
        || expected_metrics.correct_compacted != experiment.metrics.correct_compacted
        || expected_metrics.abstentions_original != experiment.metrics.abstentions_original
        || expected_metrics.abstentions_compacted != experiment.metrics.abstentions_compacted
        || expected_metrics.failed_pairs != experiment.metrics.failed_pairs
        || expected_metrics.tokens_original != experiment.metrics.tokens_original
        || expected_metrics.tokens_compacted != experiment.metrics.tokens_compacted
        || expected_metrics.tokens_reported_original_pairs
            != experiment.metrics.tokens_reported_original_pairs
        || expected_metrics.tokens_reported_compacted_pairs
            != experiment.metrics.tokens_reported_compacted_pairs
    {
        return Err("Experiment metrics do not match its pairs.".to_string());
    }
    if experiment_hash(experiment) != experiment.content_hash {
        return Err("Experiment content hash does not match its body.".to_string());
    }
    Ok(())
}

fn save_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SomniaCompactionSaveRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SOMNIA compaction request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let (workspace, storage) =
        crate::swarmtank::require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    validate_experiment(&req.experiment).map_err(|e| invalid(&e))?;
    let body = serde_json::to_string(&req.experiment).map_err(|_| crate::storage::unavailable())?;
    let excerpt: String = req
        .experiment
        .pairs
        .first()
        .map(|p| p.question.clone())
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect();
    let title = {
        let t = format!("SOMNIA compaction experiment: {excerpt}");
        if t.chars().count() > 200 {
            t.chars().take(200).collect()
        } else {
            t
        }
    };
    let details = storage.artifact_register_generated(
        &ArtifactRegisterRequest {
            contract_version: 1,
            workspace_id: workspace.clone(),
            project_id: req.project_id.clone(),
            artifact_type: COMPACTION_EXPERIMENT_ARTIFACT_TYPE.to_string(),
            title,
            body,
            source_references: vec![],
            privacy: "private".to_string(),
        },
        serde_json::json!({
            "kind": "deterministic",
            "generator": "anansi-somnia-compaction-run",
            "model": format!("{}/{}", req.experiment.provider, req.experiment.model),
        }),
        "validated",
    )?;
    serde_json::to_value(SomniaCompactionSaveResponse {
        contract_version: 1,
        artifact: details.artifact,
    })
    .map_err(|_| crate::storage::unavailable())
}

// ---------------------------------------------------------------------------
// Tests (pure functions only; the ops are covered by the e2e battery,
// which drives them through the real API + provider layer)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::somnia_compaction::compact;

    fn record(
        id: &str,
        key: &str,
        source: &str,
        revision: u32,
        kind: &str,
        trust: &str,
        text: &str,
    ) -> SomniaCompactionRecord {
        SomniaCompactionRecord {
            id: id.to_string(),
            key: key.to_string(),
            source: source.to_string(),
            revision,
            kind: kind.to_string(),
            trust: trust.to_string(),
            privacy: "standard".to_string(),
            text: text.to_string(),
            supersedes: Vec::new(),
        }
    }

    fn with_supersedes(
        mut r: SomniaCompactionRecord,
        targets: &[&str],
    ) -> SomniaCompactionRecord {
        r.supersedes = targets.iter().map(|s| s.to_string()).collect();
        r
    }

    fn pair_for(
        pair_id: &str,
        case_id: &str,
        question: &str,
        expected: Option<&str>,
        original_prompt: &str,
        compacted_prompt: &str,
        original_reply: Option<&str>,
        compacted_reply: Option<&str>,
    ) -> CompactionPairResult {
        CompactionPairResult {
            pair_id: pair_id.to_string(),
            case_id: case_id.to_string(),
            question_id: "q1".to_string(),
            question: question.to_string(),
            expected_answer: expected.map(|s| s.to_string()),
            original_prompt: original_prompt.to_string(),
            compacted_prompt: compacted_prompt.to_string(),
            original_reply: original_reply.map(|s| s.to_string()),
            compacted_reply: compacted_reply.map(|s| s.to_string()),
            original_score: score_reply(original_reply, &expected.map(|s| s.to_string()))
                .to_string(),
            compacted_score: score_reply(compacted_reply, &expected.map(|s| s.to_string()))
                .to_string(),
            original_usage: Some(CompactionUsage {
                input_tokens: Some(100),
                output_tokens: Some(10),
                total_tokens: Some(110),
            }),
            compacted_usage: Some(CompactionUsage {
                input_tokens: Some(50),
                output_tokens: Some(10),
                total_tokens: Some(60),
            }),
            failure: None,
            compaction_micros: 120,
        }
    }

    fn experiment_for(candidate: SomniaCompactionCandidate, pairs: Vec<CompactionPairResult>) -> SomniaCompactionExperiment {
        let metrics = compute_metrics(&pairs, 120, 500);
        let mut experiment = SomniaCompactionExperiment {
            format: COMPACTION_EXPERIMENT_FORMAT.to_string(),
            version: COMPACTION_EXPERIMENT_VERSION,
            run_id: "somnia-compaction-test".to_string(),
            provider: "ollama".to_string(),
            model: "llama-x".to_string(),
            locality: "local".to_string(),
            input_hash: candidate.input_hash.clone(),
            candidate,
            pairs,
            metrics,
            content_hash: String::new(),
        };
        experiment.content_hash = experiment_hash(&experiment);
        experiment
    }

    #[test]
    fn rendering_differs_only_by_record_set() {
        let records = vec![
            record("r1", "fee", "ledger", 1, "fact", "trusted", "Fee is $40."),
            with_supersedes(
                record("r2", "fee", "ledger", 2, "fact", "trusted", "Fee is $45."),
                &["r1"],
            ),
        ];
        let candidate = compact(&records).unwrap();
        let retained: Vec<SomniaCompactionRecord> = all_retained(&candidate)
            .iter()
            .map(|r| r.record.clone())
            .collect();
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].id, "r2");
        let original = build_prompt(&records, "What is the fee?");
        let compacted = build_prompt(&retained, "What is the fee?");
        assert!(original.contains("Fee is $40."));
        assert!(original.contains("Fee is $45."));
        assert!(!compacted.contains("Fee is $40."));
        assert!(compacted.contains("Fee is $45."));
        assert!(original.contains("Question: What is the fee?"));
        assert!(compacted.contains("or UNKNOWN if the history does not state it."));
    }

    #[test]
    fn scoring_matrix() {
        let want = Some("The customs fee is $45.".to_string());
        assert_eq!(score_reply(Some("The customs fee is $45."), &want), "correct");
        assert_eq!(
            score_reply(Some("  The customs fee is $45.  "), &want),
            "correct"
        );
        assert_eq!(score_reply(Some("The customs fee is $40."), &want), "incorrect");
        assert_eq!(score_reply(Some("UNKNOWN"), &want), "abstained");
        assert_eq!(score_reply(Some(""), &want), "abstained");
        assert_eq!(score_reply(None, &want), "failed");
        // Missing-fact question: abstaining is correct.
        assert_eq!(score_reply(Some("UNKNOWN"), &None), "correct");
        assert_eq!(
            score_reply(Some("Unknown — the history does not say."), &None),
            "correct"
        );
        assert_eq!(score_reply(Some("The fee is $45."), &None), "incorrect");
        assert_eq!(score_reply(None, &None), "failed");
    }

    #[test]
    fn abstention_detection() {
        assert!(is_abstention("UNKNOWN"));
        assert!(is_abstention("unknown"));
        assert!(is_abstention("Unknown."));
        assert!(is_abstention("UNKNOWN — no such record."));
        assert!(is_abstention(""));
        assert!(is_abstention("   "));
        assert!(!is_abstention("The fee is $45."));
        assert!(!is_abstention("UNKNOWNFEE"));
        assert!(!is_abstention("It is unknown to me whether..."));
    }

    #[test]
    fn approval_hash_refusal() {
        let records = vec![
            record("r1", "fee", "ledger", 1, "fact", "trusted", "Fee is $40."),
            with_supersedes(
                record("r2", "fee", "ledger", 2, "fact", "trusted", "Fee is $45."),
                &["r1"],
            ),
        ];
        let candidate = compact(&records).unwrap();
        let ok = verify_approved_candidate(&records, &[], &candidate.content_hash);
        assert!(ok.is_ok());
        // Any edit after review changes the recomputed hash.
        let mut edited = records.clone();
        edited[1].text = "Fee is $99.".to_string();
        let refused = verify_approved_candidate(&edited, &[], &candidate.content_hash);
        assert!(refused.is_err());
        // A wrong hash is refused even with untouched records.
        let wrong = verify_approved_candidate(&records, &[], &"0".repeat(64));
        assert!(wrong.is_err());
    }

    #[test]
    fn experiment_validation_and_tamper() {
        let records = vec![
            record("r1", "fee", "ledger", 1, "fact", "trusted", "Fee is $40."),
            with_supersedes(
                record("r2", "fee", "ledger", 2, "fact", "trusted", "Fee is $45."),
                &["r1"],
            ),
        ];
        let candidate = compact(&records).unwrap();
        let question = "What is the fee?";
        let pair = pair_for(
            "case-001:q1",
            "case-001",
            question,
            Some("Fee is $45."),
            &build_prompt(&records, question),
            &build_prompt(
                &all_retained(&candidate)
                    .iter()
                    .map(|r| r.record.clone())
                    .collect::<Vec<_>>(),
                question,
            ),
            Some("Fee is $45."),
            Some("Fee is $45."),
        );
        let experiment = experiment_for(candidate, vec![pair]);
        assert!(validate_experiment(&experiment).is_ok());

        // Tampering with a reply breaks its score.
        let mut tampered = experiment.clone();
        tampered.pairs[0].original_reply = Some("Fee is $40.".to_string());
        assert!(validate_experiment(&tampered).is_err());

        // Tampering with a metric breaks the metrics check.
        let mut tampered = experiment.clone();
        tampered.metrics.correct_original = 0;
        assert!(validate_experiment(&tampered).is_err());

        // Tampering with the candidate breaks its hash.
        let mut tampered = experiment.clone();
        tampered.candidate.retained.trusted[0].record.text = "Fee is $99.".to_string();
        assert!(validate_experiment(&tampered).is_err());

        // Tampering with the content hash breaks the hash check.
        let mut tampered = experiment.clone();
        tampered.content_hash = "0".repeat(64);
        assert!(validate_experiment(&tampered).is_err());

        // A failed pair without a reason is refused.
        let mut failed = experiment.clone();
        failed.pairs[0].original_score = "failed".to_string();
        failed.pairs[0].original_reply = None;
        failed.pairs[0].failure = None;
        failed.metrics = compute_metrics(&failed.pairs, 120, 500);
        failed.content_hash = experiment_hash(&failed);
        assert!(validate_experiment(&failed).is_err());
    }

    #[test]
    fn metrics_count_abstentions_independently() {
        let records = vec![record(
            "r1",
            "fee",
            "ledger",
            1,
            "fact",
            "trusted",
            "Fee is $40.",
        )];
        let question = "What is the fee?";
        let prompt = build_prompt(&records, question);
        // Missing-fact question, both arms abstain: correct AND
        // counted as abstentions.
        let pair = pair_for(
            "case-017:q1",
            "case-017",
            question,
            None,
            &prompt,
            &prompt,
            Some("UNKNOWN"),
            Some("UNKNOWN"),
        );
        assert_eq!(pair.original_score, "correct");
        assert_eq!(pair.compacted_score, "correct");
        let metrics = compute_metrics(&[pair], 90, 100);
        assert_eq!(metrics.correct_original, 1);
        assert_eq!(metrics.correct_compacted, 1);
        assert_eq!(metrics.abstentions_original, 1);
        assert_eq!(metrics.abstentions_compacted, 1);
        assert_eq!(metrics.tokens_original, Some(110));
        assert_eq!(metrics.tokens_compacted, Some(60));
        assert_eq!(metrics.failed_pairs, 0);
    }

    fn approval_cases() -> Vec<(String, String, Vec<CompactionQuestion>)> {
        vec![(
            "case-001".to_string(),
            "a".repeat(64),
            vec![CompactionQuestion {
                id: "q1".to_string(),
                text: "What is the customs fee now?".to_string(),
                expected_answer: Some("The customs fee is $45.".to_string()),
            }],
        )]
    }

    #[test]
    fn approval_hash_binds_every_reviewed_field() {
        let cases = approval_cases();
        let base = compute_approval_hash(&cases, "ollama", "llama-x", false, 20_000);
        assert_eq!(base.len(), 64);
        // Same inputs hash the same; any one field changed hashes
        // differently — questions, target, cloud, budget, candidate.
        assert_eq!(
            compute_approval_hash(&cases, "ollama", "llama-x", false, 20_000),
            base
        );
        let mut changed_questions = approval_cases();
        changed_questions[0].2[0].text = "What was the fee before?".to_string();
        assert_ne!(
            compute_approval_hash(&changed_questions, "ollama", "llama-x", false, 20_000),
            base
        );
        assert_ne!(
            compute_approval_hash(&cases, "ollama", "aaa-decoy", false, 20_000),
            base
        );
        assert_ne!(
            compute_approval_hash(&cases, "ollama", "", false, 20_000),
            base
        );
        assert_ne!(
            compute_approval_hash(&cases, "ollama", "llama-x", true, 20_000),
            base
        );
        assert_ne!(
            compute_approval_hash(&cases, "ollama", "llama-x", false, 50_000),
            base
        );
        let mut changed_candidate = approval_cases();
        changed_candidate[0].1 = "b".repeat(64);
        assert_ne!(
            compute_approval_hash(&changed_candidate, "ollama", "llama-x", false, 20_000),
            base
        );
    }

    #[test]
    fn free_only_gate_matrix() {
        // Free local without a permit: allowed.
        assert!(check_run_target("local", CostClass::Free, false, &None).is_ok());
        // Free cloud with explicit approval: allowed, no permit.
        assert!(check_run_target("cloud", CostClass::Free, true, &None).is_ok());
        // Free cloud without approval: refused (local-only request).
        assert!(check_run_target("cloud", CostClass::Free, false, &None).is_err());
        // Billable targets are refused before traffic, local or cloud,
        // with or without approval — one permit cannot cover the run.
        assert!(check_run_target("local", CostClass::Paid, false, &None).is_err());
        assert!(check_run_target("cloud", CostClass::Paid, true, &None).is_err());
        assert!(check_run_target("cloud", CostClass::Unknown, true, &None).is_err());
        // A supplied permit never authorizes a comparison.
        assert!(
            check_run_target("local", CostClass::Free, false, &Some("permit-1".to_string()))
                .is_err()
        );
    }
}
