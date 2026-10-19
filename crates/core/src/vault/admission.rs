//! OS-owned locks: never unlink a lock file, infer a dead PID, or steal a lease.
use super::{Result, VaultError};
use crate::storage::{private_permissions, reject_links};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub struct Admission(File);
impl Admission {
    fn acquire(directory: &Path, filename: &str) -> Result<Self> {
        reject_links(directory).map_err(|_| VaultError::StorageUnavailable)?;
        if !directory.is_absolute() || !directory.is_dir() {
            return Err(VaultError::StorageUnavailable);
        }
        private_permissions(directory, true, true).map_err(|_| VaultError::StorageUnavailable)?;
        let path = directory.join(filename);
        reject_links(&path).map_err(|_| VaultError::StorageUnavailable)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = match options.open(&path) {
            Ok(file) => {
                private_permissions(&path, false, false)
                    .map_err(|_| VaultError::StorageUnavailable)?;
                file
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                private_permissions(&path, false, true)
                    .map_err(|_| VaultError::StorageUnavailable)?;
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .map_err(|_| VaultError::StorageUnavailable)?
            }
            Err(_) => return Err(VaultError::StorageUnavailable),
        };
        file.try_lock().map_err(|_| VaultError::Busy)?;
        Ok(Self(file))
    }
    pub fn root(root: &Path) -> Result<Self> {
        Self::acquire(root, "vault.operation.lock")
    }
    /// All trusted instances must use the same private host/user namespace.
    /// No directory/path from a renderer is accepted by any adapter.
    pub fn kdf(host_namespace: &Path) -> Result<Self> {
        Self::acquire(host_namespace, "vault.kdf.lock")
    }
}
impl Drop for Admission {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
