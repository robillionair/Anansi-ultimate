//! Billable-retry safety: ambiguous failures after a request was
//! sent must not automatically duplicate paid inference. Every test
//! asserts exact synthetic-server hit counts — the observable proof
//! of how many times a billable request actually left this process.

mod common;

use anansi_providers::config::{RetryPolicy, TimeoutConfig};
use anansi_providers::dispatcher::Dispatcher;
use anansi_providers::error::ProviderErrorCode;
use anansi_providers::gates::{QuotaLedger, TargetLocality, ZeroCostLock};
use anansi_providers::http::{CancellationToken, RustlsTransport, StdHttpTransport};
use anansi_providers::routing::RoutingMode;
use anansi_providers::types::CostClass;
use common::*;
use std::time::Duration;

fn paid_regs(
    server: &SyntheticServer,
    cost: CostClass,
) -> Vec<anansi_providers::routing::ProviderRegistration> {
    vec![registration(
        "openai",
        format!("{}/v1", server.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", cost)],
    )]
}

fn two_paid_regs(
    a: &SyntheticServer,
    b: &SyntheticServer,
    cost: CostClass,
) -> Vec<anansi_providers::routing::ProviderRegistration> {
    vec![
        registration(
            "openai",
            format!("{}/v1", a.base_url),
            TargetLocality::Cloud,
            true,
            vec![model("gpt-x", "openai", cost)],
        ),
        registration(
            "openai",
            format!("{}/v1", b.base_url),
            TargetLocality::Cloud,
            true,
            vec![model("gpt-x", "openai", cost)],
        ),
    ]
}

#[test]
fn paid_5xx_is_never_retried() {
    let server = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(503, r#"{"error":{"message":"down"}}"#)
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res); // rig allows 2 retries
    let mut ledger = QuotaLedger::new();
    let mut c = test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(test_permit("openai", "gpt-x"));
    let err = dispatcher
        .execute(
            &paid_regs(&server, CostClass::Paid),
            &test_request("gpt-x"),
            c,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ProviderUnavailable);
    assert_eq!(
        server.hit_count(),
        1,
        "a billable 5xx may already have executed; retrying could double-bill"
    );
}

#[test]
fn unknown_cost_is_treated_as_billable() {
    let server = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(500, r#"{"error":{"message":"boom"}}"#)
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let mut c = test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off());
    let mut p = test_permit("openai", "gpt-x");
    p.allow_unknown_cost = true;
    c.permit = Some(p);
    let err = dispatcher
        .execute(
            &paid_regs(&server, CostClass::Unknown),
            &test_request("gpt-x"),
            c,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ProviderUnavailable);
    assert_eq!(server.hit_count(), 1);
}

#[test]
fn paid_5xx_does_not_fall_back_to_another_provider() {
    let down = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(503, r#"{"error":{"message":"down"}}"#)
    });
    let up = SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let mut c = test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(test_permit("openai", "gpt-x"));
    let err = dispatcher
        .execute(
            &two_paid_regs(&down, &up, CostClass::Paid),
            &test_request("gpt-x"),
            c,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ProviderUnavailable);
    assert_eq!(down.hit_count(), 1);
    assert_eq!(
        up.hit_count(),
        0,
        "fallback would just move the possible duplicate charge"
    );
}

