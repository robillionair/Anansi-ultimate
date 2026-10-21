//! Anthropic Messages API adapter. Auth is the `x-api-key` header plus
//! the required `anthropic-version` header — API keys, not OAuth
//! (see the verification record).

use super::{
    ProviderAdapter, StreamDecoder, api_key, is_redirect, json_request, map_error_status,
    redirect_error,
};
use crate::config::ValidatedEndpoint;
use crate::error::{ProviderError, ProviderErrorCode};
use crate::http::HttpRequest;
use crate::redact::Redactor;
use crate::secrets::AuthMaterial;
use crate::stream::SseParser;
use crate::types::{CompletionRequest, CompletionResponse, Role, StreamEvent, Usage};

pub const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Anthropic requires `max_tokens`; when the caller sets none, this
/// bounded default applies (documented adapter policy, not a spec value).
pub const DEFAULT_MAX_TOKENS: u32 = 1024;

pub struct AnthropicAdapter {
    provider: String,
}

impl AnthropicAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
        }
    }
}

impl ProviderAdapter for AnthropicAdapter {
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
        let key = api_key(auth, &self.provider)?;
        // Anthropic takes system text as a top-level field, not a
        // system-role message.
        let mut system = String::new();
        let mut messages = Vec::new();
        for m in &request.messages {
            if m.role == Role::System {
                if !system.is_empty() {
                    system.push('\n');
                }
                system.push_str(&m.content());
            } else {
                messages.push(serde_json::json!({
                    "role": m.role.as_str(),
                    "content": m.content(),
                }));
            }
        }
        let mut body = serde_json::json!({
            "model": request.model,
            "max_tokens": request.max_output_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            "messages": messages,
            "stream": stream,
        });
        if !system.is_empty() {
            body["system"] = serde_json::json!(system);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        Ok(json_request(
            endpoint,
            "/v1/messages",
            vec![
                ("x-api-key".to_string(), key.to_string()),
                (
                    "anthropic-version".to_string(),
                    ANTHROPIC_VERSION.to_string(),
                ),
            ],
            body,
        ))
    }

