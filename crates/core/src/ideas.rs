//! Project-scoped idea generation (MASTER_SPEC §§21–22, M6
//! Slice 2). See docs/handoffs/M6-slice2-design.md.
//!
//! Generation reuses the M5 provider substrate end to end: the
//! ops ride the `ai_command` providers arm (same CallerScope),
//! and a run orchestrates the existing provider stream ops
//! in-process. Routing, privacy gates, Zero-Cost Lock, permits
//! and disclosure are M5's, unchanged.
//!
//! Boundaries that define this slice:
//! - Context is ONLY what the user explicitly selected for this
//!   run (memory ids in the request). Nothing is ever scanned
//!   or added implicitly; each selected memory travels as a
//!   provider segment with its own privacy class and source id,
//!   so the M5 PrivacyGate decides what may leave the machine.
//! - Provider output is untrusted data: it is validated by a
//!   pure function (count 5–10, the twelve §21 fields, strict
//!   bounds) before anything is shown as a set or saved.
//! - Concepts are model-generated hypotheses. The prompt forbids
//!   invented market statistics, and no surface in this slice
//!   presents a concept as a researched fact. No scoring is
//!   computed: presentation order is the model's order.

use crate::artifacts::ArtifactRegisterRequest;
use crate::privacy::Privacy;
use crate::provider_dispatch::{CallerScope, providers_command};
use crate::readiness::{CommandError, ErrorCode};
use crate::storage::Storage;
use anansi_providers::routing::DisclosureRecord;
use anansi_providers::types::PrivacyClass;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// DTOs

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdeasOptions {
    #[serde(default)]
    pub explore_unrelated_domains: bool,
    #[serde(default)]
    pub prefer_simple: bool,
    #[serde(default)]
    pub prefer_high_revenue: bool,
    #[serde(default)]
    pub prefer_local_first: bool,
    #[serde(default)]
    pub prefer_software_only: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdeasGenerateStartRequest {
    #[serde(default = "one")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub topic: String,
    pub customer: String,
    pub budget: String,
    pub technology: String,
    pub objective: String,
    #[serde(default)]
    pub context_memory_ids: Vec<String>,
    pub options: Option<IdeasOptions>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub mode: Option<String>,
    #[serde(default)]
    pub zero_cost_lock: bool,
    pub permit_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdeasRunRequest {
    #[serde(default = "one")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub run_id: String,
}

/// One §21 concept. All fields are model-generated hypotheses.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdeaConcept {
    pub name: String,
    pub problem: String,
    pub target_customer: String,
    pub solution: String,
    pub why_useful: String,
    pub differentiation_hypothesis: String,
    pub revenue_model: String,
    pub engineering_complexity: String,
    pub implementation_cost: String,
    pub key_risks: Vec<String>,
    pub required_validation: String,
    pub minimum_viable_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdeaSet {
    pub concepts: Vec<IdeaConcept>,
    /// Provenance of the brief the set was generated from.
    pub topic: String,
    /// Set when the model returned more than 10 concepts and
    /// the set was truncated to the §22 bound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdeasGenerateStartResponse {
    pub contract_version: u8,
    pub run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disclosure: Option<DisclosureRecord>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdeasPollResponse {
    pub contract_version: u8,
    pub run_id: String,
    /// running | cancelling | complete | invalid | failed | cancelled
    pub status: String,
    pub partial_chars: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idea_set: Option<IdeaSet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disclosure: Option<DisclosureRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdeasSaveResponse {
    pub contract_version: u8,
    pub artifact: crate::artifacts::ArtifactRecord,
    pub body: String,
    pub already_saved: bool,
}

fn one() -> u8 {
    1
}

fn invalid(message: &str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn not_found() -> CommandError {
    CommandError::new(ErrorCode::NotFound, "Idea generation run not found.")
}

// ---------------------------------------------------------------------------
// Output validation — provider output is untrusted data.

const MAX_OUTPUT_CHARS: usize = 262_144;

fn field<'a>(
    obj: &'a serde_json::Map<String, Value>,
    key: &str,
    max: usize,
) -> Result<&'a str, String> {
    let value = obj
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("a concept is missing its '{key}' field"))?;
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max {
        return Err(format!(
            "a concept's '{key}' field is empty or over {max} characters"
        ));
    }
    Ok(value)
}

/// Parse and validate a model's raw output into an idea set.
/// Returns the set (with a truncation note when the model
/// overproduced) or a human-readable failure reason.
pub fn parse_idea_set(raw: &str, topic: &str) -> Result<IdeaSet, String> {
    let text = raw.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .map(|t| t.strip_suffix("```").unwrap_or(t))
        .unwrap_or(text)
        .trim();
    let parsed: Value = serde_json::from_str(text)
        .map_err(|_| "the model did not return a structured idea set (not JSON)".to_string())?;
    let items = parsed
        .get("concepts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            "the model did not return a structured idea set (no concepts array)".to_string()
        })?;
    if items.len() < 5 {
        return Err(format!(
            "the model returned {} concepts, fewer than the required 5; try generating again",
            items.len()
        ));
    }
    let total = items.len();
    let (items, note) = if total > 10 {
        (
            items[..10].to_vec(),
            Some(format!(
                "The model returned {total} concepts; the set was truncated to the maximum of 10."
            )),
        )
    } else {
        (items.clone(), None)
    };
    let mut concepts = Vec::with_capacity(items.len());
    for item in items {
        let obj = item
            .as_object()
            .ok_or_else(|| "a concept is not an object".to_string())?;
        let risks = match obj.get("keyRisks") {
            Some(Value::Array(values)) => {
                if values.is_empty() || values.len() > 6 {
                    return Err("a concept's 'keyRisks' must list 1 to 6 risks".to_string());
                }
                let mut out = Vec::new();
                for v in values {
                    let s = v
                        .as_str()
                        .ok_or_else(|| "a concept risk is not text".to_string())?;
                    let s = s.trim();
                    if s.is_empty() || s.chars().count() > 240 {
                        return Err("a concept risk is empty or over 240 characters".to_string());
                    }
                    out.push(s.to_string());
                }
                out
            }
            Some(Value::String(s)) => {
                let s = s.trim();
                if s.is_empty() || s.chars().count() > 240 {
                    return Err(
                        "a concept's 'keyRisks' is empty or over 240 characters".to_string()
                    );
                }
                vec![s.to_string()]
            }
            _ => return Err("a concept is missing its 'keyRisks' field".to_string()),
        };
        concepts.push(IdeaConcept {
            name: field(obj, "name", 120)?.to_string(),
            problem: field(obj, "problem", 800)?.to_string(),
            target_customer: field(obj, "targetCustomer", 300)?.to_string(),
            solution: field(obj, "solution", 800)?.to_string(),
            why_useful: field(obj, "whyUseful", 600)?.to_string(),
            differentiation_hypothesis: field(obj, "differentiationHypothesis", 600)?.to_string(),
            revenue_model: field(obj, "revenueModel", 400)?.to_string(),
            engineering_complexity: field(obj, "engineeringComplexity", 60)?.to_string(),
            implementation_cost: field(obj, "implementationCost", 300)?.to_string(),
            key_risks: risks,
            required_validation: field(obj, "requiredValidation", 600)?.to_string(),
            minimum_viable_version: field(obj, "minimumViableVersion", 600)?.to_string(),
        });
    }
    Ok(IdeaSet {
        concepts,
        topic: topic.to_string(),
        note,
    })
}

// ---------------------------------------------------------------------------
// Prompt construction

const INSTRUCTION: &str = "You are ANANSI's idea generation engine. Generate between 5 and 10 product concepts for the brief in the next message. Respond with ONLY a JSON object and no other text: {\"concepts\":[{\"name\":\"...\",\"problem\":\"...\",\"targetCustomer\":\"...\",\"solution\":\"...\",\"whyUseful\":\"...\",\"differentiationHypothesis\":\"...\",\"revenueModel\":\"...\",\"engineeringComplexity\":\"low|medium|high\",\"implementationCost\":\"...\",\"keyRisks\":[\"...\"],\"requiredValidation\":\"...\",\"minimumViableVersion\":\"...\"}]}. Order the concepts strongest first. Every concept is a hypothesis to be validated, not a researched fact: do not invent market-size statistics, competitor claims or novelty claims, and state assumptions plainly inside the fields. implementationCost is a rough effort estimate (for example 'about 2-4 weeks for one developer'), not a statistic.";

fn brief(req: &IdeasGenerateStartRequest) -> String {
    let o = req.options.as_ref();
    let flag = |on: bool, label: &str| {
        if on {
            format!("\n- {label}")
        } else {
            String::new()
        }
    };
    format!(
        "Brief:\n- Industry or topic: {}\n- Intended customer: {}\n- Available budget: {}\n- Preferred technology: {}\n- Objective: {}{}{}{}{}{}",
        req.topic.trim(),
        req.customer.trim(),
        req.budget.trim(),
        req.technology.trim(),
        req.objective.trim(),
        flag(
            o.is_some_and(|o| o.explore_unrelated_domains),
            "Explore unrelated domains for cross-domain ideas."
        ),
        flag(
            o.is_some_and(|o| o.prefer_simple),
            "Prefer simple products."
        ),
        flag(
            o.is_some_and(|o| o.prefer_high_revenue),
            "Prefer high-revenue opportunities."
        ),
        flag(
            o.is_some_and(|o| o.prefer_local_first),
            "Prefer local-first AI products."
        ),
        flag(
            o.is_some_and(|o| o.prefer_software_only),
            "Prefer software-only products."
        ),
    )
}

fn privacy_class(privacy: Privacy) -> PrivacyClass {
    match privacy {
        Privacy::Public => PrivacyClass::Public,
        Privacy::Standard => PrivacyClass::Standard,
        Privacy::Private => PrivacyClass::Private,
        Privacy::Sealed => PrivacyClass::Sealed,
    }
}

// ---------------------------------------------------------------------------
// Run registry (process-local, like provider streams)

struct IdeaRun {
    session: String,
    project_id: String,
    topic: String,
    context_ids: Vec<String>,
    stream_run_id: String,
    capability: String,
    disclosure: Option<DisclosureRecord>,
    text: String,
    status: String,
    idea_set: Option<IdeaSet>,
    error: Option<String>,
    saved_artifact: Option<crate::artifacts::ArtifactRecord>,
    saved_body: Option<String>,
    /// A save registration is in flight (claimed under the
    /// registry lock, executed outside it). Concurrent saves
    /// wait for the claimed save to publish instead of
    /// registering a second artifact.
    saving: bool,
    created: Instant,
}

static RUNS: Mutex<Option<HashMap<String, IdeaRun>>> = Mutex::new(None);

const RUN_TTL: Duration = Duration::from_secs(600);
const MAX_RUNS_PER_WORKSPACE: usize = 32;

fn with_runs<T>(f: impl FnOnce(&mut HashMap<String, IdeaRun>) -> T) -> T {
    let mut guard = RUNS.lock().unwrap_or_else(|p| p.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.retain(|_, r| r.created.elapsed() < RUN_TTL);
    f(map)
}

fn run_key(scope: &CallerScope, run_id: &str) -> String {
    format!("{}/{}", scope.workspace, run_id)
}

fn new_run_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("OS randomness is required for run ids");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Operations

pub fn ideas_command(
    operation: &str,
    value: Value,
    scope: &CallerScope,
) -> Result<Value, CommandError> {
    match operation {
        "ideas-generate-start" => start(value, scope),
        "ideas-generate-poll" => poll(value, scope),
        "ideas-generate-cancel" => cancel(value, scope),
        "ideas-save" => save(value, scope),
        _ => Err(invalid("Use a supported idea generation operation.")),
    }
}

fn storage(scope: &CallerScope) -> Result<Storage, CommandError> {
    let root = scope
        .storage_root
        .clone()
        .ok_or_else(|| invalid("Idea generation requires workspace storage."))?;
    Storage::new(root).map_err(|_| crate::storage::unavailable())
}

fn logical(scope: &CallerScope) -> Result<String, CommandError> {
    scope
        .logical_workspace
        .clone()
        .ok_or_else(|| invalid("A workspaceId is required."))
}

fn start(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: IdeasGenerateStartRequest = serde_json::from_value(value)
        .map_err(|_| invalid("Use a supported idea generation request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    for (label, text) in [
        ("topic", &req.topic),
        ("customer", &req.customer),
        ("budget", &req.budget),
        ("technology", &req.technology),
        ("objective", &req.objective),
    ] {
        if text.trim().is_empty() || text.chars().count() > 300 {
            return Err(invalid(&format!(
                "The {label} must be 1 to 300 characters."
            )));
        }
    }
    if req.context_memory_ids.len() > 16 {
        return Err(invalid("Select at most 16 context memories per run."));
    }
    let workspace_id = logical(scope)?;
    let storage = storage(scope)?;
    // Project must exist, be a project, and be writable.
    let connection = storage
        .connection(false)?
        .ok_or_else(crate::storage::unavailable)?;
    crate::artifacts::require_project_writable(&connection, &workspace_id, &req.project_id)?;
    drop(connection);

    // Explicit context only: read exactly the selected memories
    // in this workspace and carry each one's own privacy class.
    let mut segments = vec![
        json!({ "text": INSTRUCTION, "privacy": "standard" }),
        json!({ "text": brief(&req), "privacy": "standard" }),
    ];
    for id in &req.context_memory_ids {
        crate::workspace::validate_id(id)
            .map_err(|_| invalid("Context memory ids must use the ANANSI id format."))?;
        let details = storage
            .memory_read(crate::memory::MemoryReadRequest {
                contract_version: 1,
                workspace_id: workspace_id.clone(),
                id: id.clone(),
                revision_id: None,
            })
            .map_err(|_| invalid("A selected context memory was not found in this workspace."))?;
        segments.push(json!({
            "text": format!("Approved context from a memory titled \"{}\":\n{}", details.memory.title, details.body),
            "privacy": serde_json::to_value(privacy_class(details.memory.privacy)).unwrap_or(json!("private")),
            "sourceId": id,
        }));
    }

    // Resolve the target model from the trusted configuration:
    // an explicitly named model is used as given; otherwise the
    // first configured chat model in configuration order.
    let (provider, model) = match (&req.provider, &req.model) {
        (p, Some(m)) => (p.clone(), m.clone()),
        (Some(p), None) => (Some(p.clone()), {
            let configured = crate::provider_dispatch::configured_chat_models(scope);
            configured.into_iter().find(|(cp, _)| cp == p).map(|(_, m)| m).ok_or_else(|| invalid("No chat model is configured for this workspace; configure a provider first."))?
        }),
        _ => {
            let configured = crate::provider_dispatch::configured_chat_models(scope);
            configured
                .into_iter()
                .next()
                .map(|(p, m)| (Some(p), m))
                .ok_or_else(|| {
                    invalid("No chat model is configured for this workspace; configure a provider first.")
                })?
        }
    };
    let chosen = match &provider {
        Some(p) => json!([p, model]),
        None => Value::Null,
    };
    let dispatch = json!({
        "contractVersion": 1,
        "workspaceId": workspace_id,
        "request": {
            "model": model.clone(),
            "messages": [{ "role": "user", "segments": segments }],
            "maxOutputTokens": 4096,
            "localOnly": req.options.as_ref().is_some_and(|o| o.prefer_local_first),
            "explicitCloudAuthorization": !req.context_memory_ids.is_empty(),
        },
        "mode": req.mode.clone(),
        "zeroCostLock": req.zero_cost_lock,
        "chosen": chosen,
        "permitId": req.permit_id.clone(),
    });
    let run_id = new_run_id();
    let started = match providers_command("providers-stream-start", dispatch, scope) {
        Ok(started) => started,
        Err(e) if e.code == ErrorCode::StorageUnavailable => {
            // The provider was attempted and failed before any
            // token streamed (e.g. HTTP 500 at connect). That is
            // a generation outcome, not a request refusal: the
            // run exists in its terminal failed state so the
            // client renders one uniform failure surface.
            let run = IdeaRun {
                session: scope.session.clone(),
                project_id: req.project_id.clone(),
                topic: req.topic.trim().to_string(),
                context_ids: req.context_memory_ids.clone(),
                stream_run_id: String::new(),
                capability: String::new(),
                disclosure: None,
                text: String::new(),
                status: "failed".to_string(),
                idea_set: None,
                error: Some(e.message.clone()),
                saved_artifact: None,
                saved_body: None,
                saving: false,
                created: Instant::now(),
            };
            with_runs(|map| {
                map.insert(run_key(scope, &run_id), run);
            });
            return serde_json::to_value(IdeasGenerateStartResponse {
                contract_version: 1,
                run_id,
                disclosure: None,
            })
            .map_err(|_| crate::storage::unavailable());
        }
        Err(e) => return Err(e),
    };
    let disclosure: DisclosureRecord = serde_json::from_value(
        started
            .get("disclosure")
            .cloned()
            .ok_or_else(crate::storage::unavailable)?,
    )
    .map_err(|_| crate::storage::unavailable())?;
    let run = IdeaRun {
        session: scope.session.clone(),
        project_id: req.project_id.clone(),
        topic: req.topic.trim().to_string(),
        context_ids: req.context_memory_ids.clone(),
        stream_run_id: started
            .get("runId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        capability: started
            .get("capability")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        disclosure: Some(disclosure.clone()),
        text: String::new(),
        status: "running".to_string(),
        idea_set: None,
        error: None,
        saved_artifact: None,
        saved_body: None,
        saving: false,
        created: Instant::now(),
    };
    with_runs(|map| {
        let prefix = format!("{}/", scope.workspace);
        let mine: Vec<String> = map
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();
        if mine.len() >= MAX_RUNS_PER_WORKSPACE {
            // Evict the oldest run in this workspace.
            if let Some(oldest) = mine.iter().min_by_key(|k| map.get(*k).map(|r| r.created)) {
                map.remove(oldest);
            }
        }
        map.insert(run_key(scope, &run_id), run);
    });
    serde_json::to_value(IdeasGenerateStartResponse {
        contract_version: 1,
        run_id,
        disclosure: Some(disclosure),
    })
    .map_err(|_| crate::storage::unavailable())
}

fn poll(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: IdeasRunRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported idea run request."))?;
    let key = run_key(scope, &req.run_id);
    // Snapshot what the drain needs, then release the lock for
    // the provider poll (which does network I/O via the reader
    // thread's already-running stream; the poll itself is local).
    let (stream_run_id, capability, terminal) = with_runs(|map| {
        let run = map.get(&key).filter(|r| r.session == scope.session);
        match run {
            None => (String::new(), String::new(), None),
            Some(run) => (
                run.stream_run_id.clone(),
                run.capability.clone(),
                if matches!(
                    run.status.as_str(),
                    "complete" | "invalid" | "failed" | "cancelled"
                ) {
                    Some(run.status.clone())
                } else {
                    None
                },
            ),
        }
    });
    if stream_run_id.is_empty() && terminal.is_none() {
        return Err(not_found());
    }
    if terminal.is_none() {
        let workspace_id = logical(scope)?;
        let polled = providers_command(
            "providers-stream-poll",
            json!({
                "contractVersion": 1,
                "workspaceId": workspace_id,
                "runId": stream_run_id,
                "capability": capability,
            }),
            scope,
        )?;
        let events = polled
            .get("events")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let done = polled
            .get("done")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if let Some(d) = polled.get("disclosure")
            && let Ok(d) = serde_json::from_value::<DisclosureRecord>(d.clone())
        {
            with_runs(|map| {
                if let Some(run) = map.get_mut(&key) {
                    run.disclosure = Some(d);
                }
            });
        }
        let mut saw_completed = false;
        let mut failure: Option<String> = None;
        for event in &events {
            match event.get("kind").and_then(|v| v.as_str()) {
                Some("token") => {
                    if let Some(text) = event.get("text").and_then(|v| v.as_str()) {
                        with_runs(|map| {
                            if let Some(run) = map.get_mut(&key)
                                && run.text.chars().count() + text.chars().count()
                                    <= MAX_OUTPUT_CHARS
                            {
                                run.text.push_str(text);
                            }
                        });
                    }
                }
                Some("completed") => saw_completed = true,
                Some("error") => {
                    failure = Some(
                        event
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("The provider failed.")
                            .to_string(),
                    );
                }
                _ => {}
            }
        }
        if done {
            with_runs(|map| {
                if let Some(run) = map.get_mut(&key) {
                    if run.status == "cancelling" {
                        run.status = "cancelled".to_string();
                    } else if let Some(message) = &failure {
                        run.status = "failed".to_string();
                        run.error = Some(message.clone());
                    } else if saw_completed || failure.is_none() {
                        match parse_idea_set(&run.text.clone(), &run.topic.clone()) {
                            Ok(set) => {
                                run.idea_set = Some(set);
                                run.status = "complete".to_string();
                            }
                            Err(reason) => {
                                run.status = "invalid".to_string();
                                run.error = Some(reason);
                            }
                        }
                    }
                }
            });
        }
    }
    with_runs(|map| {
        let run = map.get(&key).ok_or_else(not_found)?;
        serde_json::to_value(IdeasPollResponse {
            contract_version: 1,
            run_id: req.run_id.clone(),
            status: run.status.clone(),
            partial_chars: run.text.chars().count(),
            idea_set: run.idea_set.clone(),
            disclosure: run.disclosure.clone(),
            error: run.error.clone(),
        })
        .map_err(|_| crate::storage::unavailable())
    })
}

fn cancel(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: IdeasRunRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported idea run request."))?;
    let key = run_key(scope, &req.run_id);
    let (stream_run_id, capability) = with_runs(|map| {
        let run = map.get_mut(&key).filter(|r| r.session == scope.session);
        match run {
            None => (String::new(), String::new()),
            Some(run) => {
                if run.status == "running" {
                    run.status = "cancelling".to_string();
                }
                (run.stream_run_id.clone(), run.capability.clone())
            }
        }
    });
    if stream_run_id.is_empty() {
        return Err(not_found());
    }
    let workspace_id = logical(scope)?;
    providers_command(
        "providers-stream-cancel",
        json!({
            "contractVersion": 1,
            "workspaceId": workspace_id,
            "runId": stream_run_id,
            "capability": capability,
        }),
        scope,
    )?;
    poll(
        json!({ "contractVersion": 1, "workspaceId": workspace_id, "runId": req.run_id }),
        scope,
    )
}

fn save(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: IdeasRunRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported idea run request."))?;
    let key = run_key(scope, &req.run_id);
    // Atomic idempotency: exactly one caller claims the save
    // (under the registry lock), registers WITHOUT holding the
    // lock, then publishes the artifact back. Concurrent saves
    // wait for the publication and resolve to the same
    // artifact; a failed registration releases the claim so a
    // waiter can retry the save itself.
    enum Claim {
        Missing,
        NotComplete,
        Ready(crate::artifacts::ArtifactRecord, String),
        Wait,
        Claimed {
            project_id: String,
            idea_set: Option<IdeaSet>,
            disclosure: Option<DisclosureRecord>,
            context_ids: Vec<String>,
        },
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    let (project_id, idea_set, disclosure, context_ids) = loop {
        let step = with_runs(|map| {
            let Some(run) = map.get_mut(&key).filter(|r| r.session == scope.session) else {
                return Claim::Missing;
            };
            if let (Some(artifact), Some(body)) =
                (run.saved_artifact.clone(), run.saved_body.clone())
            {
                return Claim::Ready(artifact, body);
            }
            if run.status != "complete" {
                return Claim::NotComplete;
            }
            if run.saving {
                return Claim::Wait;
            }
            run.saving = true;
            Claim::Claimed {
                project_id: run.project_id.clone(),
                idea_set: run.idea_set.clone(),
                disclosure: run.disclosure.clone(),
                context_ids: run.context_ids.clone(),
            }
        });
        match step {
            Claim::Missing => return Err(not_found()),
            Claim::NotComplete => {
                return Err(invalid(
                    "Only a completed idea generation can be saved; this run has not produced a validated set.",
                ));
            }
            Claim::Ready(artifact, body) => {
                return serde_json::to_value(IdeasSaveResponse {
                    contract_version: 1,
                    artifact,
                    body,
                    already_saved: true,
                })
                .map_err(|_| crate::storage::unavailable());
            }
            Claim::Wait => {
                if Instant::now() > deadline {
                    return Err(crate::storage::unavailable());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Claim::Claimed {
                project_id,
                idea_set,
                disclosure,
                context_ids,
            } => break (project_id, idea_set, disclosure, context_ids),
        }
    };
    let set = idea_set.ok_or_else(crate::storage::unavailable)?;
    let storage = storage(scope)?;
    let workspace_id = logical(scope)?;
    let body = serde_json::to_string(&set).map_err(|_| crate::storage::unavailable())?;
    let title = {
        let t = format!("Idea set: {}", set.topic);
        if t.chars().count() > 200 {
            t.chars().take(200).collect::<String>()
        } else {
            t
        }
    };
    let disclosure = disclosure.ok_or_else(crate::storage::unavailable)?;
    let details = storage.artifact_register_generated(
        &ArtifactRegisterRequest {
            contract_version: 1,
            workspace_id: workspace_id.clone(),
            project_id: project_id.clone(),
            artifact_type: "idea".to_string(),
            title,
            body: body.clone(),
            source_references: context_ids
                .iter()
                .map(|id| format!("memory://{id}"))
                .collect(),
            privacy: "private".to_string(),
        },
        json!({
            "kind": "model",
            "provider": disclosure.provider.clone(),
            "model": disclosure.model.clone(),
        }),
        "validated",
    )
    .inspect_err(|_| {
        with_runs(|map| {
            if let Some(run) = map.get_mut(&key) {
                run.saving = false;
            }
        });
    })?;
    with_runs(|map| {
        if let Some(run) = map.get_mut(&key) {
            run.saved_artifact = Some(details.artifact.clone());
            run.saved_body = Some(details.body.clone());
            run.saving = false;
        }
    });
    serde_json::to_value(IdeasSaveResponse {
        contract_version: 1,
        artifact: details.artifact,
        body: details.body,
        already_saved: false,
    })
    .map_err(|_| crate::storage::unavailable())
}
