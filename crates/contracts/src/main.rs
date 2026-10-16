use anansi_core::workspace::{
    CreateWorkspaceRequest, OpenWorkspaceRequest, UpdateSettingsRequest, WorkspaceDetails,
    WorkspaceList,
};
use anansi_core::{CommandError, Readiness, ReadinessRequest};
fn main() {
    let schemas = serde_json::json!({
        "Readiness":schemars::schema_for!(Readiness),
        "ReadinessRequest":schemars::schema_for!(ReadinessRequest),
        "CommandError":schemars::schema_for!(CommandError),
        "WorkspaceDetails":schemars::schema_for!(WorkspaceDetails),
        "WorkspaceList":schemars::schema_for!(WorkspaceList),
        "CreateWorkspaceRequest":schemars::schema_for!(CreateWorkspaceRequest),
        "OpenWorkspaceRequest":schemars::schema_for!(OpenWorkspaceRequest),
        "UpdateSettingsRequest":schemars::schema_for!(UpdateSettingsRequest)
    });
    println!("{}", serde_json::to_string_pretty(&schemas).unwrap());
}
