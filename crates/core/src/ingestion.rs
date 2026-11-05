//! Local document ingestion (M13 Slice 1): TXT/Markdown only.
//!
//! The upload mechanics mirror the bounded backup upload
//! (`backup_upload.rs`): the client declares bytes + SHA-256, sends
//! sequential ≤64 KiB parts with offset-must-equal-received, and the
//! server re-reads and hash-verifies before the job runs. The durable
//! identity is the job row itself (selection `Ingestion`, checkpoint
//! `uploading:<received>` / `staged` / `published` / `discarded`), so
//! no second staging table exists and no schema migration was needed.
//!
//! States: the job is created **Paused** while the upload is in flight
//! (the worker only claims Queued jobs), moves to Queued when the
//! upload finishes, and is published by the background worker through
//! `memory_write` with an atomic lease+dedup guard. Cancellation
//! removes staging; interruption leaves the job Paused with staging
//! intact for an explicit Resume.
use crate::{
    CommandError, Core, content,
    document::{
        DocumentProvenance, DocumentSource, MAX_ACTIVE_UPLOADS, MAX_DOCUMENT_BYTES,
        MAX_DOCUMENT_CHARS, MAX_UPLOAD_PART_BYTES,
    },
    jobs::{ExportSelection, Job, conflict, invalid},
    memory::{MemoryDraft, MemoryWriteRequest},
    privacy::Privacy,
    storage::{Storage, unavailable},
    workspace::validate_id,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngestionBegin {
    pub contract_version: u8,
    pub workspace_id: String,
    pub file_name: String,
    pub bytes: u64,
    pub sha256: String,
    pub privacy: Privacy,
    pub project_id: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngestionPart {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: String,
    pub offset: u32,
    pub bytes: Vec<u8>,
    pub finish: bool,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngestionStatus {
    pub contract_version: u8,
    pub id: String,
    pub received: u32,
    pub job: Option<Job>,
}

fn bad(message: &str) -> CommandError {
    CommandError::new(crate::ErrorCode::InvalidRequest, message)
}

fn staged_path(
    storage: &Storage,
    workspace: &str,
    upload_id: &str,
) -> Result<std::path::PathBuf, CommandError> {
    Ok(crate::export::directory(storage, workspace)?.join(format!("{upload_id}.upload")))
}

/// Bytes received so far, from a checkpoint of the form `uploading:<n>`.
fn received_from(checkpoint: &str) -> Option<u32> {
    checkpoint
        .strip_prefix("uploading:")
        .and_then(|n| n.parse::<u32>().ok())
}

fn project_exists(
    connection: &rusqlite::Connection,
    workspace: &str,
    id: &str,
) -> Result<bool, CommandError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM memories m JOIN memory_revisions r ON r.workspace_id=m.workspace_id AND r.id=m.current_revision WHERE m.workspace_id=?1 AND m.id=?2 AND json_extract(r.metadata,'$.kind')='project')",
            params![workspace, id],
            |row| row.get(0),
        )
        .map_err(|_| unavailable())
}

/// Delete staged files that can no longer be resumed: their job is
/// Completed/Cancelled, or no job row exists at all. Bounded (the
/// export directory holds at most a handful of `.upload` files) and
/// job-owned only — owner files are never touched.
fn sweep_staging(storage: &Storage, workspace: &str) -> Result<(), CommandError> {
    let directory = crate::export::directory(storage, workspace)?;
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("upload") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let connection = match storage.connection(false)? {
            Some(connection) => connection,
            None => continue,
        };
        let state: Option<String> = connection
            .query_row(
                "SELECT state FROM jobs WHERE workspace_id=?1 AND id=?2",
                params![workspace, stem],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| unavailable())?;
        match state.as_deref() {
            None | Some("Completed") | Some("Cancelled") => {
                let _ = crate::storage::reject_links(&path).map(|_| std::fs::remove_file(&path));
            }
            _ => {}
        }
    }
    Ok(())
}

