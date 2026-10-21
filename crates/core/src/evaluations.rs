//! M6 Slice 3: transparent concept evaluation & comparison.
//!
//! Concepts from a saved idea set (Slice 2) are assessed by
//! the USER against a versioned rubric — itself a registry
//! artifact — and the comparison is saved as an immutable
//! `evaluation` artifact linked to the exact source and
//! rubric versions. The only computation is a deterministic
//! weighted summary over the user's own 1–5 assessments:
//! same inputs, same outputs, no model involvement, and a
//! missing assessment is reported as "not assessed", never
//! silently scored zero. Scores are structured judgement,
//! not market evidence; the stored document says so.

use crate::artifacts::{
    ArtifactListRequest, ArtifactRecord, ArtifactRegisterRequest,
};
use crate::ideas::{IdeaConcept, parse_idea_set};
use crate::storage::{Storage, unavailable};
use crate::{CommandError, ErrorCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

fn invalid(message: &str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn not_found() -> CommandError {
    CommandError::new(ErrorCode::NotFound, "The requested record does not exist.")
}

// ---------------------------------------------------------------------
// Rubric

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RubricDimension {
    pub key: String,
    pub label: String,
    pub description: String,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RubricDoc {
    pub name: String,
    pub dimensions: Vec<RubricDimension>,
}

/// The owner-approved starting rubric (M6 plan decision 1,
/// settled 2026-10-21): five dimensions, equal weights. A
/// starting point the user can edit into a new rubric
/// version — not an objectively validated business model.
pub fn standard_rubric() -> RubricDoc {
    let dimension = |key: &str, label: &str, description: &str| RubricDimension {
        key: key.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        weight: 1.0,
    };
    RubricDoc {
        name: "Standard evaluation rubric".to_string(),
        dimensions: vec![
            dimension(
                "problem-clarity",
                "Problem clarity",
                "How clearly and specifically the problem is stated.",
            ),
            dimension(
                "customer-specificity",
                "Customer specificity",
                "How precisely the target customer is identified.",
            ),
            dimension(
                "mvp-feasibility",
                "MVP feasibility",
                "How achievable the minimum viable version is with the means available.",
            ),
            dimension(
                "budget-tech-fit",
                "Budget & technology fit",
                "How well the concept fits the stated budget and preferred technology.",
            ),
            dimension(
                "validation-readiness",
                "Validation readiness",
                "How ready the concept is for concrete validation steps.",
            ),
        ],
    }
}

/// Parse and bound a rubric document (untrusted input: it
/// arrives as an artifact body the user controls).
pub fn parse_rubric(body: &str) -> Result<RubricDoc, CommandError> {
    let doc: RubricDoc = serde_json::from_str(body)
        .map_err(|_| invalid("The rubric artifact does not contain a valid rubric document."))?;
    validate_rubric(&doc)?;
    Ok(doc)
}

pub fn validate_rubric(doc: &RubricDoc) -> Result<(), CommandError> {
    if doc.name.trim().is_empty() || doc.name.chars().count() > 120 {
        return Err(invalid("Give the rubric a name of up to 120 characters."));
    }
    if doc.dimensions.is_empty() || doc.dimensions.len() > 8 {
        return Err(invalid("A rubric has between 1 and 8 dimensions."));
    }
    let mut keys = std::collections::HashSet::new();
    let mut any_weight = false;
    for dim in &doc.dimensions {
        let key_ok = !dim.key.is_empty()
            && dim.key.chars().count() <= 40
            && dim
                .key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !key_ok || !keys.insert(dim.key.clone()) {
            return Err(invalid(
                "Rubric dimension keys are unique slugs of up to 40 characters (a-z, 0-9, -).",
            ));
        }
        if dim.label.trim().is_empty() || dim.label.chars().count() > 80 {
            return Err(invalid("Each rubric dimension needs a label of up to 80 characters."));
        }
        if dim.description.chars().count() > 300 {
            return Err(invalid("Rubric dimension descriptions are up to 300 characters."));
        }
        if !dim.weight.is_finite() || dim.weight < 0.0 || dim.weight > 1000.0 {
            return Err(invalid("Rubric weights are finite numbers between 0 and 1000."));
        }
        if dim.weight > 0.0 {
            any_weight = true;
        }
    }
    if !any_weight {
        return Err(invalid("At least one rubric dimension needs a positive weight."));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Deterministic summary

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssessmentInput {
    pub concept_index: u32,
    pub dimension: String,
    /// 1–5, or null/absent for "not assessed".
    #[serde(default)]
    pub score: Option<u8>,
    #[serde(default)]
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConceptSummary {
    pub concept_index: u32,
    /// Weighted mean over ASSESSED dimensions only; null when
    /// nothing was assessed (never zero-substituted).
    pub weighted_score: Option<f64>,
    pub assessed_dimensions: u32,
    pub total_dimensions: u32,
}

/// The slice's only computation. Deterministic: identical
/// rubric + assessments always produce identical summaries.
pub fn summarize(
    rubric: &RubricDoc,
    concept_indices: &[u32],
    assessments: &[AssessmentInput],
) -> Vec<ConceptSummary> {
    concept_indices
        .iter()
        .map(|&concept_index| {
            let mut weighted = 0.0f64;
            let mut weights = 0.0f64;
            let mut assessed = 0u32;
            for dim in &rubric.dimensions {
                if dim.weight <= 0.0 {
                    continue;
                }
                let entry = assessments.iter().find(|a| {
                    a.concept_index == concept_index && a.dimension == dim.key
                });
                if let Some(entry) = entry
                    && let Some(score) = entry.score
                {
                    weighted += f64::from(score) * dim.weight;
                    weights += dim.weight;
                    assessed += 1;
                }
            }
            ConceptSummary {
                concept_index,
                weighted_score: if weights > 0.0 {
                    Some((weighted / weights * 100.0).round() / 100.0)
                } else {
                    None
                },
                assessed_dimensions: assessed,
                total_dimensions: rubric
                    .dimensions
                    .iter()
                    .filter(|d| d.weight > 0.0)
                    .count() as u32,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------
// The stored evaluation document

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvaluationDoc {
    format: String,
    format_version: u8,
    source_artifact_id: String,
    source_version: u32,
    rubric_artifact_id: String,
    rubric_version: u32,
    /// The rubric exactly as used. Later rubric versions do
    /// not change this evaluation.
    rubric: RubricDoc,
    /// The selected concepts exactly as of the source version.
    concepts: Vec<IdeaConcept>,
    concept_indices: Vec<u32>,
    assessments: Vec<AssessmentInput>,
    summary: Vec<ConceptSummary>,
    note: String,
}

// ---------------------------------------------------------------------
// Save

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluationsSaveRequest {
    #[serde(default = "default_contract_version")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub source_artifact_id: String,
    pub source_version: u32,
    pub rubric_artifact_id: String,
    pub rubric_version: u32,
    pub concept_indices: Vec<u32>,
    #[serde(default)]
    pub assessments: Vec<AssessmentInput>,
}

fn default_contract_version() -> u8 {
    1
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationsSaveResponse {
    pub contract_version: u8,
    pub artifact: ArtifactRecord,
    pub body: String,
    pub already_saved: bool,
}

pub fn save_evaluation(
    storage: &Storage,
    req: &EvaluationsSaveRequest,
) -> Result<EvaluationsSaveResponse, CommandError> {
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    if req.concept_indices.is_empty() || req.concept_indices.len() > 3 {
        return Err(invalid("Compare between 1 and 3 concepts at a time."));
    }
    let mut indices = req.concept_indices.clone();
    indices.sort_unstable();
    if indices.windows(2).any(|w| w[0] == w[1]) {
        return Err(invalid("Select each concept at most once."));
    }

    // Workspace-scoped reads of the exact named versions;
    // both artifacts must belong to this project.
    let source = storage.artifact_read(
        &req.workspace_id,
        &req.source_artifact_id,
        Some(req.source_version),
    )?;
    if source.artifact.project_id != req.project_id {
        return Err(not_found());
    }
    if source.artifact.artifact_type != "idea" {
        return Err(invalid(
            "Evaluations start from a saved idea set artifact.",
        ));
    }
    let idea_set = parse_idea_set(&source.body, "").map_err(|e| invalid(&e))?;
    let rubric_source = storage.artifact_read(
        &req.workspace_id,
        &req.rubric_artifact_id,
        Some(req.rubric_version),
    )?;
    if rubric_source.artifact.project_id != req.project_id
        || rubric_source.artifact.artifact_type != "rubric"
    {
        return Err(not_found());
    }
    let rubric = parse_rubric(&rubric_source.body)?;

    let concepts: Vec<IdeaConcept> = indices
        .iter()
        .map(|&i| {
            idea_set
                .concepts
                .get(i as usize)
                .cloned()
                .ok_or_else(|| invalid("A selected concept does not exist in the source idea set."))
        })
        .collect::<Result<_, _>>()?;

    // Assessment validation: known concept + dimension, score
    // in range, rationale bounded, no duplicates. Missing
    // entries are simply "not assessed".
    let mut seen = std::collections::HashSet::new();
    for entry in &req.assessments {
        if !indices.contains(&entry.concept_index)
            || !rubric.dimensions.iter().any(|d| d.key == entry.dimension)
        {
            return Err(invalid(
                "An assessment names a concept or rubric dimension that is not part of this evaluation.",
            ));
        }
        if let Some(score) = entry.score
            && !(1..=5).contains(&score)
        {
            return Err(invalid("Scores are whole numbers from 1 to 5."));
        }
        if entry.rationale.chars().count() > 600 {
            return Err(invalid("Assessment rationales are up to 600 characters."));
        }
        if !seen.insert((entry.concept_index, entry.dimension.clone())) {
            return Err(invalid("Each concept and dimension can be assessed once."));
        }
    }
    let mut assessments = req.assessments.clone();
    assessments.sort_by(|a, b| {
        a.concept_index
            .cmp(&b.concept_index)
            .then_with(|| a.dimension.cmp(&b.dimension))
    });

    let summary = summarize(&rubric, &indices, &assessments);
    let doc = EvaluationDoc {
        format: "anansi-evaluation".to_string(),
        format_version: 1,
        source_artifact_id: req.source_artifact_id.clone(),
        source_version: req.source_version,
        rubric_artifact_id: req.rubric_artifact_id.clone(),
        rubric_version: req.rubric_version,
        rubric,
        concepts,
        concept_indices: indices,
        assessments,
        summary,
        note: "User-entered assessments under the rubric snapshot above. They are structured \
               judgement, not market evidence, verified demand, or a probability of success."
            .to_string(),
    };
    let body = serde_json::to_string(&doc).map_err(|_| unavailable())?;

    // Idempotency (the Slice 2 save lesson): an identical
    // evaluation — same canonical body, hence same content
    // hash — returns the existing artifact.
    let candidate_hash = format!("{:x}", Sha256::digest(body.as_bytes()));
    let existing = storage.artifact_list(&ArtifactListRequest {
        contract_version: 1,
        workspace_id: req.workspace_id.clone(),
        project_id: req.project_id.clone(),
    })?;
    for record in existing
        .artifacts
        .iter()
        .filter(|r| r.artifact_type == "evaluation")
    {
        let versions = storage.artifact_versions(&req.workspace_id, &record.artifact_id)?;
        for version in versions.versions {
            if version.content_hash == candidate_hash {
                let details =
                    storage.artifact_read(&req.workspace_id, &record.artifact_id, Some(version.version))?;
                return Ok(EvaluationsSaveResponse {
                    contract_version: 1,
                    artifact: details.artifact,
                    body: details.body,
                    already_saved: true,
                });
            }
        }
    }

    let title = {
        let t = format!("Evaluation: {}", source.artifact.title);
        if t.chars().count() > 200 {
            t.chars().take(200).collect::<String>()
        } else {
            t
        }
    };
    let details = storage.artifact_register_generated(
        &ArtifactRegisterRequest {
            contract_version: 1,
            workspace_id: req.workspace_id.clone(),
            project_id: req.project_id.clone(),
            artifact_type: "evaluation".to_string(),
            title,
            body: body.clone(),
            source_references: vec![
                format!(
                    "artifact://{}@v{}",
                    req.source_artifact_id, req.source_version
                ),
                format!(
                    "artifact://{}@v{}",
                    req.rubric_artifact_id, req.rubric_version
                ),
            ],
            privacy: "private".to_string(),
        },
        serde_json::json!({ "kind": "manual" }),
        "validated",
    )?;
    Ok(EvaluationsSaveResponse {
        contract_version: 1,
        artifact: details.artifact,
        body: details.body,
        already_saved: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assessment(concept: u32, dimension: &str, score: Option<u8>) -> AssessmentInput {
        AssessmentInput {
            concept_index: concept,
            dimension: dimension.to_string(),
            score,
            rationale: String::new(),
        }
    }

    #[test]
    fn standard_rubric_is_the_owner_approved_starting_point() {
        let rubric = standard_rubric();
        let keys: Vec<&str> = rubric.dimensions.iter().map(|d| d.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "problem-clarity",
                "customer-specificity",
                "mvp-feasibility",
                "budget-tech-fit",
                "validation-readiness"
            ]
        );
        assert!(
            rubric.dimensions.iter().all(|d| d.weight == 1.0),
            "initial weights are equal"
        );
        validate_rubric(&rubric).unwrap();
    }

    #[test]
    fn rubric_parser_enforces_bounds() {
        let good = serde_json::to_string(&standard_rubric()).unwrap();
        parse_rubric(&good).unwrap();
        assert!(parse_rubric("not json").is_err());
        assert!(parse_rubric("{}").is_err());
        // Duplicate keys refused.
        let mut dup = standard_rubric();
        dup.dimensions[1].key = dup.dimensions[0].key.clone();
        assert!(validate_rubric(&dup).is_err());
        // Non-slug key refused.
        let mut slug = standard_rubric();
        slug.dimensions[0].key = "Problem Clarity!".to_string();
        assert!(validate_rubric(&slug).is_err());
        // All-zero weights refused; out-of-range weights refused.
        let mut zero = standard_rubric();
        zero.dimensions.iter_mut().for_each(|d| d.weight = 0.0);
        assert!(validate_rubric(&zero).is_err());
        let mut heavy = standard_rubric();
        heavy.dimensions[0].weight = 1001.0;
        assert!(validate_rubric(&heavy).is_err());
        // 9 dimensions refused.
        let mut many = standard_rubric();
        many.dimensions.extend((0..4).map(|i| RubricDimension {
            key: format!("extra-{i}"),
            label: "X".to_string(),
            description: String::new(),
            weight: 1.0,
        }));
        assert!(validate_rubric(&many).is_err());
    }

    #[test]
    fn summarize_is_deterministic_and_never_zero_substitutes() {
        let rubric = standard_rubric();
        let full: Vec<AssessmentInput> = rubric
            .dimensions
            .iter()
            .enumerate()
            .map(|(i, d)| assessment(0, &d.key, Some([5, 4, 3, 4, 5][i])))
            .collect();
        let partial = vec![
            assessment(1, "problem-clarity", Some(5)),
            assessment(1, "customer-specificity", Some(5)),
            assessment(1, "mvp-feasibility", None), // recorded, unassessed
        ];
        let all = [full, partial].concat();
        let summaries = summarize(&rubric, &[0, 1, 2], &all);
        // Concept 0: (5+4+3+4+5)/5 = 4.2 over all five.
        assert_eq!(summaries[0].weighted_score, Some(4.2));
        assert_eq!(summaries[0].assessed_dimensions, 5);
        // Concept 1: only two assessed — the mean is over the
        // assessed dimensions only (5.0), NOT diluted by zeros.
        assert_eq!(summaries[1].weighted_score, Some(5.0));
        assert_eq!(summaries[1].assessed_dimensions, 2);
        assert_eq!(summaries[1].total_dimensions, 5);
        // Concept 2: nothing assessed — null, not 0.
        assert_eq!(summaries[2].weighted_score, None);
        assert_eq!(summaries[2].assessed_dimensions, 0);
        // Deterministic: same inputs, same outputs.
        assert_eq!(summaries, summarize(&rubric, &[0, 1, 2], &all));
    }

    #[test]
    fn summarize_respects_weights() {
        let mut rubric = standard_rubric();
        rubric.dimensions[0].weight = 3.0; // problem-clarity x3
        let entries = vec![
            assessment(0, "problem-clarity", Some(5)),
            assessment(0, "customer-specificity", Some(4)),
            assessment(0, "mvp-feasibility", Some(3)),
            assessment(0, "budget-tech-fit", Some(4)),
            assessment(0, "validation-readiness", Some(5)),
        ];
        let summaries = summarize(&rubric, &[0], &entries);
        // (5*3 + 4 + 3 + 4 + 5) / (3+1+1+1+1) = 31/7 = 4.43.
        assert_eq!(summaries[0].weighted_score, Some(4.43));
    }
}
