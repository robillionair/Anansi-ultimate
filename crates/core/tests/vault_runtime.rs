use anansi_core::{
    Core, ErrorCode,
    jobs::ExportSelection,
    storage::Storage,
    vault::{
        VaultError,
        catalog::{BackupReference, Phase},
        keys::Context,
        recovery::{self, RecoveryDisplay},
        runtime::{CredentialPurpose, NativeCredentialProvider, VaultRuntime},
        service::{AdapterVaultState, VaultService},
        session::{Caller, Transport},
    },
};
use serde_json::json;
use std::time::Instant;
use zeroize::Zeroizing;

struct SyntheticProvider {
    passphrase: Vec<u8>,
    source_passphrase: Option<Vec<u8>>,
    source_recovery: Option<[u8; 32]>,
    last_recovery: Option<String>,
    fail_recovery: bool,
    cancel: bool,
}

impl SyntheticProvider {
    fn new(passphrase: &[u8]) -> Self {
        Self {
            passphrase: passphrase.to_vec(),
            source_passphrase: None,
            source_recovery: None,
            last_recovery: None,
            fail_recovery: false,
            cancel: false,
        }
    }
}

impl NativeCredentialProvider for SyntheticProvider {
    fn passphrase(
        &mut self,
        purpose: CredentialPurpose,
    ) -> anansi_core::vault::Result<Zeroizing<Vec<u8>>> {
        if self.cancel {
            return Err(VaultError::CredentialUnavailable);
        }
        if purpose == CredentialPurpose::Restore
            && let Some(src) = &self.source_passphrase
        {
            return Ok(Zeroizing::new(src.clone()));
        }
        Ok(Zeroizing::new(self.passphrase.clone()))
    }
    fn recovery_key(
        &mut self,
        display: &RecoveryDisplay,
    ) -> anansi_core::vault::Result<Zeroizing<[u8; 32]>> {
        if self.cancel {
            return Err(VaultError::CredentialUnavailable);
        }
        if self.fail_recovery {
            return Err(VaultError::AcknowledgementRequired);
        }
        self.last_recovery = Some(display.text.as_str().to_owned());
        recovery::parse(display.text.as_str())
    }
    fn source_recovery_key(&mut self) -> anansi_core::vault::Result<Zeroizing<[u8; 32]>> {
        if self.cancel {
            return Err(VaultError::CredentialUnavailable);
        }
        self.source_recovery
            .map(Zeroizing::new)
            .ok_or(VaultError::AuthenticationFailed)
    }
}

fn caller(workspace: &str, root: &str) -> Caller {
    Caller {
        context: Context::new(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            workspace,
            "cccccccccccccccccccccccccccccccc",
        )
        .unwrap(),
        transport: Transport::Desktop,
        root_id: root.into(),
        caller_id: "window".into(),
    }
}

