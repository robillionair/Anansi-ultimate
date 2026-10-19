//! ANSV v1 parser and authenticated encryption. No paths or renderer secrets.
use super::{Result, VaultError};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{AeadInOut, KeyInit, Tag, XChaCha20Poly1305, XNonce};
use std::sync::Mutex;
use zeroize::Zeroizing;

pub const HEADER_LEN: usize = 64;
pub const MAX_RECORD_BYTES: usize = 1024 * 1024;
pub const MAX_JOURNAL_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ARCHIVE_BYTES: usize = 256 * 1024 * 1024;
static KDF: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfProfile {
    pub memory_kib: u32,
    pub iterations: u32,
    pub lanes: u32,
}
impl Default for KdfProfile {
    fn default() -> Self {
        Self {
            memory_kib: 65536,
            iterations: 3,
            lanes: 4,
        }
    }
}
impl KdfProfile {
    pub fn validate(self) -> Result<()> {
        if !(19456..=1048576).contains(&self.memory_kib)
            || !(1..=16).contains(&self.iterations)
            || !(1..=16).contains(&self.lanes)
        {
            return Err(VaultError::InvalidEnvelope);
        }
        Params::new(self.memory_kib, self.iterations, self.lanes, Some(32))
            .map_err(|_| VaultError::InvalidEnvelope)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    PassphraseSlot,
    RecoverySlot,
    Record,
    Journal,
    Archive,
}
impl Purpose {
    fn mode(self) -> (u8, u8) {
        match self {
            Self::PassphraseSlot => (0, 1),
            Self::RecoverySlot => (4, 0),
            Self::Record => (1, 0),
            Self::Journal => (0, 0),
            Self::Archive => (2, 1),
        }
    }
    fn limit(self) -> usize {
        match self {
            Self::PassphraseSlot | Self::RecoverySlot => 130,
            Self::Record => MAX_RECORD_BYTES,
            Self::Journal => MAX_JOURNAL_BYTES,
            Self::Archive => MAX_ARCHIVE_BYTES,
        }
    }
}

pub struct ParsedEnvelope<'a> {
    pub profile: KdfProfile,
    pub salt: &'a [u8; 16],
    pub nonce: &'a [u8; 24],
    header: &'a [u8],
    ciphertext: &'a [u8],
    tag: &'a [u8],
}
impl<'a> ParsedEnvelope<'a> {
    /// Caller's expected purpose is mandatory, rather than trusting attacker flags.
    pub fn parse(bytes: &'a [u8], expected: Purpose) -> Result<Self> {
        if bytes.len() < 80 || &bytes[..4] != b"ANSV" {
            return Err(VaultError::InvalidEnvelope);
        }
        if bytes[4] != 1 {
            return Err(VaultError::UnsupportedVersion);
        }
        if bytes[5] != 1 {
            return Err(VaultError::UnsupportedAlgorithm);
        }
        let (flags, kdf) = expected.mode();
        if bytes[6] != kdf || bytes[7] != flags {
            return Err(VaultError::InvalidEnvelope);
        }
        let u32_at = |i| u32::from_be_bytes(bytes[i..i + 4].try_into().expect("checked header"));
        let profile = KdfProfile {
            memory_kib: u32_at(8),
            iterations: u32_at(12),
            lanes: u32_at(16),
        };
        profile.validate()?;
        if kdf == 0 && profile != KdfProfile::default() {
            return Err(VaultError::InvalidEnvelope);
        }
        let size = usize::try_from(u32_at(60)).map_err(|_| VaultError::ResourceLimit)?;
        if size > expected.limit() {
            return Err(VaultError::ResourceLimit);
        }
        if matches!(expected, Purpose::PassphraseSlot | Purpose::RecoverySlot) && size != 130 {
            return Err(VaultError::InvalidEnvelope);
        }
        if size.checked_add(80) != Some(bytes.len()) {
            return Err(VaultError::InvalidEnvelope);
        }
        let salt: &[u8; 16] = bytes[20..36].try_into().expect("checked header");
        if salt.iter().all(|b| *b == 0) {
            return Err(VaultError::InvalidEnvelope);
        }
        Ok(Self {
            profile,
            salt,
            nonce: bytes[36..60].try_into().expect("checked header"),
            header: &bytes[..64],
            ciphertext: &bytes[64..64 + size],
            tag: &bytes[64 + size..],
        })
    }
    /// No plaintext escapes on authentication failure; temporary copies zeroize.
    pub fn open(&self, key: &[u8; 32]) -> Result<Zeroizing<Vec<u8>>> {
        let mut plain = Zeroizing::new(self.ciphertext.to_vec());
        let cipher = XChaCha20Poly1305::new(key.into());
        cipher
            .decrypt_inout_detached(
                &XNonce::from(*self.nonce),
                self.header,
                plain.as_mut_slice().into(),
                &Tag::try_from(self.tag).expect("checked tag"),
            )
            .map_err(|_| VaultError::AuthenticationFailed)?;
        Ok(plain)
    }
}

