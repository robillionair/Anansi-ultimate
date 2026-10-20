//! The complete M5 workflow, end to end, against a synthetic HTTPS
//! provider with test-only certificates:
//!
//!   provider configuration (JSON, references only)
//!     → credential stored in the (mock) native keyring
//!     → authorized dispatch under a paid-inference permit
//!     → response + disclosure record
//!     → streamed dispatch → mid-stream cancellation
//!     → permit single-use and Zero-Cost Lock enforced across calls
//!
//! No real credential, no real provider, no paid inference: the key
//! is a fake, the store is the keyring crate's mock builder, and the
//! server is in-process.

mod common;

use anansi_providers::error::ProviderErrorCode;
use anansi_providers::gates::ZeroCostLock;
use anansi_providers::http::{CancellationToken, RustlsTransport};
use anansi_providers::keyring_resolver::KeyringSecretResolver;
use anansi_providers::routing::{RoutingMode, RoutingRequest};
use anansi_providers::runtime::{ProviderRuntime, RuntimeConfig};
use anansi_providers::types::{ChatMessage, CompletionRequest, Role, StreamEvent};
use common::*;
use std::sync::Once;
use std::time::Duration;

static MOCK: Once = Once::new();

fn workflow_handler(req: &RecordedRequest, _hit: usize) -> ServerResponse {
    let body = req.body.as_str();
    if body.contains("\"stream\":true") {
        if body.contains("go slow") {
            let mut sse = String::new();
            for _ in 0..60 {
                sse.push_str("data: {\"choices\":[{\"delta\":{\"content\":\"tok \"}}]}\n\n");
            }
            return ServerResponse::text(200, &sse).chunked(24, Duration::from_millis(120));
        }
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"hello \"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"keyring stream\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
        return ServerResponse::text(200, sse).chunked(4, Duration::ZERO);
    }
    ServerResponse::json(200, openai_success_body())
}

fn request(text: &str) -> CompletionRequest {
    CompletionRequest::new("gpt-x", vec![ChatMessage::new(Role::User, text)])
}

fn routing(lock: ZeroCostLock) -> RoutingRequest {
    RoutingRequest {
        mode: RoutingMode::Auto,
        lock,
        ..Default::default()
    }
}

