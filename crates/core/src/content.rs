use crate::{
    CommandError, ErrorCode,
    storage::{PermissionCheckKind, private_permissions_kind, reject_links, unavailable},
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
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
                Ok(()) => private_permissions_kind(
                    &directory,
                    true,
                    false,
                    PermissionCheckKind::ContentWriteDir,
                )
                .map_err(|_| unavailable())?,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(unavailable()),
            }
        }
        private_permissions_kind(&directory, true, true, PermissionCheckKind::ContentReadDir)
            .map_err(|_| unavailable())?;
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
    private_permissions_kind(&file, false, false, PermissionCheckKind::ContentWriteFile)
        .map_err(|_| unavailable())?;
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
    private_permissions_kind(&file, false, true, PermissionCheckKind::ContentReadFile)
        .map_err(|_| unavailable())?;
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

/// Operation-scoped batch permission proofs for content reads, following
/// the `vault/io.rs` `VerifiedFiles` pattern. One Windows PowerShell
/// process verifies the whole batch (JSON path list on stdin, the same
/// `protect.ps1` owner/DACL invariant per item, verify-only so nothing
/// is mutated); other platforms check each deduplicated path with
/// `private_permissions`. Shared directories are verified once.
///
/// The proof set is created fresh per operation and dropped at its end:
/// it is never stored, never shared across operations, and never a
/// permanent positive cache, so a permission revocation takes effect on
/// the next operation. Every other check is retained: link rejection
/// runs per path before and per read, and each read still verifies
/// existence, the body hash against the database value, `MAX_BODY`,
/// and UTF-8. A read for a path outside the set (for example a
/// revision created mid-operation, or a path that was absent when the
/// batch was built) falls back to the full per-path verification in
/// `read`, never to an unverified read. A failed batch fails the whole
/// operation. On non-Windows platforms a proven read additionally
/// re-validates the file's permission bits at read time (one metadata
/// check), closing the widen-after-proof race there; on Windows that
/// race has a one-operation residual window (see `read_verified`).
pub(crate) struct VerifiedPaths {
    files: HashSet<PathBuf>,
}

