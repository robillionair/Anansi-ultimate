//! Pre-dispatch gates. Every gate runs **before** any network request;
//! a refusal from any gate is observable in tests as a synthetic-server
//! hit count of zero. Order (MASTER_SPEC §19): privacy → cost policy →
//! budget/quota → paid-inference authorization.

use crate::error::{ProviderError, ProviderErrorCode};
use crate::types::{CompletionRequest, CostClass, PrivacyClass};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Privacy gate (§§17, 19, 74)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLocality {
    /// Loopback runtime on the user's own machine.
    Local,
    /// Anything that leaves the machine.
    Cloud,
}

pub struct PrivacyGate;

impl PrivacyGate {
    pub fn check(request: &CompletionRequest, target: TargetLocality) -> Result<(), ProviderError> {
        let mut has_sealed = false;
        let mut has_private = false;
        let mut has_restricted_unredacted = false;
        for message in &request.messages {
            for segment in &message.segments {
                match segment.privacy {
                    PrivacyClass::Sealed => has_sealed = true,
                    PrivacyClass::Private => has_private = true,
                    PrivacyClass::Restricted if !segment.redacted => {
                        has_restricted_unredacted = true
                    }
                    _ => {}
                }
            }
        }
        // Sealed is refused for every target, including loopback local:
        // the baseline's semantic pipeline rejects Sealed before any
        // inference (ADR-0030: fail closed, never compiled or disclosed).
        if has_sealed {
            return Err(ProviderError::new(
                ProviderErrorCode::PrivacyRefused,
                "request contains Sealed content; Sealed material is never transmitted to any provider",
            ));
        }
        if request.local_only && target == TargetLocality::Cloud {
            return Err(ProviderError::new(
                ProviderErrorCode::PrivacyRefused,
                "request is marked local-only; cloud dispatch refused (never silently transmitted, §17)",
            ));
        }
        if has_restricted_unredacted && target == TargetLocality::Cloud {
            return Err(ProviderError::new(
                ProviderErrorCode::PrivacyRefused,
                "request contains unredacted Restricted content; redact before cloud dispatch (§19)",
            ));
        }
        if has_private && target == TargetLocality::Cloud && !request.explicit_cloud_authorization {
            return Err(ProviderError::new(
                ProviderErrorCode::PrivacyRefused,
                "request contains Private content and no explicit cloud authorization was given (ADR-0002)",
            ));
        }
        Ok(())
    }

    /// Count segments that were redacted by the caller (for disclosure).
    pub fn redaction_count(request: &CompletionRequest) -> usize {
        request
            .messages
            .iter()
            .flat_map(|m| m.segments.iter())
            .filter(|s| s.redacted)
            .count()
    }
}

// ---------------------------------------------------------------------------
// Cost policy: Zero-Cost Lock + paid-inference permit (§§16, 67; ADR-0002)
// ---------------------------------------------------------------------------

/// Zero-Cost Lock (§16). When engaged it blocks paid models,
/// unknown-cost models, free-to-paid fallback (the router only plans
/// free candidates), automatic credit purchases (no purchase API exists
/// in this package), and unapproved billable retries (the retry policy
/// consults the permit, see `dispatcher`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZeroCostLock {
    pub engaged: bool,
}

impl ZeroCostLock {
    pub fn on() -> Self {
        Self { engaged: true }
    }
    pub fn off() -> Self {
        Self { engaged: false }
    }
    pub fn check(&self, model_cost: CostClass) -> Result<(), ProviderError> {
        if self.engaged && model_cost != CostClass::Free {
            return Err(ProviderError::new(
                ProviderErrorCode::CostRefused,
                match model_cost {
                    CostClass::Paid => {
                        "Zero-Cost Lock is engaged: paid model dispatch is blocked (§16)"
                    }
                    CostClass::Unknown => {
                        "Zero-Cost Lock is engaged: unknown-cost model dispatch is blocked (§16)"
                    }
                    CostClass::Free => unreachable!(),
                },
            ));
        }
        Ok(())
    }
}

/// Explicit owner authorization for one bounded paid dispatch
/// (AGENTS.md / ADR-0002: paid inference requires explicit
/// authorization). Permits are single-use: the dispatcher consumes one
/// per dispatched candidate attempt-chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaidInferencePermit {
    pub id: String,
    pub provider: String,
    pub model: String,
    /// Maximum estimated total tokens this permit authorizes.
    pub max_estimated_total_tokens: u64,
    /// Must be true to authorize a model whose cost is unknown (M5-D5).
    pub allow_unknown_cost: bool,
}

