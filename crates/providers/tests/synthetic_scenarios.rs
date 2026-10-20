//! End-to-end dispatcher scenarios against local synthetic servers.
//! Every request stays on loopback; every credential is fake. Gate
//! tests assert the server's hit count is exactly zero — that is the
//! observable proof a refusal happened before any network request.

mod common;

use anansi_providers::config::{RetryPolicy, TimeoutConfig};
use anansi_providers::dispatcher::{DispatchContext, Dispatcher};
use anansi_providers::error::ProviderErrorCode;
use anansi_providers::gates::{
    Budget, PaidInferencePermit, QuotaLedger, TargetLocality, ZeroCostLock,
};
use anansi_providers::http::{CancellationToken, StdHttpTransport};
use anansi_providers::routing::{RoutingMode, RoutingRequest};
use anansi_providers::types::{
    ChatMessage, CompletionRequest, CostClass, PrivacyClass, Role, Segment, StreamEvent,
};
use common::*;
use std::time::{Duration, Instant};

fn ctx(
    ledger: std::sync::Arc<std::sync::Mutex<QuotaLedger>>,
    mode: RoutingMode,
    lock: ZeroCostLock,
) -> DispatchContext {
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

fn permit(provider: &str, model: &str) -> PaidInferencePermit {
    PaidInferencePermit {
        id: format!("permit-{provider}-{model}"),
        provider: provider.to_string(),
        model: model.to_string(),
        max_estimated_total_tokens: 100_000,
        allow_unknown_cost: false,
    }
}

fn user_request(model: &str) -> CompletionRequest {
    CompletionRequest::new(model, vec![ChatMessage::new(Role::User, "Say hello")])
}

fn cloud_openai_regs(
    server: &SyntheticServer,
    models: Vec<anansi_providers::types::ModelInfo>,
) -> Vec<anansi_providers::routing::ProviderRegistration> {
    vec![registration(
        "openai",
        format!("{}/v1", server.base_url),
        TargetLocality::Cloud,
        true,
        models,
    )]
}

// ---------------------------------------------------------------- success

#[test]
fn openai_success_records_usage_and_disclosure() {
    let server =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Paid)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let mut c = ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(permit("openai", "gpt-x"));
    let outcome = dispatcher
        .execute(&regs, &user_request("gpt-x"), c, &CancellationToken::new())
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(outcome.response.usage.unwrap().total(), 18);
    assert_eq!(outcome.disclosure.provider, "openai");
    assert_eq!(outcome.disclosure.endpoint_host, "127.0.0.1");
    assert!(!outcome.disclosure.local);
    assert_eq!(ledger.lock().unwrap().total_requests(), 1);
    assert_eq!(ledger.lock().unwrap().totals_for("openai").input_tokens, 12);
    // The wire request carried the fake key and the right path.
    let recorded = server.recorded();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].path, "/v1/chat/completions");
    assert_eq!(
        recorded[0].header("authorization"),
        Some(format!("Bearer {FAKE_KEY}").as_str())
    );
}

#[test]
fn ollama_local_success_without_credentials() {
    let server = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(
            200,
            r#"{"model":"qwen2.5:0.5b","message":{"role":"assistant","content":"local hi"},"done":true,"done_reason":"stop","prompt_eval_count":5,"eval_count":2}"#,
        )
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![registration(
        "ollama",
        server.base_url.clone(),
        TargetLocality::Local,
        false,
        vec![model("qwen2.5:0.5b", "ollama", CostClass::Free)],
    )];
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let outcome = dispatcher
        .execute(
            &regs,
            &user_request("qwen2.5:0.5b"),
            ctx(ledger.clone(), RoutingMode::Private, ZeroCostLock::on()),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "local hi");
    assert!(outcome.disclosure.local);
    assert!(server.recorded()[0].header("authorization").is_none());
}