impl VerifiedPaths {
    pub(crate) fn verify(
        root: &Path,
        workspace: &str,
        records: &[(String, String)],
    ) -> Result<Self, CommandError> {
        crate::workspace::validate_id(workspace)?;
        let mut ordered: Vec<(PathBuf, bool)> = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();
        let mut push = |path: PathBuf, directory: bool| {
            if seen.insert(path.clone()) {
                ordered.push((path, directory));
            }
        };
        let content_dir = root.join("content");
        push(content_dir.clone(), true);
        let workspace_dir = content_dir.join(workspace);
        push(workspace_dir, true);
        for (record, revision) in records {
            crate::workspace::validate_id(record)?;
            crate::workspace::validate_id(revision)?;
            let record_dir = root.join("content").join(workspace).join(record);
            push(record_dir.clone(), true);
            push(record_dir.join(format!("{revision}.md")), false);
        }
        if ordered.len() > 200_000 {
            return Err(unavailable());
        }
        // Absent paths stay out of the batch: a workspace with no notes
        // yet has no content directories, and a record whose directory
        // or file is missing must keep the old read behavior (link
        // rejection, then unavailable for a missing directory or
        // integrity for a missing file) through the fallback in
        // `read_verified`, not fail the whole batch up front.
        ordered.retain(|(path, _)| fs::symlink_metadata(path).is_ok());
        for (path, _) in &ordered {
            reject_links(path).map_err(|_| unavailable())?;
        }
        if ordered.is_empty() {
            return Ok(Self {
                files: HashSet::new(),
            });
        }
        #[cfg(windows)]
        {
            use std::{
                io::Write,
                os::windows::process::CommandExt,
                process::{Command, Stdio},
            };
            const SCRIPT: &str = concat!(
                "$ErrorActionPreference='Stop'; [Console]::InputEncoding=[Text.UTF8Encoding]::new($false); ",
                "$items=ConvertFrom-Json ([Console]::In.ReadToEnd()); foreach($item in $items) { ",
                "$env:ANANSI_PERMISSION_PATH=$item.path; $env:ANANSI_PERMISSION_DIRECTORY=if($item.directory){'1'}else{'0'}; ",
                "$env:ANANSI_PERMISSION_VERIFY='1'; & {\n",
                include_str!("protect.ps1"),
                "\n}; }"
            );
            let request: Vec<_> = ordered
                .iter()
                .map(|(path, directory)| serde_json::json!({"path": path, "directory": directory}))
                .collect();
            let input = serde_json::to_vec(&request).map_err(|_| unavailable())?;
            let mut child = Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
                .creation_flags(0x08000000)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|_| unavailable())?;
            let written = child
                .stdin
                .take()
                .ok_or_else(unavailable)?
                .write_all(&input);
            let result = child.wait_with_output().map_err(|_| unavailable())?;
            if written.is_err() || !result.status.success() {
                return Err(unavailable());
            }
        }
        #[cfg(not(windows))]
        for (path, directory) in &ordered {
            private_permissions_kind(path, *directory, true, PermissionCheckKind::BatchPath)
                .map_err(|_| unavailable())?;
        }
        let count = ordered.len() as u64;
        let files = ordered
            .into_iter()
            .filter(|(_, directory)| !*directory)
            .map(|(path, _)| path)
            .collect();
        crate::storage::record_permission_batch(count);
        Ok(Self { files })
    }

    pub(crate) fn read_verified(
        &self,
        root: &Path,
        workspace: &str,
        record: &str,
        revision: &str,
        expected: &str,
    ) -> Result<String, CommandError> {
        let file = verified_location(root, workspace, record, revision)?;
        if !self.files.contains(&file) {
            crate::storage::record_permission_fallback(fs::symlink_metadata(&file).is_ok());
            return read(root, workspace, record, revision, expected);
        }
        // Read-time revalidation where it costs a single metadata
        // check: a file widened after proving must fail here, so the
        // batch proof does not authorize a read of a file that is no
        // longer private. Windows has no cheap equivalent (ACL checks
        // need a process spawn, which batching exists to avoid), so
        // the Windows residual window is one operation: proofs are
        // fresh per operation and the next operation re-verifies.
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&file)
                .map_err(|_| integrity())?
                .permissions()
                .mode()
                & 0o777;
            crate::storage::record_permission_recheck();
            if mode != 0o600 {
                return Err(unavailable());
            }
        }
        if !file.exists() {
            return Err(integrity());
        }
        let mut bytes = Vec::new();
        fs::File::open(&file)
            .map_err(|_| integrity())?
            .take((MAX_BODY + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| integrity())?;
        if bytes.len() > MAX_BODY || hash(&bytes) != expected {
            return Err(integrity());
        }
        String::from_utf8(bytes).map_err(|_| integrity())
    }
}

