//! The application-facing runtime: provider *configuration* in,
//! authorized dispatch (unary and streamed) out, with the quota
//! ledger and the single-use permit set held across calls.
//!
//! This is the seam the application integrates against (see the
//! core-side `provider_dispatch` wiring): the app builds one
//! [`ProviderRuntime`] from a serializable [`RuntimeConfig`], stores
//! credentials through its [`SecretResolver`] (in production, the
//! [`crate::keyring_resolver::KeyringSecretResolver`]), and calls
//! `dispatch` / `dispatch_stream`. Everything the M5 gates promise —
//! privacy before transmission, Zero-Cost Lock, permits, budgets,
//! disclosure — is enforced inside, per call, by the `Dispatcher`.
//!
//! A fresh `Dispatcher` is assembled per call (adapters are
//! stateless), but the permit set and the ledger are the runtime's,
//! so a paid-inference permit can never authorize two dispatches
//! just because the application called twice.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::catalog::default_adapters;
use crate::config::{ProviderConfig, RetryPolicy, TimeoutConfig};
use crate::dispatcher::{DispatchContext, DispatchOutcome, Dispatcher, StreamOutcome};
use crate::error::{ProviderError, ProviderErrorCode};
use crate::gates::{Budget, PaidInferencePermit, QuotaLedger, TargetLocality};
use crate::http::{CancellationToken, RustlsTransport};
use crate::routing::{ProviderRegistration, RoutingRequest};
use crate::secrets::SecretResolver;
use crate::types::{CompletionRequest, ModelInfo};

/// Where a provider target runs, for the configuration format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalityConfig {
    Local,
    Cloud,
}

impl From<LocalityConfig> for TargetLocality {
    fn from(value: LocalityConfig) -> Self {
        match value {
            LocalityConfig::Local => TargetLocality::Local,
            LocalityConfig::Cloud => TargetLocality::Cloud,
        }
    }
}

/// One configured provider: endpoint/credential configuration, its
/// locality, and the models the user has configured for it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProvider {
    pub config: ProviderConfig,
    pub locality: LocalityConfig,
    pub models: Vec<ModelInfo>,
}

/// The complete provider configuration an application persists and
/// hands to the runtime. Credentials are NOT part of it — only
/// `ProviderConfig.credential` references, resolved at dispatch time
/// through the secret resolver (§15: configuration and secrets are
/// separate stores).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfig {
    pub providers: Vec<RuntimeProvider>,
    /// The dispatch budget for this configuration (requests, tokens,
    /// estimated cost). Wired into every dispatch — there is no
    /// implicit unlimited default once a budget is configured.
    #[serde(default)]
    pub budget: Budget,
}

pub struct ProviderRuntime {
    transport: RustlsTransport,
    resolver: Box<dyn SecretResolver + Send + Sync>,
    registrations: Vec<ProviderRegistration>,
    used_permits: Arc<Mutex<HashSet<String>>>,
    ledger: Arc<Mutex<QuotaLedger>>,
    budget: Budget,
    timeouts: TimeoutConfig,
    retry: RetryPolicy,
}

impl ProviderRuntime {
    /// Build a runtime from configuration. Endpoint policy is
    /// validated eagerly, here, so a misconfiguration fails at
    /// configuration time — not mid-conversation.
    pub fn new(
        config: RuntimeConfig,
        transport: RustlsTransport,
        resolver: Box<dyn SecretResolver + Send + Sync>,
    ) -> Result<Self, ProviderError> {
        let mut seen = HashSet::new();
        let mut registrations = Vec::with_capacity(config.providers.len());
        for provider in config.providers {
            if !seen.insert(provider.config.provider.clone()) {
                return Err(ProviderError::new(
                    ProviderErrorCode::InvalidConfiguration,
                    format!(
                        "duplicate provider in configuration: {}",
                        provider.config.provider
                    ),
                ));
            }
            provider.config.validate().map_err(|e| {
                ProviderError::new(
                    ProviderErrorCode::InvalidConfiguration,
                    format!("provider {}: {}", provider.config.provider, e.message),
                )
            })?;
            registrations.push(ProviderRegistration {
                config: provider.config,
                locality: provider.locality.into(),
                models: provider.models,
            });
        }
        let budget = config.budget.clone();
        Ok(Self {
            transport,
            resolver,
            registrations,
            used_permits: Arc::new(Mutex::new(HashSet::new())),
            ledger: Arc::new(Mutex::new(QuotaLedger::new())),
            budget,
            timeouts: TimeoutConfig::default(),
            retry: RetryPolicy::default(),
        })
    }

    /// Production wiring: Mozilla-roots HTTPS transport and the
    /// native keyring resolver.
    pub fn production(config: RuntimeConfig) -> Result<Self, ProviderError> {
        Self::new(
            config,
            RustlsTransport::new(),
            Box::new(crate::keyring_resolver::KeyringSecretResolver::new()),
        )
    }

    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.timeouts = timeouts;
        self
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    pub fn registrations(&self) -> &[ProviderRegistration] {
        &self.registrations
    }

    /// The shared usage ledger (cumulative across all calls made
    /// through this runtime — and across runtimes, when shared via
    /// `with_shared_accounting`).
    pub fn ledger(&self) -> Arc<Mutex<QuotaLedger>> {
        self.ledger.clone()
    }

