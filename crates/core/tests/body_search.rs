use anansi_core::{Core, ErrorCode};
use serde_json::{Value, json};

fn list_search(core: &Core, w: &str, query: &str, full_body: bool, extra: Value) -> Value {
    let mut r = json!({
        "contractVersion": 1,
        "workspaceId": w,
        "query": query,
        "fullBody": full_body
    });
    r.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    core.memory_command("list", r).unwrap()
}

#[test]
fn body_search_phrases_prefixes_revisions_rebuild_and_sealed_exclusion() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());

    let w = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Body Search Workspace"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let other_w = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Other Workspace"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // 1. Create records with diverse Markdown bodies across different kinds
    let doc1 = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "document",
                "draft": {
                    "title": "Quantum Foundations",
                    "body": "# Quantum Computing\nOverview of qubits.\n\n## Algorithm Architecture\nWe evaluate Shor's algorithm and Grover's search on fault-tolerant hardware.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let doc1_id = doc1["memory"]["id"].as_str().unwrap().to_string();

    let note1 = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "note",
                "draft": {
                    "title": "Meeting Notes",
                    "body": "# Weekly Sync\nDiscussion on release schedules.\n\n## Next Steps\nOptimize full-text search indexing with SQLite FTS5.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();

    // Record in another workspace - should be isolated
    core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": other_w,
            "kind": "document",
            "draft": {
                "title": "Other Quantum",
                "body": "Shor's algorithm details in foreign workspace.",
                "privacy": "private"
            }
        }),
    )
    .unwrap();

    // 2. Exact phrase search
    let phrase_res = list_search(&core, &w, "\"Shor's algorithm\"", true, json!({}));
    assert_eq!(phrase_res["memories"].as_array().unwrap().len(), 1);
    assert_eq!(phrase_res["memories"][0]["id"], doc1_id);
    assert_eq!(
        phrase_res["memories"][0]["sectionHeading"].as_str(),
        Some("## Algorithm Architecture")
    );
    assert_eq!(phrase_res["memories"][0]["lineNumber"].as_u64(), Some(5));
    assert!(
        phrase_res["memories"][0]["snippet"]
            .as_str()
            .unwrap()
            .contains("Shor's algorithm")
    );

    // 3. Prefix wildcard search
    let prefix_res = list_search(&core, &w, "fault*", true, json!({}));
    assert_eq!(prefix_res["memories"].as_array().unwrap().len(), 1);
    assert_eq!(prefix_res["memories"][0]["id"], doc1_id);

    // 4. Multi-word search
    let multi_res = list_search(&core, &w, "SQLite FTS5", true, json!({}));
    assert_eq!(multi_res["memories"].as_array().unwrap().len(), 1);
    assert_eq!(
        multi_res["memories"][0]["id"],
        note1["memory"]["id"].as_str().unwrap()
    );
    assert_eq!(
        multi_res["memories"][0]["sectionHeading"].as_str(),
        Some("## Next Steps")
    );
    assert_eq!(multi_res["memories"][0]["lineNumber"].as_u64(), Some(5));

    // 5. Sealed records must NEVER be indexed or matched in full-body search
    // Note: Creating Sealed memory directly without vault is refused by memory service.
    // If a request somehow had privacy 'sealed', it fails closed and is never indexed.
    let sealed_err = core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": w,
            "kind": "memory",
            "draft": {
                "title": "Sealed Secret",
                "body": "SuperSecretTokenThatMustNeverLeak",
                "privacy": "sealed"
            }
        }),
    );
    assert_eq!(sealed_err.unwrap_err().code, ErrorCode::VaultUnavailable);

    let secret_search = list_search(
        &core,
        &w,
        "SuperSecretTokenThatMustNeverLeak",
        true,
        json!({}),
    );
    assert!(secret_search["memories"].as_array().unwrap().is_empty());

    // 6. Revision update behavior:
    // When doc1 is revised, terms unique to revision 1 must no longer match;
    // terms in revision 2 must match!
    let rev1_id = doc1["memory"]["revisionId"].as_str().unwrap();
    core.memory_command(
        "revise",
        json!({
            "contractVersion": 1,
            "workspaceId": w,
            "id": doc1_id,
            "expectedRevision": rev1_id,
            "draft": {
                "title": "Quantum Foundations Revised",
                "body": "# Quantum Computing\nUpdated overview.\n\n## Advanced Architecture\nReplaced Shor with QuantumPhaseEstimation and adiabatic evolution.",
                "privacy": "private"
            }
        }),
    )
    .unwrap();

    // Searching for "Shor's algorithm" should now return 0 results!
    let old_search = list_search(&core, &w, "\"Shor's algorithm\"", true, json!({}));
    assert!(old_search["memories"].as_array().unwrap().is_empty());

    // Searching for "QuantumPhaseEstimation" should now match!
    let new_search = list_search(&core, &w, "QuantumPhaseEstimation", true, json!({}));
    assert_eq!(new_search["memories"].as_array().unwrap().len(), 1);
    assert_eq!(new_search["memories"][0]["id"], doc1_id);
    assert_eq!(new_search["memories"][0]["version"].as_u64(), Some(2));
    assert_eq!(
        new_search["memories"][0]["sectionHeading"].as_str(),
        Some("## Advanced Architecture")
    );

    // 7. Deduplication detection:
    // Create another document with the exact same body as doc1 revised
    let dup_doc = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "document",
                "draft": {
                    "title": "Quantum Duplicate",
                    "body": "# Quantum Computing\nUpdated overview.\n\n## Advanced Architecture\nReplaced Shor with QuantumPhaseEstimation and adiabatic evolution.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();
    let _dup_id = dup_doc["memory"]["id"].as_str().unwrap().to_string();

    let dup_search = list_search(&core, &w, "QuantumPhaseEstimation", true, json!({}));
    assert_eq!(dup_search["memories"].as_array().unwrap().len(), 2);
    // One of them must be flagged as duplicate of the other
    let has_duplicate_flag = dup_search["memories"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m.get("duplicateOf").and_then(|d| d.as_str()).is_some());
    assert!(
        has_duplicate_flag,
        "Exact duplicate content must be detected"
    );

    // 8. Rebuild search index:
    // Call rebuild_search_index and verify the index still works identically
    let indexed = core.rebuild_search_index(&w).unwrap();
    assert_eq!(indexed, 3); // note1, doc1, dup_doc

    let post_rebuild = list_search(&core, &w, "QuantumPhaseEstimation", true, json!({}));
    assert_eq!(post_rebuild["memories"].as_array().unwrap().len(), 2);

    // 9. Query validation bounds
    let err_too_long = core.memory_command(
        "list",
        json!({
            "contractVersion": 1,
            "workspaceId": w,
            "query": "a".repeat(513),
            "fullBody": true
        }),
    );
    assert_eq!(err_too_long.unwrap_err().code, ErrorCode::InvalidSearch);

    let err_empty = core.memory_command(
        "list",
        json!({
            "contractVersion": 1,
            "workspaceId": w,
            "query": "!@#$%^&*",
            "fullBody": true
        }),
    );
    assert_eq!(err_empty.unwrap_err().code, ErrorCode::InvalidSearch);
}
