//! Explicitly approved LEXI-COLLIDER → Product Forge
//! transfer (MASTER_SPEC §9, §27; M8 Slice 2). See
//! docs/handoffs/M8-slice2-forge-transfer-design.md.
//!
//! This is a model-free direct op: it contacts no
//! provider, starts no inference, creates no
//! specification approval, authorizes no paid
//! inference and permits no code execution. It binds
//! one exact saved experiment version, one selected
//! combination from that version, and the user's
//! approved brief into a durable `concept` artifact
//! that the existing Forge spec workflow can develop
//! (with its own separate explicit action).
//!
//! Boundaries that define this slice:
//! - Opening an experiment, selecting a combination
//!   or editing the brief calls no op and saves
//!   nothing. Only the explicit transfer op creates.
//! - The transfer pins (experiment id, exact version,
//!   content hash as read) — never "latest". A newer
//!   experiment version neither changes an existing
//!   transfer nor inherits its approval.
//! - Nothing is fabricated: fields the experiment
//!   does not have are absent, and unknowns stay
//!   explicit. The backend composes no brief; the
//!   approved brief is recorded exactly as the user
//!   submitted it, labeled user-authored, while the
//!   verbatim experiment fields stay labeled
//!   model-derived.
//! - Idempotency is structural, not temporal: the
//!   same (experiment, version, combination,
//!   approved brief) returns the same concept
//!   artifact, including concurrent submissions
//!   (process-wide mutex) and retries after restart
//!   (persisted scan). A different brief is a
//!   different transfer, never a silent merge.

use crate::artifacts::{ArtifactListRequest, ArtifactRegisterRequest};
use crate::collider::{ColliderCombination, ColliderExperimentDoc, validate_experiment_doc};
use crate::provider_dispatch::CallerScope;
use crate::readiness::{CommandError, ErrorCode};
use crate::storage::Storage;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// DTOs

fn one() -> u8 {
    1
}

fn invalid(message: &str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn not_found(message: &str) -> CommandError {
    CommandError::new(ErrorCode::NotFound, message)
}

/// The transfer request: an exact experiment version,
/// one selected combination from it, and the brief the
/// user approved. Unknown fields are refused.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransferRequest {
    #[serde(default = "one")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub experiment_artifact_id: String,
    pub experiment_version: u32,
    #[serde(default)]
    pub combination_index: usize,
    pub approved_brief: String,
}

/// A durable Forge-compatible concept: one approved
/// collider combination plus its provenance. The
/// `format` discriminator is what separates this body
/// from any other `concept` artifact a user registers
/// by hand.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransferConceptDoc {
    pub format: String,
    pub version: u32,
    pub source_experiment_artifact_id: String,
    pub source_experiment_version: u32,
    pub source_experiment_content_hash: String,
    pub selected_combination_index: usize,
    pub selected_combination: ColliderCombination,
    pub approved_brief: String,
    pub assumptions: Vec<String>,
    pub unknowns: Vec<String>,
    pub productization_brief: String,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransferResponse {
    pub contract_version: u8,
    pub artifact: crate::artifacts::ArtifactRecord,
    pub body: String,
    pub already_transferred: bool,
}

const MAX_BRIEF_CHARS: usize = 4000;

// ---------------------------------------------------------------------------
// Validation

fn check_text(text: &str, label: &str, max: usize) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.chars().count() > max {
        return Err(format!(
            "{label} must be 1 to {max} characters, got {}",
            trimmed.chars().count()
        ));
    }
    Ok(())
}

fn check_list(
    items: &[String],
    label: &str,
    min: usize,
    max: usize,
    max_chars: usize,
) -> Result<(), String> {
    if items.len() < min || items.len() > max {
        return Err(format!(
            "{label} must list {min} to {max} items, got {}",
            items.len()
        ));
    }
    for item in items {
        check_text(item, label, max_chars)?;
    }
    Ok(())
}