impl PaidInferencePermit {
    pub fn check(
        &self,
        provider: &str,
        model: &str,
        cost: CostClass,
        estimated_total_tokens: u64,
    ) -> Result<(), ProviderError> {
        if self.provider != provider || self.model != model {
            return Err(ProviderError::new(
                ProviderErrorCode::PermissionRefused,
                format!(
                    "paid-inference permit '{}' covers {}/{} but dispatch targets {}/{}",
                    self.id, self.provider, self.model, provider, model
                ),
            ));
        }
        if cost == CostClass::Unknown && !self.allow_unknown_cost {
            return Err(ProviderError::new(
                ProviderErrorCode::PermissionRefused,
                "model cost is unknown and the permit does not acknowledge unknown cost",
            ));
        }
        if estimated_total_tokens > self.max_estimated_total_tokens {
            return Err(ProviderError::new(
                ProviderErrorCode::PermissionRefused,
                format!(
                    "estimated {estimated_total_tokens} tokens exceed the permit ceiling of {} tokens",
                    self.max_estimated_total_tokens
                ),
            ));
        }
        Ok(())
    }
}

/// Authorization decision for one candidate, applying lock → permit in
/// order. Free models need no permit; paid/unknown always do (when the
/// lock is off).
pub fn authorize_cost(
    lock: &ZeroCostLock,
    cost: CostClass,
    provider: &str,
    model: &str,
    estimated_total_tokens: u64,
    permit: Option<&PaidInferencePermit>,
) -> Result<(), ProviderError> {
    lock.check(cost)?;
    if cost == CostClass::Free {
        return Ok(());
    }
    match permit {
        Some(p) => p.check(provider, model, cost, estimated_total_tokens),
        None => Err(ProviderError::new(
            ProviderErrorCode::PermissionRefused,
            format!(
                "dispatch to {provider}/{model} may be billable and no paid-inference permit was supplied"
            ),
        )),
    }
}

// ---------------------------------------------------------------------------
// Budgets and usage ledger (§§16, 67)
// ---------------------------------------------------------------------------

/// Project budget profiles (§67). The named profiles are qualitative in
/// the spec; ceilings are always caller-supplied numbers, never
/// invented allowances (§16). `FreeOnly` is the "Max Free mode" profile
/// (§16; decision M5-D7) and is meant to be combined with the lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BudgetProfile {
    FreeOnly,
    LowCost,
    Balanced,
    MaximumQuality,
    Custom,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Budget {
    pub profile: Option<BudgetProfile>,
    pub max_requests: Option<u64>,
    pub max_total_tokens: Option<u64>,
    /// Micro-USD ceiling, enforced only when a price is configured for
    /// the model (prices are caller-supplied with provenance, M5-D5).
    pub max_estimated_cost_micro_usd: Option<u64>,
}