#[test]
fn anthropic_success_with_api_key_header() {
    let server = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(
            200,
            r#"{"model":"claude-x","content":[{"type":"text","text":"anthropic hi"}],"stop_reason":"end_turn","usage":{"input_tokens":9,"output_tokens":4}}"#,
        )
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![registration(
        "anthropic",
        server.base_url.clone(),
        TargetLocality::Cloud,
        true,
        vec![model("claude-x", "anthropic", CostClass::Paid)],
    )];
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let mut c = ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(permit("anthropic", "claude-x"));
    let outcome = dispatcher
        .execute(
            &regs,
            &user_request("claude-x"),
            c,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "anthropic hi");
    let recorded = server.recorded();
    assert_eq!(recorded[0].path, "/v1/messages");
    assert_eq!(recorded[0].header("x-api-key"), Some(FAKE_KEY));
    assert_eq!(recorded[0].header("anthropic-version"), Some("2023-06-01"));
}

#[test]
fn gemini_success_with_header_key_not_url() {
    let server = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(
            200,
            r#"{"candidates":[{"content":{"parts":[{"text":"gemini hi"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":7,"candidatesTokenCount":3}}"#,
        )
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![registration(
        "gemini",
        format!("{}/v1beta", server.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gemini-x", "gemini", CostClass::Paid)],
    )];
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let mut c = ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(permit("gemini", "gemini-x"));
    let outcome = dispatcher
        .execute(
            &regs,
            &user_request("gemini-x"),
            c,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "gemini hi");
    let recorded = server.recorded();
    assert_eq!(recorded[0].path, "/v1beta/models/gemini-x:generateContent");
    assert!(!recorded[0].path.contains(FAKE_KEY));
    assert_eq!(recorded[0].header("x-goog-api-key"), Some(FAKE_KEY));
}

#[test]
fn azure_uses_deployment_path_and_api_key() {
    let server =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![registration(
        "azure",
        server.base_url.clone(),
        TargetLocality::Cloud,
        true,
        vec![model("dep-1", "azure", CostClass::Paid)],
    )];
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let mut c = ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(permit("azure", "dep-1"));
    let outcome = dispatcher
        .execute(&regs, &user_request("dep-1"), c, &CancellationToken::new())
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    let recorded = server.recorded();
    assert_eq!(
        recorded[0].path,
        "/openai/deployments/dep-1/chat/completions?api-version=2024-10-21"
    );
    assert_eq!(recorded[0].header("api-key"), Some(FAKE_KEY));
}

// ---------------------------------------------------------------- failures

#[test]
fn malformed_success_body_is_a_clean_error() {
    let server = SyntheticServer::start(|_req, _hit| ServerResponse::text(200, "this is not json"));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::MalformedResponse);
    assert_eq!(server.hit_count(), 1, "malformed responses are not retried");
}

#[test]
fn auth_failure_not_retried_and_never_leaks_the_key() {
    let server = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(
            401,
            &format!(r#"{{"error":{{"message":"key {FAKE_KEY} was rejected"}}}}"#),
        )
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::AuthFailed);
    assert_eq!(server.hit_count(), 1);
    assert!(
        !err.message.contains(FAKE_KEY),
        "error leaked key: {}",
        err.message
    );
    assert!(err.message.contains("[REDACTED]"));
}

#[test]
fn rate_limit_retries_then_succeeds() {
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
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let outcome = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(server.hit_count(), 2);
}

