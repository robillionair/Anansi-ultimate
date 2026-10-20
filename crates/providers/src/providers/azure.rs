//! Azure OpenAI adapter (enterprise endpoint, MASTER_SPEC §14).
//! Differences from the public OpenAI API, per Microsoft's official
//! documentation: the URL carries the *deployment* name and an
//! `api-version` query parameter, and authentication is the `api-key`
//! header (or Entra ID bearer tokens, which are an integrator concern —
//! this adapter uses the API-key form only and says so).

use super::openai::{OpenAiStreamDecoder, build_openai_body, parse_openai_response};
use super::{ProviderAdapter, StreamDecoder, api_key, json_request};
use crate::config::ValidatedEndpoint;
use crate::error::ProviderError;
use crate::http::HttpRequest;
use crate::redact::Redactor;
use crate::secrets::AuthMaterial;
use crate::types::{CompletionRequest, CompletionResponse};

/// Default REST API version. Azure versions roll frequently; this is an
/// adapter field so the integrator can pin the version their resource
/// is verified against (see the verification record).
pub const DEFAULT_AZURE_API_VERSION: &str = "2024-10-21";

pub struct AzureOpenAiAdapter {
    provider: String,
    pub api_version: String,
}

impl AzureOpenAiAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            api_version: DEFAULT_AZURE_API_VERSION.to_string(),
        }
    }
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }
}

impl ProviderAdapter for AzureOpenAiAdapter {
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
        // On Azure, the request "model" is the deployment name.
        let mut body = build_openai_body(request, stream);
        if let Some(obj) = body.as_object_mut() {
            obj.remove("model");
        }
        let path = format!(
            "/openai/deployments/{}/chat/completions?api-version={}",
            request.model, self.api_version
        );
        Ok(json_request(
            endpoint,
            &path,
            vec![("api-key".to_string(), key.to_string())],
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
        parse_openai_response(&self.provider, redactor, status, headers, body)
    }

    fn new_stream_decoder(&self) -> Box<dyn StreamDecoder> {
        Box::new(OpenAiStreamDecoder::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProviderConfig;
    use crate::secrets::SecretValue;
    use crate::types::{ChatMessage, Role};

    #[test]
    fn builds_deployment_url_with_api_key_header() {
        let adapter = AzureOpenAiAdapter::new("azure");
        let endpoint = ProviderConfig::new("azure", "https://my-resource.openai.azure.com")
            .validate()
            .unwrap();
        let req =
            CompletionRequest::new("my-deployment", vec![ChatMessage::new(Role::User, "hello")]);
        let auth = AuthMaterial::ApiKey(SecretValue::new("azure-fake-key-123456"));
        let http = adapter
            .build_request(&endpoint, &auth, &req, false)
            .unwrap();
        assert_eq!(
            http.path,
            "/openai/deployments/my-deployment/chat/completions?api-version=2024-10-21"
        );
        assert!(http.headers.iter().any(|(k, _)| k == "api-key"));
        assert!(http.headers.iter().all(|(k, _)| k != "Authorization"));
        let body: serde_json::Value = serde_json::from_slice(&http.body).unwrap();
        assert!(body.get("model").is_none());
    }
}
