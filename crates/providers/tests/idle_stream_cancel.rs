//! Regression: cancelling a stream whose provider has gone
//! SILENT — headers and one chunk delivered, then no further
//! bytes, connection held open (the gated shape).
//!
//! Before the sliced-read fix, `SocketBody::read` blocked in a
//! single `recv` for up to the idle timeout (30 s by default) and
//! prompt cancellation depended on `shutdown()` interrupting that
//! recv — true on Linux, not reliable on Windows, where the core
//! cancellation tests timed out waiting for terminal completion.
//! The read now wakes at least every 100 ms and re-checks the
//! cancel flag and both deadlines, so worker exit, terminal event
//! delivery and reservation settlement are bounded everywhere.
//!
//! These tests use deliberately long idle/total timeouts (120 s /
//! 300 s): if cancellation only took effect at a deadline, every
//! assertion here would take minutes instead of ~a second.

mod common;

use anansi_providers::config::TimeoutConfig;
use anansi_providers::dispatcher::Dispatcher;
use anansi_providers::error::ProviderErrorCode;
use anansi_providers::gates::{QuotaLedger, TargetLocality, ZeroCostLock};
use anansi_providers::http::{
    CancellationToken, HttpRequest, Transport,
};
use anansi_providers::routing::RoutingMode;
use anansi_providers::types::StreamEvent;
use common::*;
use std::io::{BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const FIRST_LINE: &str = "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"tok0 \"},\"done\":false}\n";

struct GatedTlsServer {
    base_url: String,
    release: Arc<AtomicBool>,
}

fn server_config(cert_pem: &str, key_pem: &str) -> Arc<rustls::ServerConfig> {
    let certs: Vec<rustls::pki_types::CertificateDer<'static>> =
        rustls_pemfile::certs(&mut BufReader::new(cert_pem.as_bytes()))
            .collect::<Result<_, _>>()
            .unwrap();
    let key = rustls_pemfile::private_key(&mut BufReader::new(key_pem.as_bytes()))
        .unwrap()
        .unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    Arc::new(config)
}

/// HTTPS server that answers with headers + one NDJSON chunk, then
/// holds the connection open in silence until released.
fn start_gated_server(ca: &TestCa) -> GatedTlsServer {
    let (cert_pem, key_pem) = ca.valid_server();
    let config = server_config(&cert_pem, &key_pem);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let release = Arc::new(AtomicBool::new(false));
    let release_thread = release.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let config = config.clone();
            let release = release_thread.clone();
            std::thread::spawn(move || {
                let conn = match rustls::ServerConnection::new(config) {
                    Ok(c) => c,
                    Err(_) => return,
                };
                let mut tls = rustls::StreamOwned::new(conn, stream);
                // Read the request (headers + body) — also drives
                // the handshake.
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                let mut content_length = 0usize;
                let header_end;
                loop {
                    let n = match tls.read(&mut buf) {
                        Ok(0) | Err(_) => return,
                        Ok(n) => n,
                    };
                    raw.extend_from_slice(&buf[..n]);
                    if let Some(pos) = raw
                        .windows(4)
                        .position(|w| w == b"\r\n\r\n")
                    {
                        header_end = pos + 4;
                        let text = String::from_utf8_lossy(&raw[..header_end]).to_string();
                        for line in text.lines() {
                            if let Some((k, v)) = line.split_once(':')
                                && k.eq_ignore_ascii_case("content-length")
                            {
                                content_length = v.trim().parse().unwrap_or(0);
                            }
                        }
                        break;
                    }
                }
                while raw.len() - header_end < content_length {
                    match tls.read(&mut buf) {
                        Ok(0) | Err(_) => return,
                        Ok(n) => raw.extend_from_slice(&buf[..n]),
                    }
                }
                // Headers + the first chunk, chunk-framed...
                let head = "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
                if tls.write_all(head.as_bytes()).is_err() {
                    return;
                }
                let chunk = format!("{:x}\r\n{}\r\n", FIRST_LINE.len(), FIRST_LINE);
                if tls.write_all(chunk.as_bytes()).is_err() || tls.flush().is_err() {
                    return;
                }
                // ...then silence, holding the connection open
                // until the test releases the gate (bounded).
                let start = Instant::now();
                while !release.load(Ordering::SeqCst)
                    && start.elapsed() < Duration::from_secs(120)
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                // Closing without a terminal chunk is intentional:
                // cancellation, not completion, ends this stream.
            });
        }
    });
    GatedTlsServer {
        base_url: format!("https://127.0.0.1:{port}"),
        release,
    }
}

fn long_timeouts() -> TimeoutConfig {
    TimeoutConfig {
        connect_ms: 5_000,
        first_byte_ms: 30_000,
        idle_ms: 120_000,
        total_ms: 300_000,
    }
}

