//! Provider configuration and endpoint policy.
//!
//! Endpoint policy is a security control (MASTER_SPEC §71: SSRF, DNS
//! rebinding, credential leakage), enforced deterministically in the
//! dispatcher — never by a model. Rules:
//! - scheme is `https` for anything non-loopback; `http` is loopback-only
//!   (plus an explicitly flagged insecure custom endpoint that carries no
//!   credential);
//! - no userinfo, no fragment, no query string in a configured base URL;
//! - redirects are never followed by the transport (a 3xx is surfaced as
//!   an error), so a configured host cannot silently bounce a request —
//!   and its credentials — elsewhere.

use crate::error::{ProviderError, ProviderErrorCode};
use crate::secrets::CredentialRef;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub provider: String,
    /// Base URL as entered by the user / supplied by the catalog.
    pub base_url: String,
    /// Credential reference only; never a secret value (§15, §70).
    pub credential: Option<CredentialRef>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// True for user-defined OpenAI-compatible / enterprise endpoints
    /// (§14). Custom endpoints get the same policy checks, not fewer.
    #[serde(default)]
    pub custom_endpoint: bool,
    /// Explicit owner opt-in for a plaintext non-loopback custom
    /// endpoint (e.g. an internal enterprise gateway). Never valid
    /// together with a credential.
    #[serde(default)]
    pub allow_insecure_custom: bool,
}

fn default_true() -> bool {
    true
}

impl ProviderConfig {
    pub fn new(provider: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            base_url: base_url.into(),
            credential: None,
            enabled: true,
            custom_endpoint: false,
            allow_insecure_custom: false,
        }
    }
    pub fn validate(&self) -> Result<ValidatedEndpoint, ProviderError> {
        validate_endpoint(self)
    }
}

/// A base URL after policy validation. The dispatcher and transport use
/// only this form, so there is exactly one validation point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedEndpoint {
    pub scheme: String, // "http" | "https"
    pub host: String,   // lowercase, brackets stripped for IPv6
    pub port: u16,
    pub base_path: String, // "" or "/v1" etc, no trailing slash
    pub loopback: bool,
}

impl ValidatedEndpoint {
    pub fn authority(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
    pub fn origin(&self) -> String {
        format!("{}://{}", self.scheme, self.authority())
    }
    /// Join a request path (which must start with `/`) onto the base.
    pub fn url_for(&self, path: &str) -> String {
        format!("{}{}{}", self.origin(), self.base_path, path)
    }
}

fn endpoint_err(message: impl Into<String>) -> ProviderError {
    ProviderError::new(ProviderErrorCode::EndpointRejected, message)
}

pub fn validate_endpoint(config: &ProviderConfig) -> Result<ValidatedEndpoint, ProviderError> {
    let raw = config.base_url.trim();
    let (scheme, rest) = raw
        .split_once("://")
        .ok_or_else(|| endpoint_err(format!("endpoint '{raw}' has no scheme")))?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(endpoint_err(format!(
            "endpoint scheme '{scheme}' is not allowed; use https (or http on loopback)"
        )));
    }
    if rest.contains('#') {
        return Err(endpoint_err("endpoint must not contain a fragment"));
    }
    if rest.contains('?') {
        return Err(endpoint_err(
            "endpoint must not contain a query string (secrets do not belong in URLs)",
        ));
    }
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    if authority.is_empty() {
        return Err(endpoint_err("endpoint has no host"));
    }
    if authority.contains('@') {
        return Err(endpoint_err(
            "endpoint must not contain userinfo/credentials in the URL",
        ));
    }
    let (host, port) = parse_authority(authority, &scheme)?;
    let host_lc = host.to_ascii_lowercase();
    let loopback = is_loopback_host(&host_lc);
    if scheme == "http" && !loopback {
        let allowed = config.custom_endpoint && config.allow_insecure_custom;
        if !allowed {
            return Err(endpoint_err(format!(
                "plaintext http endpoint '{host_lc}' is not loopback; use https or explicitly opt in to an insecure custom endpoint"
            )));
        }
        if config.credential.is_some() {
            return Err(endpoint_err(
                "a credential must never be sent over plaintext http to a non-loopback host",
            ));
        }
    }
    Ok(ValidatedEndpoint {
        scheme,
        host: host_lc,
        port,
        base_path: path.trim_end_matches('/').to_string(),
        loopback,
    })
}

fn parse_authority(authority: &str, scheme: &str) -> Result<(String, u16), ProviderError> {
    let default_port = if scheme == "https" { 443 } else { 80 };
    if let Some(stripped) = authority.strip_prefix('[') {
        // IPv6 literal: [::1] or [::1]:8080
        let Some(end) = stripped.find(']') else {
            return Err(endpoint_err("malformed IPv6 endpoint host"));
        };
        let host = &stripped[..end];
        let tail = &stripped[end + 1..];
        let port = if tail.is_empty() {
            default_port
        } else if let Some(p) = tail.strip_prefix(':') {
            p.parse::<u16>()
                .map_err(|_| endpoint_err("invalid endpoint port"))?
        } else {
            return Err(endpoint_err("malformed IPv6 endpoint authority"));
        };
        return Ok((host.to_string(), port));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => {
            let port = port
                .parse::<u16>()
                .map_err(|_| endpoint_err("invalid endpoint port"))?;
            Ok((host.to_string(), port))
        }
        _ => Ok((authority.to_string(), default_port)),
    }
}

