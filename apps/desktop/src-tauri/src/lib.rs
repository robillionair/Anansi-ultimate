use anansi_core::{CommandError, Core, Readiness};
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
pub fn run() {
    tauri::Builder::default()
        .manage(Core::for_host())
        .invoke_handler(tauri::generate_handler![get_readiness, workspace_command])
        .run(tauri::generate_context!())
        .expect("ANANSI desktop failed to start");
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
