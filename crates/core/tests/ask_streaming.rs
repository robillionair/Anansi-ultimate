use anansi_core::{Core, ask::AskRequest, ask_stream::AskRunRequest};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn partial_tokens_publish_and_cancellation_closes_backend_socket() {
    let fixture = tempfile::tempdir().unwrap();
    let core = Core::with_storage_root(fixture.path().join("storage"));
    let workspace = core
        .workspace_command("create", json!({"contractVersion":1,"name":"Streaming"}))
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    core.memory_command("create",json!({"contractVersion":1,"workspaceId":workspace,"draft":{"title":"Streaming architecture","body":"Streaming architecture uses local models.","privacy":"private"},"kind":"note"})).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (closed_tx, closed_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let n = socket.read(&mut buffer).unwrap();
            request.extend_from_slice(&buffer[..n]);
            if let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&request[..end]);
                let length: usize = header
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        }
        assert!(String::from_utf8_lossy(&request).contains("\"stream\":true"));
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap();
        let packet = b"{\"response\":\"Partial evidence [1]\",\"done\":false}\n";
        write!(socket, "{:x}\r\n", packet.len()).unwrap();
        socket.write_all(packet).unwrap();
        socket.write_all(b"\r\n").unwrap();
        socket.flush().unwrap();
        closed_tx
            .send(matches!(socket.read(&mut buffer), Ok(0)))
            .unwrap();
    });
    let started = core
        .start_ask(AskRequest {
            contract_version: 1,
            workspace_id: workspace.clone(),
            prompt: "Streaming architecture".into(),
            runtime_endpoint: Some(format!("http://127.0.0.1:{port}")),
            model_tag: Some("synthetic:small".into()),
            embedding_model: None,
            max_evidence_items: None,
        })
        .unwrap();
    let request = || AskRunRequest {
        contract_version: 1,
        workspace_id: workspace.clone(),
        id: started.id.clone(),
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let run = core.control_ask("ask-poll", request()).unwrap();
        if run.partial_answer.contains("Partial evidence") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "No partial token before deadline"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let wrong = AskRunRequest {
        contract_version: 1,
        workspace_id: "ffffffffffffffffffffffffffffffff".into(),
        id: started.id.clone(),
    };
    assert!(core.control_ask("ask-cancel", wrong).is_err());
    let cancelled = core.control_ask("ask-cancel", request()).unwrap();
    assert_eq!(cancelled.state, "cancelled");
    assert!(cancelled.partial_answer.is_empty());
    assert!(
        closed_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        "Cancellation did not close the server socket"
    );
    server.join().unwrap();
    core.control_ask("ask-forget", request()).unwrap();
    assert!(core.control_ask("ask-poll", request()).is_err());
}
