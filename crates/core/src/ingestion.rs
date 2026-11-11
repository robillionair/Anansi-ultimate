//! Local document ingestion (M13): TXT/Markdown and text-layer PDF.
//!
//! The upload mechanics mirror the bounded backup upload
//! (`backup_upload.rs`): the client declares bytes + SHA-256, sends
//! sequential ≤64 KiB parts with offset-must-equal-received, and the
//! server re-reads and hash-verifies before the job runs. The durable
//! identity is the job row itself (selection `Ingestion`, checkpoint
//! `uploading:<received>` / `staged` / `published` / `discarded`), so
//! no second staging table exists and no schema migration was needed.
//!
//! The text step is per-kind. TXT/Markdown is decoded as UTF-8 (one
//! leading BOM stripped and recorded, 1–8,000 chars, 64 KiB raw).
//! PDF is text-extracted by the bounded extractor
//! (`crate::pdf::extract_pdf_text`): raw file at most 1 MiB, the
//! decode budget (16 MiB) and text caps (8,000 chars/page, 32,000
//! total) enforced during extraction, and the per-page provenance
//! stored on the published document. A refused PDF fails the job
//! with the extractor's honest message; no partial import is ever
//! published.
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
        DocumentProvenance, DocumentSource, MAX_ACTIVE_UPLOADS, MAX_UPLOAD_PART_BYTES,
        max_bytes_for_kind,
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

