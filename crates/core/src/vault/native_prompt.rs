//! Native OS credential and recovery-key prompts.
//!
//! Credentials stay in native controls and guarded `Zeroizing` buffers; they
//! never cross IPC or HTTP DTOs into the browser or renderer.
use super::{
    Result, VaultError,
    recovery::RecoveryDisplay,
    runtime::{CredentialPurpose, NativeCredentialProvider},
};
use zeroize::Zeroizing;

#[cfg(windows)]
use std::ptr;

#[cfg(windows)]
#[repr(C)]
struct CreduiInfoW {
    cb_size: u32,
    hwnd_parent: *mut core::ffi::c_void,
    psz_message_text: *const u16,
    psz_caption_text: *const u16,
    hbm_banner: *mut core::ffi::c_void,
}

#[cfg(windows)]
const CREDUI_FLAGS_GENERIC_CREDENTIALS: u32 = 0x40000;
#[cfg(windows)]
const CREDUI_FLAGS_ALWAYS_SHOW_UI: u32 = 0x80;
#[cfg(windows)]
const CREDUI_FLAGS_DO_NOT_PERSIST: u32 = 0x2;
#[cfg(windows)]
const MB_YESNO: u32 = 0x00000004;
#[cfg(windows)]
const MB_ICONINFORMATION: u32 = 0x00000040;
#[cfg(windows)]
const MB_DEFBUTTON2: u32 = 0x00000100;
#[cfg(windows)]
const IDYES: i32 = 6;

#[cfg(windows)]
#[link(name = "credui")]
unsafe extern "system" {
    fn CredUIPromptForCredentialsW(
        p_ui_info: *const CreduiInfoW,
        psz_target_name: *const u16,
        p_context: *const core::ffi::c_void,
        dw_auth_error: u32,
        psz_user_name: *mut u16,
        ul_user_name_buffer_max_chars: u32,
        psz_password: *mut u16,
        ul_password_buffer_max_chars: u32,
        pf_save: *mut i32,
        dw_flags: u32,
    ) -> u32;
}

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(
        h_wnd: *mut core::ffi::c_void,
        lp_text: *const u16,
        lp_caption: *const u16,
        u_type: u32,
    ) -> i32;
}

pub struct WindowsCredentialProvider {
    workspace: String,
    target: String,
    browser_origin: Option<String>,
    browser_confirmed: bool,
}

type PromptFields = (Zeroizing<Vec<u8>>, Zeroizing<Vec<u8>>);

impl WindowsCredentialProvider {
    pub fn new(workspace: &str) -> Self {
        Self {
            workspace: workspace.to_owned(),
            target: format!("ANANSI Vault — {workspace}"),
            browser_origin: None,
            browser_confirmed: false,
        }
    }

    pub fn for_browser(workspace: &str, origin: &str) -> Self {
        Self {
            workspace: workspace.to_owned(),
            target: format!("ANANSI Browser Vault — {workspace} ({origin})"),
            browser_origin: Some(origin.to_owned()),
            browser_confirmed: false,
        }
    }

    #[cfg(windows)]
    fn ensure_browser_origin_confirmed(&mut self, purpose: &str) -> Result<()> {
        let Some(origin) = self.browser_origin.as_deref() else {
            return Ok(());
        };
        if self.browser_confirmed {
            return Ok(());
        }
        let caption: Vec<u16> = "ANANSI Paired Browser Vault Request"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let message = format!(
            "Paired browser origin {origin} requested ANANSI vault {purpose} for workspace {}.\n\nApprove this request and open the native credential prompt?",
            self.workspace
        );
        let body: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
        let confirmed = unsafe {
            MessageBoxW(
                ptr::null_mut(),
                body.as_ptr(),
                caption.as_ptr(),
                MB_YESNO | MB_DEFBUTTON2 | MB_ICONINFORMATION,
            )
        };
        if confirmed != IDYES {
            return Err(VaultError::CredentialUnavailable);
        }
        self.browser_confirmed = true;
        Ok(())
    }