#[test]
fn complete_configuration_to_cancellation_workflow() {
    // -- 1. Synthetic HTTPS provider + test CA --------------------
    let ca = TestCa::new();
    let (cert, key) = ca.valid_server();
    let server = TlsSyntheticServer::start(&cert, &key, workflow_handler);

    // -- 2. Credential into the native store (mock builder) -------
    MOCK.call_once(|| {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    });
    let resolver =
        KeyringSecretResolver::with_service(format!("anansi-workflow-test-{}", std::process::id()));
    resolver
        .store_api_key("openai-primary", FAKE_KEY.as_bytes())
        .unwrap();

    // -- 3. Provider configuration: JSON, references only ---------
    let config_json = format!(
        r#"{{
          "providers": [{{
            "config": {{
              "provider": "openai",
              "baseUrl": "{}/v1",
              "credential": {{ "provider": "openai", "reference": "openai-primary" }},
              "enabled": true
            }},
            "locality": "cloud",
            "models": [{{
              "id": "gpt-x",
              "provider": "openai",
              "capabilities": {{ "chat": true, "streaming": true, "vision": false,
                                 "coding": true, "deepReasoning": false, "embeddings": false }},
              "contextWindow": 8192,
              "cost": "paid",
              "speed": "balanced"
            }}]
          }}]
        }}"#,
        server.base_url
    );
    assert!(
        !config_json.contains(FAKE_KEY),
        "configuration never carries the key"
    );
    let config: RuntimeConfig = serde_json::from_str(&config_json).unwrap();

    let mut tls = anansi_providers::tls::TlsConfig::new();
    tls.add_root_pem(ca.cert_pem.as_bytes()).unwrap();
    let mut runtime = ProviderRuntime::new(
        config,
        RustlsTransport::with_tls_config(tls),
        Box::new(resolver),
    )
    .unwrap();

    // -- 4. Authorized unary dispatch ------------------------------
    let mut permit = test_permit("openai", "gpt-x");
    permit.id = "permit-workflow-1".into();
    let outcome = runtime
        .dispatch(
            &request("Say hello"),
            routing(ZeroCostLock::off()),
            Some(permit),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(outcome.response.text, "hello from synthetic");
    assert_eq!(outcome.response.usage.unwrap().total(), 18);
    // Disclosure: what was sent, where, under whose authority.
    let d = &outcome.disclosure;
    assert_eq!(d.provider, "openai");
    assert_eq!(d.model, "gpt-x");
    assert!(
        d.endpoint_host.starts_with("127.0.0.1"),
        "endpoint host recorded: {}",
        d.endpoint_host
    );
    assert!(!d.local);
    assert_eq!(d.cost_class, anansi_providers::types::CostClass::Paid);
    assert_eq!(d.usage.unwrap().total(), 18);
    assert_eq!(d.attempts.len(), 1);
    // The key the server saw is exactly the one the keyring held.
    assert_eq!(
        server.recorded()[0].header("authorization"),
        Some(format!("Bearer {FAKE_KEY}").as_str())
    );
    assert_eq!(server.hit_count(), 1);

    // -- 5. The same permit never authorizes twice ------------------
    let mut permit = test_permit("openai", "gpt-x");
    permit.id = "permit-workflow-1".into();
    let err = runtime
        .dispatch(
            &request("Say hello"),
            routing(ZeroCostLock::off()),
            Some(permit),
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::PermissionRefused);
    assert_eq!(server.hit_count(), 1, "a refused dispatch sends nothing");

    // -- 6. Zero-Cost Lock blocks the paid model outright -----------
    let err = runtime
        .dispatch(
            &request("Say hello"),
            routing(ZeroCostLock::on()),
            None,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert_eq!(err.code, ProviderErrorCode::ProviderUnavailable);
    assert_eq!(server.hit_count(), 1);

    // -- 7. Streamed dispatch ---------------------------------------
    let mut permit = test_permit("openai", "gpt-x");
    permit.id = "permit-workflow-2".into();
    let stream = runtime
        .dispatch_stream(
            &request("Say hello"),
            routing(ZeroCostLock::off()),
            Some(permit),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(stream.disclosure.provider, "openai");
    assert_eq!(stream.disclosure.model, "gpt-x");
    let mut text = String::new();
    let mut saw_usage = false;
    let mut saw_completed = false;
    while let Ok(event) = stream.events.recv_timeout(Duration::from_secs(5)) {
        match event.unwrap() {
            StreamEvent::Token(t) => text.push_str(&t),
            StreamEvent::Usage(_) => saw_usage = true,
            StreamEvent::Completed { .. } => {
                saw_completed = true;
                break;
            }
        }
    }
    assert_eq!(text, "hello keyring stream");
    assert!(saw_completed, "stream must terminate with Completed");
    let _ = saw_usage; // provider-dependent; the SSE here sends none
    assert_eq!(server.hit_count(), 2);

    // -- 8. Cancellation mid-stream ---------------------------------
    let mut permit = test_permit("openai", "gpt-x");
    permit.id = "permit-workflow-3".into();
    let cancel = CancellationToken::new();
    let stream = runtime
        .dispatch_stream(
            &request("go slow"),
            routing(ZeroCostLock::off()),
            Some(permit),
            &cancel,
        )
        .unwrap();
    let first = stream.events.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(first, Ok(StreamEvent::Token(_))), "{first:?}");
    cancel.cancel();
    let mut cancelled = false;
    while let Ok(event) = stream.events.recv_timeout(Duration::from_secs(4)) {
        if let Err(e) = event {
            assert_eq!(e.code, ProviderErrorCode::Cancelled);
            cancelled = true;
            break;
        }
    }
    assert!(cancelled, "the stream must end with a Cancelled event");
    // Disclosure for the cancelled run still records the attempt.
    assert_eq!(stream.disclosure.provider, "openai");
    assert_eq!(stream.disclosure.attempts.len(), 1);
    assert_eq!(server.hit_count(), 3);

    // -- 9. Ledger accumulated the completed work --------------------
    let totals = runtime.ledger().lock().unwrap().totals_for("openai");
    assert!(
        totals.requests >= 2,
        "ledger saw the completed dispatches: {totals:?}"
    );
    assert!(totals.input_tokens + totals.output_tokens >= 18);
}
