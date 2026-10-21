//! Wire binding: the request transmitted to a provider must name
//! the SELECTED CANDIDATE's model — the same provider/model the
//! privacy and cost gates, the permit, the budget estimate and the
//! disclosure describe. Before this binding, adapters built the
//! wire body from the caller's requested model, so a dispatch
//! routed to a free candidate under the Zero-Cost Lock would still
//! transmit the requested paid model.

mod common;

use anansi_providers::dispatcher::Dispatcher;
use anansi_providers::error::ProviderErrorCode;
use anansi_providers::gates::{QuotaLedger, TargetLocality, ZeroCostLock};
use anansi_providers::http::{CancellationToken, StdHttpTransport};
use anansi_providers::routing::RoutingMode;
use anansi_providers::types::{CostClass, StreamEvent};
use common::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn ledger() -> Arc<Mutex<QuotaLedger>> {
    Arc::new(Mutex::new(QuotaLedger::new()))
}

fn paid_cloud() -> (
    SyntheticServer,
    anansi_providers::routing::ProviderRegistration,
) {
    let server =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let reg = registration(
        "openai",
        format!("{}/v1", server.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-paid", "openai", CostClass::Paid)],
    );
    (server, reg)
}

#[test]
fn zero_cost_lock_dispatch_transmits_the_free_candidate_not_the_requested_paid_model() {
    let (paid_server, paid_reg) = paid_cloud();
    let free_server = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(
            200,
            "{\"model\":\"llama-free\",\"message\":{\"role\":\"assistant\",\"content\":\"hello from synthetic\"},\"done\":true,\"prompt_eval_count\":12,\"eval_count\":6}",
        )
    });
    let free_reg = registration(
        "ollama",
        free_server.base_url.clone(),
        TargetLocality::Local,
        false,
        vec![model("llama-free", "ollama", CostClass::Free)],
    );
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    // The caller asks for the PAID model; the Zero-Cost Lock
    // refuses that candidate before network and routing falls to
    // the free local candidate.
    let outcome = dispatcher
        .execute(
            &[paid_reg, free_reg],
            &test_request("gpt-paid"),
            test_ctx(ledger(), RoutingMode::Auto, ZeroCostLock::on()),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(outcome.disclosure.provider, "ollama");
    assert_eq!(outcome.disclosure.model, "llama-free");
    assert_eq!(
        paid_server.hit_count(),
        0,
        "paid candidate refused pre-network"
    );
    let recorded = free_server.recorded();
    assert_eq!(recorded.len(), 1);
    assert!(
        recorded[0].body.contains("\"model\":\"llama-free\""),
        "the wire request must name the routed free model, got: {}",
        recorded[0].body
    );
    assert!(
        !recorded[0].body.contains("gpt-paid"),
        "the requested paid model must not reach the wire: {}",
        recorded[0].body
    );
}

#[test]
fn fallback_transmits_each_candidates_own_model() {
    let (paid_server, paid_reg) = paid_cloud();
    let down =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(503, "{\"error\":\"down\"}"));
    let up = SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let down_reg = registration(
        "ollama",
        down.base_url.clone(),
        TargetLocality::Local,
        false,
        vec![model("llama-free", "ollama", CostClass::Free)],
    );
    let up_reg = registration(
        "lmstudio",
        format!("{}/v1", up.base_url),
        TargetLocality::Local,
        false,
        vec![model("lm-free", "lmstudio", CostClass::Free)],
    );
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = Dispatcher::new(
        &transport,
        &res,
        anansi_providers::catalog::default_adapters(),
    )
    .with_retry(anansi_providers::config::RetryPolicy {
        max_retries: 0,
        base_backoff_ms: 1,
        max_retry_after_ms: 10,
    });
    let mut ctx = test_ctx(ledger(), RoutingMode::Auto, ZeroCostLock::off());
    ctx.routing.preferred_providers = vec!["ollama".into(), "lmstudio".into()];
    let outcome = dispatcher
        .execute(
            &[paid_reg, down_reg, up_reg],
            &test_request("gpt-paid"),
            ctx,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.disclosure.provider, "lmstudio");
    assert_eq!(outcome.disclosure.model, "lm-free");
    assert_eq!(paid_server.hit_count(), 0);
    // The failed candidate got its own model on the wire...
    let down_recorded = down.recorded();
    assert_eq!(down_recorded.len(), 1);
    assert!(
        down_recorded[0].body.contains("\"model\":\"llama-free\""),
        "first candidate's wire model: {}",
        down_recorded[0].body
    );
    // ...and so did the fallback candidate that actually served.
    let up_recorded = up.recorded();
    assert_eq!(up_recorded.len(), 1);
    assert!(
        up_recorded[0].body.contains("\"model\":\"lm-free\""),
        "fallback candidate's wire model: {}",
        up_recorded[0].body
    );
}

