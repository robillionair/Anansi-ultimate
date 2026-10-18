use anansi_core::{
    backup::{self, BACKUP_FORMAT, BACKUP_VERSION, BackupManifest},
    restore,
};
use chrono::Utc;
use std::fs;
use tempfile::tempdir;

#[test]
fn verifies_members_and_restores_only_into_a_new_target() {
    let root = tempdir().unwrap();
    let source = root.path().join("source");
    let input = root.path().join("input.txt");
    let target = root.path().join("target");
    fs::create_dir_all(&source).unwrap();
    fs::write(&input, b"portable memory").unwrap();
    let member = backup::copy_member(&input, &source, "records.txt").unwrap();
    let manifest = BackupManifest {
        format: BACKUP_FORMAT.to_owned(),
        format_version: BACKUP_VERSION,
        workspace_id: "0123456789abcdef0123456789abcdef".to_owned(),
        workspace_name: "Synthetic workspace".to_owned(),
        created_at: Utc::now().to_rfc3339(),
        members: vec![member],
    };
    backup::write_manifest(&source, &manifest).unwrap();
    let preview = restore::preview(&source).unwrap();
    assert!(preview.verified);
    let restored = restore::restore_to_new_target(&source, &target).unwrap();
    assert_eq!(restored.total_bytes, preview.total_bytes);
    assert_eq!(
        fs::read(target.join("records.txt")).unwrap(),
        b"portable memory"
    );
    assert!(restore::restore_to_new_target(&source, &target).is_err());
}

#[test]
fn refuses_tampered_members_and_duplicate_paths() {
    let root = tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("body.md"), b"original").unwrap();
    let member = backup::copy_member(&source.join("body.md"), &source, "body-copy.md").unwrap();
    fs::write(source.join("body-copy.md"), b"tampered").unwrap();
    let manifest = BackupManifest {
        format: BACKUP_FORMAT.to_owned(),
        format_version: BACKUP_VERSION,
        workspace_id: "0123456789abcdef0123456789abcdef".to_owned(),
        workspace_name: "Synthetic workspace".to_owned(),
        created_at: Utc::now().to_rfc3339(),
        members: vec![member.clone(), member],
    };
    backup::write_manifest(&source, &manifest).unwrap();
    assert!(restore::preview(&source).is_err());
}
