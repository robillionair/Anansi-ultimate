//! Shared provider types. Segment-level privacy classification is the
//! contract the Context Compiler (MASTER_SPEC §19) hands to the dispatcher:
//! the dispatcher enforces it, it never re-classifies content.

use serde::{Deserialize, Serialize};

/// Privacy classification of one content segment. Mirrors the classes in
/// `anansi-core` (`crates/core/src/privacy.rs`) plus `Restricted`, which
/// MASTER_SPEC §19 uses without defining; see decision M5-D4 in
/// `docs/handoffs/M5-requirements-and-acceptance.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PrivacyClass {
    Public,
    Standard,
    Private,
    Sealed,
    Restricted,
}

/// One classified piece of a message. `redacted` means the caller has
/// already replaced the sensitive content of `text`; the dispatcher
/// refuses unredacted `Restricted` segments for cloud dispatch and never
/// guesses at redaction itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub text: String,
    pub privacy: PrivacyClass,
    #[serde(default)]
    pub redacted: bool,
    /// Source memory/record identifier, preserved for citations (§19).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

impl Segment {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            privacy: PrivacyClass::Standard,
            redacted: false,
            source_id: None,
        }
    }
    pub fn classified(text: impl Into<String>, privacy: PrivacyClass) -> Self {
        Self {
            text: text.into(),
            privacy,
            redacted: false,
            source_id: None,
        }
    }
    /// Flattened text used as the wire content once gates have passed.
    pub fn wire_text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: Role,
    pub segments: Vec<Segment>,
}

impl ChatMessage {
    pub fn new(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            segments: vec![Segment::plain(text)],
        }
    }
    pub fn content(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.wire_text())
            .collect::<Vec<_>>()
            .join("")
    }
}

/// Capability flags a model can be routed on (§17: capability is one of
/// the router's evaluation inputs).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitySet {
    pub chat: bool,
    pub streaming: bool,
    pub vision: bool,
    pub coding: bool,
    pub deep_reasoning: bool,
    pub embeddings: bool,
}

/// Cost knowledge about a model. `Unknown` is treated as paid everywhere
/// cost is enforced (decision M5-D5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CostClass {
    Free,
    Paid,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SpeedTier {
    Fast,
    Balanced,
    Deep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub capabilities: CapabilitySet,
    pub context_window: Option<u32>,
    pub cost: CostClass,
    pub speed: SpeedTier,
    /// Caller-supplied price (micro-USD per 1M tokens) with provenance in
    /// configuration. The crate ships no price table (M5-D5).
    pub price_input_micro_usd_per_mtok: Option<u64>,
    pub price_output_micro_usd_per_mtok: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_output_tokens: Option<u32>,
    pub temperature: Option<f32>,
    /// Ask OpenRouter to omit reasoning and reserve output for answer text.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disable_open_router_reasoning: bool,
    /// The caller asserts this content must never leave the local machine
    /// (compiled from local-only memories). The router and gates enforce
    /// it; it is never silently dropped (§17).
    #[serde(default)]
    pub local_only: bool,
    /// Explicit user authorization to send `Private` segments to a cloud
    /// provider (ADR-0002: private material requires explicit
    /// authorization). Never set implicitly by routing.
    #[serde(default)]
    pub explicit_cloud_authorization: bool,
}

impl CompletionRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            max_output_tokens: None,
            temperature: None,
            disable_open_router_reasoning: false,
            local_only: false,
            explicit_cloud_authorization: false,
        }
    }
    /// Rough pre-dispatch token estimate (chars / 4, the same heuristic
    /// class the existing disclosure manifests use). This is an estimate
    /// for gating and disclosure, never a billing figure.
    pub fn estimated_input_tokens(&self) -> u64 {
        let chars: usize = self
            .messages
            .iter()
            .flat_map(|m| m.segments.iter())
            .map(|s| s.text.chars().count())
            .sum();
        (chars as u64).div_ceil(4)
    }
    pub fn estimated_output_tokens(&self) -> u64 {
        self.max_output_tokens.unwrap_or(1024) as u64
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl Usage {
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResponse {
    pub text: String,
    pub provider: String,
    pub model: String,
    pub usage: Option<Usage>,
    pub finish_reason: Option<String>,
}

/// One streamed event. Adapters translate provider-specific stream
/// formats into this single shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    Token(String),
    Usage(Usage),
    Completed { finish_reason: Option<String> },
}
