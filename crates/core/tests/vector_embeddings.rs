use anansi_core::vector::{
    EmbeddingVector, VectorError, delete_embeddings_for_memory, generate_ollama_embedding,
    get_embedding, list_embeddings_for_workspace, store_embedding, validate_loopback_endpoint,
};
use rusqlite::Connection;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
};

#[test]
fn vector_math_and_dimensions_validation() {
    // 1. Empty vector rejected
    assert!(matches!(
        EmbeddingVector::new(vec![]),
        Err(VectorError::EmptyVector)
    ));

    // 2. <2 dimensions rejected
    assert!(matches!(
        EmbeddingVector::new(vec![1.0]),
        Err(VectorError::InvalidDimensions {
            min: 2,
            max: 1536,
            actual: 1
        })
    ));

    // 3. >1536 dimensions rejected
    let too_large = vec![0.1; 1537];
    assert!(matches!(
        EmbeddingVector::new(too_large),
        Err(VectorError::InvalidDimensions {
            min: 2,
            max: 1536,
            actual: 1537
        })
    ));

    // 4. Non-finite values rejected
    assert!(matches!(
        EmbeddingVector::new(vec![1.0, f32::NAN]),
        Err(VectorError::NonFiniteValue)
    ));
    assert!(matches!(
        EmbeddingVector::new(vec![1.0, f32::INFINITY]),
        Err(VectorError::NonFiniteValue)
    ));

    // 5. Valid vector norm & normalization
    let v = EmbeddingVector::new(vec![3.0, 4.0]).unwrap();
    assert_eq!(v.dimension(), 2);
    assert!((v.l2_norm() - 5.0).abs() < 1e-6);

    let norm_v = v.normalize().unwrap();
    assert!((norm_v.l2_norm() - 1.0).abs() < 1e-6);
    assert!((norm_v.as_slice()[0] - 0.6).abs() < 1e-6);
    assert!((norm_v.as_slice()[1] - 0.8).abs() < 1e-6);

    // 6. Cosine similarity
    let v1 = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
    let v2 = EmbeddingVector::new(vec![0.0, 1.0]).unwrap();
    let v3 = EmbeddingVector::new(vec![-1.0, 0.0]).unwrap();
    let v4 = EmbeddingVector::new(vec![2.0, 0.0]).unwrap();

    // Orthogonal: 0.0
    assert!((v1.cosine_similarity(&v2).unwrap() - 0.0).abs() < 1e-6);
    // Opposite: -1.0
    assert!((v1.cosine_similarity(&v3).unwrap() - (-1.0)).abs() < 1e-6);
    // Same direction: 1.0
    assert!((v1.cosine_similarity(&v4).unwrap() - 1.0).abs() < 1e-6);

    // Dimension mismatch
    let v_3d = EmbeddingVector::new(vec![1.0, 2.0, 3.0]).unwrap();
    assert!(matches!(
        v1.cosine_similarity(&v_3d),
        Err(VectorError::DimensionMismatch {
            expected: 2,
            actual: 3
        })
    ));
}

#[test]
fn vector_serialization_deserialization_roundtrip() {
    let original = EmbeddingVector::new(vec![0.1234, -0.5678, 12.3456, -99.999]).unwrap();
    let bytes = original.to_bytes();
    assert_eq!(bytes.len(), 4 * 4);

    let reconstructed = EmbeddingVector::from_bytes(&bytes).unwrap();
    assert_eq!(original, reconstructed);

    // Malformed byte slices
    assert!(matches!(
        EmbeddingVector::from_bytes(&[]),
        Err(VectorError::EmptyVector)
    ));
    assert!(matches!(
        EmbeddingVector::from_bytes(&[1, 2, 3]),
        Err(VectorError::DeserializationError(_))
    ));
}

