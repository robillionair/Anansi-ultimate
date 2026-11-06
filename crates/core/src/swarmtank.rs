//! M9 SwarmTank Slice 1: synthetic personas and the
//! deterministic decision engine (MASTER_SPEC §34–37,
//! §40–41).
//!
//! Everything in this module is deterministic and
//! provider-free: personas come from a fixed
//! archetype table varied by a SHA-256 seeded
//! generator, and decisions are a pure function of
//! the persona and the run input. Nothing here calls
//! a model, spends tokens, or claims measured
//! real-world demand. Every persona surface must
//! label the population as simulated.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::artifacts::{ArtifactRegisterRequest, ArtifactReviseRequest};
use crate::provider_dispatch::CallerScope;
use crate::readiness::{CommandError, ErrorCode};
use crate::storage::Storage;

// ---------------------------------------------------------------------------
// Bounds (mirroring the collider family's explicit limits)
// ---------------------------------------------------------------------------

pub const MAX_DESCRIPTION_CHARS: usize = 8000;
pub const MAX_SHORT_CHARS: usize = 500;
pub const MAX_VALUE_PROPOSITION_CHARS: usize = 3000;
pub const MAX_FEATURES: usize = 50;
pub const MAX_FEATURE_CHARS: usize = 300;
pub const MAX_LIST_ITEMS: usize = 12;
pub const MAX_LIST_ITEM_CHARS: usize = 300;
pub const MAX_NAME_CHARS: usize = 120;
pub const BATCH_SIZE: usize = 10;
pub const PRESETS: [u32; 3] = [20, 50, 100];

fn invalid(message: &str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn not_found(message: &str) -> CommandError {
    CommandError::new(ErrorCode::NotFound, message)
}

pub(crate) fn storage(scope: &CallerScope) -> Result<Storage, CommandError> {
    let root = scope
        .storage_root
        .clone()
        .ok_or_else(|| invalid("SwarmTank requires workspace storage."))?;
    Storage::new(root).map_err(|_| crate::storage::unavailable())
}

fn logical(scope: &CallerScope) -> Result<String, CommandError> {
    scope
        .logical_workspace
        .clone()
        .ok_or_else(|| invalid("A workspaceId is required."))
}

fn check_text(value: &str, field: &str, max: usize) -> Result<(), String> {
    let len = value.chars().count();
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if len > max {
        return Err(format!("{field} exceeds {max} characters"));
    }
    Ok(())
}

fn check_optional_text(value: &str, field: &str, max: usize) -> Result<(), String> {
    if value.chars().count() > max {
        return Err(format!("{field} exceeds {max} characters"));
    }
    Ok(())
}

fn check_list(
    items: &[String],
    field: &str,
    max_items: usize,
    max_chars: usize,
) -> Result<(), String> {
    if items.len() > max_items {
        return Err(format!("{field} exceeds {max_items} items"));
    }
    for item in items {
        if item.trim().is_empty() || item.chars().count() > max_chars {
            return Err(format!("{field} items must be 1 to {max_chars} characters"));
        }
    }
    Ok(())
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// A SwarmTank run input: manual fields, or fields
/// copied from a saved experiment / Forge concept.
/// `pricing` is display text; `price_amount` is the
/// optional numeric monthly price the decision engine
/// compares against budgets. A missing numeric price
/// is never invented from the text.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankInput {
    pub description: String,
    #[serde(default)]
    pub pricing: String,
    #[serde(default)]
    pub price_amount: Option<f64>,
    #[serde(default)]
    pub segment: String,
    #[serde(default)]
    pub industry: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub value_proposition: String,
}

/// The import source pin: an exact experiment or
/// concept version, never a floating "latest".
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankSource {
    pub kind: String,
    pub artifact_id: String,
    pub version: u32,
    pub content_hash: String,
}

/// One synthetic buyer. Every surface rendering a
/// persona must label it simulated; the `synthetic`
/// field is always true and is part of the stored
/// record so no consumer can render it unlabeled
/// by accident. `language` is the market's primary
/// business language for this persona (v2); it is
/// absent on runs generated before markets existed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankPersona {
    pub id: String,
    pub name: String,
    pub role: String,
    pub company_size: String,
    pub approximate_budget: f64,
    pub needs: Vec<String>,
    pub priorities: Vec<String>,
    pub skepticism: f64,
    pub price_sensitivity: f64,
    pub purchase_constraints: Vec<String>,
    pub required_trust_signals: Vec<String>,
    pub expected_implementation_effort: String,
    pub synthetic: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// One decision. `key_objection` is required for
/// Confused/Reject and null for Convert.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankDecision {
    pub persona_id: String,
    pub decision: String,
    pub reason: String,
    pub key_objection: Option<String>,
    pub perceived_value: f64,
    pub willingness_to_pay: f64,
    pub score: f64,
}

/// The stored run document. `content_hash` is the
/// SHA-256 of the canonical serialization with the
/// hash field itself empty; validation recomputes
/// it fail-closed. `market` (v2) records the market
/// the run was generated for; it is absent on runs
/// generated before markets existed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankRunDoc {
    pub format: String,
    pub version: u32,
    pub input: SwarmtankInput,
    pub seed: String,
    pub preset: u32,
    pub personas: Vec<SwarmtankPersona>,
    pub decisions: Vec<SwarmtankDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SwarmtankSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market: Option<crate::swarmtank_market::SwarmtankMarket>,
    pub content_hash: String,
}

