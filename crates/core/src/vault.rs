//! Unaudited-primitive engineering under the explicit Owner amendment.
//! Not exposed to either application adapter until integrated certification.
pub mod credential_store;
pub mod envelope;
pub mod keys;
pub mod migration;
pub mod policy;
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
}
impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vault operation failed: {self:?}")
    }
}
impl std::error::Error for VaultError {}
pub type Result<T> = std::result::Result<T, VaultError>;
