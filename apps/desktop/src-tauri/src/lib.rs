use anansi_core::{CommandError, Core, Readiness};
#[cfg(windows)]
mod vault_prompt;
#[cfg(windows)]
use anansi_core::vault::{
    VaultError,
    catalog::BackupReference,
    keys::Context,
    service::VaultService,
    session::{Caller, Transport},
};
#[cfg(windows)]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

#[cfg(windows)]
struct DesktopSession {
    caller: Caller,
    token: zeroize::Zeroizing<[u8; 16]>,
}

#[cfg(windows)]
struct DesktopVaultState {
    service: VaultService,
    sessions: Arc<Mutex<HashMap<String, DesktopSession>>>,
}

#[cfg(windows)]
fn vault_error(error: VaultError) -> CommandError {
    use anansi_core::ErrorCode;
    let (code, message) = match error {
        VaultError::Locked | VaultError::AuthenticationFailed => (
            ErrorCode::Unauthorized,
            "The vault is locked or the credential was rejected.",
        ),
        VaultError::Busy => (
            ErrorCode::VaultUnavailable,
            "The vault is busy. Retry after the current operation finishes.",
        ),
        VaultError::AcknowledgementRequired => (
            ErrorCode::InvalidRequest,
            "Recovery acknowledgement and a verified backup are required.",
        ),
        VaultError::InvalidContext => (
            ErrorCode::Forbidden,
            "The vault request does not match this workspace or application process.",
        ),
        VaultError::UnsupportedSchema => (
            ErrorCode::UnsupportedSchema,
            "The vault storage schema is not supported.",
        ),
        _ => (
            ErrorCode::VaultUnavailable,
            "The vault operation failed without changing canonical data.",
        ),
    };
    CommandError::new(code, message)
}

#[cfg(windows)]
fn text(value: &serde_json::Value, name: &str) -> Result<String, CommandError> {
    value
        .get(name)
        .and_then(serde_json::Value::as_str)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            CommandError::new(
                anansi_core::ErrorCode::InvalidRequest,
                "Use the supported vault request.",
            )
        })
}

#[cfg(windows)]
fn setup_caller(
    service: &VaultService,
    workspace: &str,
    root: &std::path::Path,
) -> Result<Caller, CommandError> {
    let root_id = anansi_core::content::hash(
        root.canonicalize()
            .map_err(|_| anansi_core::storage::unavailable())?
            .to_string_lossy()
            .as_bytes(),
    );
    let context = service.context(workspace).map(|v| v.0).unwrap_or_else(|_| {
        Context::new(
            "00000000000000000000000000000000",
            workspace,
            "00000000000000000000000000000000",
        )
        .expect("fixed context")
    });
    Ok(Caller {
        context,
        transport: Transport::Desktop,
        root_id,
        caller_id: "desktop-main".into(),
    })
}
pub fn desktop_readiness(
    core: &Core,
    request: serde_json::Value,
) -> Result<Readiness, CommandError> {
    core.readiness(request)
}
#[cfg(windows)]
#[tauri::command]
fn get_readiness(
    core: tauri::State<'_, Core>,
    request: serde_json::Value,
) -> Result<Readiness, CommandError> {
    desktop_readiness(&core, request)
}
#[cfg(windows)]
#[tauri::command]
async fn workspace_command(
    core: tauri::State<'_, Core>,
    command: String,
    request: serde_json::Value,
) -> Result<serde_json::Value, CommandError> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core.workspace_command(&command, request))
        .await
        .map_err(|_| anansi_core::storage::unavailable())?
}
#[cfg(windows)]
#[tauri::command]
async fn memory_command(
    core: tauri::State<'_, Core>,
    command: String,
    request: serde_json::Value,
) -> Result<serde_json::Value, CommandError> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core.memory_command(&command, request))
        .await
        .map_err(|_| anansi_core::storage::unavailable())?
}
#[cfg(windows)]
#[tauri::command]
async fn relationship_command(
    core: tauri::State<'_, Core>,
    command: String,
    request: serde_json::Value,
) -> Result<serde_json::Value, CommandError> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core.relationship_command(&command, request))
        .await
        .map_err(|_| anansi_core::storage::unavailable())?
}
#[cfg(windows)]
#[tauri::command]
async fn ai_command(
    core: tauri::State<'_, Core>,
    command: String,
    request: serde_json::Value,
) -> Result<serde_json::Value, CommandError> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core.ai_command(&command, request))
        .await
        .map_err(|_| anansi_core::storage::unavailable())?
}
#[cfg(windows)]
pub fn run() {
    let core = Core::for_host();
    let root = core
        .storage_root()
        .expect("ANANSI storage root unavailable");
    if !root.join(".anansi-root").exists() {
        core.workspace_command("list", serde_json::json!({"contractVersion":1}))
            .expect("ANANSI storage initialization failed");
    }
    let namespace = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("ANANSI/vault-admission");
    std::fs::create_dir_all(&namespace).expect("ANANSI vault admission directory unavailable");
    anansi_core::storage::private_permissions(&namespace, true, false)
        .expect("ANANSI vault admission permissions unavailable");
    let vault = DesktopVaultState {
        service: VaultService::new(root, namespace).expect("ANANSI vault service unavailable"),
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };
    tauri::Builder::default()
        .manage(core)
        .manage(vault)
        .invoke_handler(tauri::generate_handler![
            get_readiness,
            workspace_command,
            memory_command,
            relationship_command,
            job_command,
            vault_command,
            ai_command,
            save_export
        ])
        .run(tauri::generate_context!())
        .expect("ANANSI desktop failed to start");
}

