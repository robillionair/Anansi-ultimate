use anansi_core::{Core, ErrorCode, storage::DATABASE_NAME};
use rusqlite::params;
use serde_json::{Value, json};

#[test]
fn body_search_corpus_limits_integrity_and_one_thousand_records() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let core = Core::with_storage_root(root.clone());

    let w = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Body limits fixture"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // 1. Create a baseline record
    let first = core
        .memory_command(
            "create",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "kind": "document",
                "draft": {
                    "title": "Quantum Limits Document",
                    "body": "# Chapter 1\nQuantum supremacy and decoherence mechanisms.",
                    "privacy": "private"
                }
            }),
        )
        .unwrap();

    let revision_id = first["memory"]["revisionId"].as_str().unwrap().to_string();
    let memory_id = first["memory"]["id"].as_str().unwrap().to_string();

    let search_body = |query: &str| {
        core.memory_command(
            "list",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "query": query,
                "fullBody": true
            }),
        )
    };

    // 2. Verify basic body search works
    let initial_res = search_body("decoherence").unwrap();
    assert_eq!(initial_res["memories"].as_array().unwrap().len(), 1);

    // 3. Body integrity verification:
    // Tamper with the Markdown file on disk so the hash no longer matches!
    let file_path = root
        .join("content")
        .join(&w)
        .join(&memory_id)
        .join(format!("{revision_id}.md"));
    let original_bytes = std::fs::read(&file_path).unwrap();
    std::fs::write(&file_path, b"# Tampered\nMalicious altered content").unwrap();

    // Body search must fail closed with ContentIntegrity
    let tampered_err = search_body("decoherence");
    assert_eq!(tampered_err.unwrap_err().code, ErrorCode::ContentIntegrity);

    // Rebuild index must also fail closed with ContentIntegrity and rollback
    let rebuild_err = core.rebuild_search_index(&w);
    assert_eq!(rebuild_err.unwrap_err().code, ErrorCode::ContentIntegrity);

    // Restore original file
    std::fs::write(&file_path, original_bytes).unwrap();
    assert!(search_body("decoherence").is_ok());

    // 4. Scale test with 1,000 records
    // Insert 1,000 distinct records in a single transaction
    let mut db = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
    let tx = db.transaction().unwrap();

    for n in 1..=1000 {
        let mem_id = format!("{n:032x}");
        let rev_id = format!("{n:032x}");
        let mut meta: Value = first["memory"].clone();
        meta["id"] = json!(mem_id);
        meta["revisionId"] = json!(rev_id);
        meta["title"] = json!(format!("Document Number {n}"));

        tx.execute(
            "INSERT INTO memories(workspace_id, id, current_revision) VALUES(?1, ?2, ?3)",
            params![w, mem_id, rev_id],
        )
        .unwrap();

        tx.execute(
            "INSERT INTO memory_revisions(workspace_id, memory_id, id, version, metadata, body_hash)
             VALUES(?1, ?2, ?3, 1, ?4, ?5)",
            params![w, mem_id, rev_id, meta.to_string(), "0".repeat(64)],
        )
        .unwrap();

        // Also insert into FTS5 directly
        let body = format!("# Section {n}\nComputational complexity analysis for index item {n}.");
        tx.execute(
            "INSERT INTO memory_body_fts(workspace_id, memory_id, revision_id, title, body)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![w, mem_id, rev_id, format!("Document Number {n}"), body],
        )
        .unwrap();
    }
    tx.commit().unwrap();

    // Write disk files using the canonical content::write helper for the top 3 items
    // so they have valid private permissions and SHA-256 hashes
    drop(db);
    for n in 1..=3 {
        let mem_id = format!("{n:032x}");
        let rev_id = format!("{n:032x}");
        let body = format!("# Section {n}\nComputational complexity analysis for index item {n}.");
        let hash = anansi_core::content::write(&root, &w, &mem_id, &rev_id, &body).unwrap();

        let db = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
        db.execute(
            "UPDATE memory_revisions SET body_hash=?1 WHERE memory_id=?2",
            params![hash, mem_id],
        )
        .unwrap();
    }

    // Query 1,000 records with limit 3
    let paged_res = core
        .memory_command(
            "list",
            json!({
                "contractVersion": 1,
                "workspaceId": w,
                "query": "complexity",
                "fullBody": true,
                "limit": 3,
                "offset": 0
            }),
        )
        .unwrap();

    let memories = paged_res["memories"].as_array().unwrap();
    assert_eq!(memories.len(), 3);
    assert_eq!(paged_res["nextOffset"].as_u64(), Some(3));
    assert!(
        memories[0]["snippet"]
            .as_str()
            .unwrap()
            .contains("complexity")
    );

    // 5. Overflow bounds check:
    // When corpus exceeds MAX_RECORDS (10,000), return InvalidSearch
    let mut db = rusqlite::Connection::open(root.join(DATABASE_NAME)).unwrap();
    let tx_overflow = db.transaction().unwrap();
    for n in 1..=10_001 {
        let mem_id = format!("over_{n:027x}");
        let rev_id = format!("over_{n:027x}");
        let mut meta: Value = first["memory"].clone();
        meta["id"] = json!(mem_id);
        meta["revisionId"] = json!(rev_id);
        meta["title"] = json!("Overflow item");

        tx_overflow
            .execute(
                "INSERT INTO memories(workspace_id, id, current_revision) VALUES(?1, ?2, ?3)",
                params![w, mem_id, rev_id],
            )
            .unwrap();
        tx_overflow
            .execute(
                "INSERT INTO memory_revisions(workspace_id, memory_id, id, version, metadata, body_hash)
                 VALUES(?1, ?2, ?3, 1, ?4, ?5)",
                params![w, mem_id, rev_id, meta.to_string(), "0".repeat(64)],
            )
            .unwrap();
        tx_overflow
            .execute(
                "INSERT INTO memory_body_fts(workspace_id, memory_id, revision_id, title, body)
                 VALUES(?1, ?2, ?3, ?4, ?5)",
                params![w, mem_id, rev_id, "Overflow item", "Overflow keyword match"],
            )
            .unwrap();
    }
    tx_overflow.commit().unwrap();
    drop(db);

    // Query matching over 10,000 records must return InvalidSearch
    let overflow_err = search_body("Overflow");
    assert_eq!(overflow_err.unwrap_err().code, ErrorCode::InvalidSearch);
}
