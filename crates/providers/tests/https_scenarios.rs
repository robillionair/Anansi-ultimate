//! Synthetic HTTPS scenarios against the crate's real
//! `RustlsTransport`. Certificates are test-only (rcgen, generated in
//! process, never on disk); the client trusts the Mozilla roots plus
//! the synthetic CA — validation is fully enabled, which is exactly
//! what the rejection tests prove.

mod common;

use anansi_providers::config::{RetryPolicy, TimeoutConfig};
use anansi_providers::dispatcher::Dispatcher;
use anansi_providers::error::ProviderErrorCode;
use anansi_providers::gates::{QuotaLedger, TargetLocality, ZeroCostLock};
use anansi_providers::http::{CancellationToken, RustlsTransport};
use anansi_providers::routing::RoutingMode;
use anansi_providers::types::{CostClass, StreamEvent};
use common::*;
use std::time::{Duration, Instant};

fn https_regs(
    server: &TlsSyntheticServer,
    provider: &str,
    cost: CostClass,
) -> Vec<anansi_providers::routing::ProviderRegistration> {
    let (base, model_id) = match provider {
        "anthropic" => (server.base_url.clone(), "claude-x"),
        _ => (format!("{}/v1", server.base_url), "gpt-x"),
    };
    vec![registration(
        provider,
        base,
        TargetLocality::Cloud,
        true,
        vec![model(model_id, provider, cost)],
    )]
}