#[test]
fn paid_drop_after_send_is_never_retried() {
    let dropping = SyntheticServer::start(|_req, _hit| ServerResponse {
        status: 200,
        headers: vec![],
        body: vec![],
        chunked_piece: None,
        piece_delay: Duration::ZERO,
        delay_before: Duration::ZERO,
        drop: true, // server read the full request, then vanished
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let mut c = test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(test_permit("openai", "gpt-x"));
    let err = dispatcher
        .execute(
            &paid_regs(&dropping, CostClass::Paid),
            &test_request("gpt-x"),
            c,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ConnectionLost);
    assert_eq!(
        dropping.hit_count(),
        1,
        "the request was sent; the drop is ambiguous"
    );
}

#[test]
fn paid_timeout_is_never_retried() {
    let slow = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(200, openai_success_body()).delayed(Duration::from_secs(3))
    });
    let transport = StdHttpTransport;
    let res = resolver();
    // Retry budget exists; the billable-safety policy must override it.
    let dispatcher = Dispatcher::new(
        &transport,
        &res,
        anansi_providers::catalog::default_adapters(),
    )
    .with_retry(RetryPolicy {
        max_retries: 2,
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
    let mut c = test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(test_permit("openai", "gpt-x"));
    let err = dispatcher
        .execute(
            &paid_regs(&slow, CostClass::Paid),
            &test_request("gpt-x"),
            c,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::Timeout);
    assert_eq!(slow.hit_count(), 1);
}

#[test]
fn paid_429_is_retried_because_it_proves_non_execution() {
    let server = SyntheticServer::start(|_req, hit| {
        if hit == 0 {
            let mut r = ServerResponse::json(429, r#"{"error":{"message":"slow down"}}"#);
            r.headers.push(("Retry-After".to_string(), "0".to_string()));
            r
        } else {
            ServerResponse::json(200, openai_success_body())
        }
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let mut c = test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(test_permit("openai", "gpt-x"));
    let outcome = dispatcher
        .execute(
            &paid_regs(&server, CostClass::Paid),
            &test_request("gpt-x"),
            c,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(server.hit_count(), 2);
}

#[test]
fn paid_pre_send_failure_still_falls_back() {
    // First candidate: a port nothing listens on (connection refused
    // = the request was never sent, so falling back cannot duplicate
    // anything).
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let p = l.local_addr().unwrap().port();
        drop(l);
        p
    };
    let up = SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![
        registration(
            "openai",
            format!("http://127.0.0.1:{port}/v1"),
            TargetLocality::Cloud,
            true,
            vec![model("gpt-x", "openai", CostClass::Paid)],
        ),
        registration(
            "openai",
            format!("{}/v1", up.base_url),
            TargetLocality::Cloud,
            true,
            vec![model("gpt-x", "openai", CostClass::Paid)],
        ),
    ];
    let mut ledger = QuotaLedger::new();
    let mut c = test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(test_permit("openai", "gpt-x"));
    let outcome = dispatcher
        .execute(&regs, &test_request("gpt-x"), c, &CancellationToken::new())
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(up.hit_count(), 1);
}

#[test]
fn free_5xx_keeps_availability_retry_and_fallback() {
    let down = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(503, r#"{"error":{"message":"down"}}"#)
    });
    let up = SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let outcome = dispatcher
        .execute(
            &two_paid_regs(&down, &up, CostClass::Free),
            &test_request("gpt-x"),
            test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(
        down.hit_count(),
        3,
        "free retries are bounded but allowed (1 + 2)"
    );
    assert_eq!(up.hit_count(), 1);
}

#[test]
fn paid_stream_establishment_5xx_does_not_fall_back() {
    let down = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(503, r#"{"error":{"message":"down"}}"#)
    });
    let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"streamed\"}}]}\n\ndata: [DONE]\n\n";
    let up = SyntheticServer::start(move |_req, _hit| ServerResponse::text(200, sse));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ledger = QuotaLedger::new();
    let mut c = test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(test_permit("openai", "gpt-x"));
    let err = dispatcher
        .execute_stream(
            &two_paid_regs(&down, &up, CostClass::Paid),
            &test_request("gpt-x"),
            c,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ProviderUnavailable);
    assert_eq!(down.hit_count(), 1);
    assert_eq!(up.hit_count(), 0);
}

// The TLS transport obeys the same policy (proved over HTTPS too).
#[test]
fn paid_5xx_over_https_is_a_single_attempt() {
    let ca = TestCa::new();
    let (cert, key) = ca.valid_server();
    let server = TlsSyntheticServer::start(&cert, &key, |_req, _hit| {
        ServerResponse::json(503, r#"{"error":{"message":"down"}}"#)
    });
    let transport: RustlsTransport = tls_transport(&ca);
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![registration(
        "openai",
        format!("{}/v1", server.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", CostClass::Paid)],
    )];
    let mut ledger = QuotaLedger::new();
    let mut c = test_ctx(&mut ledger, RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(test_permit("openai", "gpt-x"));
    let err = dispatcher
        .execute(&regs, &test_request("gpt-x"), c, &CancellationToken::new())
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ProviderUnavailable);
    assert_eq!(server.hit_count(), 1);
}
