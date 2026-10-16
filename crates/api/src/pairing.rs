use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingInfo {
    pub endpoint: String,
    pub token: String,
    pub contract_version: u8,
}
pub struct Session {
    pub directory: PathBuf,
    pub file: PathBuf,
    pub token: String,
}
fn random_hex() -> io::Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(io::Error::other)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn reject_links(path: &Path) -> io::Result<()> {
    for p in path.ancestors() {
        if let Ok(meta) = fs::symlink_metadata(p) {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err(io::Error::other(
                        "Pairing path must not contain reparse points",
                    ));
                }
            }
            if meta.file_type().is_symlink() {
                return Err(io::Error::other("Pairing path must not contain symlinks"));
            }
        }
    }
    Ok(())
}
pub fn protect(path: &Path, directory: bool) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let result = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                include_str!("protect.ps1"),
            ])
            .env("ANANSI_PERMISSION_PATH", path)
            .env(
                "ANANSI_PERMISSION_DIRECTORY",
                if directory { "1" } else { "0" },
            )
            .creation_flags(0x08000000)
            .output()?;
        if !result.status.success() {
            return Err(io::Error::other(
                "Cannot enforce private pairing ACL; session disabled",
            ));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
        )?;
    }
    Ok(())
}
impl Session {
    pub fn create(root: &Path, port: u16) -> io::Result<Self> {
        if !root.is_absolute() {
            return Err(io::Error::other("Pairing root must be absolute"));
        }
        reject_links(root)?;
        fs::create_dir_all(root)?;
        let directory = root.join(format!("session-{}", random_hex()?));
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new().mode(0o700).create(&directory)?;
        }
        #[cfg(not(unix))]
        fs::create_dir(&directory)?;
        if let Err(e) = protect(&directory, true) {
            let _ = fs::remove_dir(&directory);
            return Err(e);
        }
        let file = directory.join("pairing.json");
        let mut session = Self {
            directory,
            file,
            token: String::new(),
        };
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut handle = options.open(&session.file)?;
        protect(&session.file, false)?;
        session.token = random_hex()?;
        let info = PairingInfo {
            endpoint: format!("http://127.0.0.1:{port}"),
            token: session.token.clone(),
            contract_version: 1,
        };
        handle.write_all(&serde_json::to_vec(&info)?)?;
        handle.sync_all()?;
        Ok(session)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
        let _ = fs::remove_dir(&self.directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pairing_is_private_unique_and_cleaned_on_shutdown() {
        let temp = tempfile::tempdir().unwrap();
        let s = Session::create(temp.path(), 1451).unwrap();
        let t = Session::create(temp.path(), 1451).unwrap();
        assert_ne!(s.token, t.token);
        assert_eq!(s.token.len(), 64);
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&s.file).unwrap()).unwrap();
        assert_eq!(value["token"], s.token);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&s.file).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(&s.directory).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        // On Windows protect() independently reads back and validates every ACL entry before writing secrets.
        let file = s.file.clone();
        drop(s);
        assert!(!file.exists());
    }
    #[test]
    fn rejects_relative_paths() {
        assert!(Session::create(Path::new("relative"), 1451).is_err());
    }
    #[test]
    fn permission_failure_is_closed_and_nonexistent_file_is_not_created() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");
        assert!(protect(&missing, false).is_err());
        assert!(!missing.exists());
    }
    #[cfg(windows)]
    #[test]
    fn rejects_windows_junction_ancestors() {
        let temp = tempfile::tempdir().unwrap();
        let alias = temp.path().join("alias");
        let result = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "New-Item -ItemType Junction -Path $env:ANANSI_TEST_ALIAS -Target $env:ANANSI_TEST_TARGET | Out-Null"])
            .env("ANANSI_TEST_ALIAS", &alias).env("ANANSI_TEST_TARGET", temp.path()).output().unwrap();
        assert!(result.status.success());
        assert!(Session::create(&alias.join("nested"), 1451).is_err());
        fs::remove_dir(&alias).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlink_paths() {
        let temp = tempfile::tempdir().unwrap();
        let alias = temp.path().join("alias");
        std::os::unix::fs::symlink(temp.path(), &alias).unwrap();
        assert!(Session::create(&alias, 1451).is_err());
    }
}
