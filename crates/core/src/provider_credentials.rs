//! Approved credential setup for providers (MASTER_SPEC §15):
//! the owner pastes an API key into a TRUSTED NATIVE prompt on
//! the host; the key moves from the prompt into the OS credential
//! store (the providers keyring service) inside this process. It
//! never crosses an HTTP/IPC DTO, is never accepted from a
//! browser session, and is never written to ANANSI's files, logs
//! or configuration — configuration holds only the reference.
//!
//! This mirrors the vault's native-prompt architecture
//! (`vault/native_prompt.rs`): native controls on Windows, a
//! debug-only test provider selected by environment (the same
//! pattern as `ANANSI_VAULT_TEST_PROVIDER`), and a hard refusal
//! everywhere else. Operations are authorized by the caller
//! (host/owner only, enforced in `provider_dispatch`).

use crate::readiness::{CommandError, ErrorCode};
use zeroize::Zeroizing;

fn invalid(message: &str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn unavailable(message: &str) -> CommandError {
    CommandError::new(ErrorCode::StorageUnavailable, message)
}

/// Validate a credential reference (the keyring account label):
/// bounded, printable, no whitespace surprises.
pub fn validate_reference(reference: &str) -> Result<(), CommandError> {
    if reference.is_empty() || reference.len() > 128 {
        return Err(invalid(
            "Use a credential reference of 1 to 128 characters.",
        ));
    }
    if reference
        .chars()
        .any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(invalid(
            "Credential references must not contain whitespace or control characters.",
        ));
    }
    Ok(())
}

/// Obtain an API key from the owner through the trusted native
/// prompt. The returned buffer is zeroized on drop; callers store
/// it into the keyring service immediately and never copy it
/// anywhere else.
pub fn prompt_for_api_key(
    provider: &str,
    reference: &str,
) -> Result<Zeroizing<Vec<u8>>, CommandError> {
    // Debug-only test provider, mirroring the vault's pattern:
    // compiled out of release builds entirely.
    #[cfg(debug_assertions)]
    if std::env::var_os("ANANSI_PROVIDER_TEST_CANCEL").as_deref() == Some(std::ffi::OsStr::new("1"))
    {
        // The native prompt's cancel path (ERROR_CANCELLED on
        // Windows): no key is produced, nothing is stored.
        return Err(invalid("Credential entry was cancelled."));
    }
    #[cfg(debug_assertions)]
    if let Some(value) = std::env::var_os("ANANSI_PROVIDER_TEST_CREDENTIAL") {
        return Ok(Zeroizing::new(
            value.to_string_lossy().into_owned().into_bytes(),
        ));
    }
    #[cfg(windows)]
    {
        windows_prompt(provider, reference)
    }
    #[cfg(not(windows))]
    {
        let _ = (provider, reference);
        Err(unavailable(
            "The native credential prompt is available in the ANANSI desktop app on Windows.",
        ))
    }
}

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
const ERROR_CANCELLED: u32 = 1223;

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
fn windows_prompt(provider: &str, reference: &str) -> Result<Zeroizing<Vec<u8>>, CommandError> {
    use std::ptr;
    let caption: Vec<u16> = format!("ANANSI Provider Credential — {provider}")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let message: Vec<u16> = format!(
        "Paste the API key for {provider}.\n\n\
         It is stored in the Windows credential store under the reference \
         '{reference}' for ANANSI's provider service. It is never sent to \
         the browser, written to ANANSI's files, or shown again."
    )
    .encode_utf16()
    .chain(std::iter::once(0))
    .collect();
    let mut user = Zeroizing::new(vec![0u16; 512]);
    for (slot, value) in user.iter_mut().zip("ANANSI provider key".encode_utf16()) {
        *slot = value;
    }
    let mut password = Zeroizing::new(vec![0u16; 1024]);
    let mut save = 0i32;
    let info = CreduiInfoW {
        cb_size: std::mem::size_of::<CreduiInfoW>() as u32,
        hwnd_parent: ptr::null_mut(),
        psz_message_text: message.as_ptr(),
        psz_caption_text: caption.as_ptr(),
        hbm_banner: ptr::null_mut(),
    };
    let result = unsafe {
        CredUIPromptForCredentialsW(
            &info,
            caption.as_ptr(),
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
    if result == ERROR_CANCELLED {
        password.fill(0);
        return Err(invalid("Credential entry was cancelled."));
    }
    if result != 0 {
        password.fill(0);
        return Err(unavailable("The native credential prompt is unavailable."));
    }
    let pass_len = password
        .iter()
        .position(|v| *v == 0)
        .unwrap_or(password.len());
    let secret = String::from_utf16(&password[..pass_len])
        .map_err(|_| unavailable("The credential prompt returned unreadable input."))?;
    password.fill(0);
    user.fill(0);
    if secret.is_empty() {
        return Err(invalid("No API key was entered."));
    }
    Ok(Zeroizing::new(secret.into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_env_returns_the_cancelled_error() {
        // The native prompt's cancel path, via the debug-only
        // test hook (mirrors the vault's ANANSI_VAULT_TEST_CANCEL).
        // The lib test binary has no other prompt caller, so the
        // process-global env mutation is safe here.
        unsafe {
            std::env::set_var("ANANSI_PROVIDER_TEST_CANCEL", "1");
        }
        let err = prompt_for_api_key("openai", "openai-test").unwrap_err();
        unsafe {
            std::env::remove_var("ANANSI_PROVIDER_TEST_CANCEL");
        }
        assert_eq!(err.code, ErrorCode::InvalidRequest);
        assert!(err.message.contains("cancelled"), "{}", err.message);
    }
}