#[test]
fn rate_limit_exhaustion_is_bounded() {
    let server = SyntheticServer::start(|_req, _hit| {
        let mut r = ServerResponse::json(429, r#"{"error":{"message":"quota"}}"#);
        r.headers.push(("Retry-After".to_string(), "0".to_string()));
        r
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::RateLimited);
    assert_eq!(server.hit_count(), 3, "1 initial + 2 bounded retries");
}

#[test]
fn server_error_echo_is_scrubbed() {
    let server = SyntheticServer::start(|req, _hit| {
        let echoed = req.header("authorization").unwrap_or("").to_string();
        ServerResponse::json(
            500,
            &format!(r#"{{"error":{{"message":"boom, saw {echoed}"}}}}"#),
        )
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ProviderUnavailable);
    assert!(!format!("{err}").contains(FAKE_KEY), "leaked: {err}");
    assert_eq!(server.hit_count(), 3, "5xx is retried within the bound");
}

#[test]
fn timeout_is_bounded() {
    let server = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(200, openai_success_body()).delayed(Duration::from_secs(3))
    });
    let transport = StdHttpTransport;
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
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let started = Instant::now();
    let err = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::Timeout);
    assert!(
        started.elapsed() < Duration::from_millis(2800),
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn connection_drop_and_refusal_are_connection_lost() {
    let dropping = SyntheticServer::start(|_req, _hit| ServerResponse {
        status: 200,
        headers: vec![],
        body: vec![],
        chunked_piece: None,
        piece_delay: Duration::ZERO,
        delay_before: Duration::ZERO,
        drop: true,
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = Dispatcher::new(
        &transport,
        &res,
        anansi_providers::catalog::default_adapters(),
    )
    .with_retry(RetryPolicy {
        max_retries: 1,
        base_backoff_ms: 1,
        max_retry_after_ms: 10,
    });
    let regs = cloud_openai_regs(&dropping, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ConnectionLost);
    assert_eq!(dropping.hit_count(), 2, "1 initial + 1 retry");

    // Refused: a port nothing listens on.
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let p = l.local_addr().unwrap().port();
        drop(l);
        p
    };
    let regs = vec![registration(
        "openai",
        format!("http://127.0.0.1:{port}/v1"),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", CostClass::Free)],
    )];
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ConnectionLost);
}

#[test]
fn redirect_is_never_followed() {
    let server = SyntheticServer::start(|_req, _hit| {
        let mut r = ServerResponse::text(302, "");
        r.headers.push((
            "Location".to_string(),
            "http://127.0.0.1:9/evil".to_string(),
        ));
        r
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::EndpointRejected);
    assert_eq!(server.hit_count(), 1);
}

#[test]
fn https_endpoint_fails_closed_without_tls_transport() {
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![registration(
        "openai",
        "https://api.openai.com/v1".to_string(),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", CostClass::Free)],
    )];
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::TlsUnavailable);
}

// ---------------------------------------------------------------- gates (zero network hits)

#[test]
fn paid_without_permit_is_refused_before_network() {
    let server =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Paid)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::PermissionRefused);
    assert!(err.refused_before_network());
    assert_eq!(server.hit_count(), 0);
}

#[test]
fn zero_cost_lock_blocks_paid_but_allows_free() {
    let server =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(
        &server,
        vec![
            model("gpt-x", "openai", CostClass::Paid),
            model("free-x", "openai", CostClass::Free),
        ],
    );
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    // Even an explicit user choice of the paid model is filtered out of
    // the plan by the engaged lock: no candidate, no network.
    let mut c = ctx(ledger.clone(), RoutingMode::ChooseModel, ZeroCostLock::on());
    c.routing.chosen = Some(("openai".to_string(), "gpt-x".to_string()));
    let _err = dispatcher
        .execute(&regs, &user_request("gpt-x"), c, &CancellationToken::new())
        .unwrap_err();
    assert_eq!(server.hit_count(), 0);
    // Free model under the lock succeeds — the lock is not a blanket ban.
    let c = ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::on());
    let outcome = dispatcher
        .execute(&regs, &user_request("free-x"), c, &CancellationToken::new())
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(server.hit_count(), 1);
}

#[test]
fn sealed_content_is_refused_for_local_and_cloud() {
    let server =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let sealed = CompletionRequest::new(
        "gpt-x",
        vec![ChatMessage {
            role: Role::User,
            segments: vec![Segment::classified("vault material", PrivacyClass::Sealed)],
        }],
    );
    let cloud = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &cloud,
            &sealed,
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::PrivacyRefused);
    let local = vec![registration(
        "ollama",
        server.base_url.clone(),
        TargetLocality::Local,
        false,
        vec![model("gpt-x", "ollama", CostClass::Free)],
    )];
    let mut sealed_local = sealed.clone();
    sealed_local.model = "gpt-x".to_string();
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &local,
            &sealed_local,
            ctx(ledger.clone(), RoutingMode::Private, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::PrivacyRefused);
    assert_eq!(server.hit_count(), 0, "Sealed must never reach a socket");
}

