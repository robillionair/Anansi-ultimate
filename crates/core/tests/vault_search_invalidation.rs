use anansi_core::vault::policy::SearchBoundary;
use rusqlite::Connection;

fn connection() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE VIRTUAL TABLE memory_body_fts USING fts5(workspace_id UNINDEXED, memory_id UNINDEXED, revision_id UNINDEXED, title, body);",
    )
    .unwrap();
    db.execute(
        "INSERT INTO memory_body_fts(workspace_id,memory_id,revision_id,title,body) VALUES ('w1','r1','v1','title','secret one')",
        [],
    ).unwrap();
    db.execute(
        "INSERT INTO memory_body_fts(workspace_id,memory_id,revision_id,title,body) VALUES ('w1','r2','v1','title','secret two')",
        [],
    ).unwrap();
    db.execute(
        "INSERT INTO memory_body_fts(workspace_id,memory_id,revision_id,title,body) VALUES ('w2','r3','v1','title','other')",
        [],
    ).unwrap();
    db
}

#[test]
fn record_invalidation_removes_only_target_plaintext_rows() {
    let db = connection();
    assert_eq!(
        SearchBoundary::invalidate_record(&db, "w1", "r1").unwrap(),
        1
    );
    let remaining: i64 = db
        .query_row(
            "SELECT count(*) FROM memory_body_fts WHERE workspace_id='w1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 1);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM memory_body_fts WHERE memory_id='r1'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn workspace_invalidation_removes_all_rows_without_touching_other_workspaces() {
    let db = connection();
    assert_eq!(SearchBoundary::invalidate_workspace(&db, "w1").unwrap(), 2);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM memory_body_fts WHERE workspace_id='w1'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM memory_body_fts WHERE workspace_id='w2'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}
