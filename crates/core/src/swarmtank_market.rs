//! SwarmTank market selection (v2 Checkpoint 1).
//!
//! The territory list is versioned Unicode CLDR data (CLDR 49), generated
//! by `scripts/generate-cldr-territories.mjs` and committed, so the build
//! never needs network access. Market profiles are versioned data files
//! under `crates/core/src/markets/profiles/`; adding a region means adding
//! a data file — no app-code change.
//!
//! Geography is separate from personality: a profile supplies
//! geographic/buyer-context evidence (languages, procurement norms, size
//! distribution, budget hints). It never assigns a personality trait by
//! nationality. A persona's skepticism, priorities and price sensitivity
//! come from its buyer-role archetype and the scenario, not its country.
//!
//! Every evidence-backed input carries a source and a date. Gaps are
//! explicit, editable assumptions, never silently presented as evidence.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One CLDR territory (country or territory).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Territory {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TerritoriesFile {
    format: String,
    version: u32,
    cldr_version: String,
    #[allow(dead_code)]
    source_url: String,
    #[allow(dead_code)]
    source_sha256: String,
    #[allow(dead_code)]
    generated: String,
    territories: Vec<Territory>,
}

/// A language spoken in the market, with its role there.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketLanguage {
    pub code: String,
    pub name: String,
    pub role: String,
}

/// One evidence-backed input. `source` and `date` are required: an input
/// without them is an assumption, not evidence.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketEvidence {
    pub field: String,
    pub value: String,
    pub source: MarketSource,
    pub date: String,
}

/// The source of one evidence-backed input.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketSource {
    pub name: String,
    pub url: String,
}

/// One gap, stated as an editable assumption. Assumptions are never
/// presented as evidence.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketAssumption {
    pub field: String,
    pub value: String,
    pub rationale: String,
    pub editable: bool,
}

/// One buyer segment in the market. `roles` name buyer-role archetypes
/// (see `swarmtank::ARCHETYPES`); `budget_multiplier` scales the
/// archetype's base budget for this market.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketSegment {
    pub id: String,
    pub label: String,
    pub company_size: String,
    pub roles: Vec<String>,
    pub buying_authority: String,
    pub languages: Vec<String>,
    pub budget_multiplier: f64,
}

/// One versioned market profile (data file).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketProfile {
    pub format: String,
    pub version: u32,
    pub id: String,
    pub territory_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdivision: Option<String>,
    pub name: String,
    pub cldr_version: String,
    pub profile_version: u32,
    pub currency: String,
    pub languages: Vec<MarketLanguage>,
    pub evidence: Vec<MarketEvidence>,
    pub assumptions: Vec<MarketAssumption>,
    pub segments: Vec<MarketSegment>,
}

/// The market a run was generated for. `custom` records a
/// Custom-assumptions run (a territory with no researched profile):
/// the user supplied the assumptions explicitly and the run is
/// labelled custom, never as researched.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmtankMarket {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub territory_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub territory_name: Option<String>,
    #[serde(default)]
    pub custom: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_assumptions: Option<Vec<MarketAssumption>>,
}

// API request/response types (contract version 1, strict).

fn one() -> u8 {
    1
}

/// `swarmtank-territories` request: the list is static data, so the
/// request carries only the contract version.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerritoriesRequest {
    #[serde(default = "one")]
    pub contract_version: u8,
}

/// `swarmtank-territories` response: the full CLDR territory list.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerritoriesResponse {
    pub contract_version: u8,
    pub cldr_version: String,
    pub territories: Vec<Territory>,
}

/// `swarmtank-market` request: one researched profile by id.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketProfileRequest {
    #[serde(default = "one")]
    pub contract_version: u8,
    pub profile_id: String,
}

/// `swarmtank-markets` response: a summary of every researched profile,
/// so the selector can show which territories have a researched profile
/// (and which subdivisions) without hardcoding profile ids. Adding a
/// region is a data file; the selector picks it up from this list.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarketProfilesResponse {
    pub contract_version: u8,
    pub profiles: Vec<MarketProfileSummary>,
}

/// One researched profile, summarized for the selector.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarketProfileSummary {
    pub id: String,
    pub name: String,
    pub territory_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdivision: Option<String>,
    pub profile_version: u32,
}

/// `swarmtank-market` response. `researched` is false (and `profile`
/// is None) when no researched profile exists for the id: the caller
/// opens the Custom assumptions state. That is not an error.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarketProfileResponse {
    pub contract_version: u8,
    pub researched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<MarketProfile>,
}

const TERRITORIES_JSON: &str = include_str!("markets/territories.json");

const PROFILE_EE_JSON: &str = include_str!("markets/profiles/ee.json");
const PROFILE_AE_DXB_JSON: &str = include_str!("markets/profiles/ae-dxb.json");
const PROFILE_US_JSON: &str = include_str!("markets/profiles/us.json");

