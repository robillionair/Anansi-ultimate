//! The dispatcher: the single path from a routed request to a
//! provider. For every candidate, in order, it runs privacy → cost →
//! budget gates, resolves credentials, performs bounded retries, and
//! falls back only on availability failures. Gate refusals happen
//! before any network request; the synthetic-server tests assert that
//! with hit counters.
//!
//! Retry rules (§16): a retry of a billable dispatch happens only
//! under the same paid-inference permit that authorized the first
//! attempt (permits are consumed once per candidate, not per attempt),
//! and under Zero-Cost Lock no paid candidate is ever in the chain, so
//! no billable retry can occur. Fallback (§17) never crosses privacy or
//! cost boundaries: the chain itself was filtered by the router, and
//! every gate is re-run per candidate.
//!
//! Retry *safety* (billable duplication): whether a failure may be
//! retried — or fallen back from — depends on whether the request can
//! have been executed, which `ProviderError::request_sent` records:
//!
//! - **Pre-send failures** (DNS, connect, TLS handshake;
//!   `request_sent == false`): the provider never received a complete
//!   request. Safe to retry and to fall back, for any cost class.
//! - **Explicit rejection** (HTTP 429): the provider states it did
//!   not execute the request. Safe to retry (bounded, honouring
//!   Retry-After) and to fall back, for any cost class.
//! - **Post-send ambiguous failures** (5xx, timeout awaiting the
//!   response, connection lost after the request was written,
//!   malformed response): the provider may have executed — and
//!   billed — the request. For Free models, retry/fallback proceed as
//!   before. For Paid or Unknown-cost models they are **terminal**:
//!   no automatic retry, no fallback to another candidate (which would
//!   just move the duplicate bill), and the error is surfaced so the
//!   caller can decide. Unknown cost is treated as paid, per M5-D5.
//!
//! See `may_retry` / `may_fallback` and the retry-safety integration
//! tests (`tests/retry_safety.rs`), which assert exact synthetic
//! server hit counts for each rule.

use crate::config::{RetryPolicy, TimeoutConfig};
use crate::error::{ProviderError, ProviderErrorCode};
use crate::gates::{
    Budget, PaidInferencePermit, PrivacyGate, QuotaLedger, TargetLocality, ZeroCostLock,
    authorize_cost, estimate_cost_micro_usd,
};
use crate::http::{CancellationToken, Transport};
use crate::providers::ProviderAdapter;
use crate::redact::Redactor;
use crate::routing::{
    AttemptRecord, Candidate, DisclosureRecord, ProviderRegistration, Router, RoutingRequest,
};
use crate::secrets::{AuthMaterial, SecretResolver};
use crate::types::{CompletionRequest, CompletionResponse, StreamEvent};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

pub struct Dispatcher<'a> {
    transport: &'a dyn Transport,
    resolver: &'a dyn SecretResolver,
    adapters: HashMap<String, Box<dyn ProviderAdapter>>,
    timeouts: TimeoutConfig,
    retry: RetryPolicy,
    used_permits: Arc<Mutex<HashSet<String>>>,
}

#[derive(Debug)]
pub struct DispatchOutcome {
    pub response: CompletionResponse,
    pub disclosure: DisclosureRecord,
}

pub struct StreamOutcome {
    pub events: Receiver<Result<StreamEvent, ProviderError>>,
    pub disclosure: DisclosureRecord,
}

impl std::fmt::Debug for StreamOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamOutcome")
            .field("disclosure", &self.disclosure)
            .finish_non_exhaustive()
    }
}

/// Everything a dispatch needs beyond the request itself.
pub struct DispatchContext {
    pub routing: RoutingRequest,
    pub lock: ZeroCostLock,
    pub budget: Budget,
    /// The shared usage ledger. Shared (not borrowed) so concurrent
    /// dispatches reserve against the same accounting and stream
    /// reader threads can settle their reservation when the stream
    /// ends — see `QuotaLedger::reserve`.
    pub ledger: std::sync::Arc<std::sync::Mutex<QuotaLedger>>,
    pub permit: Option<PaidInferencePermit>,
}