/// Strict bounded parser always runs before allocating Argon2 working memory.
/// Higher imported costs require explicit caller admission. Busy fails immediately:
/// no unbounded queue of passwords is retained by this module.
pub fn derive_key(
    password: &[u8],
    salt: &[u8; 16],
    profile: KdfProfile,
    allow_high_cost: bool,
) -> Result<Zeroizing<[u8; 32]>> {
    profile.validate()?;
    if password.is_empty() || password.len() > 4096 || salt.iter().all(|b| *b == 0) {
        return Err(VaultError::InvalidEnvelope);
    }
    if !allow_high_cost
        && (profile.memory_kib > 65536 || profile.iterations > 3 || profile.lanes > 4)
    {
        return Err(VaultError::ResourceLimit);
    }
    let _guard = KDF.try_lock().map_err(|_| VaultError::Busy)?;
    let params = Params::new(
        profile.memory_kib,
        profile.iterations,
        profile.lanes,
        Some(32),
    )
    .map_err(|_| VaultError::InvalidEnvelope)?;
    let mut output = Zeroizing::new([0; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password, salt, &mut *output)
        .map_err(|_| VaultError::ResourceLimit)?;
    Ok(output)
}

pub fn open_passphrase(
    bytes: &[u8],
    purpose: Purpose,
    password: &[u8],
    allow_high_cost: bool,
) -> Result<Zeroizing<Vec<u8>>> {
    if !matches!(purpose, Purpose::PassphraseSlot | Purpose::Archive) {
        return Err(VaultError::InvalidEnvelope);
    }
    let parsed = ParsedEnvelope::parse(bytes, purpose)?;
    let key = derive_key(password, parsed.salt, parsed.profile, allow_high_cost)?;
    parsed.open(&key)
}

pub fn random<const N: usize>() -> Result<Zeroizing<[u8; N]>> {
    let mut out = Zeroizing::new([0; N]);
    getrandom::fill(&mut *out).map_err(|_| VaultError::EntropyUnavailable)?;
    Ok(out)
}

pub fn seal(
    key: &[u8; 32],
    purpose: Purpose,
    plaintext: &[u8],
    profile: KdfProfile,
    salt: &[u8; 16],
) -> Result<Vec<u8>> {
    let nonce = random::<24>()?;
    seal_with_nonce(key, purpose, plaintext, profile, salt, &nonce)
}

// Deterministic nonce injection is deliberately private; production uses OS entropy.
fn seal_with_nonce(
    key: &[u8; 32],
    purpose: Purpose,
    plaintext: &[u8],
    profile: KdfProfile,
    salt: &[u8; 16],
    nonce: &[u8; 24],
) -> Result<Vec<u8>> {
    if plaintext.len() > purpose.limit() {
        return Err(VaultError::ResourceLimit);
    }
    let mut header = [0; 64];
    header[..4].copy_from_slice(b"ANSV");
    header[4] = 1;
    header[5] = 1;
    let (flags, kdf) = purpose.mode();
    header[6] = kdf;
    header[7] = flags;
    for (at, value) in [
        (8, profile.memory_kib),
        (12, profile.iterations),
        (16, profile.lanes),
        (
            60,
            u32::try_from(plaintext.len()).map_err(|_| VaultError::ResourceLimit)?,
        ),
    ] {
        header[at..at + 4].copy_from_slice(&value.to_be_bytes());
    }
    header[20..36].copy_from_slice(salt);
    header[36..60].copy_from_slice(nonce);
    let mut result = Vec::with_capacity(80 + plaintext.len());
    result.extend_from_slice(&header);
    result.resize(80 + plaintext.len(), 0);
    ParsedEnvelope::parse(&result, purpose)?;
    // Cipher operates on guarded memory; copy only ciphertext into the output.
    let mut buffer = Zeroizing::new(plaintext.to_vec());
    let tag = XChaCha20Poly1305::new(key.into())
        .encrypt_inout_detached(&XNonce::from(*nonce), &header, buffer.as_mut_slice().into())
        .map_err(|_| VaultError::AuthenticationFailed)?;
    result[64..64 + buffer.len()].copy_from_slice(&buffer);
    result[64 + buffer.len()..].copy_from_slice(&tag);
    Ok(result)
}