impl Budget {
    pub fn free_only() -> Self {
        Self {
            profile: Some(BudgetProfile::FreeOnly),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderUsageTotals {
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Records *actual* usage reported by providers (never estimates
/// dressed as actuals) and enforces the caller-supplied budget.
#[derive(Debug, Default)]
pub struct QuotaLedger {
    totals: HashMap<String, ProviderUsageTotals>,
    total_requests: u64,
    total_tokens: u64,
    total_estimated_cost_micro_usd: u64,
    // Live reservations (see `reserve`): amounts committed by
    // in-flight dispatches. Budget checks count actuals + reserved,
    // so concurrent dispatches cannot each pass a check against the
    // same headroom and collectively overrun the budget.
    reserved_requests: u64,
    reserved_tokens: u64,
    reserved_cost_micro_usd: u64,
    reservations: HashMap<u64, Reservation>,
    next_reservation_id: u64,
}

/// A live budget reservation, created by `QuotaLedger::reserve` and
/// closed by exactly one of `settle` (dispatch completed), `release`
/// (dispatch failed before completion) or the stream pair
/// `begin_stream` + `settle_stream`. While it lives, its request,
/// estimated tokens and estimated cost count against every budget
/// check — the mechanism that makes concurrent dispatch accounting
/// safe instead of check-then-act.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reservation {
    id: u64,
    tokens: u64,
    cost_micro_usd: u64,
    request_pending: bool,
}

/// Whether provider-reported quota/usage information is available.
/// `Unavailable` is a first-class, reportable answer (§16: "Where
/// quota information is unavailable, say so").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaReport {
    ProviderReported { remaining_tokens: Option<u64> },
    Unavailable { reason: String },
}

impl QuotaLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_budget(
        &self,
        budget: &Budget,
        estimated_total_tokens: u64,
        estimated_cost_micro_usd: Option<u64>,
    ) -> Result<(), ProviderError> {
        if let Some(max) = budget.max_requests
            && self.total_requests + self.reserved_requests + 1 > max
        {
            return Err(ProviderError::new(
                ProviderErrorCode::QuotaExceeded,
                format!(
                    "request budget exhausted ({}/{} used, {} reserved, max {max})",
                    self.total_requests, self.total_requests, self.reserved_requests
                ),
            ));
        }
        if let Some(max) = budget.max_total_tokens
            && self.total_tokens + self.reserved_tokens + estimated_total_tokens > max
        {
            return Err(ProviderError::new(
                ProviderErrorCode::QuotaExceeded,
                format!(
                    "token budget would be exceeded ({} used + {} reserved + {estimated_total_tokens} estimated > {max})",
                    self.total_tokens, self.reserved_tokens
                ),
            ));
        }
        if let (Some(max), Some(cost)) = (
            budget.max_estimated_cost_micro_usd,
            estimated_cost_micro_usd,
        ) && self.total_estimated_cost_micro_usd + self.reserved_cost_micro_usd + cost > max
        {
            return Err(ProviderError::new(
                ProviderErrorCode::QuotaExceeded,
                "estimated-cost budget would be exceeded",
            ));
        }
        Ok(())
    }

    /// Check the budget and, if it passes, atomically hold the
    /// request slot, the estimated tokens and the estimated cost
    /// until the dispatch settles or releases. Callers holding the
    /// ledger lock (the dispatcher's `Arc<Mutex<QuotaLedger>>`)
    /// therefore get reservation semantics, not check-then-act.
    pub fn reserve(
        &mut self,
        budget: &Budget,
        estimated_total_tokens: u64,
        estimated_cost_micro_usd: Option<u64>,
    ) -> Result<Reservation, ProviderError> {
        self.check_budget(budget, estimated_total_tokens, estimated_cost_micro_usd)?;
        self.next_reservation_id += 1;
        let reservation = Reservation {
            id: self.next_reservation_id,
            tokens: estimated_total_tokens,
            cost_micro_usd: estimated_cost_micro_usd.unwrap_or(0),
            request_pending: true,
        };
        self.reserved_requests += 1;
        self.reserved_tokens += reservation.tokens;
        self.reserved_cost_micro_usd += reservation.cost_micro_usd;
        self.reservations.insert(reservation.id, reservation);
        Ok(reservation)
    }

    fn take_reservation(&mut self, reservation: Reservation) -> Option<Reservation> {
        let live = self.reservations.remove(&reservation.id)?;
        if live.request_pending {
            self.reserved_requests = self.reserved_requests.saturating_sub(1);
        }
        self.reserved_tokens = self.reserved_tokens.saturating_sub(live.tokens);
        self.reserved_cost_micro_usd = self
            .reserved_cost_micro_usd
            .saturating_sub(live.cost_micro_usd);
        Some(live)
    }

    /// The dispatch completed: release the hold and record the
    /// actual usage.
    pub fn settle(
        &mut self,
        reservation: Reservation,
        provider: &str,
        usage: Option<crate::types::Usage>,
        estimated_cost_micro_usd: Option<u64>,
    ) {
        if self.take_reservation(reservation).is_some() {
            self.record(provider, usage, estimated_cost_micro_usd);
        }
    }

    /// The dispatch failed before completion: release the hold,
    /// record nothing.
    pub fn release(&mut self, reservation: Reservation) {
        let _ = self.take_reservation(reservation);
    }

    /// A stream was established: the request itself is real, so
    /// record it now and release the request hold — but keep the
    /// token/cost hold until the stream ends (`settle_stream`),
    /// because the tokens are still in flight.
    pub fn begin_stream(
        &mut self,
        reservation: Reservation,
        provider: &str,
        estimated_cost_micro_usd: Option<u64>,
    ) {
        if let Some(mut live) = self.reservations.remove(&reservation.id) {
            if live.request_pending {
                self.reserved_requests = self.reserved_requests.saturating_sub(1);
                live.request_pending = false;
                self.record(provider, None, estimated_cost_micro_usd);
            }
            self.reservations.insert(live.id, live);
        }
    }

    /// The stream ended (completed, errored or cancelled): release
    /// the remaining hold and record the tokens the provider
    /// actually reported, if any. The request was already recorded
    /// by `begin_stream`.
    pub fn settle_stream(
        &mut self,
        reservation: Reservation,
        provider: &str,
        usage: Option<crate::types::Usage>,
    ) {
        if self.take_reservation(reservation).is_some()
            && let Some(u) = usage
        {
            self.total_tokens += u.total();
            let entry = self.totals.entry(provider.to_string()).or_default();
            entry.input_tokens += u.input_tokens;
            entry.output_tokens += u.output_tokens;
        }
    }

    /// Amounts currently held by live reservations (observability
    /// and tests).
    pub fn reserved(&self) -> (u64, u64) {
        (self.reserved_requests, self.reserved_tokens)
    }

    pub fn record(
        &mut self,
        provider: &str,
        usage: Option<crate::types::Usage>,
        estimated_cost_micro_usd: Option<u64>,
    ) {
        self.total_requests += 1;
        if let Some(u) = usage {
            self.total_tokens += u.total();
            let entry = self.totals.entry(provider.to_string()).or_default();
            entry.requests += 1;
            entry.input_tokens += u.input_tokens;
            entry.output_tokens += u.output_tokens;
        } else {
            let entry = self.totals.entry(provider.to_string()).or_default();
            entry.requests += 1;
        }
        if let Some(c) = estimated_cost_micro_usd {
            self.total_estimated_cost_micro_usd += c;
        }
    }

    /// Per-provider totals snapshot (observability / usage ops).
    pub fn totals_snapshot(&self) -> Vec<(String, ProviderUsageTotals)> {
        let mut out: Vec<_> = self
            .totals
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    pub fn totals_for(&self, provider: &str) -> ProviderUsageTotals {
        self.totals.get(provider).cloned().unwrap_or_default()
    }

    pub fn total_requests(&self) -> u64 {
        self.total_requests
    }
}

/// Estimated cost in micro-USD from caller-supplied prices; `None` when
/// no price is configured (unknown cost, M5-D5).
pub fn estimate_cost_micro_usd(
    model: &crate::types::ModelInfo,
    estimated_input_tokens: u64,
    estimated_output_tokens: u64,
) -> Option<u64> {
    let input = model.price_input_micro_usd_per_mtok?;
    let output = model.price_output_micro_usd_per_mtok?;
    let cost = (input * estimated_input_tokens + output * estimated_output_tokens) / 1_000_000;
    Some(cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChatMessage, Role, Segment};

    fn request_with(privacy: PrivacyClass) -> CompletionRequest {
        CompletionRequest::new(
            "m",
            vec![ChatMessage {
                role: Role::User,
                segments: vec![Segment::classified("x", privacy)],
            }],
        )
    }

    #[test]
    fn sealed_refused_everywhere() {
        let r = request_with(PrivacyClass::Sealed);
        assert!(PrivacyGate::check(&r, TargetLocality::Local).is_err());
        assert!(PrivacyGate::check(&r, TargetLocality::Cloud).is_err());
    }

    #[test]
    fn private_cloud_needs_explicit_authorization() {
        let r = request_with(PrivacyClass::Private);
        assert!(PrivacyGate::check(&r, TargetLocality::Local).is_ok());
        assert!(PrivacyGate::check(&r, TargetLocality::Cloud).is_err());
        let mut r2 = request_with(PrivacyClass::Private);
        r2.explicit_cloud_authorization = true;
        assert!(PrivacyGate::check(&r2, TargetLocality::Cloud).is_ok());
    }

    #[test]
    fn lock_blocks_paid_and_unknown_but_not_free() {
        let lock = ZeroCostLock::on();
        assert!(lock.check(CostClass::Free).is_ok());
        assert_eq!(
            lock.check(CostClass::Paid).unwrap_err().code,
            ProviderErrorCode::CostRefused
        );
        assert!(lock.check(CostClass::Unknown).is_err());
        assert!(ZeroCostLock::off().check(CostClass::Paid).is_ok());
    }

    #[test]
    fn permit_must_match_and_cover() {
        let permit = PaidInferencePermit {
            id: "p1".into(),
            provider: "openai".into(),
            model: "gpt-x".into(),
            max_estimated_total_tokens: 1000,
            allow_unknown_cost: false,
        };
        assert!(
            permit
                .check("openai", "gpt-x", CostClass::Paid, 900)
                .is_ok()
        );
        assert!(
            permit
                .check("openai", "gpt-x", CostClass::Paid, 1001)
                .is_err()
        );
        assert!(
            permit
                .check("openai", "other", CostClass::Paid, 10)
                .is_err()
        );
        assert!(
            permit
                .check("openai", "gpt-x", CostClass::Unknown, 10)
                .is_err()
        );
    }

    #[test]
    fn budget_token_ceiling() {
        let mut ledger = QuotaLedger::new();
        let budget = Budget {
            max_total_tokens: Some(100),
            ..Default::default()
        };
        assert!(ledger.check_budget(&budget, 60, None).is_ok());
        ledger.record(
            "p",
            Some(crate::types::Usage {
                input_tokens: 30,
                output_tokens: 20,
            }),
            None,
        );
        assert!(ledger.check_budget(&budget, 60, None).is_err());
        assert!(ledger.check_budget(&budget, 50, None).is_ok());
    }
}

#[cfg(test)]
mod reservation_tests {
    use super::*;

    fn budget_tokens(max: u64) -> Budget {
        Budget {
            max_total_tokens: Some(max),
            ..Default::default()
        }
    }

    #[test]
    fn concurrent_reservations_cannot_overrun_the_budget() {
        // Two dispatches each estimating 60 against a 100-token
        // budget: check-then-act would pass both; reservations must
        // refuse the second.
        let mut ledger = QuotaLedger::new();
        let budget = budget_tokens(100);
        let first = ledger.reserve(&budget, 60, None).unwrap();
        let err = ledger.reserve(&budget, 60, None).unwrap_err();
        assert_eq!(err.code, ProviderErrorCode::QuotaExceeded);
        // Settling the first with 50 actual frees real headroom:
        // a 45 reservation now fits alongside the 50 recorded.
        ledger.settle(
            first,
            "openai",
            Some(crate::types::Usage {
                input_tokens: 30,
                output_tokens: 20,
            }),
            None,
        );
        assert!(ledger.reserve(&budget, 45, None).is_ok());
        assert_eq!(ledger.reserved().1, 45);
    }

    #[test]
    fn release_frees_the_hold_without_recording() {
        let mut ledger = QuotaLedger::new();
        let budget = budget_tokens(100);
        let r = ledger.reserve(&budget, 90, None).unwrap();
        ledger.release(r);
        assert_eq!(ledger.reserved(), (0, 0));
        assert_eq!(ledger.total_requests(), 0);
        assert!(ledger.reserve(&budget, 90, None).is_ok());
    }

    #[test]
    fn request_budget_counts_reservations() {
        let mut ledger = QuotaLedger::new();
        let budget = Budget {
            max_requests: Some(2),
            ..Default::default()
        };
        let _a = ledger.reserve(&budget, 1, None).unwrap();
        let _b = ledger.reserve(&budget, 1, None).unwrap();
        assert!(ledger.reserve(&budget, 1, None).is_err());
    }

    #[test]
    fn stream_lifecycle_records_request_then_tokens() {
        let mut ledger = QuotaLedger::new();
        let budget = Budget::default();
        let r = ledger.reserve(&budget, 80, None).unwrap();
        ledger.begin_stream(r, "openai", None);
        assert_eq!(ledger.total_requests(), 1);
        assert_eq!(
            ledger.reserved(),
            (0, 80),
            "token hold survives establishment"
        );
        ledger.settle_stream(
            r,
            "openai",
            Some(crate::types::Usage {
                input_tokens: 10,
                output_tokens: 5,
            }),
        );
        assert_eq!(ledger.reserved(), (0, 0));
        let totals = ledger.totals_for("openai");
        assert_eq!(totals.requests, 1);
        assert_eq!(totals.input_tokens + totals.output_tokens, 15);
    }
}
