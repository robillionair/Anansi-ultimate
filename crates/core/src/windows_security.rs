//! Native Windows owner/DACL verification for proven reads.
//!
//! Mirrors exactly the verify-only invariant of `protect.ps1`:
//! the owner SID must equal the current user, the DACL must be
//! protected (inheritance disabled), at least one rule must exist,
//! and every rule must be (current user, Allow, FullControl).
//!
//! All checks are in-process Win32 calls, so a proven read can
//! re-validate current permissions without returning to a
//! PowerShell process per file. Any API failure fails closed.

use std::path::Path;
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, PSID},
    Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT},
    Security::{
        ACCESS_ALLOWED_ACE, ACL, ACL_SIZE_INFORMATION, AclSizeInformation,
        DACL_SECURITY_INFORMATION, EqualSid, GetAce, GetAclInformation, GetLengthSid,
        GetSecurityDescriptorControl, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
        GetTokenInformation, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED,
        TOKEN_QUERY, TOKEN_USER, TokenUser,
    },
    Storage::FileSystem::FILE_ALL_ACCESS,
    System::{
        Memory::LocalFree,
        SystemServices::ACCESS_ALLOWED_ACE_TYPE,
        Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

pub(crate) enum WindowsVerify {
    Ok,
    NotFound,
    Failed,
}

fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}

/// Raw bytes of the current process user's SID, copied out of the
/// token so no handle outlives this call.
pub(crate) fn current_user_sid() -> Option<Vec<u8>> {
    unsafe {
        let mut token: HANDLE = 0;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let mut len: u32 = 0;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len);
        if len == 0 {
            CloseHandle(token);
            return None;
        }
        let mut buf: Vec<u8> = vec![0; len as usize];
        let mut returned: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenUser,
            buf.as_mut_ptr() as *mut std::ffi::c_void,
            len,
            &mut returned,
        );
        CloseHandle(token);
        if ok == 0 {
            return None;
        }
        let user = &*(buf.as_ptr() as *const TOKEN_USER);
        let sid = user.User.Sid;
        if sid.is_null() {
            return None;
        }
        let sid_len = GetLengthSid(sid) as usize;
        if sid_len == 0 || sid_len > buf.len() {
            return None;
        }
        let mut out = vec![0u8; sid_len];
        std::ptr::copy_nonoverlapping(sid as *const u8, out.as_mut_ptr(), sid_len);
        Some(out)
    }
}

fn check_descriptor(sd: PSECURITY_DESCRIPTOR, user_sid: &[u8]) -> WindowsVerify {
    unsafe {
        let user_psid = user_sid.as_ptr() as PSID;
        let mut owner: PSID = std::ptr::null_mut();
        let mut owner_defaulted: i32 = 0;
        if GetSecurityDescriptorOwner(sd, &mut owner, &mut owner_defaulted) == 0
            || owner.is_null()
            || EqualSid(owner, user_psid) == 0
        {
            return WindowsVerify::Failed;
        }
        let mut control: u16 = 0;
        let mut revision: u32 = 0;
        if GetSecurityDescriptorControl(sd, &mut control, &mut revision) == 0
            || control & SE_DACL_PROTECTED == 0
        {
            return WindowsVerify::Failed;
        }
        let mut present: i32 = 0;
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut dacl_defaulted: i32 = 0;
        if GetSecurityDescriptorDacl(sd, &mut present, &mut dacl, &mut dacl_defaulted) == 0
            || present == 0
            || dacl.is_null()
        {
            return WindowsVerify::Failed;
        }
        let mut info = ACL_SIZE_INFORMATION {
            AceCount: 0,
            AclBytesInUse: 0,
            AclBytesFree: 0,
        };
        if GetAclInformation(
            dacl,
            &mut info as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        ) == 0
            || info.AceCount == 0
        {
            return WindowsVerify::Failed;
        }
        for i in 0..info.AceCount {
            let mut ace_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            if GetAce(dacl, i, &mut ace_ptr) == 0 || ace_ptr.is_null() {
                return WindowsVerify::Failed;
            }
            let ace = &*(ace_ptr as *const ACCESS_ALLOWED_ACE);
            if ace.Header.AceType as u32 != ACCESS_ALLOWED_ACE_TYPE
                || ace.Mask & FILE_ALL_ACCESS != FILE_ALL_ACCESS
                || EqualSid(&ace.SidStart as *const _ as PSID, user_psid) == 0
            {
                return WindowsVerify::Failed;
            }
        }
        WindowsVerify::Ok
    }
}

fn path_is_private(path: &Path, user_sid: &[u8]) -> WindowsVerify {
    unsafe {
        let wide = wide_path(path);
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let err = GetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut sd,
        );
        if err != 0 {
            return if err == 2 || err == 3 {
                WindowsVerify::NotFound
            } else {
                WindowsVerify::Failed
            };
        }
        let result = check_descriptor(sd, user_sid);
        LocalFree(sd as isize);
        result
    }
}

/// The file and its proven ancestor directories (record,
/// workspace, content) must all still satisfy the private
/// invariant at read time.
pub(crate) fn proven_paths_are_private(file: &Path, user_sid: &[u8]) -> WindowsVerify {
    let mut current = Some(file);
    for _ in 0..4 {
        let Some(path) = current else { break };
        match path_is_private(path, user_sid) {
            WindowsVerify::Ok => {}
            other => return other,
        }
        current = path.parent();
    }
    WindowsVerify::Ok
}
