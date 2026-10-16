use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSettings {
    pub reduced_motion: bool,
    pub intent: Intent,
    pub onboarding_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    SecondMemory,
    InventProduct,
    BuildApplication,
    ConnectAi,
    Explore,
}
impl Intent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SecondMemory => "second_memory",
            Self::InventProduct => "invent_product",
            Self::BuildApplication => "build_application",
            Self::ConnectAi => "connect_ai",
            Self::Explore => "explore",
        }
    }
}