fn tokens_text(events: &[Result<StreamEvent, anansi_providers::error::ProviderError>]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            Ok(StreamEvent::Token(t)) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

fn collect_stream(
    outcome: &anansi_providers::dispatcher::StreamOutcome,
    max_events: usize,
) -> Vec<Result<StreamEvent, anansi_providers::error::ProviderError>> {
    let mut events = Vec::new();
    while events.len() < max_events {
        match outcome.events.recv_timeout(Duration::from_secs(10)) {
            Ok(ev) => {
                let done = matches!(ev, Ok(StreamEvent::Completed { .. }) | Err(_));
                events.push(ev);
                if done {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    events
}

#[test]
fn https_success_with_valid_certificate() {
    let ca = TestCa::new();
    let (cert, key) = ca.valid_server();
    let server = TlsSyntheticServer::start(&cert, &key, |_req, _hit| {
        ServerResponse::json(200, openai_success_body())
    });
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let outcome = dispatcher
        .execute(
            &https_regs(&server, "openai", CostClass::Free),
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(outcome.response.usage.unwrap().total(), 18);
    assert_eq!(server.hit_count(), 1);
    // The credential travelled inside the validated TLS session.
    assert_eq!(
        server.recorded()[0].header("authorization"),
        Some(format!("Bearer {FAKE_KEY}").as_str())
    );
}

#[test]
fn https_anthropic_success_with_api_key_header() {
    let ca = TestCa::new();
    let (cert, key) = ca.valid_server();
    let server = TlsSyntheticServer::start(&cert, &key, |_req, _hit| {
        ServerResponse::json(
            200,
            r#"{"model":"claude-x","content":[{"type":"text","text":"anthropic over tls"}],"stop_reason":"end_turn","usage":{"input_tokens":9,"output_tokens":4}}"#,
        )
    });
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let mut c = test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(test_permit("anthropic", "claude-x"));
    let outcome = dispatcher
        .execute(
            &https_regs(&server, "anthropic", CostClass::Paid),
            &test_request("claude-x"),
            c,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "anthropic over tls");
    assert_eq!(server.recorded()[0].header("x-api-key"), Some(FAKE_KEY));
}

#[test]
fn https_wrong_host_certificate_is_rejected_before_any_request() {
    let ca = TestCa::new();
    let (cert, key) = ca.wrong_host_server();
    let server = TlsSyntheticServer::start(&cert, &key, |_req, _hit| {
        ServerResponse::json(200, openai_success_body())
    });
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let err = dispatcher
        .execute(
            &https_regs(&server, "openai", CostClass::Free),
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::TlsError);
    assert!(
        !err.request_sent,
        "handshake failure is pre-send by construction"
    );
    assert_eq!(
        server.hit_count(),
        0,
        "no application request may reach a wrong-host endpoint"
    );
}

#[test]
fn https_expired_certificate_is_rejected() {
    let ca = TestCa::new();
    let (cert, key) = ca.expired_server();
    let server = TlsSyntheticServer::start(&cert, &key, |_req, _hit| {
        ServerResponse::json(200, openai_success_body())
    });
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let err = dispatcher
        .execute(
            &https_regs(&server, "openai", CostClass::Free),
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::TlsError);
    assert_eq!(server.hit_count(), 0);
}

#[test]
fn https_untrusted_certificate_is_rejected() {
    let (cert, key) = self_signed_server();
    let server = TlsSyntheticServer::start(&cert, &key, |_req, _hit| {
        ServerResponse::json(200, openai_success_body())
    });
    // Production default trust (Mozilla roots only): a self-signed
    // certificate must not validate, and there is no bypass to enable.
    let transport = RustlsTransport::new();
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let err = dispatcher
        .execute(
            &https_regs(&server, "openai", CostClass::Free),
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::TlsError);
    assert_eq!(server.hit_count(), 0);
    // The error names the endpoint and the problem, never a secret.
    assert!(!err.message.contains(FAKE_KEY));
}

#[test]
fn https_streaming_sse_over_tls() {
    let ca = TestCa::new();
    let (cert, key) = ca.valid_server();
    let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"hello \"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"tls stream\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    let server = TlsSyntheticServer::start(&cert, &key, move |_req, _hit| {
        ServerResponse::text(200, sse).chunked(4, Duration::ZERO)
    });
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let outcome = dispatcher
        .execute_stream(
            &https_regs(&server, "openai", CostClass::Free),
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap();
    let events = collect_stream(&outcome, 12);
    assert_eq!(tokens_text(&events), "hello tls stream");
    assert!(matches!(
        events.last(),
        Some(Ok(StreamEvent::Completed { .. }))
    ));
}

#[test]
fn https_cancellation_closes_the_tls_session() {
    let ca = TestCa::new();
    let (cert, key) = ca.valid_server();
    let mut body = String::new();
    for _ in 0..80 {
        body.push_str("data: {\"choices\":[{\"delta\":{\"content\":\"tok \"}}]}\n\n");
    }
    let server = TlsSyntheticServer::start(&cert, &key, move |_req, _hit| {
        ServerResponse::text(200, &body).chunked(24, Duration::from_millis(120))
    });
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let cancel = CancellationToken::new();
    let mut ledger = QuotaLedger::new();
    let outcome = dispatcher
        .execute_stream(
            &https_regs(&server, "openai", CostClass::Free),
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &cancel,
        )
        .unwrap();
    let first = outcome.events.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(first, Ok(StreamEvent::Token(_))), "{first:?}");
    let started = Instant::now();
    cancel.cancel();
    let mut saw_cancelled = false;
    while started.elapsed() < Duration::from_secs(4) {
        match outcome.events.recv_timeout(Duration::from_secs(4)) {
            Ok(Err(e)) => {
                assert_eq!(e.code, ProviderErrorCode::Cancelled);
                saw_cancelled = true;
                break;
            }
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    assert!(
        saw_cancelled,
        "cancellation must close the TLS session promptly"
    );
}

#[test]
fn https_redirect_is_never_followed_and_credentials_stay_put() {
    let ca = TestCa::new();
    let (cert, key) = ca.valid_server();
    let server = TlsSyntheticServer::start(&cert, &key, |_req, _hit| {
        let mut r = ServerResponse::text(302, "");
        r.headers.push((
            "Location".to_string(),
            "https://127.0.0.1:9/evil".to_string(),
        ));
        r
    });
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let err = dispatcher
        .execute(
            &https_regs(&server, "openai", CostClass::Free),
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::EndpointRejected);
    assert_eq!(
        server.hit_count(),
        1,
        "the redirect target must never be contacted"
    );
}

#[test]
fn https_timeout_remains_bounded() {
    let ca = TestCa::new();
    let (cert, key) = ca.valid_server();
    let server = TlsSyntheticServer::start(&cert, &key, |_req, _hit| {
        ServerResponse::json(200, openai_success_body()).delayed(Duration::from_secs(3))
    });
    let transport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = Dispatcher::new(
        &transport,
        &res,
        anansi_providers::catalog::default_adapters(),
    )
    .with_retry(RetryPolicy {
        max_retries: 0,
        base_backoff_ms: 1,
        max_retry_after_ms: 10,
    })
    .with_timeouts(TimeoutConfig {
        connect_ms: 500,
        first_byte_ms: 500,
        idle_ms: 500,
        total_ms: 1500,
    });
    let mut ledger = QuotaLedger::new();
    let started = Instant::now();
    let err = dispatcher
        .execute(
            &https_regs(&server, "openai", CostClass::Free),
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::Timeout);
    assert!(started.elapsed() < Duration::from_millis(2800));
}
