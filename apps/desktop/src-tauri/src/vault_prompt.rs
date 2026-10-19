#![cfg(windows)]

use anansi_core::vault::{Result, VaultError, recovery::RecoveryDisplay, runtime::{CredentialPurpose, NativeCredentialProvider}};
use std::ptr;
use windows_sys::Win32::Security::Credentials::{CredUIPromptForCredentialsW, CREDUI_INFOW, CREDUIWIN_GENERIC};
use zeroize::Zeroizing;

pub struct WindowsCredentialProvider {
    target: String,
}

impl WindowsCredentialProvider {
    pub fn new(workspace: &str) -> Self { Self { target: format!("ANANSI Vault — {workspace}") } }

    fn prompt(&mut self, purpose: &str) -> Result<(Zeroizing<Vec<u8>>, Zeroizing<Vec<u8>>)> {
        let mut target: Vec<u16> = self.target.encode_utf16().chain(std::iter::once(0)).collect();
        let mut message: Vec<u16> = format!("ANANSI {purpose}. Credentials stay in the native prompt and are never sent to the web UI.").encode_utf16().chain(std::iter::once(0)).collect();
        let mut user = vec![0u16; 512];
        let mut password = vec![0u16; 1024];
        let mut save = 0i32;
        let info = CREDUI_INFOW { cbSize: std::mem::size_of::<CREDUI_INFOW>() as u32, hwndParent: 0, pszMessageText: message.as_ptr(), pszCaptionText: target.as_ptr(), hbmBanner: 0 };
        let result = unsafe { CredUIPromptForCredentialsW(&info, target.as_ptr(), ptr::null(), 0, user.as_mut_ptr(), user.len() as u32, password.as_mut_ptr(), password.len() as u32, &mut save, CREDUIWIN_GENERIC) };
        if result != 0 { return Err(VaultError::CredentialUnavailable); }
        let user_len = user.iter().position(|v| *v == 0).unwrap_or(user.len());
        let pass_len = password.iter().position(|v| *v == 0).unwrap_or(password.len());
        let username = String::from_utf16(&user[..user_len]).map_err(|_| VaultError::CredentialUnavailable)?;
        let secret = String::from_utf16(&password[..pass_len]).map_err(|_| VaultError::CredentialUnavailable)?;
        for v in &mut password { *v = 0; }
        Ok((Zeroizing::new(username.into_bytes()), Zeroizing::new(secret.into_bytes())))
    }
}

impl NativeCredentialProvider for WindowsCredentialProvider {
    fn passphrase(&mut self, purpose: CredentialPurpose) -> Result<Zeroizing<Vec<u8>>> {
        let label = match purpose { CredentialPurpose::Setup => "setup", CredentialPurpose::Unlock => "unlock", CredentialPurpose::Rotation => "key rotation", CredentialPurpose::Restore => "restore" };
        Ok(self.prompt(label)?.1)
    }
    fn recovery_key(&mut self, display: &RecoveryDisplay) -> Result<Zeroizing<[u8; 32]>> {
        self.target.push_str(" — recovery key shown in the preceding native dialog");
        let (typed, _) = self.prompt("recovery acknowledgement: enter the displayed recovery key as the username")?;
        let text = String::from_utf8(typed.to_vec()).map_err(|_| VaultError::AcknowledgementRequired)?;
        anansi_core::vault::recovery::parse(&text)
    }
}
