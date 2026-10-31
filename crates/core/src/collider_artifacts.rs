//! M8 §27 versioned experiment artifacts: six markdown
//! documents derived deterministically from a saved
//! Collider experiment document alone.
//!
//! Purity contract: derivation is a pure function of
//! the document — fixed templates plus verbatim record
//! fields, `\n` line endings, fixed section order, one
//! trailing newline. No provider calls, no timestamps,
//! no randomness, so the same saved version always
//! derives byte-identical documents, and their SHA-256
//! hashes are stable trust anchors.
//!
//! Honesty contract: these documents ORGANIZE the
//! record; they add no new claims. Every section the
//! assignment wants but the record does not state
//! (risk severity/likelihood, performance budgets,
//! module lists, interface definitions, phase
//! breakdowns) is written as "Not stated in the
//! source record", never as an invented value. The
//! header of every document says so. The TypeScript
//! mirror (`packages/ui/src/colliderArtifacts.ts`)
//! must produce byte-identical output; the shared
//! fixture hashes in both suites guard against drift.

use crate::collider::ColliderExperimentDoc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ARTIFACT_FILES: [&str; 6] = [
    "CONCEPT.md",
    "ARCHITECTURE.md",
    "FEASIBILITY.md",
    "ROADMAP.md",
    "RISKS.md",
    "TEST_PLAN.md",
];