    #[cfg(windows)]
    fn prompt(&mut self, purpose: &str, recovery: bool) -> Result<PromptFields> {
        self.ensure_browser_origin_confirmed(purpose)?;
        let target: Vec<u16> = self
            .target
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let message: Vec<u16> =
            prompt_instructions(purpose, recovery, self.browser_origin.as_deref())
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
        let mut user = Zeroizing::new(vec![0u16; 512]);
        // The generic Win32 control has two fields. ANANSI has no account
        // username here; prefill its unused field to avoid implying OS login.
        for (slot, value) in user.iter_mut().zip("ANANSI vault".encode_utf16()) {
            *slot = value;
        }
        let mut password = Zeroizing::new(vec![0u16; 1024]);
        let mut save = 0i32;
        let info = CreduiInfoW {
            cb_size: std::mem::size_of::<CreduiInfoW>() as u32,
            hwnd_parent: ptr::null_mut(),
            psz_message_text: message.as_ptr(),
            psz_caption_text: target.as_ptr(),
            hbm_banner: ptr::null_mut(),
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
            password.fill(0);
            user.fill(0);
            return Err(VaultError::CredentialUnavailable);
        }
        let user_len = user.iter().position(|v| *v == 0).unwrap_or(user.len());
        let pass_len = password
            .iter()
            .position(|v| *v == 0)
            .unwrap_or(password.len());
        let username = String::from_utf16(&user[..user_len]);
        let secret = String::from_utf16(&password[..pass_len]);
        password.fill(0);
        user.fill(0);
        let username = username.map_err(|_| VaultError::CredentialUnavailable)?;
        let secret = secret.map_err(|_| VaultError::CredentialUnavailable)?;
        Ok((
            Zeroizing::new(username.into_bytes()),
            Zeroizing::new(secret.into_bytes()),
        ))
    }
}

