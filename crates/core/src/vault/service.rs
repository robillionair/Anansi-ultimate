//! Process-owned vault service for adapters.
//!
//! The service deliberately keeps `VaultRuntime` behind a mutex. Adapters may
//! clone the service handle, but credentials, DEKs, recovery material and
//! session tokens remain process-local and are never serialized.
use super::{
    Result, VaultError,
    catalog::{BackupReference, Phase},
    keys::Context,
    native_prompt::WindowsCredentialProvider,
    recovery::RecoveryDisplay,
    runtime::{
        EncryptedArchiveInfo, EncryptedArchivePreview, NativeCredentialProvider,
        RestoredWorkspaceInfo, VaultRuntime,
    },
    session::{Caller, Transport},
};
use crate::{CommandError, ErrorCode};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct VaultService(Arc<Mutex<VaultRuntime>>);

impl VaultService {
    pub fn new(root: PathBuf, host_namespace: PathBuf) -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(VaultRuntime::new(
            root,
            host_namespace,
        )?))))
    }

    fn with_runtime<T>(&self, f: impl FnOnce(&mut VaultRuntime) -> Result<T>) -> Result<T> {
        let mut runtime = self.0.lock().map_err(|_| VaultError::Busy)?;
        f(&mut runtime)
    }

    pub fn root_id(&self) -> Result<String> {
        self.with_runtime(|runtime| runtime.root_id())
    }

    pub fn setup<P: NativeCredentialProvider>(
        &self,
        workspace: &str,
        caller: &Caller,
        provider: &mut P,
        now: Instant,
        acknowledge_saved_copy: bool,
    ) -> Result<RecoveryDisplay> {
        self.with_runtime(|runtime| {
            runtime.setup(workspace, caller, provider, now, acknowledge_saved_copy)
        })
    }

    pub fn seal_workspace(
        &self,
        backup: &BackupReference,
        retain_plaintext_backup: bool,
    ) -> Result<()> {
        self.with_runtime(|runtime| runtime.seal_workspace(backup, retain_plaintext_backup))
    }

    pub fn unlock<P: NativeCredentialProvider>(
        &self,
        caller: &Caller,
        provider: &mut P,
        now: Instant,
    ) -> Result<Zeroizing<[u8; 16]>> {
        self.with_runtime(|runtime| runtime.unlock(caller, provider, now))
    }

    pub fn read(
        &self,
        caller: &Caller,
        token: &[u8; 16],
        now: Instant,
    ) -> Result<super::payload::Snapshot> {
        self.with_runtime(|runtime| runtime.read(caller, token, now))
    }

    pub fn lock(&self, workspace: &str) -> Result<u64> {
        self.with_runtime(|runtime| runtime.lock(workspace))
    }

    pub fn context(&self, workspace: &str) -> Result<(super::keys::Context, u64)> {
        self.with_runtime(|runtime| runtime.context(workspace))
    }

    pub fn rotate<P: NativeCredentialProvider>(
        &self,
        caller: &Caller,
        token: &[u8; 16],
        provider: &mut P,
        now: Instant,
        acknowledge_saved_copy: bool,
    ) -> Result<RecoveryDisplay> {
        self.with_runtime(|runtime| {
            runtime.rotate(caller, token, provider, now, acknowledge_saved_copy)
        })
    }

    pub fn ciphertext_backup(&self, workspace: &str) -> Result<EncryptedArchiveInfo> {
        self.with_runtime(|runtime| runtime.ciphertext_backup(workspace))
    }

    pub fn encrypted_export<P: NativeCredentialProvider>(
        &self,
        caller: &Caller,
        token: &[u8; 16],
        provider: &mut P,
        now: Instant,
    ) -> Result<EncryptedArchiveInfo> {
        self.with_runtime(|runtime| runtime.encrypted_export(caller, token, provider, now))
    }

    pub fn preview_archive(&self, archive_id: &str) -> Result<EncryptedArchivePreview> {
        self.with_runtime(|runtime| runtime.preview_archive(archive_id))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore_archive<P: NativeCredentialProvider>(
        &self,
        archive_id: &str,
        name: &str,
        encrypted_export: bool,
        use_recovery: bool,
        expected_sha256: Option<&str>,
        provider: &mut P,
        acknowledge_saved_copy: bool,
    ) -> Result<RestoredWorkspaceInfo> {
        self.with_runtime(|runtime| {
            runtime.restore_archive(
                archive_id,
                name,
                encrypted_export,
                use_recovery,
                expected_sha256,
                provider,
                acknowledge_saved_copy,
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore_archive_observed<P: NativeCredentialProvider>(
        &self,
        archive_id: &str,
        name: &str,
        encrypted_export: bool,
        use_recovery: bool,
        expected_sha256: Option<&str>,
        provider: &mut P,
        acknowledge_saved_copy: bool,
        observe: impl FnMut(Phase) -> Result<()>,
    ) -> Result<RestoredWorkspaceInfo> {
        self.with_runtime(|runtime| {
            runtime.restore_archive_observed(
                archive_id,
                name,
                encrypted_export,
                use_recovery,
                expected_sha256,
                provider,
                acknowledge_saved_copy,
                observe,
            )
        })
    }
}

struct AdapterSession {
    caller: Caller,
    token: Zeroizing<[u8; 16]>,
}

#[derive(Clone)]
pub struct AdapterVaultState {
    root: PathBuf,
    service: VaultService,
    sessions: Arc<Mutex<HashMap<(String, String), AdapterSession>>>,
}

pub fn vault_command_error(error: VaultError) -> CommandError {
    let (code, message) = match error {
        VaultError::Locked | VaultError::AuthenticationFailed => (
            ErrorCode::Unauthorized,
            "The vault is locked or the credential was rejected.",
        ),
        VaultError::Busy => (
            ErrorCode::VaultUnavailable,
            "The vault is busy. Retry after the current operation finishes.",
        ),
        VaultError::AcknowledgementRequired => (
            ErrorCode::InvalidRequest,
            "Recovery acknowledgement and a verified backup are required.",
        ),
        VaultError::InvalidContext => (
            ErrorCode::Forbidden,
            "The vault request does not match this workspace or application process.",
        ),
        VaultError::UnsupportedSchema => (
            ErrorCode::UnsupportedSchema,
            "The vault storage schema is not supported.",
        ),
        VaultError::Conflict => (
            ErrorCode::RevisionConflict,
            "The vault state or backup changed before publication completed.",
        ),
        _ => (
            ErrorCode::VaultUnavailable,
            "The vault operation failed without changing canonical data.",
        ),
    };
    CommandError::new(code, message)
}

fn required_text(
    value: &serde_json::Value,
    name: &str,
) -> std::result::Result<String, CommandError> {
    value
        .get(name)
        .and_then(serde_json::Value::as_str)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            CommandError::new(
                ErrorCode::InvalidRequest,
                "Use the supported vault request.",
            )
        })
}

fn reject_secret_dto_fields(value: &serde_json::Value) -> std::result::Result<(), CommandError> {
    let Some(obj) = value.as_object() else {
        return Err(CommandError::new(
            ErrorCode::InvalidRequest,
            "Use the supported vault request.",
        ));
    };
    if obj
        .get("contractVersion")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        return Err(CommandError::new(
            ErrorCode::UnsupportedContract,
            "Unsupported contract version.",
        ));
    }
    for forbidden in [
        "passphrase",
        "password",
        "recoveryKey",
        "recovery",
        "dek",
        "token",
        "secret",
        "key",
        "sessionToken",
    ] {
        if obj.contains_key(forbidden) {
            return Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "Vault credentials and session handles cannot be sent in request DTOs.",
            ));
        }
    }
    Ok(())
}