#[test]
fn private_content_needs_explicit_cloud_authorization() {
    let server =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let private_req = CompletionRequest::new(
        "gpt-x",
        vec![ChatMessage {
            role: Role::User,
            segments: vec![Segment::classified(
                "my private notes",
                PrivacyClass::Private,
            )],
        }],
    );
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &private_req,
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::PrivacyRefused);
    assert_eq!(server.hit_count(), 0);
    let mut authorized = private_req.clone();
    authorized.explicit_cloud_authorization = true;
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    dispatcher
        .execute(
            &regs,
            &authorized,
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(server.hit_count(), 1);
}

#[test]
fn budget_exhaustion_is_refused_before_network() {
    let server =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let mut c = ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off());
    c.budget = Budget {
        max_total_tokens: Some(10),
        ..Default::default()
    };
    let err = dispatcher
        .execute(&regs, &user_request("gpt-x"), c, &CancellationToken::new())
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::QuotaExceeded);
    assert_eq!(server.hit_count(), 0);
}

#[test]
fn permit_is_single_use() {
    let server =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Paid)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let mut c = ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(permit("openai", "gpt-x"));
    dispatcher
        .execute(&regs, &user_request("gpt-x"), c, &CancellationToken::new())
        .unwrap();
    assert_eq!(server.hit_count(), 1);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let mut c = ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off());
    c.permit = Some(permit("openai", "gpt-x"));
    let err = dispatcher
        .execute(&regs, &user_request("gpt-x"), c, &CancellationToken::new())
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::PermissionRefused);
    assert_eq!(
        server.hit_count(),
        1,
        "a consumed permit must not dispatch again"
    );
}

// ---------------------------------------------------------------- fallback

#[test]
fn fallback_reaches_second_provider_after_503() {
    let down = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(503, r#"{"error":{"message":"down"}}"#)
    });
    let up = SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![
        registration(
            "openai",
            format!("{}/v1", down.base_url),
            TargetLocality::Cloud,
            true,
            vec![model("gpt-x", "openai", CostClass::Free)],
        ),
        registration(
            "openai",
            format!("{}/v1", up.base_url),
            TargetLocality::Cloud,
            true,
            vec![model("gpt-x", "openai", CostClass::Free)],
        ),
    ];
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let outcome = dispatcher
        .execute(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(
        down.hit_count(),
        3,
        "first candidate exhausts bounded retries"
    );
    assert_eq!(up.hit_count(), 1);
    assert!(
        outcome
            .disclosure
            .attempts
            .iter()
            .any(|a| a.outcome.contains("ProviderUnavailable")),
        "attempts should record the first candidate's failure: {:?}",
        outcome.disclosure.attempts
    );
}

#[test]
fn fallback_never_crosses_from_free_to_paid_under_lock() {
    let down = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(503, r#"{"error":{"message":"down"}}"#)
    });
    let paid =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![
        registration(
            "openai",
            format!("{}/v1", down.base_url),
            TargetLocality::Cloud,
            true,
            vec![model("free-x", "openai", CostClass::Free)],
        ),
        registration(
            "openai",
            format!("{}/v1", paid.base_url),
            TargetLocality::Cloud,
            true,
            vec![model("gpt-x", "openai", CostClass::Paid)],
        ),
    ];
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let err = dispatcher
        .execute(
            &regs,
            &user_request("free-x"),
            ctx(ledger.clone(), RoutingMode::AutoFree, ZeroCostLock::on()),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ProviderUnavailable);
    assert_eq!(
        paid.hit_count(),
        0,
        "free-to-paid fallback is blocked by the lock (§16)"
    );
}

// ---------------------------------------------------------------- streaming

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