impl NativeCredentialProvider for WindowsCredentialProvider {
    fn passphrase(&mut self, purpose: CredentialPurpose) -> Result<Zeroizing<Vec<u8>>> {
        #[cfg(debug_assertions)]
        if std::env::var_os("ANANSI_VAULT_TEST_PROVIDER").as_deref()
            == Some(std::ffi::OsStr::new("1"))
        {
            if std::env::var_os("ANANSI_VAULT_TEST_CANCEL").as_deref()
                == Some(std::ffi::OsStr::new("1"))
            {
                return Err(VaultError::CredentialUnavailable);
            }
            if purpose == CredentialPurpose::Restore
                && let Ok(src) = std::env::var("ANANSI_VAULT_TEST_SOURCE_PASSPHRASE")
            {
                return Ok(Zeroizing::new(src.into_bytes()));
            }
            if purpose == CredentialPurpose::Rotation
                && let Ok(rot) = std::env::var("ANANSI_VAULT_TEST_ROTATE_PASSPHRASE")
            {
                return Ok(Zeroizing::new(rot.into_bytes()));
            }
            if let Ok(custom) = std::env::var("ANANSI_VAULT_TEST_PASSPHRASE") {
                return Ok(Zeroizing::new(custom.into_bytes()));
            }
            return Ok(Zeroizing::new(
                b"ANANSI synthetic vault acceptance passphrase".to_vec(),
            ));
        }
        #[cfg(windows)]
        {
            let label = match purpose {
                CredentialPurpose::Setup => "setup",
                CredentialPurpose::Unlock => "unlock",
                CredentialPurpose::Rotation => "key rotation",
                CredentialPurpose::Restore => "restore",
            };
            let secret = self.prompt(label, false)?.1;
            if secret.is_empty() {
                return Err(VaultError::CredentialUnavailable);
            }
            if purpose == CredentialPurpose::Setup {
                let confirmation = self.prompt("confirm the new vault passphrase", false)?.1;
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
        #[cfg(not(windows))]
        {
            let _ = purpose;
            Err(VaultError::CredentialUnavailable)
        }
    }

    fn recovery_key(&mut self, display: &RecoveryDisplay) -> Result<Zeroizing<[u8; 32]>> {
        #[cfg(debug_assertions)]
        if std::env::var_os("ANANSI_VAULT_TEST_PROVIDER").as_deref()
            == Some(std::ffi::OsStr::new("1"))
        {
            if std::env::var_os("ANANSI_VAULT_TEST_CANCEL").as_deref()
                == Some(std::ffi::OsStr::new("1"))
            {
                return Err(VaultError::CredentialUnavailable);
            }
            if std::env::var_os("ANANSI_VAULT_TEST_WRONG_RECOVERY").as_deref()
                == Some(std::ffi::OsStr::new("1"))
            {
                return Err(VaultError::AcknowledgementRequired);
            }
            if let Ok(path) = std::env::var("ANANSI_VAULT_TEST_RECOVERY_OUT") {
                let _ = std::fs::write(path, display.text.as_bytes());
            }
            return super::recovery::parse(display.text.as_str());
        }
        #[cfg(windows)]
        {
            let caption: Vec<u16> = "ANANSI Recovery Key"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let message = Zeroizing::new(format!(
                "Save this recovery key outside ANANSI:\n\n{}\n\nHave you saved a separate copy? Choose Yes, then enter THIS RECOVERY KEY in the next prompt's PASSWORD field. Do NOT repeat your vault passphrase. Leave the prefilled username unchanged. No cancels setup.",
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
            parse_recovery_input(self.prompt("RECOVERY KEY acknowledgement", true)?)
        }
        #[cfg(not(windows))]
        {
            let _ = display;
            Err(VaultError::CredentialUnavailable)
        }
    }

    fn source_recovery_key(&mut self) -> Result<Zeroizing<[u8; 32]>> {
        #[cfg(debug_assertions)]
        if std::env::var_os("ANANSI_VAULT_TEST_PROVIDER").as_deref()
            == Some(std::ffi::OsStr::new("1"))
        {
            if std::env::var_os("ANANSI_VAULT_TEST_CANCEL").as_deref()
                == Some(std::ffi::OsStr::new("1"))
            {
                return Err(VaultError::CredentialUnavailable);
            }
            let text = std::env::var("ANANSI_VAULT_TEST_SOURCE_RECOVERY")
                .map_err(|_| VaultError::AuthenticationFailed)?;
            return super::recovery::parse(&text);
        }
        #[cfg(windows)]
        {
            parse_recovery_input(self.prompt("source vault RECOVERY KEY restore", true)?)
        }
        #[cfg(not(windows))]
        {
            Err(VaultError::CredentialUnavailable)
        }
    }
}

pub fn parse_recovery_input((_, typed): PromptFields) -> Result<Zeroizing<[u8; 32]>> {
    let text = std::str::from_utf8(&typed).map_err(|_| VaultError::AcknowledgementRequired)?;
    super::recovery::parse(text).map_err(|_| VaultError::AcknowledgementRequired)
}

pub fn prompt_instructions(purpose: &str, recovery: bool, origin: Option<&str>) -> String {
    let prefix = match origin {
        Some(origin) => format!("Paired browser ({origin}) — "),
        None => String::new(),
    };
    if recovery {
        format!(
            "{prefix}RECOVERY KEY CHECK — not a login. Leave the prefilled username unchanged. In Password, enter the recovery key you just saved, including its groups if shown. Do NOT enter your vault passphrase or Windows password. Cancel abandons setup."
        )
    } else {
        format!(
            "{prefix}ANANSI {purpose}. Leave the prefilled username unchanged. In Password, enter your ANANSI vault passphrase, NOT your Windows password. Credentials stay in this native control and never enter the web UI."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_uses_password_field_and_refuses_repeated_login_credentials() {
        let key = [42; 32];
        let text = super::super::recovery::format(&key);
        let accepted = parse_recovery_input((
            Zeroizing::new(b"ANANSI vault".to_vec()),
            Zeroizing::new(text.as_bytes().to_vec()),
        ))
        .unwrap();
        assert_eq!(*accepted, key);
        assert_eq!(
            parse_recovery_input((
                Zeroizing::new(text.as_bytes().to_vec()),
                Zeroizing::new(b"original vault passphrase".to_vec())
            ))
            .unwrap_err(),
            VaultError::AcknowledgementRequired
        );
    }

    #[test]
    fn browser_prompt_instructions_name_requesting_origin() {
        let msg = prompt_instructions("unlock", false, Some("http://127.0.0.1:1420"));
        assert!(msg.contains("Paired browser (http://127.0.0.1:1420)"));
        assert!(msg.contains("never enter the web UI"));
    }
}