#[test]
fn transport_read_blocked_on_a_silent_peer_wakes_promptly_on_cancel() {
    let ca = TestCa::new();
    let server = start_gated_server(&ca);
    let transport = tls_transport(&ca);
    let endpoint = anansi_providers::config::ProviderConfig {
        provider: "ollama".into(),
        base_url: server.base_url.clone(),
        credential: None,
        enabled: true,
        custom_endpoint: false,
        allow_insecure_custom: false,
    }
    .validate()
    .unwrap();
    let request = HttpRequest {
        method: "POST".to_string(),
        endpoint,
        path: "/api/chat".into(),
        headers: vec![("Content-Type".into(), "application/json".into())],
        body: br#"{"model":"llama-x","messages":[],"stream":true}"#.to_vec(),
    };
    let cancel = CancellationToken::new();
    let started = Instant::now();
    let response = transport
        .execute(&request, &long_timeouts(), &cancel)
        .unwrap();
    let mut body = response.into_body();
    // First read: the one chunk the server sends before going silent.
    let mut buf = [0u8; 1024];
    let n = body.read(&mut buf).unwrap();
    assert!(n > 0, "first chunk arrived");
    assert!(String::from_utf8_lossy(&buf[..n]).contains("tok0"));
    // Second read blocks inside the transport (server silent).
    // Cancel from this thread after a short, explicit delay and
    // require the blocked read to end as Cancelled — far inside
    // the 120 s idle deadline.
    let canceller = cancel.clone();
    let timer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        canceller.cancel();
    });
    let err = body.read(&mut buf).unwrap_err();
    timer.join().unwrap();
    server.release.store(true, Ordering::SeqCst);
    assert_eq!(
        err.kind(),
        std::io::ErrorKind::Interrupted,
        "blocked read must surface cancellation, got: {err}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "cancellation took {:?} — the read must not wait out the idle deadline",
        started.elapsed()
    );
}

#[test]
fn dispatcher_stream_cancellation_settles_the_reservation_and_closes_the_worker() {
    let ca = TestCa::new();
    let server = start_gated_server(&ca);
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = Dispatcher::new(&transport, &res, anansi_providers::catalog::default_adapters())
        .with_timeouts(long_timeouts());
    let reg = registration(
        "ollama",
        server.base_url.clone(),
        TargetLocality::Local,
        false,
        vec![model("llama-x", "ollama", anansi_providers::types::CostClass::Free)],
    );
    let ledger = Arc::new(Mutex::new(QuotaLedger::new()));
    let ctx = test_ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off());
    let cancel = CancellationToken::new();
    let outcome = dispatcher
        .execute_stream(&[reg], &test_request("llama-x"), ctx, &cancel)
        .unwrap();
    // The first token arrives; then the provider goes silent.
    let first = outcome
        .events
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap();
    assert!(matches!(first, StreamEvent::Token(ref t) if t.contains("tok0")), "{first:?}");
    // While the stream is open and idle, the request is already
    // accounted and the token reservation is still held.
    {
        let l = ledger.lock().unwrap();
        assert_eq!(l.reserved().0, 0, "request hold moves into totals at establishment");
        assert!(l.reserved().1 > 0, "the token hold is held while streaming");
        assert_eq!(l.totals_for("ollama").requests, 1);
    }
    // Cancel: the reader worker must deliver a Cancelled error and
    // close the channel promptly — not at the idle deadline.
    let t = Instant::now();
    cancel.cancel();
    let mut saw_cancelled = false;
    loop {
        match outcome.events.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(StreamEvent::Usage(_))) => {}
            Ok(Ok(StreamEvent::Token(_))) => {}
            Ok(Ok(StreamEvent::Completed { .. })) => {
                panic!("a cancelled stream must not complete normally")
            }
            Ok(Err(e)) => {
                assert_eq!(e.code, ProviderErrorCode::Cancelled, "{e:?}");
                saw_cancelled = true;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(e) => panic!("channel error: {e}"),
        }
        assert!(t.elapsed() < Duration::from_secs(10), "worker did not exit promptly");
    }
    server.release.store(true, Ordering::SeqCst);
    assert!(saw_cancelled, "the terminal error must be Cancelled");
    assert!(
        t.elapsed() < Duration::from_secs(10),
        "worker exit + terminal delivery took {:?}",
        t.elapsed()
    );
    // Accounting cleanup: the reader thread settles before the
    // channel closes, so by now the hold is gone and the request
    // is accounted (with zero tokens — no usage ever arrived).
    let l = ledger.lock().unwrap();
    assert_eq!(l.reserved(), (0, 0), "no reservation may leak");
    let totals = l.totals_for("ollama");
    assert_eq!(totals.requests, 1);
    assert_eq!(totals.output_tokens, 0);
}
