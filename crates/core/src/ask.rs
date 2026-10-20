use crate::ai_discovery::{RuntimeKind, RuntimeStatus, probe_runtime};
use crate::{CommandError, Core, ErrorCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AskStatus {
    Answered,
    InsufficientEvidence,
    Offline,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroundedCitation {
    pub index: usize,
    pub memory_id: String,
    pub title: String,
    pub kind: String,
    pub section_heading: Option<String>,
    pub line_number: Option<u32>,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceMemoryReference {
    pub id: String,
    pub title: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisclosureManifest {
    pub runtime_kind: String,
    pub endpoint: String,
    pub model_tag: String,
    pub source_memories: Vec<SourceMemoryReference>,
    pub character_count: usize,
    pub estimated_tokens: usize,
    pub routing_reason: String,
    pub compiled_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AskRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub prompt: String,
    pub runtime_endpoint: Option<String>,
    pub model_tag: Option<String>,
    pub max_evidence_items: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AskResponse {
    pub contract_version: u8,
    pub status: AskStatus,
    pub answer: String,
    pub citations: Vec<GroundedCitation>,
    pub manifest: DisclosureManifest,
}

fn extract_query_terms(prompt: &str) -> Vec<String> {
    let stop_words = [
        "what", "did", "we", "decide", "about", "the", "a", "an", "is", "our", "for", "in", "on",
        "of", "to", "how", "why", "when", "where", "who", "which", "can", "you", "tell", "me",
        "find", "show", "give", "and", "or", "not", "with",
    ];

    let words: Vec<String> = prompt
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect();

    let content_words: Vec<String> = words
        .iter()
        .filter(|w| !stop_words.contains(&w.as_str()) && w.len() > 1)
        .cloned()
        .collect();

    if !content_words.is_empty() {
        content_words.into_iter().take(8).collect()
    } else {
        words.into_iter().take(8).collect()
    }
}

fn parse_loopback_url(endpoint: &str) -> Option<(String, u16, String)> {
    let trimmed = endpoint.trim();
    let without_proto = trimmed.strip_prefix("http://")?;

    let (host_port, path) = match without_proto.find('/') {
        Some(idx) => (&without_proto[..idx], &without_proto[idx..]),
        None => (without_proto, "/"),
    };

    let (host, port) = if let Some(idx) = host_port.find(':') {
        let h = &host_port[..idx];
        let p: u16 = host_port[idx + 1..].parse().ok()?;
        (h, p)
    } else {
        (host_port, 80)
    };

    // Strict security constraint: only allow loopback addresses
    if host != "127.0.0.1" && host != "localhost" && host != "[::1]" {
        return None;
    }

    Some((host.to_string(), port, path.to_string()))
}

fn http_post_loopback(
    host: &str,
    port: u16,
    path: &str,
    body: &str,
    timeout: Duration,
    control: Option<&crate::ask_stream::Control>,
) -> Result<(u16, String), ()> {
    let addr_str = format!("{}:{}", host, port);
    let socket_addrs: Vec<SocketAddr> = addr_str.to_socket_addrs().map_err(|_| ())?.collect();
    if socket_addrs.is_empty() {
        return Err(());
    }

    let mut stream = TcpStream::connect_timeout(&socket_addrs[0], timeout).map_err(|_| ())?;
    if let Some(control) = control {
        *control.socket.lock().map_err(|_| ())? = Some(stream.try_clone().map_err(|_| ())?);
    }
    stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .map_err(|_| ())?;
    stream.set_write_timeout(Some(timeout)).map_err(|_| ())?;
    if control.is_some_and(|c| c.cancelled.load(std::sync::atomic::Ordering::SeqCst)) {
        let _ = stream.shutdown(std::net::Shutdown::Both);
        return Err(());
    }

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: ANANSI/Ask\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path,
        host,
        port,
        body.len(),
        body
    );
    stream.write_all(request.as_bytes()).map_err(|_| ())?;

    let mut response_buf = Vec::with_capacity(32 * 1024);
    let mut chunk = [0u8; 4096];
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if std::time::Instant::now() >= deadline
            || control.is_some_and(|c| c.cancelled.load(std::sync::atomic::Ordering::SeqCst))
        {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            return Err(());
        }
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                response_buf.extend_from_slice(&chunk[..n]);
                if response_buf.len() > 128 * 1024 {
                    return Err(());
                }
                if let Some(control) = control {
                    update_stream(&response_buf, control)?;
                    if control.finished.load(std::sync::atomic::Ordering::SeqCst) {
                        break;
                    }
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(_) => return Err(()),
        }
    }

    let text = String::from_utf8_lossy(&response_buf);
    let mut parts = text.splitn(2, "\r\n\r\n");
    let header_part = parts.next().unwrap_or("");
    let raw_body = parts.next().unwrap_or("");
    let body_part = if header_part
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        String::from_utf8(dechunk(raw_body.as_bytes())?).map_err(|_| ())?
    } else {
        raw_body.to_string()
    };

    let status_code: u16 = header_part
        .lines()
        .next()
        .and_then(|line| {
            let mut words = line.split_whitespace();
            words.next()?; // HTTP/1.1
            words.next()?.parse().ok()
        })
        .unwrap_or(0);

    if let Some(control) = control {
        update_stream(&response_buf, control)?;
        if !control.finished.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(());
        }
        let answer = control.answer.lock().map_err(|_| ())?.clone();
        if !answer.is_empty() {
            return Ok((
                status_code,
                serde_json::json!({"response":answer}).to_string(),
            ));
        }
    }
    Ok((status_code, body_part))
}

fn dechunk(mut bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let mut output = Vec::new();
    while let Some(end) = bytes.windows(2).position(|v| v == b"\r\n") {
        let size = std::str::from_utf8(&bytes[..end])
            .map_err(|_| ())?
            .split(';')
            .next()
            .ok_or(())?;
        let size = usize::from_str_radix(size, 16).map_err(|_| ())?;
        bytes = &bytes[end + 2..];
        if size == 0 {
            break;
        }
        if size > 128 * 1024 {
            return Err(());
        }
        if bytes.len() < size + 2 {
            break;
        }
        if &bytes[size..size + 2] != b"\r\n" {
            return Err(());
        }
        output.extend_from_slice(&bytes[..size]);
        bytes = &bytes[size + 2..];
    }
    Ok(output)
}

fn update_stream(bytes: &[u8], control: &crate::ask_stream::Control) -> Result<(), ()> {
    let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") else {
        return Ok(());
    };
    let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
    let body = if headers.contains("transfer-encoding: chunked") {
        dechunk(&bytes[end + 4..])?
    } else {
        bytes[end + 4..].to_vec()
    };
    let text = String::from_utf8_lossy(&body);
    let mut answer = String::new();
    for line in text.lines() {
        let line = line.strip_prefix("data: ").unwrap_or(line);
        if line == "[DONE]" {
            control
                .finished
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if value["done"].as_bool() == Some(true) {
                control
                    .finished
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
            if let Some(fragment) = value["response"]
                .as_str()
                .or_else(|| value["choices"][0]["delta"]["content"].as_str())
            {
                answer.push_str(fragment);
            }
        }
    }
    if !answer.is_empty() {
        *control.answer.lock().map_err(|_| ())? = answer;
    }
    Ok(())
}

impl Core {
    pub fn ask_anansi(&self, request: AskRequest) -> Result<AskResponse, CommandError> {
        self.ask_anansi_controlled(request, None)
    }

    pub(crate) fn ask_anansi_controlled(
        &self,
        request: AskRequest,
        control: Option<&crate::ask_stream::Control>,
    ) -> Result<AskResponse, CommandError> {
        if request.contract_version != 1 {
            return Err(CommandError::new(
                ErrorCode::UnsupportedContract,
                "This contract version is not supported.",
            ));
        }

        let trimmed_prompt = request.prompt.trim();
        if trimmed_prompt.is_empty() {
            return Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "Enter a non-empty question for Ask ANANSI.",
            ));
        }
        if trimmed_prompt.len() > 500 {
            return Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "Ask ANANSI question cannot exceed 500 characters.",
            ));
        }

        let max_items = request.max_evidence_items.unwrap_or(5).clamp(1, 10);

        // Validate custom endpoint loopback early if provided
        if let Some(custom) = request.runtime_endpoint.as_deref()
            && parse_loopback_url(custom).is_none()
        {
            return Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "Inference endpoint must be a valid loopback address (127.0.0.1 or localhost).",
            ));
        }

        // 1. Context Compilation
        let query_terms = extract_query_terms(trimmed_prompt);
        let search_query = if !query_terms.is_empty() {
            query_terms.join(" ")
        } else {
            trimmed_prompt.to_string()
        };

        let storage = self.storage()?;
        self.workspace_command(
            "open",
            serde_json::json!({"contractVersion":1,"id":request.workspace_id}),
        )?;
        // Use the same compiler as the explicit context application command.
        // Rebuild/integrity checks run before reading derived retrieval data.
        storage.rebuild_search_index(&request.workspace_id)?;
        let conn = storage
            .connection(false)?
            .ok_or_else(crate::storage::unavailable)?;
        let compiled = crate::context::compile_context_with_query(
            &conn,
            storage.root_path(),
            &crate::context::ContextCompilationRequest {
                workspace_id: request.workspace_id.clone(),
                user_prompt: trimmed_prompt.to_owned(),
                max_budget_chars: Some((max_items * 800).min(4000)),
                runtime_endpoint: request.runtime_endpoint.clone(),
                model_tag: request.model_tag.clone(),
                kind_filter: None,
            },
            Some(&search_query),
        )?;
        drop(conn);
        let citations = compiled.citations;
        let source_refs = compiled.disclosure_manifest.source_memories;

        // 2. Truthful Insufficient Evidence Handling
        if citations.is_empty() {
            let manifest = DisclosureManifest {
                runtime_kind: "none".to_string(),
                endpoint: "none".to_string(),
                model_tag: "none".to_string(),
                source_memories: Vec::new(),
                character_count: 0,
                estimated_tokens: 0,
                routing_reason: "insufficient_evidence".to_string(),
                compiled_prompt: String::new(),
            };

            return Ok(AskResponse {
                contract_version: 1,
                status: AskStatus::InsufficientEvidence,
                answer: "I couldn't find any relevant memories, notes, or knowledge records in this workspace to answer your question. Add a note or memory with the relevant information first."
                    .to_string(),
                citations: Vec::new(),
                manifest,
            });
        }

        // 3. Grounding System Prompt Construction
        let compiled_prompt = compiled.system_prompt;

        let char_count = compiled_prompt.len();
        let est_tokens = char_count / 4;

        // 4. Runtime Selection & Probing
        let (endpoint, kind, model) = if let Some(custom) = request.runtime_endpoint.as_deref() {
            let detected_kind = if custom.contains(":11434") {
                RuntimeKind::Ollama
            } else if custom.contains(":1234") {
                RuntimeKind::LmStudio
            } else if let Some(ref m) = request.model_tag {
                if m.contains(':') {
                    RuntimeKind::Ollama
                } else {
                    RuntimeKind::CustomLoopback
                }
            } else {
                RuntimeKind::Ollama
            };
            (
                custom.to_string(),
                detected_kind,
                request.model_tag.unwrap_or_else(|| "default".to_string()),
            )
        } else {
            // Check Ollama then LM Studio
            let ollama_probe = probe_runtime(RuntimeKind::Ollama, "http://127.0.0.1:11434");
            if ollama_probe.status == RuntimeStatus::Available {
                let default_model = ollama_probe
                    .models
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "llama3.1:8b".to_string());
                (
                    "http://127.0.0.1:11434".to_string(),
                    RuntimeKind::Ollama,
                    request.model_tag.unwrap_or(default_model),
                )
            } else {
                let lm_probe = probe_runtime(RuntimeKind::LmStudio, "http://127.0.0.1:1234");
                if lm_probe.status == RuntimeStatus::Available {
                    let default_model = lm_probe
                        .models
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "default".to_string());
                    (
                        "http://127.0.0.1:1234".to_string(),
                        RuntimeKind::LmStudio,
                        request.model_tag.unwrap_or(default_model),
                    )
                } else {
                    (
                        "http://127.0.0.1:11434".to_string(),
                        RuntimeKind::Ollama,
                        request
                            .model_tag
                            .unwrap_or_else(|| "llama3.1:8b".to_string()),
                    )
                }
            }
        };

        let runtime_kind_str = match kind {
            RuntimeKind::Ollama => "ollama",
            RuntimeKind::LmStudio => "lmStudio",
            RuntimeKind::CustomLoopback => "customLoopback",
        };

        // 5. Loopback Execution
        if control.is_some_and(|c| c.cancelled.load(std::sync::atomic::Ordering::SeqCst)) {
            return Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "Ask run cancelled.",
            ));
        }
        let Some((host, port, _)) = parse_loopback_url(&endpoint) else {
            return Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "Inference endpoint must be a valid loopback address (127.0.0.1 or localhost).",
            ));
        };

        let (post_path, post_body) = match kind {
            RuntimeKind::Ollama => (
                "/api/generate",
                serde_json::json!({
                    "model": model,
                    "prompt": compiled_prompt,
                    "stream": control.is_some(),
                    "options":{"num_ctx":2048,"num_predict":96},
                    "keep_alive":0
                })
                .to_string(),
            ),
            RuntimeKind::LmStudio | RuntimeKind::CustomLoopback => (
                "/v1/chat/completions",
                serde_json::json!({
                    "model": model,
                    "messages": [
                        { "role": "user", "content": compiled_prompt }
                    ],
                    "stream": control.is_some(),
                    "max_tokens":96
                })
                .to_string(),
            ),
        };

        let post_result = http_post_loopback(
            &host,
            port,
            post_path,
            &post_body,
            Duration::from_secs(180),
            control,
        );

        let manifest = DisclosureManifest {
            runtime_kind: runtime_kind_str.to_string(),
            endpoint: endpoint.clone(),
            model_tag: model.clone(),
            source_memories: source_refs,
            character_count: char_count,
            estimated_tokens: est_tokens,
            routing_reason: "local-first offline inference".to_string(),
            compiled_prompt,
        };

        match post_result {
            Ok((200, res_body)) => {
                let mut answer = String::new();
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&res_body) {
                    if let Some(resp) = parsed.get("response").and_then(|v| v.as_str()) {
                        answer = resp.trim().to_string();
                    } else if let Some(content) = parsed
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|m| m.get("message"))
                        .and_then(|msg| msg.get("content"))
                        .and_then(|v| v.as_str())
                    {
                        answer = content.trim().to_string();
                    }
                }

                let status = if answer.is_empty() {
                    AskStatus::Error
                } else {
                    AskStatus::Answered
                };
                if answer.is_empty() {
                    answer = "The local inference engine returned an empty response. Verify model configuration."
                        .to_string();
                }

                Ok(AskResponse {
                    contract_version: 1,
                    status,
                    answer,
                    citations,
                    manifest,
                })
            }
            Ok((status, _)) => Ok(AskResponse {
                contract_version: 1,
                status: AskStatus::Error,
                answer: format!(
                    "Local inference server responded with unexpected HTTP status {}. Check server logs.",
                    status
                ),
                citations,
                manifest,
            }),
            Err(_) => {
                // Return truthful Offline status with compiled evidence pack
                let offline_manifest = DisclosureManifest {
                    runtime_kind: "offline".to_string(),
                    ..manifest
                };
                Ok(AskResponse {
                    contract_version: 1,
                    status: AskStatus::Offline,
                    answer: "Local AI runtime is offline. Start Ollama (http://127.0.0.1:11434) or LM Studio (http://127.0.0.1:1234) locally to generate answers from your compiled context."
                        .to_string(),
                    citations,
                    manifest: offline_manifest,
                })
            }
        }
    }
}