impl AdapterVaultState {
    pub fn for_storage_root(root: PathBuf) -> std::result::Result<Self, CommandError> {
        let storage = crate::storage::Storage::new(root.clone())?;
        if !root.join(".anansi-root").exists() {
            storage.connection(true)?;
        }
        let namespace = std::env::var_os("ANANSI_VAULT_ADMISSION_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("vault-admission"));
        std::fs::create_dir_all(&namespace).map_err(|_| crate::storage::unavailable())?;
        crate::storage::private_permissions(&namespace, true, false)
            .map_err(|_| crate::storage::unavailable())?;
        let service = VaultService::new(root.clone(), namespace).map_err(vault_command_error)?;
        Ok(Self {
            root,
            service,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn service(&self) -> &VaultService {
        &self.service
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn build_caller(
        &self,
        workspace: &str,
        transport: Transport,
        caller_id: &str,
    ) -> std::result::Result<Caller, CommandError> {
        let root_id = self.service.root_id().map_err(vault_command_error)?;
        let context = self
            .service
            .context(workspace)
            .map(|v| v.0)
            .unwrap_or_else(|_| {
                Context::new(
                    "00000000000000000000000000000000",
                    workspace,
                    "00000000000000000000000000000000",
                )
                .expect("fixed context")
            });
        Ok(Caller {
            context,
            transport,
            root_id,
            caller_id: caller_id.to_owned(),
        })
    }

    fn build_provider(
        &self,
        workspace: &str,
        browser_origin: Option<&str>,
    ) -> WindowsCredentialProvider {
        match browser_origin {
            Some(origin) => WindowsCredentialProvider::for_browser(workspace, origin),
            None => WindowsCredentialProvider::new(workspace),
        }
    }

    pub fn execute(
        &self,
        transport: Transport,
        caller_id: &str,
        browser_origin: Option<&str>,
        command: &str,
        request: serde_json::Value,
    ) -> std::result::Result<serde_json::Value, CommandError> {
        reject_secret_dto_fields(&request)?;
        if caller_id.trim().is_empty()
            || (transport == Transport::Browser
                && browser_origin.is_none_or(|o| o.trim().is_empty()))
        {
            return Err(vault_command_error(VaultError::InvalidContext));
        }
        let workspace = required_text(&request, "workspaceId")?;
        let session_key = (caller_id.to_owned(), workspace.clone());
        match command {
            "setup" => {
                let caller = self.build_caller(&workspace, transport, caller_id)?;
                let mut provider = self.build_provider(&workspace, browser_origin);
                self.service
                    .setup(&workspace, &caller, &mut provider, Instant::now(), true)
                    .map_err(vault_command_error)?;
                Ok(serde_json::json!({"contractVersion":1,"status":"acknowledged"}))
            }
            "seal" => {
                if transport != Transport::Desktop {
                    return Err(vault_command_error(VaultError::InvalidContext));
                }
                let backup = BackupReference {
                    job_id: required_text(&request, "jobId")?,
                    artifact_id: required_text(&request, "artifactId")?,
                    sha256: required_text(&request, "sha256")?,
                };
                self.service
                    .seal_workspace(&backup, true)
                    .map_err(vault_command_error)?;
                Ok(serde_json::json!({"contractVersion":1,"status":"sealed"}))
            }
            "unlock" => {
                let caller = self.build_caller(&workspace, transport, caller_id)?;
                let mut provider = self.build_provider(&workspace, browser_origin);
                let token = self
                    .service
                    .unlock(&caller, &mut provider, Instant::now())
                    .map_err(vault_command_error)?;
                self.sessions
                    .lock()
                    .map_err(|_| crate::storage::unavailable())?
                    .insert(session_key, AdapterSession { caller, token });
                Ok(serde_json::json!({"contractVersion":1,"status":"unlocked"}))
            }
            "read" => {
                let guard = self
                    .sessions
                    .lock()
                    .map_err(|_| crate::storage::unavailable())?;
                let session = guard
                    .get(&session_key)
                    .ok_or_else(|| vault_command_error(VaultError::Locked))?;
                if session.caller.transport != transport || session.caller.caller_id != caller_id {
                    return Err(vault_command_error(VaultError::InvalidContext));
                }
                let snapshot = self
                    .service
                    .read(&session.caller, &session.token, Instant::now())
                    .map_err(vault_command_error)?;
                let mut value =
                    serde_json::to_value(snapshot).map_err(|_| crate::storage::unavailable())?;
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("contractVersion".into(), serde_json::json!(1));
                }
                Ok(value)
            }
            "lock" => {
                let epoch = self.service.lock(&workspace).map_err(vault_command_error)?;
                self.sessions
                    .lock()
                    .map_err(|_| crate::storage::unavailable())?
                    .retain(|(_, w), _| w != &workspace);
                Ok(serde_json::json!({"contractVersion":1,"status":"locked","epoch":epoch}))
            }
            "rotate" => {
                let (caller, token) = {
                    let guard = self
                        .sessions
                        .lock()
                        .map_err(|_| crate::storage::unavailable())?;
                    let session = guard
                        .get(&session_key)
                        .ok_or_else(|| vault_command_error(VaultError::Locked))?;
                    (session.caller.clone(), Zeroizing::new(*session.token))
                };
                let mut provider = self.build_provider(&workspace, browser_origin);
                self.service
                    .rotate(&caller, &token, &mut provider, Instant::now(), true)
                    .map_err(vault_command_error)?;
                self.sessions
                    .lock()
                    .map_err(|_| crate::storage::unavailable())?
                    .retain(|(_, w), _| w != &workspace);
                Ok(serde_json::json!({"contractVersion":1,"status":"rotated"}))
            }
            "backup" => {
                let info = self
                    .service
                    .ciphertext_backup(&workspace)
                    .map_err(vault_command_error)?;
                Ok(serde_json::json!({
                    "contractVersion": 1,
                    "status": "backed_up",
                    "archiveId": info.archive_id,
                    "kind": info.kind,
                    "sha256": info.sha256,
                    "bytes": info.bytes,
                    "files": info.files,
                }))
            }
            "export" => {
                let (caller, token) = {
                    let guard = self
                        .sessions
                        .lock()
                        .map_err(|_| crate::storage::unavailable())?;
                    let session = guard
                        .get(&session_key)
                        .ok_or_else(|| vault_command_error(VaultError::Locked))?;
                    (session.caller.clone(), Zeroizing::new(*session.token))
                };
                let mut provider = self.build_provider(&workspace, browser_origin);
                let info = self
                    .service
                    .encrypted_export(&caller, &token, &mut provider, Instant::now())
                    .map_err(vault_command_error)?;
                Ok(serde_json::json!({
                    "contractVersion": 1,
                    "status": "exported",
                    "archiveId": info.archive_id,
                    "kind": info.kind,
                    "sha256": info.sha256,
                    "bytes": info.bytes,
                    "files": info.files,
                }))
            }
            "preview" => {
                let archive_id = required_text(&request, "archiveId")?;
                let preview = self
                    .service
                    .preview_archive(&archive_id)
                    .map_err(vault_command_error)?;
                Ok(serde_json::json!({
                    "contractVersion": 1,
                    "archiveId": preview.archive_id,
                    "authenticated": preview.authenticated,
                    "files": preview.files,
                    "bytes": preview.bytes,
                    "sha256": preview.sha256,
                }))
            }
            "restore" => {
                let archive_id = required_text(&request, "archiveId")?;
                let name = required_text(&request, "name")?;
                let encrypted_export = request
                    .get("encryptedExport")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let use_recovery = request
                    .get("useRecovery")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let expected_sha256 = request
                    .get("expectedSha256")
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.is_empty());
                let mut provider = self.build_provider(&workspace, browser_origin);
                let restored = self
                    .service
                    .restore_archive(
                        &archive_id,
                        &name,
                        encrypted_export,
                        use_recovery,
                        expected_sha256,
                        &mut provider,
                        true,
                    )
                    .map_err(vault_command_error)?;
                Ok(serde_json::json!({
                    "contractVersion": 1,
                    "status": "restored",
                    "restoredWorkspaceId": restored.restored_workspace_id,
                    "name": restored.name,
                }))
            }
            _ => Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "Use a supported vault command.",
            )),
        }
    }
}
