//! Application integration proof for the M5 provider subsystem:
//! the complete workflow through `Core::ai_command` —
//! providers-catalog, providers-dispatch, and the streamed
//! start/poll/cancel run pattern — against a fake loopback Ollama.
//! The keyring-backed HTTPS path is proved in the anansi-providers
//! test-suite (`full_workflow.rs`); this test proves the wiring the
//! application actually calls. No real provider, no credentials:
//! Ollama is local and keyless by design.

use anansi_core::Core;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

/// A minimal fake Ollama: POST /api/chat. Non-streaming requests
/// get one JSON object; streaming requests get NDJSON lines, dribbled
/// when the prompt says "slow".
fn start_fake_ollama() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            std::thread::spawn(move || {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let lower = line.to_ascii_lowercase();
                    if let Some(v) = lower.strip_prefix("content-length:") {
                        content_length = v.trim().parse().unwrap_or(0);
                    }
                    if line == "\r\n" {
                        break;
                    }
                }
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }
                let body = String::from_utf8_lossy(&body).to_string();
                let streaming = body.contains("\"stream\":true");
                let slow = body.contains("slow");
                let respond = |stream: &mut std::net::TcpStream, payload: &str| {
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        payload.len(),
                        payload
                    );
                    let _ = stream.write_all(resp.as_bytes());
                };
                if streaming {
                    let pieces: Vec<String> = (0..40)
                        .map(|i| {
                            format!(
                                "{{\"model\":\"llama-x\",\"message\":{{\"role\":\"assistant\",\"content\":\"tok{i} \"}},\"done\":false}}\n"
                            )
                        })
                        .collect();
                    // Send the response headers with chunked-style manual
                    // framing: simplest is one write per line with the
                    // full length unknown — use chunked encoding.
                    let _ = stream.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                    );
                    let mut send = |stream: &mut std::net::TcpStream, data: &str| {
                        let _ = stream.write_all(format!("{:x}\r\n", data.len()).as_bytes());
                        let _ = stream.write_all(data.as_bytes());
                        let _ = stream.write_all(b"\r\n");
                        let _ = stream.flush();
                    };
                    for piece in &pieces {
                        send(&mut stream, piece);
                        if slow {
                            std::thread::sleep(Duration::from_millis(60));
                        }
                    }
                    send(
                        &mut stream,
                        "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true,\"prompt_eval_count\":3,\"eval_count\":40}\n",
                    );
                    let _ = stream.write_all(b"0\r\n\r\n");
                } else {
                    respond(
                        &mut stream,
                        "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"core reply\"},\"done\":true,\"prompt_eval_count\":4,\"eval_count\":2}",
                    );
                }
            });
        }
    });
    format!("http://{addr}")
}

fn provider_config(base_url: &str) -> Value {
    json!({
        "providers": [{
            "config": {
                "provider": "ollama",
                "baseUrl": base_url,
                "credential": null,
                "enabled": true
            },
            "locality": "local",
            "models": [{
                "id": "llama-x",
                "provider": "ollama",
                "capabilities": { "chat": true, "streaming": true, "vision": false,
                                  "coding": true, "deepReasoning": false, "embeddings": false },
                "contextWindow": 8192,
                "cost": "free",
                "speed": "balanced"
            }]
        }]
    })
}

fn chat_request(text: &str) -> Value {
    // The CompletionRequest contract: messages carry classified
    // segments (anansi-providers types), not bare content strings.
    json!({
        "model": "llama-x",
        "messages": [{
            "role": "user",
            "segments": [{ "text": text, "privacy": "standard" }]
        }]
    })
}

#[test]
fn provider_workflow_through_ai_command() {
    let base = start_fake_ollama();
    let core = Core::default();

    // Catalog op answers without any configuration.
    let catalog = core
        .ai_command("providers-catalog", json!({}))
        .unwrap();
    let providers = catalog["providers"].as_array().unwrap();
    assert!(providers.len() >= 20, "catalog lists the provider set");
    assert!(providers.iter().any(|p| p["id"] == "ollama"));

    let config = provider_config(&base);

    // Unary dispatch: configuration → gates → transport → response
    // + disclosure, in one command.
    let out = core
        .ai_command(
            "providers-dispatch",
            json!({ "config": config, "request": chat_request("hello"), "mode": "auto" }),
        )
        .unwrap();
    assert_eq!(out["response"]["text"], "core reply");
    assert_eq!(out["response"]["usage"]["outputTokens"], 2);
    assert_eq!(out["disclosure"]["provider"], "ollama");
    assert_eq!(out["disclosure"]["model"], "llama-x");
    assert_eq!(out["disclosure"]["local"], true);

    // Streamed dispatch through start/poll, mirroring ask runs.
    let started = core
        .ai_command(
            "providers-stream-start",
            json!({ "config": config, "request": chat_request("stream please"), "mode": "auto" }),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    assert_eq!(started["disclosure"]["provider"], "ollama");
    let mut text = String::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let poll = core
            .ai_command("providers-stream-poll", json!({ "runId": run_id }))
            .unwrap();
        for event in poll["events"].as_array().unwrap() {
            if event["kind"] == "token" {
                text.push_str(event["text"].as_str().unwrap());
            }
            assert_ne!(event["kind"], "error", "stream error: {event}");
        }
        if poll["done"].as_bool().unwrap() {
            break;
        }
        assert!(Instant::now() < deadline, "stream did not finish");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(text.starts_with("tok0 tok1"), "streamed tokens: {text}");

    // Cancellation: start a slow stream, cancel it, and observe the
    // Cancelled error event through polling.
    let started = core
        .ai_command(
            "providers-stream-start",
            json!({ "config": config, "request": chat_request("slow stream"), "mode": "auto" }),
        )
        .unwrap();
    let run_id = started["runId"].as_str().unwrap().to_string();
    // Let a few tokens flow first.
    std::thread::sleep(Duration::from_millis(250));
    let first_poll = core
        .ai_command("providers-stream-poll", json!({ "runId": run_id }))
        .unwrap();
    assert!(
        first_poll["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "token"),
        "expected tokens before cancel: {first_poll}"
    );
    let cancelled = core
        .ai_command("providers-stream-cancel", json!({ "runId": run_id }))
        .unwrap();
    assert_eq!(cancelled["cancelled"], true);
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut saw_cancelled = false;
    loop {
        let poll = core
            .ai_command("providers-stream-poll", json!({ "runId": run_id }))
            .unwrap();
        for event in poll["events"].as_array().unwrap() {
            if event["kind"] == "error" && event["code"] == "Cancelled" {
                saw_cancelled = true;
            }
        }
        if poll["done"].as_bool().unwrap() {
            break;
        }
        assert!(Instant::now() < deadline, "cancelled stream did not end");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(saw_cancelled, "poll must surface the Cancelled event");
}