    fn parse_response(
        &self,
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
                format!("anthropic returned a non-JSON success body: {e}"),
            )
        })?;
        let mut text = String::new();
        if let Some(blocks) = value.get("content").and_then(|v| v.as_array()) {
            for block in blocks {
                if block.get("type").and_then(|v| v.as_str()) == Some("text")
                    && let Some(t) = block.get("text").and_then(|v| v.as_str())
                {
                    text.push_str(t);
                }
            }
        }
        if text.is_empty() {
            return Err(ProviderError::new(
                ProviderErrorCode::MalformedResponse,
                "anthropic response has no text content blocks",
            ));
        }
        Ok(CompletionResponse {
            text,
            provider: self.provider.clone(),
            model: value
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            usage: value.get("usage").map(|u| Usage {
                input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            }),
            finish_reason: value
                .get("stop_reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    fn new_stream_decoder(&self) -> Box<dyn StreamDecoder> {
        Box::new(AnthropicStreamDecoder::default())
    }
}

#[derive(Default)]
pub struct AnthropicStreamDecoder {
    parser: SseParser,
    input_tokens: u64,
    output_tokens: u64,
    finish_reason: Option<String>,
    completed: bool,
}

impl AnthropicStreamDecoder {
    fn decode_event(
        &mut self,
        event: &crate::stream::SseEvent,
        out: &mut Vec<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let kind = event.event.as_deref().unwrap_or("");
        let value: serde_json::Value = serde_json::from_str(&event.data).map_err(|e| {
            ProviderError::new(
                ProviderErrorCode::MalformedResponse,
                format!("malformed Anthropic stream JSON: {e}"),
            )
        })?;
        match kind {
            "message_start" => {
                self.input_tokens = value
                    .pointer("/message/usage/input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }
            "content_block_delta" => {
                if let Some(t) = value.pointer("/delta/text").and_then(|v| v.as_str())
                    && !t.is_empty()
                {
                    out.push(StreamEvent::Token(t.to_string()));
                }
            }
            "message_delta" => {
                if let Some(o) = value
                    .pointer("/usage/output_tokens")
                    .and_then(|v| v.as_u64())
                {
                    self.output_tokens = o;
                }
                if let Some(r) = value.pointer("/delta/stop_reason").and_then(|v| v.as_str()) {
                    self.finish_reason = Some(r.to_string());
                }
            }
            "message_stop" => {
                out.push(StreamEvent::Usage(Usage {
                    input_tokens: self.input_tokens,
                    output_tokens: self.output_tokens,
                }));
                out.push(StreamEvent::Completed {
                    finish_reason: self.finish_reason.take(),
                });
                self.completed = true;
            }
            "error" => {
                let msg = value
                    .pointer("/error/message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("stream error");
                return Err(ProviderError::new(
                    ProviderErrorCode::ProviderUnavailable,
                    format!("anthropic stream error: {msg}"),
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

impl StreamDecoder for AnthropicStreamDecoder {
    fn feed(&mut self, bytes: &[u8]) -> Result<Vec<StreamEvent>, ProviderError> {
        let mut out = Vec::new();
        for event in self.parser.feed(bytes)? {
            self.decode_event(&event, &mut out)?;
        }
        Ok(out)
    }

    fn finish(&mut self) -> Result<Vec<StreamEvent>, ProviderError> {
        let mut out = Vec::new();
        for event in self.parser.finish() {
            self.decode_event(&event, &mut out)?;
        }
        if !self.completed {
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
    use crate::secrets::SecretValue;
    use crate::types::ChatMessage;

    fn endpoint() -> ValidatedEndpoint {
        ProviderConfig::new("anthropic", "https://api.anthropic.com")
            .validate()
            .unwrap()
    }

    #[test]
    fn builds_messages_request_with_system_field() {
        let adapter = AnthropicAdapter::new("anthropic");
        let req = CompletionRequest::new(
            "claude-x",
            vec![
                ChatMessage::new(Role::System, "be terse"),
                ChatMessage::new(Role::User, "hello"),
            ],
        );
        let auth = AuthMaterial::ApiKey(SecretValue::new("sk-ant-fake-123456"));
        let http = adapter
            .build_request(&endpoint(), &auth, &req, false)
            .unwrap();
        assert_eq!(http.path, "/v1/messages");
        assert!(http.headers.iter().any(|(k, _)| k == "x-api-key"));
        assert!(
            http.headers
                .iter()
                .any(|(k, v)| k == "anthropic-version" && v == "2023-06-01")
        );
        let body: serde_json::Value = serde_json::from_slice(&http.body).unwrap();
        assert_eq!(body["system"], "be terse");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["max_tokens"], DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn parses_text_blocks_and_usage() {
        let adapter = AnthropicAdapter::new("anthropic");
        let body = br#"{"model":"claude-x","content":[{"type":"text","text":"Hello "},{"type":"text","text":"world"}],"stop_reason":"end_turn","usage":{"input_tokens":10,"output_tokens":4}}"#;
        let resp = adapter
            .parse_response(&Redactor::new(), 200, &[], body)
            .unwrap();
        assert_eq!(resp.text, "Hello world");
        assert_eq!(resp.usage.unwrap().total(), 14);
    }

    #[test]
    fn stream_decoder_emits_tokens_usage_completion() {
        let mut dec = AnthropicStreamDecoder::default();
        let raw = "event: message_start\ndata: {\"message\":{\"usage\":{\"input_tokens\":12}}}\n\nevent: content_block_delta\ndata: {\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\nevent: message_delta\ndata: {\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":3}}\n\nevent: message_stop\ndata: {}\n\n";
        let mut events = Vec::new();
        for chunk in raw.as_bytes().chunks(5) {
            events.extend(dec.feed(chunk).unwrap());
        }
        events.extend(dec.finish().unwrap());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StreamEvent::Token(t) if t == "Hi"))
        );
        assert!(events.iter().any(
            |e| matches!(e, StreamEvent::Usage(u) if u.input_tokens == 12 && u.output_tokens == 3)
        ));
        assert!(matches!(events.last(), Some(StreamEvent::Completed { .. })));
    }
}