impl Storage {
    /// Create a Paused ingestion job and its staged file. The job is
    /// Paused (not Queued) while the upload is in flight, so the worker
    /// can never claim an unuploaded file.
    pub(crate) fn enqueue_ingestion(
        &self,
        r: &IngestionBegin,
    ) -> Result<(Job, String), CommandError> {
        validate_id(&r.workspace_id)?;
        let kind = DocumentSource::kind_for_label(&r.file_name).map_err(|e| bad(&e))?;
        if r.file_name.chars().count() == 0
            || r.file_name.chars().count() > crate::document::MAX_LABEL_CHARS
            || r.file_name.contains(['/', '\\', '\0'])
        {
            return Err(bad(
                "Document file name must be 1 to 255 characters and a file name, not a path.",
            ));
        }
        if r.bytes == 0 || r.bytes > MAX_DOCUMENT_BYTES {
            return Err(bad(
                "Document file must be 1 to 65536 bytes. Larger files are refused before upload.",
            ));
        }
        if r.sha256.len() != 64
            || !r
                .sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(bad("Document SHA-256 must be 64 lowercase hex characters."));
        }
        if r.privacy == Privacy::Sealed {
            return Err(bad(
                "Sealed documents cannot be imported yet: no encrypted vault path exists for imports. Choose Private, Standard or Public.",
            ));
        }
        if let Some(pid) = &r.project_id {
            validate_id(pid)?;
        }
        self.open(&r.workspace_id)?;
        sweep_staging(self, &r.workspace_id)?;
        let mut connection = self.connection(false)?.ok_or_else(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        if let Some(pid) = &r.project_id
            && !project_exists(&tx, &r.workspace_id, pid)?
        {
            return Err(bad(
                "The selected project does not exist in this workspace.",
            ));
        }
        let active: u32 = tx
            .query_row(
                "SELECT count(*) FROM jobs WHERE workspace_id=?1 AND state IN ('Queued','Running','Paused')",
                [&r.workspace_id],
                |row| row.get(0),
            )
            .map_err(|_| unavailable())?;
        if active >= 32 {
            return Err(bad(
                "Too many active jobs in this workspace (32). Wait for one to finish or cancel it.",
            ));
        }
        let uploading: u32 = tx
            .query_row(
                "SELECT count(*) FROM jobs WHERE workspace_id=?1 AND state IN ('Queued','Running','Paused') AND json_extract(selection,'$.kind')='ingestion'",
                [&r.workspace_id],
                |row| row.get(0),
            )
            .map_err(|_| unavailable())?;
        if uploading >= MAX_ACTIVE_UPLOADS {
            return Err(bad(
                "Too many active document imports in this workspace (8). Wait for one to finish or cancel it.",
            ));
        }
        let id = content::random_id()?;
        let selection = ExportSelection::Ingestion {
            upload_id: id.clone(),
            file_name: r.file_name.clone(),
            format: kind.to_string(),
            expected_bytes: r.bytes,
            expected_hash: r.sha256.clone(),
            privacy: r.privacy,
            project_id: r.project_id.clone(),
        };
        selection.validate()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        // The staged file is created before the job row, mirroring the
        // backup upload: a failed insert leaves an orphan the next
        // upload-begin sweep removes.
        let path = staged_path(self, &r.workspace_id, &id)?;
        crate::snapshots::new_file(&path)?;
        tx.execute(
            "INSERT INTO jobs(workspace_id,id,state,version,checkpoint_version,checkpoint,selection,created_at,updated_at,log) VALUES(?1,?2,'Paused',1,1,'uploading:0',?3,?4,?4,'Upload in progress')",
            params![r.workspace_id, id, serde_json::to_string(&selection).map_err(|_| invalid())?, stamp],
        )
        .map_err(|_| unavailable())?;
        let job = crate::jobs::read(&tx, &r.workspace_id, &id)?;
        tx.commit().map_err(|_| unavailable())?;
        Ok((job, id))
    }

