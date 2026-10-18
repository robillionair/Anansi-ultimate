//! Versioned, workspace-scoped backup packages.
//!
//! The package is deliberately a directory containing a manifest and immutable
//! body copies.  It is inspectable without ANANSI and can be copied atomically
//! by the caller.  Restore never writes to the source package or an existing
//! workspace.

use crate::{CommandError, ErrorCode, storage::reject_links, workspace::validate_id};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const BACKUP_FORMAT: &str = "anansi-workspace-backup";
pub const BACKUP_VERSION: u32 = 1;
pub const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupMember {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupManifest {
    pub format: String,
    pub format_version: u32,
    pub workspace_id: String,
    pub workspace_name: String,
    pub created_at: String,
    pub members: Vec<BackupMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub manifest: BackupManifest,
    pub verified: bool,
    pub total_bytes: u64,
}

fn invalid(message: &'static str) -> CommandError {
    CommandError::new(ErrorCode::InvalidRequest, message)
}

fn safe_member(path: &str) -> Result<PathBuf, CommandError> {
    let candidate = Path::new(path);
    if path.is_empty()
        || candidate.is_absolute()
        || candidate.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(invalid("Backup member path is unsafe."));
    }
    Ok(candidate.to_path_buf())
}

fn digest(path: &Path) -> Result<(u64, String), CommandError> {
    let bytes = fs::read(path).map_err(|_| invalid("Backup member cannot be read."))?;
    let mut hash = Sha256::new();
    hash.update(&bytes);
    Ok((bytes.len() as u64, format!("{:x}", hash.finalize())))
}

pub fn write_manifest(root: &Path, manifest: &BackupManifest) -> Result<(), CommandError> {
    validate_id(&manifest.workspace_id)?;
    if manifest.format != BACKUP_FORMAT || manifest.format_version != BACKUP_VERSION {
        return Err(invalid("Unsupported backup format."));
    }
    reject_links(root).map_err(|_| invalid("Backup path contains a link."))?;
    fs::create_dir_all(root).map_err(|_| invalid("Backup destination cannot be created."))?;
    let data = serde_json::to_vec_pretty(manifest)
        .map_err(|_| invalid("Backup manifest cannot be encoded."))?;
    if data.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(invalid("Backup manifest is too large."));
    }
    fs::write(root.join("manifest.json"), data)
        .map_err(|_| invalid("Backup manifest cannot be written."))
}

pub fn read_and_verify(root: &Path) -> Result<BackupPreview, CommandError> {
    reject_links(root).map_err(|_| invalid("Backup path contains a link."))?;
    let manifest_path = root.join("manifest.json");
    let data = fs::read(&manifest_path).map_err(|_| invalid("Backup manifest is missing."))?;
    if data.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(invalid("Backup manifest is too large."));
    }
    let manifest: BackupManifest =
        serde_json::from_slice(&data).map_err(|_| invalid("Backup manifest is invalid."))?;
    validate_id(&manifest.workspace_id)?;
    if manifest.format != BACKUP_FORMAT || manifest.format_version != BACKUP_VERSION {
        return Err(invalid("Unsupported backup format."));
    }
    let mut total = 0u64;
    let mut seen = std::collections::HashSet::new();
    for member in &manifest.members {
        if !seen.insert(member.path.clone()) {
            return Err(invalid("Backup contains duplicate members."));
        }
        let relative = safe_member(&member.path)?;
        let path = root.join(relative);
        reject_links(&path).map_err(|_| invalid("Backup member path contains a link."))?;
        let (bytes, hash) = digest(&path)?;
        if bytes != member.bytes || hash != member.sha256 {
            return Err(invalid("Backup member integrity check failed."));
        }
        total = total
            .checked_add(bytes)
            .ok_or_else(|| invalid("Backup is too large."))?;
    }
    Ok(BackupPreview {
        manifest,
        verified: true,
        total_bytes: total,
    })
}

pub fn copy_member(
    source: &Path,
    root: &Path,
    relative: &str,
) -> Result<BackupMember, CommandError> {
    let relative_path = safe_member(relative)?;
    let destination = root.join(&relative_path);
    if destination.exists() {
        return Err(invalid("Backup member overwrite is refused."));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|_| invalid("Backup member directory cannot be created."))?;
    }
    fs::copy(source, &destination).map_err(|_| invalid("Backup member cannot be copied."))?;
    let (bytes, sha256) = digest(&destination)?;
    Ok(BackupMember {
        path: relative.to_owned(),
        bytes,
        sha256,
    })
}
