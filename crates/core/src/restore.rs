//! Safe validation and fresh-target materialization for backup packages.

use crate::{
    CommandError,
    backup::{BackupPreview, read_and_verify},
    storage::reject_links,
};
use std::{fs, path::Path};

pub fn preview(root: &Path) -> Result<BackupPreview, CommandError> {
    read_and_verify(root)
}

pub fn restore_to_new_target(source: &Path, target: &Path) -> Result<BackupPreview, CommandError> {
    if source == target {
        return Err(crate::CommandError::new(
            crate::ErrorCode::InvalidRequest,
            "Restore source and target must be different.",
        ));
    }
    let preview = read_and_verify(source)?;
    reject_links(target).map_err(|_| {
        crate::CommandError::new(
            crate::ErrorCode::InvalidRequest,
            "Restore target contains a link.",
        )
    })?;
    if target.exists()
        && fs::read_dir(target)
            .map_err(|_| crate::storage::unavailable())?
            .next()
            .is_some()
    {
        return Err(crate::CommandError::new(
            crate::ErrorCode::InvalidRequest,
            "Restore target must be new and empty.",
        ));
    }
    fs::create_dir_all(target).map_err(|_| crate::storage::unavailable())?;
    let mut restored_manifest = preview.manifest.clone();
    let mut id = [0_u8; 16];
    getrandom::fill(&mut id).map_err(|_| crate::storage::unavailable())?;
    restored_manifest.workspace_id = id.iter().map(|byte| format!("{byte:02x}")).collect();
    let manifest_bytes =
        serde_json::to_vec_pretty(&restored_manifest).map_err(|_| crate::storage::unavailable())?;
    fs::write(target.join("manifest.json"), manifest_bytes)
        .map_err(|_| crate::storage::unavailable())?;
    for member in &preview.manifest.members {
        let relative = Path::new(&member.path);
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|_| crate::storage::unavailable())?;
        }
        fs::copy(source.join(relative), destination).map_err(|_| crate::storage::unavailable())?;
    }
    let verified = read_and_verify(target)?;
    if verified.manifest.workspace_id == preview.manifest.workspace_id {
        return Err(crate::CommandError::new(
            crate::ErrorCode::InvalidRequest,
            "Restore reused the source workspace identity.",
        ));
    }
    Ok(verified)
}