#[test]
fn sqlite_embedding_cache_roundtrip_and_cascade() {
    let conn = Connection::open_in_memory().unwrap();

    // Create canonical memories and memory_revisions tables
    conn.execute_batch(
        "CREATE TABLE memories (
            workspace_id TEXT NOT NULL,
            id TEXT NOT NULL,
            current_revision TEXT NOT NULL DEFAULT 'rev_1',
            PRIMARY KEY (workspace_id, id)
        );
        CREATE TABLE memory_revisions (
            workspace_id TEXT NOT NULL,
            memory_id TEXT NOT NULL,
            id TEXT NOT NULL,
            metadata TEXT NOT NULL
        );",
    )
    .unwrap();

    // Insert sample memory
    conn.execute(
        "INSERT INTO memories (workspace_id, id, current_revision) VALUES ('ws_1', 'mem_1', 'rev_1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO memory_revisions (workspace_id, memory_id, id, metadata) VALUES ('ws_1', 'mem_1', 'rev_1', '{\"privacy\":\"private\",\"status\":\"active\"}')",
        [],
    )
    .unwrap();

    let vec_chunk0 = EmbeddingVector::new(vec![0.1, 0.2, 0.3, 0.4]).unwrap();
    let vec_chunk1 = EmbeddingVector::new(vec![0.5, 0.6, 0.7, 0.8]).unwrap();

    // Store chunk 0
    store_embedding(&conn, "ws_1", "mem_1", 0, "nomic-embed-text", &vec_chunk0).unwrap();
    // Store chunk 1
    store_embedding(&conn, "ws_1", "mem_1", 1, "nomic-embed-text", &vec_chunk1).unwrap();

    // Retrieve chunk 0
    let retrieved0 = get_embedding(&conn, "ws_1", "mem_1", 0, "nomic-embed-text")
        .unwrap()
        .expect("chunk 0 should exist");
    assert_eq!(retrieved0, vec_chunk0);

    // Retrieve chunk 1
    let retrieved1 = get_embedding(&conn, "ws_1", "mem_1", 1, "nomic-embed-text")
        .unwrap()
        .expect("chunk 1 should exist");
    assert_eq!(retrieved1, vec_chunk1);

    // Non-existent chunk
    let non_existent = get_embedding(&conn, "ws_1", "mem_1", 99, "nomic-embed-text").unwrap();
    assert!(non_existent.is_none());

    // List embeddings
    let list = list_embeddings_for_workspace(&conn, "ws_1", "nomic-embed-text").unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].0, "mem_1");
    assert_eq!(list[0].1, 0);
    assert_eq!(list[1].0, "mem_1");
    assert_eq!(list[1].1, 1);

    // Delete embeddings for memory
    delete_embeddings_for_memory(&conn, "ws_1", "mem_1").unwrap();
    let list_after_delete =
        list_embeddings_for_workspace(&conn, "ws_1", "nomic-embed-text").unwrap();
    assert_eq!(list_after_delete.len(), 0);
}

#[test]
fn fail_closed_sealed_memory_exclusion() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE memories (
            workspace_id TEXT NOT NULL,
            id TEXT NOT NULL,
            current_revision TEXT NOT NULL DEFAULT 'rev_1',
            PRIMARY KEY (workspace_id, id)
        );
        CREATE TABLE memory_revisions (
            workspace_id TEXT NOT NULL,
            memory_id TEXT NOT NULL,
            id TEXT NOT NULL,
            metadata TEXT NOT NULL
        );",
    )
    .unwrap();

    // Insert sealed memory
    conn.execute(
        "INSERT INTO memories (workspace_id, id, current_revision) VALUES ('ws_vault', 'mem_vault_secret', 'rev_1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO memory_revisions (workspace_id, memory_id, id, metadata) VALUES ('ws_vault', 'mem_vault_secret', 'rev_1', '{\"privacy\":\"sealed\",\"status\":\"active\"}')",
        [],
    )
    .unwrap();

    let vec = EmbeddingVector::new(vec![0.1, 0.2, 0.3, 0.4]).unwrap();

    // Storing embedding for sealed memory MUST fail closed
    let result = store_embedding(
        &conn,
        "ws_vault",
        "mem_vault_secret",
        0,
        "nomic-embed-text",
        &vec,
    );
    assert!(matches!(result, Err(VectorError::SealedRecordRefused)));

    // Verify 0 rows stored
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM memory_embeddings WHERE memory_id = 'mem_vault_secret'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn loopback_validation_and_synthetic_server() {
    // 1. Loopback validation
    assert!(validate_loopback_endpoint("http://127.0.0.1:11434").is_ok());
    assert!(validate_loopback_endpoint("http://localhost:11434").is_ok());
    assert!(validate_loopback_endpoint("127.0.0.1:8080").is_ok());

    assert!(matches!(
        validate_loopback_endpoint("http://api.openai.com/v1"),
        Err(VectorError::InvalidEndpoint(_))
    ));
    assert!(matches!(
        validate_loopback_endpoint("https://192.168.1.100:11434"),
        Err(VectorError::InvalidEndpoint(_))
    ));

    // 2. Synthetic Ollama embedding server
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);

            // Notify request received
            tx.send(req.contains("/api/embeddings")).unwrap();

            let response_body = r#"{"embedding": [0.1, 0.2, 0.3, 0.4]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let endpoint = format!("http://127.0.0.1:{}", port);
    let vector =
        generate_ollama_embedding(&endpoint, "nomic-embed-text", "Search query text").unwrap();

    assert_eq!(vector.dimension(), 4);
    assert_eq!(vector.as_slice(), &[0.1, 0.2, 0.3, 0.4]);
    assert!(rx.recv().unwrap());
    handle.join().unwrap();

    // 3. Offline server connection failure
    let offline_endpoint = "http://127.0.0.1:1";
    let offline_result = generate_ollama_embedding(offline_endpoint, "nomic-embed-text", "Query");
    assert!(matches!(offline_result, Err(VectorError::Offline(_))));
}
