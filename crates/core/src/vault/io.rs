//! Operation-scoped ACL verification; same protect.ps1 invariant in one process.
use super::{Result, VaultError};
#[cfg(not(windows))]
use crate::storage::private_permissions;
use crate::storage::reject_links;
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) struct VerifiedFiles {
    files: BTreeSet<PathBuf>,
}
impl VerifiedFiles {
    pub(crate) fn verify(paths: &[(PathBuf, bool)], protect_new: bool) -> Result<Self> {
        if paths.len() > 200_000 {
            return Err(VaultError::ResourceLimit);
        }
        for (path, _) in paths {
            reject_links(path).map_err(|_| VaultError::StorageUnavailable)?;
        }
        #[cfg(windows)]
        {
            use std::{
                os::windows::process::CommandExt,
                process::{Command, Stdio},
            };
            const SCRIPT: &str = concat!(
                "$ErrorActionPreference='Stop'; [Console]::InputEncoding=[Text.UTF8Encoding]::new($false); ",
                "$items=ConvertFrom-Json ([Console]::In.ReadToEnd()); foreach($item in $items) { ",
                "$env:ANANSI_PERMISSION_PATH=$item.path; $env:ANANSI_PERMISSION_DIRECTORY=if($item.directory){'1'}else{'0'}; ",
                "$env:ANANSI_PERMISSION_VERIFY=if($item.create){'0'}else{'1'}; & {\n",
                include_str!("../protect.ps1"),
                "\n}; }"
            );
            let request:Vec<_>=paths.iter().map(|(path,directory)|serde_json::json!({"path":path,"directory":directory,"create":protect_new})).collect();
            let input = serde_json::to_vec(&request).map_err(|_| VaultError::StorageUnavailable)?;
            let mut child = Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
                .creation_flags(0x08000000)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|_| VaultError::StorageUnavailable)?;
            let written = child
                .stdin
                .take()
                .ok_or(VaultError::StorageUnavailable)?
                .write_all(&input);
            let result = child
                .wait_with_output()
                .map_err(|_| VaultError::StorageUnavailable)?;
            if written.is_err() || !result.status.success() {
                return Err(VaultError::StorageUnavailable);
            }
        }
        #[cfg(not(windows))]
        for (path, directory) in paths {
            private_permissions(path, *directory, !protect_new)
                .map_err(|_| VaultError::StorageUnavailable)?;
        }
        Ok(Self {
            files: paths
                .iter()
                .filter(|(_, directory)| !*directory)
                .map(|(path, _)| path.clone())
                .collect(),
        })
    }
    pub(crate) fn read(&self, path: &Path, maximum: usize) -> Result<Vec<u8>> {
        if !self.files.contains(path) {
            return Err(VaultError::StorageUnavailable);
        }
        reject_links(path).map_err(|_| VaultError::StorageUnavailable)?;
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|_| VaultError::StorageUnavailable)?
            .take((maximum + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| VaultError::StorageUnavailable)?;
        if bytes.len() > maximum {
            return Err(VaultError::ResourceLimit);
        }
        Ok(bytes)
    }
}
/// Only ciphertext is written before batched leaf-DACL protection. Parent is
/// already private; inherited owner-only access is narrowed/protected before use.
pub(crate) fn write_cipher(path: &Path, bytes: &[u8]) -> Result<()> {
    reject_links(path).map_err(|_| VaultError::StorageUnavailable)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|_| VaultError::StorageUnavailable)?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| VaultError::StorageUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn batched_acl_proof_rejects_overbroad_file_access() {
        let temp = tempfile::tempdir().unwrap();
        crate::storage::private_permissions(temp.path(), true, false).unwrap();
        let first = temp.path().join("first.ansv");
        let second = temp.path().join("second.ansv");
        write_cipher(&first, b"synthetic ciphertext").unwrap();
        write_cipher(&second, b"synthetic ciphertext").unwrap();
        let files = [(first.clone(), false), (second.clone(), false)];
        VerifiedFiles::verify(&files, true).unwrap();
        assert_eq!(
            VerifiedFiles::verify(&files, false)
                .unwrap()
                .read(&first, 64)
                .unwrap(),
            b"synthetic ciphertext"
        );
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let status = std::process::Command::new("icacls.exe")
                .arg(&second)
                .args(["/grant", "*S-1-1-0:(R)"])
                .creation_flags(0x08000000)
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap();
            assert!(status.success());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&second, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        assert!(VerifiedFiles::verify(&files, false).is_err());
        crate::storage::private_permissions(&second, false, false).unwrap();
        assert!(VerifiedFiles::verify(&files, false).is_ok());
    }
}
