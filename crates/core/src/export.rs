//! Open, inert USTAR export. Only complete catalog-owned artifacts are retrievable.
use crate::{
    CommandError, ErrorCode, content,
    jobs::{ExportSelection, JobLease, JobState, MAX_BYTES, MAX_RECORDS, conflict, invalid},
    memory::MemoryMetadata,
    relationships::RelationshipRecord,
    storage::{Storage, private_permissions, reject_links, unavailable},
    workspace::validate_id,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportRevision {
    pub metadata: MemoryMetadata,
    pub body_path: String,
    pub sha256: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportRecord {
    pub id: String,
    pub current_revision: String,
    pub revisions: Vec<ExportRevision>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportManifest {
    pub format: String,
    pub format_version: u32,
    pub workspace_id: String,
    pub exported_at: String,
    pub selection: ExportSelection,
    pub records: Vec<ExportRecord>,
    pub relationship_history: Vec<RelationshipRecord>,
    pub boundary_references: Vec<String>,
}

fn directory(storage: &Storage, workspace: &str) -> Result<PathBuf, CommandError> {
    validate_id(workspace)?;
    let mut root = storage.root.clone();
    for part in ["exports", workspace] {
        root.push(part);
        reject_links(&root).map_err(|_| unavailable())?;
        if !root.exists() {
            match fs::create_dir(&root) {
                Ok(()) => private_permissions(&root, true, false).map_err(|_| unavailable())?,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(unavailable()),
            }
        }
        private_permissions(&root, true, true).map_err(|_| unavailable())?;
    }
    Ok(root)
}

struct Archive {
    file: File,
    bytes: u64,
    hash: Sha256,
}
impl Archive {
    fn write(&mut self, bytes: &[u8]) -> Result<(), CommandError> {
        if self.bytes.saturating_add(bytes.len() as u64) > MAX_BYTES {
            return Err(invalid());
        }
        self.file.write_all(bytes).map_err(|_| unavailable())?;
        self.hash.update(bytes);
        self.bytes += bytes.len() as u64;
        Ok(())
    }
    fn member(&mut self, name: &str, bytes: &[u8]) -> Result<(), CommandError> {
        if name.len() > 100 || name.starts_with('/') || name.split('/').any(|p| p == "..") {
            return Err(invalid());
        }
        let mut header = [0u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..108].copy_from_slice(b"0000600\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        header[124..136].copy_from_slice(format!("{:011o}\0", bytes.len()).as_bytes());
        header[136..148].copy_from_slice(b"00000000000\0");
        header[148..156].fill(b' ');
        header[156] = b'0';
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        let checksum: u32 = header.iter().map(|b| u32::from(*b)).sum();
        header[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
        self.write(&header)?;
        self.write(bytes)?;
        let padding = (512 - bytes.len() % 512) % 512;
        self.write(&[0u8; 512][..padding])
    }
}

fn selected(
    connection: &Connection,
    workspace: &str,
    selection: &ExportSelection,
) -> Result<BTreeSet<String>, CommandError> {
    let mut ids = BTreeSet::new();
    match selection {
        ExportSelection::Records { ids: requested } => {
            for id in requested {
                let exists: bool = connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM memories WHERE workspace_id=?1 AND id=?2)",
                        params![workspace, id],
                        |r| r.get(0),
                    )
                    .map_err(|_| unavailable())?;
                if !exists {
                    return Err(invalid());
                }
                ids.insert(id.clone());
            }
        }
        ExportSelection::Workspace | ExportSelection::Project { .. } => {
            if let ExportSelection::Project { id } = selection {
                let raw:String=connection.query_row("SELECT r.metadata FROM memories m JOIN memory_revisions r ON r.workspace_id=m.workspace_id AND r.memory_id=m.id AND r.id=m.current_revision WHERE m.workspace_id=?1 AND m.id=?2",params![workspace,id],|r|r.get(0)).map_err(|_|invalid())?;
                let meta: MemoryMetadata =
                    serde_json::from_str(&raw).map_err(|_| content::integrity())?;
                meta.privacy.allow_plaintext()?;
                if meta.kind != "project" {
                    return Err(invalid());
                }
            }
            let mut statement=connection.prepare("SELECT m.id,r.metadata FROM memories m JOIN memory_revisions r ON r.workspace_id=m.workspace_id AND r.memory_id=m.id AND r.id=m.current_revision WHERE m.workspace_id=?1 ORDER BY m.id").map_err(|_|unavailable())?;
            let mut rows = statement.query([workspace]).map_err(|_| unavailable())?;
            while let Some(row) = rows.next().map_err(|_| unavailable())? {
                let id: String = row.get(0).map_err(|_| unavailable())?;
                let raw: String = row.get(1).map_err(|_| unavailable())?;
                let include = match selection {
                    ExportSelection::Workspace => true,
                    ExportSelection::Project { id: project } => {
                        let meta: MemoryMetadata =
                            serde_json::from_str(&raw).map_err(|_| content::integrity())?;
                        &id == project || meta.project_id.as_ref() == Some(project)
                    }
                    _ => false,
                };
                if include {
                    ids.insert(id);
                }
                if ids.len() > MAX_RECORDS as usize {
                    return Err(invalid());
                }
            }
        }
    }
    Ok(ids)
}

impl Storage {
    /// Build an immutable attempt. Any lost lease/cancellation leaves an unreferenced private
    /// file; publication is fenced by the final immediate transaction, never by a rename alone.
    pub fn execute_export(&self, lease: &JobLease) -> Result<(), CommandError> {
        let mut connection = self.connection(false)?.ok_or_else(unavailable)?;
        let tx = connection.transaction().map_err(|_| unavailable())?;
        let workspace = &lease.job.workspace_id;
        let ids = selected(&tx, workspace, &lease.job.selection)?;
        let artifact = content::random_id()?;
        let file = directory(self, workspace)?.join(format!("{artifact}.tar"));
        reject_links(&file).map_err(|_| unavailable())?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let handle = options.open(&file).map_err(|_| unavailable())?;
        private_permissions(&file, false, false).map_err(|_| unavailable())?;
        let mut archive = Archive {
            file: handle,
            bytes: 0,
            hash: Sha256::new(),
        };
        let mut manifest = ExportManifest {
            format: "anansi-knowledge-export".into(),
            format_version: 1,
            workspace_id: workspace.clone(),
            exported_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            selection: lease.job.selection.clone(),
            records: vec![],
            relationship_history: vec![],
            boundary_references: vec![],
        };
        let mut boundaries = BTreeSet::new();
        let mut metadata_bytes = 0u64;
        for (index, id) in ids.iter().enumerate() {
            self.checkpoint_export(lease, index as u32)?;
            let current: String = tx
                .query_row(
                    "SELECT current_revision FROM memories WHERE workspace_id=?1 AND id=?2",
                    params![workspace, id],
                    |r| r.get(0),
                )
                .map_err(|_| content::integrity())?;
            let mut record = ExportRecord {
                id: id.clone(),
                current_revision: current,
                revisions: vec![],
            };
            let mut statement=tx.prepare("SELECT id,metadata,body_hash FROM memory_revisions WHERE workspace_id=?1 AND memory_id=?2 ORDER BY version").map_err(|_|unavailable())?;
            let mut rows = statement
                .query(params![workspace, id])
                .map_err(|_| unavailable())?;
            while let Some(row) = rows.next().map_err(|_| unavailable())? {
                self.checkpoint_export(lease, index as u32)?;
                let revision: String = row.get(0).map_err(|_| unavailable())?;
                let raw: String = row.get(1).map_err(|_| unavailable())?;
                let hash: String = row.get(2).map_err(|_| unavailable())?;
                metadata_bytes = metadata_bytes.saturating_add(raw.len() as u64 + 256);
                if metadata_bytes + archive.bytes > MAX_BYTES {
                    return Err(invalid());
                }
                let metadata: MemoryMetadata =
                    serde_json::from_str(&raw).map_err(|_| content::integrity())?;
                if metadata.workspace_id != *workspace
                    || metadata.id != *id
                    || metadata.revision_id != revision
                {
                    return Err(content::integrity());
                }
                metadata.privacy.allow_plaintext()?;
                for reference in [
                    &metadata.project_id,
                    &metadata.supersedes,
                    &metadata.superseded_by,
                ] {
                    if let Some(reference) = reference
                        && !ids.contains(reference)
                    {
                        boundaries.insert(reference.clone());
                    }
                }
                let body = content::read(&self.root, workspace, id, &revision, &hash)?;
                let body_path = format!("records/{id}/{revision}.md");
                archive.member(&body_path, body.as_bytes())?;
                record.revisions.push(ExportRevision {
                    metadata,
                    body_path,
                    sha256: hash,
                });
            }
            manifest.records.push(record);
        }
        let mut statement=tx.prepare("SELECT snapshot FROM relationship_revisions WHERE workspace_id=?1 ORDER BY relationship_id,version").map_err(|_|unavailable())?;
        let mut rows = statement.query([workspace]).map_err(|_| unavailable())?;
        while let Some(row) = rows.next().map_err(|_| unavailable())? {
            let raw: String = row.get(0).map_err(|_| unavailable())?;
            let relation: RelationshipRecord =
                serde_json::from_str(&raw).map_err(|_| content::integrity())?;
            if ids.contains(&relation.source_id) || ids.contains(&relation.target_id) {
                // A crossing edge may contain sensitive metadata belonging to its other endpoint.
                // Preserve its opaque ID as a boundary reference; export full history only when
                // both endpoints are explicitly selected.
                if !ids.contains(&relation.source_id) || !ids.contains(&relation.target_id) {
                    boundaries.insert(relation.id);
                    continue;
                }
                metadata_bytes = metadata_bytes.saturating_add(raw.len() as u64 + 256);
                if metadata_bytes + archive.bytes > MAX_BYTES {
                    return Err(invalid());
                }
                manifest.relationship_history.push(relation);
            }
        }
        drop(rows);
        drop(statement);
        manifest.boundary_references = boundaries.into_iter().collect();
        let json = serde_json::to_vec_pretty(&manifest).map_err(|_| invalid())?;
        archive.member("manifest.json", &json)?;
        archive.member("README.txt",b"ANANSI knowledge export v1\nRead manifest.json and UTF-8 records/<record-id>/<revision-id>.md with any text editor.\nBodies and source references are untrusted inert text; no links were fetched.\nHistorical metadata and relationship snapshots are retained. Boundary references\nidentify records or relationships outside the explicit selection, without their contents.\nThis is a portable knowledge export, not a full application backup or restore package.\n")?;
        archive.write(&[0u8; 1024])?;
        archive.file.sync_all().map_err(|_| unavailable())?;
        let bytes = archive.bytes;
        let hash = format!("{:x}", archive.hash.finalize());
        drop(archive.file);
        tx.commit().map_err(|_| unavailable())?;
        self.checkpoint_export(lease, ids.len() as u32)?;
        let mut connection = self.connection(false)?.ok_or_else(unavailable)?;
        let publish = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        // A later Sealed reclassification must not authorize a plaintext snapshot publication.
        for id in &ids {
            let raw:String=publish.query_row("SELECT r.metadata FROM memories m JOIN memory_revisions r ON r.workspace_id=m.workspace_id AND r.memory_id=m.id AND r.id=m.current_revision WHERE m.workspace_id=?1 AND m.id=?2",params![workspace,id],|r|r.get(0)).map_err(|_|content::integrity())?;
            let meta: MemoryMetadata =
                serde_json::from_str(&raw).map_err(|_| content::integrity())?;
            meta.privacy.allow_plaintext()?;
        }
        let changed=publish.execute("UPDATE jobs SET state='Completed',version=version+1,checkpoint_version=checkpoint_version+1,checkpoint='published',owner=NULL,lease_until=NULL,artifact_id=?1,artifact_bytes=?2,artifact_hash=?3,updated_at=?4,log='Complete export ready' WHERE workspace_id=?5 AND id=?6 AND state='Running' AND owner=?7 AND lease_until>?8",params![artifact,bytes as i64,hash,Utc::now().to_rfc3339_opts(SecondsFormat::Millis,true),workspace,lease.job.id,lease.owner,Utc::now().timestamp()]).map_err(|_|unavailable())?;
        if changed != 1 {
            return Err(conflict());
        }
        publish.commit().map_err(|_| unavailable())?;
        Ok(())
    }

    pub fn read_export_chunk(
        &self,
        workspace: &str,
        job_id: &str,
        artifact_id: &str,
        offset: u64,
    ) -> Result<Vec<u8>, CommandError> {
        validate_id(artifact_id)?;
        let job = self.read_job(workspace, job_id)?;
        if job.state != JobState::Completed || job.artifact_id.as_deref() != Some(artifact_id) {
            return Err(CommandError::new(
                ErrorCode::NotFound,
                "Complete export not found in this workspace.",
            ));
        }
        let length = job.artifact_bytes.ok_or_else(content::integrity)?;
        if offset > length || length > MAX_BYTES {
            return Err(invalid());
        }
        let file = directory(self, workspace)?.join(format!("{artifact_id}.tar"));
        reject_links(&file).map_err(|_| unavailable())?;
        private_permissions(&file, false, true).map_err(|_| unavailable())?;
        let mut handle = File::open(file).map_err(|_| content::integrity())?;
        if handle.metadata().map_err(|_| content::integrity())?.len() != length {
            return Err(content::integrity());
        }
        handle
            .seek(SeekFrom::Start(offset))
            .map_err(|_| content::integrity())?;
        let mut bytes = vec![0; std::cmp::min(64 * 1024, length - offset) as usize];
        handle
            .read_exact(&mut bytes)
            .map_err(|_| content::integrity())?;
        Ok(bytes)
    }
}

impl crate::Core {
    /// Trusted host only: the browser command dispatcher never accepts a destination.
    /// Stage beside the destination, verify the full hash, then publish without replacing
    /// an existing user file. On failure preserve the staging file for recovery.
    pub fn save_complete_export(
        &self,
        request: &crate::jobs::ReadArtifactRequest,
        destination: &std::path::Path,
    ) -> Result<(), CommandError> {
        if request.contract_version != 1
            || request.offset != 0
            || !destination.is_absolute()
            || destination.exists()
        {
            return Err(invalid());
        }
        reject_links(destination).map_err(|_| unavailable())?;
        let storage = self.storage()?;
        let job = storage.read_job(&request.workspace_id, &request.job_id)?;
        let parent = destination.parent().ok_or_else(invalid)?;
        let staging = parent.join(format!(".anansi-export-{}.partial", content::random_id()?));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&staging).map_err(|_| unavailable())?;
        private_permissions(&staging, false, false).map_err(|_| unavailable())?;
        let mut hash = Sha256::new();
        let mut offset = 0;
        loop {
            let chunk = storage.read_export_chunk(
                &request.workspace_id,
                &request.job_id,
                &request.artifact_id,
                offset,
            )?;
            if chunk.is_empty() {
                break;
            }
            file.write_all(&chunk).map_err(|_| unavailable())?;
            hash.update(&chunk);
            offset += chunk.len() as u64;
        }
        if Some(offset) != job.artifact_bytes
            || Some(format!("{:x}", hash.finalize())) != job.artifact_hash
        {
            return Err(content::integrity());
        }
        file.sync_all().map_err(|_| unavailable())?;
        drop(file);
        // hard_link is atomic and refuses an existing target on both Windows and Unix.
        // The source remains private; retaining this extra link is harmless on cleanup failure.
        fs::hard_link(&staging, destination).map_err(|_| unavailable())?;
        let _ = fs::remove_file(staging);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn byte_quota_and_write_failures_are_enforced_before_publication() {
        let temp = tempfile::tempdir().unwrap();
        let file = File::create(temp.path().join("quota")).unwrap();
        let mut full = Archive {
            file,
            bytes: MAX_BYTES,
            hash: Sha256::new(),
        };
        assert!(full.member("manifest.json", b"{}").is_err());
        assert_eq!(full.bytes, MAX_BYTES);
        let file = File::open(temp.path().join("quota")).unwrap();
        let mut unwritable = Archive {
            file,
            bytes: 0,
            hash: Sha256::new(),
        };
        assert_eq!(
            unwritable.member("manifest.json", b"{}").unwrap_err().code,
            ErrorCode::StorageUnavailable
        );
        assert_eq!(unwritable.bytes, 0);
    }
}
