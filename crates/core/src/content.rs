use crate::{
    CommandError, ErrorCode,
    storage::{private_permissions, reject_links, unavailable},
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub const MAX_BODY: usize = 256 * 1024;
pub fn integrity() -> CommandError {
    CommandError::new(
        ErrorCode::ContentIntegrity,
        "Memory content failed integrity verification. Preserve the storage root and restore a verified copy; no replacement was made.",
    )
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn random_id() -> Result<String, CommandError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| unavailable())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn location(
    root: &Path,
    workspace: &str,
    record: &str,
    revision: &str,
    create: bool,
) -> Result<PathBuf, CommandError> {
    for id in [workspace, record, revision] {
        crate::workspace::validate_id(id)?;
    }
    let mut directory = root.to_path_buf();
    for part in ["content", workspace, record] {
        directory.push(part);
        reject_links(&directory).map_err(|_| unavailable())?;
        if create && !directory.exists() {
            match fs::create_dir(&directory) {
                Ok(()) => {
                    private_permissions(&directory, true, false).map_err(|_| unavailable())?
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(unavailable()),
            }
        }
        private_permissions(&directory, true, true).map_err(|_| unavailable())?;
    }
    let file = directory.join(format!("{revision}.md"));
    reject_links(&file).map_err(|_| unavailable())?;
    Ok(file)
}
pub fn write(
    root: &Path,
    workspace: &str,
    record: &str,
    revision: &str,
    body: &str,
) -> Result<String, CommandError> {
    let file = location(root, workspace, record, revision, true)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut handle = options.open(&file).map_err(|_| unavailable())?;
    private_permissions(&file, false, false).map_err(|_| unavailable())?;
    handle
        .write_all(body.as_bytes())
        .and_then(|_| handle.sync_all())
        .map_err(|_| unavailable())?;
    Ok(hash(body.as_bytes()))
}
pub fn read(
    root: &Path,
    workspace: &str,
    record: &str,
    revision: &str,
    expected: &str,
) -> Result<String, CommandError> {
    let file = location(root, workspace, record, revision, false)?;
    if !file.exists() {
        return Err(integrity());
    }
    private_permissions(&file, false, true).map_err(|_| unavailable())?;
    let mut bytes = Vec::new();
    fs::File::open(file)
        .map_err(|_| integrity())?
        .take((MAX_BODY + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| integrity())?;
    if bytes.len() > MAX_BODY || hash(&bytes) != expected {
        return Err(integrity());
    }
    String::from_utf8(bytes).map_err(|_| integrity())
}