/// The revision note a transfer writes onto a Forge
/// concept's new version.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankAddendum {
    pub run_artifact_id: String,
    pub run_version: u32,
    pub run_content_hash: String,
    pub top_objections: Vec<CountedItem>,
    pub top_confusion_points: Vec<CountedItem>,
    pub recommendations: Vec<SwarmtankRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CountedItem {
    pub item: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankRecommendation {
    pub title: String,
    pub detail: String,
    pub assumption: String,
    pub synthetic_evidence: String,
    pub uncertainty: String,
    pub validation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankReport {
    pub total: u32,
    pub converts: u32,
    pub confused: u32,
    pub rejects: u32,
    pub conversion_rate: f64,
    pub average_willingness_to_pay: f64,
    pub top_confusion_points: Vec<CountedItem>,
    pub top_objections: Vec<CountedItem>,
    pub objection_breakdown: Vec<CountedItem>,
    pub segments: Vec<CountedItem>,
    pub recommendations: Vec<SwarmtankRecommendation>,
}

// Requests / responses (contract version 1, strict).

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankPersonasRequest {
    #[serde(default = "one")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub input: SwarmtankInput,
    pub preset: u32,
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default)]
    pub source: Option<SwarmtankSource>,
    #[serde(default)]
    pub market: Option<crate::swarmtank_market::SwarmtankMarket>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankBatchInfo {
    pub index: u32,
    pub count: u32,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankPersonasResponse {
    pub contract_version: u8,
    pub personas: Vec<SwarmtankPersona>,
    pub batches: Vec<SwarmtankBatchInfo>,
    pub seed: String,
    pub preset: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankEvaluateRequest {
    #[serde(default = "one")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub input: SwarmtankInput,
    pub preset: u32,
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default)]
    pub source: Option<SwarmtankSource>,
    #[serde(default)]
    pub market: Option<crate::swarmtank_market::SwarmtankMarket>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankEvaluateResponse {
    pub contract_version: u8,
    pub artifact: crate::artifacts::ArtifactRecord,
    pub body: String,
    pub report: SwarmtankReport,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankReportRequest {
    #[serde(default = "one")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub artifact_id: String,
    #[serde(default)]
    pub version: Option<u32>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankReportResponse {
    pub contract_version: u8,
    pub artifact: crate::artifacts::ArtifactRecord,
    pub report: SwarmtankReport,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankTransferRequest {
    #[serde(default = "one")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub run_artifact_id: String,
    pub run_version: u32,
    pub concept_artifact_id: String,
    pub expected_concept_version: u32,
    /// M9 Slice 3: when a scenario branch is applied,
    /// both fields name it; otherwise both are absent
    /// and the transfer carries baseline recommendations.
    #[serde(default)]
    pub scenario_artifact_id: Option<String>,
    #[serde(default)]
    pub scenario_version: Option<u32>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankTransferResponse {
    pub contract_version: u8,
    pub artifact: crate::artifacts::ArtifactRecord,
    pub already_applied: bool,
}

fn one() -> u8 {
    1
}

// ---------------------------------------------------------------------------
// Seeded generator: SHA-256 counter mode over the
// seed string. No new dependency, identical bytes
// on every platform for the same seed.
// ---------------------------------------------------------------------------

struct Seeded {
    seed: String,
    counter: u64,
    buffer: Vec<u8>,
    position: usize,
}

impl Seeded {
    fn new(seed: &str) -> Self {
        Self {
            seed: seed.to_string(),
            counter: 0,
            buffer: Vec::new(),
            position: 0,
        }
    }

    fn refill(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update(self.seed.as_bytes());
        hasher.update(self.counter.to_be_bytes());
        self.buffer = hasher.finalize().to_vec();
        self.counter += 1;
        self.position = 0;
    }

    /// Next uniform value in [0, 1).
    fn next_f64(&mut self) -> f64 {
        if self.position + 8 > self.buffer.len() {
            self.refill();
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.buffer[self.position..self.position + 8]);
        self.position += 8;
        let bits = u64::from_be_bytes(bytes) >> 11;
        bits as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, low: f64, high: f64) -> f64 {
        low + self.next_f64() * (high - low)
    }
}

// ---------------------------------------------------------------------------
// Archetypes (§36 examples, extended to a fixed table
// of twelve; nothing here is presented as a real
// person — every generated persona is synthetic).
// ---------------------------------------------------------------------------

struct Archetype {
    role: &'static str,
    company_size: &'static str,
    base_budget: f64,
    skepticism: (f64, f64),
    price_sensitivity: (f64, f64),
    segment_keywords: &'static [&'static str],
    authority: f64,
    effort: &'static str,
    effort_class: f64,
    needs_pool: &'static [&'static str],
    priorities_pool: &'static [&'static str],
    constraints_pool: &'static [&'static str],
    signals_pool: &'static [&'static str],
}

const ARCHETYPES: [Archetype; 12] = [
    Archetype {
        role: "Cost-sensitive freelancer",
        company_size: "1",
        base_budget: 500.0,
        skepticism: (0.3, 0.6),
        price_sensitivity: (0.7, 1.0),
        segment_keywords: &["freelanc"],
        authority: 0.4,
        effort: "Low",
        effort_class: 0.0,
        needs_pool: &[
            "Low monthly cost",
            "Fast setup",
            "No long contract",
            "Self-serve onboarding",
        ],
        priorities_pool: &["Price", "Speed", "Simplicity"],
        constraints_pool: &["Solo budget approval", "No IT support"],
        signals_pool: &["Free trial", "Transparent pricing"],
    },
    Archetype {
        role: "Enterprise finance director",
        company_size: "1000+",
        base_budget: 50000.0,
        skepticism: (0.5, 0.8),
        price_sensitivity: (0.2, 0.5),
        segment_keywords: &["enterprise", "finance"],
        authority: 0.7,
        effort: "High",
        effort_class: 1.0,
        needs_pool: &[
            "Audit trail",
            "Compliance reporting",
            "Volume pricing",
            "Vendor security review",
        ],
        priorities_pool: &["Compliance", "Reliability", "Total cost"],
        constraints_pool: &["Procurement process", "Security review required"],
        signals_pool: &["SOC 2 report", "Reference customers"],
    },
    Archetype {
        role: "Technical founder",
        company_size: "2-10",
        base_budget: 5000.0,
        skepticism: (0.4, 0.7),
        price_sensitivity: (0.4, 0.7),
        segment_keywords: &["startup", "founder", "technical"],
        authority: 0.9,
        effort: "Low",
        effort_class: 0.0,
        needs_pool: &[
            "API access",
            "Fast integration",
            "Technical documentation",
            "Usage-based pricing",
        ],
        priorities_pool: &["Integration speed", "Technical depth", "Flexibility"],
        constraints_pool: &["Small team bandwidth"],
        signals_pool: &["Public documentation", "Working demo"],
    },
    Archetype {
        role: "Small agency owner",
        company_size: "11-50",
        base_budget: 3000.0,
        skepticism: (0.3, 0.6),
        price_sensitivity: (0.5, 0.8),
        segment_keywords: &["agency", "agencies"],
        authority: 0.9,
        effort: "Medium",
        effort_class: 0.5,
        needs_pool: &[
            "Client reporting",
            "Multi-client seats",
            "White labeling",
            "Fast onboarding",
        ],
        priorities_pool: &["Client impact", "Time saved", "Price"],
        constraints_pool: &["Client budget cycles"],
        signals_pool: &["Case studies", "Free trial"],
    },
    Archetype {
        role: "Corporate procurement manager",
        company_size: "201-1000",
        base_budget: 20000.0,
        skepticism: (0.6, 0.9),
        price_sensitivity: (0.3, 0.6),
        segment_keywords: &["corporate", "procurement"],
        authority: 0.7,
        effort: "High",
        effort_class: 1.0,
        needs_pool: &[
            "Vendor comparison",
            "Contract terms",
            "Renewal control",
            "Spend reporting",
        ],
        priorities_pool: &["Contract safety", "Price", "Vendor stability"],
        constraints_pool: &["Approved vendor list", "Annual budget cycle"],
        signals_pool: &["Enterprise references", "Security questionnaire"],
    },
    Archetype {
        role: "Privacy-conscious consultant",
        company_size: "1",
        base_budget: 2000.0,
        skepticism: (0.7, 1.0),
        price_sensitivity: (0.4, 0.7),
        segment_keywords: &["privacy", "consult"],
        authority: 0.4,
        effort: "Low",
        effort_class: 0.0,
        needs_pool: &[
            "Local data storage",
            "No tracking",
            "Data export",
            "Clear privacy terms",
        ],
        priorities_pool: &["Privacy", "Data control", "Trust"],
        constraints_pool: &["Client confidentiality duties"],
        signals_pool: &["Privacy policy", "Local-first storage"],
    },
    Archetype {
        role: "Overwhelmed operations manager",
        company_size: "51-200",
        base_budget: 8000.0,
        skepticism: (0.4, 0.7),
        price_sensitivity: (0.4, 0.6),
        segment_keywords: &["operations", "ops"],
        authority: 0.7,
        effort: "Medium",
        effort_class: 0.5,
        needs_pool: &[
            "Less manual work",
            "Clear dashboards",
            "Team onboarding",
            "Reliable support",
        ],
        priorities_pool: &["Time saved", "Ease of use", "Support"],
        constraints_pool: &["No time for long rollout"],
        signals_pool: &["Onboarding help", "Support response time"],
    },
    Archetype {
        role: "Startup product manager",
        company_size: "11-50",
        base_budget: 4000.0,
        skepticism: (0.3, 0.6),
        price_sensitivity: (0.5, 0.8),
        segment_keywords: &["startup", "product"],
        authority: 0.7,
        effort: "Medium",
        effort_class: 0.5,
        needs_pool: &[
            "Roadmap visibility",
            "Team collaboration",
            "Analytics",
            "Fast iteration",
        ],
        priorities_pool: &["Iteration speed", "Team fit", "Price"],
        constraints_pool: &["Quarterly budget review"],
        signals_pool: &["Product changelog", "Free trial"],
    },
    Archetype {
        role: "Nonprofit program lead",
        company_size: "11-50",
        base_budget: 1500.0,
        skepticism: (0.3, 0.6),
        price_sensitivity: (0.7, 1.0),
        segment_keywords: &["nonprofit", "charity"],
        authority: 0.7,
        effort: "Low",
        effort_class: 0.0,
        needs_pool: &[
            "Low cost",
            "Grant reporting",
            "Volunteer access",
            "Simple setup",
        ],
        priorities_pool: &["Cost", "Mission fit", "Simplicity"],
        constraints_pool: &["Grant-restricted funds", "Board approval"],
        signals_pool: &["Nonprofit pricing", "Transparent pricing"],
    },
    Archetype {
        role: "E-commerce store owner",
        company_size: "2-10",
        base_budget: 2500.0,
        skepticism: (0.3, 0.6),
        price_sensitivity: (0.6, 0.9),
        segment_keywords: &["ecommerce", "e-commerce", "retail", "store"],
        authority: 0.9,
        effort: "Low",
        effort_class: 0.0,
        needs_pool: &[
            "Sales conversion",
            "Store integration",
            "Promotion tools",
            "Mobile checkout",
        ],
        priorities_pool: &["Revenue impact", "Integration", "Price"],
        constraints_pool: &["Seasonal cash flow"],
        signals_pool: &["Live store examples", "Free trial"],
    },
    Archetype {
        role: "IT administrator",
        company_size: "51-200",
        base_budget: 6000.0,
        skepticism: (0.5, 0.8),
        price_sensitivity: (0.4, 0.7),
        segment_keywords: &["it", "infrastructure"],
        authority: 0.4,
        effort: "High",
        effort_class: 1.0,
        needs_pool: &[
            "Single sign-on",
            "Admin controls",
            "Uptime guarantees",
            "Migration help",
        ],
        priorities_pool: &["Security", "Control", "Reliability"],
        constraints_pool: &["Change windows", "Existing stack lock-in"],
        signals_pool: &["Security documentation", "Uptime history"],
    },
    Archetype {
        role: "Marketing lead",
        company_size: "51-200",
        base_budget: 5000.0,
        skepticism: (0.3, 0.6),
        price_sensitivity: (0.5, 0.8),
        segment_keywords: &["marketing"],
        authority: 0.7,
        effort: "Medium",
        effort_class: 0.5,
        needs_pool: &[
            "Campaign analytics",
            "Lead tracking",
            "Brand controls",
            "Fast launch",
        ],
        priorities_pool: &["Measurable impact", "Speed", "Ease of use"],
        constraints_pool: &["Campaign calendar deadlines"],
        signals_pool: &["Published results", "Free trial"],
    },
];

const FIRST_NAMES: [&str; 20] = [
    "Ava", "Liam", "Maya", "Noah", "Zoe", "Ethan", "Iris", "Omar", "Lena", "Felix", "Nora", "Hugo",
    "Mila", "Jonas", "Tara", "Elif", "Marco", "Ines", "Ravi", "Sofia",
];
const LAST_NAMES: [&str; 20] = [
    "Keller",
    "Novak",
    "Haddad",
    "Larsen",
    "Okafor",
    "Reyes",
    "Kim",
    "Costa",
    "Weber",
    "Ali",
    "Fischer",
    "Dubois",
    "Lindqvist",
    "Moreau",
    "Petrov",
    "Nguyen",
    "Silva",
    "Berg",
    "Kaur",
    "Meyer",
];

fn window(pool: &[&str], start: usize, count: usize) -> Vec<String> {
    (0..count)
        .map(|i| pool[(start + i) % pool.len()].to_string())
        .collect()
}

pub(crate) fn validate_input(input: &SwarmtankInput) -> Result<(), String> {
    check_text(&input.description, "description", MAX_DESCRIPTION_CHARS)?;
    check_optional_text(&input.pricing, "pricing", MAX_SHORT_CHARS)?;
    check_optional_text(&input.segment, "segment", MAX_SHORT_CHARS)?;
    check_optional_text(&input.industry, "industry", MAX_SHORT_CHARS)?;
    check_optional_text(
        &input.value_proposition,
        "valueProposition",
        MAX_VALUE_PROPOSITION_CHARS,
    )?;
    check_list(&input.features, "features", MAX_FEATURES, MAX_FEATURE_CHARS)?;
    if let Some(price) = input.price_amount
        && (!price.is_finite() || price < 0.0)
    {
        return Err("priceAmount must be a finite non-negative number".to_string());
    }
    Ok(())
}

fn validate_persona(persona: &SwarmtankPersona) -> Result<(), String> {
    if persona.id.trim().is_empty() || persona.id.chars().count() > MAX_NAME_CHARS {
        return Err("persona id must be 1 to 120 characters".to_string());
    }
    check_text(&persona.name, "persona.name", MAX_NAME_CHARS)?;
    check_text(&persona.role, "persona.role", MAX_NAME_CHARS)?;
    check_text(
        &persona.company_size,
        "persona.companySize",
        MAX_SHORT_CHARS,
    )?;
    if !persona.approximate_budget.is_finite() || persona.approximate_budget < 0.0 {
        return Err("approximateBudget must be a finite non-negative number".to_string());
    }
    check_list(&persona.needs, "needs", MAX_LIST_ITEMS, MAX_LIST_ITEM_CHARS)?;
    check_list(
        &persona.priorities,
        "priorities",
        MAX_LIST_ITEMS,
        MAX_LIST_ITEM_CHARS,
    )?;
    check_list(
        &persona.purchase_constraints,
        "purchaseConstraints",
        MAX_LIST_ITEMS,
        MAX_LIST_ITEM_CHARS,
    )?;
    check_list(
        &persona.required_trust_signals,
        "requiredTrustSignals",
        MAX_LIST_ITEMS,
        MAX_LIST_ITEM_CHARS,
    )?;
    check_text(
        &persona.expected_implementation_effort,
        "expectedImplementationEffort",
        MAX_SHORT_CHARS,
    )?;
    if !(0.0..=1.0).contains(&persona.skepticism) || !persona.skepticism.is_finite() {
        return Err("skepticism must be between 0.0 and 1.0".to_string());
    }
    if !(0.0..=1.0).contains(&persona.price_sensitivity) || !persona.price_sensitivity.is_finite() {
        return Err("priceSensitivity must be between 0.0 and 1.0".to_string());
    }
    if !persona.synthetic {
        return Err("personas must be labeled synthetic".to_string());
    }
    Ok(())
}

/// Generate a population in validated batches of
/// `BATCH_SIZE`. The same seed + preset always
/// yields the same personas in the same order.
/// True when `role` names a buyer-role archetype. Used by the market
/// profile validator so a profile cannot name a role that does not exist.
pub fn archetype_role_exists(role: &str) -> bool {
    ARCHETYPES.iter().any(|a| a.role == role)
}

pub fn generate_personas(
    seed: &str,
    preset: u32,
) -> Result<(Vec<SwarmtankPersona>, Vec<SwarmtankBatchInfo>), String> {
    if !PRESETS.contains(&preset) {
        return Err("preset must be 20, 50, or 100".to_string());
    }
    let mut rng = Seeded::new(seed);
    // Round-robin archetype order, then a seeded
    // shuffle so populations are not striped.
    let mut order: Vec<usize> = (0..preset as usize).map(|i| i % ARCHETYPES.len()).collect();
    for i in (1..order.len()).rev() {
        let j = (rng.next_f64() * (i + 1) as f64) as usize;
        order.swap(i, j);
    }
    let mut personas = Vec::with_capacity(preset as usize);
    let mut batches = Vec::new();
    for (batch_index, chunk) in order.chunks(BATCH_SIZE).enumerate() {
        let mut batch = Vec::with_capacity(chunk.len());
        for (offset, &archetype_index) in chunk.iter().enumerate() {
            let global_index = batch_index * BATCH_SIZE + offset;
            let archetype = &ARCHETYPES[archetype_index];
            let skepticism = round2(rng.range(archetype.skepticism.0, archetype.skepticism.1));
            let price_sensitivity =
                round2(rng.range(archetype.price_sensitivity.0, archetype.price_sensitivity.1));
            let budget = (archetype.base_budget * rng.range(0.8, 1.2)).round();
            let first = FIRST_NAMES[(rng.next_f64() * FIRST_NAMES.len() as f64) as usize];
            let last = LAST_NAMES[(rng.next_f64() * LAST_NAMES.len() as f64) as usize];
            let needs_start = (rng.next_f64() * archetype.needs_pool.len() as f64) as usize;
            let priorities_start =
                (rng.next_f64() * archetype.priorities_pool.len() as f64) as usize;
            let constraints_start =
                (rng.next_f64() * archetype.constraints_pool.len() as f64) as usize;
            let signals_start = (rng.next_f64() * archetype.signals_pool.len() as f64) as usize;
            let persona = SwarmtankPersona {
                id: format!("persona-{:03}", global_index + 1),
                name: format!("{first} {last}"),
                role: archetype.role.to_string(),
                company_size: archetype.company_size.to_string(),
                approximate_budget: budget,
                needs: window(archetype.needs_pool, needs_start, 3),
                priorities: window(archetype.priorities_pool, priorities_start, 2),
                skepticism,
                price_sensitivity,
                purchase_constraints: window(archetype.constraints_pool, constraints_start, 2),
                required_trust_signals: window(archetype.signals_pool, signals_start, 2),
                expected_implementation_effort: archetype.effort.to_string(),
                synthetic: true,
                language: None,
            };
            validate_persona(&persona)?;
            batch.push(persona);
        }
        batches.push(SwarmtankBatchInfo {
            index: batch_index as u32,
            count: batch.len() as u32,
        });
        personas.extend(batch);
    }
    Ok((personas, batches))
}

/// Generate personas shaped by a market profile (v2 Checkpoint 1).
///
/// The profile's segments decide the archetype mix (round-robin across
/// segments, then across the roles in each segment), the segment's
/// `budget_multiplier` scales the archetype's base budget, the segment's
/// `company_size` replaces the archetype's, and the segment's first
/// language becomes the persona's language. The archetype's role,
/// skepticism, price sensitivity, needs, priorities, constraints and
/// trust signals are unchanged: geography shapes which buyers appear
/// and what they can spend, never their personality.
///
/// A selection with no researched profile (a Custom-assumptions run, or
/// no market at all) falls back to the unshaped `generate_personas`:
/// the standard archetype mix, with no language set. The run doc still
/// records the market selection, so the run is labelled custom.
pub fn generate_personas_for_market(
    seed: &str,
    preset: u32,
    market: &crate::swarmtank_market::SwarmtankMarket,
) -> Result<(Vec<SwarmtankPersona>, Vec<SwarmtankBatchInfo>), String> {
    let profile = market
        .profile_id
        .as_deref()
        .and_then(crate::swarmtank_market::get_market_profile);
    let Some(profile) = profile else {
        return generate_personas(seed, preset);
    };
    if !PRESETS.contains(&preset) {
        return Err("preset must be 20, 50, or 100".to_string());
    }
    // One (archetype, segment) pair per role in each segment, in segment
    // order; the population cycles through them round-robin, then a
    // seeded shuffle so populations are not striped.
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (segment_index, segment) in profile.segments.iter().enumerate() {
        for role in &segment.roles {
            if let Some(archetype_index) = ARCHETYPES.iter().position(|a| a.role == *role) {
                pairs.push((archetype_index, segment_index));
            }
        }
    }
    if pairs.is_empty() {
        return Err("market profile has no usable segments".to_string());
    }
    let mut rng = Seeded::new(seed);
    let mut order: Vec<usize> = (0..preset as usize).map(|i| i % pairs.len()).collect();
    for i in (1..order.len()).rev() {
        let j = (rng.next_f64() * (i + 1) as f64) as usize;
        order.swap(i, j);
    }
    let mut personas = Vec::with_capacity(preset as usize);
    let mut batches = Vec::new();
    for (batch_index, chunk) in order.chunks(BATCH_SIZE).enumerate() {
        let mut batch = Vec::with_capacity(chunk.len());
        for (offset, &pair_index) in chunk.iter().enumerate() {
            let global_index = batch_index * BATCH_SIZE + offset;
            let (archetype_index, segment_index) = pairs[pair_index];
            let archetype = &ARCHETYPES[archetype_index];
            let segment = &profile.segments[segment_index];
            let skepticism = round2(rng.range(archetype.skepticism.0, archetype.skepticism.1));
            let price_sensitivity =
                round2(rng.range(archetype.price_sensitivity.0, archetype.price_sensitivity.1));
            let budget = (archetype.base_budget
                * segment.budget_multiplier
                * rng.range(0.8, 1.2))
            .round();
            let first = FIRST_NAMES[(rng.next_f64() * FIRST_NAMES.len() as f64) as usize];
            let last = LAST_NAMES[(rng.next_f64() * LAST_NAMES.len() as f64) as usize];
            let needs_start = (rng.next_f64() * archetype.needs_pool.len() as f64) as usize;
            let priorities_start =
                (rng.next_f64() * archetype.priorities_pool.len() as f64) as usize;
            let constraints_start =
                (rng.next_f64() * archetype.constraints_pool.len() as f64) as usize;
            let signals_start = (rng.next_f64() * archetype.signals_pool.len() as f64) as usize;
            let language = segment
                .languages
                .first()
                .cloned()
                .or_else(|| {
                    profile
                        .languages
                        .iter()
                        .find(|l| l.role == "business")
                        .map(|l| l.code.clone())
                });
            let persona = SwarmtankPersona {
                id: format!("persona-{:03}", global_index + 1),
                name: format!("{first} {last}"),
                role: archetype.role.to_string(),
                company_size: segment.company_size.to_string(),
                approximate_budget: budget,
                needs: window(archetype.needs_pool, needs_start, 3),
                priorities: window(archetype.priorities_pool, priorities_start, 2),
                skepticism,
                price_sensitivity,
                purchase_constraints: window(archetype.constraints_pool, constraints_start, 2),
                required_trust_signals: window(archetype.signals_pool, signals_start, 2),
                expected_implementation_effort: archetype.effort.to_string(),
                synthetic: true,
                language,
            };
            validate_persona(&persona)?;
            batch.push(persona);
        }
        batches.push(SwarmtankBatchInfo {
            index: batch_index as u32,
            count: batch.len() as u32,
        });
        personas.extend(batch);
    }
    Ok((personas, batches))
}

// ---------------------------------------------------------------------------
// Decision engine (§37): a pure function of the
// persona and the run input. Scores are internal
// model indicators, never calibrated probabilities.
// ---------------------------------------------------------------------------

struct Factors {
    fit: f64,
    budget: f64,
    price: f64,
    trust: f64,
    switching: f64,
    clarity: f64,
    authority: f64,
    score: f64,
}

fn archetype_for(persona: &SwarmtankPersona) -> Option<&'static Archetype> {
    ARCHETYPES.iter().find(|a| a.role == persona.role)
}

