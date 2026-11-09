//! Model routing (MASTER_SPEC §17) and the disclosure record (§19).
//!
//! The router produces an ordered fallback chain of candidates. It
//! evaluates privacy, cost, availability, capability, context window
//! and user preferences — in that order of precedence, because a
//! candidate that fails privacy or cost must not be rescued by being
//! fast or preferred. The dispatcher walks the chain and re-runs every
//! gate per candidate, so a fallback can never cross a boundary the
//! first candidate was stopped by (notably: free → paid under the
//! Zero-Cost Lock, and local → cloud for local-only data).

use crate::config::ProviderConfig;
use crate::error::{ProviderError, ProviderErrorCode};
use crate::gates::{TargetLocality, ZeroCostLock};
use crate::types::{CapabilitySet, CostClass, ModelInfo, PrivacyClass, Usage};
use serde::{Deserialize, Serialize};

/// User-facing routing modes, verbatim from MASTER_SPEC §17.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum RoutingMode {
    #[default]
    Auto,
    AutoFree,
    Private,
    Fast,
    Deep,
    Coding,
    Vision,
    ChooseModel,
}

/// One registered provider + the models the caller has configured for
/// it. Model lists are caller configuration: the catalog names
/// providers, it does not invent their model inventories.
#[derive(Debug, Clone)]
pub struct ProviderRegistration {
    pub config: ProviderConfig,
    pub locality: TargetLocality,
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub provider: String,
    pub model: ModelInfo,
    pub config_index: usize,
    pub locality: TargetLocality,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct RoutingRequest {
    pub mode: RoutingMode,
    pub required: CapabilitySet,
    pub local_only: bool,
    /// Engaged lock filters paid/unknown candidates out of the chain
    /// entirely (so fallback cannot cross into paid, §16).
    pub lock: ZeroCostLock,
    /// Preference order of provider ids (user preferences, §17).
    pub preferred_providers: Vec<String>,
    /// For `ChooseModel`: the exact provider/model the user picked.
    pub chosen: Option<(String, String)>,
    pub estimated_input_tokens: u64,
}

pub struct Router;

impl Router {
    pub fn plan(
        registrations: &[ProviderRegistration],
        request: &RoutingRequest,
    ) -> Result<Vec<Candidate>, ProviderError> {
        let mut candidates: Vec<Candidate> = Vec::new();
        for (idx, reg) in registrations.iter().enumerate() {
            if !reg.config.enabled {
                continue;
            }
            // Validate configuration lazily but deterministically: a
            // misconfigured provider is skipped with no candidate, and
            // direct dispatch to it fails in the dispatcher with the
            // validation error. Planning never rewrites config.
            if reg.config.validate().is_err() {
                continue;
            }
            for model in &reg.models {
                if let Some(reason) = Self::admit(reg, model, request) {
                    candidates.push(Candidate {
                        provider: reg.config.provider.clone(),
                        model: model.clone(),
                        config_index: idx,
                        locality: reg.locality,
                        reason,
                    });
                }
            }
        }
        // ChooseModel: exactly the chosen candidate, still gated later.
        if request.mode == RoutingMode::ChooseModel {
            if let Some((provider, model_id)) = &request.chosen {
                candidates.retain(|c| &c.provider == provider && &c.model.id == model_id);
                if candidates.is_empty() {
                    return Err(ProviderError::new(
                        ProviderErrorCode::Unsupported,
                        format!(
                            "chosen model {provider}/{model_id} is not registered, not enabled, or fails the active privacy/cost filters"
                        ),
                    ));
                }
                candidates.truncate(1);
                return Ok(candidates);
            }
            return Err(ProviderError::new(
                ProviderErrorCode::Unsupported,
                "ChooseModel mode requires a chosen provider/model",
            ));
        }
        if candidates.is_empty() {
            return Err(ProviderError::new(
                ProviderErrorCode::ProviderUnavailable,
                "no provider/model candidate satisfies the routing mode, privacy and cost constraints",
            ));
        }
        Self::order(&mut candidates, request);
        Ok(candidates)
    }

    /// Returns `Some(reason)` when the model is admitted to the chain.
    fn admit(
        reg: &ProviderRegistration,
        model: &ModelInfo,
        request: &RoutingRequest,
    ) -> Option<String> {
        let caps = &model.capabilities;
        let req = &request.required;
        if (req.chat && !caps.chat)
            || (req.streaming && !caps.streaming)
            || (req.vision && !caps.vision)
            || (req.coding && !caps.coding)
            || (req.deep_reasoning && !caps.deep_reasoning)
            || (req.embeddings && !caps.embeddings)
        {
            return None;
        }
        // Privacy first: local-only data and Private mode never plan a
        // cloud candidate at all (§17).
        if (request.local_only || request.mode == RoutingMode::Private)
            && reg.locality == TargetLocality::Cloud
        {
            return None;
        }
        // Cost: the lock and Auto Free admit free models only.
        if (request.lock.engaged || request.mode == RoutingMode::AutoFree)
            && model.cost != CostClass::Free
        {
            return None;
        }
        // Mode capability shaping.
        match request.mode {
            RoutingMode::Vision if !caps.vision => return None,
            RoutingMode::Coding if !caps.coding => return None,
            RoutingMode::Deep if !caps.deep_reasoning && !caps.chat => return None,
            _ => {}
        }
        let reason = match request.mode {
            RoutingMode::Private => "private mode: loopback-local candidate only",
            RoutingMode::AutoFree => "auto-free: free-cost candidate",
            RoutingMode::Fast => "fast mode: speed-tier ordered candidate",
            RoutingMode::Deep => "deep mode: reasoning-capable candidate",
            RoutingMode::Coding => "coding mode: coding-capable candidate",
            RoutingMode::Vision => "vision mode: vision-capable candidate",
            RoutingMode::ChooseModel => "user-chosen model",
            RoutingMode::Auto => "auto: preference/capability ordered candidate",
        };
        Some(reason.to_string())
    }

