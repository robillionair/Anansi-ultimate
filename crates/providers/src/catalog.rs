//! Provider catalog: the registry MASTER_SPEC §82 names as an M5
//! deliverable. Every provider §14 lists as a *possible connection* is
//! present with an explicit status. Nothing here claims a provider is
//! live-verified: statuses distinguish implemented adapters, the
//! OpenAI-compatible family, deferred IAM-signed clouds, and
//! catalog-only entries. Auth methods reflect the official provider
//! documentation recorded in
//! `docs/handoffs/M5-provider-verification.md` (verified 2026-10-20).

use crate::gates::TargetLocality;
use crate::providers::ProviderAdapter;
use crate::providers::anthropic::AnthropicAdapter;
use crate::providers::azure::AzureOpenAiAdapter;
use crate::providers::gemini::GeminiAdapter;
use crate::providers::ollama::OllamaAdapter;
use crate::providers::openai::OpenAiCompatibleAdapter;
use crate::routing::ProviderRegistration;
use crate::secrets::AuthMethod;
use crate::types::ModelInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    OpenAiCompatible,
    Anthropic,
    Gemini,
    Ollama,
    AzureOpenAi,
    /// Cloud IAM signing (SigV4 / Google ADC) — separate mechanism.
    CloudIam,
    CustomOpenAiCompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterStatus {
    /// A dedicated adapter in this crate.
    Implemented,
    /// Served by the OpenAI-compatible adapter against the provider's
    /// official compatible endpoint.
    CompatibleFamily,
    /// Not implemented, with the recorded reason. Never presented as
    /// working.
    Deferred(&'static str),
    /// Listed for completeness; usable only as a custom endpoint once
    /// account-specific values are supplied.
    CatalogOnly(&'static str),
}

#[derive(Debug, Clone)]
pub struct ProviderEntry {
    pub id: &'static str,
    pub display_name: &'static str,
    pub kind: AdapterKind,
    /// Official base URL; `None` where the endpoint is per-account /
    /// per-resource / per-region and must come from configuration.
    pub base_url: Option<&'static str>,
    pub auth: &'static [AuthMethod],
    pub locality: TargetLocality,
    pub status: AdapterStatus,
    pub docs_url: &'static str,
}

const KEY: &[AuthMethod] = &[AuthMethod::ApiKey];
const NONE: &[AuthMethod] = &[AuthMethod::None];
const IAM: &[AuthMethod] = &[AuthMethod::CloudIam];

fn cloud(
    id: &'static str,
    display_name: &'static str,
    kind: AdapterKind,
    base_url: Option<&'static str>,
    auth: &'static [AuthMethod],
    status: AdapterStatus,
    docs_url: &'static str,
) -> ProviderEntry {
    ProviderEntry {
        id,
        display_name,
        kind,
        base_url,
        auth,
        locality: TargetLocality::Cloud,
        status,
        docs_url,
    }
}

pub fn catalog() -> Vec<ProviderEntry> {
    use AdapterStatus::*;
    vec![
        // --- Local runtimes (§§14, 18) ---
        ProviderEntry {
            id: "ollama",
            display_name: "Ollama (local)",
            kind: AdapterKind::Ollama,
            base_url: Some("http://127.0.0.1:11434"),
            auth: NONE,
            locality: TargetLocality::Local,
            status: Implemented,
            docs_url: "https://github.com/ollama/ollama/blob/main/docs/api.md",
        },
        ProviderEntry {
            id: "lmstudio",
            display_name: "LM Studio (local)",
            kind: AdapterKind::OpenAiCompatible,
            base_url: Some("http://127.0.0.1:1234/v1"),
            auth: NONE,
            locality: TargetLocality::Local,
            status: CompatibleFamily,
            docs_url: "https://lmstudio.ai/docs/developer/openai-compat",
        },
        // --- Dedicated adapters ---
        cloud(
            "openai",
            "OpenAI",
            AdapterKind::OpenAiCompatible,
            Some("https://api.openai.com/v1"),
            KEY,
            Implemented,
            "https://platform.openai.com/docs/api-reference/chat",
        ),
        cloud(
            "anthropic",
            "Anthropic",
            AdapterKind::Anthropic,
            Some("https://api.anthropic.com"),
            KEY,
            Implemented,
            "https://docs.claude.com/en/api/messages",
        ),
        cloud(
            "gemini",
            "Google Gemini",
            AdapterKind::Gemini,
            Some("https://generativelanguage.googleapis.com/v1beta"),
            KEY,
            Implemented,
            "https://ai.google.dev/gemini-api/docs/text-generation",
        ),
        cloud(
            "azure",
            "Azure OpenAI",
            AdapterKind::AzureOpenAi,
            None,
            KEY,
            Implemented,
            "https://learn.microsoft.com/en-us/azure/ai-foundry/openai/reference",
        ),
        // --- OpenAI-compatible family (§14) ---
        cloud(
            "openrouter",
            "OpenRouter",
            AdapterKind::OpenAiCompatible,
            Some("https://openrouter.ai/api/v1"),
            KEY,
            CompatibleFamily,
            "https://openrouter.ai/docs/api-reference/overview",
        ),
        cloud(
            "mistral",
            "Mistral AI",
            AdapterKind::OpenAiCompatible,
            Some("https://api.mistral.ai/v1"),
            KEY,
            CompatibleFamily,
            "https://docs.mistral.ai/api/",
        ),
        cloud(
            "xai",
            "xAI",
            AdapterKind::OpenAiCompatible,
            Some("https://api.x.ai/v1"),
            KEY,
            CompatibleFamily,
            "https://docs.x.ai/docs/api-reference",
        ),
        cloud(
            "moonshot",
            "Kimi / Moonshot",
            AdapterKind::OpenAiCompatible,
            Some("https://api.moonshot.ai/v1"),
            KEY,
            CompatibleFamily,
            "https://platform.moonshot.ai/docs",
        ),
        cloud(
            "nvidia-nim",
            "NVIDIA NIM",
            AdapterKind::OpenAiCompatible,
            Some("https://integrate.api.nvidia.com/v1"),
            KEY,
            CompatibleFamily,
            "https://docs.api.nvidia.com/nim/reference/llm-apis",
        ),
        cloud(
            "groq",
            "Groq",
            AdapterKind::OpenAiCompatible,
            Some("https://api.groq.com/openai/v1"),
            KEY,
            CompatibleFamily,
            "https://console.groq.com/docs/openai",
        ),
        cloud(
            "cerebras",
            "Cerebras",
            AdapterKind::OpenAiCompatible,
            Some("https://api.cerebras.ai/v1"),
            KEY,
            CompatibleFamily,
            "https://inference-docs.cerebras.ai/resources/openai",
        ),
        cloud(
            "cohere",
            "Cohere (compatibility API)",
            AdapterKind::OpenAiCompatible,
            Some("https://api.cohere.ai/compatibility/v1"),
            KEY,
            CompatibleFamily,
            "https://docs.cohere.com/docs/openai-compatibility",
        ),
        cloud(
            "together",
            "Together AI",
            AdapterKind::OpenAiCompatible,
            Some("https://api.together.xyz/v1"),
            KEY,
            CompatibleFamily,
            "https://docs.together.ai/docs/openai-api-compatibility",
        ),
        cloud(
            "fireworks",
            "Fireworks AI",
            AdapterKind::OpenAiCompatible,
            Some("https://api.fireworks.ai/inference/v1"),
            KEY,
            CompatibleFamily,
            "https://docs.fireworks.ai/tools-sdks/openai-compatibility",
        ),
        cloud(
            "perplexity",
            "Perplexity",
            AdapterKind::OpenAiCompatible,
            Some("https://api.perplexity.ai"),
            KEY,
            CompatibleFamily,
            "https://docs.perplexity.ai/api-reference/chat-completions-post",
        ),
        cloud(
            "huggingface",
            "Hugging Face (router)",
            AdapterKind::OpenAiCompatible,
            Some("https://router.huggingface.co/v1"),
            KEY,
            CompatibleFamily,
            "https://huggingface.co/docs/inference-providers/index",
        ),
        cloud(
            "zai",
            "Z.AI",
            AdapterKind::OpenAiCompatible,
            Some("https://api.z.ai/api/paas/v4"),
            KEY,
            CompatibleFamily,
            "https://docs.z.ai/guides/overview/quick-start",
        ),
        // --- Account-scoped / IAM-signed clouds ---
        cloud(
            "cloudflare",
            "Cloudflare Workers AI",
            AdapterKind::OpenAiCompatible,
            None,
            KEY,
            CatalogOnly(
                "endpoint embeds the account id: https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1 — configure as a custom endpoint",
            ),
            "https://developers.cloudflare.com/workers-ai/configuration/openai-compatibility/",
        ),
        cloud(
            "bedrock",
            "AWS Bedrock",
            AdapterKind::CloudIam,
            None,
            IAM,
            Deferred(
                "requires AWS Signature Version 4 signing of every request; an API-key adapter would be a fabrication",
            ),
            "https://docs.aws.amazon.com/bedrock/latest/userguide/api-reference.html",
        ),
        cloud(
            "vertex",
            "Google Vertex AI",
            AdapterKind::CloudIam,
            None,
            IAM,
            Deferred(
                "requires Google ADC / service-account OAuth2 access tokens; distinct from the Gemini API key path",
            ),
            "https://cloud.google.com/vertex-ai/generative-ai/docs/model-reference/inference",
        ),
        // --- Custom endpoints (§14) ---
        ProviderEntry {
            id: "custom-openai",
            display_name: "Custom OpenAI-compatible endpoint",
            kind: AdapterKind::CustomOpenAiCompatible,
            base_url: None,
            auth: KEY,
            locality: TargetLocality::Cloud,
            status: Implemented,
            docs_url: "https://platform.openai.com/docs/api-reference/chat",
        },
    ]
}

pub fn entry(id: &str) -> Option<ProviderEntry> {
    catalog().into_iter().find(|e| e.id == id)
}

/// Construct the adapter for a catalog entry, where one exists.
pub fn adapter_for(entry: &ProviderEntry) -> Option<Box<dyn ProviderAdapter>> {
    match entry.status {
        AdapterStatus::Deferred(_) | AdapterStatus::CatalogOnly(_) => None,
        _ => match entry.kind {
            AdapterKind::OpenAiCompatible | AdapterKind::CustomOpenAiCompatible => {
                Some(Box::new(OpenAiCompatibleAdapter::new(entry.id)))
            }
            AdapterKind::Anthropic => Some(Box::new(AnthropicAdapter::new(entry.id))),
            AdapterKind::Gemini => Some(Box::new(GeminiAdapter::new(entry.id))),
            AdapterKind::Ollama => Some(Box::new(OllamaAdapter::new(entry.id))),
            AdapterKind::AzureOpenAi => Some(Box::new(AzureOpenAiAdapter::new(entry.id))),
            AdapterKind::CloudIam => None,
        },
    }
}

/// One prototype adapter per dispatchable catalog provider, ready for
/// `Dispatcher::new`. Custom provider ids not in the catalog fall back
/// to the OpenAI-compatible adapter at registration time in integrator
/// code (documented in the handoff).
pub fn default_adapters() -> Vec<Box<dyn ProviderAdapter>> {
    catalog().iter().filter_map(adapter_for).collect()
}

/// Build a routing registration from a catalog entry, caller-supplied
/// models and an optional base-URL override (required where the entry
/// has no fixed base URL, e.g. Azure resources).
pub fn registration(
    entry: &ProviderEntry,
    base_url_override: Option<String>,
    credential: Option<crate::secrets::CredentialRef>,
    models: Vec<ModelInfo>,
) -> Option<ProviderRegistration> {
    let base_url = base_url_override.or_else(|| entry.base_url.map(|s| s.to_string()))?;
    let mut config = crate::config::ProviderConfig::new(entry.id, base_url);
    config.credential = credential;
    config.custom_endpoint = entry.kind == AdapterKind::CustomOpenAiCompatible;
    Some(ProviderRegistration {
        config,
        locality: entry.locality,
        models,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_spec_named_provider_is_present() {
        // MASTER_SPEC §14 "Possible connections", verbatim names mapped
        // to catalog ids.
        let ids: Vec<&str> = catalog().iter().map(|e| e.id).collect();
        for expected in [
            "openrouter",
            "openai",
            "gemini",
            "huggingface",
            "mistral",
            "anthropic",
            "xai",
            "moonshot",
            "nvidia-nim",
            "groq",
            "cerebras",
            "cohere",
            "together",
            "fireworks",
            "perplexity",
            "zai",
            "cloudflare",
            "azure",
            "bedrock",
            "vertex",
            "ollama",
            "lmstudio",
            "custom-openai",
        ] {
            assert!(ids.contains(&expected), "missing provider {expected}");
        }
    }

    #[test]
    fn iam_clouds_are_deferred_not_faked() {
        for id in ["bedrock", "vertex"] {
            let e = entry(id).unwrap();
            assert!(matches!(e.status, AdapterStatus::Deferred(_)), "{id}");
            assert_eq!(e.auth, IAM);
            assert!(adapter_for(&e).is_none());
        }
    }

    #[test]
    fn no_provider_claims_oauth_without_basis() {
        // Catalog policy: OAuthUser appears only where the verification
        // record documents official user OAuth for inference. At
        // verification time (2026-10-20) that set is empty for the
        // implemented API surfaces.
        for e in catalog() {
            assert!(
                !e.auth.contains(&AuthMethod::OAuthUser),
                "{} claims OAuthUser without a verification basis",
                e.id
            );
        }
    }
}
