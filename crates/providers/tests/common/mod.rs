#![allow(dead_code)]
//! Synthetic provider server + test rig. The server is a std
//! `TcpListener` on loopback with an ephemeral port; handlers are
//! scripted per test and can dribble chunked bodies, delay, redirect,
//! echo headers, or drop connections. No test traffic leaves loopback,
//! and the only credentials in play are obviously fake.

use anansi_providers::catalog;
use anansi_providers::config::ProviderConfig;
use anansi_providers::dispatcher::Dispatcher;
use anansi_providers::gates::TargetLocality;
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

fn read_request(stream: &mut impl Read) -> Option<RecordedRequest> {
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

fn write_response(stream: &mut impl Write, resp: &ServerResponse) {
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

pub fn dispatcher<'a, T: anansi_providers::http::Transport>(
    transport: &'a T,
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

// ---------------------------------------------------------------------------
// Synthetic HTTPS: test-only CA (rcgen) + rustls-terminating server.
// Certificates exist only inside the test process; keys are generated
// per run and never touch disk. The client side uses the crate's real
// RustlsTransport with the test CA added as an extra root — validation
// itself is never disabled anywhere in this harness.

use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, KeyUsagePurpose,
    SanType,
};
use std::io::BufReader;

/// A throwaway certificate authority for synthetic HTTPS tests.
pub struct TestCa {
    cert: rcgen::Certificate,
    key: KeyPair,
    pub cert_pem: String,
}

impl TestCa {
    pub fn new() -> Self {
        let key = KeyPair::generate().unwrap();
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "ANANSI Synthetic Test CA");
        params.distinguished_name = dn;
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        let cert = params.self_signed(&key).unwrap();
        let cert_pem = cert.pem();
        Self {
            cert,
            key,
            cert_pem,
        }
    }

    fn issue(&self, sans: Vec<SanType>, expired: bool) -> (String, String) {
        let key = KeyPair::generate().unwrap();
        let mut params = CertificateParams::default();
        params.subject_alt_names = sans;
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
        if expired {
            params.not_before = time::OffsetDateTime::now_utc() - time::Duration::days(3);
            params.not_after = time::OffsetDateTime::now_utc() - time::Duration::days(1);
        }
        let cert = params.signed_by(&key, &self.cert, &self.key).unwrap();
        (cert.pem(), key.serialize_pem())
    }

    /// Server cert valid for 127.0.0.1 and localhost.
    pub fn valid_server(&self) -> (String, String) {
        self.issue(
            vec![
                SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
                SanType::DnsName("localhost".try_into().unwrap()),
            ],
            false,
        )
    }

    /// Server cert for a *different* host, from the trusted CA:
    /// chain-valid but hostname-invalid for 127.0.0.1.
    pub fn wrong_host_server(&self) -> (String, String) {
        self.issue(
            vec![SanType::DnsName("other.invalid".try_into().unwrap())],
            false,
        )
    }

    /// Server cert with correct SANs but an expired validity window.
    pub fn expired_server(&self) -> (String, String) {
        self.issue(
            vec![SanType::IpAddress(std::net::IpAddr::V4(
                std::net::Ipv4Addr::LOCALHOST,
            ))],
            true,
        )
    }
}

/// A self-signed server certificate no test CA vouches for.
pub fn self_signed_server() -> (String, String) {
    let key = KeyPair::generate().unwrap();
    let mut params = CertificateParams::default();
    params.subject_alt_names = vec![SanType::IpAddress(std::net::IpAddr::V4(
        std::net::Ipv4Addr::LOCALHOST,
    ))];
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    let cert = params.self_signed(&key).unwrap();
    (cert.pem(), key.serialize_pem())
}

fn server_config(cert_pem: &str, key_pem: &str) -> std::sync::Arc<rustls::ServerConfig> {
    let certs: Vec<rustls::pki_types::CertificateDer<'static>> =
        rustls_pemfile::certs(&mut BufReader::new(cert_pem.as_bytes()))
            .collect::<Result<_, _>>()
            .unwrap();
    let key = rustls_pemfile::private_key(&mut BufReader::new(key_pem.as_bytes()))
        .unwrap()
        .unwrap();
    let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    std::sync::Arc::new(config)
}

/// A synthetic provider server speaking HTTPS via rustls, with the
/// same scripting/hit-counting contract as [`SyntheticServer`]. A
/// connection whose TLS handshake fails is dropped before any
/// request is read, so it never increments the hit counter.
pub struct TlsSyntheticServer {
    pub base_url: String,
    pub hits: Arc<AtomicUsize>,
    pub requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

impl TlsSyntheticServer {
    pub fn start(
        cert_pem: &str,
        key_pem: &str,
        handler: impl Fn(&RecordedRequest, usize) -> ServerResponse + Send + Sync + 'static,
    ) -> Self {
        let config = server_config(cert_pem, key_pem);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let handler: Handler = Arc::new(handler);
        let hits_thread = hits.clone();
        let requests_thread = requests.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let config = config.clone();
                let hits = hits_thread.clone();
                let requests = requests_thread.clone();
                let handler = handler.clone();
                std::thread::spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
                    let conn = match rustls::ServerConnection::new(config) {
                        Ok(c) => c,
                        Err(_) => return,
                    };
                    let mut tls = rustls::StreamOwned::new(conn, stream);
                    // read_request drives the handshake on first read;
                    // a rejected handshake yields no request at all.
                    let Some(req) = read_request(&mut tls) else {
                        return;
                    };
                    let hit = hits.fetch_add(1, Ordering::SeqCst);
                    requests.lock().unwrap().push(req.clone());
                    let resp = handler(&req, hit);
                    if resp.drop {
                        return;
                    }
                    if !resp.delay_before.is_zero() {
                        std::thread::sleep(resp.delay_before);
                    }
                    write_response(&mut tls, &resp);
                });
            }
        });
        Self {
            base_url: format!("https://127.0.0.1:{port}"),
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

/// The crate's production transport, trusting the Mozilla roots plus
/// the synthetic test CA. Validation is fully on.
pub fn tls_transport(ca: &TestCa) -> anansi_providers::http::RustlsTransport {
    let mut config = anansi_providers::tls::TlsConfig::new();
    config.add_root_pem(ca.cert_pem.as_bytes()).unwrap();
    anansi_providers::http::RustlsTransport::with_tls_config(config)
}

// ---------------------------------------------------------------- dispatch helpers

use anansi_providers::dispatcher::DispatchContext;
use anansi_providers::gates::{Budget, PaidInferencePermit, QuotaLedger, ZeroCostLock};
use anansi_providers::routing::RoutingRequest;
use anansi_providers::types::{ChatMessage, CompletionRequest, Role};

pub fn test_request(model: &str) -> CompletionRequest {
    CompletionRequest::new(model, vec![ChatMessage::new(Role::User, "Say hello")])
}

pub fn test_permit(provider: &str, model: &str) -> PaidInferencePermit {
    PaidInferencePermit {
        id: format!("permit-{provider}-{model}"),
        provider: provider.to_string(),
        model: model.to_string(),
        max_estimated_total_tokens: 100_000,
        allow_unknown_cost: false,
    }
}

pub fn test_ctx<'a>(
    ledger: &'a mut QuotaLedger,
    mode: anansi_providers::routing::RoutingMode,
    lock: ZeroCostLock,
) -> DispatchContext<'a> {
    DispatchContext {
        routing: RoutingRequest {
            mode,
            lock,
            ..Default::default()
        },
        lock,
        budget: Budget::default(),
        ledger,
        permit: None,
    }
}