#[cfg(test)]
mod streaming_protocol_tests {
    use super::*;
    use std::sync::atomic::Ordering;
    #[test]
    fn openai_sse_and_ollama_ndjson_require_explicit_completion() {
        let control = crate::ask_stream::Control::default();
        update_stream(
            b"HTTP/1.1 200 OK\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
            &control,
        )
        .unwrap();
        assert_eq!(*control.answer.lock().unwrap(), "Hello");
        assert!(!control.finished.load(Ordering::SeqCst));
        update_stream(b"HTTP/1.1 200 OK\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\ndata: [DONE]\n\n",&control).unwrap();
        assert!(control.finished.load(Ordering::SeqCst));
        let control = crate::ask_stream::Control::default();
        update_stream(b"HTTP/1.1 200 OK\r\n\r\n{\"response\":\"World\",\"done\":false}\n{\"response\":\"!\",\"done\":true}\n",&control).unwrap();
        assert_eq!(*control.answer.lock().unwrap(), "World!");
        assert!(control.finished.load(Ordering::SeqCst));
    }
    #[test]
    fn incomplete_and_malformed_http_chunks_are_bounded() {
        assert_eq!(dechunk(b"5\r\nhe").unwrap(), b"");
        assert_eq!(dechunk(b"5\r\nhello\r\n0\r\n\r\n").unwrap(), b"hello");
        assert!(dechunk(b"5\r\nhelloXX").is_err());
        assert!(dechunk(b"ffffff\r\n").is_err());
    }
}
