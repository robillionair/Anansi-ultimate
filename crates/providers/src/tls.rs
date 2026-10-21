//! TLS configuration for [`crate::http::RustlsTransport`].
//!
//! This crate does **not** implement TLS or any cryptographic
//! primitive. It configures rustls — the maintained Rust TLS library —
//! with the ring crypto provider and the Mozilla root set from
//! `webpki-roots`. Certificate-chain and hostname validation are
//! rustls/WebPKI defaults and there is deliberately **no API in this
//! crate to weaken or disable them**: no accept-any-certificate
//! verifier, no danger flags, no environment-variable bypass.
//!
//! The only trust customization offered is *adding* root
//! certificates ([`TlsConfig::add_root_pem`] /
//! [`TlsConfig::add_root_der`]) — for private-CA deployments and for
//! the synthetic test CAs. Adding a root can only extend trust to
//! CAs the operator explicitly chose; validation itself (chain,
//! expiry, hostname/SAN) always runs.

use crate::error::{ProviderError, ProviderErrorCode};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;

/// Trust configuration for HTTPS dispatch. Defaults to the Mozilla
/// root set (via `webpki-roots`), exactly as shipped browsers trust.
#[derive(Debug, Clone, Default)]
pub struct TlsConfig {
    extra_roots: Vec<CertificateDer<'static>>,
}

impl TlsConfig {
    /// Mozilla roots, no additions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a root certificate (DER form) to the trust set.
    pub fn add_root_der(&mut self, der: CertificateDer<'static>) -> &mut Self {
        self.extra_roots.push(der);
        self
    }

    /// Add every certificate found in a PEM bundle to the trust set.
    pub fn add_root_pem(&mut self, pem: &[u8]) -> Result<&mut Self, ProviderError> {
        let mut added = 0usize;
        for item in CertificateDer::pem_slice_iter(pem) {
            let cert = item.map_err(|e| {
                ProviderError::new(
                    ProviderErrorCode::TlsError,
                    format!("invalid PEM root certificate: {e}"),
                )
            })?;
            self.extra_roots.push(cert);
            added += 1;
        }
        if added == 0 {
            return Err(ProviderError::new(
                ProviderErrorCode::TlsError,
                "no certificate found in PEM root bundle",
            ));
        }
        Ok(self)
    }

    /// Build the rustls client configuration. Validation is the
    /// rustls default: WebPKI chain building to the configured roots,
    /// expiry checks, and SAN/hostname verification against the
    /// endpoint host at handshake time.
    pub fn client_config(&self) -> Arc<ClientConfig> {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        for extra in &self.extra_roots {
            // A duplicate or malformed extra root must not silently
            // weaken anything; rustls rejects invalid additions and we
            // keep the strict Mozilla set regardless.
            let _ = roots.add(extra.clone());
        }
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("ring provider supports the default protocol versions")
            .with_root_certificates(roots)
            .with_no_client_auth();
        Arc::new(config)
    }
}

/// The rustls server name (DNS name or IP) for an endpoint host.
pub(crate) fn server_name(host: &str) -> Result<ServerName<'static>, ProviderError> {
    ServerName::try_from(host.to_string()).map_err(|e| {
        ProviderError::new(
            ProviderErrorCode::TlsError,
            format!("endpoint host '{host}' is not a valid TLS server name: {e}"),
        )
        .pre_send()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_builds_with_validation_defaults() {
        // Construction must succeed and must not require any bypass;
        // the real assertions on validation behaviour live in the
        // synthetic HTTPS integration tests.
        let cfg = TlsConfig::new();
        let _ = cfg.client_config();
    }

    #[test]
    fn pem_without_certificates_is_an_error() {
        let mut cfg = TlsConfig::new();
        assert!(cfg.add_root_pem(b"not a pem bundle").is_err());
    }

    #[test]
    fn server_name_accepts_dns_and_ip() {
        assert!(server_name("api.example.com").is_ok());
        assert!(server_name("127.0.0.1").is_ok());
        assert!(server_name("").is_err());
    }
}
