use anansi_core::vault::{
    VaultError,
    keys::Context,
    recovery::RecoveryDisplay,
    runtime::{CredentialPurpose, NativeCredentialProvider, VaultRuntime},
    session::{Caller, Transport},
};
use std::time::Instant;
use zeroize::Zeroizing;

struct SyntheticProvider {
    passphrase: Vec<u8>,
}
impl NativeCredentialProvider for SyntheticProvider {
    fn passphrase(
        &mut self,
        _purpose: CredentialPurpose,
    ) -> anansi_core::vault::Result<Zeroizing<Vec<u8>>> {
        Ok(Zeroizing::new(self.passphrase.clone()))
    }
    fn recovery_key(
        &mut self,
        _display: &RecoveryDisplay,
    ) -> anansi_core::vault::Result<Zeroizing<[u8; 32]>> {
        Err(VaultError::AcknowledgementRequired)
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
        passphrase: b"synthetic passphrase".to_vec(),
    };
    let mut browser = caller(&workspace, &root_id);
    browser.transport = Transport::Browser;
    let err = match runtime.setup(&workspace, &browser, &mut provider, Instant::now(), true) {
        Ok(_) => panic!("setup unexpectedly succeeded"),
        Err(err) => err,
    };
    assert_eq!(err, VaultError::InvalidContext);
}