    fn order(candidates: &mut [Candidate], request: &RoutingRequest) {
        let pref = &request.preferred_providers;
        let pref_rank = |provider: &str| {
            pref.iter()
                .position(|p| p == provider)
                .unwrap_or(usize::MAX - 1)
        };
        candidates.sort_by(|a, b| {
            // Local before cloud in Private/AutoFree (already filtered),
            // then mode shaping, then user preference, then stable id.
            let tier = |c: &Candidate| match request.mode {
                RoutingMode::Fast => match c.model.speed {
                    crate::types::SpeedTier::Fast => 0,
                    crate::types::SpeedTier::Balanced => 1,
                    crate::types::SpeedTier::Deep => 2,
                },
                RoutingMode::Deep => match c.model.speed {
                    crate::types::SpeedTier::Deep => 0,
                    crate::types::SpeedTier::Balanced => 1,
                    crate::types::SpeedTier::Fast => 2,
                },
                _ => 0,
            };
            tier(a)
                .cmp(&tier(b))
                .then(pref_rank(&a.provider).cmp(&pref_rank(&b.provider)))
                .then(a.provider.cmp(&b.provider))
                .then(a.model.id.cmp(&b.model.id))
        });
    }
}

/// "View What ANANSI Sent" at adapter level (§19): what was about to be
/// / was transmitted, to whom, and why. The UI rendering of this record
/// is application integration; producing it is the dispatcher's job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DisclosureRecord {
    pub provider: String,
    pub model: String,
    pub endpoint_host: String,
    pub local: bool,
    pub routing_mode: RoutingMode,
    pub routing_reason: String,
    pub message_count: usize,
    pub segment_count: usize,
    pub redaction_count: usize,
    /// The strongest privacy class among the dispatched segments
    /// (the caller's label, as the gate saw it). Older disclosures
    /// without the field read as Standard.
    #[serde(default)]
    pub strongest_privacy: PrivacyClass,
    pub estimated_input_tokens: u64,
    pub cost_class: CostClass,
    pub usage: Option<Usage>,
    pub attempts: Vec<AttemptRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttemptRecord {
    pub provider: String,
    pub model: String,
    pub outcome: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProviderConfig;
    use crate::types::{CapabilitySet, SpeedTier};

    fn model(id: &str, provider: &str, cost: CostClass, caps: CapabilitySet) -> ModelInfo {
        ModelInfo {
            id: id.into(),
            provider: provider.into(),
            capabilities: caps,
            context_window: Some(8192),
            cost,
            speed: SpeedTier::Balanced,
            price_input_micro_usd_per_mtok: None,
            price_output_micro_usd_per_mtok: None,
        }
    }

    fn regs() -> Vec<ProviderRegistration> {
        let chat = CapabilitySet {
            chat: true,
            streaming: true,
            ..Default::default()
        };
        vec![
            ProviderRegistration {
                config: ProviderConfig::new("ollama", "http://127.0.0.1:11434"),
                locality: TargetLocality::Local,
                models: vec![model("qwen2.5:0.5b", "ollama", CostClass::Free, chat)],
            },
            ProviderRegistration {
                config: ProviderConfig::new("openai", "https://api.openai.com/v1"),
                locality: TargetLocality::Cloud,
                models: vec![model("gpt-x", "openai", CostClass::Paid, chat)],
            },
        ]
    }

    #[test]
    fn private_mode_plans_local_only() {
        let plan = Router::plan(
            &regs(),
            &RoutingRequest {
                mode: RoutingMode::Private,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].provider, "ollama");
    }

    #[test]
    fn lock_filters_paid_from_chain() {
        let plan = Router::plan(
            &regs(),
            &RoutingRequest {
                mode: RoutingMode::Auto,
                lock: ZeroCostLock::on(),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(plan.iter().all(|c| c.model.cost == CostClass::Free));
    }

    #[test]
    fn local_only_never_plans_cloud() {
        let plan = Router::plan(
            &regs(),
            &RoutingRequest {
                mode: RoutingMode::Auto,
                local_only: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(plan.iter().all(|c| c.locality == TargetLocality::Local));
    }

    #[test]
    fn choose_model_respects_filters() {
        let err = Router::plan(
            &regs(),
            &RoutingRequest {
                mode: RoutingMode::ChooseModel,
                chosen: Some(("openai".into(), "gpt-x".into())),
                lock: ZeroCostLock::on(),
                ..Default::default()
            },
        );
        assert!(err.is_err());
    }
}