    /// Replace this runtime's accounting with shared state. The
    /// application uses this so permit consumption and usage totals
    /// survive configuration changes: rebuilding the runtime for a
    /// new configuration must never resurrect a spent permit or
    /// reset the ledger (proved by the A→B→A replay test in core).
    pub fn with_shared_accounting(
        mut self,
        used_permits: Arc<Mutex<HashSet<String>>>,
        ledger: Arc<Mutex<QuotaLedger>>,
    ) -> Self {
        self.used_permits = used_permits;
        self.ledger = ledger;
        self
    }

    /// One authorized unary dispatch, with disclosure.
    pub fn dispatch(
        &mut self,
        request: &CompletionRequest,
        routing: RoutingRequest,
        permit: Option<PaidInferencePermit>,
        cancel: &CancellationToken,
    ) -> Result<DispatchOutcome, ProviderError> {
        let dispatcher = Dispatcher::new(&self.transport, &*self.resolver, default_adapters())
            .with_timeouts(self.timeouts)
            .with_retry(self.retry)
            .with_used_permits(self.used_permits.clone());
        let lock = routing.lock;
        let ctx = DispatchContext {
            routing,
            lock,
            budget: self.budget.clone(),
            ledger: self.ledger.clone(),
            permit,
        };
        dispatcher.execute(&self.registrations, request, ctx, cancel)
    }

    /// One authorized streamed dispatch. The returned outcome owns
    /// the event channel; cancelling `cancel` closes the stream with
    /// a `Cancelled` event, and the disclosure record is available
    /// immediately (it describes what was authorized and attempted).
    pub fn dispatch_stream(
        &mut self,
        request: &CompletionRequest,
        routing: RoutingRequest,
        permit: Option<PaidInferencePermit>,
        cancel: &CancellationToken,
    ) -> Result<StreamOutcome, ProviderError> {
        let dispatcher = Dispatcher::new(&self.transport, &*self.resolver, default_adapters())
            .with_timeouts(self.timeouts)
            .with_retry(self.retry)
            .with_used_permits(self.used_permits.clone());
        let lock = routing.lock;
        let ctx = DispatchContext {
            routing,
            lock,
            budget: self.budget.clone(),
            ledger: self.ledger.clone(),
            permit,
        };
        dispatcher.execute_stream(&self.registrations, request, ctx, cancel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::RoutingMode;
    use crate::secrets::{AuthMaterial, CredentialRef, SecretValue};
    use crate::types::{ChatMessage, CostClass, Role};
    use std::collections::HashMap as Map;

    struct MapResolver(Map<String, String>);
    impl SecretResolver for MapResolver {
        fn resolve(&self, credential: &CredentialRef) -> Result<AuthMaterial, ProviderError> {
            self.0
                .get(&credential.reference)
                .map(|k| AuthMaterial::ApiKey(SecretValue::new(k.clone())))
                .ok_or_else(|| {
                    ProviderError::new(ProviderErrorCode::AuthFailed, "not found").pre_send()
                })
        }
    }

    fn config() -> RuntimeConfig {
        let mut provider = ProviderConfig::new("openai", "https://api.example.invalid/v1");
        provider.credential = Some(CredentialRef::new("openai", "openai-primary"));
        RuntimeConfig {
            budget: Budget::default(),
            providers: vec![RuntimeProvider {
                config: provider,
                locality: LocalityConfig::Cloud,
                models: vec![ModelInfo {
                    id: "gpt-x".into(),
                    provider: "openai".into(),
                    cost: CostClass::Paid,
                    capabilities: Default::default(),
                    context_window: Some(8192),
                    speed: crate::types::SpeedTier::Balanced,
                    price_input_micro_usd_per_mtok: None,
                    price_output_micro_usd_per_mtok: None,
                }],
            }],
        }
    }

    #[test]
    fn duplicate_providers_are_rejected_at_configuration_time() {
        let mut c = config();
        c.providers.push(c.providers[0].clone());
        let err =
            ProviderRuntime::new(c, RustlsTransport::new(), Box::new(MapResolver(Map::new())))
                .err()
                .expect("duplicates must fail");
        assert_eq!(err.code, ProviderErrorCode::InvalidConfiguration);
    }

    #[test]
    fn invalid_endpoints_are_rejected_at_configuration_time() {
        let mut c = config();
        c.providers[0].config.base_url = "http://api.example.invalid/v1".into();
        let err =
            ProviderRuntime::new(c, RustlsTransport::new(), Box::new(MapResolver(Map::new())))
                .err()
                .expect("plaintext cloud endpoint must fail");
        assert_eq!(err.code, ProviderErrorCode::InvalidConfiguration);
    }

    #[test]
    fn configuration_roundtrips_through_json_without_secrets() {
        let json = serde_json::to_string(&config()).unwrap();
        assert!(!json.contains("sk-"), "config format must never carry keys");
        let back: RuntimeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.providers.len(), 1);
        assert_eq!(back.providers[0].models[0].id, "gpt-x");
        let _ = ChatMessage::new(Role::User, "x");
        let _ = RoutingMode::Auto;
    }
}
