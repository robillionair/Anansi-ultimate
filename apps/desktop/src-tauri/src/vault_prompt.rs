#![cfg(windows)]

use anansi_core::vault::{
    Result, VaultError,
    recovery::RecoveryDisplay,
    runtime::{CredentialPurpose, NativeCredentialProvider},
};
use std::ptr;
use windows_sys::Win32::Security::Credentials::{
    CREDUI_FLAGS_ALWAYS_SHOW_UI, CREDUI_FLAGS_DO_NOT_PERSIST, CREDUI_FLAGS_GENERIC_CREDENTIALS,
    CREDUI_INFOW, CredUIPromptForCredentialsW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IDYES, MB_DEFBUTTON2, MB_ICONINFORMATION, MB_YESNO, MessageBoxW,
};
use zeroize::Zeroizing;

pub struct WindowsCredentialProvider {
    target: String,
}

type PromptFields = (Zeroizing<Vec<u8>>, Zeroizing<Vec<u8>>);

impl WindowsCredentialProvider {
    pub fn new(workspace: &str) -> Self {
        Self {
            target: format!("ANANSI Vault — {workspace}"),
        }
    }

    fn prompt(&mut self, purpose: &str) -> Result<PromptFields> {
        let target: Vec<u16> = self
            .target
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let message: Vec<u16> = format!("ANANSI {purpose}. Credentials stay in the native prompt and are never sent to the web UI.").encode_utf16().chain(std::iter::once(0)).collect();
        let mut user = Zeroizing::new(vec![0u16; 512]);
        let mut password = Zeroizing::new(vec![0u16; 1024]);
        let mut save = 0i32;
        let info = CREDUI_INFOW {
            cbSize: std::mem::size_of::<CREDUI_INFOW>() as u32,
            hwndParent: ptr::null_mut(),
            pszMessageText: message.as_ptr(),
            pszCaptionText: target.as_ptr(),
            hbmBanner: ptr::null_mut(),
        };
        let result = unsafe {
            CredUIPromptForCredentialsW(
                &info,
                target.as_ptr(),
                ptr::null(),
                0,
                user.as_mut_ptr(),
                user.len() as u32,
                password.as_mut_ptr(),
                password.len() as u32,
                &mut save,
                CREDUI_FLAGS_GENERIC_CREDENTIALS
                    | CREDUI_FLAGS_DO_NOT_PERSIST
                    | CREDUI_FLAGS_ALWAYS_SHOW_UI,
            )
        };
        if result != 0 {
            return Err(VaultError::CredentialUnavailable);
        }
        let user_len = user.iter().position(|v| *v == 0).unwrap_or(user.len());
        let pass_len = password
            .iter()
            .position(|v| *v == 0)
            .unwrap_or(password.len());
        let username =
            String::from_utf16(&user[..user_len]).map_err(|_| VaultError::CredentialUnavailable)?;
        let secret = String::from_utf16(&password[..pass_len])
            .map_err(|_| VaultError::CredentialUnavailable)?;
        password.fill(0);
        user.fill(0);
        Ok((
            Zeroizing::new(username.into_bytes()),
            Zeroizing::new(secret.into_bytes()),
        ))
    }
}

impl NativeCredentialProvider for WindowsCredentialProvider {
    fn passphrase(&mut self, purpose: CredentialPurpose) -> Result<Zeroizing<Vec<u8>>> {
        #[cfg(all(debug_assertions, feature = "vault-test-provider"))]
        if std::env::var_os("ANANSI_VAULT_TEST_PROVIDER").as_deref()
            == Some(std::ffi::OsStr::new("1"))
        {
            let _ = purpose;
            return Ok(Zeroizing::new(
                b"ANANSI synthetic vault acceptance passphrase".to_vec(),
            ));
        }
        let label = match purpose {
            CredentialPurpose::Setup => "setup",
            CredentialPurpose::Unlock => "unlock",
            CredentialPurpose::Rotation => "key rotation",
            CredentialPurpose::Restore => "restore",
        };
        let secret = self.prompt(label)?.1;
        if secret.is_empty() {
            return Err(VaultError::CredentialUnavailable);
        }
        if purpose == CredentialPurpose::Setup {
            let confirmation = self.prompt("confirm the new vault passphrase")?.1;
            let different = secret.len() != confirmation.len()
                || secret
                    .iter()
                    .zip(confirmation.iter())
                    .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                    != 0;
            if different {
                return Err(VaultError::AuthenticationFailed);
            }
        }
        Ok(secret)
    }
    fn recovery_key(&mut self, display: &RecoveryDisplay) -> Result<Zeroizing<[u8; 32]>> {
        #[cfg(all(debug_assertions, feature = "vault-test-provider"))]
        if std::env::var_os("ANANSI_VAULT_TEST_PROVIDER").as_deref()
            == Some(std::ffi::OsStr::new("1"))
        {
            return anansi_core::vault::recovery::parse(display.text.as_str());
        }
        let caption: Vec<u16> = "ANANSI Recovery Key"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let message = Zeroizing::new(format!(
            "Save this recovery key outside ANANSI:\n\n{}\n\nHave you saved a separate copy? Choose Yes to acknowledge and retype it in the next native prompt. No cancels setup.",
            display.text.as_str()
        ));
        let body = Zeroizing::new(
            message
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect::<Vec<u16>>(),
        );
        let acknowledged = unsafe {
            MessageBoxW(
                ptr::null_mut(),
                body.as_ptr(),
                caption.as_ptr(),
                MB_YESNO | MB_DEFBUTTON2 | MB_ICONINFORMATION,
            )
        };
        if acknowledged != IDYES {
            return Err(VaultError::AcknowledgementRequired);
        }
        self.target.push_str(" — recovery acknowledgement");
        let (typed, _) = self
            .prompt("recovery acknowledgement: enter the displayed recovery key as the username")?;
        let text = Zeroizing::new(
            String::from_utf8(typed.to_vec()).map_err(|_| VaultError::AcknowledgementRequired)?,
        );
        anansi_core::vault::recovery::parse(text.as_str())
    }
}