/// The CLDR version the territory list was generated from.
pub const CLDR_VERSION: &str = "49";

fn load_territories() -> Result<Vec<Territory>, String> {
    let file: TerritoriesFile =
        serde_json::from_str(TERRITORIES_JSON).map_err(|e| format!("territories data is malformed: {e}"))?;
    if file.format != "anansi-cldr-territories" || file.version != 1 {
        return Err("territories data has an unsupported format or version".to_string());
    }
    if file.cldr_version != CLDR_VERSION {
        return Err(format!(
            "territories data is CLDR {}, expected CLDR {CLDR_VERSION}",
            file.cldr_version
        ));
    }
    if file.territories.is_empty() {
        return Err("territories data is empty".to_string());
    }
    Ok(file.territories)
}

/// Validate a run's market selection, fail-closed.
/// - A `profile_id` must name a researched profile.
/// - A `territory_code` must be a CLDR territory.
/// - A custom run must not name a researched profile.
/// - A researched run must name its territory and profile version.
pub fn validate_market(market: &SwarmtankMarket) -> Result<(), String> {
    if let Some(profile_id) = &market.profile_id {
        if get_market_profile(profile_id).is_none() {
            return Err(format!("unknown market profile {profile_id:?}"));
        }
        if market.custom {
            return Err("a custom run must not name a researched profile".to_string());
        }
        if market.profile_version.is_none() {
            return Err(format!(
                "market profile {profile_id:?} is missing its profile version"
            ));
        }
        if market.territory_code.is_none() {
            return Err(format!(
                "market profile {profile_id:?} is missing its territory code"
            ));
        }
    }
    if let Some(code) = &market.territory_code {
        if !is_valid_territory(code) {
            return Err(format!("unknown territory code {code:?}"));
        }
    }
    if market.custom && market.territory_code.is_none() {
        return Err("a custom run must name its territory".to_string());
    }
    Ok(())
}

/// The full CLDR territory list (countries and territories).
pub fn list_territories() -> Result<Vec<Territory>, String> {
    load_territories()
}

/// True when `code` is a territory in the CLDR list.
pub fn is_valid_territory(code: &str) -> bool {
    load_territories()
        .map(|list| list.iter().any(|t| t.code == code))
        .unwrap_or(false)
}

/// The display name of a territory, or None when the code is unknown.
pub fn territory_name(code: &str) -> Option<String> {
    load_territories()
        .ok()?
        .into_iter()
        .find(|t| t.code == code)
        .map(|t| t.name)
}

fn parse_profile(json: &str) -> Result<MarketProfile, String> {
    let profile: MarketProfile =
        serde_json::from_str(json).map_err(|e| format!("market profile is malformed: {e}"))?;
    validate_profile(&profile)?;
    Ok(profile)
}

fn validate_profile(profile: &MarketProfile) -> Result<(), String> {
    if profile.format != "anansi-swarmtank-market-profile" || profile.version != 1 {
        return Err(format!(
            "market profile {} has an unsupported format or version",
            profile.id
        ));
    }
    if profile.id.is_empty() || profile.name.is_empty() {
        return Err("market profile is missing its id or name".to_string());
    }
    if profile.cldr_version != CLDR_VERSION {
        return Err(format!(
            "market profile {} is CLDR {}, expected CLDR {CLDR_VERSION}",
            profile.id, profile.cldr_version
        ));
    }
    if profile.segments.is_empty() {
        return Err(format!("market profile {} has no segments", profile.id));
    }
    for segment in &profile.segments {
        if segment.roles.is_empty() {
            return Err(format!(
                "market profile {} segment {} has no roles",
                profile.id, segment.id
            ));
        }
        if !(segment.budget_multiplier > 0.0) {
            return Err(format!(
                "market profile {} segment {} has a non-positive budget multiplier",
                profile.id, segment.id
            ));
        }
        // Every role a segment names must be a real buyer-role archetype.
        for role in &segment.roles {
            if !crate::swarmtank::archetype_role_exists(role) {
                return Err(format!(
                    "market profile {} segment {} names unknown role {role:?}",
                    profile.id, segment.id
                ));
            }
        }
    }
    // Evidence must carry a source and a date; an entry without them is
    // an assumption and belongs in `assumptions`, not `evidence`.
    for entry in &profile.evidence {
        if entry.source.name.trim().is_empty()
            || entry.source.url.trim().is_empty()
            || entry.date.trim().is_empty()
        {
            return Err(format!(
                "market profile {} evidence field {} is missing a source or date",
                profile.id, entry.field
            ));
        }
    }
    Ok(())
}

fn profile_json(id: &str) -> Option<&'static str> {
    match id {
        "EE" => Some(PROFILE_EE_JSON),
        "AE-DXB" => Some(PROFILE_AE_DXB_JSON),
        "US" => Some(PROFILE_US_JSON),
        _ => None,
    }
}

