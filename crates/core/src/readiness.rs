use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadinessRequest {
    pub contract_version: u8,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Readiness {
    pub contract_version: u8,
    pub application_name: String,
    pub application_version: String,
    pub api_version: Option<String>,
    pub core_version: String,
    pub operating_system: String,
    pub uptime_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidRequest,
    UnsupportedContract,
    Unauthorized,
    Forbidden,
    NotFound,
    InvalidWorkspace,
    WorkspaceExists,
    WorkspaceNotFound,
    StorageUnavailable,
    UnsupportedSchema,
    CorruptStorage,
    InvalidMemory,
    InvalidSearch,
    MemoryNotFound,
    RevisionConflict,
    ContentIntegrity,
    VaultUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CommandError {
    pub code: ErrorCode,
    pub message: String,
}

impl CommandError {
    pub fn new(code: ErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone)]
pub struct Core {
    started: Instant,
    workspace_storage: Result<crate::storage::Storage, CommandError>,
}
impl Default for Core {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            workspace_storage: Err(crate::storage::unavailable()),
        }
    }
}
impl Core {
    pub fn with_storage_root(root: std::path::PathBuf) -> Self {
        Self {
            started: Instant::now(),
            workspace_storage: crate::storage::Storage::new(root),
        }
    }
    pub fn for_host() -> Self {
        let configured = std::env::var_os("ANANSI_STORAGE_ROOT").map(std::path::PathBuf::from);
        let root = configured.or_else(|| {
            if cfg!(windows) {
                std::env::var_os("LOCALAPPDATA")
                    .map(|p| std::path::PathBuf::from(p).join("ANANSI/workspaces"))
            } else {
                std::env::var_os("HOME")
                    .map(|p| std::path::PathBuf::from(p).join(".local/share/anansi/workspaces"))
            }
        });
        let core = root.map(Self::with_storage_root).unwrap_or_default();
        core.wake_export_worker();
        core.start_backup_scheduler();
        core
    }
    pub(crate) fn storage(&self) -> Result<&crate::storage::Storage, CommandError> {
        self.workspace_storage
            .as_ref()
            .map_err(|_| crate::storage::unavailable())
    }

    /// Returns the configured local storage root for trusted native adapters.
    /// This path is never exposed to browser clients.
    pub fn storage_root(&self) -> Result<std::path::PathBuf, CommandError> {
        Ok(self.storage()?.root_path().to_path_buf())
    }
    pub fn readiness(&self, input: serde_json::Value) -> Result<Readiness, CommandError> {
        let request: ReadinessRequest = serde_json::from_value(input).map_err(|_| {
            CommandError::new(
                ErrorCode::InvalidRequest,
                "Use the supported readiness request.",
            )
        })?;
        if request.contract_version != 1 {
            return Err(CommandError::new(
                ErrorCode::UnsupportedContract,
                "This contract version is not supported.",
            ));
        }
        Ok(Readiness {
            contract_version: 1,
            application_name: "ANANSI Ultimate".into(),
            application_version: env!("CARGO_PKG_VERSION").into(),
            api_version: None,
            core_version: env!("CARGO_PKG_VERSION").into(),
            operating_system: std::env::consts::OS.into(),
            uptime_ms: self
                .started
                .elapsed()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn readiness_is_live_and_uses_build_metadata() {
        let core = Core::default();
        let r = core.readiness(json!({"contractVersion":1})).unwrap();
        assert_eq!(r.core_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(r.operating_system, std::env::consts::OS);
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(
            core.readiness(json!({"contractVersion":1}))
                .unwrap()
                .uptime_ms
                > r.uptime_ms
        );
    }
    #[test]
    fn input_rejects_unknown_fields_types_and_versions() {
        let core = Core::default();
        for input in [
            json!({}),
            json!({"contractVersion":"1"}),
            json!({"contractVersion":1,"path":"C:/"}),
            json!(null),
        ] {
            assert_eq!(
                core.readiness(input).unwrap_err().code,
                ErrorCode::InvalidRequest
            );
        }
        assert_eq!(
            core.readiness(json!({"contractVersion":2}))
                .unwrap_err()
                .code,
            ErrorCode::UnsupportedContract
        );
    }
}
