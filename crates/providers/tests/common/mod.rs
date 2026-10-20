//! Synthetic provider server + test rig. The server is a std
//! `TcpListener` on loopback with an ephemeral port; handlers are
//! scripted per test and can dribble chunked bodies, delay, redirect,
//! echo headers, or drop connections. No test traffic leaves loopback,
//! and the only credentials in play are obviously fake.

use anansi_providers::catalog;
use anansi_providers::config::ProviderConfig;
use anansi_providers::dispatcher::Dispatcher;
use anansi_providers::gates::TargetLocality;
use anansi_providers::http::StdHttpTransport;
use anansi_providers::routing::ProviderRegistration;
use anansi_providers::secrets::{CredentialRef, InMemoryResolver};
use anansi_providers::types::{CapabilitySet, CostClass, ModelInfo, SpeedTier};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const FAKE_KEY: &str = "sk-fake-test-1234567890";

#[derive(Debug, Clone)]
#[allow(dead_code)] // method is recorded for debugging failing tests
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl RecordedRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct ServerResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Encode the body as chunked transfer-encoding in small pieces.
    pub chunked_piece: Option<usize>,
    /// Sleep between chunk pieces (streaming/cancellation tests).
    pub piece_delay: Duration,
    /// Sleep before writing anything (timeout tests).
    pub delay_before: Duration,
    /// Close the connection with no response at all.
    pub drop: bool,
}

impl ServerResponse {
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: body.as_bytes().to_vec(),
            chunked_piece: None,
            piece_delay: Duration::ZERO,
            delay_before: Duration::ZERO,
            drop: false,
        }
    }
    pub fn text(status: u16, body: &str) -> Self {
        let mut r = Self::json(status, body);
        r.headers = vec![("Content-Type".to_string(), "text/plain".to_string())];
        r
    }
    pub fn chunked(mut self, piece: usize, delay: Duration) -> Self {
        self.chunked_piece = Some(piece);
        self.piece_delay = delay;
        self
    }
    pub fn delayed(mut self, delay: Duration) -> Self {
        self.delay_before = delay;
        self
    }
}

pub struct SyntheticServer {
    pub base_url: String,
    pub hits: Arc<AtomicUsize>,
    pub requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

type Handler = Arc<dyn Fn(&RecordedRequest, usize) -> ServerResponse + Send + Sync>;

impl SyntheticServer {
    pub fn start(
        handler: impl Fn(&RecordedRequest, usize) -> ServerResponse + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let handler: Handler = Arc::new(handler);
        let hits_thread = hits.clone();
        let requests_thread = requests.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let hits = hits_thread.clone();
                let requests = requests_thread.clone();
                let handler = handler.clone();
                std::thread::spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                    let Some(req) = read_request(&mut stream) else {
                        return;
                    };
                    let hit = hits.fetch_add(1, Ordering::SeqCst);
                    requests.lock().unwrap().push(req.clone());
                    let resp = handler(&req, hit);
                    if resp.drop {
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        return;
                    }
                    if !resp.delay_before.is_zero() {
                        std::thread::sleep(resp.delay_before);
                    }
                    write_response(&mut stream, &resp);
                });
            }
        });
        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            hits,
            requests,
        }
    }

    pub fn hit_count(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
    pub fn recorded(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

fn read_request(stream: &mut std::net::TcpStream) -> Option<RecordedRequest> {
    let mut raw: Vec<u8> = Vec::new();
    let mut buf = [0u8; 4096];
    let header_end = loop {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        raw.extend_from_slice(&buf[..n]);
        if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        if raw.len() > 128 * 1024 {
            return None;
        }
    };
    let head = String::from_utf8_lossy(&raw[..header_end]).to_string();
    let mut lines = head.lines();
    let first = lines.next()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let mut headers = Vec::new();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let (k, v) = (k.trim().to_string(), v.trim().to_string());
            if k.eq_ignore_ascii_case("content-length") {
                content_length = v.parse().unwrap_or(0);
            }
            headers.push((k, v));
        }
    }
    let mut body = raw[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
    }
    body.truncate(content_length);
    Some(RecordedRequest {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).to_string(),
    })
}