    /// Mark an ingestion job Failed with a specific log. The checkpoint
    /// is deliberately left untouched: a Failed job keeps `staged` (and
    /// its bytes) for Retry, or `discarded` when staging was removed.
    /// Returns Ok only when the row was actually flipped; a Cancelled
    /// job is already terminal and stays as it is.
    fn fail_ingestion(
        &self,
        lease: &crate::jobs::JobLease,
        log: &str,
    ) -> Result<bool, CommandError> {
        let connection = self.connection(false)?.ok_or_else(unavailable)?;
        let changed = connection
            .execute(
                "UPDATE jobs SET state='Failed',version=version+1,checkpoint_version=checkpoint_version+1,owner=NULL,lease_until=NULL,updated_at=?1,log=?2 WHERE workspace_id=?3 AND id=?4 AND state='Running' AND owner=?5 AND lease_until>?6",
                params![chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), log, lease.job.workspace_id, lease.job.id, lease.owner, chrono::Utc::now().timestamp()],
            )
            .map_err(|_| unavailable())?;
        Ok(changed == 1)
    }

    fn complete_ingestion(
        &self,
        lease: &crate::jobs::JobLease,
        memory_id: &str,
        raw_bytes: u64,
        raw_hash: &str,
        log: &str,
    ) -> Result<bool, CommandError> {
        let connection = self.connection(false)?.ok_or_else(unavailable)?;
        let changed = connection
            .execute(
                "UPDATE jobs SET state='Completed',version=version+1,checkpoint_version=checkpoint_version+1,checkpoint='published',artifact_id=?1,artifact_bytes=?2,artifact_hash=?3,owner=NULL,lease_until=NULL,updated_at=?4,log=?5 WHERE workspace_id=?6 AND id=?7 AND state='Running' AND owner=?8 AND lease_until>?9",
                params![memory_id, raw_bytes as i64, raw_hash, chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), log, lease.job.workspace_id, lease.job.id, lease.owner, chrono::Utc::now().timestamp()],
            )
            .map_err(|_| unavailable())?;
        Ok(changed == 1)
    }

    /// Publish a staged upload as a `document` memory. All handled
    /// outcomes return Ok: validation failures mark the job Failed with
    /// the specific refusal and keep staging for Retry; only a storage
    /// failure returns Err (the worker loop's generic fail path then
    /// applies best-effort, and the lease eventually expires to Paused).
    pub fn execute_ingestion(&self, lease: &crate::jobs::JobLease) -> Result<(), CommandError> {
        let selection = match &lease.job.selection {
            ExportSelection::Ingestion {
                file_name,
                format: kind,
                expected_bytes,
                expected_hash,
                privacy,
                project_id,
                ..
            } => (
                file_name.clone(),
                kind.clone(),
                *expected_bytes,
                expected_hash.clone(),
                *privacy,
                project_id.clone(),
            ),
            _ => return Err(unavailable()),
        };
        let (file_name, kind, expected_bytes, expected_hash, privacy, project_id) = selection;
        // The staged file is the ground truth: `claim_export` resets the
        // checkpoint to `snapshot` when claiming, so the checkpoint
        // cannot prove the upload finished. The length + hash check
        // below is the authoritative gate (a missing or short file
        // fails here with the staging message).
        let path = staged_path(self, &lease.job.workspace_id, &lease.job.id)?;
        crate::storage::reject_links(&path).map_err(|_| invalid())?;
        let bytes = match crate::snapshots::limited(&path, MAX_DOCUMENT_BYTES) {
            Ok(bytes) => bytes,
            Err(_) => {
                let _ =
                    self.fail_ingestion(lease, "Upload staging is missing; import the file again.");
                return Ok(());
            }
        };
        if bytes.len() as u64 != expected_bytes || content::hash(&bytes) != expected_hash {
            let _ = self.fail_ingestion(
                lease,
                "Upload bytes do not match the declared size or hash; import the file again.",
            );
            return Ok(());
        }
        let text = match String::from_utf8(bytes.clone()) {
            Ok(text) => text,
            Err(_) => {
                let _ = self.fail_ingestion(
                    lease,
                    "Document is not valid UTF-8. Only UTF-8 text files can be imported.",
                );
                return Ok(());
            }
        };
        let (bom, text) = match text.strip_prefix('\u{feff}') {
            Some(rest) => (true, rest.to_string()),
            None => (false, text),
        };
        if text.trim().is_empty() || text.chars().count() as u64 > MAX_DOCUMENT_CHARS {
            let _ = self.fail_ingestion(
                lease,
                "Document text must be 1 to 8000 characters and not empty or whitespace-only.",
            );
            return Ok(());
        }
        let text_hash = content::hash(text.as_bytes());
        let source = DocumentSource {
            label: file_name.clone(),
            kind: kind.clone(),
            bom,
            raw_bytes: expected_bytes,
            raw_hash: expected_hash.clone(),
            text_hash: text_hash.clone(),
            text_chars: text.chars().count() as u64,
        };
        if let Err(message) = source.validate(&text) {
            let _ = self.fail_ingestion(lease, &message);
            return Ok(());
        }
        // Best-effort wording for the completion log; the guard inside
        // memory_write is the authoritative dedup.
        let already = self
            .connection(false)?
            .and_then(|connection| {
                connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM memories m JOIN memory_revisions r ON r.workspace_id=m.workspace_id AND r.memory_id=m.id AND r.id=m.current_revision WHERE m.workspace_id=?1 AND json_extract(r.metadata,'$.kind')='document' AND json_extract(r.metadata,'$.title')=?2 AND r.body_hash=?3)",
                        params![lease.job.workspace_id, file_name, text_hash],
                        |row| row.get::<_, bool>(0),
                    )
                    .ok()
            })
            .unwrap_or(false);
        let request = MemoryWriteRequest {
            contract_version: 1,
            workspace_id: lease.job.workspace_id.clone(),
            id: None,
            expected_revision: None,
            draft: MemoryDraft {
                title: file_name.clone(),
                body: text,
                privacy,
                event_date: None,
                valid_from: None,
                valid_until: None,
                confidence: None,
                source_references: vec![],
                project_id: project_id.clone().map(Some),
                due_date: None,
                completed: None,
            },
            kind: Some("document".to_string()),
            lifecycle: None,
            restore_archive: false,
            restore_revision: None,
            confirm_privacy_change: false,
            provenance: Some("document_import".to_string()),
            document: Some(DocumentProvenance {
                file_name: file_name.clone(),
                media_type: DocumentProvenance::media_type_for(&kind).to_string(),
                raw_hash: expected_hash.clone(),
                text_hash: text_hash.clone(),
                raw_bytes: expected_bytes,
                text_chars: source.text_chars,
                bom,
            }),
            job_guard: Some(crate::document::JobPublishGuard {
                job_id: lease.job.id.clone(),
                owner: lease.owner.clone(),
                dedup_title: Some(file_name.clone()),
                dedup_text_hash: Some(text_hash),
            }),
        };
        let published = match self.memory_write(request) {
            Ok(details) => details,
            Err(error) => {
                // A guard refusal means the job was cancelled or its
                // lease expired: nothing was published and the job is
                // already terminal (or will be recovered to Paused), so
                // do not also mark it Failed.
                if error.code == crate::ErrorCode::RevisionConflict {
                    return Ok(());
                }
                let _ = self.fail_ingestion(lease, &error.message);
                return Ok(());
            }
        };
        let _ = std::fs::remove_file(&path);
        let log = if already {
            "Already imported"
        } else {
            "Document imported"
        };
        let _ = self.complete_ingestion(
            lease,
            &published.memory.id,
            expected_bytes,
            &expected_hash,
            log,
        );
        Ok(())
    }
}

