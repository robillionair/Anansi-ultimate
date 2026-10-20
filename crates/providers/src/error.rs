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
    /// HTTPS endpoint but no TLS transport is wired (decision M5-D3;
    /// superseded for dispatch by `RustlsTransport`, kept for plain
    /// transports and for integrators that have not enabled TLS).
    TlsUnavailable,
    /// TLS handshake failed — certificate invalid, expired, untrusted,
    /// wrong-host, or a protocol error — before any request bytes
    /// were sent. The message never contains credential material.
    TlsError,
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
    /// Whether the request bytes had been fully written to the
    /// provider when this failure occurred. This is the load-bearing
    /// fact for billable-retry safety: `false` means the provider
    /// never received a complete request, so retrying cannot
    /// duplicate inference; `true` means execution is ambiguous for
    /// transport-class failures (the provider may have run — and
    /// billed — the request before the failure was observed).
    /// Defaults to `true` (the conservative assumption); transports
    /// explicitly mark DNS/connect/TLS-handshake failures with
    /// [`ProviderError::pre_send`].
    pub request_sent: bool,
}

impl ProviderError {
    pub fn new(code: ProviderErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            status: None,
            retry_after: None,
            request_sent: true,
        }
    }
    /// Mark that the failure happened before the request was sent
    /// (DNS, connect, TLS handshake). Only transports call this, and
    /// only for failures where no complete request can have reached
    /// the provider.
    pub fn pre_send(mut self) -> Self {
        self.request_sent = false;
        self
    }
    pub fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }
    pub fn with_retry_after(mut self, after: Duration) -> Self {
        self.retry_after = Some(after);
        self
    }
    /// Whether this failure class is retryable *in principle*. The
    /// dispatcher applies the billable-safety policy on top of this
    /// (see `dispatcher::may_retry`): a post-send ambiguous failure of
    /// a paid model is never auto-retried even when this returns true.
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
            ProviderErrorCode::TlsError => "TlsError",
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