pub fn is_loopback_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    if h == "localhost" || h.ends_with(".localhost") || h == "::1" {
        return true;
    }
    // 127.0.0.0/8
    if let Some(first) = h.split('.').next()
        && first == "127"
        && h.split('.').count() == 4
        && h.split('.').all(|o| o.parse::<u8>().is_ok())
    {
        return true;
    }
    false
}

/// Bounded timeouts (decision M5-D6: package policy values, clamped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeoutConfig {
    pub connect_ms: u64,
    pub first_byte_ms: u64,
    pub idle_ms: u64,
    /// ONE overall deadline per transport execution, spanning DNS
    /// resolution, connect, the TLS handshake, the request write and
    /// the response read (enforced in `http.rs`). The phase values
    /// bound inactivity within a phase; `total_ms` bounds the whole,
    /// so a peer dribbling bytes slowly is still cut off.
    pub total_ms: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_ms: 5_000,
            first_byte_ms: 30_000,
            idle_ms: 30_000,
            total_ms: 180_000,
        }
    }
}

impl TimeoutConfig {
    pub fn clamped(mut self) -> Self {
        self.connect_ms = self.connect_ms.clamp(100, 30_000);
        self.first_byte_ms = self.first_byte_ms.clamp(500, 120_000);
        self.idle_ms = self.idle_ms.clamp(500, 120_000);
        self.total_ms = self.total_ms.clamp(1_000, 600_000);
        self
    }
    pub fn connect(&self) -> Duration {
        Duration::from_millis(self.connect_ms)
    }
    pub fn first_byte(&self) -> Duration {
        Duration::from_millis(self.first_byte_ms)
    }
    pub fn idle(&self) -> Duration {
        Duration::from_millis(self.idle_ms)
    }
    pub fn total(&self) -> Duration {
        Duration::from_millis(self.total_ms)
    }
}

/// Bounded retry policy (decision M5-D6). Retries additionally pass the
/// cost gate: a billable retry is never automatic (§16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryPolicy {
    pub max_retries: u8,
    pub base_backoff_ms: u64,
    pub max_retry_after_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_backoff_ms: 250,
            max_retry_after_ms: 30_000,
        }
    }
}

impl RetryPolicy {
    pub fn clamped(mut self) -> Self {
        self.max_retries = self.max_retries.min(5);
        self.base_backoff_ms = self.base_backoff_ms.clamp(0, 10_000);
        self.max_retry_after_ms = self.max_retry_after_ms.clamp(0, 120_000);
        self
    }
    /// Delay before retry `attempt` (0-based), honouring Retry-After up
    /// to the configured cap.
    pub fn delay(&self, attempt: u8, retry_after: Option<Duration>) -> Duration {
        if let Some(after) = retry_after {
            return after.min(Duration::from_millis(self.max_retry_after_ms));
        }
        let factor = 1u64.checked_shl(attempt as u32).unwrap_or(32);
        Duration::from_millis(self.base_backoff_ms.saturating_mul(factor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(url: &str) -> ProviderConfig {
        ProviderConfig::new("test", url)
    }

    #[test]
    fn accepts_https_and_loopback_http() {
        let e = cfg("https://api.example.com/v1").validate().unwrap();
        assert_eq!(e.host, "api.example.com");
        assert_eq!(e.port, 443);
        assert_eq!(e.base_path, "/v1");
        assert!(!e.loopback);
        let l = cfg("http://127.0.0.1:11434").validate().unwrap();
        assert!(l.loopback);
        assert_eq!(l.port, 11434);
        assert!(cfg("http://localhost:1234/v1").validate().is_ok());
        assert!(cfg("http://[::1]:8080").validate().is_ok());
    }

    #[test]
    fn rejects_credentials_query_and_plaintext() {
        assert!(
            cfg("https://user:pass@api.example.com/v1")
                .validate()
                .is_err()
        );
        assert!(
            cfg("https://api.example.com/v1?key=secret")
                .validate()
                .is_err()
        );
        assert!(cfg("http://api.example.com/v1").validate().is_err());
        assert!(cfg("ftp://127.0.0.1/x").validate().is_err());
    }

    #[test]
    fn insecure_custom_needs_opt_in_and_no_credential() {
        let mut c = cfg("http://gateway.internal:8080/v1");
        c.custom_endpoint = true;
        assert!(c.validate().is_err());
        c.allow_insecure_custom = true;
        assert!(c.validate().is_ok());
        c.credential = Some(CredentialRef::new("p", "r"));
        assert!(c.validate().is_err());
    }

    #[test]
    fn retry_after_is_capped() {
        let p = RetryPolicy::default();
        assert_eq!(
            p.delay(0, Some(Duration::from_secs(90))),
            Duration::from_millis(30_000)
        );
        assert_eq!(p.delay(1, None), Duration::from_millis(500));
    }
}
