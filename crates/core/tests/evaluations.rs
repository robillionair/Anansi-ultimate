//! M6 Slice 3 integration: transparent concept evaluation —
//! user-entered assessments under a versioned rubric, saved
//! as an immutable evaluation artifact linked to the exact
//! source/rubric versions, with deterministic summaries,
//! idempotent saves, and history that rubric changes never
//! rewrite.

use anansi_core::{Core, ErrorCode};
use serde_json::{Value, json};

fn draft(title: &str, body: &str) -> Value {
    json!({ "title": title, "body": body, "privacy": "private",
            "eventDate": null, "validFrom": null, "validUntil": null,
            "confidence": null, "sourceReferences": [] })
}

fn code(result: Result<Value, anansi_core::CommandError>) -> ErrorCode {
    result.unwrap_err().code
}

fn concept(name: &str) -> Value {
    json!({
        "name": name, "problem": "A problem.", "targetCustomer": "Cafes",
        "solution": "A tool.", "whyUseful": "Saves time.",
        "differentiationHypothesis": "A hypothesis.", "revenueModel": "Subscription.",
        "engineeringComplexity": "medium", "implementationCost": "3-5 weeks",
        "keyRisks": ["Adoption"], "requiredValidation": "Interviews.",
        "minimumViableVersion": "One workflow."
    })
}