/// Decide which staged files can no longer be resumed: their job is
/// Completed/Cancelled, or no job row exists at all. Bounded (the
/// export directory holds at most a handful of `.upload` files) and
/// job-owned only — owner files are never touched.
///
/// The decision runs on the caller's open transaction, so the
/// job-state reads see the same snapshot as the insert that follows
/// and never contend with a concurrent begin on a second connection
/// (on Windows that cross-connection read failed with
/// StorageUnavailable and aborted the begin with HTTP 503). Only the
/// *paths* are returned: the caller deletes them after its
/// transaction commits, so a file is never removed while its job row
/// is still uncommitted — a concurrent begin's staged file can never
/// be mistaken for an orphan. Genuine storage failures (unreadable
/// directory, failed job lookup) still surface as errors; only a
/// single unreadable directory entry is skipped, and its file simply
/// waits for the next sweep.
fn sweep_staging(
    tx: &rusqlite::Transaction,
    storage: &Storage,
    workspace: &str,
) -> Result<Vec<std::path::PathBuf>, CommandError> {
    let directory = crate::export::directory(storage, workspace)?;
    let entries = std::fs::read_dir(&directory).map_err(|_| unavailable())?;
    let mut stale = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("upload") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let state: Option<String> = tx
            .query_row(
                "SELECT state FROM jobs WHERE workspace_id=?1 AND id=?2",
                params![workspace, stem],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| unavailable())?;
        match state.as_deref() {
            // Active uploads (Queued/Running/Paused) are always
            // preserved; only terminal or ownerless staging is stale.
            None | Some("Completed") | Some("Cancelled") => stale.push(path),
            _ => {}
        }
    }
    Ok(stale)
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
        if r.bytes == 0 || r.bytes > max_bytes_for_kind(kind) {
            return Err(bad(&format!(
                "Document file must be 1 to {} bytes. Larger files are refused before upload.",
                max_bytes_for_kind(kind)
            )));
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
        // The stale-staging decision runs inside this transaction, on
        // the same connection as the insert below — never across a
        // concurrent begin's uncommitted transaction.
        let stale = sweep_staging(&tx, self, &r.workspace_id)?;
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
        tx.execute(
            "INSERT INTO jobs(workspace_id,id,state,version,checkpoint_version,checkpoint,selection,created_at,updated_at,log) VALUES(?1,?2,'Paused',1,1,'uploading:0',?3,?4,?4,'Upload in progress')",
            params![r.workspace_id, id, serde_json::to_string(&selection).map_err(|_| invalid())?, stamp],
        )
        .map_err(|_| unavailable())?;
        let job = crate::jobs::read(&tx, &r.workspace_id, &id)?;
        tx.commit().map_err(|_| unavailable())?;
        // The staged file is created only after the job row commits:
        // a file never exists without its row, so a concurrent
        // begin's sweep can never mistake a live upload for an orphan.
        // If creation fails, the job is failed honestly instead of
        // sitting Paused with no staging to upload into, and the error
        // surfaces.
        let path = staged_path(self, &r.workspace_id, &id)?;
        if let Err(e) = crate::snapshots::new_file(&path) {
            self.fail_paused_ingestion(&r.workspace_id, &id);
            return Err(e);
        }
        for path in stale {
            let _ = crate::storage::reject_links(&path).map(|_| std::fs::remove_file(&path));
        }
        Ok((job, id))
    }

    /// Mark a Paused ingestion job Failed (best effort). Used when the
    /// staged file could not be created after the job row committed:
    /// the job never sits Paused with no staging to upload into, and
    /// the checkpoint reads `discarded` because no staging exists to
    /// resume. A later Retry fails honestly in the worker ("Upload
    /// staging is missing; import the file again.").
    fn fail_paused_ingestion(&self, workspace: &str, id: &str) {
        let Ok(Some(connection)) = self.connection(false) else {
            return;
        };
        let _ = connection.execute(
            "UPDATE jobs SET state='Failed',version=version+1,checkpoint_version=checkpoint_version+1,checkpoint='discarded',updated_at=?1,log='Upload staging could not be created.' WHERE workspace_id=?2 AND id=?3 AND state='Paused'",
            params![
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                workspace,
                id
            ],
        );
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
        let bytes = match crate::snapshots::limited(&path, max_bytes_for_kind(&kind)) {
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
        // The text step is per-kind. TXT/Markdown is decoded as UTF-8
        // (one leading BOM stripped and recorded). PDF is text-extracted
        // by the bounded extractor: the decode budget (16 MiB) and the
        // text caps (8,000 chars/page, 32,000 total) are enforced there.
        // DOCX is text-extracted by its bounded extractor (16 MiB
        // cumulative inflation, 128 entries, 32,000 chars), which also
        // returns per-section provenance. Any extractor refusal fails
        // the job with the extractor's honest message — no partial
        // import is ever published.
        let (text, bom, pages, sections) = if kind == "pdf" {
            match crate::pdf::extract_pdf_text(&bytes) {
                Ok(extracted) => (extracted.body, false, Some(extracted.page_provenance), None),
                Err(error) => {
                    let _ = self.fail_ingestion(lease, &error.message());
                    return Ok(());
                }
            }
        } else if kind == "docx" {
            match crate::docx::extract_docx_text(&bytes) {
                Ok(extracted) => (
                    extracted.body,
                    false,
                    None,
                    Some(extracted.section_provenance),
                ),
                Err(error) => {
                    let _ = self.fail_ingestion(lease, &error.message());
                    return Ok(());
                }
            }
        } else {
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
            if text.trim().is_empty()
                || text.chars().count() as u64 > crate::document::max_chars_for_kind(&kind)
            {
                let _ = self.fail_ingestion(
                    lease,
                    "Document text must be 1 to 8000 characters and not empty or whitespace-only.",
                );
                return Ok(());
            }
            (text, bom, None, None)
        };
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
                pages,
                sections,
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
        let (expected_bytes, expected_hash, kind) = match &job.selection {
            ExportSelection::Ingestion {
                expected_bytes,
                expected_hash,
                format,
                ..
            } => (*expected_bytes, expected_hash.clone(), format.clone()),
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
            // staged bytes must hash to the declared value. The read
            // cap is per-kind (a PDF may be up to 1 MiB).
            let staged = crate::snapshots::limited(&path, max_bytes_for_kind(&kind))
                .map_err(|_| invalid())?;
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
            self.wake_export_worker(crate::jobs::WakeKind::Deliberate);
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
    /// drive the job to a terminal state. The worker runs on its
    /// own thread and serves every storage in the process, so the
    /// wait is condition-based: it polls the job state until
    /// terminal, up to a bound, recording every state transition.
    /// On timeout it panics with the full history, so a failure
    /// names what the job did instead of only its last state. The
    /// wait observes only — it never re-wakes the worker, which
    /// would mask a lost wake-up, and never forces completion.
    fn wait_for_terminal(core: &Core, ws: &str, id: &str) -> Job {
        let start = std::time::Instant::now();
        let bound = std::time::Duration::from_secs(60);
        let mut history: Vec<String> = Vec::new();
        let mut last_state: Option<crate::jobs::JobState> = None;
        loop {
            let job = read_job(core, ws, id);
            if last_state.as_ref() != Some(&job.state) {
                history.push(format!(
                    "{:?}ms {:?} v{} checkpoint={:?} log={:?}",
                    start.elapsed().as_millis(),
                    job.state,
                    job.version,
                    job.checkpoint,
                    job.log
                ));
                last_state = Some(job.state.clone());
            }
            if matches!(
                job.state,
                crate::jobs::JobState::Completed
                    | crate::jobs::JobState::Failed
                    | crate::jobs::JobState::Cancelled
            ) {
                return job;
            }
            if start.elapsed() >= bound {
                panic!(
                    "job {id} in workspace {ws} never reached a terminal state within {bound:?}; transitions:\n{}",
                    history.join("\n")
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
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

    fn root_of(core: &Core) -> std::path::PathBuf {
        core.storage().unwrap().root_path().to_path_buf()
    }

    /// Wait until the worker parks this root after a claim
    /// failure. Bounded and condition-based: it polls the parked
    /// signal the worker records, never a timing assumption.
    /// Panics with the job's state on timeout.
    fn wait_for_parked(core: &Core, ws: &str, id: &str, root: &std::path::Path) {
        let start = std::time::Instant::now();
        let bound = std::time::Duration::from_secs(10);
        loop {
            if crate::jobs::claim_parked(root) {
                return;
            }
            if start.elapsed() >= bound {
                let job = read_job(core, ws, id);
                panic!(
                    "root {root:?} was never parked within {bound:?}; job state {:?} v{} log={:?}",
                    job.state, job.version, job.log
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }

    fn create_project(core: &Core, ws: &str) -> String {
        core.memory_command(
            "create",
            json!({
                "contractVersion": 1, "workspaceId": ws, "kind": "project",
                "draft": { "title": "Harbor App", "body": "Product project.", "privacy": "private",
                           "eventDate": null, "validFrom": null, "validUntil": null,
                           "confidence": null, "sourceReferences": [] }
            }),
        )
        .unwrap()["memory"]["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn begin_project(
        core: &Core,
        ws: &str,
        name: &str,
        bytes: &[u8],
        sha: &str,
        pid: &str,
    ) -> serde_json::Value {
        core.job_command(
            "ingest-begin",
            json!({"contractVersion":1,"workspaceId":ws,"fileName":name,"bytes":bytes.len(),"sha256":sha,"privacy":"private","projectId":pid}),
        )
        .unwrap()
    }

    /// The public polling path: a plain job list. Passive
    /// observation only — it must never re-arm an exhausted
    /// root (CR-01). Tests that need a deliberate re-arm use
    /// `recover` instead.
    fn wake(core: &Core, ws: &str) {
        core.job_command(
            "list",
            json!({"contractVersion":1,"workspaceId":ws,"offset":0}),
        )
        .unwrap();
    }

    /// The explicit recovery path (CR-02): a version-fenced
    /// control that re-arms an exhausted root. Returns the
    /// control response job.
    fn recover(core: &Core, ws: &str, id: &str) -> crate::jobs::Job {
        let job = read_job(core, ws, id);
        let response = core
            .job_command(
                "control",
                json!({"contractVersion":1,"workspaceId":ws,"id":id,"expectedVersion":job.version,"action":"recover"}),
            )
            .unwrap();
        serde_json::from_value(response["job"].clone()).unwrap()
    }

    /// The job as the public polling path sees it, including any
    /// recovery diagnostic.
    fn listed_job(core: &Core, ws: &str, id: &str) -> crate::jobs::Job {
        let response = core
            .job_command(
                "list",
                json!({"contractVersion":1,"workspaceId":ws,"offset":0}),
            )
            .unwrap();
        let jobs = response["jobs"].as_array().unwrap();
        let found = jobs
            .iter()
            .find(|job| job["id"].as_str() == Some(id))
            .expect("job present in list");
        serde_json::from_value(found.clone()).unwrap()
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
    fn pdf_happy_path_publishes_document_with_page_provenance() {
        let (_temp, core, ws) = setup();
        let bytes = include_bytes!("../../../tests/fixtures/pdf/normal-3page.pdf");
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "normal-3page.pdf", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let done = part(&core, &ws, &id, 0, bytes, true).unwrap();
        assert_eq!(done["job"]["state"], "Queued");
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Completed);
        let memory_id = job.artifact_id.clone().unwrap();
        let read = core
            .memory_command(
                "read",
                json!({"contractVersion":1,"workspaceId":ws,"id":memory_id}),
            )
            .unwrap();
        // The stored body is the three pages joined by form feed.
        let body = read["body"].as_str().unwrap();
        assert!(body.contains("Harbor Ledger"));
        assert!(body.contains("Second page"));
        assert!(body.contains("Third page"));
        assert_eq!(body.matches('\x0c').count(), 2);
        let doc = &read["memory"]["document"];
        assert_eq!(doc["fileName"], "normal-3page.pdf");
        assert_eq!(doc["mediaType"], "application/pdf");
        assert_eq!(doc["rawHash"], sha);
        assert_eq!(doc["bom"], false);
        // Page provenance: three pages, contiguous, covering the body.
        let pages = doc["pages"].as_array().unwrap();
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0]["page"], 1);
        assert_eq!(pages[1]["page"], 2);
        assert_eq!(pages[2]["page"], 3);
        assert_eq!(pages[0]["startChar"], 0);
        let total: u64 = pages.iter().map(|p| p["chars"].as_u64().unwrap()).sum();
        assert_eq!(total + 2 * 3, body.chars().count() as u64);
        // Each page hash is the SHA-256 of that page's text.
        let page_texts: Vec<&str> = body.split("\n\x0c\n").collect();
        assert_eq!(page_texts.len(), 3);
        for (page, text) in page_texts.iter().enumerate() {
            assert_eq!(
                pages[page]["hash"],
                content::hash(text.as_bytes()),
                "page {} hash must match its text",
                page + 1
            );
        }
        // Full-body search finds a term that lives only on page 2.
        let found = core
            .memory_command(
                "list",
                json!({"contractVersion":1,"workspaceId":ws,"kind":"document","query":"aurora","fullBody":true}),
            )
            .unwrap();
        assert_eq!(found["memories"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn docx_happy_path_publishes_document_with_section_provenance() {
        let (_temp, core, ws) = setup();
        let bytes = include_bytes!("../../../tests/fixtures/docx/normal.docx");
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "normal.docx", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let done = part(&core, &ws, &id, 0, bytes, true).unwrap();
        assert_eq!(done["job"]["state"], "Queued");
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Completed);
        let memory_id = job.artifact_id.clone().unwrap();
        let read = core
            .memory_command(
                "read",
                json!({"contractVersion":1,"workspaceId":ws,"id":memory_id}),
            )
            .unwrap();
        // The stored body is paragraphs in order, headings as `#` lines.
        let body = read["body"].as_str().unwrap();
        assert!(body.starts_with("# Harbor Ledger\n"));
        assert!(body.contains("## Manifest"));
        assert!(body.contains("### Notes"));
        assert!(!body.contains('\x0c'));
        let doc = &read["memory"]["document"];
        assert_eq!(doc["fileName"], "normal.docx");
        assert_eq!(
            doc["mediaType"],
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        );
        assert_eq!(doc["rawHash"], sha);
        assert_eq!(doc["bom"], false);
        // Section provenance: four sections, contiguous, covering the body.
        let sections = doc["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 4);
        assert_eq!(sections[0]["heading"], "Harbor Ledger");
        assert_eq!(sections[0]["level"], 1);
        assert_eq!(sections[1]["heading"], "Customs");
        assert_eq!(sections[2]["heading"], "Manifest");
        assert_eq!(sections[2]["level"], 2);
        assert_eq!(sections[3]["heading"], "Notes");
        assert_eq!(sections[3]["level"], 3);
        assert_eq!(sections[0]["startChar"], 0);
        let total: u64 = sections.iter().map(|s| s["chars"].as_u64().unwrap()).sum();
        // Three single-newline separators between four sections.
        assert_eq!(total + 3, body.chars().count() as u64);
        // Each section hash is the SHA-256 of that section's text,
        // sliced from the body by its recorded range.
        for section in sections {
            let start = section["startChar"].as_u64().unwrap() as usize;
            let chars = section["chars"].as_u64().unwrap() as usize;
            let text: String = body.chars().skip(start).take(chars).collect();
            assert_eq!(
                section["hash"],
                content::hash(text.as_bytes()),
                "section {:?} hash must match its text",
                section["heading"]
            );
        }
        // Full-body search finds a term that lives only in one section.
        let found = core
            .memory_command(
                "list",
                json!({"contractVersion":1,"workspaceId":ws,"kind":"document","query":"Customs","fullBody":true}),
            )
            .unwrap();
        assert_eq!(found["memories"].as_array().unwrap().len(), 1);
        // The chunker gives the section heading for the Ask citation:
        // the heading line is stored verbatim (with its `#` prefix),
        // as for Markdown documents.
        let chunks =
            crate::context::chunk_markdown_sections("test", "normal.docx", "document", body);
        let manifest = chunks
            .iter()
            .find(|c| c.section_heading.as_deref() == Some("## Manifest"))
            .unwrap();
        assert_eq!(manifest.section_heading.as_deref(), Some("## Manifest"));
    }

    #[test]
    fn docx_begin_uses_the_docx_raw_limit() {
        let (_temp, core, ws) = setup();
        let sha = content::hash(b"declared");
        // A DOCX between the TXT/Markdown cap (64 KiB) and the DOCX
        // cap (1 MiB) is accepted at begin: the selection validator
        // must use the per-kind limit, not the 64 KiB text cap.
        assert!(core
            .job_command(
                "ingest-begin",
                json!({"contractVersion":1,"workspaceId":ws,"fileName":"big.docx","bytes":100_000,"sha256":sha,"privacy":"private"}),
            )
            .is_ok());
        // Over the 1 MiB DOCX cap it still refuses before any upload.
        assert!(core
            .job_command(
                "ingest-begin",
                json!({"contractVersion":1,"workspaceId":ws,"fileName":"huge.docx","bytes":1_048_577,"sha256":sha,"privacy":"private"}),
            )
            .is_err());
    }

    #[test]
    fn docx_refusals_publish_nothing() {
        let (_temp, core, ws) = setup();
        // (fixture, a substring the job log must carry)
        let cases: [(&str, &[u8], &str); 6] = [
            (
                "macro.docx",
                include_bytes!("../../../tests/fixtures/docx/macro.docx"),
                "does not support",
            ),
            (
                "altchunk.docx",
                include_bytes!("../../../tests/fixtures/docx/altchunk.docx"),
                "does not support",
            ),
            (
                "encrypted-entry.docx",
                include_bytes!("../../../tests/fixtures/docx/encrypted-entry.docx"),
                "password-protected",
            ),
            (
                "zip-bomb.docx",
                include_bytes!("../../../tests/fixtures/docx/zip-bomb.docx"),
                "could not be parsed",
            ),
            (
                "oversized-text.docx",
                include_bytes!("../../../tests/fixtures/docx/oversized-text.docx"),
                "exceeds the limit",
            ),
            (
                "corrupt-bad-xml.docx",
                include_bytes!("../../../tests/fixtures/docx/corrupt-bad-xml.docx"),
                "could not be parsed",
            ),
        ];
        for (name, bytes, expected) in cases {
            let sha = content::hash(bytes);
            let opened = begin(&core, &ws, name, bytes, &sha, "private");
            let id = opened["id"].as_str().unwrap().to_string();
            let done = part(&core, &ws, &id, 0, bytes, true).unwrap();
            assert_eq!(done["job"]["state"], "Queued");
            let job = wait_for_terminal(&core, &ws, &id);
            assert_eq!(job.state, crate::jobs::JobState::Failed, "{name} must fail");
            assert!(
                job.log.contains(expected),
                "{name}: log must contain {expected:?}, got {:?}",
                job.log
            );
            // Nothing was published for the refused file.
            let listed = core
                .memory_command(
                    "list",
                    json!({"contractVersion":1,"workspaceId":ws,"kind":"document","query":name,"fullBody":true}),
                )
                .unwrap();
            assert_eq!(listed["memories"].as_array().unwrap().len(), 0);
        }
    }

    #[test]
    fn pdf_begin_uses_the_pdf_raw_limit() {
        let (_temp, core, ws) = setup();
        let sha = content::hash(b"declared");
        // A PDF between the TXT/Markdown cap (64 KiB) and the PDF cap
        // (1 MiB) is accepted at begin: the selection validator must
        // use the per-kind limit, not the 64 KiB text cap.
        assert!(core
            .job_command(
                "ingest-begin",
                json!({"contractVersion":1,"workspaceId":ws,"fileName":"big.pdf","bytes":100_000,"sha256":sha,"privacy":"private"}),
            )
            .is_ok());
        // Over the 1 MiB PDF cap it still refuses before any upload.
        assert!(core
            .job_command(
                "ingest-begin",
                json!({"contractVersion":1,"workspaceId":ws,"fileName":"huge.pdf","bytes":1_048_577,"sha256":sha,"privacy":"private"}),
            )
            .is_err());
    }

    #[test]
    fn pdf_refusals_publish_nothing() {
        let (_temp, core, ws) = setup();
        // (fixture, a substring the job log must carry)
        let cases: [(&str, &[u8], &str); 4] = [
            (
                "encrypted-user.pdf",
                include_bytes!("../../../tests/fixtures/pdf/encrypted-user.pdf"),
                "password-protected",
            ),
            (
                "scanned-image.pdf",
                include_bytes!("../../../tests/fixtures/pdf/scanned-image.pdf"),
                "no readable text",
            ),
            (
                "decompression-bomb.pdf",
                include_bytes!("../../../tests/fixtures/pdf/decompression-bomb.pdf"),
                "decode budget",
            ),
            (
                "malformed-truncated.pdf",
                include_bytes!("../../../tests/fixtures/pdf/malformed-truncated.pdf"),
                "could not be parsed",
            ),
        ];
        for (name, bytes, expected) in cases {
            let sha = content::hash(bytes);
            let opened = begin(&core, &ws, name, bytes, &sha, "private");
            let id = opened["id"].as_str().unwrap().to_string();
            part(&core, &ws, &id, 0, bytes, true).unwrap();
            let job = wait_for_terminal(&core, &ws, &id);
            assert_eq!(job.state, crate::jobs::JobState::Failed, "{name} must fail");
            assert!(
                job.log.contains(expected),
                "{name} log {job:?} must mention {expected}"
            );
        }
        assert_eq!(
            list_documents(&core, &ws)["memories"]
                .as_array()
                .unwrap()
                .len(),
            0,
            "refused PDFs must publish no document"
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
        // Unsupported format names the follow-up status (DOCX is
        // supported since the M13 DOCX slice; PPTX still refuses).
        let error = core
            .job_command(
                "ingest-begin",
                json!({"contractVersion":1,"workspaceId":ws,"fileName":"report.pptx","bytes":5,"sha256":sha,"privacy":"private"}),
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

    #[test]
    fn sweep_preserves_active_uploads_and_removes_stale_staging() {
        let (_temp, core, ws) = setup();
        let storage = core.storage().unwrap();
        let exports = storage.root_path().join("exports").join(&ws);
        // An active (Paused) upload whose staging must survive the sweep.
        let bytes = b"active upload stays";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "active.txt", bytes, &sha, "private");
        let active_id = opened["id"].as_str().unwrap().to_string();
        let active_path = exports.join(format!("{active_id}.upload"));
        assert!(active_path.exists());
        // A completed import whose staged file the worker already
        // removed; plant a stale file at its path, plus an ownerless one.
        let done_bytes = b"already done";
        let done = begin(
            &core,
            &ws,
            "done.txt",
            done_bytes,
            &content::hash(done_bytes),
            "private",
        );
        let done_id = done["id"].as_str().unwrap().to_string();
        part(&core, &ws, &done_id, 0, done_bytes, true).unwrap();
        assert_eq!(
            wait_for_terminal(&core, &ws, &done_id).state,
            crate::jobs::JobState::Completed
        );
        let done_path = exports.join(format!("{done_id}.upload"));
        assert!(!done_path.exists());
        std::fs::write(&done_path, b"stale bytes").unwrap();
        let orphan_path = exports.join("00000000000000000000000000000000.upload");
        std::fs::write(&orphan_path, b"no job owns this").unwrap();
        // The next begin runs the sweep inside its transaction.
        let next_bytes = b"next";
        let next = begin(
            &core,
            &ws,
            "next.txt",
            next_bytes,
            &content::hash(next_bytes),
            "private",
        );
        let next_id = next["id"].as_str().unwrap().to_string();
        assert!(active_path.exists(), "sweep deleted an active upload");
        assert!(!done_path.exists(), "sweep kept a completed job's staging");
        assert!(!orphan_path.exists(), "sweep kept ownerless staging");
        assert!(exports.join(format!("{next_id}.upload")).exists());
    }

    #[test]
    fn enqueue_surfaces_genuine_storage_failure() {
        let (temp, core, ws) = setup();
        // Plant a regular file where the workspace export directory
        // belongs, so the staging sweep cannot list it: a genuine
        // storage failure must surface from ingest-begin, never be
        // swallowed by cleanup.
        let exports = temp.path().join("storage").join("exports");
        std::fs::create_dir_all(&exports).unwrap();
        std::fs::write(exports.join(&ws), b"not a directory").unwrap();
        let bytes = b"nowhere to stage";
        let error = core
            .job_command(
                "ingest-begin",
                json!({"contractVersion":1,"workspaceId":ws,"fileName":"blocked.txt","bytes":bytes.len(),"sha256":content::hash(bytes),"privacy":"private"}),
            )
            .unwrap_err();
        assert_eq!(error.code, crate::ErrorCode::StorageUnavailable);
        let list = core
            .job_command(
                "list",
                json!({"contractVersion":1,"workspaceId":ws,"offset":0}),
            )
            .unwrap();
        assert!(list["jobs"].as_array().unwrap().is_empty());
    }

    #[test]
    fn concurrent_begin_calls_all_succeed_and_publish_once() {
        let (_temp, core, ws) = setup();
        let bytes = b"The same file, imported eight times at once.";
        let sha = content::hash(bytes);
        // Eight concurrent ingest-begin calls for the identical file:
        // the staging sweep must never abort a begin (the Windows
        // finding: cross-connection reads failed with 503), and the
        // atomic publish guard must still publish exactly one document.
        let opened: Vec<Result<serde_json::Value, CommandError>> = std::thread::scope(|s| {
            let mut handles = Vec::new();
            for _ in 0..8 {
                handles.push(s.spawn(|| {
                        core.job_command(
                            "ingest-begin",
                            json!({"contractVersion":1,"workspaceId":ws,"fileName":"eight.txt","bytes":bytes.len(),"sha256":sha,"privacy":"private"}),
                        )
                    }));
            }
            handles
                .into_iter()
                .map(|h| h.join().expect("begin thread panicked"))
                .collect()
        });
        assert_eq!(opened.len(), 8);
        let mut ids = Vec::new();
        for result in &opened {
            let value = result
                .as_ref()
                .unwrap_or_else(|e| panic!("concurrent ingest-begin failed: {e:?}"));
            ids.push(value["id"].as_str().unwrap().to_string());
        }
        for id in &ids {
            part(&core, &ws, id, 0, bytes, true).unwrap();
        }
        let mut artifact_id = None;
        for id in &ids {
            let job = wait_for_terminal(&core, &ws, id);
            assert_eq!(job.state, crate::jobs::JobState::Completed);
            match artifact_id {
                None => artifact_id = job.artifact_id,
                Some(ref first) => assert_eq!(job.artifact_id.as_ref(), Some(first)),
            }
        }
        assert!(artifact_id.is_some());
        assert_eq!(
            list_documents(&core, &ws)["memories"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn burst_of_storages_all_complete() {
        // Eight fresh storages, one ingestion job each, all woken in
        // a tight burst: the single process-global worker must serve
        // every pending storage. This is the deterministic probe for
        // the broad-run scheduling condition (Windows finding: a job
        // stuck Queued past a fixed poll window while siblings
        // completed). No timing assumption beyond the bounded
        // diagnostic wait.
        let mut fixtures = Vec::new();
        for _ in 0..8 {
            let (temp, core, ws) = setup();
            let bytes = b"Burst import.";
            let sha = content::hash(bytes);
            let opened = begin(&core, &ws, "burst.txt", bytes, &sha, "private");
            let id = opened["id"].as_str().unwrap().to_string();
            part(&core, &ws, &id, 0, bytes, true).unwrap();
            fixtures.push((temp, core, ws, id));
        }
        for (_temp, core, ws, id) in &fixtures {
            let job = wait_for_terminal(core, ws, id);
            assert_eq!(job.state, crate::jobs::JobState::Completed);
        }
        for (_temp, core, ws, _id) in &fixtures {
            assert_eq!(
                list_documents(core, ws)["memories"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
        }
    }

    #[test]
    fn reviewer_status_poll_does_not_rearm_exhausted_claim_budget() {
        let (_temp, core, ws) = setup();
        let (_fence_temp, fence_core, fence_ws) = setup();
        let bytes = b"Read-only polling must not restart exhausted recovery.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "poll.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        let before = crate::jobs::remaining_claim_faults(&root);
        assert_eq!(before, 1_000_000 - 3);
        crate::jobs::clear_claim_parked(&root);
        // This is the public operation Documents calls automatically every 4s.
        let response = core
            .job_command(
                "list",
                json!({"contractVersion":1,"workspaceId":ws,"offset":0}),
            )
            .unwrap();
        println!("status response after exhausted recovery: {response}");
        // A healthy root queued after the read is a worker-progress fence. If
        // the read re-armed the failed root, FIFO must attempt that root before
        // completing the fence. A corrected read can leave the root parked and
        // still complete the fence; no sleep or second parking event is needed.
        let fence_bytes = b"Healthy worker progress fence.";
        let fence_sha = content::hash(fence_bytes);
        let fence_opened = begin(
            &fence_core,
            &fence_ws,
            "fence.txt",
            fence_bytes,
            &fence_sha,
            "private",
        );
        let fence_id = fence_opened["id"].as_str().unwrap().to_string();
        part(&fence_core, &fence_ws, &fence_id, 0, fence_bytes, true).unwrap();
        assert_eq!(
            wait_for_terminal(&fence_core, &fence_ws, &fence_id).state,
            crate::jobs::JobState::Completed,
        );
        let after = crate::jobs::remaining_claim_faults(&root);
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
        assert_eq!(
            after, before,
            "read-only job polling re-armed an exhausted claim budget"
        );
    }

    #[test]
    fn transient_claim_failure_recovers_without_another_wake() {
        let (_temp, core, ws) = setup();
        let bytes = b"Transient recovery.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "transient.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 2, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        // No further user action: the worker's own retries must
        // carry the job to completion.
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Completed);
        assert_eq!(crate::jobs::remaining_claim_faults(&root), 0);
        assert!(!crate::jobs::claim_parked(&root));
        let docs = list_documents(&core, &ws)["memories"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0]["privacy"].as_str().unwrap(), "private");
    }

    #[test]
    fn failing_root_does_not_block_other_roots() {
        let (_temp_a, core_a, ws_a) = setup();
        let (_temp_b, core_b, ws_b) = setup();
        let bytes_a = b"Stranded import.";
        let sha_a = content::hash(bytes_a);
        let opened_a = begin(&core_a, &ws_a, "a.txt", bytes_a, &sha_a, "private");
        let id_a = opened_a["id"].as_str().unwrap().to_string();
        let bytes_b = b"Healthy import.";
        let sha_b = content::hash(bytes_b);
        let opened_b = begin(&core_b, &ws_b, "b.txt", bytes_b, &sha_b, "private");
        let id_b = opened_b["id"].as_str().unwrap().to_string();
        let root_a = root_of(&core_a);
        crate::jobs::arm_claim_faults(&root_a, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core_a, &ws_a, &id_a, 0, bytes_a, true).unwrap();
        part(&core_b, &ws_b, &id_b, 0, bytes_b, true).unwrap();
        // B completes while A is still failing.
        let job_b = wait_for_terminal(&core_b, &ws_b, &id_b);
        assert_eq!(job_b.state, crate::jobs::JobState::Completed);
        // A is parked, its job still Queued and observable, after
        // exactly the design's budget of 3 attempts.
        wait_for_parked(&core_a, &ws_a, &id_a, &root_a);
        let job_a = read_job(&core_a, &ws_a, &id_a);
        assert_eq!(job_a.state, crate::jobs::JobState::Queued);
        assert_eq!(crate::jobs::remaining_claim_faults(&root_a), 1_000_000 - 3);
        crate::jobs::clear_claim_faults(&root_a);
    }

    #[test]
    fn persistent_claim_failure_is_bounded_and_parks() {
        let (_temp, core, ws) = setup();
        let bytes = b"Persistent failure.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "persistent.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        // Exactly the design's budget of 3 attempts, never a spin.
        assert_eq!(crate::jobs::remaining_claim_faults(&root), 1_000_000 - 3);
        let job = read_job(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Queued);
        // Recovery after the fault clears, via the explicit
        // recovery action (a plain poll must not re-arm).
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
        recover(&core, &ws, &id);
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Completed);
    }

    #[test]
    fn corrupt_claim_failure_parks_immediately_and_recovers_on_wake() {
        let (_temp, core, ws) = setup();
        let bytes = b"Corrupt catalog.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "corrupt.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::CorruptStorage);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        // One attempt only: a corrupt catalog is never retried.
        assert_eq!(crate::jobs::remaining_claim_faults(&root), 1_000_000 - 1);
        let job = read_job(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Queued);
        // Recovery after the fault clears, via the explicit
        // recovery action (a plain poll must not re-arm).
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
        recover(&core, &ws, &id);
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Completed);
    }

    #[test]
    fn cancel_during_claim_retries_stays_cancelled() {
        let (_temp, core, ws) = setup();
        let bytes = b"Cancel me during retries.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "cancel-retry.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        // Cancel while the worker is mid-retry: the control path
        // runs on this thread and must stay responsive. The claim
        // faults mean the worker can hold no lease, so the version
        // read here cannot race the worker.
        let job = read_job(&core, &ws, &id);
        core.job_command(
            "control",
            json!({"contractVersion":1,"workspaceId":ws,"id":id,"expectedVersion":job.version,"action":"cancel"}),
        )
        .unwrap();
        let job = read_job(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Cancelled);
        // The worker parks; clearing the fault and polling must
        // never resurrect the cancelled job. No sleep is needed:
        // a claim selects Queued jobs only, so a Cancelled job is
        // unreachable by recovery whatever the worker's timing.
        wait_for_parked(&core, &ws, &id, &root);
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
        wake(&core, &ws);
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

    #[test]
    fn recovery_publishes_once_with_declared_privacy_and_project() {
        let (_temp, core, ws) = setup();
        let pid = create_project(&core, &ws);
        let bytes = b"Project import under faults.";
        let sha = content::hash(bytes);
        let opened = begin_project(&core, &ws, "project.txt", bytes, &sha, &pid);
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 2, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Completed);
        assert!(job.artifact_id.is_some());
        let docs = list_documents(&core, &ws)["memories"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0]["privacy"].as_str().unwrap(), "private");
        assert_eq!(docs[0]["projectId"].as_str().unwrap(), pid);
    }

    #[test]
    fn repeated_polls_do_not_rearm_persistent_claim_failure() {
        let (_temp, core, ws) = setup();
        let (_fence_temp, fence_core, fence_ws) = setup();
        let bytes = b"Repeated polling must not restart recovery.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "poll-many.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        assert_eq!(crate::jobs::remaining_claim_faults(&root), 1_000_000 - 3);
        // Three ordinary polls, as Documents would issue them.
        wake(&core, &ws);
        wake(&core, &ws);
        wake(&core, &ws);
        // Worker-progress fence: a healthy root completes, proving
        // the worker is still serving others while this root
        // stays parked.
        let fence_bytes = b"Healthy fence for repeated polls.";
        let fence_sha = content::hash(fence_bytes);
        let fence_opened = begin(
            &fence_core,
            &fence_ws,
            "fence.txt",
            fence_bytes,
            &fence_sha,
            "private",
        );
        let fence_id = fence_opened["id"].as_str().unwrap().to_string();
        part(&fence_core, &fence_ws, &fence_id, 0, fence_bytes, true).unwrap();
        assert_eq!(
            wait_for_terminal(&fence_core, &fence_ws, &fence_id).state,
            crate::jobs::JobState::Completed,
        );
        let after = crate::jobs::remaining_claim_faults(&root);
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
        assert_eq!(after, 1_000_000 - 3);
    }

    #[test]
    fn polls_do_not_rearm_non_retryable_claim_failure() {
        let (_temp, core, ws) = setup();
        let (_fence_temp, fence_core, fence_ws) = setup();
        let bytes = b"Corrupt parking survives polling.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "poll-corrupt.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::CorruptStorage);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        assert_eq!(crate::jobs::remaining_claim_faults(&root), 1_000_000 - 1);
        wake(&core, &ws);
        wake(&core, &ws);
        wake(&core, &ws);
        let fence_bytes = b"Healthy fence for corrupt polls.";
        let fence_sha = content::hash(fence_bytes);
        let fence_opened = begin(
            &fence_core,
            &fence_ws,
            "fence.txt",
            fence_bytes,
            &fence_sha,
            "private",
        );
        let fence_id = fence_opened["id"].as_str().unwrap().to_string();
        part(&fence_core, &fence_ws, &fence_id, 0, fence_bytes, true).unwrap();
        assert_eq!(
            wait_for_terminal(&fence_core, &fence_ws, &fence_id).state,
            crate::jobs::JobState::Completed,
        );
        let after = crate::jobs::remaining_claim_faults(&root);
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
        assert_eq!(after, 1_000_000 - 1);
    }

    #[test]
    fn new_work_rearms_exhausted_root_and_serves_stranded_job() {
        let (_temp, core, ws) = setup();
        let bytes = b"Stranded by exhaustion.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "stranded.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
        // Genuinely new work on the same root is a deliberate
        // wake: it re-arms the root and the worker serves the
        // stranded job too, in FIFO order.
        let second = b"New work re-arms.";
        let second_sha = content::hash(second);
        let opened2 = begin(&core, &ws, "second.txt", second, &second_sha, "private");
        let id2 = opened2["id"].as_str().unwrap().to_string();
        part(&core, &ws, &id2, 0, second, true).unwrap();
        assert_eq!(
            wait_for_terminal(&core, &ws, &id).state,
            crate::jobs::JobState::Completed
        );
        assert_eq!(
            wait_for_terminal(&core, &ws, &id2).state,
            crate::jobs::JobState::Completed
        );
    }

    #[test]
    fn deliberate_wake_rearms_exhausted_root() {
        let (_temp, core, ws) = setup();
        let bytes = b"Deliberate wake re-arms.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "deliberate.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
        // The deliberate wake API (startup, new work, explicit
        // controls) re-arms; polling alone never does.
        core.wake_export_worker(crate::jobs::WakeKind::Deliberate);
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Completed);
    }

    #[test]
    fn diagnostic_shown_for_exhausted_queued_job_without_row_mutation() {
        let (_temp, core, ws) = setup();
        let bytes = b"Diagnosed exhaustion.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "diagnosed.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        // The durable row is byte-identical to the pre-parking
        // state: parking wrote nothing without a lease.
        let row = read_job(&core, &ws, &id);
        assert_eq!(row.state, crate::jobs::JobState::Queued);
        assert_eq!(row.version, 2);
        assert_eq!(row.checkpoint, "staged");
        assert_eq!(row.checkpoint_version, 2);
        assert_eq!(row.progress, 100);
        assert_eq!(row.log, "Upload complete; queued for import");
        assert!(row.recovery_diagnostic.is_none());
        // The public polling path carries the diagnostic, with
        // the exact fixed message (sanitization by equality).
        let listed = listed_job(&core, &ws, &id);
        let diagnostic = listed.recovery_diagnostic.expect("diagnostic present");
        assert_eq!(
            diagnostic.reason,
            crate::jobs::JobRecoveryReason::StorageUnavailable
        );
        assert_eq!(diagnostic.attempts, 3);
        assert_eq!(
            diagnostic.message,
            "Recovery stopped because storage was unavailable. Check the storage folder and permissions, then choose Retry recovery."
        );
        assert!(!diagnostic.message.contains(root.to_str().unwrap()));
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
    }

    #[test]
    fn diagnostic_for_corrupt_exhaustion_names_restore() {
        let (_temp, core, ws) = setup();
        let bytes = b"Corrupt diagnosis.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "corrupt-diagnosis.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::CorruptStorage);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        let listed = listed_job(&core, &ws, &id);
        let diagnostic = listed.recovery_diagnostic.expect("diagnostic present");
        assert_eq!(
            diagnostic.reason,
            crate::jobs::JobRecoveryReason::CorruptStorage
        );
        assert_eq!(diagnostic.attempts, 1);
        assert_eq!(
            diagnostic.message,
            "Recovery stopped because storage could not be read. Keep the original files and restore a verified backup, then choose Retry recovery."
        );
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
    }

    #[test]
    fn diagnostic_absent_for_other_states_and_roots() {
        let (_temp, core, ws) = setup();
        let (_other_temp, other_core, other_ws) = setup();
        // A Paused job on the exhausted root: the upload never
        // finished, so it is not stranded by claim recovery.
        let paused_bytes = b"Paused upload.";
        let paused_sha = content::hash(paused_bytes);
        let paused_opened = begin(
            &core,
            &ws,
            "paused.txt",
            paused_bytes,
            &paused_sha,
            "private",
        );
        let paused_id = paused_opened["id"].as_str().unwrap().to_string();
        let bytes = b"Exhausted root job.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "exhausted.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        // The Queued job carries the diagnostic; the Paused job
        // on the same root does not.
        assert!(listed_job(&core, &ws, &id).recovery_diagnostic.is_some());
        assert_eq!(
            listed_job(&core, &ws, &paused_id).state,
            crate::jobs::JobState::Paused
        );
        assert!(
            listed_job(&core, &ws, &paused_id)
                .recovery_diagnostic
                .is_none()
        );
        // A job on a healthy root never sees another root's
        // diagnostic.
        let healthy_bytes = b"Healthy root job.";
        let healthy_sha = content::hash(healthy_bytes);
        let healthy_opened = begin(
            &other_core,
            &other_ws,
            "healthy.txt",
            healthy_bytes,
            &healthy_sha,
            "private",
        );
        let healthy_id = healthy_opened["id"].as_str().unwrap().to_string();
        assert!(
            listed_job(&other_core, &other_ws, &healthy_id)
                .recovery_diagnostic
                .is_none()
        );
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
    }

    #[test]
    fn recover_action_recovers_and_clears_diagnostic() {
        let (_temp, core, ws) = setup();
        let bytes = b"Explicit recovery.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "recover.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        assert!(listed_job(&core, &ws, &id).recovery_diagnostic.is_some());
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
        let recovered = recover(&core, &ws, &id);
        assert_eq!(recovered.state, crate::jobs::JobState::Queued);
        assert_eq!(recovered.log, "Export recovery requested explicitly");
        assert!(recovered.recovery_diagnostic.is_none());
        // The diagnostic is gone on the next poll too, and the
        // job completes through the ordinary claim path.
        assert!(listed_job(&core, &ws, &id).recovery_diagnostic.is_none());
        let job = wait_for_terminal(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Completed);
    }

    #[test]
    fn recover_action_is_version_fenced_and_state_bounded() {
        let (_temp, core, ws) = setup();
        let bytes = b"Fenced recovery.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "fenced.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        // Wrong version conflicts.
        let stale = core.job_command(
            "control",
            json!({"contractVersion":1,"workspaceId":ws,"id":id,"expectedVersion":999,"action":"recover"}),
        );
        assert!(stale.is_err());
        // Recover on a Paused job is refused.
        let job = read_job(&core, &ws, &id);
        assert_eq!(job.state, crate::jobs::JobState::Paused);
        let refused = core.job_command(
            "control",
            json!({"contractVersion":1,"workspaceId":ws,"id":id,"expectedVersion":job.version,"action":"recover"}),
        );
        assert!(refused.is_err());
        // (Recover on a healthy Queued job is exercised through
        // the exhausted-root path in
        // `recover_action_recovers_and_clears_diagnostic`: a
        // Queued job on a healthy root is served immediately,
        // so no deterministic healthy-Queued state exists to
        // call Recover on. Recover only clears a possibly
        // absent exhaustion record and wakes deliberately —
        // both no-ops on a healthy root.)
    }

    #[test]
    fn cancel_of_diagnosed_job_stays_cancelled_with_nothing_published() {
        let (_temp, core, ws) = setup();
        let bytes = b"Cancel the diagnosed job.";
        let sha = content::hash(bytes);
        let opened = begin(&core, &ws, "cancel-diagnosed.txt", bytes, &sha, "private");
        let id = opened["id"].as_str().unwrap().to_string();
        let root = root_of(&core);
        crate::jobs::arm_claim_faults(&root, 1_000_000, crate::ErrorCode::StorageUnavailable);
        part(&core, &ws, &id, 0, bytes, true).unwrap();
        wait_for_parked(&core, &ws, &id, &root);
        assert!(listed_job(&core, &ws, &id).recovery_diagnostic.is_some());
        let job = read_job(&core, &ws, &id);
        core.job_command(
            "control",
            json!({"contractVersion":1,"workspaceId":ws,"id":id,"expectedVersion":job.version,"action":"cancel"}),
        )
        .unwrap();
        // Terminal jobs carry no diagnostic.
        let listed = listed_job(&core, &ws, &id);
        assert_eq!(listed.state, crate::jobs::JobState::Cancelled);
        assert!(listed.recovery_diagnostic.is_none());
        // Clearing the fault and deliberately re-arming must
        // never publish the cancelled job.
        crate::jobs::clear_claim_faults(&root);
        crate::jobs::clear_claim_parked(&root);
        core.wake_export_worker(crate::jobs::WakeKind::Deliberate);
        // A fence proves the worker ran after the deliberate
        // wake: the cancelled job is still unreachable.
        let fence_bytes = b"Fence after cancel.";
        let fence_sha = content::hash(fence_bytes);
        let (_fence_temp, fence_core, fence_ws) = setup();
        let fence_opened = begin(
            &fence_core,
            &fence_ws,
            "fence.txt",
            fence_bytes,
            &fence_sha,
            "private",
        );
        let fence_id = fence_opened["id"].as_str().unwrap().to_string();
        part(&fence_core, &fence_ws, &fence_id, 0, fence_bytes, true).unwrap();
        assert_eq!(
            wait_for_terminal(&fence_core, &fence_ws, &fence_id).state,
            crate::jobs::JobState::Completed,
        );
        assert_eq!(
            read_job(&core, &ws, &id).state,
            crate::jobs::JobState::Cancelled
        );
        assert_eq!(
            list_documents(&core, &ws)["memories"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }
}
