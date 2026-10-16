use crate::{CommandError, Core, ErrorCode, ReadinessRequest, settings::WorkspaceSettings};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Workspace {
    pub id: String,
    pub name: String,
}
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceDetails {
    pub contract_version: u8,
    pub schema_version: u32,
    pub workspace: Workspace,
    pub settings: WorkspaceSettings,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceList {
    pub contract_version: u8,
    pub workspaces: Vec<Workspace>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateWorkspaceRequest {
    pub contract_version: u8,
    pub name: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenWorkspaceRequest {
    pub contract_version: u8,
    pub id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSettingsRequest {
    pub contract_version: u8,
    pub id: String,
    pub reduced_motion: bool,
}
fn input<T: DeserializeOwned>(value: serde_json::Value) -> Result<T, CommandError> {
    if value
        .get("contractVersion")
        .and_then(serde_json::Value::as_u64)
        .is_some_and(|version| version != 1)
    {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "This contract version is not supported.",
        ));
    }
    serde_json::from_value(value).map_err(|_| {
        CommandError::new(
            ErrorCode::InvalidRequest,
            "Use the supported workspace request.",
        )
    })
}
pub(crate) fn validate_name(name: &str) -> Result<(), CommandError> {
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if name.is_empty()
        || name.len() > 64
        || name.trim() != name
        || !name.starts_with(|c: char| c.is_ascii_alphanumeric())
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-'))
        || reserved.contains(&name.to_ascii_uppercase().as_str())
    {
        return Err(CommandError::new(
            ErrorCode::InvalidWorkspace,
            "Use 1–64 letters, numbers, spaces, underscores or hyphens; start with a letter or number.",
        ));
    }
    Ok(())
}
pub(crate) fn validate_id(id: &str) -> Result<(), CommandError> {
    if id.len() != 32
        || !id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(CommandError::new(
            ErrorCode::InvalidWorkspace,
            "Choose a workspace from the local list.",
        ));
    }
    Ok(())
}
impl Core {
    pub fn workspace_command(
        &self,
        command: &str,
        value: serde_json::Value,
    ) -> Result<serde_json::Value, CommandError> {
        // Validate requests before accessing storage; neither names nor IDs become paths.
        match command {
            "list" => {
                let _: ReadinessRequest = input(value)?;
                serde_json::to_value(WorkspaceList {
                    contract_version: 1,
                    workspaces: self.storage()?.list()?,
                })
                .map_err(|_| crate::storage::unavailable())
            }
            "create" => {
                let request: CreateWorkspaceRequest = input(value)?;
                validate_name(&request.name)?;
                serde_json::to_value(self.storage()?.create(&request.name)?)
                    .map_err(|_| crate::storage::unavailable())
            }
            "open" => {
                let request: OpenWorkspaceRequest = input(value)?;
                validate_id(&request.id)?;
                serde_json::to_value(self.storage()?.open(&request.id)?)
                    .map_err(|_| crate::storage::unavailable())
            }
            "settings" => {
                let request: UpdateSettingsRequest = input(value)?;
                validate_id(&request.id)?;
                serde_json::to_value(
                    self.storage()?
                        .update(&request.id, request.reduced_motion)?,
                )
                .map_err(|_| crate::storage::unavailable())
            }
            _ => Err(CommandError::new(ErrorCode::NotFound, "Command not found.")),
        }
    }
}