struct Fixture {
    _dir: tempfile::TempDir,
    core: Core,
    ws: String,
    project: String,
    idea_artifact: String,
    rubric_artifact: String,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(dir.path().to_path_buf());
    let ws = core
        .workspace_command("create", json!({ "contractVersion": 1, "name": "Eval" }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let project = core
        .memory_command(
            "create",
            json!({ "contractVersion": 1, "workspaceId": ws, "kind": "project", "draft": draft("Harbor App", "project") }),
        )
        .unwrap()["memory"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // A saved idea set (four concepts), as Slice 2 stores it.
    let idea_body = json!({
        "topic": "harbor logistics",
        "concepts": [concept("Alpha"), concept("Beta"), concept("Gamma"), concept("Delta"), concept("Epsilon")]
    })
    .to_string();
    let idea = core
        .artifacts_command(
            "register",
            json!({ "contractVersion": 1, "workspaceId": ws, "projectId": project,
                    "type": "idea", "title": "Idea set: harbor logistics", "body": idea_body,
                    "sourceReferences": [], "privacy": "private" }),
        )
        .unwrap();
    // The rubric, from the single standard definition in core.
    let standard = core
        .artifacts_command(
            "evaluations-standard-rubric",
            json!({ "contractVersion": 1, "workspaceId": ws, "projectId": project }),
        )
        .unwrap();
    let rubric = core
        .artifacts_command(
            "register",
            json!({ "contractVersion": 1, "workspaceId": ws, "projectId": project,
                    "type": "rubric", "title": "Standard evaluation rubric",
                    "body": standard["rubric"].to_string(),
                    "sourceReferences": [], "privacy": "private" }),
        )
        .unwrap();
    Fixture {
        _dir: dir,
        core,
        ws,
        project,
        idea_artifact: idea["artifact"]["artifactId"].as_str().unwrap().to_string(),
        rubric_artifact: rubric["artifact"]["artifactId"].as_str().unwrap().to_string(),
    }
}

fn full_assessments() -> Value {
    json!([
        { "conceptIndex": 0, "dimension": "problem-clarity", "score": 5, "rationale": "Sharp." },
        { "conceptIndex": 0, "dimension": "customer-specificity", "score": 4, "rationale": "" },
        { "conceptIndex": 0, "dimension": "mvp-feasibility", "score": 3, "rationale": "" },
        { "conceptIndex": 0, "dimension": "budget-tech-fit", "score": 4, "rationale": "" },
        { "conceptIndex": 0, "dimension": "validation-readiness", "score": 5, "rationale": "" },
        { "conceptIndex": 2, "dimension": "problem-clarity", "score": 5, "rationale": "" },
        { "conceptIndex": 2, "dimension": "customer-specificity", "score": 5, "rationale": "" }
    ])
}

fn save(f: &Fixture, extra: Value) -> Result<Value, anansi_core::CommandError> {
    let mut request = json!({
        "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project,
        "sourceArtifactId": f.idea_artifact, "sourceVersion": 1,
        "rubricArtifactId": f.rubric_artifact, "rubricVersion": 1,
        "conceptIndices": [0, 2], "assessments": full_assessments()
    });
    if let (Some(target), Some(patch)) = (request.as_object_mut(), extra.as_object()) {
        for (k, v) in patch {
            target.insert(k.clone(), v.clone());
        }
    }
    f.core.artifacts_command("evaluations-save", request)
}

#[test]
fn evaluation_save_embeds_snapshot_summary_and_provenance() {
    let f = fixture();
    let saved = save(&f, json!({})).unwrap();
    assert_eq!(saved["alreadySaved"], false);
    let artifact = &saved["artifact"];
    assert_eq!(artifact["type"], "evaluation");
    assert_eq!(artifact["generator"], json!({ "kind": "manual" }));
    assert_eq!(artifact["validationStatus"], "validated");
    assert_eq!(
        artifact["sourceReferences"],
        json!([
            format!("artifact://{}@v1", f.idea_artifact),
            format!("artifact://{}@v1", f.rubric_artifact)
        ])
    );
    let body: Value = serde_json::from_str(saved["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["format"], "anansi-evaluation");
    assert_eq!(body["rubric"]["name"], "Standard evaluation rubric");
    assert_eq!(body["concepts"][0]["name"], "Alpha");
    assert_eq!(body["concepts"][1]["name"], "Gamma");
    // Concept 0 fully assessed: (5+4+3+4+5)/5 = 4.2.
    assert_eq!(body["summary"][0]["weightedScore"], 4.2);
    assert_eq!(body["summary"][0]["assessedDimensions"], 5);
    // Concept 2 partially assessed: missing dimensions are
    // excluded, never zero-substituted.
    assert_eq!(body["summary"][1]["weightedScore"], 5.0);
    assert_eq!(body["summary"][1]["assessedDimensions"], 2);
    assert_eq!(body["summary"][1]["totalDimensions"], 5);
    assert!(
        body["note"].as_str().unwrap().contains("not market evidence"),
        "the honesty note is stored with the document"
    );
}

#[test]
fn identical_save_is_idempotent_and_rubric_revision_never_rewrites_history() {
    let f = fixture();
    let first = save(&f, json!({})).unwrap();
    let again = save(&f, json!({})).unwrap();
    assert_eq!(again["alreadySaved"], true);
    assert_eq!(
        again["artifact"]["artifactId"],
        first["artifact"]["artifactId"]
    );
    let list = f
        .core
        .artifacts_command(
            "list",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "projectId": f.project }),
        )
        .unwrap();
    let evaluations = list["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["type"] == "evaluation")
        .count();
    assert_eq!(evaluations, 1, "no duplicate evaluation artifact");

    // Revise the rubric (problem-clarity weight 3): history
    // keeps the v1 snapshot; a new evaluation embeds v2 and
    // recomputes.
    let rubric_read = f
        .core
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": f.rubric_artifact, "version": 1 }),
        )
        .unwrap();
    let mut rubric_doc: Value = serde_json::from_str(rubric_read["body"].as_str().unwrap()).unwrap();
    rubric_doc["dimensions"][0]["weight"] = json!(3.0);
    f.core
        .artifacts_command(
            "revise",
            json!({ "contractVersion": 1, "workspaceId": f.ws, "artifactId": f.rubric_artifact,
                    "expectedVersion": 1, "title": "Standard evaluation rubric",
                    "body": rubric_doc.to_string(), "sourceReferences": [] }),
        )
        .unwrap();
    let historical = f
        .core
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws,
                    "artifactId": first["artifact"]["artifactId"], "version": 1 }),
        )
        .unwrap();
    let historical_body: Value =
        serde_json::from_str(historical["body"].as_str().unwrap()).unwrap();
    assert_eq!(historical_body["rubricVersion"], 1);
    assert_eq!(
        historical_body["rubric"]["dimensions"][0]["weight"], 1.0,
        "the saved evaluation keeps the rubric as used"
    );
    assert_eq!(historical_body["summary"][0]["weightedScore"], 4.2);

