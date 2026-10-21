//! Ollama local runtime adapter (`/api/chat`, NDJSON streaming).
//! Loopback only in practice: endpoint policy allows its plaintext
//! HTTP because the host is loopback, and it carries no credential.
//! Mirrors the shape `anansi-core` already uses for local Ask, in the
//! provider-adapter form M5 requires.

use super::{
    ProviderAdapter, StreamDecoder, is_redirect, json_request, map_error_status, messages_json,
    redirect_error,
};
use crate::config::ValidatedEndpoint;
use crate::error::{ProviderError, ProviderErrorCode};
use crate::http::HttpRequest;
use crate::redact::Redactor;
use crate::secrets::AuthMaterial;
use crate::stream::NdjsonParser;
use crate::types::{CompletionRequest, CompletionResponse, StreamEvent, Usage};

pub struct OllamaAdapter {
    provider: String,
}

impl OllamaAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
        }
    }

    fn parse_payload(
        &self,
        value: &serde_json::Value,
    ) -> Result<(String, Option<Usage>, Option<String>, bool), ProviderError> {
        let text = value
            .pointer("/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProviderError::new(
                    ProviderErrorCode::MalformedResponse,
                    "ollama response has no message.content",
                )
            })?
            .to_string();
        let done = value.get("done").and_then(|v| v.as_bool()).unwrap_or(false);
        let usage = if done {
            Some(Usage {
                input_tokens: value
                    .get("prompt_eval_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                output_tokens: value
                    .get("eval_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
            })
        } else {
            None
        };
        let finish = value
            .get("done_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok((text, usage, finish, done))
    }
}

impl ProviderAdapter for OllamaAdapter {
    fn provider_id(&self) -> &str {
        &self.provider
    }

    fn build_request(
        &self,
        endpoint: &ValidatedEndpoint,
        _auth: &AuthMaterial,
        request: &CompletionRequest,
        stream: bool,
    ) -> Result<HttpRequest, ProviderError> {
        let mut options = serde_json::json!({});
        if let Some(temp) = request.temperature {
            options["temperature"] = serde_json::json!(temp);
        }
        if let Some(max) = request.max_output_tokens {
            options["num_predict"] = serde_json::json!(max);
        }
        let body = serde_json::json!({
            "model": request.model,
            "messages": messages_json(request),
            "stream": stream,
            "options": options,
        });
        Ok(json_request(endpoint, "/api/chat", Vec::new(), body))
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
                format!("ollama returned a non-JSON success body: {e}"),
            )
        })?;
        let (text, usage, finish, _done) = self.parse_payload(&value)?;
        Ok(CompletionResponse {
            text,
            provider: self.provider.clone(),
            model: value
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            usage,
            finish_reason: finish,
        })
    }

    fn new_stream_decoder(&self) -> Box<dyn StreamDecoder> {
        Box::new(OllamaStreamDecoder {
            parser: NdjsonParser::new(),
            adapter: OllamaAdapter::new(self.provider.clone()),
            completed: false,
        })
    }
}

pub struct OllamaStreamDecoder {
    parser: NdjsonParser,
    adapter: OllamaAdapter,
    completed: bool,
}

impl OllamaStreamDecoder {
    fn decode_line(&mut self, line: &str, out: &mut Vec<StreamEvent>) -> Result<(), ProviderError> {
        let value: serde_json::Value = serde_json::from_str(line).map_err(|e| {
            ProviderError::new(
                ProviderErrorCode::MalformedResponse,
                format!("malformed Ollama stream JSON: {e}"),
            )
        })?;
        let (text, usage, finish, done) = self.adapter.parse_payload(&value)?;
        if !text.is_empty() {
            out.push(StreamEvent::Token(text));
        }
        if done {
            if let Some(u) = usage {
                out.push(StreamEvent::Usage(u));
            }
            out.push(StreamEvent::Completed {
                finish_reason: finish,
            });
            self.completed = true;
        }
        Ok(())
    }
}

impl StreamDecoder for OllamaStreamDecoder {
    fn feed(&mut self, bytes: &[u8]) -> Result<Vec<StreamEvent>, ProviderError> {
        let mut out = Vec::new();
        for line in self.parser.feed(bytes)? {
            self.decode_line(&line, &mut out)?;
        }
        Ok(out)
    }

    fn finish(&mut self) -> Result<Vec<StreamEvent>, ProviderError> {
        let mut out = Vec::new();
        for line in self.parser.finish() {
            self.decode_line(&line, &mut out)?;
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
    use crate::types::ChatMessage;

    fn endpoint() -> ValidatedEndpoint {
        ProviderConfig::new("ollama", "http://127.0.0.1:11434")
            .validate()
            .unwrap()
    }

    #[test]
    fn builds_chat_request_without_auth() {
        let adapter = OllamaAdapter::new("ollama");
        let req = CompletionRequest::new(
            "qwen2.5:0.5b",
            vec![ChatMessage::new(crate::types::Role::User, "hello")],
        );
        let http = adapter
            .build_request(&endpoint(), &AuthMaterial::None, &req, false)
            .unwrap();
        assert_eq!(http.path, "/api/chat");
        assert!(http.headers.iter().all(|(k, _)| k != "Authorization"));
    }

    #[test]
    fn parses_response_and_stream() {
        let adapter = OllamaAdapter::new("ollama");
        let body = br#"{"model":"qwen2.5:0.5b","message":{"role":"assistant","content":"hi"},"done":true,"done_reason":"stop","prompt_eval_count":8,"eval_count":3}"#;
        let resp = adapter
            .parse_response(&Redactor::new(), 200, &[], body)
            .unwrap();
        assert_eq!(resp.text, "hi");
        assert_eq!(resp.usage.unwrap().total(), 11);

        let mut dec = OllamaStreamDecoder {
            parser: NdjsonParser::new(),
            adapter: OllamaAdapter::new("ollama"),
            completed: false,
        };
        let raw = "{\"message\":{\"content\":\"Hel\"},\"done\":false}\n{\"message\":{\"content\":\"lo\"},\"done\":true,\"done_reason\":\"stop\",\"prompt_eval_count\":8,\"eval_count\":2}\n";
        let mut events = Vec::new();
        for chunk in raw.as_bytes().chunks(4) {
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
        assert!(matches!(events.last(), Some(StreamEvent::Completed { .. })));
    }
}
