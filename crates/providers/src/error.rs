//! Sanitized provider error taxonomy. Every message that can reach a log
//! or a caller passes through `redact::Redactor` at construction sites
//! that include provider-supplied text; provider response bodies are
//! excerpted and capped, never echoed whole.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderErrorCode {
    /// 401/403 from the provider, or a missing credential.
    AuthFailed,
    /// 429. `retry_after` carries the honoured hint when present.
    RateLimited,
    Timeout,
    ConnectionLost,
    MalformedResponse,
    /// A gate refused before any network request (authorization, permit).
    PermissionRefused,
    /// Zero-Cost Lock or cost policy refusal, before any network request.
    CostRefused,
    /// Budget/quota ceiling reached, before any network request.
    QuotaExceeded,
    /// Privacy gate refusal, before any network request.
    PrivacyRefused,
    /// Endpoint failed policy validation, or a redirect was refused.
    EndpointRejected,
    /// HTTPS endpoint but no TLS transport is wired (decision M5-D3).
    TlsUnavailable,
    Cancelled,
    /// 5xx or otherwise unavailable provider.
    ProviderUnavailable,
    /// The adapter/provider does not support the requested operation.
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderError {
    pub code: ProviderErrorCode,
    pub message: String,
    pub status: Option<u16>,
    pub retry_after: Option<Duration>,
}

impl ProviderError {
    pub fn new(code: ProviderErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            status: None,
            retry_after: None,
        }
    }
    pub fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }
    pub fn with_retry_after(mut self, after: Duration) -> Self {
        self.retry_after = Some(after);
        self
    }
    /// Whether the retry policy may retry this failure (same candidate).
    pub fn retryable(&self) -> bool {
        match self.code {
            ProviderErrorCode::RateLimited | ProviderErrorCode::ProviderUnavailable => true,
            // A dropped connection is only retryable before any streamed
            // event was emitted; the dispatcher tracks that separately.
            ProviderErrorCode::ConnectionLost => true,
            _ => false,
        }
    }
    /// Stable code name for compact, secret-free attempt records.
    pub fn code_name(&self) -> &'static str {
        match self.code {
            ProviderErrorCode::AuthFailed => "AuthFailed",
            ProviderErrorCode::RateLimited => "RateLimited",
            ProviderErrorCode::Timeout => "Timeout",
            ProviderErrorCode::ConnectionLost => "ConnectionLost",
            ProviderErrorCode::MalformedResponse => "MalformedResponse",
            ProviderErrorCode::PermissionRefused => "PermissionRefused",
            ProviderErrorCode::CostRefused => "CostRefused",
            ProviderErrorCode::QuotaExceeded => "QuotaExceeded",
            ProviderErrorCode::PrivacyRefused => "PrivacyRefused",
            ProviderErrorCode::EndpointRejected => "EndpointRejected",
            ProviderErrorCode::TlsUnavailable => "TlsUnavailable",
            ProviderErrorCode::Cancelled => "Cancelled",
            ProviderErrorCode::ProviderUnavailable => "ProviderUnavailable",
            ProviderErrorCode::Unsupported => "Unsupported",
        }
    }

    /// Whether this refusal happened before any network request. Gate
    /// tests assert on this distinction.
    pub fn refused_before_network(&self) -> bool {
        matches!(
            self.code,
            ProviderErrorCode::PermissionRefused
                | ProviderErrorCode::CostRefused
                | ProviderErrorCode::QuotaExceeded
                | ProviderErrorCode::PrivacyRefused
                | ProviderErrorCode::EndpointRejected
        )
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProviderError {}
