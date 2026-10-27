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
//!   artifact, including concurrent submissions in
//!   other processes (one SQLite IMMEDIATE
//!   transaction around the receipt scan and both
//!   inserts) and retries after restart (the receipt
//!   row persists). A different brief is a
//!   different transfer, never a silent merge.
//! - Approval is a version-1 `transfer-approval`
//!   receipt row pinning the full approved snapshot,
//!   not the concept's format string or generator
//!   metadata: Forge rechecks the receipt and the
//!   pinned experiment before any provider traffic.

use crate::artifacts::ArtifactRegisterRequest;
use crate::collider::{ColliderCombination, ColliderExperimentDoc, validate_experiment_doc};
use crate::provider_dispatch::CallerScope;
use crate::readiness::{CommandError, ErrorCode};
use crate::storage::Storage;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

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

/// The backend-only approval receipt for one transfer.
/// Registered by raw SQL through the transfer op, exactly
/// like `specification-approval` rows, and deliberately
/// absent from `ARTIFACT_TYPES`, so generic registration
/// of a receipt is refused and generic revision can only
/// add a later version. Only the version-1 row confers
/// approval: it pins the full approved snapshot, and
/// Forge checks it against both the requested concept
/// body and the pinned experiment before any provider
/// traffic (see `validate_transfer_approval`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransferApprovalDoc {
    pub format: String,
    pub version: u32,
    pub concept_artifact_id: String,
    pub concept_version: u32,
    pub concept_content_hash: String,
    pub approved_brief: String,
    pub source_experiment_artifact_id: String,
    pub source_experiment_version: u32,
    pub source_experiment_content_hash: String,
    pub selected_combination_index: usize,
    pub selected_combination: ColliderCombination,
    pub assumptions: Vec<String>,
    pub unknowns: Vec<String>,
    pub productization_brief: String,
    pub provider: String,
    pub model: String,
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

