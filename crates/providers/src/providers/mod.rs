//! Provider adapters. An adapter is three pure pieces — build the HTTP
//! request, parse a complete response, decode a stream — so every
//! adapter is testable against fixtures without a network, and the
//! dispatcher owns all I/O, gating, retry and fallback policy.
//!
//! Adapter families (MASTER_SPEC §14 names *possible connections*;
//! §82 names OpenRouter plus "other verified provider integrations"):
//! - `OpenAiCompatibleAdapter`: the OpenAI Chat Completions shape, which
//!   most §14 providers officially expose (OpenAI, OpenRouter, Mistral,
//!   xAI, Moonshot/Kimi, NVIDIA NIM, Groq, Cerebras, Together, Fireworks,
//!   Perplexity, Cohere compatibility API, Z.AI, Hugging Face router,
//!   LM Studio and custom endpoints).
//! - `AnthropicAdapter`: Messages API.
//! - `GeminiAdapter`: generateContent API.
//! - `OllamaAdapter`: local `/api/chat` (NDJSON).
//! - `AzureOpenAiAdapter`: OpenAI shape with deployment URLs and the
//!   `api-key` header.
//!
//! AWS Bedrock and Vertex AI authenticate with cloud IAM signing, not
//! API keys; they are catalogued `Deferred` rather than faked (see the
//! catalog and the verification record).

pub mod anthropic;
pub mod azure;
pub mod gemini;
pub mod ollama;
pub mod openai;

use crate::config::ValidatedEndpoint;
use crate::error::{ProviderError, ProviderErrorCode};
use crate::http::HttpRequest;
use crate::redact::Redactor;
use crate::secrets::AuthMaterial;
use crate::types::{CompletionRequest, CompletionResponse, StreamEvent};
use std::time::Duration;

pub trait ProviderAdapter: Send + Sync {
    fn provider_id(&self) -> &str;

    /// Build one HTTP attempt. Pure: no I/O, no secret logging. The
    /// dispatcher has already run every gate before this is called.
    fn build_request(
        &self,
        endpoint: &ValidatedEndpoint,
        auth: &AuthMaterial,
        request: &CompletionRequest,
        stream: bool,
    ) -> Result<HttpRequest, ProviderError>;

    /// Parse a complete (non-streaming) response. Handles error
    /// statuses as well as success; provider text is excerpted through
    /// the redactor before it can enter an error message.
    fn parse_response(
        &self,
        redactor: &Redactor,
        status: u16,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<CompletionResponse, ProviderError>;

    fn new_stream_decoder(&self) -> Box<dyn StreamDecoder>;
}

pub trait StreamDecoder: Send {
    fn feed(&mut self, bytes: &[u8]) -> Result<Vec<StreamEvent>, ProviderError>;
    fn finish(&mut self) -> Result<Vec<StreamEvent>, ProviderError>;
}

/// Extract a human message from a provider error body, trying the
/// shapes the implemented families actually return. Always excerpted
/// and scrubbed by the caller via `redact`.
pub fn extract_error_message(body: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    let candidates = [
        value.pointer("/error/message").and_then(|v| v.as_str()),
        value.get("error").and_then(|v| v.as_str()),
        value.get("message").and_then(|v| v.as_str()),
        value.pointer("/error/status").and_then(|v| v.as_str()),
    ];
    candidates
        .into_iter()
        .flatten()
        .next()
        .map(|s| s.to_string())
}

/// Map an HTTP error status to the sanitized taxonomy, honouring
/// `Retry-After` (seconds) on 429.
pub fn map_error_status(
    redactor: &Redactor,
    status: u16,
    headers: &[(String, String)],
    body: &[u8],
) -> ProviderError {
    let provider_msg = extract_error_message(body)
        .map(|m| redactor.excerpt(&m, 240))
        .unwrap_or_default();
    let suffix = if provider_msg.is_empty() {
        String::new()
    } else {
        format!(": {provider_msg}")
    };
    let retry_after = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("retry-after"))
        .and_then(|(_, v)| v.trim().parse::<u64>().ok())
        .map(Duration::from_secs);
    let (code, label) = match status {
        401 | 403 => (ProviderErrorCode::AuthFailed, "authentication failed"),
        429 => (ProviderErrorCode::RateLimited, "rate limited"),
        500..=599 => (
            ProviderErrorCode::ProviderUnavailable,
            "provider unavailable",
        ),
        400..=499 => (
            ProviderErrorCode::MalformedResponse,
            "provider rejected the request",
        ),
        _ => (
            ProviderErrorCode::ProviderUnavailable,
            "unexpected provider status",
        ),
    };
    let mut err =
        ProviderError::new(code, format!("HTTP {status} {label}{suffix}")).with_status(status);
    if let Some(after) = retry_after {
        err = err.with_retry_after(after);
    }
    err
}

/// A redirect is never followed (endpoint policy, §71): credentials
/// must not be bounced to a host the user did not configure.
pub fn redirect_error(
    status: u16,
    headers: &[(String, String)],
    redactor: &Redactor,
) -> ProviderError {
    let location = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("location"))
        .map(|(_, v)| redactor.excerpt(v, 200))
        .unwrap_or_else(|| "(no Location header)".to_string());
    ProviderError::new(
        ProviderErrorCode::EndpointRejected,
        format!(
            "provider attempted an HTTP {status} redirect to {location}; redirects are not followed"
        ),
    )
    .with_status(status)
}

pub fn is_redirect(status: u16) -> bool {
    (300..400).contains(&status)
}

/// The API key from resolved auth material, for header-style auth.
pub fn api_key<'a>(auth: &'a AuthMaterial, provider: &str) -> Result<&'a str, ProviderError> {
    match auth {
        AuthMaterial::ApiKey(secret) | AuthMaterial::BearerToken(secret) => Ok(secret.expose()),
        AuthMaterial::None => Err(ProviderError::new(
            ProviderErrorCode::AuthFailed,
            format!("provider '{provider}' requires an API key but no credential was resolved"),
        )),
    }
}

/// `Authorization: Bearer <key>` header when auth material exists.
pub fn bearer_header(auth: &AuthMaterial) -> Option<(String, String)> {
    match auth {
        AuthMaterial::ApiKey(secret) | AuthMaterial::BearerToken(secret) => Some((
            "Authorization".to_string(),
            format!("Bearer {}", secret.expose()),
        )),
        AuthMaterial::None => None,
    }
}

pub fn json_request(
    endpoint: &ValidatedEndpoint,
    path: &str,
    headers: Vec<(String, String)>,
    body: serde_json::Value,
) -> HttpRequest {
    let mut headers = headers;
    headers.push(("Content-Type".to_string(), "application/json".to_string()));
    HttpRequest {
        method: "POST".to_string(),
        endpoint: endpoint.clone(),
        path: format!("{}{}", endpoint.base_path, path),
        headers,
        body: serde_json::to_vec(&body).unwrap_or_default(),
    }
}

pub fn messages_json(request: &CompletionRequest) -> serde_json::Value {
    serde_json::Value::Array(
        request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role.as_str(),
                    "content": m.content(),
                })
            })
            .collect(),
    )
}
