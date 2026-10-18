//! Durable export job state and lease fencing. Payloads and logs never contain source text.
use crate::{
    CommandError, ErrorCode, content,
    storage::{Storage, unavailable},
    workspace::validate_id,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    Backup { application: bool },
    Project { id: String },
    Records { ids: Vec<String> },
}
impl ExportSelection {
    pub(crate) fn validate(&self) -> Result<(), CommandError> {
        match self {
            Self::Workspace | Self::Backup { .. } => Ok(()),
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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum JobAction {
    Pause,
    Resume,
    Cancel,
    Retry,
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

fn read(connection: &Connection, workspace: &str, id: &str) -> Result<Job, CommandError> {
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
    })
}

/// A worker identity is generated by trusted Rust code and never returned to clients.
pub struct JobLease {
    pub job: Job,
    pub(crate) owner: String,
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
            .map(|id| read(&connection, workspace, id))
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
            _ => return Err(invalid()),
        };
        tx.execute("UPDATE jobs SET state=?1,version=version+1,owner=NULL,lease_until=NULL,updated_at=?2,log=?3 WHERE workspace_id=?4 AND id=?5",params![state_name(&next),now(),match next { JobState::Queued=>"Export queued explicitly",JobState::Paused=>"Export paused explicitly",_=>"Export cancelled explicitly" },workspace,id]).map_err(|_| unavailable())?;
        let result = read(&tx, workspace, id)?;
        tx.commit().map_err(|_| unavailable())?;
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
        tx.execute("UPDATE jobs SET state='Running',version=version+1,checkpoint_version=checkpoint_version+1,checkpoint='snapshot',progress=0,owner=?1,lease_until=?2,updated_at=?3,log='Export snapshot started' WHERE workspace_id=?4 AND id=?5",params![owner,Utc::now().timestamp()+LEASE_SECONDS,now(),workspace,id]).map_err(|_| unavailable())?;
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

struct WorkerQueue {
    running: bool,
    pending: Vec<Storage>,
}
static PROCESS_WORKER: std::sync::Mutex<WorkerQueue> = std::sync::Mutex::new(WorkerQueue {
    running: false,
    pending: Vec::new(),
});
impl crate::Core {
    /// One background export worker per process. Cross-process work is fenced by SQLite.
    pub fn wake_export_worker(&self) {
        let Ok(storage) = self.storage() else {
            return;
        };
        let mut queue = PROCESS_WORKER
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !queue
            .pending
            .iter()
            .any(|pending| pending.root == storage.root)
        {
            queue.pending.push(storage.clone());
        }
        if queue.running {
            return;
        }
        queue.running = true;
        drop(queue);
        std::thread::spawn(|| {
            loop {
                // Taking pending work and deciding to stop share the wake-up mutex.
                // A request committed during shutdown therefore cannot lose its wake-up.
                let storage = {
                    let mut queue = PROCESS_WORKER
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    match queue.pending.pop() {
                        Some(storage) => storage,
                        None => {
                            queue.running = false;
                            return;
                        }
                    }
                };
                while let Ok(Some(lease)) = storage.claim_export() {
                    let result = if matches!(lease.job.selection, ExportSelection::Backup { .. }) {
                        storage.execute_backup(&lease)
                    } else {
                        storage.execute_export(&lease)
                    };
                    if result.is_err() {
                        let _ = storage.fail_export(&lease);
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
        if matches!(
            command,
            "backup" | "preview" | "restore" | "schedule" | "schedule-read"
        ) {
            return self.backup_operation(command, input);
        }
        let response = match command {
            "export" => {
                let request: EnqueueExportRequest =
                    serde_json::from_value(input).map_err(|_| invalid())?;
                if matches!(request.selection, ExportSelection::Backup { .. }) {
                    return Err(invalid());
                }
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
                serde_json::to_value(JobList {
                    contract_version: 1,
                    jobs,
                    next_offset,
                })
            }
            "control" => {
                let request: ControlJobRequest =
                    serde_json::from_value(input).map_err(|_| invalid())?;
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
        if command != "artifact" {
            self.wake_export_worker();
        }
        Ok(response)
    }
}