/// Strict validation for a transfer approval receipt
/// body, used by Forge admission and restore
/// pre-validation. Returns the parsed doc on success.
pub fn validate_transfer_approval_doc(raw: &str) -> Result<TransferApprovalDoc, String> {
    let doc: TransferApprovalDoc = serde_json::from_str(raw)
        .map_err(|e| format!("transfer approval is not readable: {e}"))?;
    if doc.format != "anansi-transfer-approval" {
        return Err(format!(
            "transfer approval format must be anansi-transfer-approval, got {}",
            doc.format
        ));
    }
    if doc.version != 1 {
        return Err(format!(
            "transfer approval version must be 1, got {}",
            doc.version
        ));
    }
    if doc.concept_artifact_id.trim().is_empty()
        || doc.concept_content_hash.trim().is_empty()
        || doc.source_experiment_artifact_id.trim().is_empty()
        || doc.source_experiment_content_hash.trim().is_empty()
        || doc.provider.trim().is_empty()
        || doc.model.trim().is_empty()
    {
        return Err("transfer approval is missing its concept or experiment provenance".to_string());
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

/// The model-derived fields a transfer must carry,
/// derived from one pinned experiment version: the
/// selected combination, the feasibility hypotheses
/// as assumptions, unknown costs followed by
/// dependencies as the ordered unknowns, the
/// productization concept brief, and provider/model
/// provenance. Shared by the transfer op, the Forge
/// admission check and restore pre-validation so all
/// three agree on what counts as the same transfer.
pub fn expected_transfer_fields(
    experiment: &ColliderExperimentDoc,
    combination_index: usize,
) -> Result<
    (
        ColliderCombination,
        Vec<String>,
        Vec<String>,
        String,
        String,
        String,
    ),
    String,
> {
    let combination = experiment
        .result
        .combinations
        .get(combination_index)
        .cloned()
        .ok_or_else(|| {
            "The selected combination does not exist in the pinned experiment version.".to_string()
        })?;
    let mut unknowns = experiment.result.feasibility.unknown_costs.clone();
    unknowns.extend(experiment.result.feasibility.dependencies.clone());
    Ok((
        combination,
        experiment.result.feasibility.hypotheses.clone(),
        unknowns,
        experiment.result.productization.concept_brief.clone(),
        experiment.provider.clone(),
        experiment.model.clone(),
    ))
}

/// Authoritative admission check for a transferred
/// concept at Forge dispatch. A format string,
/// shape-valid JSON, client flag or editable generator
/// metadata is not approval: the caller must present
/// a concept version whose version-1 transfer-approval
/// receipt (generator kind `transfer`, which manual
/// registration can never produce) pins the same
/// concept id, version and content hash, the same
/// approved brief, experiment binding and combination,
/// and the same model-derived snapshot — and that
/// snapshot must match the pinned experiment version
/// in the same workspace/project, whose stored hash
/// must equal its body hash and the pinned hash.
/// Any mismatch refuses before any provider traffic.
/// Saving or revising a concept confers no approval.
pub fn validate_transfer_approval(
    storage: &Storage,
    workspace_id: &str,
    project_id: &str,
    concept_artifact_id: &str,
    concept_version: u32,
    concept_content_hash: &str,
    concept: &TransferConceptDoc,
) -> Result<(), CommandError> {
    let unapproved = || {
        invalid(&format!(
            "Concept artifact {concept_artifact_id} version {concept_version} has not been approved through an explicit collider transfer. Approve that exact version through the transfer operation before developing it; saving or revising a concept does not approve it."
        ))
    };
    let (record, body) = storage
        .find_transfer_receipt(
            workspace_id,
            project_id,
            concept_artifact_id,
            concept_version,
            concept_content_hash,
        )?
        .ok_or_else(unapproved)?;
    let generator_kind = record
        .generator
        .as_ref()
        .and_then(|g| g.get("kind"))
        .and_then(|v| v.as_str());
    if generator_kind != Some("transfer") || record.validation_status != "validated" {
        return Err(unapproved());
    }
    let approval = validate_transfer_approval_doc(&body).map_err(|_| unapproved())?;
    if approval.concept_artifact_id != concept_artifact_id
        || approval.concept_version != concept_version
        || approval.concept_content_hash != concept_content_hash
        || approval.approved_brief != concept.approved_brief
        || approval.source_experiment_artifact_id != concept.source_experiment_artifact_id
        || approval.source_experiment_version != concept.source_experiment_version
        || approval.source_experiment_content_hash != concept.source_experiment_content_hash
        || approval.selected_combination_index != concept.selected_combination_index
        || approval.selected_combination.name != concept.selected_combination.name
        || approval.selected_combination.description != concept.selected_combination.description
        || approval.assumptions != concept.assumptions
        || approval.unknowns != concept.unknowns
        || approval.productization_brief != concept.productization_brief
        || approval.provider != concept.provider
        || approval.model != concept.model
    {
        return Err(unapproved());
    }
    let source = storage
        .artifact_read(
            workspace_id,
            &concept.source_experiment_artifact_id,
            Some(concept.source_experiment_version),
        )
        .map_err(|_| unapproved())?;
    if source.artifact.project_id != project_id
        || source.artifact.artifact_type != "experiment"
        || crate::content::hash(source.body.as_bytes()) != source.artifact.content_hash
        || source.artifact.content_hash != concept.source_experiment_content_hash
    {
        return Err(unapproved());
    }
    let experiment: ColliderExperimentDoc =
        serde_json::from_str(&source.body).map_err(|_| unapproved())?;
    validate_experiment_doc(&experiment).map_err(|_| unapproved())?;
    let (combination, assumptions, unknowns, productization_brief, provider, model) =
        expected_transfer_fields(&experiment, concept.selected_combination_index)
            .map_err(|_| unapproved())?;
    if combination.name != concept.selected_combination.name
        || combination.description != concept.selected_combination.description
        || assumptions != concept.assumptions
        || unknowns != concept.unknowns
        || productization_brief != concept.productization_brief
        || provider != concept.provider
        || model != concept.model
    {
        return Err(unapproved());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Transfer op

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
    let (combination, assumptions, unknowns, productization_brief, provider, model) =
        expected_transfer_fields(&experiment, req.combination_index)
            .map_err(|e| invalid(&e))?;
    // The approved brief is recorded exactly as
    // submitted: validation above rejects whitespace-only
    // and overlong briefs, but accepted text keeps its
    // indentation, tabs and terminal newlines.
    let doc = TransferConceptDoc {
        format: "anansi-forge-concept".to_string(),
        version: 1,
        source_experiment_artifact_id: req.experiment_artifact_id.clone(),
        source_experiment_version: req.experiment_version,
        source_experiment_content_hash: content_hash.clone(),
        selected_combination_index: req.combination_index,
        selected_combination: combination.clone(),
        approved_brief: req.approved_brief.clone(),
        assumptions: assumptions.clone(),
        unknowns: unknowns.clone(),
        productization_brief: productization_brief.clone(),
        provider: provider.clone(),
        model: model.clone(),
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
    // The receipt is built from the same values the
    // concept validation above already checked; its
    // concept id and content hash are pinned inside
    // `register_concept_with_receipt` when the concept
    // row is inserted, and Forge and restore validate
    // the pinned receipt strictly before trusting it.
    let approval = TransferApprovalDoc {
        format: "anansi-transfer-approval".to_string(),
        version: 1,
        // Pinned on commit, once the concept row exists
        // and its content hash is known.
        concept_artifact_id: String::new(),
        concept_version: 1,
        concept_content_hash: String::new(),
        approved_brief: req.approved_brief.clone(),
        source_experiment_artifact_id: req.experiment_artifact_id.clone(),
        source_experiment_version: req.experiment_version,
        source_experiment_content_hash: content_hash,
        selected_combination_index: req.combination_index,
        selected_combination: combination.clone(),
        assumptions,
        unknowns,
        productization_brief,
        provider,
        model,
    };
    // Idempotency is durable, not temporal: the same
    // (experiment, version, combination, approved
    // brief) returns the same concept and its original
    // approved snapshot, whether the second submission
    // is a concurrent request in another process, a
    // retry after restart, or a retry after a generic
    // revision changed the latest concept version.
    // The check and the create run in one SQLite
    // IMMEDIATE transaction (see
    // `register_concept_with_receipt`): a second
    // process blocks on the database write lock until
    // the first commits, then finds the receipt and
    // returns the original instead of duplicating.
    let (details, already_transferred) = storage.register_concept_with_receipt(
        &workspace_id,
        &req.project_id,
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
        approval,
    )?;
    let body = if already_transferred {
        details.body.clone()
    } else {
        body
    };
    serde_json::to_value(TransferResponse {
        contract_version: 1,
        artifact: details.artifact,
        body,
        already_transferred,
    })
    .map_err(|_| crate::storage::unavailable())
}

// ---------------------------------------------------------------------------
// Restore remapping

/// Rewrite the pinned concept and experiment
/// references in an `anansi-transfer-approval` body
/// through the restore identity map. Returns `None`
/// for other bodies. A reference to a concept or
/// experiment that is not in the backup fails closed;
/// the caller recomputes the row hash afterwards, and
/// the fix-up pass rewrites the pinned concept
/// content hash to the restored concept row's hash.
pub fn remap_transfer_approval_references(
    body: &str,
    ids: &BTreeMap<String, String>,
) -> Result<Option<String>, CommandError> {
    let Ok(mut doc) = serde_json::from_str::<Value>(body) else {
        return Ok(None);
    };
    if doc.get("format").and_then(|v| v.as_str()) != Some("anansi-transfer-approval") {
        return Ok(None);
    }
    for key in ["conceptArtifactId", "sourceExperimentArtifactId"] {
        let old = doc
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(crate::jobs::invalid)?;
        let new = ids.get(old).ok_or_else(crate::jobs::invalid)?;
        doc[key] = Value::String(new.clone());
    }
    Ok(Some(
        serde_json::to_string(&doc).map_err(|_| crate::storage::unavailable())?,
    ))
}

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