fn write_response(stream: &mut std::net::TcpStream, resp: &ServerResponse) {
    let reason = match resp.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        302 => "Found",
        _ => "Status",
    };
    let mut head = format!("HTTP/1.1 {} {}\r\n", resp.status, reason);
    for (k, v) in &resp.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    if let Some(piece) = resp.chunked_piece {
        head.push_str("Transfer-Encoding: chunked\r\nConnection: close\r\n\r\n");
        if stream.write_all(head.as_bytes()).is_err() {
            return;
        }
        for chunk in resp.body.chunks(piece.max(1)) {
            let frame = format!("{:x}\r\n", chunk.len());
            if stream.write_all(frame.as_bytes()).is_err()
                || stream.write_all(chunk).is_err()
                || stream.write_all(b"\r\n").is_err()
                || stream.flush().is_err()
            {
                return; // client went away (cancellation) — expected
            }
            if !resp.piece_delay.is_zero() {
                std::thread::sleep(resp.piece_delay);
            }
        }
        let _ = stream.write_all(b"0\r\n\r\n");
    } else {
        head.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            resp.body.len()
        ));
        if stream.write_all(head.as_bytes()).is_err() {
            return;
        }
        let _ = stream.write_all(&resp.body);
    }
    let _ = stream.flush();
}

/// A model fixture.
pub fn model(id: &str, provider: &str, cost: CostClass) -> ModelInfo {
    ModelInfo {
        id: id.to_string(),
        provider: provider.to_string(),
        capabilities: CapabilitySet {
            chat: true,
            streaming: true,
            vision: false,
            coding: true,
            deep_reasoning: false,
            embeddings: false,
        },
        context_window: Some(8192),
        cost,
        speed: SpeedTier::Balanced,
        price_input_micro_usd_per_mtok: None,
        price_output_micro_usd_per_mtok: None,
    }
}

/// A registration pointing a provider id at a synthetic server. The
/// registration's locality is what gates see; the URL is loopback so
/// the std transport can reach it.
pub fn registration(
    provider: &str,
    base_url: String,
    locality: TargetLocality,
    with_credential: bool,
    models: Vec<ModelInfo>,
) -> ProviderRegistration {
    let mut config = ProviderConfig::new(provider, base_url);
    if with_credential {
        config.credential = Some(CredentialRef::new(provider, format!("{provider}-key-1")));
    }
    ProviderRegistration {
        config,
        locality,
        models,
    }
}

/// A resolver holding the one fake key for any reference.
pub fn resolver() -> InMemoryResolver {
    let mut r = InMemoryResolver::new();
    for name in [
        "openai-key-1",
        "anthropic-key-1",
        "gemini-key-1",
        "azure-key-1",
        "custom-openai-key-1",
    ] {
        r.insert(name, FAKE_KEY);
    }
    r
}

pub fn dispatcher<'a>(
    transport: &'a StdHttpTransport,
    resolver: &'a InMemoryResolver,
) -> Dispatcher<'a> {
    Dispatcher::new(transport, resolver, catalog::default_adapters())
        .with_retry(anansi_providers::config::RetryPolicy {
            max_retries: 2,
            base_backoff_ms: 1,
            max_retry_after_ms: 100,
        })
        .with_timeouts(anansi_providers::config::TimeoutConfig {
            connect_ms: 1000,
            first_byte_ms: 2000,
            idle_ms: 1000,
            total_ms: 8000,
        })
}

pub fn openai_success_body() -> &'static str {
    r#"{"model":"gpt-x","choices":[{"message":{"role":"assistant","content":"hello from synthetic"},"finish_reason":"stop"}],"usage":{"prompt_tokens":12,"completion_tokens":6}}"#
}
