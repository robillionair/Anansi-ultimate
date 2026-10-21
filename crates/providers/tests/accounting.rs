//! Budget accounting under concurrency and across stream endings.
//! The reservation system (QuotaLedger::reserve/settle/release) is
//! what these tests prove at the dispatcher level: budget checks are
//! no longer check-then-act.

mod common;

use anansi_providers::dispatcher::Dispatcher;
use anansi_providers::error::ProviderErrorCode;
use anansi_providers::gates::{Budget, QuotaLedger, TargetLocality, ZeroCostLock};
use anansi_providers::http::{CancellationToken, StdHttpTransport};
use anansi_providers::routing::RoutingMode;
use anansi_providers::types::{CostClass, StreamEvent};
use common::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[test]
fn concurrent_dispatches_cannot_overrun_the_request_budget() {
    // A slow server keeps all four dispatches genuinely in flight at
    // once. Budget: 2 requests. Without reservations, all four would
    // pass the budget check before any of them recorded usage.
    let server = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(200, openai_success_body()).delayed(Duration::from_millis(400))
    });
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = Arc::new(dispatcher(&transport, &res));
    let ledger = Arc::new(Mutex::new(QuotaLedger::new()));
    let regs = Arc::new(vec![registration(
        "openai",
        format!("{}/v1", server.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", CostClass::Free)],
    )]);
    let budget = Budget {
        max_requests: Some(2),
        ..Default::default()
    };
    let mut ok = 0;
    let mut refused = 0;
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..4 {
            let dispatcher = dispatcher.clone();
            let ledger = ledger.clone();
            let regs = regs.clone();
            let budget = budget.clone();
            handles.push(scope.spawn(move || {
                let mut ctx = test_ctx(ledger, RoutingMode::Auto, ZeroCostLock::off());
                ctx.budget = budget;
                dispatcher.execute(
                    &regs,
                    &test_request("gpt-x"),
                    ctx,
                    &CancellationToken::new(),
                )
            }));
        }
        for h in handles {
            match h.join().unwrap() {
                Ok(_) => ok += 1,
                Err(e) => {
                    assert_eq!(e.code, ProviderErrorCode::QuotaExceeded, "{e:?}");
                    refused += 1;
                }
            }
        }
    });
    assert_eq!(ok, 2, "exactly the budgeted number may dispatch");
    assert_eq!(refused, 2);
    assert_eq!(
        server.hit_count(),
        2,
        "refused dispatches produce no traffic"
    );
}

#[test]
fn failed_dispatch_releases_its_reservation() {
    let down = SyntheticServer::start(|_req, _hit| {
        ServerResponse::json(503, r#"{"error":{"message":"down"}}"#)
    });
    let up = SyntheticServer::start(|_req, _hit| ServerResponse::json(200, openai_success_body()));
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
    let ledger = Arc::new(Mutex::new(QuotaLedger::new()));
    let budget = Budget {
        max_requests: Some(1),
        ..Default::default()
    };
    let mut ctx = test_ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off());
    ctx.budget = budget.clone();
    let regs_down = vec![registration(
        "openai",
        format!("{}/v1", down.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", CostClass::Free)],
    )];
    let err = dispatcher
        .execute(
            &regs_down,
            &test_request("gpt-x"),
            ctx,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ProviderUnavailable);
    // The failed dispatch held the only request slot while in
    // flight; if the hold leaked, this second dispatch would be
    // refused by the budget instead of succeeding.
    let mut ctx = test_ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off());
    ctx.budget = budget;
    let regs_up = vec![registration(
        "openai",
        format!("{}/v1", up.base_url),
        TargetLocality::Cloud,
        true,
        vec![model("gpt-x", "openai", CostClass::Free)],
    )];
    let outcome = dispatcher
        .execute(
            &regs_up,
            &test_request("gpt-x"),
            ctx,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(ledger.lock().unwrap().reserved(), (0, 0));
}

#[test]
fn streamed_usage_is_recorded_when_the_stream_ends() {
    // Ollama NDJSON carries usage in its final object; the stream
    // decoder surfaces it as a Usage event, and the reader thread
    // settles the reservation with it when the stream ends.
    let ndjson = "{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"str\"},\"done\":false}\n{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"eamed\"},\"done\":false}\n{\"model\":\"llama-x\",\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true,\"prompt_eval_count\":7,\"eval_count\":5}\n";
    let server = SyntheticServer::start(move |_req, _hit| ServerResponse::text(200, ndjson));
    let transport = StdHttpTransport;
    let res = resolver();
    let dispatcher = dispatcher(&transport, &res);
    let ledger = Arc::new(Mutex::new(QuotaLedger::new()));
    let regs = vec![registration(
        "ollama",
        server.base_url.clone(),
        TargetLocality::Local,
        false,
        vec![model("llama-x", "ollama", CostClass::Free)],
    )];
    let outcome = dispatcher
        .execute_stream(
            &regs,
            &test_request("llama-x"),
            test_ctx(ledger.clone(), RoutingMode::Auto, ZeroCostLock::off()),
            &CancellationToken::new(),
        )
        .unwrap();
    let mut text = String::new();
    while let Ok(event) = outcome.events.recv_timeout(Duration::from_secs(5)) {
        match event.unwrap() {
            StreamEvent::Token(t) => text.push_str(&t),
            StreamEvent::Completed { .. } => break,
            StreamEvent::Usage(_) => {}
        }
    }
    assert_eq!(text, "streamed");
    // Settlement happens in the reader thread just after the final
    // event; wait for it to land.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        {
            let guard = ledger.lock().unwrap();
            let totals = guard.totals_for("ollama");
            if totals.input_tokens == 7 && totals.output_tokens == 5 {
                assert_eq!(totals.requests, 1);
                assert_eq!(
                    guard.reserved(),
                    (0, 0),
                    "no hold may leak after the stream"
                );
                return;
            }
        }
        assert!(Instant::now() < deadline, "stream usage never settled");
        std::thread::sleep(Duration::from_millis(20));
    }
}
