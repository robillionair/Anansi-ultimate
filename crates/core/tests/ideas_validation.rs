//! M6 Slice 2 acceptance criterion 2: idea-set output
//! validation. Provider output is untrusted data — count
//! bounds, all twelve §21 fields, strict bounds, malformed and
//! refusal handling, fence tolerance, unknown-field dropping.

use anansi_core::ideas::parse_idea_set;
use serde_json::{Value, json};

fn concept(name: &str) -> Value {
    json!({
        "name": name,
        "problem": "A real problem worth solving.",
        "targetCustomer": "Independent cafes",
        "solution": "A focused tool that solves it.",
        "whyUseful": "It saves the customer time every week.",
        "differentiationHypothesis": "Hypothesis: incumbents ignore this segment.",
        "revenueModel": "Monthly subscription.",
        "engineeringComplexity": "medium",
        "implementationCost": "about 3-5 weeks for one developer",
        "keyRisks": ["Adoption may be slow"],
        "requiredValidation": "Interview five target customers.",
        "minimumViableVersion": "One workflow, end to end."
    })
}

fn set_of(n: usize) -> String {
    let concepts: Vec<Value> = (0..n).map(|i| concept(&format!("Idea {i}"))).collect();
    json!({ "concepts": concepts }).to_string()
}

#[test]
fn accepts_five_and_ten_and_truncates_eleven_with_a_note() {
    assert_eq!(parse_idea_set(&set_of(5), "t").unwrap().concepts.len(), 5);
    assert_eq!(parse_idea_set(&set_of(10), "t").unwrap().concepts.len(), 10);
    let truncated = parse_idea_set(&set_of(11), "t").unwrap();
    assert_eq!(truncated.concepts.len(), 10);
    assert!(truncated.note.unwrap().contains("11"));
    assert_eq!(truncated.topic, "t");
}

#[test]
fn rejects_too_few_malformed_and_prose_refusals() {
    assert!(parse_idea_set(&set_of(4), "t").unwrap_err().contains('4'));
    assert!(parse_idea_set("{not json", "t").is_err());
    assert!(
        parse_idea_set("I cannot help with that request.", "t")
            .unwrap_err()
            .contains("did not return a structured idea set")
    );
    assert!(parse_idea_set("{\"concepts\": {}}", "t").is_err());
}

#[test]
fn tolerates_code_fences_and_drops_unknown_fields() {
    let mut c = concept("Fenced");
    c["unexpectedField"] = json!("drop me");
    let raw = format!(
        "```json\n{}\n```",
        json!({ "concepts": [c, concept("B"), concept("C"), concept("D"), concept("E")] })
    );
    let set = parse_idea_set(&raw, "t").unwrap();
    assert_eq!(set.concepts.len(), 5);
    let encoded = serde_json::to_string(&set).unwrap();
    assert!(
        !encoded.contains("unexpectedField"),
        "unknown fields are dropped, never stored"
    );
}

#[test]
fn enforces_every_field_bound() {
    // Missing a §21 field.
    let mut missing = concept("X");
    missing
        .as_object_mut()
        .unwrap()
        .remove("requiredValidation");
    let raw =
        json!({ "concepts": [missing, concept("B"), concept("C"), concept("D"), concept("E")] })
            .to_string();
    assert!(
        parse_idea_set(&raw, "t")
            .unwrap_err()
            .contains("requiredValidation")
    );
    // Overlong name.
    let mut long = concept("X");
    long["name"] = json!("x".repeat(121));
    let raw = json!({ "concepts": [long, concept("B"), concept("C"), concept("D"), concept("E")] })
        .to_string();
    assert!(parse_idea_set(&raw, "t").is_err());
    // Empty problem.
    let mut empty = concept("X");
    empty["problem"] = json!("   ");
    let raw =
        json!({ "concepts": [empty, concept("B"), concept("C"), concept("D"), concept("E")] })
            .to_string();
    assert!(parse_idea_set(&raw, "t").is_err());
    // Risks: empty array refused, lone string normalized.
    let mut no_risks = concept("X");
    no_risks["keyRisks"] = json!([]);
    let raw =
        json!({ "concepts": [no_risks, concept("B"), concept("C"), concept("D"), concept("E")] })
            .to_string();
    assert!(parse_idea_set(&raw, "t").is_err());
    let mut string_risk = concept("X");
    string_risk["keyRisks"] = json!("one risk");
    let raw = json!({ "concepts": [string_risk, concept("B"), concept("C"), concept("D"), concept("E")] }).to_string();
    let set = parse_idea_set(&raw, "t").unwrap();
    assert_eq!(set.concepts[0].key_risks, vec!["one risk".to_string()]);
}
