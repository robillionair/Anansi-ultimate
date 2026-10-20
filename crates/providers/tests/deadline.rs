//! The overall request deadline (`TimeoutConfig::total_ms`) must
//! bind *every* phase of a transport execution — resolution, connect,
//! TLS handshake, write, read — not just socket inactivity. The
//! dribbling-handshake test is the important one: a peer that sends
//! handshake bytes one at a time never trips a per-read timeout, so
//! only the overall deadline can stop it.

mod common;

use anansi_providers::config::TimeoutConfig;
use anansi_providers::dispatcher::Dispatcher;
use anansi_providers::error::ProviderErrorCode;
use anansi_providers::gates::{QuotaLedger, TargetLocality, ZeroCostLock};
use anansi_providers::http::CancellationToken;
use anansi_providers::routing::RoutingMode;
use anansi_providers::types::CostClass;
use common::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

/// A TCP proxy that forwards client→server traffic immediately but
/// dribbles server→client bytes one at a time. The bytes are a
/// genuine rustls handshake flight — valid, just pathologically
/// slow — so certificate validation is not what stops it.
fn dribble_proxy(target: std::net::SocketAddr, per_byte: Duration) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for inbound in listener.incoming() {
            let Ok(client_side) = inbound else { continue };
            let Ok(server_side) = TcpStream::connect(target) else {
                continue;
            };
            {
                let mut c = match client_side.try_clone() {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let mut s = match server_side.try_clone() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                std::thread::spawn(move || {
                    let _ = std::io::copy(&mut c, &mut s);
                });
            }
            let mut from = server_side;
            let mut to = client_side;
            std::thread::spawn(move || {
                let mut byte = [0u8; 1];
                loop {
                    match from.read(&mut byte) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            if to.write_all(&byte).is_err() {
                                break;
                            }
                            std::thread::sleep(per_byte);
                        }
                    }
                }
            });
        }
    });
    addr
}

fn tls_server_addr(server: &TlsSyntheticServer) -> std::net::SocketAddr {
    server
        .base_url
        .trim_start_matches("https://")
        .parse()
        .unwrap()
}

fn deadline_dispatcher<'a, T: anansi_providers::http::Transport>(
    transport: &'a T,
    resolver: &'a dyn anansi_providers::secrets::SecretResolver,
    timeouts: TimeoutConfig,
) -> Dispatcher<'a> {
    // max_retries: 0 isolates the transport deadline: a pre-send
    // Timeout is (correctly, per policy) retryable at the dispatcher
    // layer, which would multiply the elapsed time by the attempts.
    Dispatcher::new(transport, resolver, anansi_providers::catalog::default_adapters())
        .with_retry(anansi_providers::config::RetryPolicy {
            max_retries: 0,
            base_backoff_ms: 1,
            max_retry_after_ms: 10,
        })
        .with_timeouts(timeouts)
}

#[test]
fn dribbling_tls_handshake_is_bounded_by_the_overall_deadline() {
    let ca = TestCa::new();
    let (cert, key) = ca.valid_server();
    let server = TlsSyntheticServer::start(&cert, &key, |_req, _hit| {
        ServerResponse::json(200, openai_success_body())
    });
    // 3ms per byte: the genuine handshake flight (~2 KB) would take
    // ~6 s to arrive. Per-read timeouts never fire (bytes keep
    // coming); the 1.2 s overall deadline must.
    let proxy = dribble_proxy(tls_server_addr(&server), Duration::from_millis(3));
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = deadline_dispatcher(
        &transport,
        &res,
        TimeoutConfig {
            connect_ms: 500,
            first_byte_ms: 10_000,
            idle_ms: 10_000,
            total_ms: 1_200,
        },
    );
    let regs = vec![registration(
        "openai",
        format!("https://{proxy}/v1"),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", CostClass::Free)],
    )];
    let mut ledger = QuotaLedger::new();
    let started = Instant::now();
    let err = dispatcher
        .execute(
            &regs,
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    let elapsed = started.elapsed();
    assert_eq!(err.code, ProviderErrorCode::Timeout, "{err:?}");
    assert!(!err.request_sent, "the deadline fired before any request existed");
    assert!(
        elapsed < Duration::from_millis(3_000),
        "handshake must be cut at the deadline, took {elapsed:?}"
    );
    assert_eq!(server.hit_count(), 0, "no request reached the application");
}

#[test]
fn silent_server_handshake_times_out_pre_send() {
    // Accepts TCP and never speaks: the handshake read must expire
    // (first-byte bound inside the overall deadline).
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            std::thread::spawn(move || {
                let _s = stream;
                std::thread::sleep(Duration::from_secs(30));
            });
        }
    });
    let ca = TestCa::new();
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = deadline_dispatcher(
        &transport,
        &res,
        TimeoutConfig {
            connect_ms: 500,
            first_byte_ms: 800,
            idle_ms: 800,
            total_ms: 5_000,
        },
    );
    let regs = vec![registration(
        "openai",
        format!("https://{addr}/v1"),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", CostClass::Free)],
    )];
    let mut ledger = QuotaLedger::new();
    let started = Instant::now();
    let err = dispatcher
        .execute(
            &regs,
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::Timeout);
    assert!(!err.request_sent);
    assert!(started.elapsed() < Duration::from_millis(3_500));
}

#[test]
fn dribbling_response_body_is_bounded_by_the_overall_deadline() {
    // Headers arrive at once; the body then drips 1 byte / 100 ms —
    // each read succeeds, the idle timeout never fires, and only the
    // overall deadline (whose clock started before connect) ends it.
    let server = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(200, openai_success_body()).chunked(1, Duration::from_millis(100))
    });
    let transport = anansi_providers::http::StdHttpTransport;
    let res = resolver();
    let dispatcher = deadline_dispatcher(
        &transport,
        &res,
        TimeoutConfig {
            connect_ms: 200,
            first_byte_ms: 1_000,
            idle_ms: 1_000,
            total_ms: 1_200,
        },
    );
    let regs = vec![registration(
        "openai",
        format!("{}/v1", server.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", CostClass::Free)],
    )];
    let mut ledger = QuotaLedger::new();
    let started = Instant::now();
    let err = dispatcher
        .execute(
            &regs,
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::Timeout, "{err:?}");
    assert!(started.elapsed() < Duration::from_millis(3_000));
}

#[test]
fn fast_https_request_stays_well_within_the_deadline() {
    // Control: the deadline machinery must not disturb the happy path.
    let ca = TestCa::new();
    let (cert, key) = ca.valid_server();
    let server = TlsSyntheticServer::start(&cert, &key, |_req, _hit| {
        ServerResponse::json(200, openai_success_body())
    });
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = deadline_dispatcher(
        &transport,
        &res,
        TimeoutConfig {
            connect_ms: 500,
            first_byte_ms: 2_000,
            idle_ms: 1_000,
            total_ms: 5_000,
        },
    );
    let regs = vec![registration(
        "openai",
        format!("{}/v1", server.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", CostClass::Free)],
    )];
    let mut ledger = QuotaLedger::new();
    let outcome = dispatcher
        .execute(
            &regs,
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
}