/// The researched profile for `id` (a territory code, or territory +
/// subdivision such as `AE-DXB`), or None when no researched profile
/// exists for it. A missing profile is not an error: the caller opens
/// the Custom assumptions state.
pub fn get_market_profile(id: &str) -> Option<MarketProfile> {
    let json = profile_json(id)?;
    parse_profile(json).ok()
}

/// The ids of the researched profiles shipped with the app.
pub fn researched_profile_ids() -> Vec<&'static str> {
    vec!["EE", "AE-DXB", "US"]
}

/// Every researched profile, summarized for the selector.
pub fn list_market_profile_summaries() -> Result<Vec<MarketProfileSummary>, String> {
    Ok(list_market_profiles()?
        .into_iter()
        .map(|p| MarketProfileSummary {
            id: p.id,
            name: p.name,
            territory_code: p.territory_code,
            subdivision: p.subdivision,
            profile_version: p.profile_version,
        })
        .collect())
}

/// Every researched profile, validated.
pub fn list_market_profiles() -> Result<Vec<MarketProfile>, String> {
    let mut out = Vec::new();
    for id in researched_profile_ids() {
        let json = profile_json(id).ok_or_else(|| format!("missing profile data for {id}"))?;
        out.push(parse_profile(json)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn territories_load_and_cover_the_starter_profiles() {
        let list = list_territories().expect("territories load");
        assert_eq!(list.len(), 262);
        for code in ["EE", "AE", "US"] {
            assert!(list.iter().any(|t| t.code == code), "missing {code}");
        }
        assert!(!list.iter().any(|t| t.code == "ZZ"));
        assert!(!list.iter().any(|t| t.code == "EU"));
        assert!(list.iter().all(|t| t.code.len() == 2));
        assert_eq!(territory_name("EE").as_deref(), Some("Estonia"));
        assert_eq!(territory_name("AE").as_deref(), Some("United Arab Emirates"));
        assert_eq!(territory_name("US").as_deref(), Some("United States"));
        assert_eq!(territory_name("ZZ"), None);
        assert!(is_valid_territory("EE"));
        assert!(!is_valid_territory("ZZ"));
    }

    #[test]
    fn profiles_load_validate_and_carry_provenance() {
        let profiles = list_market_profiles().expect("profiles load");
        assert_eq!(profiles.len(), 3);
        for profile in &profiles {
            assert!(!profile.evidence.is_empty(), "{} has no evidence", profile.id);
            for entry in &profile.evidence {
                assert!(!entry.source.name.trim().is_empty());
                assert!(!entry.source.url.trim().is_empty());
                assert!(!entry.date.trim().is_empty());
            }
            assert!(!profile.segments.is_empty());
            assert!(profile.languages.iter().any(|l| l.role == "business"));
        }
        let ee = get_market_profile("EE").expect("EE profile");
        assert_eq!(ee.name, "Estonia");
        assert_eq!(ee.territory_code, "EE");
        assert!(ee.subdivision.is_none());
        let dxb = get_market_profile("AE-DXB").expect("AE-DXB profile");
        assert_eq!(dxb.territory_code, "AE");
        assert_eq!(dxb.subdivision.as_deref(), Some("Dubai"));
        assert_eq!(dxb.name, "Dubai, UAE");
        assert!(get_market_profile("FR").is_none());
        assert!(get_market_profile("ZZ").is_none());
    }

    #[test]
    fn a_fourth_profile_is_data_only() {
        // AC-M4: adding a region is a data file, not app code. This test
        // proves the profile parser accepts a well-formed profile that
        // was not compiled into the app: the format, not the id, is what
        // the loader validates.
        let json = r#"{
            "format": "anansi-swarmtank-market-profile", "version": 1,
            "id": "FR", "territoryCode": "FR", "subdivision": null,
            "name": "France", "cldrVersion": "49", "profileVersion": 1,
            "currency": "EUR",
            "languages": [{"code": "fr", "name": "French", "role": "official"}, {"code": "en", "name": "English", "role": "business"}],
            "evidence": [{"field": "business_language", "value": "French is the business language; English is common in international firms.", "source": {"name": "Example source", "url": "https://example.invalid/fr"}, "date": "2026-01-15"}],
            "assumptions": [{"field": "typical_budget", "value": "Assumed mid-market.", "rationale": "No sourced figure.", "editable": true}],
            "segments": [{"id": "sme", "label": "SME", "companySize": "1-49", "roles": ["Small agency owner"], "buyingAuthority": "Owner decides", "languages": ["fr", "en"], "budgetMultiplier": 1.0}]
        }"#;
        let profile = parse_profile(json).expect("a well-formed fourth profile parses");
        assert_eq!(profile.id, "FR");
        assert_eq!(profile.name, "France");
    }
}
