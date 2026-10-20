//! The native secret resolver: provider API keys in the OS
//! credential store via the `keyring` crate (Windows Credential
//! Manager, macOS Keychain, Linux Secret Service), implementing the
//! existing `SecretResolver` boundary.
//!
//! Conventions, chosen to sit beside — never inside — the vault:
//! vault entries live under service `com.robillionair.anansi.vault`
//! (see `crates/core/src/vault/credential_store.rs`); provider
//! credentials live under [`PROVIDER_CREDENTIAL_SERVICE`], one entry
//! per credential reference (the entry "user" is the
//! `CredentialRef.reference`, e.g. `openai-primary`). The two stores
//! cannot collide, and the resolver never sees vault material.
//!
//! Failure posture mirrors core: where no native store is available
//! (e.g. a Linux build without the `linux-secret-service` feature,
//! or no Secret Service daemon), resolution fails closed with
//! `AuthFailed` — never a fallback to files, environment variables
//! or prompts. Secrets are never logged: error messages carry the
//! reference and a bounded store error, never secret bytes.
//!
//! The resolver uses the portable `keyring::Entry::new` API, so the
//! keyring crate's *mock* credential builder intercepts it in tests
//! (see the unit tests) — no test touches a real credential store.

use crate::error::{ProviderError, ProviderErrorCode};
use crate::secrets::{AuthMaterial, CredentialRef, SecretResolver, SecretValue};

/// OS credential-store service name for all provider credentials.
pub const PROVIDER_CREDENTIAL_SERVICE: &str = "com.robillionair.anansi.providers";

#[derive(Debug)]
pub struct KeyringSecretResolver {
    service: String,
    /// Entry handles are cached per reference: cheaper in
    /// production, and required for the keyring mock builder (which
    /// scopes stored data to the entry instance) in tests.
    entries: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<keyring::Entry>>>,
}

impl Default for KeyringSecretResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringSecretResolver {
    pub fn new() -> Self {
        Self {
            service: PROVIDER_CREDENTIAL_SERVICE.to_string(),
            entries: Default::default(),
        }
    }

    /// A resolver over a different store service namespace (tests,
    /// embedders that keep per-profile stores).
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            entries: Default::default(),
        }
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    fn entry(&self, reference: &str) -> Result<std::sync::Arc<keyring::Entry>, ProviderError> {
        if let Some(entry) = self
            .entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(reference)
        {
            return Ok(entry.clone());
        }
        let entry = keyring::Entry::new(&self.service, reference).map_err(|e| {
            ProviderError::new(
                ProviderErrorCode::AuthFailed,
                format!(
                    "native credential store unavailable for reference {reference}: {}",
                    bounded(&e.to_string())
                ),
            )
            .pre_send()
        })?;
        let entry = std::sync::Arc::new(entry);
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(reference.to_string(), entry.clone());
        Ok(entry)
    }

    /// Store (or replace) the API key for a credential reference.
    /// This is the write half of the §15 "user pastes an API key"
    /// setup flow; the key passes through memory only — callers must
    /// not log or persist it anywhere else.
    pub fn store_api_key(&self, reference: &str, secret: &[u8]) -> Result<(), ProviderError> {
        self.entry(reference)?.set_secret(secret).map_err(|e| {
            ProviderError::new(
                ProviderErrorCode::AuthFailed,
                format!(
                    "could not store credential for reference {reference}: {}",
                    bounded(&e.to_string())
                ),
            )
            .pre_send()
        })
    }

    /// Remove a stored credential (provider removal / key rotation).
    pub fn delete(&self, reference: &str) -> Result<(), ProviderError> {
        let result = self.entry(reference)?.delete_credential().map_err(|e| {
            ProviderError::new(
                ProviderErrorCode::AuthFailed,
                format!(
                    "could not delete credential for reference {reference}: {}",
                    bounded(&e.to_string())
                ),
            )
            .pre_send()
        });
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(reference);
        result
    }
}

fn bounded(text: &str) -> String {
    text.chars().take(160).collect()
}

impl SecretResolver for KeyringSecretResolver {
    fn resolve(&self, credential: &CredentialRef) -> Result<AuthMaterial, ProviderError> {
        let bytes = self.entry(&credential.reference)?.get_secret().map_err(|e| {
            // Covers NoEntry as well as store failures: from the
            // dispatcher's side both mean "no usable credential".
            ProviderError::new(
                ProviderErrorCode::AuthFailed,
                format!(
                    "no usable credential for reference {} (provider {}): {}",
                    credential.reference,
                    credential.provider,
                    bounded(&e.to_string())
                ),
            )
            .pre_send()
        })?;
        let text = String::from_utf8(bytes).map_err(|_| {
            ProviderError::new(
                ProviderErrorCode::AuthFailed,
                format!(
                    "stored credential for reference {} is not valid UTF-8",
                    credential.reference
                ),
            )
        })?;
        Ok(AuthMaterial::ApiKey(SecretValue::new(text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static MOCK: Once = Once::new();

    /// Install the keyring mock builder once per test process. All
    /// tests then use unique service namespaces, so the shared mock
    /// store cannot couple them.
    fn mock_resolver() -> KeyringSecretResolver {
        MOCK.call_once(|| {
            keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        });
        KeyringSecretResolver::with_service(format!(
            "anansi-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn store_then_resolve_roundtrips_the_key() {
        let resolver = mock_resolver();
        resolver
            .store_api_key("openai-primary", b"sk-fake-keyring-0001")
            .unwrap();
        let material = resolver
            .resolve(&CredentialRef::new("openai", "openai-primary"))
            .unwrap();
        match material {
            AuthMaterial::ApiKey(v) => assert_eq!(v.expose(), "sk-fake-keyring-0001"),
            other => panic!("expected ApiKey, got {other:?}"),
        }
    }

    #[test]
    fn missing_reference_fails_closed() {
        let resolver = mock_resolver();
        let err = resolver
            .resolve(&CredentialRef::new("openai", "never-stored"))
            .unwrap_err();
        assert_eq!(err.code, ProviderErrorCode::AuthFailed);
        assert!(!err.request_sent);
    }

    #[test]
    fn delete_removes_the_credential() {
        let resolver = mock_resolver();
        resolver.store_api_key("doomed", b"sk-fake-doomed").unwrap();
        resolver.delete("doomed").unwrap();
        assert!(
            resolver
                .resolve(&CredentialRef::new("openai", "doomed"))
                .is_err()
        );
    }

    #[test]
    fn errors_never_contain_secret_material() {
        let resolver = mock_resolver();
        resolver
            .store_api_key("real-ref", b"sk-fake-must-not-leak")
            .unwrap();
        let err = resolver
            .resolve(&CredentialRef::new("openai", "other-ref"))
            .unwrap_err();
        assert!(!err.message.contains("sk-fake-must-not-leak"));
        // Debug output of resolved material is redacted by design.
        let material = resolver
            .resolve(&CredentialRef::new("openai", "real-ref"))
            .unwrap();
        assert!(!format!("{material:?}").contains("sk-fake-must-not-leak"));
    }
}
