//! Trusted vault orchestration kept outside serialized command DTOs.
//! Adapters provide credentials through a native-only provider; browsers never
//! pass passphrases or recovery keys through this runtime.
use super::{
    Result, VaultError,
    catalog::{BackupReference, VaultStore},
    recovery::{AcknowledgedSetup, RecoveryDisplay},
    session::{Caller, Sessions},
};
use std::{path::PathBuf, time::Instant};
use zeroize::Zeroizing;

pub trait NativeCredentialProvider {
    fn passphrase(&mut self, purpose: CredentialPurpose) -> Result<Zeroizing<Vec<u8>>>;
    fn recovery_key(&mut self, display: &RecoveryDisplay) -> Result<Zeroizing<[u8; 32]>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialPurpose {
    Setup,
    Unlock,
    Rotation,
    Restore,
}

pub struct VaultRuntime {
    store: VaultStore,
    sessions: Sessions,
    pending_setup: Option<AcknowledgedSetup>,
}

impl VaultRuntime {
    pub fn new(root: PathBuf, host_namespace: PathBuf) -> Result<Self> {
        Ok(Self {
            store: VaultStore::new(root, host_namespace)?,
            sessions: Sessions::default(),
            pending_setup: None,
        })
    }

    pub fn setup<P: NativeCredentialProvider>(
        &mut self,
        workspace: &str,
        caller: &Caller,
        provider: &mut P,
        now: Instant,
        acknowledge_saved_copy: bool,
    ) -> Result<RecoveryDisplay> {
        if caller.transport != super::session::Transport::Desktop
            || caller.context.workspace_id() != workspace
            || caller.root_id != self.store.root_id()?
        {
            return Err(VaultError::InvalidContext);
        }
        let passphrase = provider.passphrase(CredentialPurpose::Setup)?;
        let mut pending = self.store.begin(workspace, &passphrase)?;
        let display = pending.display_once()?;
        let recovery = provider.recovery_key(&display)?;
        let ack: AcknowledgedSetup = pending.acknowledge(
            &super::recovery::format(&recovery),
            &display.challenge,
            acknowledge_saved_copy,
        )?;
        // Setup is intentionally not auto-published. The caller must present a
        // verified M2.5 BackupReference before seal_workspace is invoked.
        self.pending_setup = Some(ack);
        let _ = now;
        Ok(display)
    }

    pub fn unlock<P: NativeCredentialProvider>(
        &mut self,
        caller: &Caller,
        provider: &mut P,
        now: Instant,
    ) -> Result<Zeroizing<[u8; 16]>> {
        let password = provider.passphrase(CredentialPurpose::Unlock)?;
        self.store
            .unlock(&mut self.sessions, caller, &password, now)
    }

    pub fn lock(&mut self, workspace: &str) -> Result<u64> {
        self.store.lock(workspace, &mut self.sessions)
    }

    /// Returns the active vault context and durable epoch after sealing.
    /// Adapters use this to construct a caller-bound session; no key material
    /// crosses this boundary.
    pub fn context(&self, workspace: &str) -> Result<(super::keys::Context, u64)> {
        self.store.context(workspace)
    }

    /// Reads a fully authenticated snapshot for an already authorized handle.
    /// The store performs a second epoch/session check before returning data.
    pub fn read(
        &mut self,
        caller: &Caller,
        token: &[u8; 16],
        now: Instant,
    ) -> Result<super::payload::Snapshot> {
        self.store.read(&mut self.sessions, caller, token, now)
    }

    pub fn seal_workspace(
        &mut self,
        backup: &BackupReference,
        retain_plaintext_backup: bool,
    ) -> Result<()> {
        let setup = self
            .pending_setup
            .take()
            .ok_or(VaultError::AcknowledgementRequired)?;
        self.store
            .seal_workspace(setup, backup, retain_plaintext_backup)
    }
}
