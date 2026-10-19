//! Process-local capability with caller context and monotonic expiry.
//! Durable epoch fencing must be provided by the integration transaction.
use super::{Result, VaultError, envelope, keys::Context};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Desktop,
    Browser,
}
#[derive(Clone, PartialEq, Eq)]
pub struct Caller {
    pub context: Context,
    pub transport: Transport,
    /// Trusted host-owned root identity; never a frontend path.
    pub root_id: String,
    /// Native window or paired browser identity, assigned by the adapter.
    pub caller_id: String,
}
struct Session {
    caller: Caller,
    token: Zeroizing<[u8; 16]>,
    dek: Zeroizing<[u8; 32]>,
    epoch: u64,
    created: Instant,
    touched: Instant,
}
#[derive(Default)]
pub struct Sessions {
    active: Option<Session>,
}
impl Sessions {
    pub fn lock(&mut self) {
        self.active = None;
    }
    /// Called only after trusted-input authentication; no serializable key type.
    pub fn establish(
        &mut self,
        caller: Caller,
        dek: Zeroizing<[u8; 32]>,
        epoch: u64,
        now: Instant,
    ) -> Result<Zeroizing<[u8; 16]>> {
        self.lock();
        let token = envelope::random::<16>()?;
        let output = Zeroizing::new(*token);
        self.active = Some(Session {
            caller,
            token,
            dek,
            epoch,
            created: now,
            touched: now,
        });
        Ok(output)
    }
    /// Invoke again inside the fenced publication boundary. Never return a key.
    pub fn authorize<T>(
        &mut self,
        caller: &Caller,
        token: &[u8; 16],
        epoch: u64,
        now: Instant,
        operation: impl FnOnce(&[u8; 32]) -> Result<T>,
    ) -> Result<T> {
        let expired = self.active.as_ref().is_some_and(|s| {
            s.epoch != epoch
                || now
                    .checked_duration_since(s.created)
                    .is_none_or(|d| d >= Duration::from_secs(14400))
                || now
                    .checked_duration_since(s.touched)
                    .is_none_or(|d| d >= Duration::from_secs(900))
        });
        if expired {
            self.lock();
        }
        let s = self.active.as_mut().ok_or(VaultError::Locked)?;
        // Fixed-length comparison without early exit for bearer capability.
        let difference = s
            .token
            .iter()
            .zip(token)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b));
        if difference != 0 || &s.caller != caller {
            return Err(VaultError::Locked);
        }
        s.touched = now;
        operation(&s.dek)
    }
}