/// Strict validation for a transfer concept body, used
/// by the transfer op itself, by forge-spec-start when
/// it admits a transferred concept, and by restore
/// pre-validation. Returns the parsed doc on success.
pub fn validate_transfer_concept(raw: &str) -> Result<TransferConceptDoc, String> {
    let doc: TransferConceptDoc = serde_json::from_str(raw)
        .map_err(|e| format!("transferred concept is not readable: {e}"))?;
    if doc.format != "anansi-forge-concept" {
        return Err(format!(
            "transferred concept format must be anansi-forge-concept, got {}",
            doc.format
        ));
    }
    if doc.version != 1 {
        return Err(format!(
            "transferred concept version must be 1, got {}",
            doc.version
        ));
    }
    if doc.source_experiment_artifact_id.trim().is_empty()
        || doc.source_experiment_content_hash.trim().is_empty()
        || doc.provider.trim().is_empty()
        || doc.model.trim().is_empty()
    {
        return Err("transferred concept is missing its experiment provenance".to_string());
    }
    check_text(
        &doc.selected_combination.name,
        "selectedCombination.name",
        120,
    )?;
    check_text(
        &doc.selected_combination.description,
        "selectedCombination.description",
        1200,
    )?;
    check_text(&doc.approved_brief, "approvedBrief", MAX_BRIEF_CHARS)?;
    check_list(&doc.assumptions, "assumptions", 1, 8, 600)?;
    check_list(&doc.unknowns, "unknowns", 0, 16, 300)?;
    check_text(&doc.productization_brief, "productizationBrief", 2000)?;
    Ok(doc)
}

// ---------------------------------------------------------------------------
// Transfer op

/// Serializes scan-and-register so two concurrent
/// submissions of the same transfer cannot both
/// create a concept. Retries after restart are
/// collapsed by the persisted scan below, not by
/// this mutex.
static TRANSFER_LOCK: Mutex<()> = Mutex::new(());

fn storage(scope: &CallerScope) -> Result<Storage, CommandError> {
    let root = scope
        .storage_root
        .clone()
        .ok_or_else(|| invalid("Transfer requires workspace storage."))?;
    Storage::new(root).map_err(|_| crate::storage::unavailable())
}

fn logical(scope: &CallerScope) -> Result<String, CommandError> {
    scope
        .logical_workspace
        .clone()
        .ok_or_else(|| invalid("A workspaceId is required."))
}