const NOT_STATED: &str = "Not stated in the source record.";
const HEADER_NOTE: &str = "Derived from a saved Collider experiment record. This document organizes the record's own fields; it adds no new claims. Anything the record does not state is marked \"Not stated in the source record.\"";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ColliderArtifactHash {
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ColliderArtifacts {
    pub concept: ColliderArtifactHash,
    pub architecture: ColliderArtifactHash,
    pub feasibility: ColliderArtifactHash,
    pub roadmap: ColliderArtifactHash,
    pub risks: ColliderArtifactHash,
    pub test_plan: ColliderArtifactHash,
}

fn single_line(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn header(doc: &ColliderExperimentDoc, title: &str) -> Vec<String> {
    vec![
        format!("# {title}"),
        String::new(),
        format!(
            "{} (provider {}, model {}).",
            HEADER_NOTE,
            single_line(&doc.provider),
            single_line(&doc.model)
        ),
        String::new(),
    ]
}

fn bullets(items: &[String]) -> Vec<String> {
    if items.is_empty() {
        return vec!["None recorded.".to_string()];
    }
    items.iter().map(|item| format!("- {item}")).collect()
}

fn section(title: &str, body: Vec<String>) -> Vec<String> {
    let mut lines = vec![format!("## {title}"), String::new()];
    lines.extend(body);
    lines.push(String::new());
    lines
}

fn subsection(title: &str, body: Vec<String>) -> Vec<String> {
    let mut lines = vec![format!("### {title}"), String::new()];
    lines.extend(body);
    lines.push(String::new());
    lines
}

fn concept(doc: &ColliderExperimentDoc) -> String {
    let mut lines = header(doc, "Concept");
    lines.extend(section("Beams", vec![]));
    lines.extend(subsection("Beam A", vec![doc.beam_a.clone()]));
    lines.extend(subsection("Beam B", vec![doc.beam_b.clone()]));
    lines.extend(section(
        "Source understanding",
        vec![doc.result.source_understanding.clone()],
    ));
    lines.extend(section(
        "Domain intersections",
        bullets(&doc.result.cross_domain_mappings),
    ));
    lines.extend(section(
        "Conceptual primitives",
        bullets(&doc.result.conceptual_primitives),
    ));
    let mut combinations: Vec<String> = Vec::new();
    for combination in &doc.result.combinations {
        combinations.extend(subsection(
            &combination.name,
            vec![combination.description.clone()],
        ));
    }
    lines.extend(section("Proposed combinations", combinations));
    lines.extend(section(
        "Concept brief",
        vec![doc.result.productization.concept_brief.clone()],
    ));
    lines.extend(section(
        "Unique value proposition",
        vec![
            "The record states no separate value-proposition statement. The concept brief above is the record's own statement of value."
                .to_string(),
        ],
    ));
    finish(lines)
}

fn architecture(doc: &ColliderExperimentDoc) -> String {
    let mut lines = header(doc, "Architecture");
    lines.extend(section(
        "Technical architecture",
        vec![doc.result.productization.architecture.clone()],
    ));
    let mut blocks = bullets(&doc.result.conceptual_primitives);
    blocks.push(
        "Note: these are conceptual primitives from the record, not a module list."
            .to_string(),
    );
    lines.extend(section("Conceptual building blocks", blocks));
    lines.extend(section("Module decomposition", vec![NOT_STATED.to_string()]));
    lines.extend(section(
        "Technology stack options",
        vec![NOT_STATED.to_string()],
    ));
    lines.extend(section("Interfaces", vec![NOT_STATED.to_string()]));
    finish(lines)
}

fn feasibility(doc: &ColliderExperimentDoc) -> String {
    let f = &doc.result.feasibility;
    let mut lines = header(doc, "Feasibility");
    lines.extend(section("Hypotheses", bullets(&f.hypotheses)));
    lines.extend(section("Dependencies", bullets(&f.dependencies)));
    lines.extend(section("Unknown costs", bullets(&f.unknown_costs)));
    lines.extend(section("Risks", bullets(&f.risks)));
    lines.extend(section("Testability", vec![f.testability.clone()]));
    lines.extend(section(
        "Complexity hotspots",
        vec![
            "The record names no separate complexity hotspots. The dependencies above are the record's own complexity inputs."
                .to_string(),
        ],
    ));
    lines.extend(section(
        "Performance bottlenecks",
        vec![NOT_STATED.to_string()],
    ));
    lines.extend(section(
        "Mitigation strategies",
        vec![NOT_STATED.to_string()],
    ));
    finish(lines)
}

fn roadmap(doc: &ColliderExperimentDoc) -> String {
    let p = &doc.result.productization;
    let mut lines = header(doc, "Roadmap");
    lines.extend(section(
        "Delivery timeline",
        vec![p.roadmap.clone()],
    ));
    lines.extend(section(
        "Phases",
        vec![
            "The record states no separate phase breakdown; the delivery timeline above is the record's own timeline."
                .to_string(),
        ],
    ));
    lines.extend(section(
        "Critical milestones",
        vec![NOT_STATED.to_string()],
    ));
    lines.extend(section(
        "Dependencies",
        bullets(&doc.result.feasibility.dependencies),
    ));
    let mut inputs: Vec<String> = Vec::new();
    inputs.extend(subsection("Test plan", vec![p.test_plan.clone()]));
    inputs.extend(subsection(
        "Prototype proposal",
        vec![p.prototype_proposal.clone()],
    ));
    lines.extend(section("Delivery inputs", inputs));
    finish(lines)
}

fn risks(doc: &ColliderExperimentDoc) -> String {
    let mut lines = header(doc, "Risk register");
    let rows: Vec<String> = doc
        .result
        .feasibility
        .risks
        .iter()
        .map(|risk| {
            format!(
                "- {risk} — Severity: {NOT_STATED} Likelihood: {NOT_STATED} Mitigation: {NOT_STATED}"
            )
        })
        .collect();
    lines.extend(section("Risks", bullets(&rows)));
    let mut costs = bullets(&doc.result.feasibility.unknown_costs);
    costs.push(
        "These are uncertain costs from the record, not risks.".to_string(),
    );
    lines.extend(section("Uncertain costs", costs));
    lines.extend(section(
        "Risk categories",
        vec![
            "The record assigns no categories to its risks. The register above lists every recorded risk without invented categories."
                .to_string(),
        ],
    ));
    finish(lines)
}

fn test_plan(doc: &ColliderExperimentDoc) -> String {
    let p = &doc.result.productization;
    let mut lines = header(doc, "Test plan");
    let mut acceptance: Vec<String> = Vec::new();
    for combination in &doc.result.combinations {
        acceptance.extend(subsection(
            &format!("Acceptance: {}", combination.name),
            vec![
                combination.description.clone(),
                String::new(),
                format!("Pass criterion: {NOT_STATED}"),
            ],
        ));
    }
    lines.extend(section("Acceptance test specifications", acceptance));
    lines.extend(section("Test plan", vec![p.test_plan.clone()]));
    lines.extend(section(
        "Testability",
        vec![doc.result.feasibility.testability.clone()],
    ));
    lines.extend(section(
        "Edge cases",
        {
            let mut edge = vec![
                "Edge-case inputs from the record's own risks (each risk names a failure mode to test):"
                    .to_string(),
            ];
            edge.extend(bullets(&doc.result.feasibility.risks));
            edge
        },
    ));
    lines.extend(section(
        "Adversarial inputs",
        vec![NOT_STATED.to_string()],
    ));
    lines.extend(section(
        "Stress scenarios",
        vec![NOT_STATED.to_string()],
    ));
    lines.extend(section(
        "Performance budgets",
        vec![NOT_STATED.to_string()],
    ));
    finish(lines)
}

fn finish(lines: Vec<String>) -> String {
    // Exactly one trailing newline, regardless of
    // trailing newlines inside verbatim fields.
    let text = lines.join("\n");
    let trimmed = text.trim_end_matches('\n');
    format!("{trimmed}\n")
}

/// Derive one artifact by file name, or `None` for an
/// unknown name. The caller owns the name list.
pub fn derive_artifact(doc: &ColliderExperimentDoc, file: &str) -> Option<String> {
    match file {
        "CONCEPT.md" => Some(concept(doc)),
        "ARCHITECTURE.md" => Some(architecture(doc)),
        "FEASIBILITY.md" => Some(feasibility(doc)),
        "ROADMAP.md" => Some(roadmap(doc)),
        "RISKS.md" => Some(risks(doc)),
        "TEST_PLAN.md" => Some(test_plan(doc)),
        _ => None,
    }
}

/// Derive all six artifacts in `ARTIFACT_FILES` order.
pub fn derive_all(doc: &ColliderExperimentDoc) -> Vec<(String, String)> {
    ARTIFACT_FILES
        .iter()
        .filter_map(|file| derive_artifact(doc, file).map(|text| (file.to_string(), text)))
        .collect()
}

fn hash_entry(text: &str) -> ColliderArtifactHash {
    ColliderArtifactHash {
        sha256: crate::content::hash(text.as_bytes()),
        bytes: text.len(),
    }
}

/// Compute the stored artifact hashes for a document.
pub fn compute_artifacts(doc: &ColliderExperimentDoc) -> ColliderArtifacts {
    let mut concept = None;
    let mut architecture = None;
    let mut feasibility = None;
    let mut roadmap = None;
    let mut risks = None;
    let mut test_plan = None;
    for (file, text) in derive_all(doc) {
        let entry = hash_entry(&text);
        match file.as_str() {
            "CONCEPT.md" => concept = Some(entry),
            "ARCHITECTURE.md" => architecture = Some(entry),
            "FEASIBILITY.md" => feasibility = Some(entry),
            "ROADMAP.md" => roadmap = Some(entry),
            "RISKS.md" => risks = Some(entry),
            "TEST_PLAN.md" => test_plan = Some(entry),
            _ => {}
        }
    }
    ColliderArtifacts {
        concept: concept.unwrap(),
        architecture: architecture.unwrap(),
        feasibility: feasibility.unwrap(),
        roadmap: roadmap.unwrap(),
        risks: risks.unwrap(),
        test_plan: test_plan.unwrap(),
    }
}

/// Fail-closed recomputation check: every stored hash
/// and byte count must match a fresh derivation.
/// Absence is handled by the caller (legacy records
/// carry no field); presence with ANY mismatch is a
/// rejection, never a partial acceptance.
pub fn check_artifacts(
    doc: &ColliderExperimentDoc,
    recorded: &ColliderArtifacts,
) -> Result<(), String> {
    let computed = compute_artifacts(doc);
    let pairs = [
        ("CONCEPT.md", &computed.concept, &recorded.concept),
        (
            "ARCHITECTURE.md",
            &computed.architecture,
            &recorded.architecture,
        ),
        (
            "FEASIBILITY.md",
            &computed.feasibility,
            &recorded.feasibility,
        ),
        ("ROADMAP.md", &computed.roadmap, &recorded.roadmap),
        ("RISKS.md", &computed.risks, &recorded.risks),
        ("TEST_PLAN.md", &computed.test_plan, &recorded.test_plan),
    ];
    for (file, computed_entry, recorded_entry) in pairs {
        if computed_entry.sha256 != recorded_entry.sha256
            || computed_entry.bytes != recorded_entry.bytes
        {
            return Err(format!(
                "artifact {file} hash or byte count does not match the derived document"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod artifact_tests {
    use super::*;
    use crate::collider::{ColliderControls, ColliderResult};
    use serde_json::json;

    pub fn fixture_doc() -> ColliderExperimentDoc {
        let result: ColliderResult = serde_json::from_value(json!({
            "sourceUnderstanding": "Both beams describe bounded work.",
            "conceptualPrimitives": ["Bounded work", "Shared heat", "Timed orbit"],
            "crossDomainMappings": ["Bounded work maps to timed orbit", "Shared heat maps to bounded work"],
            "combinations": [
                { "name": "Orbit kitchen", "description": "A kitchen timed like an orbit." },
                { "name": "Heat timer", "description": "Heat applied on a schedule." }
            ],
            "feasibility": {
                "hypotheses": ["Timers help cooks", "Orbits explain timing"],
                "dependencies": ["A clock"],
                "unknownCosts": [],
                "risks": ["Burnt food"],
                "testability": "Cook one timed meal."
            },
            "productization": {
                "conceptBrief": "Brief text.",
                "architecture": "Architecture text.",
                "roadmap": "Roadmap text.",
                "testPlan": "Test plan text.",
                "prototypeProposal": "Prototype text."
            }
        }))
        .expect("fixture result parses");
        ColliderExperimentDoc {
            format: "anansi-collider-experiment".to_string(),
            version: 1,
            beam_a: "Beam A text about kitchens.".to_string(),
            beam_b: "Beam B text about orbitals.".to_string(),
            controls: ColliderControls {
                creativity: "balanced".to_string(),
                technical_depth: "standard".to_string(),
                practicality: "balanced".to_string(),
                innovation_distance: "moderate".to_string(),
                budget: String::new(),
                preferred_output_type: "concept".to_string(),
            },
            memory_context: vec![],
            project_context: None,
            document_a: None,
            document_b: None,
            url_a: None,
            url_b: None,
            result,
            provider: "test-provider".to_string(),
            model: "test-model".to_string(),
            disclosure: serde_json::from_value(json!({
                "provider": "test-provider",
                "model": "test-model",
                "endpointHost": "localhost",
                "local": true,
                "routingMode": "auto",
                "routingReason": "fixture",
                "messageCount": 1,
                "segmentCount": 1,
                "redactionCount": 0,
                "estimatedInputTokens": 10,
                "costClass": "free",
                "usage": null,
                "attempts": []
            }))
            .expect("fixture disclosure parses"),
            artifacts: None,
        }
    }

    #[test]
    fn derivation_is_deterministic_and_structured() {
        let doc = fixture_doc();
        let first = derive_all(&doc);
        let second = derive_all(&doc);
        assert_eq!(first, second);
        assert_eq!(first.len(), 6);
        for (file, text) in &first {
            assert!(text.ends_with('\n'), "{file}");
            assert!(!text.ends_with("\n\n"), "{file}");
            assert!(text.contains("Not stated in the source record."), "{file}");
            assert!(text.contains("test-provider"), "{file}");
        }
        // Hash vectors are pinned in
        // `derivation_hashes_match_the_typescript_mirror`
        // below, from values this suite prints.
        let hashes = compute_artifacts(&doc);
        assert_eq!(hashes.concept.bytes, derive_artifact(&doc, "CONCEPT.md").unwrap().len());
        check_artifacts(&doc, &hashes).expect("fresh hashes validate");
    }

    #[test]
    fn tampered_hashes_fail_closed() {
        let doc = fixture_doc();
        let mut hashes = compute_artifacts(&doc);
        hashes.risks.sha256 = "0".repeat(64);
        assert!(check_artifacts(&doc, &hashes).is_err());
        let mut hashes = compute_artifacts(&doc);
        hashes.roadmap.bytes += 1;
        assert!(check_artifacts(&doc, &hashes).is_err());
    }

    #[test]
    fn derivation_hashes_match_the_typescript_mirror() {
        // Pinned vectors: the TypeScript mirror asserts
        // these exact hashes for the same fixture. If
        // either side changes, this test and the TS test
        // disagree and the drift is visible.
        let doc = fixture_doc();
        let hashes = compute_artifacts(&doc);
        assert_eq!(hashes.concept.sha256, "e2cadb381eda099cd0c34ffd318847e5291fdb2b5d3e685502dd88a31299398c");
        assert_eq!(hashes.concept.bytes, 882);
        assert_eq!(hashes.architecture.sha256, "fbd19e41179b140e584a128ff3e2bce470a9735cce141ced119af2573069f007");
        assert_eq!(hashes.architecture.bytes, 628);
        assert_eq!(hashes.feasibility.sha256, "19aaa9410f39d5fc981307091a7bf4743169a94d1a6c3bd68c550f7bcd156481");
        assert_eq!(hashes.feasibility.bytes, 704);
        assert_eq!(hashes.roadmap.sha256, "9f53524f011d6621d87e77b9c719db5b07bcdd809151c473e31756c4d9b50c22");
        assert_eq!(hashes.roadmap.bytes, 589);
        assert_eq!(hashes.risks.sha256, "b9e4a8c213d0d7921651a5d8bdc410f6755bba5055fd73e09fef3c13060d3275");
        assert_eq!(hashes.risks.bytes, 657);
        assert_eq!(hashes.test_plan.sha256, "9a9726a6d2e41cfcd364ee3aee42a2531a295aa500b8acbd38091d57973516d5");
        assert_eq!(hashes.test_plan.bytes, 869);
    }
}