/// Read-path location with link rejection but no permission calls: the
/// caller proves the paths with `VerifiedPaths::verify` first. Mirrors
/// `location(..., create = false)` minus its permission checks.
fn verified_location(
    root: &Path,
    workspace: &str,
    record: &str,
    revision: &str,
) -> Result<PathBuf, CommandError> {
    for id in [workspace, record, revision] {
        crate::workspace::validate_id(id)?;
    }
    let mut directory = root.to_path_buf();
    for part in ["content", workspace, record] {
        directory.push(part);
        reject_links(&directory).map_err(|_| unavailable())?;
    }
    let file = directory.join(format!("{revision}.md"));
    reject_links(&file).map_err(|_| unavailable())?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WS: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const REC_A: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const REC_B: &str = "cccccccccccccccccccccccccccccccc";
    const REV: &str = "dddddddddddddddddddddddddddddddd";

    fn widen(path: &Path) {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let status = std::process::Command::new("icacls.exe")
                .arg(path)
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
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
    }

    #[test]
    fn verified_paths_batch_fallback_and_revocation() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let body_a = "The Tern archive shelf has reference code BATCH-991.";
        let body_b = "Unrelated collection entry contains printed maps.";
        let hash_a = write(root, WS, REC_A, REV, body_a).unwrap();
        let hash_b = write(root, WS, REC_B, REV, body_b).unwrap();
        let records = vec![
            (REC_A.to_string(), REV.to_string()),
            (REC_B.to_string(), REV.to_string()),
        ];
        let proofs = VerifiedPaths::verify(root, WS, &records).unwrap();
        assert_eq!(
            proofs.read_verified(root, WS, REC_A, REV, &hash_a).unwrap(),
            body_a
        );
        // Fallback: a record outside the proof set still reads through
        // the full per-path verification, never unverified.
        let narrow =
            VerifiedPaths::verify(root, WS, &[(REC_A.to_string(), REV.to_string())]).unwrap();
        assert_eq!(
            narrow.read_verified(root, WS, REC_B, REV, &hash_b).unwrap(),
            body_b
        );
        // The body hash is still enforced on a verified path.
        let file_a = root
            .join("content")
            .join(WS)
            .join(REC_A)
            .join(format!("{REV}.md"));
        std::fs::write(&file_a, "tampered").unwrap();
        assert!(proofs.read_verified(root, WS, REC_A, REV, &hash_a).is_err());
        std::fs::write(&file_a, body_a).unwrap();
        // Widening after proving: the read-time recheck (non-Windows)
        // fails the read, so a widened file is not read on a stale
        // proof. On Windows the batch-only proof keeps its
        // one-operation residual window, and a fresh batch fails in
        // both cases. Repair restores both the proven read and fresh
        // verification.
        let file_b = root
            .join("content")
            .join(WS)
            .join(REC_B)
            .join(format!("{REV}.md"));
        widen(&file_b);
        #[cfg(not(windows))]
        assert!(proofs.read_verified(root, WS, REC_B, REV, &hash_b).is_err());
        #[cfg(windows)]
        assert!(proofs.read_verified(root, WS, REC_B, REV, &hash_b).is_ok());
        assert!(VerifiedPaths::verify(root, WS, &[(REC_B.to_string(), REV.to_string())]).is_err());
        crate::storage::private_permissions(&file_b, false, false).unwrap();
        assert!(proofs.read_verified(root, WS, REC_B, REV, &hash_b).is_ok());
        assert!(VerifiedPaths::verify(root, WS, &[(REC_B.to_string(), REV.to_string())]).is_ok());
    }
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;

    const WSA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const WSB: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const REC1: &str = "cccccccccccccccccccccccccccccccc";
    const REC2: &str = "dddddddddddddddddddddddddddddddd";
    const REC3: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const REC4: &str = "ffffffffffffffffffffffffffffffff";
    const REC5: &str = "11111111111111111111111111111111";
    const REV1: &str = "22222222222222222222222222222222";
    const REV2: &str = "33333333333333333333333333333333";

    fn widen(path: &Path) {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let status = std::process::Command::new("icacls.exe")
                .arg(path)
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
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
    }

    fn file_path(root: &Path, ws: &str, rec: &str, rev: &str) -> PathBuf {
        root.join("content")
            .join(ws)
            .join(rec)
            .join(format!("{rev}.md"))
    }

    #[test]
    fn outside_set_and_other_root_fall_back_to_full_verification() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let root2 = temp.path().join("second-root");
        std::fs::create_dir(&root2).unwrap();
        let hash_b = write(
            root,
            WSB,
            REC1,
            REV1,
            "The other workspace holds 44 crates.",
        )
        .unwrap();
        let hash_a = write(
            root,
            WSA,
            REC1,
            REV1,
            "The first workspace holds 12 crates.",
        )
        .unwrap();
        let hash_a2 = write(&root2, WSA, REC1, REV1, "The second root holds 12 crates.").unwrap();
        let proofs =
            VerifiedPaths::verify(root, WSA, &[(REC1.to_string(), REV1.to_string())]).unwrap();
        // Another workspace: fallback reads a properly permissioned
        // file, and refuses an overbroad one. Proofs never authorize it.
        assert_eq!(
            proofs
                .read_verified(root, WSB, REC1, REV1, &hash_b)
                .unwrap(),
            "The other workspace holds 44 crates."
        );
        widen(&file_path(root, WSB, REC1, REV1));
        assert!(
            proofs
                .read_verified(root, WSB, REC1, REV1, &hash_b)
                .is_err()
        );
        // Another storage root: same fallback/fail rules.
        assert_eq!(
            proofs
                .read_verified(&root2, WSA, REC1, REV1, &hash_a2)
                .unwrap(),
            "The second root holds 12 crates."
        );
        widen(&file_path(&root2, WSA, REC1, REV1));
        assert!(
            proofs
                .read_verified(&root2, WSA, REC1, REV1, &hash_a2)
                .is_err()
        );
        // The proven path itself still reads.
        assert_eq!(
            proofs
                .read_verified(root, WSA, REC1, REV1, &hash_a)
                .unwrap(),
            "The first workspace holds 12 crates."
        );
    }

    #[test]
    fn replaced_bodies_fail_integrity() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let hash = write(root, WSA, REC2, REV1, "Original body with 31 stones.").unwrap();
        let proofs =
            VerifiedPaths::verify(root, WSA, &[(REC2.to_string(), REV1.to_string())]).unwrap();
        // Hash-altered body.
        std::fs::write(
            file_path(root, WSA, REC2, REV1),
            "Altered body with 31 stones.",
        )
        .unwrap();
        assert!(proofs.read_verified(root, WSA, REC2, REV1, &hash).is_err());
        // Missing body.
        std::fs::write(
            file_path(root, WSA, REC2, REV1),
            "Original body with 31 stones.",
        )
        .unwrap();
        std::fs::remove_file(file_path(root, WSA, REC2, REV1)).unwrap();
        assert!(proofs.read_verified(root, WSA, REC2, REV1, &hash).is_err());
        // Oversized body.
        let big = "x".repeat(MAX_BODY + 1);
        std::fs::write(file_path(root, WSA, REC2, REV1), &big).unwrap();
        assert!(proofs.read_verified(root, WSA, REC2, REV1, &hash).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn replaced_paths_fail_link_rejection() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let target = root.join("target.md");
        std::fs::write(&target, "Target body.").unwrap();
        let hash_f = write(root, WSA, REC3, REV1, "File link body.").unwrap();
        let hash_d = write(root, WSA, REC4, REV1, "Directory link body.").unwrap();
        let proofs = VerifiedPaths::verify(
            root,
            WSA,
            &[
                (REC3.to_string(), REV1.to_string()),
                (REC4.to_string(), REV1.to_string()),
            ],
        )
        .unwrap();
        // File replaced by a symlink.
        std::fs::remove_file(file_path(root, WSA, REC3, REV1)).unwrap();
        std::os::unix::fs::symlink(&target, file_path(root, WSA, REC3, REV1)).unwrap();
        assert!(
            proofs
                .read_verified(root, WSA, REC3, REV1, &hash_f)
                .is_err()
        );
        // Directory replaced by a symlink.
        let dir = root.join("content").join(WSA).join(REC4);
        std::fs::remove_dir_all(&dir).unwrap();
        let other_dir = root.join("other-dir");
        std::fs::create_dir(&other_dir).unwrap();
        std::os::unix::fs::symlink(&other_dir, &dir).unwrap();
        assert!(
            proofs
                .read_verified(root, WSA, REC4, REV1, &hash_d)
                .is_err()
        );
    }

    #[test]
    fn revised_record_reads_new_revision_via_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let hash1 = write(root, WSA, REC5, REV1, "First revision counts 10.").unwrap();
        let proofs =
            VerifiedPaths::verify(root, WSA, &[(REC5.to_string(), REV1.to_string())]).unwrap();
        assert_eq!(
            proofs.read_verified(root, WSA, REC5, REV1, &hash1).unwrap(),
            "First revision counts 10."
        );
        let hash2 = write(root, WSA, REC5, REV2, "Second revision counts 20.").unwrap();
        // New revision path was never proven: full verification, new hash.
        assert_eq!(
            proofs.read_verified(root, WSA, REC5, REV2, &hash2).unwrap(),
            "Second revision counts 20."
        );
    }

    #[test]
    fn failed_batch_leaves_no_trace() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, WSA, REC1, REV1, "Trace body.").unwrap();
        let file = file_path(root, WSA, REC1, REV1);
        widen(&file);
        let before: Vec<_> = std::fs::read_dir(root.join("content").join(WSA).join(REC1))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(VerifiedPaths::verify(root, WSA, &[(REC1.to_string(), REV1.to_string())]).is_err());
        let after: Vec<_> = std::fs::read_dir(root.join("content").join(WSA).join(REC1))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(before, after, "failed verify-only batch creates nothing");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o644,
                "failed verify-only batch repairs nothing"
            );
        }
    }

    #[test]
    #[cfg(unix)]
    fn widened_file_after_proof_fails_read() {
        // Race case: permissions widened after proving must fail the
        // read once the read-time recheck is in place. This test
        // fails against batch-only semantics and passes after the
        // repair.
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let hash = write(root, WSA, REC1, REV1, "Race body holds 5.").unwrap();
        let proofs =
            VerifiedPaths::verify(root, WSA, &[(REC1.to_string(), REV1.to_string())]).unwrap();
        assert_eq!(
            proofs.read_verified(root, WSA, REC1, REV1, &hash).unwrap(),
            "Race body holds 5."
        );
        widen(&file_path(root, WSA, REC1, REV1));
        assert!(proofs.read_verified(root, WSA, REC1, REV1, &hash).is_err());
    }
}
