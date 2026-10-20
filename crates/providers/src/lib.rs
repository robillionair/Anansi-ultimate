//! ANANSI M5 — AI Matrix provider-integration package.
//!
//! Built on baseline `5b7a87b` (branch `muse/m5-provider-integration`).
//! This crate is deliberately dependency-light, mirroring `anansi-core`:
//! serde/serde_json only, hand-rolled bounded HTTP over `std::net`,
//! socket-shutdown cancellation (same pattern as `anansi-core` Ask runs).

pub mod catalog;
pub mod config;
pub mod dispatcher;
pub mod error;
pub mod gates;
pub mod http;
pub mod providers;
pub mod redact;
pub mod routing;
pub mod secrets;
pub mod stream;
pub mod types;
