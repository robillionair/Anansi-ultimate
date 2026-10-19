//! RFC4648 encoding and native-only setup acknowledgement. No serde/debug secrets.
use super::{
    Result, VaultError, envelope,
    keys::{Context, KeySlots},
};
use zeroize::Zeroizing;
const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const GROUPS: [usize; 8] = [7, 7, 7, 7, 6, 6, 6, 6];

pub fn format(key: &[u8; 32]) -> Zeroizing<String> {
    let mut encoded = Zeroizing::new(Vec::with_capacity(52));
    let mut buffer = 0u16;
    let mut bits = 0;
    for byte in key {
        buffer = (buffer << 8) | u16::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            encoded.push(ALPHABET[usize::from((buffer >> bits) & 31)]);
        }
    }
    if bits > 0 {
        encoded.push(ALPHABET[usize::from((buffer << (5 - bits)) & 31)]);
    }
    let mut text = Zeroizing::new(String::with_capacity(59));
    let mut at = 0;
    for (i, size) in GROUPS.iter().enumerate() {
        if i > 0 {
            text.push('-');
        }
        text.push_str(std::str::from_utf8(&encoded[at..at + size]).expect("ASCII alphabet"));
        at += size;
    }
    text
}
pub fn parse(text: &str) -> Result<Zeroizing<[u8; 32]>> {
    if text.len() != 59 {
        return Err(VaultError::InvalidEnvelope);
    }
    let groups: Vec<_> = text.split('-').collect();
    if groups.len() != 8 || groups.iter().zip(GROUPS).any(|(s, n)| s.len() != n) {
        return Err(VaultError::InvalidEnvelope);
    }
    let mut out = Zeroizing::new([0; 32]);
    let mut at = 0;
    let mut bits = 0;
    let mut buffer = 0u16;
    for byte in groups.iter().flat_map(|g| g.bytes()) {
        let value = ALPHABET
            .iter()
            .position(|v| *v == byte)
            .ok_or(VaultError::InvalidEnvelope)?;
        buffer = (buffer << 5) | value as u16;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            if at >= 32 {
                return Err(VaultError::InvalidEnvelope);
            }
            out[at] = (buffer >> bits) as u8;
            at += 1;
        }
    }
    if at != 32 || format(&out).as_str() != text {
        return Err(VaultError::InvalidEnvelope);
    }
    Ok(out)
}

pub struct RecoveryDisplay {
    pub text: Zeroizing<String>,
    pub challenge: [u8; 16],
}
pub struct PendingSetup {
    root_id: String,
    context: Context,
    slots: KeySlots,
    recovery: Zeroizing<[u8; 32]>,
    challenge: [u8; 16],
    displayed: bool,
}
pub struct AcknowledgedSetup {
    pub(crate) root_id: String,
    pub(crate) context: Context,
    pub(crate) slots: KeySlots,
    pub(crate) dek: Zeroizing<[u8; 32]>,
}
impl PendingSetup {
    pub(crate) fn new(context: Context, password: &[u8], root_id: String) -> Result<Self> {
        let (slots, recovery) = KeySlots::initialize(&context, password)?;
        Ok(Self {
            root_id,
            context,
            slots,
            recovery,
            challenge: *envelope::random::<16>()?,
            displayed: false,
        })
    }
    pub fn display_once(&mut self) -> Result<RecoveryDisplay> {
        if self.displayed {
            return Err(VaultError::AcknowledgementRequired);
        }
        self.displayed = true;
        Ok(RecoveryDisplay {
            text: format(&self.recovery),
            challenge: self.challenge,
        })
    }
    pub fn acknowledge(
        self,
        typed_key: &str,
        challenge: &[u8; 16],
        saved_copy: bool,
    ) -> Result<AcknowledgedSetup> {
        if !self.displayed || !saved_copy || self.challenge != *challenge {
            return Err(VaultError::AcknowledgementRequired);
        }
        let supplied = parse(typed_key)?;
        let dek = self.slots.recover(&self.context, &supplied)?;
        Ok(AcknowledgedSetup {
            root_id: self.root_id,
            context: self.context,
            slots: self.slots,
            dek,
        })
    }
}