fn contains(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn fit_factor(persona: &SwarmtankPersona, input: &SwarmtankInput) -> f64 {
    let archetype = match archetype_for(persona) {
        Some(a) => a,
        None => return 0.4,
    };
    let mut fit = 0.4;
    if archetype
        .segment_keywords
        .iter()
        .any(|k| contains(&input.segment, k))
    {
        fit += 0.3;
    }
    let haystack = format!("{} {}", input.features.join(" "), input.value_proposition);
    if !persona.needs.is_empty() {
        let hits = persona
            .needs
            .iter()
            .filter(|need| {
                need.split_whitespace()
                    .any(|word| word.chars().count() > 3 && contains(&haystack, word))
            })
            .count();
        fit += 0.3 * (hits as f64 / persona.needs.len() as f64);
    }
    fit.clamp(0.0, 1.0)
}

fn clarity_factor(input: &SwarmtankInput) -> f64 {
    let present = [
        !input.description.trim().is_empty(),
        !input.pricing.trim().is_empty(),
        !input.value_proposition.trim().is_empty(),
        !input.features.is_empty(),
    ]
    .iter()
    .filter(|p| **p)
    .count();
    match present {
        4 => 0.9,
        3 => 0.7,
        2 => 0.5,
        1 => 0.3,
        _ => 0.1,
    }
}

fn evaluate_factors(persona: &SwarmtankPersona, input: &SwarmtankInput) -> Factors {
    let fit = fit_factor(persona, input);
    let (budget, price) = match input.price_amount {
        Some(price) if price > 0.0 => {
            let budget = if persona.approximate_budget >= price {
                1.0
            } else if persona.approximate_budget <= 0.0 {
                0.0
            } else {
                persona.approximate_budget / price
            };
            let denom = price.max(persona.approximate_budget);
            let price_factor = if denom > 0.0 {
                (1.0 - persona.price_sensitivity * (price / denom)).clamp(0.0, 1.0)
            } else {
                0.5
            };
            (budget, price_factor)
        }
        _ => (1.0, 0.5),
    };
    let trust = 0.75 - 0.5 * persona.skepticism;
    let effort_class = archetype_for(persona)
        .map(|a| a.effort_class)
        .unwrap_or(0.5);
    let switching = 1.0 - effort_class * 0.5;
    let clarity = clarity_factor(input);
    let authority = archetype_for(persona).map(|a| a.authority).unwrap_or(0.5);
    let score = 0.25 * fit
        + 0.20 * budget
        + 0.15 * price
        + 0.10 * trust
        + 0.10 * switching
        + 0.10 * clarity
        + 0.10 * authority;
    Factors {
        fit,
        budget,
        price,
        trust,
        switching,
        clarity,
        authority,
        score,
    }
}

fn objection_for(factor: &str) -> &'static str {
    match factor {
        "budget" => "Budget constraints",
        "price" => "Price sensitivity",
        "trust" => "Missing trust signals",
        "switching" => "Implementation effort",
        "clarity" => "Unclear value proposition",
        "authority" => "Lacks purchasing authority",
        _ => "Weak product-customer fit",
    }
}

fn category_for(objection: &str) -> &'static str {
    match objection {
        "Budget constraints"
        | "Price sensitivity"
        | "Price exceeds budget"
        | "Price exceeds budget under high price sensitivity" => "price",
        "Missing trust signals" => "trust",
        "Unclear value proposition" => "clarity",
        "Weak product-customer fit" => "fit",
        "Lacks purchasing authority" => "authority",
        "High setup overhead exceeds team capacity" => "switching",
        _ if objection.starts_with("Missing critical required feature:") => "fit",
        _ => "switching",
    }
}

/// Evaluate one persona. The veto is the only
/// non-score rule: a price strictly above budget
/// with price sensitivity at or above 0.5 caps
/// the outcome at Reject.
pub fn evaluate_persona(persona: &SwarmtankPersona, input: &SwarmtankInput) -> SwarmtankDecision {
    let f = evaluate_factors(persona, input);
    let veto = match input.price_amount {
        Some(price) => {
            price > 0.0 && price > persona.approximate_budget && persona.price_sensitivity >= 0.5
        }
        None => false,
    };
    let decision = if veto {
        "Reject"
    } else if f.score >= 0.65 {
        "Convert"
    } else if f.score >= 0.40 {
        "Confused"
    } else {
        "Reject"
    };
    let perceived_value = round2(0.6 * f.fit + 0.4 * f.score);
    let willingness_to_pay =
        (persona.approximate_budget * perceived_value * (1.0 - 0.5 * persona.price_sensitivity))
            .round()
            .clamp(0.0, persona.approximate_budget);
    let key_objection = if decision == "Convert" {
        None
    } else if veto {
        Some("Price exceeds budget".to_string())
    } else {
        let ranked = [
            ("fit", f.fit),
            ("budget", f.budget),
            ("price", f.price),
            ("trust", f.trust),
            ("switching", f.switching),
            ("clarity", f.clarity),
            ("authority", f.authority),
        ];
        let lowest = ranked
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        lowest.map(|(name, _)| objection_for(name).to_string())
    };
    SwarmtankDecision {
        persona_id: persona.id.clone(),
        decision: decision.to_string(),
        reason: format!(
            "Score {:.2}: fit {:.2}, budget {:.2}, price {:.2}, trust {:.2}, switching {:.2}, clarity {:.2}, authority {:.2}.",
            f.score, f.fit, f.budget, f.price, f.trust, f.switching, f.clarity, f.authority
        ),
        key_objection,
        perceived_value,
        willingness_to_pay,
        score: round2(f.score),
    }
}

pub fn evaluate_population(
    personas: &[SwarmtankPersona],
    input: &SwarmtankInput,
) -> Vec<SwarmtankDecision> {
    personas
        .iter()
        .map(|p| evaluate_persona(p, input))
        .collect()
}

// ---------------------------------------------------------------------------
// M9 Slice 3: scenario sensitivity (§39). A
// separate evaluator over a baseline run: the
// baseline engine above is frozen, and scenario
// decisions never rewrite baseline decisions.
// The TS mirror in packages/ui/src/
// swarmtankSensitivity.ts implements exactly these
// formulas; any change here must land there too,
// including the integer-cents reason formatting
// (§2.4 of the Slice 3 design: Rust {:.2} and JS
// toFixed(2) round half-values differently).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScenarioTrustSignals {
    #[serde(default)]
    pub soc2_security: bool,
    #[serde(default)]
    pub free_trial_available: bool,
    #[serde(default)]
    pub sla_guarantee: bool,
    #[serde(default)]
    pub money_back_guarantee: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankScenario {
    pub price_monthly: Option<f64>,
    pub active_features: Vec<String>,
    pub trust_signals: ScenarioTrustSignals,
    #[serde(default = "default_setup_difficulty")]
    pub setup_difficulty: String,
}