#[cfg(windows)]
#[tauri::command]
async fn vault_command(
    core: tauri::State<'_, Core>,
    state: tauri::State<'_, DesktopVaultState>,
    command: String,
    request: serde_json::Value,
) -> Result<serde_json::Value, CommandError> {
    let core = core.inner().clone();
    let service = state.service.clone();
    let state_handle = state.inner().sessions.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let workspace = text(&request, "workspaceId")?;
        let root = core.storage_root()?;
        match command.as_str() {
            "setup" => {
                let caller = setup_caller(&service, &workspace, &root)?;
                let mut provider = vault_prompt::WindowsCredentialProvider::new(&workspace);
                service
                    .setup(&workspace, &caller, &mut provider, Instant::now(), true)
                    .map_err(vault_error)?;
                Ok(serde_json::json!({"contractVersion":1,"status":"acknowledged"}))
            }
            "seal" => {
                let backup = BackupReference {
                    job_id: text(&request, "jobId")?,
                    artifact_id: text(&request, "artifactId")?,
                    sha256: text(&request, "sha256")?,
                };
                service.seal_workspace(&backup, true).map_err(vault_error)?;
                Ok(serde_json::json!({"contractVersion":1,"status":"sealed"}))
            }
            "unlock" => {
                let caller = setup_caller(&service, &workspace, &root)?;
                let mut provider = vault_prompt::WindowsCredentialProvider::new(&workspace);
                let token = service
                    .unlock(&caller, &mut provider, Instant::now())
                    .map_err(vault_error)?;
                state_handle
                    .lock()
                    .map_err(|_| anansi_core::storage::unavailable())?
                    .insert(workspace, DesktopSession { caller, token });
                Ok(serde_json::json!({"contractVersion":1,"status":"unlocked"}))
            }
            "read" => {
                let guard = state_handle
                    .lock()
                    .map_err(|_| anansi_core::storage::unavailable())?;
                let session = guard
                    .get(&workspace)
                    .ok_or_else(|| vault_error(VaultError::Locked))?;
                let snapshot = service
                    .read(&session.caller, &session.token, Instant::now())
                    .map_err(vault_error)?;
                serde_json::to_value(snapshot).map_err(|_| anansi_core::storage::unavailable())
            }
            "lock" => {
                let epoch = service.lock(&workspace).map_err(vault_error)?;
                state_handle
                    .lock()
                    .map_err(|_| anansi_core::storage::unavailable())?
                    .remove(&workspace);
                Ok(serde_json::json!({"contractVersion":1,"status":"locked","epoch":epoch}))
            }
            _ => Err(CommandError::new(
                anansi_core::ErrorCode::InvalidRequest,
                "Use a supported vault command.",
            )),
        }
    })
    .await
    .map_err(|_| anansi_core::storage::unavailable())?
}

#[cfg(windows)]
#[tauri::command]
async fn job_command(
    core: tauri::State<'_, Core>,
    command: String,
    request: serde_json::Value,
) -> Result<serde_json::Value, CommandError> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core.job_command(&command, request))
        .await
        .map_err(|_| anansi_core::storage::unavailable())?
}

#[cfg(windows)]
#[tauri::command]
async fn save_export(
    core: tauri::State<'_, Core>,
    request: serde_json::Value,
) -> Result<bool, CommandError> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        use std::os::windows::process::CommandExt;
        let parsed: anansi_core::jobs::ReadArtifactRequest =
            serde_json::from_value(request.clone())
                .map_err(|_| anansi_core::jobs::request_error())?;
        if parsed.offset != 0 {
            return Err(anansi_core::jobs::request_error());
        }
        // Validate ownership/completion before opening any trusted system dialog.
        core.job_command("artifact", request)?;
        let output = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-STA",
                "-Command",
                include_str!("export-save.ps1"),
            ])
            .creation_flags(0x08000000)
            .env("ANANSI_EXPORT_PARENT_PID", std::process::id().to_string())
            .output()
            .map_err(|_| anansi_core::storage::unavailable())?;
        if !output.status.success() {
            return Err(anansi_core::storage::unavailable());
        }
        let chosen =
            String::from_utf8(output.stdout).map_err(|_| anansi_core::storage::unavailable())?;
        if chosen.is_empty() {
            return Ok(false);
        }
        core.save_complete_export(&parsed, std::path::Path::new(&chosen))?;
        Ok(true)
    })
    .await
    .map_err(|_| anansi_core::storage::unavailable())?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_adapter_delegates_to_the_same_validated_core() {
        let core = Core::default();
        assert_eq!(
            desktop_readiness(&core, serde_json::json!({"contractVersion":1}))
                .unwrap()
                .core_version,
            env!("CARGO_PKG_VERSION")
        );
        for input in [
            serde_json::json!({"contractVersion":2}),
            serde_json::json!({"contractVersion":1,"path":"forbidden"}),
        ] {
            assert_eq!(
                desktop_readiness(&core, input.clone()),
                core.readiness(input)
            );
        }
    }
}