impl<'a> Dispatcher<'a> {
    pub fn new(
        transport: &'a dyn Transport,
        resolver: &'a dyn SecretResolver,
        adapters: Vec<Box<dyn ProviderAdapter>>,
    ) -> Self {
        Self {
            transport,
            resolver,
            adapters: adapters
                .into_iter()
                .map(|a| (a.provider_id().to_string(), a))
                .collect(),
            timeouts: TimeoutConfig::default(),
            retry: RetryPolicy::default(),
            used_permits: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Share the single-use permit set with other dispatchers.
    /// `ProviderRuntime` builds a dispatcher per call but holds the
    /// set across calls, so one permit can never authorize two
    /// dispatches through the runtime.
    pub fn with_used_permits(mut self, used: Arc<Mutex<HashSet<String>>>) -> Self {
        self.used_permits = used;
        self
    }

    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.timeouts = timeouts.clamped();
        self
    }
    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry.clamped();
        self
    }

    fn adapter_ref(&self, provider: &str) -> Option<&dyn ProviderAdapter> {
        self.adapters.get(provider).map(|b| b.as_ref())
    }

    /// Run every pre-network gate for one candidate. Returns the
    /// resolved endpoint + auth material + a redactor primed with the
    /// secret (so any provider echo of it is scrubbed downstream).
    fn prepare(
        &self,
        reg: &ProviderRegistration,
        candidate: &Candidate,
        request: &CompletionRequest,
        ctx: &DispatchContext,
        permit_consumed: &mut bool,
    ) -> Result<PreparedDispatch, ProviderError> {
        // Privacy gate first (§19 order).
        PrivacyGate::check(request, candidate.locality)?;
        // Cost gate: lock, then permit for billable models.
        let estimated_total = request.estimated_input_tokens() + request.estimated_output_tokens();
        authorize_cost(
            &ctx.lock,
            candidate.model.cost,
            &candidate.provider,
            &candidate.model.id,
            estimated_total,
            ctx.permit.as_ref(),
        )?;
        // A permit authorizes one candidate dispatch chain; consume it
        // exactly once here (retries reuse this authorization).
        if candidate.model.cost != crate::types::CostClass::Free
            && !*permit_consumed
            && let Some(permit) = &ctx.permit
        {
            let mut used = self.used_permits.lock().unwrap();
            if !used.insert(permit.id.clone()) {
                return Err(ProviderError::new(
                    ProviderErrorCode::PermissionRefused,
                    format!("paid-inference permit '{}' was already consumed", permit.id),
                ));
            }
            *permit_consumed = true;
        }
        // Budget gate.
        let est_cost = estimate_cost_micro_usd(
            &candidate.model,
            request.estimated_input_tokens(),
            request.estimated_output_tokens(),
        );
        let reservation =
            lock_ledger(&ctx.ledger).reserve(&ctx.budget, estimated_total, est_cost)?;
        // Endpoint validation (single point, config module).
        let endpoint = reg.config.validate()?;
        // Credentials: resolve reference → material; prime redactor.
        let mut redactor = Redactor::new();
        let auth = match &reg.config.credential {
            Some(cred) => {
                let material = self.resolver.resolve(cred)?;
                match &material {
                    AuthMaterial::ApiKey(s) | AuthMaterial::BearerToken(s) => {
                        redactor.register(s.expose())
                    }
                    AuthMaterial::None => {}
                }
                material
            }
            None => AuthMaterial::None,
        };
        Ok(PreparedDispatch {
            endpoint,
            auth,
            redactor,
            estimated_cost_micro_usd: est_cost,
            reservation,
        })
    }

