//! Small shared command surface; all destinations are managed IDs.
use crate::{
    CommandError, Core,
    jobs::{ExportSelection, JobDetails, invalid},
    storage::{Storage, unavailable},
};
use chrono::Utc;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub application: bool,
    pub plaintext_consent: bool,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub job_id: String,
    pub artifact_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub job_id: String,
    pub artifact_id: String,
    pub name: String,
    pub expected_hash: String,
    pub confirm: bool,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleRequest {
    pub contract_version: u8,
    pub workspace_id: String,
    pub enabled: bool,
    pub interval_seconds: u32,
    pub plaintext_consent: bool,
    pub keep_all_versions: bool,
    pub managed_destination: bool,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleReadRequest {
    pub contract_version: u8,
    pub workspace_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupSchedule {
    pub contract_version: u8,
    pub enabled: bool,
    pub interval_seconds: u32,
    pub next_due: Option<i64>,
    pub missed_runs: u32,
    pub destination: String,
    pub retention: String,
}

impl Storage {
    pub fn read_backup_schedule(&self, w: &str) -> Result<BackupSchedule, CommandError> {
        self.open(w)?;
        let db = self.connection(false)?.ok_or_else(unavailable)?;
        let row:Option<(bool,u32,i64,u32)>=db.query_row("SELECT enabled,interval_seconds,next_due,missed_runs FROM backup_schedules WHERE workspace_id=?1",[w],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|_|unavailable())?;
        let (enabled, interval, next, missed) = row.unwrap_or((false, 86400, 0, 0));
        Ok(BackupSchedule {
            contract_version: 1,
            enabled,
            interval_seconds: interval,
            next_due: if next == 0 { None } else { Some(next) },
            missed_runs: missed,
            destination: "Private managed storage on this computer".into(),
            retention: "Keep all versions; no automatic deletion".into(),
        })
    }
    pub fn set_backup_schedule(&self, r: ScheduleRequest) -> Result<BackupSchedule, CommandError> {
        self.open(&r.workspace_id)?;
        if !(3600..=2592000).contains(&r.interval_seconds)
            || r.enabled && (!r.plaintext_consent || !r.keep_all_versions || !r.managed_destination)
        {
            return Err(invalid());
        }
        let db = self.connection(false)?.ok_or_else(unavailable)?;
        db.execute("INSERT INTO backup_schedules(workspace_id,enabled,interval_seconds,next_due,consent_version) VALUES(?1,?2,?3,?4,1) ON CONFLICT(workspace_id) DO UPDATE SET enabled=excluded.enabled,interval_seconds=excluded.interval_seconds,next_due=excluded.next_due",params![r.workspace_id,r.enabled,r.interval_seconds,Utc::now().timestamp()+i64::from(r.interval_seconds)]).map_err(|_|unavailable())?;
        self.read_backup_schedule(&r.workspace_id)
    }
    /// Atomic due-time advancement and enqueue fence competing native/API schedulers.
    /// Skipped intervals are counted; a single current catch-up snapshot is queued.
    pub fn tick_backup_schedules(&self, now: i64) -> Result<u32, CommandError> {
        let Some(mut db) = self.connection(false)? else {
            return Ok(0);
        };
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        let mut query=tx.prepare("SELECT workspace_id,interval_seconds,next_due FROM backup_schedules WHERE enabled=1 AND next_due<=?1").map_err(|_|unavailable())?;
        let rows = query
            .query_map([now], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|_| unavailable())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| unavailable())?;
        drop(query);
        let mut queued = 0;
        for (w, interval, due) in rows {
            let count:u32=tx.query_row("SELECT count(*) FROM jobs WHERE workspace_id=?1 AND state IN ('Queued','Running','Paused')",[&w],|r|r.get(0)).map_err(|_|unavailable())?;
            if count >= 32 {
                continue;
            }
            let missed = (now - due) / interval;
            tx.execute("UPDATE backup_schedules SET next_due=?1,missed_runs=missed_runs+?2 WHERE workspace_id=?3",params![now+interval,missed,w]).map_err(|_|unavailable())?;
            let selection = serde_json::to_string(&ExportSelection::Backup { application: false })
                .map_err(|_| invalid())?;
            tx.execute("INSERT INTO jobs(workspace_id,id,state,version,checkpoint_version,checkpoint,selection,created_at,updated_at,log) VALUES(?1,?2,'Queued',1,1,'queued',?3,?4,?4,'Scheduled workspace backup queued')",params![w,crate::content::random_id()?,selection,Utc::now().to_rfc3339()]).map_err(|_|unavailable())?;
            queued += 1;
        }
        tx.commit().map_err(|_| unavailable())?;
        Ok(queued)
    }
}
impl Core {
    pub(crate) fn backup_operation(
        &self,
        command: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, CommandError> {
        let storage = self.storage()?;
        let response = match command {
            "backup" => {
                let r: BackupRequest = serde_json::from_value(input).map_err(|_| invalid())?;
                if !r.plaintext_consent {
                    return Err(invalid());
                }
                serde_json::to_value(JobDetails {
                    contract_version: 1,
                    job: storage.enqueue_export(
                        &r.workspace_id,
                        ExportSelection::Backup {
                            application: r.application,
                        },
                    )?,
                })
            }
            "preview" => {
                let r: SnapshotRequest = serde_json::from_value(input).map_err(|_| invalid())?;
                serde_json::to_value(
                    storage
                        .verify_snapshot(&r.workspace_id, &r.job_id, &r.artifact_id)?
                        .preview,
                )
            }
            "restore" => {
                let r: RestoreRequest = serde_json::from_value(input).map_err(|_| invalid())?;
                if !r.confirm {
                    return Err(invalid());
                }
                serde_json::to_value(storage.restore_snapshot(
                    &r.workspace_id,
                    &r.job_id,
                    &r.artifact_id,
                    &r.name,
                    &r.expected_hash,
                )?)
            }
            "schedule" => {
                let r: ScheduleRequest = serde_json::from_value(input).map_err(|_| invalid())?;
                serde_json::to_value(storage.set_backup_schedule(r)?)
            }
            "schedule-read" => {
                let r: ScheduleReadRequest =
                    serde_json::from_value(input).map_err(|_| invalid())?;
                serde_json::to_value(storage.read_backup_schedule(&r.workspace_id)?)
            }
            _ => return Err(invalid()),
        }
        .map_err(|_| unavailable())?;
        self.wake_export_worker();
        Ok(response)
    }
    pub(crate) fn start_backup_scheduler(&self) {
        let core = self.clone();
        std::thread::spawn(move || {
            loop {
                if let Ok(storage) = core.storage()
                    && storage
                        .tick_backup_schedules(Utc::now().timestamp())
                        .unwrap_or(0)
                        > 0
                {
                    core.wake_export_worker();
                }
                std::thread::sleep(std::time::Duration::from_secs(30));
            }
        });
    }
}
