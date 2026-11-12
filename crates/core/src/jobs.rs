//! Durable export job state and lease fencing. Payloads and logs never contain source text.
use crate::{
    CommandError, ErrorCode, content,
    document::DocumentSource,
    privacy::Privacy,
    storage::{Storage, unavailable},
    workspace::validate_id,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const MAX_RECORDS: u32 = 10_000;
pub const MAX_BYTES: u64 = 256 * 1024 * 1024;
pub const LEASE_SECONDS: i64 = 120;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ExportSelection {
    Workspace,
    Backup {
        application: bool,
    },
    Project {
        id: String,
    },
    Records {
        ids: Vec<String>,
    },
    /// Local document import (M13 Slice 1). The durable job-spec enum
    /// already carries a `Backup` variant; `Ingestion` is the same idea:
    /// the job row durably records the staged upload's identity, so no
    /// second staging table is needed. `upload_id` is the job id itself.
    #[serde(rename_all = "camelCase")]
    Ingestion {
        upload_id: String,
        file_name: String,
        format: String,
        expected_bytes: u64,
        expected_hash: String,
        privacy: Privacy,
        project_id: Option<String>,
    },
}
impl ExportSelection {
    pub(crate) fn validate(&self) -> Result<(), CommandError> {
        match self {
            Self::Workspace | Self::Backup { .. } => Ok(()),
            Self::Ingestion {
                upload_id,
                file_name,
                format,
                expected_bytes,
                expected_hash,
                privacy,
                project_id,
            } => {
                validate_id(upload_id)?;
                if file_name.chars().count() == 0
                    || file_name.chars().count() > crate::document::MAX_LABEL_CHARS
                    || file_name.contains(['/', '\\', '\0'])
                {
                    return Err(invalid());
                }
                // The kind must be the one the file-name extension names;
                // a forged selection cannot smuggle another kind past the
                // worker, which re-validates the staged bytes anyway.
                if DocumentSource::kind_for_label(file_name).map_err(|_| invalid())? != *format {
                    return Err(invalid());
                }
                if *expected_bytes == 0
                    || *expected_bytes > crate::document::max_bytes_for_kind(format)
                    || expected_hash.len() != 64
                    || !expected_hash
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                {
                    return Err(invalid());
                }
                if *privacy == Privacy::Sealed {
                    return Err(invalid());
                }
                if let Some(pid) = project_id {
                    validate_id(pid)?;
                }
                Ok(())
            }
            Self::Project { id } => validate_id(id),
            Self::Records { ids } => {
                if ids.is_empty() || ids.len() > MAX_RECORDS as usize {
                    return Err(invalid());
                }
                let mut seen = std::collections::HashSet::new();
                for id in ids {
                    validate_id(id)?;
                    if !seen.insert(id) {
                        return Err(invalid());
                    }
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Job {
    pub id: String,
    pub workspace_id: String,
    pub state: JobState,
    pub version: u32,
    pub checkpoint_version: u32,
    pub checkpoint: String,
    pub selection: ExportSelection,
    pub progress: u32,
    pub max_records: u32,
    pub max_bytes: u64,
    pub artifact_id: Option<String>,
    pub artifact_bytes: Option<u64>,
    pub artifact_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub log: String,
    /// Present only for Queued jobs whose storage root exhausted
    /// claim recovery. Computed at read time from in-memory
    /// worker state — never stored on the durable job row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_diagnostic: Option<JobRecoveryDiagnostic>,
}

/// Why claim recovery was exhausted for a storage root. Bounded
/// by construction: only the two failure kinds `claim_export`
/// can produce. The diagnostic message is fixed per reason and
/// carries no filesystem detail.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum JobRecoveryReason {
    StorageUnavailable,
    CorruptStorage,
}

impl JobRecoveryReason {
    fn from_claim_error(code: &ErrorCode) -> Self {
        match code {
            ErrorCode::StorageUnavailable => Self::StorageUnavailable,
            // The only other code `claim_export` can produce is
            // `CorruptStorage` from `connection()`; anything else
            // is unreachable in practice and is treated the same
            // conservative way: user action required, no retry.
            _ => Self::CorruptStorage,
        }
    }

    /// Fixed sanitized instruction. Never interpolates paths,
    /// SQLite text, or record contents.
    fn message(&self) -> &'static str {
        match self {
            Self::StorageUnavailable => {
                "Recovery stopped because storage was unavailable. Check the storage folder and permissions, then choose Retry recovery."
            }
            Self::CorruptStorage => {
                "Recovery stopped because storage could not be read. Keep the original files and restore a verified backup, then choose Retry recovery."
            }
        }
    }
}

/// Actionable production diagnostic for a Queued job stranded by
/// exhausted claim recovery. Attached at read time by `list_jobs`
/// and `control_job` when the job's storage root is exhausted.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobRecoveryDiagnostic {
    pub reason: JobRecoveryReason,
    /// Claim attempts consumed in the arming that exhausted
    /// recovery (3 for unavailable storage, 1 for corrupt).
    pub attempts: u32,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum JobAction {
    Pause,
    Resume,
    Cancel,
    Retry,
    /// Deliberate re-arm of an exhausted root for a Queued job.
    /// Valid from Queued only; clears the root's exhaustion and
    /// wakes the worker. Harmless on a healthy root.
    Recover,
}

/// Whether a worker wake-up is a deliberate user action (new
/// work, an explicit control, startup) or passive status
/// observation (polling). Only deliberate wakes re-arm an
/// exhausted root; passive wakes never reset a budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeKind {
    Deliberate,
    Passive,
}

pub(crate) fn invalid() -> CommandError {
    CommandError::new(
        ErrorCode::InvalidRequest,
        "Use a valid job selection or state transition.",
    )
}
pub fn request_error() -> CommandError {
    invalid()
}
pub(crate) fn conflict() -> CommandError {
    CommandError::new(
        ErrorCode::RevisionConflict,
        "The job changed or its lease expired. Refresh before retrying.",
    )
}
fn missing() -> CommandError {
    CommandError::new(ErrorCode::NotFound, "Job not found in this workspace.")
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
fn state_name(state: &JobState) -> &'static str {
    match state {
        JobState::Queued => "Queued",
        JobState::Running => "Running",
        JobState::Paused => "Paused",
        JobState::Completed => "Completed",
        JobState::Failed => "Failed",
        JobState::Cancelled => "Cancelled",
    }
}

pub(crate) fn read(
    connection: &Connection,
    workspace: &str,
    id: &str,
) -> Result<Job, CommandError> {
    type JobRow = (
        String,
        u32,
        u32,
        String,
        String,
        u32,
        Option<String>,
        Option<i64>,
        Option<String>,
        String,
        String,
        String,
    );
    let data: Option<JobRow> = connection.query_row(
        "SELECT state,version,checkpoint_version,checkpoint,selection,progress,artifact_id,artifact_bytes,artifact_hash,created_at,updated_at,log FROM jobs WHERE workspace_id=?1 AND id=?2",
        params![workspace,id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?))
    ).optional().map_err(|_| unavailable())?;
    let (
        state,
        version,
        checkpoint_version,
        checkpoint,
        selection,
        progress,
        artifact_id,
        artifact_bytes,
        artifact_hash,
        created_at,
        updated_at,
        log,
    ) = data.ok_or_else(missing)?;
    Ok(Job {
        id: id.into(),
        workspace_id: workspace.into(),
        state: serde_json::from_value(serde_json::Value::String(state))
            .map_err(|_| unavailable())?,
        version,
        checkpoint_version,
        checkpoint,
        selection: serde_json::from_str(&selection).map_err(|_| unavailable())?,
        progress,
        max_records: MAX_RECORDS,
        max_bytes: MAX_BYTES,
        artifact_id,
        artifact_bytes: artifact_bytes.and_then(|n| u64::try_from(n).ok()),
        artifact_hash,
        created_at,
        updated_at,
        log,
        // The diagnostic is attached at the API boundary
        // (`list_jobs` / `control_job`), never from the row.
        recovery_diagnostic: None,
    })
}

/// A worker identity is generated by trusted Rust code and never returned to clients.
pub struct JobLease {
    pub job: Job,
    pub(crate) owner: String,
}

/// Best-effort removal of one ingestion job's staged upload file.
/// Never fails the caller; the upload-begin sweep removes leftovers.
fn discard_ingestion_staging(storage: &Storage, workspace: &str, selection: &ExportSelection) {
    if let ExportSelection::Ingestion { upload_id, .. } = selection
        && let Ok(directory) = crate::export::directory(storage, workspace)
    {
        let path = directory.join(format!("{upload_id}.upload"));
        let _ = crate::storage::reject_links(&path).map(|_| std::fs::remove_file(&path));
    }
}

impl Storage {
    pub fn enqueue_export(
        &self,
        workspace: &str,
        selection: ExportSelection,
    ) -> Result<Job, CommandError> {
        validate_id(workspace)?;
        selection.validate()?;
        self.open(workspace)?;
        let mut connection = self.connection(false)?.ok_or_else(missing)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        // Bound durable queued work, rather than allowing a client to exhaust the catalog.
        let count: u32 = tx.query_row("SELECT count(*) FROM jobs WHERE workspace_id=?1 AND state IN ('Queued','Running','Paused')",[workspace],|r|r.get(0)).map_err(|_| unavailable())?;
        if count >= 32 {
            return Err(invalid());
        }
        let id = content::random_id()?;
        let stamp = now();
        tx.execute("INSERT INTO jobs(workspace_id,id,state,version,checkpoint_version,checkpoint,selection,created_at,updated_at,log) VALUES(?1,?2,'Queued',1,1,'queued',?3,?4,?4,'Export queued')",params![workspace,id,serde_json::to_string(&selection).map_err(|_| invalid())?,stamp]).map_err(|_| unavailable())?;
        let job = read(&tx, workspace, &id)?;
        tx.commit().map_err(|_| unavailable())?;
        Ok(job)
    }

    pub fn recover_jobs(&self) -> Result<u32, CommandError> {
        let Some(connection) = self.connection(false)? else {
            return Ok(0);
        };
        connection.execute("UPDATE jobs SET state='Paused',owner=NULL,lease_until=NULL,version=version+1,updated_at=?1,log='Worker interrupted; resume explicitly' WHERE state='Running' AND lease_until<=?2",params![now(),Utc::now().timestamp()]).map(|n| n as u32).map_err(|_| unavailable())
    }

    pub fn read_job(&self, workspace: &str, id: &str) -> Result<Job, CommandError> {
        validate_id(workspace)?;
        validate_id(id)?;
        let connection = self.connection(false)?.ok_or_else(missing)?;
        read(&connection, workspace, id)
    }

    /// Read a job WITHOUT opening the catalog through the
    /// migrating connection. Verification paths must observe
    /// the catalog exactly as it is: a lookup that silently
    /// migrates would rewrite the durable schema it only
    /// means to inspect — bumping a legacy schema-8 catalog
    /// to 10 — and would leave that side effect behind even
    /// when verification then fails. The vault seal depends
    /// on this: its schema upgrade is explicit and
    /// transactional, so it must see the true starting
    /// version, and a pre-switch failure must leave the
    /// catalog untouched.
    pub(crate) fn read_job_readonly(&self, workspace: &str, id: &str) -> Result<Job, CommandError> {
        validate_id(workspace)?;
        validate_id(id)?;
        let connection = Connection::open_with_flags(
            self.root.join(crate::storage::DATABASE_NAME),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|_| missing())?;
        read(&connection, workspace, id)
    }

    /// Attach the recovery diagnostic to a Queued job when this
    /// storage root exhausted claim recovery. Read-time only: the
    /// durable row is never written, and only this root's own
    /// exhaustion is ever exposed.
    fn attach_recovery_diagnostic(&self, job: &mut Job) {
        if job.state != JobState::Queued {
            return;
        }
        let record = {
            let queue = PROCESS_WORKER
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            queue.exhaustion_for(&self.root)
        };
        if let Some(record) = record {
            job.recovery_diagnostic = Some(JobRecoveryDiagnostic {
                reason: record.reason,
                attempts: record.attempts,
                message: record.reason.message().to_string(),
            });
        }
    }

    /// Newest first, bounded page; state changes always require the observed version.
    pub fn list_jobs(&self, workspace: &str, offset: u32) -> Result<Vec<Job>, CommandError> {
        validate_id(workspace)?;
        self.open(workspace)?;
        self.recover_jobs()?;
        let connection = self.connection(false)?.ok_or_else(missing)?;
        let mut statement = connection.prepare("SELECT id FROM jobs WHERE workspace_id=?1 ORDER BY created_at DESC,id DESC LIMIT 50 OFFSET ?2").map_err(|_| unavailable())?;
        let ids = statement
            .query_map(params![workspace, offset], |r| r.get::<_, String>(0))
            .map_err(|_| unavailable())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| unavailable())?;
        ids.iter()
            .map(|id| {
                let mut job = read(&connection, workspace, id)?;
                self.attach_recovery_diagnostic(&mut job);
                Ok(job)
            })
            .collect()
    }

    pub fn control_job(
        &self,
        workspace: &str,
        id: &str,
        expected: u32,
        action: JobAction,
    ) -> Result<Job, CommandError> {
        validate_id(workspace)?;
        validate_id(id)?;
        self.recover_jobs()?;
        let mut connection = self.connection(false)?.ok_or_else(missing)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        let job = read(&tx, workspace, id)?;
        if expected != job.version {
            return Err(conflict());
        }
        let next = match (action, &job.state) {
            (JobAction::Pause, JobState::Queued | JobState::Running) => JobState::Paused,
            (JobAction::Resume, JobState::Paused) => JobState::Queued,
            (JobAction::Cancel, JobState::Queued | JobState::Running | JobState::Paused) => {
                JobState::Cancelled
            }
            (JobAction::Retry, JobState::Failed | JobState::Cancelled) => JobState::Queued,
            (JobAction::Recover, JobState::Queued) => JobState::Queued,
            _ => return Err(invalid()),
        };
        // An ingestion job whose upload has not finished cannot be queued:
        // the worker would find no staged bytes. Resume waits until the
        // upload completes (checkpoint `staged`); the Documents UI offers
        // "Resume upload" for the in-flight case instead.
        if matches!(action, JobAction::Resume)
            && matches!(job.selection, ExportSelection::Ingestion { .. })
            && job.checkpoint != "staged"
        {
            return Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "This import's upload is incomplete; resume the upload instead of the job.",
            ));
        }
        let log = match action {
            JobAction::Recover => "Export recovery requested explicitly",
            _ => match next {
                JobState::Queued => "Export queued explicitly",
                JobState::Paused => "Export paused explicitly",
                _ => "Export cancelled explicitly",
            },
        };
        tx.execute("UPDATE jobs SET state=?1,version=version+1,owner=NULL,lease_until=NULL,updated_at=?2,log=?3 WHERE workspace_id=?4 AND id=?5",params![state_name(&next),now(),log,workspace,id]).map_err(|_| unavailable())?;
        let mut result = read(&tx, workspace, id)?;
        tx.commit().map_err(|_| unavailable())?;
        // A successful deliberate action that queues work clears
        // the root's exhaustion, so the response below — and the
        // following deliberate wake — see the re-armed state.
        // Failed controls clear nothing. Pause/Cancel never clear.
        if matches!(
            action,
            JobAction::Retry | JobAction::Resume | JobAction::Recover
        ) {
            let mut queue = PROCESS_WORKER
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            queue.clear_exhausted(&self.root);
        }
        self.attach_recovery_diagnostic(&mut result);
        // Cancelling an import abandons its staged bytes. Best effort and
        // after the commit: a failed delete never blocks the cancellation,
        // and the upload-begin sweep removes any leftover later.
        if matches!(next, JobState::Cancelled) {
            discard_ingestion_staging(self, workspace, &job.selection);
        }
        Ok(result)
    }

    pub fn claim_export(&self) -> Result<Option<JobLease>, CommandError> {
        self.recover_jobs()?;
        let Some(mut connection) = self.connection(false)? else {
            return Ok(None);
        };
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        let selected: Option<(String,String)> = tx.query_row("SELECT workspace_id,id FROM jobs WHERE state='Queued' ORDER BY created_at,id LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_| unavailable())?;
        let Some((workspace, id)) = selected else {
            return Ok(None);
        };
        let owner = content::random_id()?;
        // Ingestion jobs keep their checkpoint and progress: the
        // checkpoint is the upload's durable state (`staged` proves the
        // upload finished, so an interrupted job can Resume), and
        // progress is the upload percent. Export jobs reset both to
        // the snapshot scratch values.
        tx.execute("UPDATE jobs SET state='Running',version=version+1,checkpoint_version=checkpoint_version+1,checkpoint=CASE WHEN json_extract(selection,'$.kind')='ingestion' THEN checkpoint ELSE 'snapshot' END,progress=CASE WHEN json_extract(selection,'$.kind')='ingestion' THEN progress ELSE 0 END,owner=?1,lease_until=?2,updated_at=?3,log=CASE WHEN json_extract(selection,'$.kind')='ingestion' THEN 'Document import started' ELSE 'Export snapshot started' END WHERE workspace_id=?4 AND id=?5",params![owner,Utc::now().timestamp()+LEASE_SECONDS,now(),workspace,id]).map_err(|_| unavailable())?;
        let job = read(&tx, &workspace, &id)?;
        tx.commit().map_err(|_| unavailable())?;
        Ok(Some(JobLease { job, owner }))
    }

    pub fn checkpoint_export(&self, lease: &JobLease, progress: u32) -> Result<(), CommandError> {
        if progress > MAX_RECORDS {
            return Err(invalid());
        }
        let connection = self.connection(false)?.ok_or_else(missing)?;
        let time = Utc::now().timestamp();
        let changed = connection.execute("UPDATE jobs SET progress=?1,checkpoint_version=checkpoint_version+1,checkpoint='snapshot',lease_until=?2,updated_at=?3 WHERE workspace_id=?4 AND id=?5 AND state='Running' AND owner=?6 AND lease_until>?7",params![progress,time+LEASE_SECONDS,now(),lease.job.workspace_id,lease.job.id,lease.owner,time]).map_err(|_| unavailable())?;
        if changed != 1 {
            return Err(conflict());
        }
        Ok(())
    }

    /// Fixed safe failure category only. Never serialize filesystem errors or record contents.
    pub fn fail_export(&self, lease: &JobLease) -> Result<(), CommandError> {
        let connection = self.connection(false)?.ok_or_else(missing)?;
        let changed = connection.execute("UPDATE jobs SET state='Failed',version=version+1,checkpoint_version=checkpoint_version+1,checkpoint='failed',owner=NULL,lease_until=NULL,updated_at=?1,log='Export failed; verify limits, content integrity and private storage before retry' WHERE workspace_id=?2 AND id=?3 AND state='Running' AND owner=?4 AND lease_until>?5",params![now(),lease.job.workspace_id,lease.job.id,lease.owner,Utc::now().timestamp()]).map_err(|_| unavailable())?;
        if changed != 1 {
            return Err(conflict());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnqueueExportRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub selection: ExportSelection,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListJobsRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    #[serde(default)]
    pub offset: u32,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ControlJobRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: String,
    pub expected_version: u32,
    pub action: JobAction,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadArtifactRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub job_id: String,
    pub artifact_id: String,
    pub offset: u32,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobDetails {
    pub contract_version: u8,
    pub job: Job,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobList {
    pub contract_version: u8,
    pub jobs: Vec<Job>,
    pub next_offset: Option<u32>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactChunk {
    pub contract_version: u8,
    pub bytes: Vec<u8>,
}

#[derive(Default)]
struct WorkerQueue {
    running: bool,
    /// Storages are served in wake order (FIFO): a storage woken
    /// early must not be starved by storages woken later. The
    /// previous Vec+pop served LIFO, which stalled an early
    /// ingestion job past a fixed poll window on a contended
    /// broad run.
    pending: VecDeque<PendingRoot>,
    /// Roots whose claim recovery is exhausted, with the reason
    /// and attempts consumed. Passive wakes never touch this
    /// record; only a deliberate wake or a successful queueing
    /// control clears it. In-memory only: a restart recovers.
    exhausted: Vec<ExhaustedRoot>,
    /// Roots the worker loop currently holds outside the pending
    /// queue. A passive wake of an active root must not insert
    /// a fresh zero-count entry: the in-flight entry's requeue
    /// would drop its failure count in favor of the fresh one.
    /// Marked by `take_next`, cleared by `finish`, `requeue`,
    /// and parking — every exit from serving a root.
    active: Vec<std::path::PathBuf>,
}

/// A parked root's production exhaustion record: the reason
/// recovery stopped and the attempts consumed in the arming
/// that exhausted it. This is the state behind the
/// `recoveryDiagnostic` on Queued jobs — deliberately separate
/// from the test-only parked signal.
#[derive(Debug, Clone)]
struct ExhaustedRoot {
    root: std::path::PathBuf,
    reason: JobRecoveryReason,
    attempts: u32,
}

/// Consecutive `StorageUnavailable` claim failures tolerated per
/// arming before the worker parks the root. Each attempt already
/// runs through SQLite with a 5 s busy timeout, so three attempts
/// span real contention; more would only delay parking a
/// persistently sick root.
const MAX_CLAIM_FAILURES: u32 = 3;
/// Kindness delay before re-claiming a root that already failed
/// a claim. The attempt bound — not this delay — is what
/// prevents a busy loop; the delay only spares a sick store
/// from immediate re-probing.
const CLAIM_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(100);

struct PendingRoot {
    storage: Storage,
    claim_failures: u32,
}

impl WorkerQueue {
    /// Queue a storage root for the worker, deduplicated by root.
    /// A deliberate wake clears any exhaustion record and resets
    /// a pending root's failure count: a fresh user action earns
    /// a fresh budget. A deliberate wake of an active root adds
    /// a fresh entry, served after the current attempt; the
    /// stale requeue/park guards protect that newer decision.
    /// A passive wake (status polling) never touches an
    /// exhausted or active root and never resets a pending
    /// root's count — observation must not re-arm recovery or
    /// discard an in-flight budget.
    /// Returns true when the caller must start the worker thread.
    fn wake(&mut self, storage: Storage, kind: WakeKind) -> bool {
        match kind {
            WakeKind::Deliberate => {
                #[cfg(test)]
                record_worker_state(&storage.root, |state| {
                    state.pending = true;
                    state.ever_registered = true;
                });
                self.clear_exhausted(&storage.root);
                if let Some(pending) = self
                    .pending
                    .iter_mut()
                    .find(|pending| pending.storage.root == storage.root)
                {
                    pending.claim_failures = 0;
                } else {
                    self.pending.push_back(PendingRoot {
                        storage,
                        claim_failures: 0,
                    });
                }
            }
            WakeKind::Passive => {
                if self
                    .exhausted
                    .iter()
                    .any(|entry| entry.root == storage.root)
                    || self.active.iter().any(|root| *root == storage.root)
                {
                    return false;
                }
                if !self
                    .pending
                    .iter()
                    .any(|pending| pending.storage.root == storage.root)
                {
                    #[cfg(test)]
                    record_worker_state(&storage.root, |state| {
                        state.pending = true;
                        state.ever_registered = true;
                    });
                    self.pending.push_back(PendingRoot {
                        storage,
                        claim_failures: 0,
                    });
                }
            }
        }
        if self.running {
            return false;
        }
        self.running = true;
        true
    }

    /// Record that a root's claim recovery is exhausted. Replaces
    /// any earlier record for the root. Returns false — marking
    /// nothing — when a pending entry exists for the root: that
    /// entry can only come from a deliberate wake that arrived
    /// while the root was active, and the stale in-flight
    /// failure must not override the newer deliberate recovery.
    /// The fresh entry is then served with its own budget, and
    /// its own attempt decides the outcome.
    fn mark_exhausted(
        &mut self,
        root: std::path::PathBuf,
        reason: JobRecoveryReason,
        attempts: u32,
    ) -> bool {
        if self.pending.iter().any(|entry| entry.storage.root == root) {
            return false;
        }
        self.exhausted.retain(|entry| entry.root != root);
        self.exhausted.push(ExhaustedRoot {
            root,
            reason,
            attempts,
        });
        true
    }

    /// Clear a root's exhaustion record (deliberate re-arm).
    fn clear_exhausted(&mut self, root: &std::path::Path) {
        self.exhausted.retain(|entry| entry.root.as_path() != root);
    }

    /// A copy of a root's exhaustion record, if any. The
    /// diagnostic attach path reads through this.
    fn exhaustion_for(&self, root: &std::path::Path) -> Option<ExhaustedRoot> {
        self.exhausted
            .iter()
            .find(|entry| entry.root.as_path() == root)
            .cloned()
    }

    /// Take the next storage to serve, marking it active so
    /// passive wakes cannot insert a competing fresh entry while
    /// the worker holds it. When the queue is empty the worker
    /// is marked idle and None is returned, so the caller stops
    /// the thread. Taking work and deciding to stop share the
    /// caller's lock hold, so a wake-up committed during
    /// shutdown cannot be lost.
    fn take_next(&mut self) -> Option<PendingRoot> {
        #[cfg(test)]
        let taken = {
            // Gated roots stay pending: serve the first
            // ungated entry, leaving gated entries in place
            // and in order. Never blocks: a pure scan under
            // the lock hold. When every entry is gated the
            // worker goes idle, exactly as for an empty
            // queue; the gate release re-wakes it.
            match self
                .pending
                .iter()
                .position(|entry| !admissions_gated(&entry.storage.root))
            {
                Some(index) => self.pending.remove(index),
                None => None,
            }
        };
        #[cfg(not(test))]
        let taken = self.pending.pop_front();
        match taken {
            Some(pending) => {
                #[cfg(test)]
                record_worker_state(&pending.storage.root, |state| {
                    state.pending = false;
                    state.active = true;
                });
                if !self.active.iter().any(|root| *root == pending.storage.root) {
                    self.active.push(pending.storage.root.clone());
                }
                Some(pending)
            }
            None => {
                self.running = false;
                None
            }
        }
    }

    /// The worker stopped serving a root without requeueing it
    /// (drained: no more jobs to claim). Clear its active mark
    /// so later wakes see it as idle again.
    fn finish(&mut self, root: &std::path::Path) {
        #[cfg(test)]
        record_worker_state(root, |state| {
            state.active = false;
        });
        self.active.retain(|entry| entry.as_path() != root);
    }

    /// Re-queue a root whose claim failed, at the back, so other
    /// roots are served before the retry. The root leaves the
    /// active set: it is pending again (or dropped, when a fresh
    /// wake already re-pended the root while it was being served
    /// — the pending entry, fresh budget, wins and the stale
    /// in-flight count is discarded by design).
    fn requeue(&mut self, pending: PendingRoot) {
        #[cfg(test)]
        record_worker_state(&pending.storage.root, |state| {
            state.pending = true;
        });
        self.finish(&pending.storage.root.clone());
        if !self
            .pending
            .iter()
            .any(|entry| entry.storage.root == pending.storage.root)
        {
            self.pending.push_back(pending);
        }
    }
}

static PROCESS_WORKER: std::sync::Mutex<WorkerQueue> = std::sync::Mutex::new(WorkerQueue {
    running: false,
    pending: VecDeque::new(),
    exhausted: Vec::new(),
    active: Vec::new(),
});

/// Forced claim failures for fault-injection tests, keyed by
/// storage-root path: remaining failures + the error code to
/// return. Test builds only; compiled out of shipped builds.
#[cfg(test)]
static CLAIM_FAULTS: std::sync::Mutex<Vec<(std::path::PathBuf, u32, ErrorCode)>> =
    std::sync::Mutex::new(Vec::new());
/// Roots the worker parked after a claim failure (test builds
/// only). Lets tests wait on the parking condition instead of
/// polling process-global worker state, which parallel tests
/// would make flaky.
#[cfg(test)]
static CLAIM_PARKED: std::sync::Mutex<Vec<std::path::PathBuf>> = std::sync::Mutex::new(Vec::new());
/// Per-root worker admission counts (test builds only): how
/// many times the worker loop took this root from the queue.
/// Lets parking waits distinguish "never admitted" (queueing
/// delay behind other fixtures) from "admitted and attempted
/// but never parked" (a backend defect).
#[cfg(test)]
static CLAIM_ADMISSIONS: std::sync::Mutex<Vec<(std::path::PathBuf, u32)>> =
    std::sync::Mutex::new(Vec::new());
/// Per-root claim attempt counts (test builds only): how many
/// times the worker loop called `claim_for_worker` for this
/// root, whether the claim hit the fault table or the store.
#[cfg(test)]
static CLAIM_ATTEMPTS: std::sync::Mutex<Vec<(std::path::PathBuf, u32)>> =
    std::sync::Mutex::new(Vec::new());

#[cfg(test)]
fn bump_counter(table: &std::sync::Mutex<Vec<(std::path::PathBuf, u32)>>, root: &std::path::Path) {
    let mut table = table.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(entry) = table.iter_mut().find(|(entry, _)| entry.as_path() == root) {
        entry.1 += 1;
    } else {
        table.push((root.to_path_buf(), 1));
    }
}

#[cfg(test)]
fn read_counter(
    table: &std::sync::Mutex<Vec<(std::path::PathBuf, u32)>>,
    root: &std::path::Path,
) -> u32 {
    table
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .find(|(entry, _)| entry.as_path() == root)
        .map(|(_, count)| *count)
        .unwrap_or(0)
}

/// How many times the worker loop admitted this root (test
/// builds only).
#[cfg(test)]
pub(crate) fn claim_admissions(root: &std::path::Path) -> u32 {
    read_counter(&CLAIM_ADMISSIONS, root)
}

/// How many times the worker loop attempted a claim for this
/// root (test builds only).
#[cfg(test)]
pub(crate) fn claim_attempts(root: &std::path::Path) -> u32 {
    read_counter(&CLAIM_ATTEMPTS, root)
}

/// Per-root worker queue state observed by parking waits (test
/// builds only): whether the root currently has a pending
/// queue entry, whether the worker currently holds it, and
/// whether it was ever registered. Lets the wait distinguish
/// registered pending work (legitimate queueing behind other
/// fixtures) from a root that was never scheduled and from an
/// admitted worker that stops progressing. Updated by the
/// queue methods themselves, so the mirror tracks exactly the
/// entries the worker serves — including a deliberate wake of
/// an active root (pending AND active at once) and requeues
/// (active back to pending). The mirror mutex is only ever
/// taken while holding the queue mutex or alone, and never
/// takes the queue mutex, so no lock order can invert.
#[cfg(test)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct WorkerTestState {
    pub pending: bool,
    pub active: bool,
    pub ever_registered: bool,
}

/// Per-root worker queue states (test builds only).
#[cfg(test)]
static WORKER_STATE: std::sync::Mutex<Vec<(std::path::PathBuf, WorkerTestState)>> =
    std::sync::Mutex::new(Vec::new());

#[cfg(test)]
fn record_worker_state(root: &std::path::Path, update: impl FnOnce(&mut WorkerTestState)) {
    let mut table = WORKER_STATE
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(entry) = table.iter_mut().find(|(entry, _)| entry.as_path() == root) {
        update(&mut entry.1);
    } else {
        let mut state = WorkerTestState::default();
        update(&mut state);
        table.push((root.to_path_buf(), state));
    }
}

/// The worker queue state observed for one root (test builds
/// only). A root no queue method has ever recorded reads as
/// never registered.
#[cfg(test)]
pub(crate) fn worker_test_state(root: &std::path::Path) -> WorkerTestState {
    WORKER_STATE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .find(|(entry, _)| entry.as_path() == root)
        .map(|(_, state)| *state)
        .unwrap_or_default()
}

/// Roots whose admission is held by a test-only gate (test
/// builds only). A gated root keeps its pending queue entry
/// but is never taken by the worker, so a test can start an
/// observer against a root that is provably pending with
/// zero admissions, then release the gate and watch it
/// complete. Never consulted in shipped builds.
#[cfg(test)]
static ADMISSION_GATES: std::sync::Mutex<Vec<std::path::PathBuf>> =
    std::sync::Mutex::new(Vec::new());

/// Hold a root pending: the worker will not admit it until
/// released (test builds only). Idempotent.
#[cfg(test)]
pub(crate) fn gate_admissions(root: &std::path::Path) {
    let mut gates = ADMISSION_GATES
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if !gates.iter().any(|entry| entry.as_path() == root) {
        gates.push(root.to_path_buf());
    }
}

/// Release a held root (test builds only). Idempotent; the
/// caller re-wakes the root so an idle worker restarts.
#[cfg(test)]
pub(crate) fn release_admissions(root: &std::path::Path) {
    ADMISSION_GATES
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|entry| entry.as_path() != root);
}

/// Whether a root's admission is held (test builds only).
/// A plain mutex scan: never blocks, so it is safe to call
/// while holding the queue mutex.
#[cfg(test)]
fn admissions_gated(root: &std::path::Path) -> bool {
    ADMISSION_GATES
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .any(|entry| entry.as_path() == root)
}

/// Arm forced claim failures for one storage root (test builds
/// only). Each worker claim for that root consumes one failure
/// and returns the armed code instead of touching the store.
/// Scoped by root so parallel tests cannot interfere.
#[cfg(test)]
pub(crate) fn arm_claim_faults(root: &std::path::Path, failures: u32, code: ErrorCode) {
    let mut faults = CLAIM_FAULTS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(entry) = faults
        .iter_mut()
        .find(|(entry, _, _)| entry.as_path() == root)
    {
        entry.1 = failures;
        entry.2 = code;
    } else {
        faults.push((root.to_path_buf(), failures, code));
    }
}

#[cfg(test)]
pub(crate) fn remaining_claim_faults(root: &std::path::Path) -> u32 {
    CLAIM_FAULTS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .find(|(entry, _, _)| entry.as_path() == root)
        .map(|(_, remaining, _)| *remaining)
        .unwrap_or(0)
}

#[cfg(test)]
pub(crate) fn clear_claim_faults(root: &std::path::Path) {
    CLAIM_FAULTS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|(entry, _, _)| entry.as_path() != root);
}

#[cfg(test)]
pub(crate) fn claim_parked(root: &std::path::Path) -> bool {
    CLAIM_PARKED
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .any(|entry| entry.as_path() == root)
}

#[cfg(test)]
pub(crate) fn clear_claim_parked(root: &std::path::Path) {
    CLAIM_PARKED
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|entry| entry.as_path() != root);
}

/// Record that the worker parked a root (test builds only; a
/// no-op in shipped builds, where the observable state is the
/// jobs left Queued and the next user wake re-arming the root).
#[cfg(test)]
fn record_claim_parked(storage: &Storage) {
    let mut parked = CLAIM_PARKED
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if !parked.iter().any(|entry| *entry == storage.root) {
        parked.push(storage.root.clone());
    }
}
#[cfg(not(test))]
fn record_claim_parked(_storage: &Storage) {}

/// Park a root: clear its active mark and record its
/// production exhaustion (reason + attempts) under the queue
/// lock, and record the test-only parked signal — but only
/// when exhaustion was actually marked. A deliberate wake that
/// arrived while the root was active leaves a fresh pending
/// entry, and the stale in-flight failure must not override
/// that newer recovery decision: the root is not parked, no
/// signal is recorded, and the fresh entry is served next.
/// The two records are deliberately separate: tests may clear
/// the signal without re-arming the root.
fn park_root(storage: &Storage, reason: JobRecoveryReason, attempts: u32) {
    let marked = {
        let mut queue = PROCESS_WORKER
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        queue.finish(&storage.root);
        queue.mark_exhausted(storage.root.clone(), reason, attempts)
    };
    if marked {
        record_claim_parked(storage);
    }
}

/// The worker loop's claim call. In test builds a fault table can
/// force failures for one storage root, so recovery behavior is
/// exercised deterministically without filesystem races. In
/// shipped builds this delegates directly to `claim_export`.
/// The attempt counter records every call in test builds.
fn claim_for_worker(storage: &Storage) -> Result<Option<JobLease>, CommandError> {
    #[cfg(test)]
    {
        bump_counter(&CLAIM_ATTEMPTS, &storage.root);
        let mut faults = CLAIM_FAULTS
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some((_, remaining, code)) = faults
            .iter_mut()
            .find(|(entry, _, _)| *entry == storage.root)
        {
            if *remaining > 0 {
                *remaining -= 1;
                return Err(CommandError::new(code.clone(), "Injected claim fault."));
            }
        }
    }
    storage.claim_export()
}
impl crate::Core {
    /// One background export worker per process. Cross-process work is fenced by SQLite.
    /// A deliberate wake re-arms an exhausted root; a passive wake
    /// (status polling) never does.
    pub fn wake_export_worker(&self, kind: WakeKind) {
        let Ok(storage) = self.storage() else {
            return;
        };
        let start = {
            let mut queue = PROCESS_WORKER
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            queue.wake(storage.clone(), kind)
        };
        if !start {
            return;
        }
        std::thread::spawn(|| {
            loop {
                // Taking pending work and deciding to stop share the wake-up mutex.
                // A request committed during shutdown therefore cannot lose its wake-up.
                let mut pending = {
                    let mut queue = PROCESS_WORKER
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    match queue.take_next() {
                        Some(pending) => pending,
                        None => return,
                    }
                };
                #[cfg(test)]
                bump_counter(&CLAIM_ADMISSIONS, &pending.storage.root);
                if pending.claim_failures > 0 {
                    std::thread::sleep(CLAIM_RETRY_DELAY);
                }
                loop {
                    match claim_for_worker(&pending.storage) {
                        Ok(Some(lease)) => {
                            pending.claim_failures = 0;
                            let result = match &lease.job.selection {
                                ExportSelection::Backup { .. } => {
                                    pending.storage.execute_backup(&lease)
                                }
                                ExportSelection::Ingestion { .. } => {
                                    pending.storage.execute_ingestion(&lease)
                                }
                                _ => pending.storage.execute_export(&lease),
                            };
                            if result.is_err() {
                                let _ = pending.storage.fail_export(&lease);
                            }
                        }
                        Ok(None) => {
                            // Drained: no more jobs to claim for
                            // this root. Clear the active mark so
                            // later wakes see the root as idle.
                            let mut queue = PROCESS_WORKER
                                .lock()
                                .unwrap_or_else(|error| error.into_inner());
                            queue.finish(&pending.storage.root);
                            break;
                        }
                        Err(error) => {
                            // Only StorageUnavailable is retried, and
                            // only within the budget. Any other code
                            // (a corrupt catalog above all) needs user
                            // action: park immediately without
                            // consuming retries.
                            if error.code != ErrorCode::StorageUnavailable {
                                park_root(
                                    &pending.storage,
                                    JobRecoveryReason::from_claim_error(&error.code),
                                    pending.claim_failures + 1,
                                );
                                break;
                            }
                            pending.claim_failures += 1;
                            if pending.claim_failures >= MAX_CLAIM_FAILURES {
                                park_root(
                                    &pending.storage,
                                    JobRecoveryReason::StorageUnavailable,
                                    pending.claim_failures,
                                );
                                break;
                            }
                            // Re-queue at the back so other roots
                            // make progress before this root is
                            // retried.
                            let mut queue = PROCESS_WORKER
                                .lock()
                                .unwrap_or_else(|error| error.into_inner());
                            queue.requeue(pending);
                            break;
                        }
                    }
                }
            }
        });
    }
    pub fn job_command(
        &self,
        command: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, CommandError> {
        if input
            .get("contractVersion")
            .and_then(serde_json::Value::as_u64)
            != Some(1)
        {
            return Err(CommandError::new(
                ErrorCode::UnsupportedContract,
                "Use contract version 1.",
            ));
        }
        let storage = self.storage()?;
        if matches!(command, "upload-begin" | "upload-part") {
            return self.upload_operation(command, input);
        }
        if matches!(command, "ingest-begin" | "ingest-part") {
            return self.ingestion_operation(command, input);
        }
        if matches!(
            command,
            "backup" | "preview" | "restore" | "schedule" | "schedule-read"
        ) {
            return self.backup_operation(command, input);
        }
        // The wake kind is decided per command: only deliberate
        // actions re-arm an exhausted root. Status polling ("list")
        // and non-queueing controls ("pause"/"cancel") wake
        // passively, so observation can never grant a fresh
        // recovery budget.
        let mut wake_kind = None;
        let response = match command {
            "export" => {
                let request: EnqueueExportRequest =
                    serde_json::from_value(input).map_err(|_| invalid())?;
                if matches!(request.selection, ExportSelection::Backup { .. }) {
                    return Err(invalid());
                }
                wake_kind = Some(WakeKind::Deliberate);
                serde_json::to_value(JobDetails {
                    contract_version: 1,
                    job: storage.enqueue_export(&request.workspace_id, request.selection)?,
                })
            }
            "list" => {
                let request: ListJobsRequest =
                    serde_json::from_value(input).map_err(|_| invalid())?;
                let jobs = storage.list_jobs(&request.workspace_id, request.offset)?;
                let next_offset = if jobs.len() == 50 {
                    request.offset.checked_add(50)
                } else {
                    None
                };
                wake_kind = Some(WakeKind::Passive);
                serde_json::to_value(JobList {
                    contract_version: 1,
                    jobs,
                    next_offset,
                })
            }
            "control" => {
                let request: ControlJobRequest =
                    serde_json::from_value(input).map_err(|_| invalid())?;
                wake_kind = Some(match request.action {
                    JobAction::Retry | JobAction::Resume | JobAction::Recover => {
                        WakeKind::Deliberate
                    }
                    JobAction::Pause | JobAction::Cancel => WakeKind::Passive,
                });
                serde_json::to_value(JobDetails {
                    contract_version: 1,
                    job: storage.control_job(
                        &request.workspace_id,
                        &request.id,
                        request.expected_version,
                        request.action,
                    )?,
                })
            }
            "artifact" => {
                let request: ReadArtifactRequest =
                    serde_json::from_value(input).map_err(|_| invalid())?;
                serde_json::to_value(ArtifactChunk {
                    contract_version: 1,
                    bytes: storage.read_export_chunk(
                        &request.workspace_id,
                        &request.job_id,
                        &request.artifact_id,
                        u64::from(request.offset),
                    )?,
                })
            }
            _ => return Err(invalid()),
        }
        .map_err(|_| unavailable())?;
        if let Some(kind) = wake_kind {
            self.wake_export_worker(kind);
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn storage(name: &str) -> Storage {
        Storage {
            root: PathBuf::from(name),
        }
    }

    #[test]
    fn worker_queue_serves_storages_in_wake_order() {
        let mut queue = WorkerQueue::default();
        assert!(queue.wake(storage("a"), WakeKind::Deliberate));
        assert!(!queue.wake(storage("b"), WakeKind::Deliberate));
        assert!(!queue.wake(storage("c"), WakeKind::Deliberate));
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("a"));
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("b"));
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("c"));
        assert!(queue.take_next().is_none());
        assert!(!queue.running);
    }

    // Mirror tests use root names no other test touches: the
    // mirror is process-global and keyed by root, and the
    // queue tests above share the fake roots "a"/"b"/"c"
    // across parallel tests. Unique keys keep these
    // assertions deterministic.
    #[test]
    fn worker_test_state_tracks_wake_take_requeue_finish_cycle() {
        let mut queue = WorkerQueue::default();
        let root = PathBuf::from("mirror-cycle");
        assert_eq!(worker_test_state(&root), WorkerTestState::default());
        queue.wake(storage("mirror-cycle"), WakeKind::Deliberate);
        assert_eq!(
            worker_test_state(&root),
            WorkerTestState {
                pending: true,
                active: false,
                ever_registered: true,
            }
        );
        let pending = queue.take_next().unwrap();
        assert_eq!(
            worker_test_state(&root),
            WorkerTestState {
                pending: false,
                active: true,
                ever_registered: true,
            }
        );
        queue.requeue(pending);
        assert_eq!(
            worker_test_state(&root),
            WorkerTestState {
                pending: true,
                active: false,
                ever_registered: true,
            }
        );
        let _ = queue.take_next().unwrap();
        queue.finish(&root);
        assert_eq!(
            worker_test_state(&root),
            WorkerTestState {
                pending: false,
                active: false,
                ever_registered: true,
            }
        );
    }

    #[test]
    fn worker_test_state_deliberate_wake_of_active_root_is_pending_and_active() {
        let mut queue = WorkerQueue::default();
        let root = PathBuf::from("mirror-active-fresh");
        queue.wake(storage("mirror-active-fresh"), WakeKind::Deliberate);
        let _ = queue.take_next().unwrap();
        queue.wake(storage("mirror-active-fresh"), WakeKind::Deliberate);
        assert_eq!(
            worker_test_state(&root),
            WorkerTestState {
                pending: true,
                active: true,
                ever_registered: true,
            }
        );
    }

    #[test]
    fn worker_test_state_passive_wake_of_exhausted_root_records_nothing() {
        let mut queue = WorkerQueue::default();
        let root = PathBuf::from("mirror-exhausted");
        queue.mark_exhausted(root.clone(), JobRecoveryReason::StorageUnavailable, 3);
        assert!(!queue.wake(storage("mirror-exhausted"), WakeKind::Passive));
        assert_eq!(worker_test_state(&root), WorkerTestState::default());
        // A passive wake of an idle root does register it.
        let idle = PathBuf::from("mirror-passive-idle");
        assert!(queue.wake(storage("mirror-passive-idle"), WakeKind::Passive));
        assert_eq!(
            worker_test_state(&idle),
            WorkerTestState {
                pending: true,
                active: false,
                ever_registered: true,
            }
        );
    }

    // Gate tests use root names no other test touches, for
    // the same process-global-table reason as the mirror
    // tests above.
    #[test]
    fn admission_gate_holds_front_entry_and_serves_next_root() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("gate-front"), WakeKind::Deliberate);
        queue.wake(storage("gate-next"), WakeKind::Deliberate);
        gate_admissions(&PathBuf::from("gate-front"));
        // The gated front entry is skipped; the next root is
        // served, and the gated entry keeps its place.
        assert_eq!(
            queue.take_next().unwrap().storage.root,
            PathBuf::from("gate-next")
        );
        assert!(queue.take_next().is_none());
        assert!(!queue.running);
        // Release: the gated root is served with its entry
        // intact, and a wake restarts the idle worker.
        release_admissions(&PathBuf::from("gate-front"));
        assert!(queue.wake(storage("gate-front"), WakeKind::Deliberate));
        assert_eq!(
            queue.take_next().unwrap().storage.root,
            PathBuf::from("gate-front")
        );
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn admission_gate_is_root_scoped() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("gate-scoped-a"), WakeKind::Deliberate);
        queue.wake(storage("gate-scoped-b"), WakeKind::Deliberate);
        gate_admissions(&PathBuf::from("gate-scoped-a"));
        assert!(admissions_gated(&PathBuf::from("gate-scoped-a")));
        assert!(!admissions_gated(&PathBuf::from("gate-scoped-b")));
        assert_eq!(
            queue.take_next().unwrap().storage.root,
            PathBuf::from("gate-scoped-b")
        );
        release_admissions(&PathBuf::from("gate-scoped-a"));
        assert!(!admissions_gated(&PathBuf::from("gate-scoped-a")));
        assert_eq!(
            queue.take_next().unwrap().storage.root,
            PathBuf::from("gate-scoped-a")
        );
    }

    #[test]
    fn worker_queue_dedup_keeps_first_position() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        queue.wake(storage("b"), WakeKind::Deliberate);
        queue.wake(storage("a"), WakeKind::Deliberate);
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("a"));
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("b"));
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_restarts_after_going_idle() {
        let mut queue = WorkerQueue::default();
        assert!(queue.wake(storage("a"), WakeKind::Deliberate));
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("a"));
        assert!(queue.take_next().is_none());
        assert!(!queue.running);
        assert!(queue.wake(storage("b"), WakeKind::Deliberate));
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("b"));
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_requeue_goes_to_the_back() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        queue.wake(storage("b"), WakeKind::Deliberate);
        let pending = queue.take_next().unwrap();
        assert_eq!(pending.storage.root, PathBuf::from("a"));
        queue.requeue(pending);
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("b"));
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("a"));
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_rewake_resets_failure_count() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let mut pending = queue.take_next().unwrap();
        pending.claim_failures = 2;
        queue.requeue(pending);
        queue.wake(storage("a"), WakeKind::Deliberate);
        let pending = queue.take_next().unwrap();
        assert_eq!(pending.claim_failures, 0);
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_requeue_drops_when_fresh_wake_pending() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let pending = queue.take_next().unwrap();
        // A fresh wake arrives while the root is being served.
        queue.wake(storage("a"), WakeKind::Deliberate);
        queue.requeue(pending);
        // Exactly one entry, with the fresh budget.
        let pending = queue.take_next().unwrap();
        assert_eq!(pending.claim_failures, 0);
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_drains_after_parking() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let mut pending = queue.take_next().unwrap();
        pending.claim_failures = 2;
        queue.requeue(pending);
        let pending = queue.take_next().unwrap();
        assert_eq!(pending.claim_failures, 2);
        // Parked: no requeue. The queue reports idle and a later
        // wake restarts the worker.
        assert!(queue.take_next().is_none());
        assert!(!queue.running);
        assert!(queue.wake(storage("b"), WakeKind::Deliberate));
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("b"));
    }

    #[test]
    fn worker_queue_passive_wake_ignores_exhausted_root() {
        let mut queue = WorkerQueue::default();
        queue.mark_exhausted(PathBuf::from("a"), JobRecoveryReason::StorageUnavailable, 3);
        // Passive observation must not re-add the root, must not
        // start the worker, and must keep the exhaustion record.
        assert!(!queue.wake(storage("a"), WakeKind::Passive));
        assert!(queue.take_next().is_none());
        assert!(!queue.running);
        let record = queue
            .exhaustion_for(&PathBuf::from("a"))
            .expect("exhaustion kept");
        assert_eq!(record.reason, JobRecoveryReason::StorageUnavailable);
        assert_eq!(record.attempts, 3);
        // A deliberate wake re-arms: the record clears and the
        // root is served with a fresh budget.
        assert!(queue.wake(storage("a"), WakeKind::Deliberate));
        assert!(queue.exhaustion_for(&PathBuf::from("a")).is_none());
        let pending = queue.take_next().unwrap();
        assert_eq!(pending.claim_failures, 0);
    }

    #[test]
    fn reviewer_worker_queue_passive_wake_preserves_inflight_budget() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let mut active = queue.take_next().unwrap();
        active.claim_failures = 2;
        // Status polling lands while the worker holds this root outside the
        // pending queue, e.g. while SQLite is checking a busy catalog.
        assert!(!queue.wake(storage("a"), WakeKind::Passive));
        queue.requeue(active);
        let retry = queue.take_next().unwrap();
        assert_eq!(retry.storage.root, PathBuf::from("a"));
        assert_eq!(
            retry.claim_failures, 2,
            "passive polling discarded the in-flight failure budget",
        );
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_repeated_passive_wakes_of_active_root_add_nothing() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let mut active = queue.take_next().unwrap();
        active.claim_failures = 1;
        // Documents polls every few seconds; several polls may land
        // while one claim is in flight. None may insert an entry.
        assert!(!queue.wake(storage("a"), WakeKind::Passive));
        assert!(!queue.wake(storage("a"), WakeKind::Passive));
        assert!(!queue.wake(storage("a"), WakeKind::Passive));
        queue.requeue(active);
        let retry = queue.take_next().unwrap();
        assert_eq!(retry.claim_failures, 1);
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_deliberate_wake_of_active_root_adds_fresh_entry() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let mut active = queue.take_next().unwrap();
        active.claim_failures = 2;
        // A deliberate re-arm during the active attempt earns a
        // fresh budget: the stale requeue drops and the fresh
        // entry wins.
        queue.wake(storage("a"), WakeKind::Deliberate);
        queue.requeue(active);
        let retry = queue.take_next().unwrap();
        assert_eq!(retry.storage.root, PathBuf::from("a"));
        assert_eq!(retry.claim_failures, 0);
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_stale_park_suppressed_by_deliberate_fresh_entry() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let _active = queue.take_next().unwrap();
        queue.wake(storage("a"), WakeKind::Deliberate);
        // The in-flight attempt exhausts its budget, but a newer
        // deliberate recovery is already pending: the stale park
        // must not mark the root exhausted.
        assert!(!queue.mark_exhausted(
            PathBuf::from("a"),
            JobRecoveryReason::StorageUnavailable,
            3,
        ));
        assert!(queue.exhaustion_for(&PathBuf::from("a")).is_none());
        let retry = queue.take_next().unwrap();
        assert_eq!(retry.storage.root, PathBuf::from("a"));
        assert_eq!(retry.claim_failures, 0);
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_mark_exhausted_without_pending_entry_marks() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let _active = queue.take_next().unwrap();
        // No deliberate re-arm arrived: the park proceeds.
        assert!(
            queue.mark_exhausted(PathBuf::from("a"), JobRecoveryReason::StorageUnavailable, 3,)
        );
        let record = queue.exhaustion_for(&PathBuf::from("a")).unwrap();
        assert_eq!(record.attempts, 3);
        assert_eq!(record.reason, JobRecoveryReason::StorageUnavailable);
    }

    #[test]
    fn worker_queue_finish_clears_active_mark() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let active = queue.take_next().unwrap();
        assert!(!queue.wake(storage("a"), WakeKind::Passive));
        queue.finish(&active.storage.root);
        // Drained and finished: the root is idle again, so a
        // passive wake queues it like any healthy root.
        queue.wake(storage("a"), WakeKind::Passive);
        let retry = queue.take_next().unwrap();
        assert_eq!(retry.storage.root, PathBuf::from("a"));
        assert_eq!(retry.claim_failures, 0);
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_deliberate_fresh_entry_keeps_fifo_position() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        queue.wake(storage("b"), WakeKind::Deliberate);
        let _active = queue.take_next().unwrap();
        queue.wake(storage("a"), WakeKind::Deliberate);
        // The fresh entry for the active root queues behind the
        // already-pending root; it never jumps the queue.
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("b"));
        let retry = queue.take_next().unwrap();
        assert_eq!(retry.storage.root, PathBuf::from("a"));
        assert_eq!(retry.claim_failures, 0);
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_duplicate_wakes_never_duplicate_active_entries() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let mut active = queue.take_next().unwrap();
        active.claim_failures = 2;
        queue.wake(storage("a"), WakeKind::Deliberate);
        queue.wake(storage("a"), WakeKind::Deliberate);
        queue.wake(storage("a"), WakeKind::Passive);
        queue.requeue(active);
        // Exactly one entry survives: the deliberate fresh one.
        let retry = queue.take_next().unwrap();
        assert_eq!(retry.claim_failures, 0);
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_passive_wake_keeps_failure_count() {
        let mut queue = WorkerQueue::default();
        queue.wake(storage("a"), WakeKind::Deliberate);
        let mut pending = queue.take_next().unwrap();
        pending.claim_failures = 2;
        queue.requeue(pending);
        queue.wake(storage("a"), WakeKind::Passive);
        let pending = queue.take_next().unwrap();
        assert_eq!(pending.claim_failures, 2);
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_passive_wake_adds_healthy_root() {
        let mut queue = WorkerQueue::default();
        // A root that is neither pending nor exhausted is served
        // by a passive wake exactly as before.
        assert!(queue.wake(storage("a"), WakeKind::Passive));
        assert_eq!(queue.take_next().unwrap().storage.root, PathBuf::from("a"));
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn worker_queue_exhaustion_record_round_trip() {
        let mut queue = WorkerQueue::default();
        assert!(queue.exhaustion_for(&PathBuf::from("a")).is_none());
        queue.mark_exhausted(PathBuf::from("a"), JobRecoveryReason::StorageUnavailable, 3);
        let record = queue.exhaustion_for(&PathBuf::from("a")).unwrap();
        assert_eq!(record.reason, JobRecoveryReason::StorageUnavailable);
        assert_eq!(record.attempts, 3);
        // Re-marking replaces the record.
        queue.mark_exhausted(PathBuf::from("a"), JobRecoveryReason::CorruptStorage, 1);
        let record = queue.exhaustion_for(&PathBuf::from("a")).unwrap();
        assert_eq!(record.reason, JobRecoveryReason::CorruptStorage);
        assert_eq!(record.attempts, 1);
        // Other roots are unaffected.
        assert!(queue.exhaustion_for(&PathBuf::from("b")).is_none());
        queue.clear_exhausted(&PathBuf::from("a"));
        assert!(queue.exhaustion_for(&PathBuf::from("a")).is_none());
    }
}