    pub fn execute(
        &self,
        registrations: &[ProviderRegistration],
        request: &CompletionRequest,
        ctx: DispatchContext,
        cancel: &CancellationToken,
    ) -> Result<DispatchOutcome, ProviderError> {
        let plan = Router::plan(registrations, &ctx.routing)?;
        let mut attempts: Vec<AttemptRecord> = Vec::new();
        let mut first_gate_error: Option<ProviderError> = None;
        let mut last_error: Option<ProviderError> = None;
        let mut permit_consumed = false;

        for candidate in &plan {
            let reg = &registrations[candidate.config_index];
            let prepared = match self.prepare(reg, candidate, request, &ctx, &mut permit_consumed) {
                Ok(p) => p,
                Err(e) => {
                    attempts.push(AttemptRecord {
                        provider: candidate.provider.clone(),
                        model: candidate.model.id.clone(),
                        outcome: format!("refused before network: {}", e.code_name()),
                    });
                    if e.refused_before_network() && first_gate_error.is_none() {
                        first_gate_error = Some(e);
                    } else if !e.refused_before_network() && last_error.is_none() {
                        last_error = Some(e);
                    }
                    continue;
                }
            };
            let Some(adapter) = self.adapter_ref(&candidate.provider) else {
                lock_ledger(&ctx.ledger).release(prepared.reservation);
                attempts.push(AttemptRecord {
                    provider: candidate.provider.clone(),
                    model: candidate.model.id.clone(),
                    outcome: "no adapter registered for provider".to_string(),
                });
                continue;
            };
            // Wire binding: the request that reaches the provider
            // names the SELECTED CANDIDATE's model — the same model
            // the gates, permit, budget estimate and disclosure
            // describe. The caller's requested model selects and
            // constrains routing (notably in Choose Model mode); it
            // must never ride onto the wire past a different routed
            // candidate, or e.g. a Zero-Cost-Lock dispatch routed
            // to a free model would transmit the requested paid
            // model.
            let mut bound_request = request.clone();
            bound_request.model = candidate.model.id.clone();
            match self.execute_candidate(
                adapter,
                &prepared,
                &bound_request,
                cancel,
                &mut attempts,
                candidate,
            ) {
                Ok(response) => {
                    lock_ledger(&ctx.ledger).settle(
                        prepared.reservation,
                        &candidate.provider,
                        response.usage,
                        prepared.estimated_cost_micro_usd,
                    );
                    let disclosure = self.disclosure(
                        candidate,
                        &prepared,
                        request,
                        &ctx,
                        attempts,
                        response.usage,
                    );
                    return Ok(DispatchOutcome {
                        response,
                        disclosure,
                    });
                }
                Err(e) => {
                    lock_ledger(&ctx.ledger).release(prepared.reservation);
                    // Availability-class failures may fall back to the
                    // next candidate — unless the failure is ambiguous
                    // for a billable model, where falling back could
                    // duplicate paid inference at another provider.
                    if may_fallback(&e, candidate.model.cost) {
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(first_gate_error.or(last_error).unwrap_or_else(|| {
            ProviderError::new(
                ProviderErrorCode::ProviderUnavailable,
                "no candidate could be dispatched",
            )
        }))
    }

    fn execute_candidate(
        &self,
        adapter: &dyn ProviderAdapter,
        prepared: &PreparedDispatch,
        request: &CompletionRequest,
        cancel: &CancellationToken,
        attempts: &mut Vec<AttemptRecord>,
        candidate: &Candidate,
    ) -> Result<CompletionResponse, ProviderError> {
        let http_req = adapter.build_request(&prepared.endpoint, &prepared.auth, request, false)?;
        let mut attempt = 0u8;
        loop {
            cancel_check(cancel)?;
            let mut response = match self.transport.execute(&http_req, &self.timeouts, cancel) {
                Ok(r) => r,
                Err(e) => {
                    attempts.push(AttemptRecord {
                        provider: candidate.provider.clone(),
                        model: candidate.model.id.clone(),
                        outcome: format!("transport error: {}", e.code_name()),
                    });
                    if may_retry(&e, candidate.model.cost, attempt, self.retry.max_retries) {
                        sleep_before_retry(&self.retry, attempt, e.retry_after);
                        attempt += 1;
                        continue;
                    }
                    return Err(e);
                }
            };
            let status = response.status;
            let headers = response.headers.clone();
            let body = response.read_body_capped(MAX_RESPONSE_BYTES)?;
            match adapter.parse_response(&prepared.redactor, status, &headers, &body) {
                Ok(resp) => {
                    attempts.push(AttemptRecord {
                        provider: candidate.provider.clone(),
                        model: candidate.model.id.clone(),
                        outcome: format!("HTTP {status} success"),
                    });
                    return Ok(resp);
                }
                Err(e) => {
                    attempts.push(AttemptRecord {
                        provider: candidate.provider.clone(),
                        model: candidate.model.id.clone(),
                        outcome: format!("HTTP {status}: {}", e.code_name()),
                    });
                    if may_retry(&e, candidate.model.cost, attempt, self.retry.max_retries) {
                        sleep_before_retry(&self.retry, attempt, e.retry_after);
                        attempt += 1;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Streaming dispatch. Gates, planning and candidate preparation
    /// are identical to `execute`. Fallback is possible only until the
    /// stream is established (connect/status stage); once events flow,
    /// an error is delivered on the channel — a stream is never
    /// silently restarted against another provider.
    pub fn execute_stream(
        &self,
        registrations: &[ProviderRegistration],
        request: &CompletionRequest,
        ctx: DispatchContext,
        cancel: &CancellationToken,
    ) -> Result<StreamOutcome, ProviderError> {
        let plan = Router::plan(registrations, &ctx.routing)?;
        let mut attempts: Vec<AttemptRecord> = Vec::new();
        let mut first_gate_error: Option<ProviderError> = None;
        let mut last_error: Option<ProviderError> = None;
        let mut permit_consumed = false;

        for candidate in &plan {
            let reg = &registrations[candidate.config_index];
            let prepared = match self.prepare(reg, candidate, request, &ctx, &mut permit_consumed) {
                Ok(p) => p,
                Err(e) => {
                    attempts.push(AttemptRecord {
                        provider: candidate.provider.clone(),
                        model: candidate.model.id.clone(),
                        outcome: format!("refused before network: {}", e.code_name()),
                    });
                    if e.refused_before_network() && first_gate_error.is_none() {
                        first_gate_error = Some(e);
                    }
                    continue;
                }
            };
            let Some(adapter) = self.adapter_ref(&candidate.provider) else {
                continue;
            };
            // Wire binding (see execute): the streamed request also
            // names the selected candidate's model, never the
            // caller's requested model.
            let mut bound_request = request.clone();
            bound_request.model = candidate.model.id.clone();
            let request = &bound_request;
            let http_req =
                match adapter.build_request(&prepared.endpoint, &prepared.auth, request, true) {
                    Ok(r) => r,
                    Err(e) => {
                        lock_ledger(&ctx.ledger).release(prepared.reservation);
                        last_error = Some(e);
                        continue;
                    }
                };
            // One establishment attempt per candidate for streaming;
            // retrying a stream pre-first-byte is the fallback loop
            // itself (next candidate), which keeps semantics simple and
            // observable.
            let response = match self.transport.execute(&http_req, &self.timeouts, cancel) {
                Ok(r) => r,
                Err(e) => {
                    lock_ledger(&ctx.ledger).release(prepared.reservation);
                    attempts.push(AttemptRecord {
                        provider: candidate.provider.clone(),
                        model: candidate.model.id.clone(),
                        outcome: format!("stream connect failed: {}", e.code_name()),
                    });
                    if may_fallback(&e, candidate.model.cost) {
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            };
            let status = response.status;
            if (300..400).contains(&status) {
                lock_ledger(&ctx.ledger).release(prepared.reservation);
                let err =
                    crate::providers::redirect_error(status, &response.headers, &prepared.redactor);
                attempts.push(AttemptRecord {
                    provider: candidate.provider.clone(),
                    model: candidate.model.id.clone(),
                    outcome: "redirect refused".to_string(),
                });
                last_error = Some(err);
                continue;
            }
            if !(200..300).contains(&status) {
                let headers = response.headers.clone();
                let mut response = response;
                let body = response.read_body_capped(MAX_RESPONSE_BYTES)?;
                let err = match adapter.parse_response(&prepared.redactor, status, &headers, &body)
                {
                    Err(e) => e,
                    Ok(_) => ProviderError::new(
                        ProviderErrorCode::ProviderUnavailable,
                        format!("unexpected HTTP {status} for streaming request"),
                    ),
                };
                lock_ledger(&ctx.ledger).release(prepared.reservation);
                attempts.push(AttemptRecord {
                    provider: candidate.provider.clone(),
                    model: candidate.model.id.clone(),
                    outcome: format!("HTTP {status}: {}", err.code_name()),
                });
                if may_fallback(&err, candidate.model.cost) {
                    last_error = Some(err);
                    continue;
                }
                return Err(err);
            }
            // Stream established.
            attempts.push(AttemptRecord {
                provider: candidate.provider.clone(),
                model: candidate.model.id.clone(),
                outcome: format!("HTTP {status} streaming"),
            });
            lock_ledger(&ctx.ledger).begin_stream(
                prepared.reservation,
                &candidate.provider,
                prepared.estimated_cost_micro_usd,
            );
            let disclosure = self.disclosure(candidate, &prepared, request, &ctx, attempts, None);
            let (tx, rx) = channel::<Result<StreamEvent, ProviderError>>();
            let mut decoder = adapter.new_stream_decoder();
            let mut body = response.into_body();
            let cancel = cancel.clone();
            // Streaming accounting: the token reservation stays live
            // until this thread ends — on completion, error AND
            // cancellation alike — where it settles with whatever
            // usage the provider actually reported.
            let stream_ledger = ctx.ledger.clone();
            let stream_reservation = prepared.reservation;
            let stream_provider = candidate.provider.clone();
            std::thread::spawn(move || {
                let mut last_usage: Option<crate::types::Usage> = None;
                let track = |events: &Vec<StreamEvent>, last: &mut Option<crate::types::Usage>| {
                    for ev in events {
                        if let StreamEvent::Usage(u) = ev {
                            *last = Some(*u);
                        }
                    }
                };
                let settle = |last_usage: Option<crate::types::Usage>| {
                    lock_ledger(&stream_ledger).settle_stream(
                        stream_reservation,
                        &stream_provider,
                        last_usage,
                    );
                };
                let mut buf = [0u8; 8192];
                let send_err = |e: ProviderError| {
                    let _ = tx.send(Err(e));
                };
                // Every exit from this loop settles the stream's
                // token reservation exactly once, below.
                'stream: loop {
                    if cancel.is_cancelled() {
                        send_err(ProviderError::new(
                            ProviderErrorCode::Cancelled,
                            "stream cancelled by caller",
                        ));
                        break 'stream;
                    }
                    match body.read(&mut buf) {
                        Ok(0) => {
                            // A cancelled stream also surfaces as EOF
                            // (the token shuts the socket down); report
                            // cancellation, not a clean finish.
                            if cancel.is_cancelled() {
                                send_err(ProviderError::new(
                                    ProviderErrorCode::Cancelled,
                                    "stream cancelled by caller",
                                ));
                                break 'stream;
                            }
                            match decoder.finish() {
                                Ok(events) => {
                                    track(&events, &mut last_usage);
                                    for ev in events {
                                        if tx.send(Ok(ev)).is_err() {
                                            break 'stream;
                                        }
                                    }
                                }
                                Err(e) => send_err(e),
                            }
                            break 'stream;
                        }
                        Ok(n) => match decoder.feed(&buf[..n]) {
                            Ok(events) => {
                                track(&events, &mut last_usage);
                                for ev in events {
                                    if tx.send(Ok(ev)).is_err() {
                                        break 'stream;
                                    }
                                }
                            }
                            Err(e) => {
                                send_err(e);
                                break 'stream;
                            }
                        },
                        Err(io_err) => {
                            if cancel.is_cancelled() {
                                send_err(ProviderError::new(
                                    ProviderErrorCode::Cancelled,
                                    "stream cancelled by caller",
                                ));
                            } else {
                                send_err(crate::http::map_io_error(io_err));
                            }
                            break 'stream;
                        }
                    }
                }
                settle(last_usage);
            });
            return Ok(StreamOutcome {
                events: rx,
                disclosure,
            });
        }
        Err(first_gate_error.or(last_error).unwrap_or_else(|| {
            ProviderError::new(
                ProviderErrorCode::ProviderUnavailable,
                "no candidate could establish a stream",
            )
        }))
    }

    fn disclosure(
        &self,
        candidate: &Candidate,
        prepared: &PreparedDispatch,
        request: &CompletionRequest,
        ctx: &DispatchContext,
        attempts: Vec<AttemptRecord>,
        usage: Option<crate::types::Usage>,
    ) -> DisclosureRecord {
        DisclosureRecord {
            provider: candidate.provider.clone(),
            model: candidate.model.id.clone(),
            endpoint_host: prepared.endpoint.host.clone(),
            local: candidate.locality == TargetLocality::Local,
            routing_mode: ctx.routing.mode,
            routing_reason: candidate.reason.clone(),
            message_count: request.messages.len(),
            segment_count: request.messages.iter().map(|m| m.segments.len()).sum(),
            redaction_count: PrivacyGate::redaction_count(request),
            estimated_input_tokens: request.estimated_input_tokens(),
            cost_class: candidate.model.cost,
            usage,
            attempts,
        }
    }
}

pub const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

struct PreparedDispatch {
    endpoint: crate::config::ValidatedEndpoint,
    auth: AuthMaterial,
    redactor: Redactor,
    estimated_cost_micro_usd: Option<u64>,
    /// The budget reservation made by `prepare`. Exactly one of
    /// settle / release / the stream pair closes it (see
    /// `QuotaLedger`); the execute loops own that lifecycle.
    reservation: crate::gates::Reservation,
}

fn lock_ledger(
    ledger: &std::sync::Arc<std::sync::Mutex<QuotaLedger>>,
) -> std::sync::MutexGuard<'_, QuotaLedger> {
    ledger
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn is_fallback_eligible(e: &ProviderError) -> bool {
    matches!(
        e.code,
        ProviderErrorCode::ProviderUnavailable
            | ProviderErrorCode::RateLimited
            | ProviderErrorCode::ConnectionLost
            | ProviderErrorCode::Timeout
    )
}

/// A failure after which the provider may already have executed the
/// request: it was fully sent, and the failure class (5xx, timeout,
/// post-send connection loss, malformed response) does not prove
/// non-execution. HTTP 429 is deliberately absent — it is the
/// provider's explicit statement that the request was not executed.
fn is_ambiguous_after_send(e: &ProviderError) -> bool {
    e.request_sent
        && matches!(
            e.code,
            ProviderErrorCode::ProviderUnavailable
                | ProviderErrorCode::ConnectionLost
                | ProviderErrorCode::Timeout
                | ProviderErrorCode::MalformedResponse
        )
}

/// May this failure be retried against the same candidate? See the
/// module-level retry-safety policy.
fn may_retry(
    e: &ProviderError,
    cost: crate::types::CostClass,
    attempt: u8,
    max_retries: u8,
) -> bool {
    use crate::types::CostClass;
    if attempt >= max_retries {
        return false;
    }
    if !e.request_sent {
        // Pre-send: only transport-class failures qualify, and a TLS
        // failure is never retried blindly (it will fail identically
        // and may indicate an attack or misconfiguration).
        return matches!(
            e.code,
            ProviderErrorCode::ConnectionLost | ProviderErrorCode::Timeout
        );
    }
    if e.code == ProviderErrorCode::RateLimited {
        return true; // explicitly not executed
    }
    // Post-send: only free dispatches may auto-retry ambiguous
    // failures; billable ones would risk duplicate paid inference.
    cost == CostClass::Free && e.retryable()
}

/// May dispatch fall back to the next candidate after this failure?
fn may_fallback(e: &ProviderError, cost: crate::types::CostClass) -> bool {
    if !is_fallback_eligible(e) {
        return false;
    }
    // Falling back after an ambiguous billable failure just moves
    // the possible duplicate charge to another provider.
    !(cost != crate::types::CostClass::Free && is_ambiguous_after_send(e))
}

fn cancel_check(cancel: &CancellationToken) -> Result<(), ProviderError> {
    if cancel.is_cancelled() {
        Err(ProviderError::new(
            ProviderErrorCode::Cancelled,
            "request cancelled by caller",
        ))
    } else {
        Ok(())
    }
}

fn sleep_before_retry(policy: &RetryPolicy, attempt: u8, retry_after: Option<std::time::Duration>) {
    let delay = policy.delay(attempt, retry_after);
    if !delay.is_zero() {
        std::thread::sleep(delay);
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    use crate::types::CostClass;

    fn err(code: ProviderErrorCode) -> ProviderError {
        ProviderError::new(code, "policy test")
    }

    #[test]
    fn pre_send_failures_retry_and_fall_back_for_paid() {
        let e = err(ProviderErrorCode::ConnectionLost).pre_send();
        assert!(may_retry(&e, CostClass::Paid, 0, 2));
        assert!(may_fallback(&e, CostClass::Paid));
        let t = err(ProviderErrorCode::Timeout).pre_send();
        assert!(may_retry(&t, CostClass::Unknown, 0, 2));
    }

    #[test]
    fn tls_failures_are_never_retried() {
        let e = err(ProviderErrorCode::TlsError).pre_send();
        assert!(!may_retry(&e, CostClass::Free, 0, 2));
        assert!(!may_fallback(&e, CostClass::Free));
    }

    #[test]
    fn explicit_429_retries_for_paid() {
        let e = err(ProviderErrorCode::RateLimited);
        assert!(e.request_sent, "status-derived errors are post-send");
        assert!(may_retry(&e, CostClass::Paid, 0, 2));
        assert!(may_fallback(&e, CostClass::Paid));
        assert!(!may_retry(&e, CostClass::Paid, 2, 2), "bound still applies");
    }

    #[test]
    fn ambiguous_post_send_failures_are_terminal_for_billable() {
        for code in [
            ProviderErrorCode::ProviderUnavailable,
            ProviderErrorCode::ConnectionLost,
            ProviderErrorCode::Timeout,
            ProviderErrorCode::MalformedResponse,
        ] {
            let e = err(code);
            for cost in [CostClass::Paid, CostClass::Unknown] {
                assert!(
                    !may_retry(&e, cost, 0, 2),
                    "{code:?} must not retry for {cost:?}"
                );
                assert!(
                    !may_fallback(&e, cost),
                    "{code:?} must not fall back for {cost:?}"
                );
            }
            // Free models keep the availability behaviour.
            assert!(
                may_fallback(&e, CostClass::Free) || code == ProviderErrorCode::MalformedResponse
            );
        }
    }

    #[test]
    fn free_post_send_availability_failures_still_retry() {
        let e = err(ProviderErrorCode::ProviderUnavailable);
        assert!(may_retry(&e, CostClass::Free, 0, 2));
        assert!(may_fallback(&e, CostClass::Free));
        let lost = err(ProviderErrorCode::ConnectionLost);
        assert!(may_retry(&lost, CostClass::Free, 0, 2));
    }
}
