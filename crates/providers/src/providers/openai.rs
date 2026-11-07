//! OpenAI Chat Completions shape. This one adapter serves every §14
//! provider whose official API is OpenAI-compatible; the provider id
//! and base URL come from the catalog/config, never hardcoded per
//! provider inside the adapter.

use super::{
    ProviderAdapter, StreamDecoder, bearer_header, is_redirect, json_request, map_error_status,
    messages_json, redirect_error,
};
use crate::config::ValidatedEndpoint;
use crate::error::{ProviderError, ProviderErrorCode};
use crate::http::HttpRequest;
use crate::redact::Redactor;
use crate::secrets::AuthMaterial;
use crate::stream::SseParser;
use crate::types::{CompletionRequest, CompletionResponse, StreamEvent, Usage};

pub struct OpenAiCompatibleAdapter {
    provider: String,
}

impl OpenAiCompatibleAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
        }
    }
}

pub fn build_openai_body(request: &CompletionRequest, stream: bool) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": request.model,
        "messages": messages_json(request),
        "stream": stream,
    });
    if let Some(max) = request.max_output_tokens {
        body["max_tokens"] = serde_json::json!(max);
    }
    if let Some(temp) = request.temperature {
        body["temperature"] = serde_json::json!(temp);
    }
    if stream {
        body["stream_options"] = serde_json::json!({"include_usage": true});
    }
    body
}