impl Core {
    pub(crate) fn ingestion_operation(
        &self,
        command: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, CommandError> {
        let storage = self.storage()?;
        if command == "ingest-begin" {
            let r: IngestionBegin = serde_json::from_value(input).map_err(|_| invalid())?;
            let (job, id) = storage.enqueue_ingestion(&r)?;
            return serde_json::to_value(IngestionStatus {
                contract_version: 1,
                id,
                received: 0,
                job: Some(job),
            })
            .map_err(|_| unavailable());
        }
        let r: IngestionPart = serde_json::from_value(input).map_err(|_| invalid())?;
        validate_id(&r.workspace_id)?;
        validate_id(&r.id)?;
        storage.open(&r.workspace_id)?;
        if r.bytes.len() > MAX_UPLOAD_PART_BYTES {
            return Err(invalid());
        }
        let mut db = storage.connection(false)?.ok_or_else(unavailable)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        let job = crate::jobs::read(&tx, &r.workspace_id, &r.id)?;
        let (expected_bytes, expected_hash) = match &job.selection {
            ExportSelection::Ingestion {
                expected_bytes,
                expected_hash,
                ..
            } => (*expected_bytes, expected_hash.clone()),
            _ => return Err(invalid()),
        };
        if job.state != crate::jobs::JobState::Paused {
            return Err(conflict());
        }
        let Some(received) = received_from(&job.checkpoint) else {
            return Err(conflict());
        };
        if r.offset != received
            || r.offset.saturating_add(r.bytes.len() as u32) > expected_bytes as u32
        {
            return Err(conflict());
        }
        let path = staged_path(storage, &r.workspace_id, &r.id)?;
        crate::storage::reject_links(&path).map_err(|_| invalid())?;
        let file_len = std::fs::metadata(&path).map_err(|_| invalid())?.len();
        if file_len != u64::from(received) {
            return Err(conflict());
        }
        {
            use std::io::{Seek, Write};
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .map_err(|_| unavailable())?;
            file.seek(std::io::SeekFrom::Start(u64::from(received)))
                .map_err(|_| unavailable())?;
            file.write_all(&r.bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| unavailable())?;
        }
        let new_received = received + r.bytes.len() as u32;
        let progress = (u64::from(new_received) * 100 / expected_bytes) as u32;
        if r.finish {
            if u64::from(new_received) != expected_bytes {
                return Err(invalid());
            }
            // Verify the complete upload before the job may run: the
            // staged bytes must hash to the declared value.
            let staged =
                crate::snapshots::limited(&path, MAX_DOCUMENT_BYTES).map_err(|_| invalid())?;
            if staged.len() as u64 != expected_bytes || content::hash(&staged) != expected_hash {
                let failed = tx
                    .execute(
                        "UPDATE jobs SET state='Failed',version=version+1,checkpoint_version=checkpoint_version+1,checkpoint='discarded',owner=NULL,lease_until=NULL,updated_at=?1,log='Upload bytes do not match the declared hash; import the file again.' WHERE workspace_id=?2 AND id=?3 AND checkpoint_version=?4",
                        params![chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), r.workspace_id, r.id, job.checkpoint_version],
                    )
                    .map_err(|_| unavailable())?;
                if failed != 1 {
                    return Err(conflict());
                }
                tx.commit().map_err(|_| unavailable())?;
                let _ = std::fs::remove_file(&path);
                return Err(bad(
                    "Upload bytes do not match the declared hash; import the file again.",
                ));
            }
            let changed = tx
                .execute(
                    "UPDATE jobs SET state='Queued',version=version+1,checkpoint_version=checkpoint_version+1,checkpoint='staged',progress=100,updated_at=?1,log='Upload complete; queued for import' WHERE workspace_id=?2 AND id=?3 AND checkpoint_version=?4",
                    params![chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), r.workspace_id, r.id, job.checkpoint_version],
                )
                .map_err(|_| unavailable())?;
            if changed != 1 {
                return Err(conflict());
            }
        } else {
            let changed = tx
                .execute(
                    "UPDATE jobs SET checkpoint=?1,checkpoint_version=checkpoint_version+1,progress=?2,updated_at=?3 WHERE workspace_id=?4 AND id=?5 AND checkpoint_version=?6",
                    params![format!("uploading:{new_received}"), progress, chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), r.workspace_id, r.id, job.checkpoint_version],
                )
                .map_err(|_| unavailable())?;
            if changed != 1 {
                return Err(conflict());
            }
        }
        let job = crate::jobs::read(&tx, &r.workspace_id, &r.id)?;
        tx.commit().map_err(|_| unavailable())?;
        // A finished upload queued the job: wake the worker (this op
        // returns early from `job_command`, before its shared wake call).
        if r.finish {
            self.wake_export_worker();
        }
        serde_json::to_value(IngestionStatus {
            contract_version: 1,
            id: r.id,
            received: new_received,
            job: Some(job),
        })
        .map_err(|_| unavailable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn setup() -> (tempfile::TempDir, Core, String) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("storage");
        let core = Core::with_storage_root(root);
        let ws = core
            .workspace_command(
                "create",
                json!({"contractVersion":1,"name":"Ingestion fixture"}),
            )
            .unwrap()["workspace"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        (temp, core, ws)
    }

    fn begin(
        core: &Core,
        ws: &str,
        name: &str,
        bytes: &[u8],
        sha: &str,
        privacy: &str,
    ) -> serde_json::Value {
        core.job_command(
            "ingest-begin",
            json!({"contractVersion":1,"workspaceId":ws,"fileName":name,"bytes":bytes.len(),"sha256":sha,"privacy":privacy}),
        )
        .unwrap()
    }

    fn part(
        core: &Core,
        ws: &str,
        id: &str,
        offset: u32,
        bytes: &[u8],
        finish: bool,
    ) -> Result<serde_json::Value, CommandError> {
        core.job_command(
            "ingest-part",
            json!({"contractVersion":1,"workspaceId":ws,"id":id,"offset":offset,"bytes":bytes,"finish":finish}),
        )
    }

    /// Wait for the background worker (woken by the finish op) to
    /// drive the job to a terminal state. The worker runs on its own
    /// thread, so tests must poll rather than race a manual claim.
    fn wait_for_terminal(core: &Core, ws: &str, id: &str) -> Job {
        for _ in 0..200 {
            let job = read_job(core, ws, id);
            if matches!(
                job.state,
                crate::jobs::JobState::Completed
                    | crate::jobs::JobState::Failed
                    | crate::jobs::JobState::Cancelled
            ) {
                return job;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        read_job(core, ws, id)
    }

    /// Fabricate a staged, Queued ingestion job without waking the
    /// worker (used by tests that must own the claim themselves).
    /// Writes the staged bytes and moves the job row to Queued/staged
    /// directly; the upload path itself is covered by the happy-path
    /// test, which uses the real ops.
    fn stage_for_test(core: &Core, ws: &str, id: &str, bytes: &[u8]) {
        let storage = core.storage().unwrap();
        let path = storage
            .root_path()
            .join("exports")
            .join(ws)
            .join(format!("{id}.upload"));
        std::fs::write(&path, bytes).unwrap();
        storage
            .connection(false)
            .unwrap()
            .unwrap()
            .execute(
                "UPDATE jobs SET state='Queued',checkpoint='staged',progress=100 WHERE workspace_id=?1 AND id=?2",
                rusqlite::params![ws, id],
            )
            .unwrap();
    }

    fn read_job(core: &Core, ws: &str, id: &str) -> Job {
        core.storage().unwrap().read_job(ws, id).unwrap()
    }

    fn list_documents(core: &Core, ws: &str) -> serde_json::Value {
        core.memory_command(
            "list",
            json!({"contractVersion":1,"workspaceId":ws,"kind":"document"}),
        )
        .unwrap()
    }

    #[test]
    fn happy_path_publishes_document_with_provenance() {
        let (_temp, core, ws) = setup();
        let text = "# Notes\n\nLocal **first** document.";
        let bytes = text.as_bytes();
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "notes.md", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let job = read_job(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Paused);
        assert_eq!(job.checkpoint, "uploading:0");
        let done = part(&core, &ws, &id, 0, bytes, true).unwrap();
        assert_eq!(done["job"]["state"], "Queued");
        assert_eq!(done["job"]["checkpoint"], "staged");
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Completed);
        assert_eq!(job.checkpoint, "published");
        let memory_id = job.artifact_id.clone().unwrap();
        assert_eq!(job.artifact_bytes, Some(bytes.len() as u64));
        assert_eq!(job.artifact_hash, Some(sha.clone()));
        let read = core
            .memory_command(
                "read",
                json!({"contractVersion":1,"workspaceId":ws,"id":memory_id}),
            )
            .unwrap();
        assert_eq!(read["body"], text);
        assert_eq!(read["memory"]["kind"], "document");
        assert_eq!(read["memory"]["provenance"], "document_import");
        assert_eq!(read["memory"]["document"]["fileName"], "notes.md");
        assert_eq!(read["memory"]["document"]["mediaType"], "text/markdown");
        assert_eq!(read["memory"]["document"]["rawHash"], sha);
        assert_eq!(
            read["memory"]["document"]["textHash"],
            content::hash(text.as_bytes())
        );
        assert_eq!(read["memory"]["document"]["bom"], false);
        // The staged file is gone after a completed import.
        assert!(
            !std::path::Path::new(&format!(
                "{}/exports/{}/{}.upload",
                core.storage().unwrap().root_path().display(),
                ws,
                id
            ))
            .exists()
        );
    }

    #[test]
    fn duplicate_import_completes_with_the_same_record() {
        let (_temp, core, ws) = setup();
        let bytes = b"Same file twice.";
        let sha = content::hash(bytes);
        let mut ids = Vec::new();
        for _ in 0..2 {
            let opened = begin(&core, &ws, "twice.txt", bytes, &sha, "private");
            let id = opened["id"].as_str().unwrap().to_string();
            part(&core, &ws, &id, 0, bytes, true).unwrap();
            wait_for_terminal(&core, &ws, &id);
            ids.push(id);
        }
        let first = read_job(&core, &ws, &ids[0]);
        let second = read_job(&core, &ws, &ids[1]);
        assert_eq!(first.state, crate::jobs::JobState::Completed);
        assert_eq!(second.state, crate::jobs::JobState::Completed);
        assert_eq!(first.artifact_id, second.artifact_id);
        assert_eq!(second.log, "Already imported");
        assert_eq!(
            list_documents(&core, &ws)["memories"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn finish_hash_mismatch_fails_and_discards_staging() {
        let (_temp, core, ws) = setup();
        let sent = b"actually sent";
        let declared = content::hash(b"declared instead");
        let opened = begin(&core, &ws, "mismatch.txt", sent, &declared, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let error = part(&core, &ws, &id, 0, sent, true).unwrap_err();
        assert!(error.message.contains("do not match the declared hash"));
        let job = read_job(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Failed);
        assert_eq!(job.checkpoint, "discarded");
        assert_eq!(
            list_documents(&core, &ws)["memories"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn part_offset_mismatch_conflicts() {
        let (_temp, core, ws) = setup();
        let bytes = b"offset test";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "offset.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let error = part(&core, &ws, &id, 5, bytes, false).unwrap_err();
        assert_eq!(error.code, crate::ErrorCode::RevisionConflict);
        let job = read_job(&core, &ws, &id);
        assert_eq!(job.checkpoint, "uploading:0");
    }

    #[test]
    fn invalid_utf8_fails_without_publishing() {
        let (_temp, core, ws) = setup();
        let bytes = vec![0xff, 0xfe, b'A'];
        let sha = content::hash(&bytes);
        let opened = begin(&core, &ws, "bad.txt", &bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        part(&core, &ws, &id, 0, &bytes, true).unwrap();
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Failed);
        assert!(job.log.contains("not valid UTF-8"));
        assert_eq!(
            list_documents(&core, &ws)["memories"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn cancel_during_upload_removes_staging() {
        let (_temp, core, ws) = setup();
        let bytes = b"cancel me please";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "cancel.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        part(&core, &ws, &id, 0, &bytes[..4], false).unwrap();
        let job = read_job(&core, &ws, &id);
        core.job_command(
            "control",
            json!({"contractVersion":1,"workspaceId":ws,"id":id,"expectedVersion":job.version,"action":"cancel"}),
        )
        .unwrap();
        let job = read_job(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Cancelled);
        // A further part on a cancelled job conflicts; nothing resumes.
        assert!(part(&core, &ws, &id, 4, &bytes[4..], true).is_err());
        assert_eq!(
            list_documents(&core, &ws)["memories"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn resume_refused_until_upload_finishes() {
        let (_temp, core, ws) = setup();
        let bytes = b"resume guard";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "guard.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let job = read_job(&core, &ws, &id);
        let error = core
            .job_command(
                "control",
                json!({"contractVersion":1,"workspaceId":ws,"id":id,"expectedVersion":job.version,"action":"resume"}),
            )
            .unwrap_err();
        assert!(error.message.contains("resume the upload"));
    }

    #[test]
    fn begin_refusals_are_explicit() {
        let (_temp, core, ws) = setup();
        let bytes = b"small";
        let sha = content::hash(bytes);
        // Sealed: no encrypted path exists for imports.
        assert!(core
            .job_command(
                "ingest-begin",
                json!({"contractVersion":1,"workspaceId":ws,"fileName":"s.txt","bytes":5,"sha256":sha,"privacy":"sealed"}),
            )
            .is_err());
        // Unsupported format names the follow-up status.
        let error = core
            .job_command(
                "ingest-begin",
                json!({"contractVersion":1,"workspaceId":ws,"fileName":"report.pdf","bytes":5,"sha256":sha,"privacy":"private"}),
            )
            .unwrap_err();
        assert!(error.message.contains("not supported"));
        // Oversize (declared) refuses before any upload.
        assert!(core
            .job_command(
                "ingest-begin",
                json!({"contractVersion":1,"workspaceId":ws,"fileName":"big.txt","bytes":65537,"sha256":sha,"privacy":"private"}),
            )
            .is_err());
        // A forged upload id (path-like file name) refuses.
        assert!(core
            .job_command(
                "ingest-begin",
                json!({"contractVersion":1,"workspaceId":ws,"fileName":"../escape.txt","bytes":5,"sha256":sha,"privacy":"private"}),
            )
            .is_err());
    }

    #[test]
    fn forged_provenance_refused_by_memory_write() {
        let (_temp, core, ws) = setup();
        let body = "Forged provenance.";
        let error = core
            .memory_command(
                "create",
                json!({"contractVersion":1,"workspaceId":ws,"kind":"document","draft":{"title":"forged.txt","body":body,"privacy":"private"},"provenance":"document_import","document":{"fileName":"forged.txt","mediaType":"text/plain","rawHash":"0".repeat(64),"textHash":"0".repeat(64),"rawBytes":18,"textChars":18,"bom":false}}),
            )
            .unwrap_err();
        assert_eq!(error.code, crate::ErrorCode::InvalidMemory);
        assert_eq!(
            list_documents(&core, &ws)["memories"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn stale_lease_recovers_to_paused_then_resumes_once() {
        let (_temp, core, ws) = setup();
        let bytes = b"interrupted publish";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "interrupted.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        stage_for_test(&core, &ws, &id, bytes);
        let storage = core.storage().unwrap();
        let lease = storage.claim_export().unwrap().unwrap();
        assert_eq!(lease.job.id, id);
        // Simulate a killed worker: expire the lease, then list (which
        // runs recover_jobs) must flip the job to Paused with staging
        // intact. An explicit Resume then re-runs it exactly once.
        storage
            .connection(false)
            .unwrap()
            .unwrap()
            .execute(
                "UPDATE jobs SET lease_until=1 WHERE workspace_id=?1 AND id=?2",
                rusqlite::params![ws, id],
            )
            .unwrap();
        let jobs = storage.list_jobs(&ws, 0).unwrap();
        let job = jobs.iter().find(|j| j.id == id).unwrap();
        assert_eq!(job.state, crate::jobs::JobState::Paused);
        assert!(job.log.contains("Worker interrupted"));
        core.job_command(
            "control",
            json!({"contractVersion":1,"workspaceId":ws,"id":id,"expectedVersion":job.version,"action":"resume"}),
        )
        .unwrap();
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Completed);
        assert_eq!(
            list_documents(&core, &ws)["memories"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn cancelled_job_publishes_nothing() {
        let (_temp, core, ws) = setup();
        let bytes = b"cancelled before publish";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "late.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        stage_for_test(&core, &ws, &id, bytes);
        let storage = core.storage().unwrap();
        let lease = storage.claim_export().unwrap().unwrap();
        assert_eq!(lease.job.id, id);
        // Cancel wins the race: the worker's guard must refuse to publish.
        let job = read_job(&core, &ws, &id);
        core.job_command(
            "control",
            json!({"contractVersion":1,"workspaceId":ws,"id":id,"expectedVersion":job.version,"action":"cancel"}),
        )
        .unwrap();
        storage.execute_ingestion(&lease).unwrap();
        let job = read_job(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Cancelled);
        assert_eq!(
            list_documents(&core, &ws)["memories"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }
}
