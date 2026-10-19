//! Exact 130-byte authenticated context keyslots. No secret serialization traits.
use super::{
    Result, VaultError,
    envelope::{self, KdfProfile, ParsedEnvelope, Purpose},
};
use zeroize::Zeroizing;

#[derive(Clone, PartialEq, Eq)]
pub struct Context {
    vault: [u8; 32],
    workspace: [u8; 32],
    generation: [u8; 32],
}
impl Context {
    pub fn new(vault: &str, workspace: &str, generation: &str) -> Result<Self> {
        fn id(s: &str) -> Result<[u8; 32]> {
            if s.len() != 32
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(VaultError::InvalidContext);
            }
            Ok(s.as_bytes().try_into().expect("validated ID"))
        }
        Ok(Self {
            vault: id(vault)?,
            workspace: id(workspace)?,
            generation: id(generation)?,
        })
    }
    pub fn vault_id(&self) -> &str {
        std::str::from_utf8(&self.vault).expect("validated ID")
    }
    pub fn workspace_id(&self) -> &str {
        std::str::from_utf8(&self.workspace).expect("validated ID")
    }
    pub fn generation_id(&self) -> &str {
        std::str::from_utf8(&self.generation).expect("validated ID")
    }
    fn payload(&self, dek: &[u8; 32], purpose: Purpose) -> Result<Zeroizing<[u8; 130]>> {
        let mut out = Zeroizing::new([0; 130]);
        out[0] = 1;
        out[1] = match purpose {
            Purpose::PassphraseSlot => 1,
            Purpose::RecoverySlot => 2,
            _ => return Err(VaultError::InvalidContext),
        };
        out[2..34].copy_from_slice(&self.vault);
        out[34..66].copy_from_slice(&self.workspace);
        out[66..98].copy_from_slice(&self.generation);
        out[98..].copy_from_slice(dek);
        Ok(out)
    }
    fn decode(&self, plain: &[u8], purpose: Purpose) -> Result<Zeroizing<[u8; 32]>> {
        // Compare context independently of tag validity: valid ciphertext may be swapped.
        let expected = self.payload(&[0; 32], purpose)?;
        if plain.len() != 130 || plain[..98] != expected[..98] {
            return Err(VaultError::InvalidContext);
        }
        Ok(Zeroizing::new(
            plain[98..].try_into().expect("validated keyslot"),
        ))
    }
}

pub struct KeySlots {
    pub passphrase: Vec<u8>,
    pub recovery: Vec<u8>,
}
impl KeySlots {
    pub fn initialize(context: &Context, password: &[u8]) -> Result<(Self, Zeroizing<[u8; 32]>)> {
        let dek = envelope::random::<32>()?;
        let recovery = envelope::random::<32>()?;
        let slots = Self::wrap(context, &dek, password, &recovery)?;
        Ok((slots, recovery))
    }
    fn wrap(
        context: &Context,
        dek: &[u8; 32],
        password: &[u8],
        recovery: &[u8; 32],
    ) -> Result<Self> {
        let passphrase = Self::wrap_passphrase(context, dek, password)?;
        let salt = envelope::random::<16>()?;
        let payload = context.payload(dek, Purpose::RecoverySlot)?;
        let wrapped_recovery = envelope::seal(
            recovery,
            Purpose::RecoverySlot,
            &*payload,
            KdfProfile::default(),
            &salt,
        )?;
        // Read-back verification before the caller may persist/publish these slots.
        let verified = ParsedEnvelope::parse(&wrapped_recovery, Purpose::RecoverySlot)?;
        let _checked = context.decode(&verified.open(recovery)?, Purpose::RecoverySlot)?;
        Ok(Self {
            passphrase,
            recovery: wrapped_recovery,
        })
    }
    pub fn wrap_passphrase(context: &Context, dek: &[u8; 32], password: &[u8]) -> Result<Vec<u8>> {
        let salt = envelope::random::<16>()?;
        let key = envelope::derive_key(password, &salt, KdfProfile::default(), false)?;
        let payload = context.payload(dek, Purpose::PassphraseSlot)?;
        let slot = envelope::seal(
            &key,
            Purpose::PassphraseSlot,
            &*payload,
            KdfProfile::default(),
            &salt,
        )?;
        let decoded = ParsedEnvelope::parse(&slot, Purpose::PassphraseSlot)?.open(&key)?;
        let _checked = context.decode(&decoded, Purpose::PassphraseSlot)?;
        Ok(slot)
    }
    pub fn unlock(
        &self,
        context: &Context,
        password: &[u8],
        allow_high_cost: bool,
    ) -> Result<Zeroizing<[u8; 32]>> {
        let plain = envelope::open_passphrase(
            &self.passphrase,
            Purpose::PassphraseSlot,
            password,
            allow_high_cost,
        )?;
        context.decode(&plain, Purpose::PassphraseSlot)
    }
    pub fn recover(&self, context: &Context, recovery: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>> {
        let plain = ParsedEnvelope::parse(&self.recovery, Purpose::RecoverySlot)?.open(recovery)?;
        context.decode(&plain, Purpose::RecoverySlot)
    }
    /// Builds verified replacement slots; caller must fence persistent publication.
    /// Does not mutate original slots if entropy/KDF/authentication fails.
    pub fn rotate_after_recovery(
        &self,
        context: &Context,
        old_recovery: &[u8; 32],
        new_password: &[u8],
    ) -> Result<(Self, Zeroizing<[u8; 32]>)> {
        let dek = self.recover(context, old_recovery)?;
        let recovery = envelope::random::<32>()?;
        Ok((
            Self::wrap(context, &dek, new_password, &recovery)?,
            recovery,
        ))
    }
}
