use anansi_core::{CommandError, Readiness, ReadinessRequest};
fn main() {
    let schemas = serde_json::json!({
        "Readiness":schemars::schema_for!(Readiness),
        "ReadinessRequest":schemars::schema_for!(ReadinessRequest),
        "CommandError":schemars::schema_for!(CommandError)
    });
    println!("{}", serde_json::to_string_pretty(&schemas).unwrap());
}