pub fn parse_openai_response(
    provider: &str,
    redactor: &Redactor,
    status: u16,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<CompletionResponse, ProviderError> {
    if is_redirect(status) {
        return Err(redirect_error(status, headers, redactor));
    }
    if !(200..300).contains(&status) {
        return Err(map_error_status(redactor, status, headers, body));
    }
    let value: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
        ProviderError::new(
            ProviderErrorCode::MalformedResponse,
            format!("{provider} returned a non-JSON success body: {e}"),
        )
    })?;
    let text = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ProviderError::new(
                ProviderErrorCode::MalformedResponse,
                format!("{provider} response has no choices[0].message.content"),
            )
        })?
        .to_string();
    let usage = value.get("usage").map(|u| Usage {
        input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        output_tokens: u
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    });
    Ok(CompletionResponse {
        text,
        provider: provider.to_string(),
        model: value
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        usage,
        finish_reason: value
            .pointer("/choices/0/finish_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

impl ProviderAdapter for OpenAiCompatibleAdapter {
    fn provider_id(&self) -> &str {
        &self.provider
    }

    fn build_request(
        &self,
        endpoint: &ValidatedEndpoint,
        auth: &AuthMaterial,
        request: &CompletionRequest,
        stream: bool,
    ) -> Result<HttpRequest, ProviderError> {
        let mut headers = Vec::new();
        if let Some(h) = bearer_header(auth) {
            headers.push(h);
        }
        let mut body = build_openai_body(request, stream);
        if self.provider == "openrouter" && request.disable_open_router_reasoning {
            body["reasoning"] = serde_json::json!({"enabled": false});
            body["provider"] = serde_json::json!({"require_parameters": true});
        }
        Ok(json_request(endpoint, "/chat/completions", headers, body))
    }

    fn parse_response(
        &self,
        redactor: &Redactor,
        status: u16,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<CompletionResponse, ProviderError> {
        parse_openai_response(&self.provider, redactor, status, headers, body)
    }

    fn new_stream_decoder(&self) -> Box<dyn StreamDecoder> {
        Box::new(OpenAiStreamDecoder::default())
    }
}

#[derive(Default)]
pub struct OpenAiStreamDecoder {
    parser: SseParser,
    finish_reason: Option<String>,
    saw_done: bool,
}

impl OpenAiStreamDecoder {
    fn decode_data(&mut self, data: &str, out: &mut Vec<StreamEvent>) -> Result<(), ProviderError> {
        if data == "[DONE]" {
            self.saw_done = true;
            out.push(StreamEvent::Completed {
                finish_reason: self.finish_reason.take(),
            });
            return Ok(());
        }
        let value: serde_json::Value = serde_json::from_str(data).map_err(|e| {
            ProviderError::new(
                ProviderErrorCode::MalformedResponse,
                format!("malformed SSE JSON from provider: {e}"),
            )
        })?;
        if let Some(u) = value.get("usage").filter(|u| !u.is_null()) {
            out.push(StreamEvent::Usage(Usage {
                input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                output_tokens: u
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
            }));
        }
        if let Some(reason) = value
            .pointer("/choices/0/finish_reason")
            .and_then(|v| v.as_str())
        {
            self.finish_reason = Some(reason.to_string());
        }
        if let Some(text) = value
            .pointer("/choices/0/delta/content")
            .and_then(|v| v.as_str())
            && !text.is_empty()
        {
            out.push(StreamEvent::Token(text.to_string()));
        }
        Ok(())
    }
}

impl StreamDecoder for OpenAiStreamDecoder {
    fn feed(&mut self, bytes: &[u8]) -> Result<Vec<StreamEvent>, ProviderError> {
        let mut out = Vec::new();
        for event in self.parser.feed(bytes)? {
            self.decode_data(&event.data, &mut out)?;
        }
        Ok(out)
    }

    fn finish(&mut self) -> Result<Vec<StreamEvent>, ProviderError> {
        let mut out = Vec::new();
        for event in self.parser.finish() {
            self.decode_data(&event.data, &mut out)?;
        }
        if !self.saw_done {
            out.push(StreamEvent::Completed {
                finish_reason: self.finish_reason.take(),
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProviderConfig;
    use crate::types::{ChatMessage, Role};

    fn endpoint() -> ValidatedEndpoint {
        ProviderConfig::new("openai", "https://api.openai.com/v1")
            .validate()
            .unwrap()
    }

    #[test]
    fn builds_chat_completions_request() {
        let adapter = OpenAiCompatibleAdapter::new("openai");
        let req = CompletionRequest::new("gpt-x", vec![ChatMessage::new(Role::User, "hello")]);
        let auth = AuthMaterial::ApiKey(crate::secrets::SecretValue::new("sk-fake-1234567890"));
        let http = adapter
            .build_request(&endpoint(), &auth, &req, false)
            .unwrap();
        assert_eq!(http.path, "/v1/chat/completions");
        assert!(
            http.headers
                .iter()
                .any(|(k, v)| k == "Authorization" && v == "Bearer sk-fake-1234567890")
        );
        let body: serde_json::Value = serde_json::from_slice(&http.body).unwrap();
        assert_eq!(body["model"], "gpt-x");
        assert_eq!(body["messages"][0]["content"], "hello");
    }

    #[test]
    fn openrouter_reasoning_disable_is_explicit_and_provider_specific() {
        let mut request =
            CompletionRequest::new("model", vec![ChatMessage::new(Role::User, "hello")]);
        let openrouter = OpenAiCompatibleAdapter::new("openrouter");
        let openai = OpenAiCompatibleAdapter::new("openai");
        let auth = AuthMaterial::None;
        let body = |adapter: &OpenAiCompatibleAdapter, request: &CompletionRequest| {
            let http = adapter
                .build_request(&endpoint(), &auth, request, false)
                .unwrap();
            serde_json::from_slice::<serde_json::Value>(&http.body).unwrap()
        };
        assert!(body(&openrouter, &request).get("reasoning").is_none());
        request.disable_open_router_reasoning = true;
        let routed = body(&openrouter, &request);
        assert_eq!(routed["reasoning"]["enabled"], false);
        assert_eq!(routed["provider"]["require_parameters"], true);
        assert!(body(&openai, &request).get("reasoning").is_none());
    }

    #[test]
    fn parses_success_and_usage() {
        let adapter = OpenAiCompatibleAdapter::new("openai");
        let body = br#"{"model":"gpt-x","choices":[{"message":{"role":"assistant","content":"hi there"},"finish_reason":"stop"}],"usage":{"prompt_tokens":11,"completion_tokens":7}}"#;
        let resp = adapter
            .parse_response(&Redactor::new(), 200, &[], body)
            .unwrap();
        assert_eq!(resp.text, "hi there");
        assert_eq!(resp.usage.unwrap().total(), 18);
        assert_eq!(resp.finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn parses_error_without_leaking_key() {
        let adapter = OpenAiCompatibleAdapter::new("openai");
        let mut redactor = Redactor::new();
        redactor.register("sk-fake-1234567890");
        let body = br#"{"error":{"message":"invalid key sk-fake-1234567890 supplied"}}"#;
        let err = adapter
            .parse_response(&redactor, 401, &[], body)
            .unwrap_err();
        assert_eq!(err.code, ProviderErrorCode::AuthFailed);
        assert!(
            !err.message.contains("sk-fake-1234567890"),
            "{}",
            err.message
        );
    }

    #[test]
    fn stream_decoder_handles_split_events() {
        let mut dec = OpenAiStreamDecoder::default();
        let raw = "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
        let mut events = Vec::new();
        for chunk in raw.as_bytes().chunks(3) {
            events.extend(dec.feed(chunk).unwrap());
        }
        events.extend(dec.finish().unwrap());
        let text: String = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::Token(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "Hello");
        assert!(matches!(
            events.last(),
            Some(StreamEvent::Completed { finish_reason: Some(r) }) if r == "stop"
        ));
    }
}
