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
pub fn run() {
    tauri::Builder::default()
        .manage(Core::default())
        .invoke_handler(tauri::generate_handler![get_readiness])
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
