//! Google Gemini `generateContent` adapter. Auth uses the
//! `x-goog-api-key` header (the API key is not placed in a URL query
//! string — endpoint policy in this crate forbids secrets in URLs).

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

pub struct GeminiAdapter {
    provider: String,
}

impl GeminiAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
        }
    }

    fn parse_payload(
        &self,
        value: &serde_json::Value,
    ) -> Result<(String, Option<Usage>, Option<String>), ProviderError> {
        let mut text = String::new();
        if let Some(parts) = value
            .pointer("/candidates/0/content/parts")
            .and_then(|v| v.as_array())
        {
            for part in parts {
                if let Some(t) = part.get("text").and_then(|v| v.as_str()) {
                    text.push_str(t);
                }
            }
        }
        if text.is_empty() {
            return Err(ProviderError::new(
                ProviderErrorCode::MalformedResponse,
                "gemini response has no candidates[0].content.parts text",
            ));
        }
        let usage = value.get("usageMetadata").map(|u| Usage {
            input_tokens: u
                .get("promptTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            output_tokens: u
                .get("candidatesTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
        });
        let finish = value
            .pointer("/candidates/0/finishReason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok((text, usage, finish))
    }
}

impl ProviderAdapter for GeminiAdapter {
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
        let mut system = String::new();
        let mut contents = Vec::new();
        for m in &request.messages {
            match m.role {
                Role::System => {
                    if !system.is_empty() {
                        system.push('\n');
                    }
                    system.push_str(&m.content());
                }
                Role::User => contents.push(serde_json::json!({
                    "role": "user",
                    "parts": [{"text": m.content()}],
                })),
                Role::Assistant => contents.push(serde_json::json!({
                    "role": "model",
                    "parts": [{"text": m.content()}],
                })),
            }
        }
        let mut body = serde_json::json!({ "contents": contents });
        if !system.is_empty() {
            body["systemInstruction"] = serde_json::json!({"parts": [{"text": system}]});
        }
        let mut generation = serde_json::json!({});
        if let Some(max) = request.max_output_tokens {
            generation["maxOutputTokens"] = serde_json::json!(max);
        }
        if let Some(temp) = request.temperature {
            generation["temperature"] = serde_json::json!(temp);
        }
        if generation.as_object().is_some_and(|o| !o.is_empty()) {
            body["generationConfig"] = generation;
        }
        let action = if stream {
            "streamGenerateContent?alt=sse"
        } else {
            "generateContent"
        };
        Ok(json_request(
            endpoint,
            &format!("/models/{}:{}", request.model, action),
            vec![("x-goog-api-key".to_string(), key.to_string())],
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
                format!("gemini returned a non-JSON success body: {e}"),
            )
        })?;
        let (text, usage, finish) = self.parse_payload(&value)?;
        Ok(CompletionResponse {
            text,
            provider: self.provider.clone(),
            model: String::new(),
            usage,
            finish_reason: finish,
        })
    }

    fn new_stream_decoder(&self) -> Box<dyn StreamDecoder> {
        Box::new(GeminiStreamDecoder {
            parser: SseParser::new(),
            adapter: GeminiAdapter::new(self.provider.clone()),
            completed: false,
        })
    }
}

pub struct GeminiStreamDecoder {
    parser: SseParser,
    adapter: GeminiAdapter,
    completed: bool,
}

impl GeminiStreamDecoder {
    fn decode_data(&mut self, data: &str, out: &mut Vec<StreamEvent>) -> Result<(), ProviderError> {
        let value: serde_json::Value = serde_json::from_str(data).map_err(|e| {
            ProviderError::new(
                ProviderErrorCode::MalformedResponse,
                format!("malformed Gemini stream JSON: {e}"),
            )
        })?;
        let (text, usage, finish) = self.adapter.parse_payload(&value)?;
        out.push(StreamEvent::Token(text));
        if let Some(u) = usage {
            out.push(StreamEvent::Usage(u));
        }
        if let Some(f) = finish {
            out.push(StreamEvent::Completed {
                finish_reason: Some(f),
            });
            self.completed = true;
        }
        Ok(())
    }
}

impl StreamDecoder for GeminiStreamDecoder {
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
        if !self.completed {
            out.push(StreamEvent::Completed {
                finish_reason: None,
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
        ProviderConfig::new("gemini", "https://generativelanguage.googleapis.com/v1beta")
            .validate()
            .unwrap()
    }

    #[test]
    fn builds_generate_content_request() {
        let adapter = GeminiAdapter::new("gemini");
        let req = CompletionRequest::new(
            "gemini-x",
            vec![
                ChatMessage::new(Role::System, "sys"),
                ChatMessage::new(Role::User, "hello"),
                ChatMessage::new(Role::Assistant, "hi"),
            ],
        );
        let auth = AuthMaterial::ApiKey(SecretValue::new("AIza-fake-key-123456"));
        let http = adapter
            .build_request(&endpoint(), &auth, &req, true)
            .unwrap();
        assert_eq!(
            http.path,
            "/v1beta/models/gemini-x:streamGenerateContent?alt=sse"
        );
        assert!(http.headers.iter().any(|(k, _)| k == "x-goog-api-key"));
        assert!(!http.path.contains("AIza"), "key must not be in the URL");
        let body: serde_json::Value = serde_json::from_slice(&http.body).unwrap();
        assert_eq!(body["contents"][1]["role"], "model");
        assert_eq!(body["systemInstruction"]["parts"][0]["text"], "sys");
    }

    #[test]
    fn parses_response() {
        let adapter = GeminiAdapter::new("gemini");
        let body = br#"{"candidates":[{"content":{"parts":[{"text":"Hello "},{"text":"there"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":9,"candidatesTokenCount":5}}"#;
        let resp = adapter
            .parse_response(&Redactor::new(), 200, &[], body)
            .unwrap();
        assert_eq!(resp.text, "Hello there");
        assert_eq!(resp.usage.unwrap().total(), 14);
        assert_eq!(resp.finish_reason.as_deref(), Some("STOP"));
    }
}
