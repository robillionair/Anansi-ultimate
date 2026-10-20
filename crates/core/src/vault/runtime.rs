//! Trusted vault orchestration kept outside serialized command DTOs.
//! Adapters provide credentials through a native-only provider; browsers never
//! pass passphrases or recovery keys through this runtime.
use super::{
    Result, VaultError,
    archive::CiphertextBackup,
    catalog::{BackupReference, Phase, VaultStore},
    envelope,
    io::write_cipher,
    migration::read_bounded,
    payload::{id, invalid},
    recovery::{AcknowledgedSetup, RecoveryDisplay},
    session::{Caller, Sessions},
};
use crate::{
    content,
    storage::{private_permissions, reject_links},
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Instant};
use zeroize::Zeroizing;

pub trait NativeCredentialProvider {
    fn passphrase(&mut self, purpose: CredentialPurpose) -> Result<Zeroizing<Vec<u8>>>;
    fn recovery_key(&mut self, display: &RecoveryDisplay) -> Result<Zeroizing<[u8; 32]>>;
    fn source_recovery_key(&mut self) -> Result<Zeroizing<[u8; 32]>> {
        Err(VaultError::AuthenticationFailed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialPurpose {
    Setup,
    Unlock,
    Rotation,
    Restore,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedArchiveInfo {
    pub archive_id: String,
    pub kind: String,
    pub sha256: String,
    pub bytes: usize,
    pub files: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedArchivePreview {
    pub archive_id: String,
    pub authenticated: bool,
    pub files: usize,
    pub bytes: usize,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoredWorkspaceInfo {
    pub restored_workspace_id: String,
    pub name: String,
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

    pub fn root_id(&self) -> Result<String> {
        self.store.root_id()
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
        if caller.caller_id.trim().is_empty() || caller.root_id != self.store.root_id()? {
            return Err(VaultError::InvalidContext);
        }
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

    pub fn rotate<P: NativeCredentialProvider>(
        &mut self,
        caller: &Caller,
        token: &[u8; 16],
        provider: &mut P,
        now: Instant,
        acknowledge_saved_copy: bool,
    ) -> Result<RecoveryDisplay> {
        let passphrase = provider.passphrase(CredentialPurpose::Rotation)?;
        let mut pending = self
            .store
            .rotate(&mut self.sessions, caller, token, &passphrase, now)?;
        let display = pending.display_once()?;
        let recovery = provider.recovery_key(&display)?;
        let ack = pending.acknowledge(
            &super::recovery::format(&recovery),
            &display.challenge,
            acknowledge_saved_copy,
        )?;
        self.store
            .finish_rotation(ack, &mut self.sessions, now, |_| Ok(()))?;
        Ok(display)
    }

    fn archive_directory(&self, create: bool) -> Result<PathBuf> {
        let path = self.store.root.join("vault-archives");
        reject_links(&path).map_err(invalid)?;
        if create && !path.exists() {
            std::fs::create_dir(&path).map_err(invalid)?;
            private_permissions(&path, true, false).map_err(invalid)?;
        }
        if path.exists() {
            private_permissions(&path, true, true).map_err(invalid)?;
        }
        Ok(path)
    }

    fn save_archive(&self, kind: &str, bytes: &[u8], files: usize) -> Result<EncryptedArchiveInfo> {
        let dir = self.archive_directory(true)?;
        let archive_id = content::random_id().map_err(invalid)?;
        let path = dir.join(format!("{archive_id}.ancb"));
        write_cipher(&path, bytes)?;
        private_permissions(&path, false, false).map_err(|_| VaultError::StorageUnavailable)?;
        Ok(EncryptedArchiveInfo {
            archive_id,
            kind: kind.to_owned(),
            sha256: content::hash(bytes),
            bytes: bytes.len(),
            files,
        })
    }

    pub fn read_archive_bytes(&self, archive_id: &str) -> Result<Vec<u8>> {
        id(archive_id)?;
        let dir = self.archive_directory(false)?;
        let path = dir.join(format!("{archive_id}.ancb"));
        reject_links(&path).map_err(invalid)?;
        read_bounded(&path, envelope::MAX_ARCHIVE_BYTES + 80)
    }

    pub fn ciphertext_backup(&self, workspace: &str) -> Result<EncryptedArchiveInfo> {
        let bytes = self.store.ciphertext_backup(workspace)?;
        let preview = CiphertextBackup::preview(&bytes)?;
        self.save_archive("ciphertext_backup", &bytes, preview.files)
    }

    pub fn encrypted_export<P: NativeCredentialProvider>(
        &mut self,
        caller: &Caller,
        token: &[u8; 16],
        provider: &mut P,
        now: Instant,
    ) -> Result<EncryptedArchiveInfo> {
        let passphrase = provider.passphrase(CredentialPurpose::Setup)?;
        let bytes =
            self.store
                .encrypted_export(&mut self.sessions, caller, token, &passphrase, now)?;
        self.save_archive("encrypted_export", &bytes, 1)
    }

    pub fn preview_archive(&self, archive_id: &str) -> Result<EncryptedArchivePreview> {
        let bytes = self.read_archive_bytes(archive_id)?;
        let preview = CiphertextBackup::preview(&bytes)?;
        Ok(EncryptedArchivePreview {
            archive_id: archive_id.to_owned(),
            authenticated: preview.authenticated,
            files: preview.files,
            bytes: preview.bytes,
            sha256: content::hash(&bytes),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore_archive<P: NativeCredentialProvider>(
        &mut self,
        archive_id: &str,
        name: &str,
        encrypted_export: bool,
        use_recovery: bool,
        expected_sha256: Option<&str>,
        provider: &mut P,
        acknowledge_saved_copy: bool,
    ) -> Result<RestoredWorkspaceInfo> {
        self.restore_archive_observed(
            archive_id,
            name,
            encrypted_export,
            use_recovery,
            expected_sha256,
            provider,
            acknowledge_saved_copy,
            |_| Ok(()),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore_archive_observed<P: NativeCredentialProvider>(
        &mut self,
        archive_id: &str,
        name: &str,
        encrypted_export: bool,
        use_recovery: bool,
        expected_sha256: Option<&str>,
        provider: &mut P,
        acknowledge_saved_copy: bool,
        observe: impl FnMut(Phase) -> Result<()>,
    ) -> Result<RestoredWorkspaceInfo> {
        let raw = self.read_archive_bytes(archive_id)?;
        if let Some(expected) = expected_sha256
            && content::hash(&raw) != expected
        {
            return Err(VaultError::AuthenticationFailed);
        }
        let mut pending = if encrypted_export {
            if use_recovery {
                return Err(VaultError::InvalidContext);
            }
            let source_passphrase = provider.passphrase(CredentialPurpose::Restore)?;
            let unpacked = self.store.decrypt_export(&raw, &source_passphrase)?;
            let dest_passphrase = provider.passphrase(CredentialPurpose::Setup)?;
            self.store
                .prepare_restore(&unpacked, &source_passphrase, name, &dest_passphrase)?
        } else if use_recovery {
            let source_recovery = provider.source_recovery_key()?;
            let dest_passphrase = provider.passphrase(CredentialPurpose::Setup)?;
            self.store
                .prepare_restore_recovery(&raw, &source_recovery, name, &dest_passphrase)?
        } else {
            let source_passphrase = provider.passphrase(CredentialPurpose::Restore)?;
            let dest_passphrase = provider.passphrase(CredentialPurpose::Setup)?;
            self.store
                .prepare_restore(&raw, &source_passphrase, name, &dest_passphrase)?
        };
        let display = pending.display_once()?;
        let recovery = provider.recovery_key(&display)?;
        let ack = pending.acknowledge(
            &super::recovery::format(&recovery),
            &display.challenge,
            acknowledge_saved_copy,
        )?;
        let restored_workspace_id = self.store.finish_restore(ack, observe)?;
        Ok(RestoredWorkspaceInfo {
            restored_workspace_id,
            name: name.to_owned(),
        })
    }
}
