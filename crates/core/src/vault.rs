//! Unaudited-primitive engineering under the explicit Owner amendment.
//! Not exposed to either application adapter until integrated certification.
pub mod admission;
pub mod archive;
pub mod catalog;
pub mod credential_store;
pub mod envelope;
pub(crate) mod io;
pub(crate) mod json;
pub mod keys;
pub mod migration;
pub mod payload;
pub mod policy;
pub mod recovery;
pub mod runtime;
pub mod service;
pub mod session;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultError {
    InvalidEnvelope,
    UnsupportedVersion,
    UnsupportedAlgorithm,
    ResourceLimit,
    AuthenticationFailed,
    EntropyUnavailable,
    Busy,
    Locked,
    InvalidContext,
    CredentialUnavailable,
    StorageUnavailable,
    Conflict,
    AcknowledgementRequired,
    UnsupportedSchema,
}
impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vault operation failed: {self:?}")
    }
}
impl std::error::Error for VaultError {}
pub type Result<T> = std::result::Result<T, VaultError>;