    let rescored = save(&f, json!({ "rubricVersion": 2 })).unwrap();
    assert_eq!(rescored["alreadySaved"], false);
    let body: Value = serde_json::from_str(rescored["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["rubricVersion"], 2);
    // (5*3 + 4 + 3 + 4 + 5) / 7 = 4.43.
    assert_eq!(body["summary"][0]["weightedScore"], 4.43);
}

#[test]
fn refusals_bounds_isolation_archive_and_restart() {
    let f = fixture();
    // Selection bounds.
    assert_eq!(
        code(save(&f, json!({ "conceptIndices": [0, 1, 2, 3] }))),
        ErrorCode::InvalidRequest,
        "at most 3 concepts"
    );
    assert_eq!(
        code(save(&f, json!({ "conceptIndices": [9] }))),
        ErrorCode::InvalidRequest,
        "a concept index outside the source set is refused"
    );
    // Score bounds + unknown dimension.
    let mut bad_score = full_assessments();
    bad_score[0]["score"] = json!(6);
    assert_eq!(
        code(save(&f, json!({ "assessments": bad_score }))),
        ErrorCode::InvalidRequest
    );
    let mut bad_dim = full_assessments();
    bad_dim[0]["dimension"] = json!("market-size");
    assert_eq!(
        code(save(&f, json!({ "assessments": bad_dim }))),
        ErrorCode::InvalidRequest
    );
    // Nonexistent versions and cross-type references.
    assert_eq!(
        code(save(&f, json!({ "sourceVersion": 99 }))),
        ErrorCode::NotFound
    );
    assert_eq!(
        code(save(
            &f,
            json!({ "rubricArtifactId": f.idea_artifact.clone() })
        )),
        ErrorCode::NotFound,
        "the idea artifact is not a rubric"
    );
    // Cross-workspace: reads are workspace-scoped, so ws A's
    // artifact ids are invisible from ws B.
    let ws_b = f
        .core
        .workspace_command("create", json!({ "contractVersion": 1, "name": "Other" }))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let foreign = f.core.artifacts_command(
        "evaluations-save",
        json!({ "contractVersion": 1, "workspaceId": ws_b, "projectId": f.project,
                "sourceArtifactId": f.idea_artifact, "sourceVersion": 1,
                "rubricArtifactId": f.rubric_artifact, "rubricVersion": 1,
                "conceptIndices": [0], "assessments": [] }),
    );
    assert_eq!(code(foreign), ErrorCode::NotFound);

    // Archived projects refuse evaluation saves (the
    // registry's write rule, inherited unchanged).
    let f2 = fixture();
    let project = f2
        .core
        .memory_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f2.ws, "id": f2.project }),
        )
        .unwrap();
    let rev = project["memory"]["revisionId"].as_str().unwrap().to_string();
    f2.core
        .memory_command(
            "revise",
            json!({ "contractVersion": 1, "workspaceId": f2.ws, "id": f2.project,
                    "expectedRevision": rev,
                    "draft": draft("Harbor App", "project"),
                    "lifecycle": { "state": "archived", "confirmed": true, "successorId": null } }),
        )
        .unwrap();
    assert_eq!(
        code(save(&f2, json!({}))),
        ErrorCode::InvalidRequest,
        "archived project refuses the evaluation save"
    );

    // Restart: a fresh Core reopens the saved evaluation.
    let saved = save(&f, json!({})).unwrap();
    let reopened = Core::with_storage_root(f._dir.path().to_path_buf());
    let read = reopened
        .artifacts_command(
            "read",
            json!({ "contractVersion": 1, "workspaceId": f.ws,
                    "artifactId": saved["artifact"]["artifactId"], "version": 1 }),
        )
        .unwrap();
    assert_eq!(read["body"], saved["body"]);
    assert_eq!(read["artifact"]["contentHash"], saved["artifact"]["contentHash"]);
}