fn tokens_text(events: &[Result<StreamEvent, anansi_providers::error::ProviderError>]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            Ok(StreamEvent::Token(t)) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn openai_streaming_over_dribbled_chunks() {
    let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"hello \"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"stream\"},\"finish_reason\":\"stop\"}]}\n\ndata: {\"choices\":[],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":4}}\n\ndata: [DONE]\n\n";
    let server = SyntheticServer::start(move |_req, _hit| {
        ServerResponse::text(200, sse).chunked(3, Duration::ZERO)
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let outcome = dispatcher
        .execute_stream(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap();
    let events = collect_stream(&outcome, 16);
    assert_eq!(tokens_text(&events), "hello stream");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Ok(StreamEvent::Usage(u)) if u.total() == 14))
    );
    assert!(
        matches!(events.last(), Some(Ok(StreamEvent::Completed { finish_reason: Some(r) })) if r == "stop")
    );
    // The request really asked for streaming.
    assert!(server.recorded()[0].body.contains("\"stream\":true"));
}

#[test]
fn ollama_streaming_ndjson() {
    let ndjson = "{\"message\":{\"content\":\"Hel\"},\"done\":false}\n{\"message\":{\"content\":\"lo\"},\"done\":true,\"done_reason\":\"stop\",\"prompt_eval_count\":4,\"eval_count\":2}\n";
    let server = SyntheticServer::start(move |_req, _hit| {
        ServerResponse::text(200, ndjson).chunked(5, Duration::ZERO)
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![registration(
        "ollama",
        server.base_url.clone(),
        TargetLocality::Local,
        false,
        vec![model("qwen2.5:0.5b", "ollama", CostClass::Free)],
    )];
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let outcome = dispatcher
        .execute_stream(
            &regs,
            &user_request("qwen2.5:0.5b"),
            ctx(ledger.clone(), RoutingMode::Private, ZeroCostLock::on()),
            &CancellationToken::new(),
        )
        .unwrap();
    let events = collect_stream(&outcome, 16);
    assert_eq!(tokens_text(&events), "Hello");
    assert!(matches!(
        events.last(),
        Some(Ok(StreamEvent::Completed { .. }))
    ));
}

#[test]
fn streaming_cancellation_closes_the_stream() {
    let mut body = String::new();
    for _ in 0..80 {
        body.push_str("data: {\"choices\":[{\"delta\":{\"content\":\"tok \"}}]}\n\n");
    }
    let server = SyntheticServer::start(move |_req, _hit| {
        ServerResponse::text(200, &body).chunked(24, Duration::from_millis(120))
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = cloud_openai_regs(&server, vec![model("gpt-x", "openai", CostClass::Free)]);
    let cancel = CancellationToken::new();
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let outcome = dispatcher
        .execute_stream(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &cancel,
        )
        .unwrap();
    // Read the first token, then cancel.
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
        "stream must report Cancelled promptly after cancel()"
    );
    assert!(started.elapsed() < Duration::from_secs(4));
}

#[test]
fn streaming_falls_back_before_first_byte() {
    let down = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(503, r#"{"error":{"message":"down"}}"#)
    });
    let sse =
        "data: {\"choices\":[{\"delta\":{\"content\":\"from fallback\"}}]}\n\ndata: [DONE]\n\n";
    let up = SyntheticServer::start(move |_req, _hit| ServerResponse::text(200, sse));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let regs = vec![
        registration(
            "openai",
            format!("{}/v1", down.base_url),
            TargetLocality::Cloud,
            true,
            vec![model("gpt-x", "openai", CostClass::Free)],
        ),
        registration(
            "openai",
            format!("{}/v1", up.base_url),
            TargetLocality::Cloud,
            true,
            vec![model("gpt-x", "openai", CostClass::Free)],
        ),
    ];
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(QuotaLedger::new()));
    let outcome = dispatcher
        .execute_stream(
            &regs,
            &user_request("gpt-x"),
            ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap();
    let events = collect_stream(&outcome, 8);
    assert_eq!(tokens_text(&events), "from fallback");
}
