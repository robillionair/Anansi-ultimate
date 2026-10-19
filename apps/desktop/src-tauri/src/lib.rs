use anansi_core::{CommandError, Core, Readiness};
#[cfg(windows)]
mod vault_prompt;
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
pub fn run() {
    tauri::Builder::default()
        .manage(Core::for_host())
        .invoke_handler(tauri::generate_handler![
            get_readiness,
            workspace_command,
            memory_command,
            relationship_command,
            job_command,
            save_export
        ])
        .run(tauri::generate_context!())
        .expect("ANANSI desktop failed to start");
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