fn default_setup_difficulty() -> String {
    "moderate".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioTransition {
    pub from: String,
    pub to: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioObjectionDelta {
    pub item: String,
    pub baseline_count: u32,
    pub scenario_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioMetrics {
    pub total: u32,
    pub converts: u32,
    pub confused: u32,
    pub rejects: u32,
    pub conversion_rate: f64,
    pub average_willingness_to_pay: f64,
    pub average_perceived_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioMetricsPair {
    pub baseline: ScenarioMetrics,
    pub scenario: ScenarioMetrics,
    pub price_elasticity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioComparison {
    pub transitions: Vec<ScenarioTransition>,
    pub converted_ids: Vec<String>,
    pub lost_ids: Vec<String>,
    pub unchanged_count: u32,
    pub resolved_objections: Vec<ScenarioObjectionDelta>,
    pub introduced_objections: Vec<ScenarioObjectionDelta>,
    pub metrics: ScenarioMetricsPair,
}

/// The stored scenario branch document. Self-contained
/// by design: it embeds the parent run's personas,
/// input, and baseline decisions, so validation
/// recomputes everything from the document itself
/// and never needs a parent lookup — which is what
/// keeps branches restore-stable after artifact ids
/// are remapped (the parent pin below is historical
/// provenance only, like the Slice 1 source pin).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankScenarioDoc {
    pub format: String,
    pub version: u32,
    pub parent_artifact_id: String,
    pub parent_version: u32,
    pub parent_content_hash: String,
    pub scenario: SwarmtankScenario,
    pub personas: Vec<SwarmtankPersona>,
    pub baseline_input: SwarmtankInput,
    pub baseline_decisions: Vec<SwarmtankDecision>,
    pub decisions: Vec<SwarmtankDecision>,
    pub transitions: Vec<ScenarioTransition>,
    pub metrics: ScenarioMetricsPair,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankScenarioDiff {
    pub scenario_artifact_id: String,
    pub scenario_version: u32,
    pub scenario_content_hash: String,
    pub baseline_conversion_rate: f64,
    pub scenario_conversion_rate: f64,
    pub conversion_delta: f64,
    pub baseline_average_willingness_to_pay: f64,
    pub scenario_average_willingness_to_pay: f64,
    pub optimal_price_recommendation: String,
    pub resolved_objections: Vec<CountedItem>,
    pub introduced_objections: Vec<CountedItem>,
    pub transitions: Vec<ScenarioTransition>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankScenarioRequest {
    #[serde(default = "one")]
    pub contract_version: u8,
    pub workspace_id: String,
    pub project_id: String,
    pub parent_artifact_id: String,
    pub parent_version: u32,
    pub scenario: SwarmtankScenario,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmtankScenarioResponse {
    pub contract_version: u8,
    pub artifact: crate::artifacts::ArtifactRecord,
    pub body: String,
    pub report: SwarmtankReport,
    pub transitions: Vec<ScenarioTransition>,
    pub metrics: ScenarioMetricsPair,
}

/// Integer-cents formatting shared with the TS
/// mirror: round half away from zero to cents,
/// then print exactly two decimals. Never use
/// {:.2} here — it rounds half-to-even and would
/// diverge from the TS mirror on exact halves.
fn fmt2(value: f64) -> String {
    let cents = (value * 100.0).round() as i64;
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

fn scenario_reason(f: &Factors) -> String {
    format!(
        "Score {}: fit {}, budget {}, price {}, trust {}, switching {}, clarity {}, authority {}.",
        fmt2(f.score),
        fmt2(f.fit),
        fmt2(f.budget),
        fmt2(f.price),
        fmt2(f.trust),
        fmt2(f.switching),
        fmt2(f.clarity),
        fmt2(f.authority)
    )
}

fn validate_scenario(
    scenario: &SwarmtankScenario,
    baseline: &SwarmtankInput,
) -> Result<(), String> {
    if let Some(price) = scenario.price_monthly {
        if !price.is_finite() || price < 0.0 {
            return Err("scenario priceMonthly must be a finite non-negative number".to_string());
        }
        if let Some(base) = baseline.price_amount
            && price > 10.0 * base
        {
            return Err("scenario priceMonthly must not exceed 10x the baseline price".to_string());
        }
    }
    check_list(
        &scenario.active_features,
        "activeFeatures",
        MAX_FEATURES,
        MAX_FEATURE_CHARS,
    )?;
    let mut seen = std::collections::HashSet::new();
    for feature in &scenario.active_features {
        if !seen.insert(feature.clone()) {
            return Err("activeFeatures must not contain duplicates".to_string());
        }
        if !baseline.features.contains(feature) {
            return Err("activeFeatures must be a subset of the baseline features".to_string());
        }
    }
    if !matches!(
        scenario.setup_difficulty.as_str(),
        "effortless" | "moderate" | "complex"
    ) {
        return Err("setupDifficulty must be effortless, moderate, or complex".to_string());
    }
    Ok(())
}

/// Needs covered by a feature list under the Slice 1
/// word-overlap rule: a need counts when any word
/// longer than 3 chars appears in the joined feature
/// + value-proposition text.
fn covered_needs(
    persona: &SwarmtankPersona,
    features: &[String],
    value_proposition: &str,
) -> Vec<String> {
    let haystack = format!("{} {value_proposition}", features.join(" "));
    persona
        .needs
        .iter()
        .filter(|need| {
            need.split_whitespace()
                .any(|word| word.chars().count() > 3 && contains(&haystack, word))
        })
        .cloned()
        .collect()
}

fn has_security_signal(persona: &SwarmtankPersona) -> bool {
    persona
        .required_trust_signals
        .iter()
        .chain(persona.purchase_constraints.iter())
        .any(|signal| {
            contains(signal, "soc")
                || contains(signal, "security")
                || contains(signal, "compliance")
        })
}

fn decision_rank(decision: &str) -> i32 {
    match decision {
        "Convert" => 2,
        "Confused" => 1,
        _ => 0,
    }
}

/// Evaluate one persona under a scenario. The
/// baseline decision drives only the downgrade /
/// upgrade rules; the score itself is recomputed
/// from the scenario inputs with the Slice 1
/// weights and thresholds.
pub fn evaluate_scenario_persona(
    persona: &SwarmtankPersona,
    baseline_input: &SwarmtankInput,
    baseline_decision: &SwarmtankDecision,
    scenario: &SwarmtankScenario,
) -> SwarmtankDecision {
    let mut scenario_input = baseline_input.clone();
    scenario_input.price_amount = scenario.price_monthly.or(baseline_input.price_amount);
    scenario_input.features = scenario.active_features.clone();
    let mut f = evaluate_factors(persona, &scenario_input);
    // Trust deltas (§2.2 of the design).
    let effective_skepticism = (persona.skepticism
        - if scenario.trust_signals.free_trial_available {
            0.20
        } else {
            0.0
        })
    .clamp(0.0, 1.0);
    f.trust = 0.75 - 0.5 * effective_skepticism;
    if scenario.trust_signals.soc2_security && has_security_signal(persona) {
        f.trust = f.trust.max(0.80);
    }
    if scenario.trust_signals.sla_guarantee {
        f.trust = (f.trust + 0.05).min(1.0);
    }
    if scenario.trust_signals.money_back_guarantee {
        f.price = (f.price + 0.10).min(1.0);
    }
    // Setup-effort override.
    match scenario.setup_difficulty.as_str() {
        "effortless" => f.switching = 1.0,
        "complex" => f.switching = f.switching.min(0.5),
        _ => {}
    }
    f.score = 0.25 * f.fit
        + 0.20 * f.budget
        + 0.15 * f.price
        + 0.10 * f.trust
        + 0.10 * f.switching
        + 0.10 * f.clarity
        + 0.10 * f.authority;
    let mut decision = if f.score >= 0.65 {
        "Convert"
    } else if f.score >= 0.40 {
        "Confused"
    } else {
        "Reject"
    };
    let mut key_objection: Option<String> = if decision == "Convert" {
        None
    } else {
        let ranked = [
            ("fit", f.fit),
            ("budget", f.budget),
            ("price", f.price),
            ("trust", f.trust),
            ("switching", f.switching),
            ("clarity", f.clarity),
            ("authority", f.authority),
        ];
        let lowest = ranked
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        lowest.map(|(name, _)| objection_for(name).to_string())
    };
    // Override ladder, in order (design §2.3).
    let scenario_price = scenario_input.price_amount;
    let veto = matches!(scenario_price, Some(price) if price > 0.0)
        && scenario_price.is_some_and(|price| price > persona.approximate_budget)
        && persona.price_sensitivity >= 0.7;
    if veto {
        decision = "Reject";
        key_objection = Some("Price exceeds budget under high price sensitivity".to_string());
    } else {
        if baseline_decision.decision == "Convert" && decision == "Convert" {
            let baseline_covered = covered_needs(
                persona,
                &baseline_input.features,
                &baseline_input.value_proposition,
            );
            let scenario_covered = covered_needs(
                persona,
                &scenario.active_features,
                &baseline_input.value_proposition,
            );
            if !baseline_covered.is_empty() && scenario_covered.is_empty() {
                let first = baseline_covered
                    .iter()
                    .find(|need| !scenario_covered.contains(need))
                    .cloned()
                    .unwrap_or_else(|| baseline_covered[0].clone());
                decision = "Confused";
                key_objection = Some(format!("Missing critical required feature: {first}"));
            } else if scenario.setup_difficulty == "complex"
                && persona.expected_implementation_effort == "Low"
            {
                decision = "Confused";
                key_objection = Some("High setup overhead exceeds team capacity".to_string());
            }
        }
        if baseline_decision.decision == "Confused"
            && decision == "Confused"
            && scenario.trust_signals.free_trial_available
            && matches!(
                persona.role.as_str(),
                "Cost-sensitive freelancer" | "Small agency owner"
            )
            && matches!(
                baseline_decision.key_objection.as_deref(),
                Some("Price sensitivity") | Some("Missing trust signals")
            )
        {
            decision = "Convert";
            key_objection = None;
        }
    }
    let perceived_value = round2(0.6 * f.fit + 0.4 * f.score);
    let willingness_to_pay =
        (persona.approximate_budget * perceived_value * (1.0 - 0.5 * persona.price_sensitivity))
            .round()
            .clamp(0.0, persona.approximate_budget);
    SwarmtankDecision {
        persona_id: persona.id.clone(),
        decision: decision.to_string(),
        reason: scenario_reason(&f),
        key_objection,
        perceived_value,
        willingness_to_pay,
        score: round2(f.score),
    }
}

pub fn evaluate_scenario_population(
    personas: &[SwarmtankPersona],
    baseline_input: &SwarmtankInput,
    baseline_decisions: &[SwarmtankDecision],
    scenario: &SwarmtankScenario,
) -> Vec<SwarmtankDecision> {
    // Positional zip with an id check: in a validated
    // document decisions sit in persona order, so a
    // mismatch can only come from unvalidated input —
    // and then the persona is skipped, which the
    // caller's count check rejects fail-closed rather
    // than silently borrowing another persona's
    // baseline decision.
    personas
        .iter()
        .zip(baseline_decisions.iter())
        .filter(|(p, d)| p.id == d.persona_id)
        .map(|(p, baseline)| evaluate_scenario_persona(p, baseline_input, baseline, scenario))
        .collect()
}

fn scenario_metrics(decisions: &[SwarmtankDecision]) -> ScenarioMetrics {
    let total = decisions.len() as u32;
    let converts = decisions.iter().filter(|d| d.decision == "Convert").count() as u32;
    let confused = decisions
        .iter()
        .filter(|d| d.decision == "Confused")
        .count() as u32;
    let rejects = decisions.iter().filter(|d| d.decision == "Reject").count() as u32;
    ScenarioMetrics {
        total,
        converts,
        confused,
        rejects,
        conversion_rate: if total > 0 {
            round2(100.0 * converts as f64 / total as f64)
        } else {
            0.0
        },
        average_willingness_to_pay: if total > 0 {
            round2(decisions.iter().map(|d| d.willingness_to_pay).sum::<f64>() / total as f64)
        } else {
            0.0
        },
        average_perceived_value: if total > 0 {
            round2(decisions.iter().map(|d| d.perceived_value).sum::<f64>() / total as f64)
        } else {
            0.0
        },
    }
}

fn price_elasticity(
    baseline_price: Option<f64>,
    scenario_price: Option<f64>,
    baseline_rate: f64,
    scenario_rate: f64,
) -> Option<f64> {
    let base = baseline_price?;
    let price = scenario_price?;
    if base <= 0.0 || baseline_rate <= 0.0 || (price - base).abs() < f64::EPSILON {
        return None;
    }
    Some(round2(
        ((scenario_rate - baseline_rate) / baseline_rate) / ((price - base) / base),
    ))
}

pub fn compute_scenario_comparison(
    baseline_decisions: &[SwarmtankDecision],
    scenario_decisions: &[SwarmtankDecision],
    baseline_price: Option<f64>,
    scenario_price: Option<f64>,
) -> ScenarioComparison {
    let baseline_metrics = scenario_metrics(baseline_decisions);
    let scenario_metrics = scenario_metrics(scenario_decisions);
    let mut counts: Vec<(String, String, u32)> = Vec::new();
    let mut converted_ids = Vec::new();
    let mut lost_ids = Vec::new();
    let mut unchanged_count = 0u32;
    for (base, scen) in baseline_decisions.iter().zip(scenario_decisions.iter()) {
        if let Some(entry) = counts
            .iter_mut()
            .find(|(f, t, _)| f == &base.decision && t == &scen.decision)
        {
            entry.2 += 1;
        } else {
            counts.push((base.decision.clone(), scen.decision.clone(), 1));
        }
        let rank_diff = decision_rank(&scen.decision) - decision_rank(&base.decision);
        if rank_diff > 0 {
            converted_ids.push(scen.persona_id.clone());
        } else if rank_diff < 0 {
            lost_ids.push(scen.persona_id.clone());
        } else {
            unchanged_count += 1;
        }
    }
    counts.sort_by(|a, b| {
        decision_rank(&b.0)
            .cmp(&decision_rank(&a.0))
            .then_with(|| decision_rank(&b.1).cmp(&decision_rank(&a.1)))
    });
    let transitions = counts
        .into_iter()
        .map(|(from, to, count)| ScenarioTransition { from, to, count })
        .collect();
    let count_objections = |decisions: &[SwarmtankDecision]| {
        let mut out: Vec<(String, u32)> = Vec::new();
        for d in decisions {
            if let Some(objection) = &d.key_objection {
                if let Some(entry) = out.iter_mut().find(|(item, _)| item == objection) {
                    entry.1 += 1;
                } else {
                    out.push((objection.clone(), 1));
                }
            }
        }
        out
    };
    let baseline_counts = count_objections(baseline_decisions);
    let scenario_counts = count_objections(scenario_decisions);
    let mut resolved = Vec::new();
    for (item, base_count) in &baseline_counts {
        let scen_count = scenario_counts
            .iter()
            .find(|(i, _)| i == item)
            .map(|(_, c)| *c)
            .unwrap_or(0);
        if scen_count < *base_count {
            resolved.push(ScenarioObjectionDelta {
                item: item.clone(),
                baseline_count: *base_count,
                scenario_count: scen_count,
            });
        }
    }
    let mut introduced = Vec::new();
    for (item, scen_count) in &scenario_counts {
        let base_count = baseline_counts
            .iter()
            .find(|(i, _)| i == item)
            .map(|(_, c)| *c)
            .unwrap_or(0);
        if *scen_count > base_count {
            introduced.push(ScenarioObjectionDelta {
                item: item.clone(),
                baseline_count: base_count,
                scenario_count: *scen_count,
            });
        }
    }
    resolved.sort_by(|a, b| {
        (b.baseline_count - b.scenario_count)
            .cmp(&(a.baseline_count - a.scenario_count))
            .then_with(|| a.item.cmp(&b.item))
    });
    introduced.sort_by(|a, b| {
        (b.scenario_count - b.baseline_count)
            .cmp(&(a.scenario_count - a.baseline_count))
            .then_with(|| a.item.cmp(&b.item))
    });
    ScenarioComparison {
        transitions,
        converted_ids,
        lost_ids,
        unchanged_count,
        resolved_objections: resolved,
        introduced_objections: introduced,
        metrics: ScenarioMetricsPair {
            price_elasticity: price_elasticity(
                baseline_price,
                scenario_price,
                baseline_metrics.conversion_rate,
                scenario_metrics.conversion_rate,
            ),
            baseline: baseline_metrics,
            scenario: scenario_metrics,
        },
    }
}

pub fn canonical_scenario_hash(doc: &SwarmtankScenarioDoc) -> String {
    let mut copy = doc.clone();
    copy.content_hash = String::new();
    let body = serde_json::to_string(&copy).unwrap_or_default();
    crate::content::hash(body.as_bytes())
}

fn decisions_match(stored: &[SwarmtankDecision], fresh: &[SwarmtankDecision]) -> bool {
    stored.len() == fresh.len()
        && stored.iter().zip(fresh.iter()).all(|(s, f)| {
            s.persona_id == f.persona_id
                && s.decision == f.decision
                && s.reason == f.reason
                && s.key_objection == f.key_objection
                && (s.perceived_value - f.perceived_value).abs() <= f64::EPSILON
                && (s.willingness_to_pay - f.willingness_to_pay).abs() <= f64::EPSILON
                && (s.score - f.score).abs() <= f64::EPSILON
        })
}

/// Full branch validation: shape, bounds, scenario
/// bounds against the embedded baseline input,
/// baseline recomputation, scenario recomputation,
/// transition/metric recomputation, and the content
/// hash. Self-contained: no parent lookup, so a
/// branch stays valid across restores that remap
/// artifact ids.
pub fn validate_swarmtank_scenario_doc(doc: &SwarmtankScenarioDoc) -> Result<(), String> {
    if doc.format != "anansi-swarmtank-scenario" {
        return Err(format!(
            "swarmtank scenario format must be anansi-swarmtank-scenario, got {}",
            doc.format
        ));
    }
    if doc.version != 1 {
        return Err(format!(
            "swarmtank scenario version must be 1, got {}",
            doc.version
        ));
    }
    if doc.parent_artifact_id.trim().is_empty() || doc.parent_version == 0 {
        return Err("scenario parent pin must name an artifact version".to_string());
    }
    if doc.parent_content_hash.len() != 64
        || !doc
            .parent_content_hash
            .chars()
            .all(|c| c.is_ascii_hexdigit())
    {
        return Err("scenario parent content hash must be 64 hex characters".to_string());
    }
    validate_input(&doc.baseline_input)?;
    validate_scenario(&doc.scenario, &doc.baseline_input)?;
    if doc.personas.is_empty()
        || doc.personas.len() != doc.baseline_decisions.len()
        || doc.personas.len() != doc.decisions.len()
    {
        return Err("scenario persona and decision counts must match and be non-empty".to_string());
    }
    for (index, persona) in doc.personas.iter().enumerate() {
        if persona.id != format!("persona-{:03}", index + 1) {
            return Err(
                "scenario persona ids must be sequential persona-001.. in order".to_string(),
            );
        }
        validate_persona(persona)?;
    }
    let recomputed_baseline = evaluate_population(&doc.personas, &doc.baseline_input);
    if !decisions_match(&doc.baseline_decisions, &recomputed_baseline) {
        return Err("stored baseline decisions do not match recomputation".to_string());
    }
    for decision in doc.baseline_decisions.iter().chain(doc.decisions.iter()) {
        if !matches!(
            decision.decision.as_str(),
            "Convert" | "Confused" | "Reject"
        ) {
            return Err("scenario decisions must be Convert, Confused, or Reject".to_string());
        }
        if decision.decision == "Convert" && decision.key_objection.is_some() {
            return Err("Convert decisions must not carry a key objection".to_string());
        }
        if decision.decision != "Convert" && decision.key_objection.is_none() {
            return Err("Confused and Reject decisions must carry a key objection".to_string());
        }
    }
    let recomputed_scenario = evaluate_scenario_population(
        &doc.personas,
        &doc.baseline_input,
        &doc.baseline_decisions,
        &doc.scenario,
    );
    if !decisions_match(&doc.decisions, &recomputed_scenario) {
        return Err("stored scenario decisions do not match recomputation".to_string());
    }
    let effective_price = doc
        .scenario
        .price_monthly
        .or(doc.baseline_input.price_amount);
    let comparison = compute_scenario_comparison(
        &doc.baseline_decisions,
        &doc.decisions,
        doc.baseline_input.price_amount,
        effective_price,
    );
    if serde_json::to_value(&doc.transitions).unwrap_or_default()
        != serde_json::to_value(&comparison.transitions).unwrap_or_default()
    {
        return Err("stored scenario transitions do not match recomputation".to_string());
    }
    let metrics_match = |a: &ScenarioMetrics, b: &ScenarioMetrics| {
        a.total == b.total
            && a.converts == b.converts
            && a.confused == b.confused
            && a.rejects == b.rejects
            && (a.conversion_rate - b.conversion_rate).abs() <= f64::EPSILON
            && (a.average_willingness_to_pay - b.average_willingness_to_pay).abs() <= f64::EPSILON
            && (a.average_perceived_value - b.average_perceived_value).abs() <= f64::EPSILON
    };
    if !metrics_match(&doc.metrics.baseline, &comparison.metrics.baseline)
        || !metrics_match(&doc.metrics.scenario, &comparison.metrics.scenario)
        || doc.metrics.price_elasticity != comparison.metrics.price_elasticity
    {
        return Err("stored scenario metrics do not match recomputation".to_string());
    }
    if canonical_scenario_hash(doc) != doc.content_hash {
        return Err("swarmtank scenario content hash mismatch".to_string());
    }
    Ok(())
}

fn parse_scenario_doc(raw: &str) -> Result<SwarmtankScenarioDoc, CommandError> {
    let doc: SwarmtankScenarioDoc = serde_json::from_str(raw)
        .map_err(|_| invalid("The SwarmTank scenario body is not readable."))?;
    validate_swarmtank_scenario_doc(&doc).map_err(|e| invalid(&e))?;
    Ok(doc)
}

pub fn build_scenario_doc(
    run: &SwarmtankRunDoc,
    parent_artifact_id: &str,
    parent_version: u32,
    parent_content_hash: &str,
    scenario: &SwarmtankScenario,
) -> Result<SwarmtankScenarioDoc, String> {
    validate_scenario(scenario, &run.input)?;
    let decisions =
        evaluate_scenario_population(&run.personas, &run.input, &run.decisions, scenario);
    let effective_price = scenario.price_monthly.or(run.input.price_amount);
    let comparison = compute_scenario_comparison(
        &run.decisions,
        &decisions,
        run.input.price_amount,
        effective_price,
    );
    let mut doc = SwarmtankScenarioDoc {
        format: "anansi-swarmtank-scenario".to_string(),
        version: 1,
        parent_artifact_id: parent_artifact_id.to_string(),
        parent_version,
        parent_content_hash: parent_content_hash.to_string(),
        scenario: scenario.clone(),
        personas: run.personas.clone(),
        baseline_input: run.input.clone(),
        baseline_decisions: run.decisions.clone(),
        decisions,
        transitions: comparison.transitions,
        metrics: comparison.metrics,
        content_hash: String::new(),
    };
    doc.content_hash = canonical_scenario_hash(&doc);
    validate_swarmtank_scenario_doc(&doc)?;
    Ok(doc)
}

// ---------------------------------------------------------------------------
// Report derivation (§40): pure aggregation over
// the stored decisions. Recommendations are rule
// outputs over observed objections, each carrying
// its assumption, synthetic evidence, uncertainty,
// and real-customer validation step.
// ---------------------------------------------------------------------------

fn top_counts(decisions: &[SwarmtankDecision], wanted: &str, limit: usize) -> Vec<CountedItem> {
    let mut counts: Vec<(String, u32)> = Vec::new();
    for d in decisions {
        if d.decision != wanted {
            continue;
        }
        if let Some(objection) = &d.key_objection {
            if let Some(entry) = counts.iter_mut().find(|(item, _)| item == objection) {
                entry.1 += 1;
            } else {
                counts.push((objection.clone(), 1));
            }
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
        .into_iter()
        .take(limit)
        .map(|(item, count)| CountedItem { item, count })
        .collect()
}

fn recommendation(
    title: &str,
    detail: &str,
    assumption: &str,
    evidence: String,
    validation: &str,
) -> SwarmtankRecommendation {
    SwarmtankRecommendation {
        title: title.to_string(),
        detail: detail.to_string(),
        assumption: assumption.to_string(),
        synthetic_evidence: evidence,
        uncertainty: "Synthetic only — these personas are simulated, not real customers, and the counts are unvalidated model output.".to_string(),
        validation: validation.to_string(),
    }
}

fn recommendations_for(report: &SwarmtankReport) -> Vec<SwarmtankRecommendation> {
    let mut out = Vec::new();
    let count_for = |category: &str| -> u32 {
        report
            .objection_breakdown
            .iter()
            .find(|c| c.item == category)
            .map(|c| c.count)
            .unwrap_or(0)
    };
    if count_for("price") > 0 {
        out.push(recommendation(
            "Revisit pricing and packaging",
            "Price objections lead the synthetic objections; test a lower entry tier or clearer packaging before changing the product.",
            "Personas weigh price against their stated budget the way the decision model assumes.",
            format!("{} price-category objections across {} evaluated personas.", count_for("price"), report.total),
            "Interview 5–10 real buyers in the target segment about what they would pay and what the price must include.",
        ));
    }
    if count_for("trust") > 0 {
        out.push(recommendation(
            "Strengthen trust signals",
            "Trust objections recur; publish the proof these personas ask for (trials, references, security or privacy documentation) before widening reach.",
            "Personas treat absent trust signals as a reason to hold back, scaled by skepticism.",
            format!("{} trust-category objections across {} evaluated personas.", count_for("trust"), report.total),
            "Ask real prospects which proof would change their mind, and check whether that proof already exists unpublished.",
        ));
    }
    if count_for("clarity") > 0 {
        out.push(recommendation(
            "Clarify the value proposition",
            "Confusion concentrates on unclear value; rewrite the top message to state who it is for, what changes, and what it costs, in that order.",
            "Personas score clarity from which input fields are present and stated.",
            format!("{} clarity-category objections across {} evaluated personas.", count_for("clarity"), report.total),
            "Run five-second comprehension tests on the new message with real members of the segment.",
        ));
    }
    if count_for("fit") > 0 {
        out.push(recommendation(
            "Tighten segment fit",
            "Fit objections mean the stated segment and the stated needs do not line up; narrow the segment or add the missing capability the needs name.",
            "Personas score fit from segment keywords and needs coverage in the record's own fields.",
            format!("{} fit-category objections across {} evaluated personas.", count_for("fit"), report.total),
            "Validate the narrowed segment with real customer interviews before repositioning.",
        ));
    }
    if count_for("switching") > 0 {
        out.push(recommendation(
            "Lower implementation effort",
            "Switching objections point at rollout cost; offer guided onboarding, migration help, or a smaller first step.",
            "Personas weigh expected implementation effort against company size.",
            format!("{} switching-category objections across {} evaluated personas.", count_for("switching"), report.total),
            "Ask real evaluators what the first week of adoption would actually cost them.",
        ));
    }
    if count_for("authority") > 0 {
        out.push(recommendation(
            "Enable the economic buyer",
            "Authority objections mean the evaluator cannot sign; add buyer-facing material (business case, procurement pack) for the approver.",
            "Personas carry a fixed seniority class per archetype.",
            format!("{} authority-category objections across {} evaluated personas.", count_for("authority"), report.total),
            "Confirm with real deals who actually signs and what that approver needs to see.",
        ));
    }
    out
}

pub fn derive_report(run: &SwarmtankRunDoc) -> SwarmtankReport {
    let total = run.decisions.len() as u32;
    let converts = run
        .decisions
        .iter()
        .filter(|d| d.decision == "Convert")
        .count() as u32;
    let confused = run
        .decisions
        .iter()
        .filter(|d| d.decision == "Confused")
        .count() as u32;
    let rejects = run
        .decisions
        .iter()
        .filter(|d| d.decision == "Reject")
        .count() as u32;
    let conversion_rate = if total > 0 {
        round2(100.0 * converts as f64 / total as f64)
    } else {
        0.0
    };
    let average_willingness_to_pay = if total > 0 {
        round2(
            run.decisions
                .iter()
                .map(|d| d.willingness_to_pay)
                .sum::<f64>()
                / total as f64,
        )
    } else {
        0.0
    };
    let top_confusion_points = top_counts(&run.decisions, "Confused", 3);
    let top_objections = top_counts(&run.decisions, "Reject", 3);
    let mut breakdown: Vec<(String, u32)> = Vec::new();
    for d in &run.decisions {
        if let Some(objection) = &d.key_objection {
            let category = category_for(objection).to_string();
            if let Some(entry) = breakdown.iter_mut().find(|(item, _)| *item == category) {
                entry.1 += 1;
            } else {
                breakdown.push((category, 1));
            }
        }
    }
    breakdown.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let objection_breakdown = breakdown
        .into_iter()
        .map(|(item, count)| CountedItem { item, count })
        .collect::<Vec<_>>();
    let mut segments: Vec<(String, u32)> = Vec::new();
    for p in &run.personas {
        if let Some(entry) = segments.iter_mut().find(|(item, _)| *item == p.role) {
            entry.1 += 1;
        } else {
            segments.push((p.role.clone(), 1));
        }
    }
    segments.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let segments = segments
        .into_iter()
        .map(|(item, count)| CountedItem { item, count })
        .collect::<Vec<_>>();
    let mut report = SwarmtankReport {
        total,
        converts,
        confused,
        rejects,
        conversion_rate,
        average_willingness_to_pay,
        top_confusion_points,
        top_objections,
        objection_breakdown,
        segments,
        recommendations: Vec::new(),
    };
    report.recommendations = recommendations_for(&report);
    report
}

// ---------------------------------------------------------------------------
// Content hash and fail-closed validation
// ---------------------------------------------------------------------------

pub fn canonical_hash(doc: &SwarmtankRunDoc) -> String {
    let mut copy = doc.clone();
    copy.content_hash = String::new();
    let body = serde_json::to_string(&copy).unwrap_or_default();
    crate::content::hash(body.as_bytes())
}

/// Full validation: shape, bounds, identifier
/// discipline, decision recomputation, and the
/// content hash. Any mismatch refuses the doc.
/// Decisions are recomputed from the stored
/// personas and input and compared field by
/// field, so a tampered decision cannot pass even
/// if the hash were recomputed by an attacker
/// with write access to the hash field alone —
/// the recomputation is independent of it.
pub fn validate_swarmtank_run_doc(doc: &SwarmtankRunDoc) -> Result<(), String> {
    if doc.format != "anansi-swarmtank-run" {
        return Err(format!(
            "swarmtank run format must be anansi-swarmtank-run, got {}",
            doc.format
        ));
    }
    if doc.version != 1 {
        return Err(format!(
            "swarmtank run version must be 1, got {}",
            doc.version
        ));
    }
    if !PRESETS.contains(&doc.preset) {
        return Err("swarmtank run preset must be 20, 50, or 100".to_string());
    }
    validate_input(&doc.input)?;
    if doc.personas.len() != doc.preset as usize {
        return Err("persona count must equal the preset".to_string());
    }
    if doc.decisions.len() != doc.preset as usize {
        return Err("decision count must equal the preset".to_string());
    }
    for (index, persona) in doc.personas.iter().enumerate() {
        if persona.id != format!("persona-{:03}", index + 1) {
            return Err("persona ids must be sequential persona-001.. in order".to_string());
        }
        validate_persona(persona)?;
    }
    let mut seen = std::collections::HashSet::new();
    for persona in &doc.personas {
        if !seen.insert(persona.id.clone()) {
            return Err("duplicate persona id".to_string());
        }
    }
    let recomputed = evaluate_population(&doc.personas, &doc.input);
    for (stored, fresh) in doc.decisions.iter().zip(recomputed.iter()) {
        if stored.persona_id != fresh.persona_id
            || stored.decision != fresh.decision
            || stored.reason != fresh.reason
            || stored.key_objection != fresh.key_objection
            || (stored.perceived_value - fresh.perceived_value).abs() > f64::EPSILON
            || (stored.willingness_to_pay - fresh.willingness_to_pay).abs() > f64::EPSILON
            || (stored.score - fresh.score).abs() > f64::EPSILON
        {
            return Err("stored decision does not match recomputation".to_string());
        }
        if !matches!(stored.decision.as_str(), "Convert" | "Confused" | "Reject") {
            return Err("decision must be Convert, Confused, or Reject".to_string());
        }
        if stored.decision == "Convert" && stored.key_objection.is_some() {
            return Err("Convert decisions must not carry a key objection".to_string());
        }
        if stored.decision != "Convert" && stored.key_objection.is_none() {
            return Err("Confused and Reject decisions must carry a key objection".to_string());
        }
    }
    if let Some(source) = &doc.source {
        if !matches!(source.kind.as_str(), "experiment" | "concept") {
            return Err("source kind must be experiment or concept".to_string());
        }
        if source.artifact_id.trim().is_empty() || source.version == 0 {
            return Err("source pin must name an artifact version".to_string());
        }
        if source.content_hash.len() != 64
            || !source.content_hash.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err("source content hash must be 64 hex characters".to_string());
        }
    }
    if canonical_hash(doc) != doc.content_hash {
        return Err("swarmtank run content hash mismatch".to_string());
    }
    Ok(())
}

fn parse_run_doc(raw: &str) -> Result<SwarmtankRunDoc, CommandError> {
    let doc: SwarmtankRunDoc = serde_json::from_str(raw)
        .map_err(|_| invalid("The SwarmTank run body is not readable."))?;
    validate_swarmtank_run_doc(&doc).map_err(|e| invalid(&e))?;
    Ok(doc)
}

/// Import mapping: field copy only. Fields the
/// source does not state stay empty.
pub fn import_from_experiment(doc: &crate::collider::ColliderExperimentDoc) -> SwarmtankInput {
    SwarmtankInput {
        description: doc.result.productization.concept_brief.clone(),
        features: doc
            .result
            .conceptual_primitives
            .iter()
            .take(MAX_FEATURES)
            .cloned()
            .collect(),
        ..Default::default()
    }
}

pub fn import_from_concept(doc: &crate::transfer::TransferConceptDoc) -> SwarmtankInput {
    SwarmtankInput {
        description: doc.approved_brief.clone(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

pub(crate) fn require_workspace_and_project(
    scope: &CallerScope,
    workspace_id: &str,
    project_id: &str,
) -> Result<(String, Storage), CommandError> {
    let workspace = logical(scope)?;
    if workspace != workspace_id {
        return Err(invalid(
            "The workspaceId does not match the session workspace.",
        ));
    }
    let storage = storage(scope)?;
    let connection = storage
        .connection(false)?
        .ok_or_else(crate::storage::unavailable)?;
    crate::artifacts::require_project_writable(&connection, &workspace, project_id)?;
    drop(connection);
    Ok((workspace, storage))
}

pub(crate) fn resolve_seed(seed: Option<String>) -> Result<String, CommandError> {
    match seed {
        Some(s) if !s.trim().is_empty() => {
            if s.chars().count() > 200 {
                return Err(invalid("The seed must be at most 200 characters."));
            }
            Ok(s)
        }
        Some(_) => Err(invalid("The seed must not be empty.")),
        None => crate::content::random_id().map_err(|_| crate::storage::unavailable()),
    }
}

fn personas_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SwarmtankPersonasRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    validate_input(&req.input).map_err(|e| invalid(&e))?;
    if let Some(source) = &req.source
        && !matches!(source.kind.as_str(), "experiment" | "concept")
    {
        return Err(invalid("The source kind must be experiment or concept."));
    }
    let seed = resolve_seed(req.seed)?;
    if let Some(market) = &req.market {
        crate::swarmtank_market::validate_market(market).map_err(|e| invalid(&e))?;
    }
    let (personas, batches) = match &req.market {
        Some(market) => generate_personas_for_market(&seed, req.preset, market),
        None => generate_personas(&seed, req.preset),
    }
    .map_err(|e| invalid(&e))?;
    serde_json::to_value(SwarmtankPersonasResponse {
        contract_version: 1,
        personas,
        batches,
        seed,
        preset: req.preset,
    })
    .map_err(|_| crate::storage::unavailable())
}

fn evaluate_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SwarmtankEvaluateRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let (workspace, storage) =
        require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    validate_input(&req.input).map_err(|e| invalid(&e))?;
    let seed = resolve_seed(req.seed)?;
    if let Some(market) = &req.market {
        crate::swarmtank_market::validate_market(market).map_err(|e| invalid(&e))?;
    }
    // Regenerate server-side from the seed: the
    // client preview is never trusted for storage.
    let (personas, _) = match &req.market {
        Some(market) => generate_personas_for_market(&seed, req.preset, market),
        None => generate_personas(&seed, req.preset),
    }
    .map_err(|e| invalid(&e))?;
    let decisions = evaluate_population(&personas, &req.input);
    let mut doc = SwarmtankRunDoc {
        format: "anansi-swarmtank-run".to_string(),
        version: 1,
        input: req.input.clone(),
        seed: seed.clone(),
        preset: req.preset,
        personas,
        decisions,
        source: req.source.clone(),
        market: req.market.clone(),
        content_hash: String::new(),
    };
    doc.content_hash = canonical_hash(&doc);
    validate_swarmtank_run_doc(&doc).map_err(|e| invalid(&e))?;
    let body = serde_json::to_string(&doc).map_err(|_| crate::storage::unavailable())?;
    let title = {
        let excerpt: String = req
            .input
            .description
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(160)
            .collect();
        let t = format!("SwarmTank run: {excerpt}");
        if t.chars().count() > 200 {
            t.chars().take(200).collect()
        } else {
            t
        }
    };
    let details = storage.artifact_register_generated(
        &ArtifactRegisterRequest {
            contract_version: 1,
            workspace_id: workspace.clone(),
            project_id: req.project_id.clone(),
            artifact_type: "simulation".to_string(),
            title,
            body: body.clone(),
            source_references: vec![],
            privacy: "private".to_string(),
        },
        serde_json::json!({
            "kind": "deterministic",
            "generator": "anansi-swarmtank",
            "seed": seed,
        }),
        "validated",
    )?;
    let report = derive_report(&doc);
    serde_json::to_value(SwarmtankEvaluateResponse {
        contract_version: 1,
        artifact: details.artifact,
        body,
        report,
    })
    .map_err(|_| crate::storage::unavailable())
}

fn report_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SwarmtankReportRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let workspace = logical(scope)?;
    if workspace != req.workspace_id {
        return Err(invalid(
            "The workspaceId does not match the session workspace.",
        ));
    }
    let storage = storage(scope)?;
    let details = storage
        .artifact_read(&workspace, &req.artifact_id, req.version)
        .map_err(|_| not_found("The SwarmTank run does not exist in this workspace."))?;
    if details.artifact.artifact_type != "simulation" {
        return Err(invalid("The selected artifact is not a SwarmTank run."));
    }
    let doc = parse_run_doc(&details.body)?;
    let report = derive_report(&doc);
    serde_json::to_value(SwarmtankReportResponse {
        contract_version: 1,
        artifact: details.artifact,
        report,
    })
    .map_err(|_| crate::storage::unavailable())
}

fn transfer_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SwarmtankTransferRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let (workspace, storage) =
        require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    let run = storage
        .artifact_read(&workspace, &req.run_artifact_id, Some(req.run_version))
        .map_err(|_| not_found("The SwarmTank run does not exist in this workspace."))?;
    if run.artifact.artifact_type != "simulation" {
        return Err(invalid("The selected artifact is not a SwarmTank run."));
    }
    if run.artifact.project_id != req.project_id {
        return Err(invalid(
            "The SwarmTank run does not belong to this project.",
        ));
    }
    let run_doc = parse_run_doc(&run.body)?;
    let concept_latest = storage
        .artifact_read(&workspace, &req.concept_artifact_id, None)
        .map_err(|_| not_found("The Forge concept does not exist in this workspace."))?;
    if concept_latest.artifact.artifact_type != "concept" {
        return Err(invalid("The selected artifact is not a Forge concept."));
    }
    if concept_latest.artifact.project_id != req.project_id {
        return Err(invalid(
            "The Forge concept does not belong to this project.",
        ));
    }
    let mut concept_doc = crate::transfer::validate_transfer_concept(&concept_latest.body)
        .map_err(|e| invalid(&e))?;
    // M9 Slice 3: an optional scenario branch, resolved
    // and validated server-side — never trusted from
    // the client. The branch must belong to the run
    // being transferred.
    let scenario_branch: Option<(crate::artifacts::ArtifactRecord, SwarmtankScenarioDoc)> =
        match (req.scenario_artifact_id.clone(), req.scenario_version) {
            (None, None) => None,
            (Some(artifact_id), Some(version)) => {
                let branch = storage
                    .artifact_read(&workspace, &artifact_id, Some(version))
                    .map_err(|_| {
                        not_found("The SwarmTank scenario branch does not exist in this workspace.")
                    })?;
                if branch.artifact.artifact_type != "scenario" {
                    return Err(invalid(
                        "The selected artifact is not a SwarmTank scenario branch.",
                    ));
                }
                if branch.artifact.project_id != req.project_id {
                    return Err(invalid(
                        "The SwarmTank scenario branch does not belong to this project.",
                    ));
                }
                let branch_doc = parse_scenario_doc(&branch.body)?;
                if branch_doc.parent_artifact_id != req.run_artifact_id
                    || branch_doc.parent_version != req.run_version
                    || branch_doc.parent_content_hash != run.artifact.content_hash
                {
                    return Err(invalid("The scenario branch does not belong to this run."));
                }
                Some((branch.artifact, branch_doc))
            }
            _ => {
                return Err(invalid(
                    "scenarioArtifactId and scenarioVersion must be given together.",
                ));
            }
        };
    // Structural idempotency, mirroring §27: a
    // concept version already carrying this run's
    // addendum (and, when given, this scenario's
    // diff) is returned unchanged.
    let diff_matches = match (&concept_doc.swarmtank_scenario_diff, &scenario_branch) {
        (None, None) => true,
        (Some(diff), Some((artifact, _))) => {
            diff.scenario_artifact_id == artifact.artifact_id
                && diff.scenario_version == artifact.version
                && diff.scenario_content_hash == artifact.content_hash
        }
        _ => false,
    };
    if let Some(addendum) = &concept_doc.swarmtank_addendum
        && addendum.run_artifact_id == req.run_artifact_id
        && addendum.run_version == req.run_version
        && addendum.run_content_hash == run.artifact.content_hash
        && diff_matches
    {
        return serde_json::to_value(SwarmtankTransferResponse {
            contract_version: 1,
            artifact: concept_latest.artifact,
            already_applied: true,
        })
        .map_err(|_| crate::storage::unavailable());
    }
    if concept_latest.artifact.version != req.expected_concept_version {
        return Err(CommandError::new(
            ErrorCode::RevisionConflict,
            "The Forge concept was revised by someone else; reload it before applying.",
        ));
    }
    let report = derive_report(&run_doc);
    let (recommendations, scenario_diff) = match &scenario_branch {
        Some((artifact, branch_doc)) => {
            let scenario_run = SwarmtankRunDoc {
                format: "anansi-swarmtank-run".to_string(),
                version: 1,
                input: branch_doc.baseline_input.clone(),
                seed: String::new(),
                preset: branch_doc.personas.len() as u32,
                personas: branch_doc.personas.clone(),
                decisions: branch_doc.decisions.clone(),
                source: None,
                market: None,
                content_hash: String::new(),
            };
            let scenario_report = derive_report(&scenario_run);
            let price_text = |price: Option<f64>| match price {
                Some(value) => format!("${value}"),
                None => "unstated".to_string(),
            };
            let effective_price = branch_doc
                .scenario
                .price_monthly
                .or(run_doc.input.price_amount);
            let conversion_delta = round2(scenario_report.conversion_rate - report.conversion_rate);
            let optimal_price_recommendation = if scenario_report.conversion_rate
                > report.conversion_rate
            {
                format!(
                    "Scenario price {} raises synthetic conversion by {} points vs the baseline price {}.",
                    price_text(effective_price),
                    fmt2(conversion_delta),
                    price_text(run_doc.input.price_amount)
                )
            } else {
                format!(
                    "No tested scenario price improved conversion; the baseline price {} remains the synthetic optimum.",
                    price_text(run_doc.input.price_amount)
                )
            };
            let top_deltas = |deltas: &[ScenarioObjectionDelta]| -> Vec<CountedItem> {
                deltas
                    .iter()
                    .take(3)
                    .map(|d| CountedItem {
                        item: d.item.clone(),
                        count: d.baseline_count.abs_diff(d.scenario_count),
                    })
                    .collect()
            };
            let comparison = compute_scenario_comparison(
                &branch_doc.baseline_decisions,
                &branch_doc.decisions,
                run_doc.input.price_amount,
                effective_price,
            );
            (
                scenario_report.recommendations.clone(),
                Some(SwarmtankScenarioDiff {
                    scenario_artifact_id: artifact.artifact_id.clone(),
                    scenario_version: artifact.version,
                    scenario_content_hash: artifact.content_hash.clone(),
                    baseline_conversion_rate: report.conversion_rate,
                    scenario_conversion_rate: scenario_report.conversion_rate,
                    conversion_delta,
                    baseline_average_willingness_to_pay: report.average_willingness_to_pay,
                    scenario_average_willingness_to_pay: scenario_report.average_willingness_to_pay,
                    optimal_price_recommendation,
                    resolved_objections: top_deltas(&comparison.resolved_objections),
                    introduced_objections: top_deltas(&comparison.introduced_objections),
                    transitions: branch_doc.transitions.clone(),
                }),
            )
        }
        None => (report.recommendations.clone(), None),
    };
    concept_doc.swarmtank_addendum = Some(SwarmtankAddendum {
        run_artifact_id: req.run_artifact_id.clone(),
        run_version: req.run_version,
        run_content_hash: run.artifact.content_hash.clone(),
        top_objections: report.top_objections.clone(),
        top_confusion_points: report.top_confusion_points.clone(),
        recommendations,
    });
    // A baseline application clears a stale scenario
    // diff; a scenario application writes its own.
    concept_doc.swarmtank_scenario_diff = scenario_diff;
    let body = serde_json::to_string(&concept_doc).map_err(|_| crate::storage::unavailable())?;
    let details = storage.artifact_revise(&ArtifactReviseRequest {
        contract_version: 1,
        workspace_id: workspace,
        artifact_id: req.concept_artifact_id,
        expected_version: req.expected_concept_version,
        title: concept_latest.artifact.title.clone(),
        body,
        source_references: vec![],
    })?;
    serde_json::to_value(SwarmtankTransferResponse {
        contract_version: 1,
        artifact: details.artifact,
        already_applied: false,
    })
    .map_err(|_| crate::storage::unavailable())
}

fn scenario_op(value: Value, scope: &CallerScope) -> Result<Value, CommandError> {
    let req: SwarmtankScenarioRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let (workspace, storage) =
        require_workspace_and_project(scope, &req.workspace_id, &req.project_id)?;
    let parent = storage
        .artifact_read(
            &workspace,
            &req.parent_artifact_id,
            Some(req.parent_version),
        )
        .map_err(|_| not_found("The SwarmTank run does not exist in this workspace."))?;
    if parent.artifact.artifact_type != "simulation" {
        return Err(invalid("The selected artifact is not a SwarmTank run."));
    }
    if parent.artifact.project_id != req.project_id {
        return Err(invalid(
            "The SwarmTank run does not belong to this project.",
        ));
    }
    let run_doc = parse_run_doc(&parent.body)?;
    let branch = build_scenario_doc(
        &run_doc,
        &req.parent_artifact_id,
        req.parent_version,
        &parent.artifact.content_hash,
        &req.scenario,
    )
    .map_err(|e| invalid(&e))?;
    let body = serde_json::to_string(&branch).map_err(|_| crate::storage::unavailable())?;
    let title = {
        let excerpt: String = run_doc
            .input
            .description
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(160)
            .collect();
        let t = format!("SwarmTank scenario: {excerpt}");
        if t.chars().count() > 200 {
            t.chars().take(200).collect()
        } else {
            t
        }
    };
    let details = storage.artifact_register_generated(
        &ArtifactRegisterRequest {
            contract_version: 1,
            workspace_id: workspace.clone(),
            project_id: req.project_id.clone(),
            artifact_type: "scenario".to_string(),
            title,
            body: body.clone(),
            source_references: vec![],
            privacy: "private".to_string(),
        },
        serde_json::json!({
            "kind": "deterministic",
            "generator": "anansi-swarmtank-scenario",
        }),
        "validated",
    )?;
    let scenario_run = SwarmtankRunDoc {
        format: "anansi-swarmtank-run".to_string(),
        version: 1,
        input: branch.baseline_input.clone(),
        seed: String::new(),
        preset: branch.personas.len() as u32,
        personas: branch.personas.clone(),
        decisions: branch.decisions.clone(),
        source: None,
        market: None,
        content_hash: String::new(),
    };
    let report = derive_report(&scenario_run);
    serde_json::to_value(SwarmtankScenarioResponse {
        contract_version: 1,
        artifact: details.artifact,
        body,
        report,
        transitions: branch.transitions.clone(),
        metrics: branch.metrics.clone(),
    })
    .map_err(|_| crate::storage::unavailable())
}

pub fn swarmtank_command(
    operation: &str,
    value: Value,
    scope: &CallerScope,
) -> Result<Value, CommandError> {
    match operation {
        "swarmtank-personas" => personas_op(value, scope),
        "swarmtank-evaluate" => evaluate_op(value, scope),
        "swarmtank-report" => report_op(value, scope),
        "swarmtank-transfer-to-forge" => transfer_op(value, scope),
        "swarmtank-scenario" => scenario_op(value, scope),
        "swarmtank-territories" => territories_op(value),
        "swarmtank-market" => market_op(value),
        "swarmtank-markets" => markets_op(value),
        "swarmtank-ai-estimate" | "swarmtank-ai-start" | "swarmtank-ai-poll" | "swarmtank-ai-cancel" | "swarmtank-ai-branch" => {
            crate::swarmtank_ai::swarmtank_ai_command(operation, value, scope)
        }
        _ => Err(invalid("Use a supported SwarmTank operation.")),
    }
}

/// `swarmtank-territories`: the versioned CLDR territory list. No
/// workspace or project is needed: the list is static data.
fn territories_op(value: Value) -> Result<Value, CommandError> {
    let req: crate::swarmtank_market::TerritoriesRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let territories =
        crate::swarmtank_market::list_territories().map_err(|e| invalid(&e))?;
    serde_json::to_value(crate::swarmtank_market::TerritoriesResponse {
        contract_version: 1,
        cldr_version: crate::swarmtank_market::CLDR_VERSION.to_string(),
        territories,
    })
    .map_err(|_| crate::storage::unavailable())
}

/// `swarmtank-markets`: a summary of every researched profile, so the
/// selector can show which territories have one (and which
/// subdivisions) without hardcoding profile ids.
fn markets_op(value: Value) -> Result<Value, CommandError> {
    let req: crate::swarmtank_market::TerritoriesRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let profiles = crate::swarmtank_market::list_market_profile_summaries()
        .map_err(|e| invalid(&e))?;
    serde_json::to_value(crate::swarmtank_market::MarketProfilesResponse {
        contract_version: 1,
        profiles,
    })
    .map_err(|_| crate::storage::unavailable())
}

/// `swarmtank-market`: one researched profile by id. A missing profile
/// is not an error — `researched` is false and the caller opens the
/// Custom assumptions state.
fn market_op(value: Value) -> Result<Value, CommandError> {
    let req: crate::swarmtank_market::MarketProfileRequest =
        serde_json::from_value(value).map_err(|_| invalid("Use a supported SwarmTank request."))?;
    if req.contract_version != 1 {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Use contract version 1.",
        ));
    }
    let profile = crate::swarmtank_market::get_market_profile(&req.profile_id);
    serde_json::to_value(crate::swarmtank_market::MarketProfileResponse {
        contract_version: 1,
        researched: profile.is_some(),
        profile,
    })
    .map_err(|_| crate::storage::unavailable())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> SwarmtankInput {
        SwarmtankInput {
            description: "A scheduling tool for small agencies that automates client reporting."
                .to_string(),
            pricing: "$49 per month".to_string(),
            price_amount: Some(49.0),
            segment: "small agencies".to_string(),
            industry: "services".to_string(),
            features: vec![
                "Automated client reporting".to_string(),
                "Team dashboards".to_string(),
            ],
            value_proposition: "Agencies save ten hours a week on reporting.".to_string(),
        }
    }

    fn valid_doc() -> SwarmtankRunDoc {
        let (personas, _) = generate_personas("test-seed", 20).unwrap();
        let decisions = evaluate_population(&personas, &input());
        let mut doc = SwarmtankRunDoc {
            format: "anansi-swarmtank-run".to_string(),
            version: 1,
            input: input(),
            seed: "test-seed".to_string(),
            preset: 20,
            personas,
            decisions,
            source: None,
            market: None,
            content_hash: String::new(),
        };
        doc.content_hash = canonical_hash(&doc);
        doc
    }

    #[test]
    fn generation_is_deterministic_for_a_seed() {
        let (first, first_batches) = generate_personas("seed-a", 20).unwrap();
        let (second, _second_batches) = generate_personas("seed-a", 20).unwrap();
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        assert_eq!(first_batches.len(), 2);
        assert!(first_batches.iter().all(|b| b.count == 10));
        let (other, _) = generate_personas("seed-b", 20).unwrap();
        assert_ne!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&other).unwrap()
        );
    }

    #[test]
    fn market_shapes_personas_without_changing_personality() {
        use crate::swarmtank_market::SwarmtankMarket;
        // AC-M5: personas for a market use the profile's roles, company
        // sizes, budget scaling and language; the archetype's skepticism
        // and price sensitivity ranges are unchanged.
        let market = SwarmtankMarket {
            profile_id: Some("EE".to_string()),
            profile_version: Some(1),
            territory_code: Some("EE".to_string()),
            territory_name: Some("Estonia".to_string()),
            custom: false,
            custom_assumptions: None,
        };
        let (personas, _) = generate_personas_for_market("seed-m", 20, &market).unwrap();
        assert_eq!(personas.len(), 20);
        let profile = crate::swarmtank_market::get_market_profile("EE").unwrap();
        let allowed_roles: Vec<&str> = profile
            .segments
            .iter()
            .flat_map(|s| s.roles.iter().map(|r| r.as_str()))
            .collect();
        let allowed_sizes: Vec<&str> = profile
            .segments
            .iter()
            .map(|s| s.company_size.as_str())
            .collect();
        for persona in &personas {
            assert!(
                allowed_roles.contains(&persona.role.as_str()),
                "role {} is not in the EE profile",
                persona.role
            );
            assert!(
                allowed_sizes.contains(&persona.company_size.as_str()),
                "company size {} is not in the EE profile",
                persona.company_size
            );
            // Language comes from the market (et or en for EE).
            assert!(
                matches!(persona.language.as_deref(), Some("et") | Some("en")),
                "unexpected language {:?}",
                persona.language
            );
            assert!(persona.synthetic);
        }
        // Deterministic for the same seed + market.
        let (again, _) = generate_personas_for_market("seed-m", 20, &market).unwrap();
        assert_eq!(
            serde_json::to_string(&personas).unwrap(),
            serde_json::to_string(&again).unwrap()
        );
        // A different market gives a different population.
        let us = SwarmtankMarket {
            profile_id: Some("US".to_string()),
            profile_version: Some(1),
            territory_code: Some("US".to_string()),
            territory_name: Some("United States".to_string()),
            custom: false,
            custom_assumptions: None,
        };
        let (us_personas, _) = generate_personas_for_market("seed-m", 20, &us).unwrap();
        assert_ne!(
            serde_json::to_string(&personas).unwrap(),
            serde_json::to_string(&us_personas).unwrap()
        );
        // No market (and a Custom-assumptions run) fall back to the
        // unshaped mix with no language set.
        let (plain, _) = generate_personas("seed-m", 20).unwrap();
        assert!(plain.iter().all(|p| p.language.is_none()));
        let custom = SwarmtankMarket {
            profile_id: None,
            profile_version: None,
            territory_code: Some("FR".to_string()),
            territory_name: Some("France".to_string()),
            custom: true,
            custom_assumptions: Some(vec![]),
        };
        let (custom_personas, _) =
            generate_personas_for_market("seed-m", 20, &custom).unwrap();
        assert_eq!(
            serde_json::to_string(&plain).unwrap(),
            serde_json::to_string(&custom_personas).unwrap()
        );
    }

    #[test]
    fn market_validation_rejects_unknown_profiles_and_territories() {
        use crate::swarmtank_market::{SwarmtankMarket, validate_market};
        let unknown_profile = SwarmtankMarket {
            profile_id: Some("XX".to_string()),
            profile_version: Some(1),
            territory_code: Some("XX".to_string()),
            territory_name: None,
            custom: false,
            custom_assumptions: None,
        };
        assert!(validate_market(&unknown_profile).is_err());
        let unknown_territory = SwarmtankMarket {
            profile_id: None,
            profile_version: None,
            territory_code: Some("ZZ".to_string()),
            territory_name: None,
            custom: true,
            custom_assumptions: Some(vec![]),
        };
        assert!(validate_market(&unknown_territory).is_err());
        let custom_with_profile = SwarmtankMarket {
            profile_id: Some("EE".to_string()),
            profile_version: Some(1),
            territory_code: Some("EE".to_string()),
            territory_name: Some("Estonia".to_string()),
            custom: true,
            custom_assumptions: None,
        };
        assert!(validate_market(&custom_with_profile).is_err());
        let custom_without_territory = SwarmtankMarket {
            profile_id: None,
            profile_version: None,
            territory_code: None,
            territory_name: None,
            custom: true,
            custom_assumptions: Some(vec![]),
        };
        assert!(validate_market(&custom_without_territory).is_err());
    }

    #[test]
    fn presets_produce_exact_counts_in_batches_of_ten() {
        for (preset, batches) in [(20, 2), (50, 5), (100, 10)] {
            let (personas, infos) = generate_personas("seed-a", preset).unwrap();
            assert_eq!(personas.len(), preset as usize);
            assert_eq!(infos.len(), batches);
            assert!(infos.iter().all(|b| b.count == 10));
            assert!(personas.iter().all(|p| p.synthetic));
        }
        assert!(generate_personas("seed-a", 30).is_err());
    }

    #[test]
    fn decisions_cover_three_states_with_required_fields() {
        let (personas, _) = generate_personas("seed-a", 100).unwrap();
        let decisions = evaluate_population(&personas, &input());
        assert_eq!(decisions.len(), 100);
        for (persona, decision) in personas.iter().zip(decisions.iter()) {
            assert!(matches!(
                decision.decision.as_str(),
                "Convert" | "Confused" | "Reject"
            ));
            assert_eq!(decision.persona_id, persona.id);
            assert!((0.0..=1.0).contains(&decision.perceived_value));
            assert!((0.0..=1.0).contains(&decision.score));
            assert!(decision.willingness_to_pay >= 0.0);
            assert!(decision.willingness_to_pay <= persona.approximate_budget);
            match decision.decision.as_str() {
                "Convert" => assert!(decision.key_objection.is_none()),
                _ => assert!(decision.key_objection.is_some()),
            }
        }
    }

    #[test]
    fn veto_rejects_over_budget_with_high_price_sensitivity() {
        let mut persona = generate_personas("seed-a", 20)
            .unwrap()
            .0
            .into_iter()
            .next()
            .unwrap();
        persona.approximate_budget = 10.0;
        persona.price_sensitivity = 0.9;
        let mut priced = input();
        priced.price_amount = Some(500.0);
        let decision = evaluate_persona(&persona, &priced);
        assert_eq!(decision.decision, "Reject");
        assert_eq!(
            decision.key_objection.as_deref(),
            Some("Price exceeds budget")
        );
        // The same persona with no stated price is not vetoed.
        let mut unpriced = input();
        unpriced.price_amount = None;
        let unpriced_decision = evaluate_persona(&persona, &unpriced);
        assert_ne!(
            unpriced_decision.key_objection.as_deref(),
            Some("Price exceeds budget")
        );
    }

    #[test]
    fn zero_budget_yields_zero_willingness_to_pay() {
        let mut persona = generate_personas("seed-a", 20)
            .unwrap()
            .0
            .into_iter()
            .next()
            .unwrap();
        persona.approximate_budget = 0.0;
        let decision = evaluate_persona(&persona, &input());
        assert_eq!(decision.willingness_to_pay, 0.0);
    }

    #[test]
    fn extreme_skepticism_and_sensitivity_stay_in_bounds() {
        let mut persona = generate_personas("seed-a", 20)
            .unwrap()
            .0
            .into_iter()
            .next()
            .unwrap();
        persona.skepticism = 1.0;
        persona.price_sensitivity = 1.0;
        let decision = evaluate_persona(&persona, &input());
        assert!((0.0..=1.0).contains(&decision.perceived_value));
        assert!((0.0..=1.0).contains(&decision.score));
        persona.skepticism = 0.0;
        persona.price_sensitivity = 0.0;
        let decision = evaluate_persona(&persona, &input());
        assert!((0.0..=1.0).contains(&decision.perceived_value));
    }

    #[test]
    fn run_doc_validates_and_tampering_fails_closed() {
        let doc = valid_doc();
        assert!(validate_swarmtank_run_doc(&doc).is_ok());
        let raw = serde_json::to_string(&doc).unwrap();
        assert!(parse_run_doc(&raw).is_ok());

        let mut tampered = doc.clone();
        tampered.decisions[0].score = 0.99;
        assert!(validate_swarmtank_run_doc(&tampered).is_err());

        let mut tampered = doc.clone();
        tampered.personas[0].skepticism = 0.01;
        assert!(validate_swarmtank_run_doc(&tampered).is_err());

        let mut tampered = doc.clone();
        tampered.content_hash = "0".repeat(64);
        assert!(validate_swarmtank_run_doc(&tampered).is_err());

        let mut tampered = doc.clone();
        tampered.decisions[0].key_objection = None;
        if tampered.decisions[0].decision != "Convert" {
            assert!(validate_swarmtank_run_doc(&tampered).is_err());
        }

        let mut tampered = doc.clone();
        tampered.personas.truncate(19);
        assert!(validate_swarmtank_run_doc(&tampered).is_err());
    }

    #[test]
    fn report_aggregates_and_recommendations_carry_section_40_fields() {
        let doc = valid_doc();
        let report = derive_report(&doc);
        assert_eq!(report.total, 20);
        assert_eq!(
            report.converts + report.confused + report.rejects,
            report.total
        );
        assert!(report.top_confusion_points.len() <= 3);
        assert!(report.top_objections.len() <= 3);
        for recommendation in &report.recommendations {
            assert!(!recommendation.assumption.is_empty());
            assert!(!recommendation.synthetic_evidence.is_empty());
            assert!(!recommendation.uncertainty.is_empty());
            assert!(!recommendation.validation.is_empty());
        }
        let expected_rate = if report.total > 0 {
            round2(100.0 * report.converts as f64 / report.total as f64)
        } else {
            0.0
        };
        assert_eq!(report.conversion_rate, expected_rate);
    }

    #[test]
    fn input_bounds_are_enforced() {
        let mut bad = input();
        bad.description = "x".repeat(MAX_DESCRIPTION_CHARS + 1);
        assert!(validate_input(&bad).is_err());
        let mut bad = input();
        bad.price_amount = Some(-1.0);
        assert!(validate_input(&bad).is_err());
        let mut bad = input();
        bad.features = vec!["x".to_string(); MAX_FEATURES + 1];
        assert!(validate_input(&bad).is_err());
    }

    // ---- M9 Slice 3 scenario tests. Vectors are
    // hand-computed from the design formulas and
    // shared with the TS mirror tests; any formula
    // change must update both suites. ----

    fn scenario_persona(
        role: &str,
        budget: f64,
        price_sensitivity: f64,
        skepticism: f64,
        needs: Vec<&str>,
        effort: &str,
    ) -> SwarmtankPersona {
        SwarmtankPersona {
            id: "persona-001".to_string(),
            name: "Test Persona".to_string(),
            role: role.to_string(),
            company_size: "11-50".to_string(),
            approximate_budget: budget,
            needs: needs.into_iter().map(|s| s.to_string()).collect(),
            priorities: vec!["Price".to_string()],
            skepticism,
            price_sensitivity,
            purchase_constraints: vec![],
            required_trust_signals: vec![],
            expected_implementation_effort: effort.to_string(),
            synthetic: true,
            language: None,
        }
    }

    fn scenario_input(
        price: Option<f64>,
        segment: &str,
        features: Vec<&str>,
        value_proposition: &str,
        pricing: &str,
    ) -> SwarmtankInput {
        SwarmtankInput {
            description: "A scheduling tool for small agencies.".to_string(),
            pricing: pricing.to_string(),
            price_amount: price,
            segment: segment.to_string(),
            industry: "services".to_string(),
            features: features.into_iter().map(|s| s.to_string()).collect(),
            value_proposition: value_proposition.to_string(),
        }
    }

    fn default_scenario(input: &SwarmtankInput) -> SwarmtankScenario {
        SwarmtankScenario {
            price_monthly: None,
            active_features: input.features.clone(),
            trust_signals: ScenarioTrustSignals {
                soc2_security: false,
                free_trial_available: false,
                sla_guarantee: false,
                money_back_guarantee: false,
            },
            setup_difficulty: "moderate".to_string(),
        }
    }

    #[test]
    fn scenario_veto_requires_high_price_sensitivity() {
        // Freelancer, budget 500, sensitivity 0.9:
        // baseline price 49 converts (score 0.79);
        // scenario price 600 vetoes at >= 0.7.
        let persona = scenario_persona(
            "Cost-sensitive freelancer",
            500.0,
            0.9,
            0.5,
            vec!["Low monthly cost"],
            "Low",
        );
        let baseline = scenario_input(
            Some(49.0),
            "freelanc",
            vec!["Automated client reporting"],
            "Agencies save ten hours a week on reporting.",
            "$49",
        );
        let baseline_decision = evaluate_persona(&persona, &baseline);
        assert_eq!(baseline_decision.decision, "Convert");
        let mut scenario = default_scenario(&baseline);
        scenario.price_monthly = Some(600.0);
        let decision =
            evaluate_scenario_persona(&persona, &baseline, &baseline_decision, &scenario);
        assert_eq!(decision.decision, "Reject");
        assert_eq!(
            decision.key_objection.as_deref(),
            Some("Price exceeds budget under high price sensitivity")
        );
    }

    #[test]
    fn scenario_veto_does_not_rewrite_the_baseline_veto() {
        // Agency owner, budget 100, sensitivity 0.6:
        // baseline price 500 vetoes at >= 0.5, but the
        // scenario veto needs >= 0.7, so the same
        // price re-evaluates by score to Convert
        // (fit 1.0, score 0.66).
        let persona = scenario_persona(
            "Small agency owner",
            100.0,
            0.6,
            0.4,
            vec!["Client reporting"],
            "Medium",
        );
        let baseline = scenario_input(
            Some(500.0),
            "small agencies",
            vec!["Client reporting"],
            "Saves time.",
            "$500",
        );
        let baseline_decision = evaluate_persona(&persona, &baseline);
        assert_eq!(baseline_decision.decision, "Reject");
        assert_eq!(
            baseline_decision.key_objection.as_deref(),
            Some("Price exceeds budget")
        );
        let mut scenario = default_scenario(&baseline);
        scenario.price_monthly = Some(500.0);
        let decision =
            evaluate_scenario_persona(&persona, &baseline, &baseline_decision, &scenario);
        assert_eq!(decision.decision, "Convert");
        assert_eq!(decision.key_objection, None);
    }

    #[test]
    fn free_trial_resolves_freelancer_price_hesitation() {
        // Freelancer, score 0.5925 Confused on price
        // sensitivity; the trial's skepticism cut alone
        // lifts it only to 0.6025, but the explicit
        // freelancer/agency resolution rule converts it.
        let persona = scenario_persona(
            "Cost-sensitive freelancer",
            300.0,
            0.9,
            1.0,
            vec!["Low monthly cost"],
            "Low",
        );
        let baseline = scenario_input(
            Some(250.0),
            "other",
            vec!["Automated client reporting"],
            "Saves time.",
            "$250",
        );
        let baseline_decision = evaluate_persona(&persona, &baseline);
        assert_eq!(baseline_decision.decision, "Confused");
        assert_eq!(
            baseline_decision.key_objection.as_deref(),
            Some("Price sensitivity")
        );
        let mut scenario = default_scenario(&baseline);
        scenario.trust_signals.free_trial_available = true;
        let decision =
            evaluate_scenario_persona(&persona, &baseline, &baseline_decision, &scenario);
        assert_eq!(decision.decision, "Convert");
        assert_eq!(decision.key_objection, None);
    }

    #[test]
    fn feature_drop_downgrades_a_convert_with_exact_objection() {
        let persona = scenario_persona(
            "Small agency owner",
            3000.0,
            0.5,
            0.4,
            vec!["Client reporting"],
            "Medium",
        );
        let baseline = scenario_input(
            Some(49.0),
            "small agencies",
            vec!["Client reporting"],
            "Saves time.",
            "$49",
        );
        let baseline_decision = evaluate_persona(&persona, &baseline);
        assert_eq!(baseline_decision.decision, "Convert");
        let mut scenario = default_scenario(&baseline);
        scenario.active_features = vec![];
        let decision =
            evaluate_scenario_persona(&persona, &baseline, &baseline_decision, &scenario);
        assert_eq!(decision.decision, "Confused");
        assert_eq!(
            decision.key_objection.as_deref(),
            Some("Missing critical required feature: Client reporting")
        );
    }

    #[test]
    fn complex_setup_downgrades_low_effort_converts() {
        let persona = scenario_persona(
            "Cost-sensitive freelancer",
            5000.0,
            0.3,
            0.3,
            vec!["Fast setup"],
            "Low",
        );
        let baseline = scenario_input(
            Some(49.0),
            "freelanc",
            vec!["Fast setup"],
            "Saves time.",
            "$49",
        );
        let baseline_decision = evaluate_persona(&persona, &baseline);
        assert_eq!(baseline_decision.decision, "Convert");
        let mut scenario = default_scenario(&baseline);
        scenario.setup_difficulty = "complex".to_string();
        let decision =
            evaluate_scenario_persona(&persona, &baseline, &baseline_decision, &scenario);
        assert_eq!(decision.decision, "Confused");
        assert_eq!(
            decision.key_objection.as_deref(),
            Some("High setup overhead exceeds team capacity")
        );
    }

    #[test]
    fn soc2_floor_lifts_a_trust_confused_enterprise_persona() {
        // Enterprise finance director, score 0.60
        // Confused on trust (0.25 lowest); SOC 2
        // raises trust to 0.80, score to 0.655.
        let mut persona = scenario_persona(
            "Enterprise finance director",
            50.0,
            0.4,
            1.0,
            vec!["Audit trail"],
            "High",
        );
        persona.required_trust_signals = vec!["SOC 2 report".to_string()];
        persona.purchase_constraints = vec!["Security review required".to_string()];
        let baseline = scenario_input(
            Some(100.0),
            "enterprise finance",
            vec!["Unrelated widget"],
            "Saves time.",
            "$100",
        );
        let baseline_decision = evaluate_persona(&persona, &baseline);
        assert_eq!(baseline_decision.decision, "Confused");
        assert_eq!(
            baseline_decision.key_objection.as_deref(),
            Some("Missing trust signals")
        );
        let mut scenario = default_scenario(&baseline);
        scenario.trust_signals.soc2_security = true;
        let decision =
            evaluate_scenario_persona(&persona, &baseline, &baseline_decision, &scenario);
        assert_eq!(decision.decision, "Convert");
        assert_eq!(decision.key_objection, None);
        assert!(decision.reason.contains("trust 0.80"));
    }

    #[test]
    fn default_scenario_reproduces_baseline_decisions() {
        let doc = valid_doc();
        let scenario = default_scenario(&doc.input);
        let scenario_decisions =
            evaluate_scenario_population(&doc.personas, &doc.input, &doc.decisions, &scenario);
        assert_eq!(scenario_decisions.len(), doc.decisions.len());
        for (base, scen) in doc.decisions.iter().zip(scenario_decisions.iter()) {
            // Reason strings are excluded: baseline
            // reasons use Rust {:.2} while scenario
            // reasons use integer-cents formatting,
            // which can differ on exact halves. Every
            // other field must agree exactly.
            assert_eq!(base.persona_id, scen.persona_id);
            assert_eq!(base.decision, scen.decision);
            assert_eq!(base.key_objection, scen.key_objection);
            assert_eq!(base.perceived_value, scen.perceived_value);
            assert_eq!(base.willingness_to_pay, scen.willingness_to_pay);
            assert_eq!(base.score, scen.score);
        }
    }

    #[test]
    fn scenario_branch_validates_and_tampering_fails_closed() {
        let doc = valid_doc();
        let mut scenario = default_scenario(&doc.input);
        scenario.price_monthly = Some(99.0);
        let branch = build_scenario_doc(&doc, "artifact-abc", 1, &"b".repeat(64), &scenario)
            .expect("branch builds");
        assert!(validate_swarmtank_scenario_doc(&branch).is_ok());

        let mut tampered = branch.clone();
        tampered.decisions[0].score = 0.99;
        assert!(validate_swarmtank_scenario_doc(&tampered).is_err());

        let mut tampered = branch.clone();
        tampered.content_hash = "0".repeat(64);
        assert!(validate_swarmtank_scenario_doc(&tampered).is_err());

        let mut tampered = branch.clone();
        tampered.scenario.active_features = vec!["Not a baseline feature".to_string()];
        assert!(validate_swarmtank_scenario_doc(&tampered).is_err());

        let mut tampered = branch.clone();
        tampered.transitions.clear();
        assert!(validate_swarmtank_scenario_doc(&tampered).is_err());
    }

    #[test]
    fn transition_matrix_counts_migrations() {
        let mk = |id: &str, decision: &str, objection: Option<&str>| SwarmtankDecision {
            persona_id: id.to_string(),
            decision: decision.to_string(),
            reason: "Score 0.50.".to_string(),
            key_objection: objection.map(|s| s.to_string()),
            perceived_value: 0.5,
            willingness_to_pay: 100.0,
            score: 0.5,
        };
        let baseline = vec![
            mk("persona-001", "Convert", None),
            mk("persona-002", "Confused", Some("Price sensitivity")),
            mk("persona-003", "Reject", Some("Budget constraints")),
        ];
        let scenario = vec![
            mk("persona-001", "Confused", Some("Price sensitivity")),
            mk("persona-002", "Convert", None),
            mk("persona-003", "Reject", Some("Price sensitivity")),
        ];
        let comparison = compute_scenario_comparison(&baseline, &scenario, Some(49.0), Some(49.0));
        assert_eq!(comparison.converted_ids, vec!["persona-002".to_string()]);
        assert_eq!(comparison.lost_ids, vec!["persona-001".to_string()]);
        assert_eq!(comparison.unchanged_count, 1);
        assert_eq!(comparison.transitions.len(), 3);
        assert_eq!(comparison.resolved_objections.len(), 1);
        assert_eq!(comparison.resolved_objections[0].item, "Budget constraints");
        assert_eq!(comparison.resolved_objections[0].baseline_count, 1);
        assert_eq!(comparison.resolved_objections[0].scenario_count, 0);
        assert_eq!(comparison.introduced_objections.len(), 1);
        assert_eq!(
            comparison.introduced_objections[0].item,
            "Price sensitivity"
        );
        assert_eq!(comparison.introduced_objections[0].baseline_count, 1);
        assert_eq!(comparison.introduced_objections[0].scenario_count, 2);
    }
}