pub fn transfer(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: TransferRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported transfer request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    if req.approved_brief.trim().is_empty() || req.approved_brief.chars().count() > MAX_BRIEF_CHARS
    {
        return Err(invalid("The approved brief must be 1 to 4000 characters."));
    }
    let workspace_id = logical(scope)?;
    let storage = storage(scope)?;
    let connection = storage
        .connection(false)?
        .ok_or_else(crate::storage::unavailable)?;
    crate::artifacts::require_project_writable(&connection, &workspace_id, &req.project_id)?;
    drop(connection);

    // Pin the exact source version: the combination
    // comes from that version's body, never from a
    // later revision, and the binding records the
    // content hash as read.
    let source = storage
        .artifact_read(
            &workspace_id,
            &req.experiment_artifact_id,
            Some(req.experiment_version),
        )
        .map_err(|_| {
            not_found("The selected experiment version does not exist in this workspace.")
        })?;
    if source.artifact.project_id != req.project_id {
        return Err(invalid(
            "The selected experiment does not belong to this project.",
        ));
    }
    if source.artifact.artifact_type != "experiment" {
        return Err(invalid(
            "A transfer starts from a saved collider experiment.",
        ));
    }
    let experiment: ColliderExperimentDoc = serde_json::from_str(&source.body)
        .map_err(|_| invalid("The selected experiment is not a readable collider experiment."))?;
    validate_experiment_doc(&experiment).map_err(|e| {
        invalid(&format!(
            "The selected experiment is not a valid collider experiment: {e}"
        ))
    })?;
    let content_hash = crate::content::hash(source.body.as_bytes());
    if content_hash != source.artifact.content_hash {
        return Err(invalid(
            "The selected experiment's stored hash does not match its body; it may have been tampered with.",
        ));
    }
    let combination = experiment
        .result
        .combinations
        .get(req.combination_index)
        .cloned()
        .ok_or_else(|| {
            invalid("The selected combination does not exist in the pinned experiment version.")
        })?;
    let mut unknowns = experiment.result.feasibility.unknown_costs.clone();
    unknowns.extend(experiment.result.feasibility.dependencies.clone());

    let doc = TransferConceptDoc {
        format: "anansi-forge-concept".to_string(),
        version: 1,
        source_experiment_artifact_id: req.experiment_artifact_id.clone(),
        source_experiment_version: req.experiment_version,
        source_experiment_content_hash: content_hash,
        selected_combination_index: req.combination_index,
        selected_combination: combination.clone(),
        approved_brief: req.approved_brief.trim().to_string(),
        assumptions: experiment.result.feasibility.hypotheses.clone(),
        unknowns,
        productization_brief: experiment.result.productization.concept_brief.clone(),
        provider: experiment.provider.clone(),
        model: experiment.model.clone(),
    };
    validate_transfer_concept(
        &serde_json::to_string(&doc).map_err(|_| crate::storage::unavailable())?,
    )
    .map_err(|e| invalid(&format!("The transfer concept is not valid: {e}")))?;
    let body = serde_json::to_string(&doc).map_err(|_| crate::storage::unavailable())?;
    let title = {
        let name: String = combination.name.chars().take(160).collect();
        format!("Concept: {name}")
    };

    let _guard = TRANSFER_LOCK
        .lock()
        .map_err(|_| crate::storage::unavailable())?;
    // Idempotency: the same (experiment, version,
    // combination, approved brief) returns the same
    // concept, whether the second submission is a
    // concurrent request or a retry after restart.
    let listed = storage.artifact_list(&ArtifactListRequest {
        contract_version: 1,
        workspace_id: workspace_id.clone(),
        project_id: req.project_id.clone(),
    })?;
    for candidate in listed.artifacts {
        if candidate.artifact_type != "concept" {
            continue;
        }
        let Ok(details) = storage.artifact_read(
            &workspace_id,
            &candidate.artifact_id,
            Some(candidate.version),
        ) else {
            continue;
        };
        let Ok(existing) = validate_transfer_concept(&details.body) else {
            continue;
        };
        if existing.source_experiment_artifact_id == req.experiment_artifact_id
            && existing.source_experiment_version == req.experiment_version
            && existing.selected_combination_index == req.combination_index
            && existing.approved_brief == doc.approved_brief
        {
            return serde_json::to_value(TransferResponse {
                contract_version: 1,
                artifact: details.artifact,
                body: details.body,
                already_transferred: true,
            })
            .map_err(|_| crate::storage::unavailable());
        }
    }

    let details = storage.artifact_register_generated(
        &ArtifactRegisterRequest {
            contract_version: 1,
            workspace_id: workspace_id.clone(),
            project_id: req.project_id.clone(),
            artifact_type: "concept".to_string(),
            title,
            body: body.clone(),
            // Canonical reference format, so restore
            // remaps it through the identity map like
            // every other artifact reference.
            source_references: vec![format!(
                "artifact://{}@v{}",
                req.experiment_artifact_id, req.experiment_version
            )],
            privacy: "private".to_string(),
        },
        json!({
            "kind": "transfer",
            "provider": experiment.provider,
            "model": experiment.model,
        }),
        "validated",
    )?;
    serde_json::to_value(TransferResponse {
        contract_version: 1,
        artifact: details.artifact,
        body,
        already_transferred: false,
    })
    .map_err(|_| crate::storage::unavailable())
}

// ---------------------------------------------------------------------------
// Restore remapping

/// Rewrite the pinned experiment reference in an
/// `anansi-forge-concept` body through the restore
/// identity map. Returns `None` for other bodies. A
/// reference to an experiment that is not in the
/// backup fails closed; the caller recomputes the
/// row hash afterwards.
pub fn remap_transfer_references(
    body: &str,
    ids: &BTreeMap<String, String>,
) -> Result<Option<String>, CommandError> {
    let Ok(mut doc) = serde_json::from_str::<Value>(body) else {
        return Ok(None);
    };
    if doc.get("format").and_then(|v| v.as_str()) != Some("anansi-forge-concept") {
        return Ok(None);
    }
    let old = doc
        .get("sourceExperimentArtifactId")
        .and_then(|v| v.as_str())
        .ok_or_else(crate::jobs::invalid)?;
    let new = ids.get(old).ok_or_else(crate::jobs::invalid)?;
    doc["sourceExperimentArtifactId"] = Value::String(new.clone());
    Ok(Some(
        serde_json::to_string(&doc).map_err(|_| crate::storage::unavailable())?,
    ))
}
