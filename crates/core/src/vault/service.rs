//! Process-owned vault service for adapters.
//!
//! The service deliberately keeps `VaultRuntime` behind a mutex. Adapters may
//! clone the service handle, but credentials, DEKs, recovery material and
//! session tokens remain process-local and are never serialized.
use super::{Result, VaultError, catalog::BackupReference, recovery::RecoveryDisplay, runtime::{NativeCredentialProvider, VaultRuntime}, session::Caller};
use std::{path::PathBuf, sync::{Arc, Mutex}, time::Instant};
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct VaultService(Arc<Mutex<VaultRuntime>>);

impl VaultService {
    pub fn new(root: PathBuf, host_namespace: PathBuf) -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(VaultRuntime::new(root, host_namespace)?))))
    }

    fn with_runtime<T>(&self, f: impl FnOnce(&mut VaultRuntime) -> Result<T>) -> Result<T> {
        let mut runtime = self.0.lock().map_err(|_| VaultError::Busy)?;
        f(&mut runtime)
    }

    pub fn setup<P: NativeCredentialProvider>(&self, workspace: &str, caller: &Caller, provider: &mut P, now: Instant, acknowledge_saved_copy: bool) -> Result<RecoveryDisplay> {
        self.with_runtime(|runtime| runtime.setup(workspace, caller, provider, now, acknowledge_saved_copy))
    }

    pub fn seal_workspace(&self, backup: &BackupReference, retain_plaintext_backup: bool) -> Result<()> {
        self.with_runtime(|runtime| runtime.seal_workspace(backup, retain_plaintext_backup))
    }

    pub fn unlock<P: NativeCredentialProvider>(&self, caller: &Caller, provider: &mut P, now: Instant) -> Result<Zeroizing<[u8; 16]>> {
        self.with_runtime(|runtime| runtime.unlock(caller, provider, now))
    }

    pub fn read(&self, caller: &Caller, token: &[u8; 16], now: Instant) -> Result<super::payload::Snapshot> {
        self.with_runtime(|runtime| runtime.read(caller, token, now))
    }

    pub fn lock(&self, workspace: &str) -> Result<u64> {
        self.with_runtime(|runtime| runtime.lock(workspace))
    }

    pub fn context(&self, workspace: &str) -> Result<(super::keys::Context, u64)> {
        self.with_runtime(|runtime| runtime.context(workspace))
    }
}
