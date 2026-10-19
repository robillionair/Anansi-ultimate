use anansi_core::Core;
use serde_json::{Value, json};

fn search_body(core: &Core, w: &str, query: &str, extra: Value) -> Value {
    let mut r = json!({
        "contractVersion": 1,
        "workspaceId": w,
        "query": query,
        "fullBody": true
    });
    r.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    core.memory_command("list", r).unwrap()
}

#[test]
fn retrieval_evaluation_suite_paraphrase_contradiction_temporal_and_missing_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("eval_store");
    let core = Core::with_storage_root(root);

    let w = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Evaluation Suite Workspace"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // =========================================================================
    // 1. BENCHMARK: Paraphrase Evaluation (Master Spec §11, §78)
    // =========================================================================
    // Document contains exact terminology.
    // Pure lexical search should match exact stems/tokens, and fail on pure synonyms
    // (documenting the measured lexical baseline that M4 vector search will augment).
    let para_doc = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "document",
                "draft": {
                    "title": "Autonomous Vehicle Perception",
                    "body": "# Computer Vision\nLiDAR and camera sensor fusion detects pedestrians and lane boundaries with high accuracy.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let para_id = para_doc["memory"]["id"].as_str().unwrap();

    // Exact lexical match:
    let exact_res = search_body(&core, &w, "sensor fusion", json!({}));
    assert_eq!(exact_res["memories"].as_array().unwrap().len(), 1);
    assert_eq!(exact_res["memories"][0]["id"], para_id);

    // Paraphrase / synonym test ("automated driving sight" instead of "Autonomous Vehicle Perception" / "sensor fusion"):
    // pure lexical search must return 0 results without hallucinating, proving lexical boundaries.
    let synonym_res = search_body(&core, &w, "\"automated driving sight\"", json!({}));
    assert!(
        synonym_res["memories"].as_array().unwrap().is_empty(),
        "Pure lexical FTS5 must not hallucinate matches for unindexed synonyms (M4 baseline)"
    );

    // =========================================================================
    // 2. BENCHMARK: Contradiction Handling & Recency Resolution (Master Spec §78)
    // =========================================================================
    // Two records make conflicting statements regarding a project milestone deadline.
    // Record A has an older update date; Record B has a newer update date.
    // Lexical search for the topic matches both, but Record B must rank first.
    let old_fact = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "decision",
                "draft": {
                    "title": "Launch Target Decision Alpha",
                    "body": "# Launch Target\nThe product launch date is scheduled for March 15, 2026.",
                    "eventDate": "2026-01-10T00:00:00Z",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let old_fact_id = old_fact["memory"]["id"].as_str().unwrap().to_string();

    let new_fact = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "decision",
                "draft": {
                    "title": "Launch Target Decision Beta",
                    "body": "# Launch Target\nThe product launch date is postponed to October 25, 2026 due to component delays.",
                    "eventDate": "2026-09-15T00:00:00Z",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let new_fact_id = new_fact["memory"]["id"].as_str().unwrap().to_string();

    let contra_res = search_body(&core, &w, "\"product launch date\"", json!({}));
    let contra_list = contra_res["memories"].as_array().unwrap();
    assert_eq!(contra_list.len(), 2);
    // The newer fact (Decision Beta, September event date) must be ranked ahead of Decision Alpha
    assert_eq!(
        contra_list[0]["id"], new_fact_id,
        "Newer fact must rank before older contradicting statement"
    );
    assert_eq!(contra_list[1]["id"], old_fact_id);

    // =========================================================================
    // 3. BENCHMARK: Temporal Facts & Validity Windows (Master Spec §11)
    // =========================================================================
    // Expired records must be excluded by default and included only when requested.
    let expired_doc = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "note",
                "draft": {
                    "title": "Temporary Access Token Protocol",
                    "body": "# Security Protocol\nTemporary credentials expire automatically on 2020-01-01.",
                    "validUntil": "2020-01-01T00:00:00Z",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let exp_id = expired_doc["memory"]["id"].as_str().unwrap();

    // Default search excludes expired records
    let default_temporal = search_body(&core, &w, "\"Temporary credentials\"", json!({}));
    assert!(default_temporal["memories"].as_array().unwrap().is_empty());

    // Explicit includeExpired includes the record
    let with_expired = search_body(
        &core,
        &w,
        "\"Temporary credentials\"",
        json!({"includeExpired": true}),
    );
    assert_eq!(with_expired["memories"].as_array().unwrap().len(), 1);
    assert_eq!(with_expired["memories"][0]["id"], exp_id);

    // =========================================================================
    // 4. BENCHMARK: Missing Evidence & Negative Assertions (Master §11, §78)
    // =========================================================================
    // Querying for completely absent topics or contradictory facts absent from corpus
    // must yield truthful empty results with zero false positives.
    let missing_res = search_body(&core, &w, "HyperdriveWarpEngineUnobtainium", json!({}));
    assert_eq!(missing_res["memories"].as_array().unwrap().len(), 0);
    assert!(missing_res["nextOffset"].is_null());

    // =========================================================================
    // 5. BENCHMARK: Grounded Citation Accuracy (Master Spec §11, §78)
    // =========================================================================
    // Evidence location must accurately extract the nearest enclosing Markdown heading
    // and exact 1-based line number.
    let citation_doc = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "document",
                "draft": {
                    "title": "System Architecture Specification",
                    "body": "# 1. Overview\nHigh-level system goals.\n\n# 2. Subsystems\nDiscussion of core components.\n\n## 2.1 Search Engine\nFull-text indexing employs SQLite FTS5 with BM25 ranking.\n\n## 2.2 Storage Engine\nSQLite with WAL journal mode.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let cite_id = citation_doc["memory"]["id"].as_str().unwrap();

    let cite_res = search_body(&core, &w, "\"BM25 ranking\"", json!({}));
    assert_eq!(cite_res["memories"].as_array().unwrap().len(), 1);
    let matched_item = &cite_res["memories"][0];
    assert_eq!(matched_item["id"], cite_id);
    assert_eq!(
        matched_item["sectionHeading"].as_str(),
        Some("## 2.1 Search Engine")
    );
    assert_eq!(matched_item["lineNumber"].as_u64(), Some(8));
    assert!(
        matched_item["snippet"]
            .as_str()
            .unwrap()
            .contains("BM25 ranking")
    );
}