#[test]
fn runtime_rejects_browser_setup_before_any_credential_input() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let core = anansi_core::Core::with_storage_root(root.clone());
    let workspace = core
        .workspace_command(
            "create",
            serde_json::json!({"contractVersion":1,"name":"Runtime"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let root_id =
        anansi_core::content::hash(root.canonicalize().unwrap().to_string_lossy().as_bytes());
    std::fs::write(root.join(".anansi-root"), b"ANANSI workspace root v1\n").unwrap();
    let mut runtime = VaultRuntime::new(root.clone(), temp.path().to_path_buf()).unwrap();
    let mut provider = SyntheticProvider {
        fail_recovery: true,
        ..SyntheticProvider::new(b"synthetic passphrase")
    };
    let mut browser = caller(&workspace, &root_id);
    browser.transport = Transport::Browser;
    let err = match runtime.setup(&workspace, &browser, &mut provider, Instant::now(), true) {
        Ok(_) => panic!("setup unexpectedly succeeded"),
        Err(err) => err,
    };
    assert_eq!(err, VaultError::InvalidContext);
}

#[test]
fn service_browser_authorization_and_encrypted_backup_export_restore_workflows() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let host = temp.path().join("admission");
    std::fs::create_dir_all(&host).unwrap();
    anansi_core::storage::private_permissions(&host, true, false).unwrap();

    let core = Core::with_storage_root(root.clone());
    let workspace = core
        .workspace_command(
            "create",
            json!({"contractVersion":1,"name":"Encrypted archive source"}),
        )
        .unwrap()["workspace"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    core.memory_command(
        "create",
        json!({
            "contractVersion": 1,
            "workspaceId": workspace,
            "kind": "note",
            "draft": {
                "title": "Encrypted backup note",
                "body": "Ciphertext-only portable backup and export verification body.",
                "privacy": "private"
            }
        }),
    )
    .unwrap();

    let storage = Storage::new(root.clone()).unwrap();
    let job = storage
        .enqueue_export(&workspace, ExportSelection::Backup { application: false })
        .unwrap();
    let lease = storage.claim_export().unwrap().unwrap();
    storage.execute_backup(&lease).unwrap();
    let complete = storage.read_job(&workspace, &job.id).unwrap();
    let backup_ref = BackupReference {
        job_id: job.id,
        artifact_id: complete.artifact_id.unwrap(),
        sha256: complete.artifact_hash.unwrap(),
    };

    let service = VaultService::new(root.clone(), host).unwrap();
    let root_id = service.root_id().unwrap();
    let mut provider = SyntheticProvider::new(b"source-vault-passphrase");
    let setup_caller = Caller {
        context: Context::new(
            "00000000000000000000000000000000",
            &workspace,
            "00000000000000000000000000000000",
        )
        .unwrap(),
        transport: Transport::Desktop,
        root_id: root_id.clone(),
        caller_id: "desktop-main".into(),
    };
    let display = service
        .setup(
            &workspace,
            &setup_caller,
            &mut provider,
            Instant::now(),
            true,
        )
        .unwrap();
    let source_recovery = *recovery::parse(display.text.as_str()).unwrap();
    service.seal_workspace(&backup_ref, true).unwrap();

    // Paired browser unlock and authorized read
    let (active_ctx, _) = service.context(&workspace).unwrap();
    let browser_caller = Caller {
        context: active_ctx.clone(),
        transport: Transport::Browser,
        root_id: root_id.clone(),
        caller_id: "browser:http://127.0.0.1:1420".into(),
    };
    let browser_token = service
        .unlock(&browser_caller, &mut provider, Instant::now())
        .unwrap();
    let snapshot = service
        .read(&browser_caller, &browser_token, Instant::now())
        .unwrap();
    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(
        snapshot.records[0].body,
        "Ciphertext-only portable backup and export verification body."
    );

    // Wrong origin or wrong transport cannot use the paired browser session token
    let mut wrong_origin = browser_caller.clone();
    wrong_origin.caller_id = "browser:http://127.0.0.1:9999".into();
    assert_eq!(
        service
            .read(&wrong_origin, &browser_token, Instant::now())
            .err(),
        Some(VaultError::Locked)
    );

    // Encrypted export while unlocked, and ciphertext backup + preview while locked
    let mut export_provider = SyntheticProvider::new(b"portable-export-passphrase");
    let exported = service
        .encrypted_export(
            &browser_caller,
            &browser_token,
            &mut export_provider,
            Instant::now(),
        )
        .unwrap();
    assert_eq!(exported.kind, "encrypted_export");

    service.lock(&workspace).unwrap();
    assert_eq!(
        service
            .read(&browser_caller, &browser_token, Instant::now())
            .err(),
        Some(VaultError::Locked)
    );

    let backed_up = service.ciphertext_backup(&workspace).unwrap();
    assert_eq!(backed_up.kind, "ciphertext_backup");
    let preview = service.preview_archive(&backed_up.archive_id).unwrap();
    assert!(!preview.authenticated);
    assert_eq!(preview.sha256, backed_up.sha256);

    // Wrong key, tamper, cancel, and mid-restore interruption preserve existing state
    let mut wrong_key = SyntheticProvider::new(b"fresh-dest-passphrase");
    wrong_key.source_passphrase = Some(b"wrong-source-passphrase".to_vec());
    assert_eq!(
        service
            .restore_archive(
                &backed_up.archive_id,
                "Wrong key restore",
                false,
                false,
                Some(&backed_up.sha256),
                &mut wrong_key,
                true,
            )
            .unwrap_err(),
        VaultError::AuthenticationFailed
    );

    let mut cancelled = SyntheticProvider::new(b"fresh-dest-passphrase");
    cancelled.source_passphrase = Some(b"source-vault-passphrase".to_vec());
    cancelled.cancel = true;
    assert_eq!(
        service
            .restore_archive(
                &backed_up.archive_id,
                "Cancelled restore",
                false,
                false,
                Some(&backed_up.sha256),
                &mut cancelled,
                true,
            )
            .unwrap_err(),
        VaultError::CredentialUnavailable
    );

    let mut bad_hash = SyntheticProvider::new(b"fresh-dest-passphrase");
    bad_hash.source_passphrase = Some(b"source-vault-passphrase".to_vec());
    assert_eq!(
        service
            .restore_archive(
                &backed_up.archive_id,
                "Tampered hash restore",
                false,
                false,
                Some(&"0".repeat(64)),
                &mut bad_hash,
                true,
            )
            .unwrap_err(),
        VaultError::AuthenticationFailed
    );

    for phase in [Phase::PreJournal, Phase::MidStage, Phase::PreSwitch] {
        let mut interrupted = SyntheticProvider::new(b"fresh-dest-passphrase");
        interrupted.source_passphrase = Some(b"source-vault-passphrase".to_vec());
        assert_eq!(
            service
                .restore_archive_observed(
                    &backed_up.archive_id,
                    "Interrupted restore",
                    false,
                    false,
                    Some(&backed_up.sha256),
                    &mut interrupted,
                    true,
                    |observed| {
                        if observed == phase {
                            Err(VaultError::StorageUnavailable)
                        } else {
                            Ok(())
                        }
                    },
                )
                .unwrap_err(),
            VaultError::StorageUnavailable
        );
    }

    // Fresh restore from ciphertext backup using recovery key
    let mut recovery_restore = SyntheticProvider::new(b"restored-from-recovery-passphrase");
    recovery_restore.source_recovery = Some(source_recovery);
    let restored_from_backup = service
        .restore_archive(
            &backed_up.archive_id,
            "Restored from ciphertext backup",
            false,
            true,
            Some(&backed_up.sha256),
            &mut recovery_restore,
            true,
        )
        .unwrap();
    assert_ne!(restored_from_backup.restored_workspace_id, workspace);

    // Fresh restore from encrypted export using export passphrase
    let mut export_restore = SyntheticProvider::new(b"restored-from-export-passphrase");
    export_restore.source_passphrase = Some(b"portable-export-passphrase".to_vec());
    let restored_from_export = service
        .restore_archive(
            &exported.archive_id,
            "Restored from encrypted export",
            true,
            false,
            Some(&exported.sha256),
            &mut export_restore,
            true,
        )
        .unwrap();
    assert_ne!(restored_from_export.restored_workspace_id, workspace);
    assert_ne!(
        restored_from_export.restored_workspace_id,
        restored_from_backup.restored_workspace_id
    );

    // Unlock and read the restored workspace via paired browser caller
    let (restored_ctx, _) = service
        .context(&restored_from_export.restored_workspace_id)
        .unwrap();
    let restored_browser = Caller {
        context: restored_ctx,
        transport: Transport::Browser,
        root_id,
        caller_id: "browser:http://127.0.0.1:1420".into(),
    };
    let restored_token = service
        .unlock(&restored_browser, &mut export_restore, Instant::now())
        .unwrap();
    let restored_snapshot = service
        .read(&restored_browser, &restored_token, Instant::now())
        .unwrap();
    assert_eq!(restored_snapshot.records.len(), 1);
    assert_eq!(
        restored_snapshot.records[0].body,
        "Ciphertext-only portable backup and export verification body."
    );
    assert_eq!(
        restored_snapshot.origin.as_ref().unwrap().workspace_id,
        workspace
    );

    // AdapterVaultState rejects secret-bearing DTO fields before any operation
    let adapter = AdapterVaultState::for_storage_root(root).unwrap();
    for secret_key in ["passphrase", "password", "recoveryKey", "dek", "token"] {
        let err = adapter
            .execute(
                Transport::Browser,
                "browser:http://127.0.0.1:1420",
                Some("http://127.0.0.1:1420"),
                "unlock",
                json!({
                    "contractVersion": 1,
                    "workspaceId": workspace,
                    secret_key: "forbidden"
                }),
            )
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidRequest);
    }
}