#[test]
fn streaming_transmits_the_routed_candidates_model() {
    let (paid_server, paid_reg) = paid_cloud();
    let ndjson = "{\"model\":\"llama-free\",\"message\":{\"role\":\"assistant\",\"content\":\"str\"},\"done\":false}\n{\"model\":\"llama-free\",\"message\":{\"role\":\"assistant\",\"content\":\"eamed\"},\"done\":true,\"prompt_eval_count\":3,\"eval_count\":2}\n";
    let free_server = SyntheticServer::start(move |_req, _hit| ServerResponse::text(200, ndjson));
    let free_reg = registration(
        "ollama",
        free_server.base_url.clone(),
        TargetLocality::Local,
        false,
        vec![model("llama-free", "ollama", CostClass::Free)],
    );
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let outcome = dispatcher
        .execute_stream(
            &[paid_reg, free_reg],
            &test_request("gpt-paid"),
            test_ctx(ledger(), RoutingMode::Auto, ZeroCostLock::on()),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.disclosure.model, "llama-free");
    let mut text = String::new();
    while let Ok(event) = outcome.events.recv_timeout(Duration::from_secs(5)) {
        match event.unwrap() {
            StreamEvent::Token(t) => text.push_str(&t),
            StreamEvent::Completed { .. } => break,
            StreamEvent::Usage(_) => {}
        }
    }
    assert_eq!(text, "streamed");
    assert_eq!(paid_server.hit_count(), 0);
    let recorded = free_server.recorded();
    assert_eq!(recorded.len(), 1);
    assert!(
        recorded[0].body.contains("\"model\":\"llama-free\""),
        "stream wire model: {}",
        recorded[0].body
    );
    assert!(!recorded[0].body.contains("gpt-paid"));
}

#[test]
fn choose_model_still_dispatches_exactly_the_chosen_model() {
    // Control: where the caller legitimately chooses, wire,
    // disclosure and gates all agree on the chosen model.
    let server =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let reg = registration(
        "openai",
        format!("{}/v1", server.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-paid", "openai", CostClass::Paid)],
    );
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ctx = test_ctx(ledger(), RoutingMode::ChooseModel, ZeroCostLock::off());
    ctx.routing.chosen = Some(("openai".into(), "gpt-paid".into()));
    ctx.permit = Some(test_permit("openai", "gpt-paid"));
    let outcome = dispatcher
        .execute(
            &[reg],
            &test_request("gpt-paid"),
            ctx,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.disclosure.model, "gpt-paid");
    let recorded = server.recorded();
    assert!(recorded[0].body.contains("\"model\":\"gpt-paid\""));
    let expected_auth = format!("Bearer {FAKE_KEY}");
    assert_eq!(
        recorded[0].header("authorization"),
        Some(expected_auth.as_str())
    );
}

#[test]
fn permit_mismatch_refuses_that_candidate_and_the_chain_uses_the_authorized_one() {
    // The permit binds to the candidate: it does not authorize the
    // anthropic candidate (refused before network), and the chain
    // proceeds to the candidate the permit actually covers — which
    // then serves with its own model on the wire.
    let server_a =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let server_b =
        SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
    let reg_a = registration(
        "openai",
        format!("{}/v1", server_a.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-a", "openai", CostClass::Paid)],
    );
    let reg_b = registration(
        "anthropic",
        format!("{}/v1", server_b.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("claude-b", "anthropic", CostClass::Paid)],
    );
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let mut ctx = test_ctx(ledger(), RoutingMode::Auto, ZeroCostLock::off());
    ctx.routing.preferred_providers = vec!["anthropic".into()];
    ctx.permit = Some(test_permit("openai", "gpt-a"));
    let outcome = dispatcher
        .execute(
            &[reg_a, reg_b],
            &test_request("gpt-a"),
            ctx,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(
        server_b.hit_count(),
        0,
        "unauthorized candidate: no traffic"
    );
    assert_eq!(outcome.disclosure.provider, "openai");
    assert_eq!(outcome.disclosure.model, "gpt-a");
    assert!(
        outcome
            .disclosure
            .attempts
            .iter()
            .any(|a| a.provider == "anthropic" && a.outcome.contains("PermissionRefused")),
        "the refusal is on the record: {:?}",
        outcome.disclosure.attempts
    );
    // And when NO candidate matches the permit, the dispatch
    // refuses outright with zero traffic anywhere.
    let mut ctx = test_ctx(ledger(), RoutingMode::Auto, ZeroCostLock::off());
    ctx.permit = Some(test_permit("openai", "gpt-nonexistent"));
    let err = dispatcher
        .execute(
            &[registration(
                "anthropic",
                format!("{}/v1", server_b.base_url),
                TargetLocality::Cloud,
                true,
                vec![model("claude-b", "anthropic", CostClass::Paid)],
            )],
            &test_request("claude-b"),
            ctx,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::PermissionRefused, "{err:?}");
    assert_eq!(server_b.hit_count(), 0);
}
