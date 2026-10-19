//! Opt-in native storage of passphrase-wrapped slots only. No default/mock backend.
//! Synchronous calls belong on the adapter's blocking worker, never the UI thread.
use super::{
    Result, VaultError,
    envelope::{ParsedEnvelope, Purpose},
    keys::Context,
};
use std::sync::Mutex;
use zeroize::Zeroizing;

static STORE: Mutex<()> = Mutex::new(());
const SERVICE: &str = "com.robillionair.anansi.vault";

/// Construction requires successful authenticated context decoding first.
pub struct WrappedSlot {
    bytes: Vec<u8>,
    context: Context,
}
impl WrappedSlot {
    pub fn authenticate(context: &Context, bytes: &[u8], password: &[u8]) -> Result<Self> {
        ParsedEnvelope::parse(bytes, Purpose::PassphraseSlot)?;
        let slots = super::keys::KeySlots {
            passphrase: bytes.to_vec(),
            recovery: Vec::new(),
        };
        let _dek = slots.unlock(context, password, false)?;
        Ok(Self {
            bytes: bytes.to_vec(),
            context: context.clone(),
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

fn entry(context: &Context) -> Result<keyring::Entry> {
    #[cfg(windows)]
    let credential =
        keyring::windows::WinCredential::new_with_target(None, SERVICE, context.vault_id());
    #[cfg(target_os = "macos")]
    let credential =
        keyring::macos::MacCredential::new_with_target(None, SERVICE, context.vault_id());
    #[cfg(target_os = "linux")]
    let credential =
        keyring::secret_service::SsCredential::new_with_target(None, SERVICE, context.vault_id());
    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    {
        credential
            .map(|c| keyring::Entry::new_with_credential(Box::new(c)))
            .map_err(|_| VaultError::CredentialUnavailable)
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = context;
        Err(VaultError::CredentialUnavailable)
    }
}

pub fn save(context: &Context, token: &WrappedSlot, consent: bool) -> Result<()> {
    if !consent || token.context != *context {
        return Err(VaultError::InvalidContext);
    }
    let _guard = STORE
        .lock()
        .map_err(|_| VaultError::CredentialUnavailable)?;
    ParsedEnvelope::parse(&token.bytes, Purpose::PassphraseSlot)?;
    entry(context)?
        .set_secret(&token.bytes)
        .map_err(|_| VaultError::CredentialUnavailable)
}

/// Returned ciphertext cannot unlock automatically: passphrase/context auth is required.
pub fn load(context: &Context) -> Result<Zeroizing<Vec<u8>>> {
    let _guard = STORE
        .lock()
        .map_err(|_| VaultError::CredentialUnavailable)?;
    let bytes = Zeroizing::new(
        entry(context)?
            .get_secret()
            .map_err(|_| VaultError::CredentialUnavailable)?,
    );
    ParsedEnvelope::parse(&bytes, Purpose::PassphraseSlot)?;
    Ok(bytes)
}

pub fn forget(context: &Context, consent: bool) -> Result<()> {
    if !consent {
        return Err(VaultError::InvalidContext);
    }
    let _guard = STORE
        .lock()
        .map_err(|_| VaultError::CredentialUnavailable)?;
    entry(context)?
        .delete_credential()
        .map_err(|_| VaultError::CredentialUnavailable)
}
