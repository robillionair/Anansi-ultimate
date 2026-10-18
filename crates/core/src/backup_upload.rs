//! Bounded owned uploads permit recovery from a saved archive into a fresh catalog.
use crate::{
    CommandError, Core, content,
    jobs::{ExportSelection, MAX_BYTES, conflict, invalid},
    storage::unavailable,
};
use rusqlite::{TransactionBehavior, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::{Seek, Write};
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UploadBegin {
    pub contract_version: u8,
    pub workspace_id: String,
    pub bytes: u32,
    pub sha256: String,
    pub plaintext_consent: bool,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UploadPart {
    pub contract_version: u8,
    pub workspace_id: String,
    pub id: String,
    pub offset: u32,
    pub bytes: Vec<u8>,
    pub finish: bool,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UploadStatus {
    pub contract_version: u8,
    pub id: String,
    pub received: u32,
    pub job: Option<crate::jobs::Job>,
}
impl Core {
    pub(crate) fn upload_operation(
        &self,
        command: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, CommandError> {
        let storage = self.storage()?;
        let result = if command == "upload-begin" {
            let r: UploadBegin = serde_json::from_value(input).map_err(|_| invalid())?;
            storage.open(&r.workspace_id)?;
            if !r.plaintext_consent
                || r.bytes < 1024
                || u64::from(r.bytes) > MAX_BYTES
                || r.sha256.len() != 64
                || !r
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return Err(invalid());
            }
            let db = storage.connection(false)?.ok_or_else(unavailable)?;
            let count: u32 = db
                .query_row(
                    "SELECT count(*) FROM backup_uploads WHERE workspace_id=?1",
                    [&r.workspace_id],
                    |r| r.get(0),
                )
                .map_err(|_| unavailable())?;
            if count >= 8 {
                return Err(invalid());
            }
            let id = content::random_id()?;
            let path =
                crate::export::directory(storage, &r.workspace_id)?.join(format!("{id}.upload"));
            crate::snapshots::new_file(&path)?;
            db.execute("INSERT INTO backup_uploads(workspace_id,id,expected_bytes,expected_hash) VALUES(?1,?2,?3,?4)",params![r.workspace_id,id,r.bytes,r.sha256]).map_err(|_|unavailable())?;
            UploadStatus {
                contract_version: 1,
                id,
                received: 0,
                job: None,
            }
        } else {
            let r: UploadPart = serde_json::from_value(input).map_err(|_| invalid())?;
            crate::workspace::validate_id(&r.workspace_id)?;
            crate::workspace::validate_id(&r.id)?;
            if r.bytes.len() > 65536 {
                return Err(invalid());
            }
            let directory = crate::export::directory(storage, &r.workspace_id)?;
            let path = directory.join(format!("{}.upload", r.id));
            crate::storage::reject_links(&path).map_err(|_| invalid())?;
            let mut db = storage.connection(false)?.ok_or_else(unavailable)?;
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|_| unavailable())?;
            let (expected,hash,received):(u32,String,u32)=tx.query_row("SELECT expected_bytes,expected_hash,received FROM backup_uploads WHERE workspace_id=?1 AND id=?2",params![r.workspace_id,r.id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).map_err(|_|invalid())?;
            if r.offset != received || r.offset.saturating_add(r.bytes.len() as u32) > expected {
                return Err(conflict());
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .map_err(|_| unavailable())?;
            file.seek(std::io::SeekFrom::Start(u64::from(received)))
                .map_err(|_| unavailable())?;
            file.write_all(&r.bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| unavailable())?;
            let received = received + r.bytes.len() as u32;
            tx.execute(
                "UPDATE backup_uploads SET received=?1 WHERE workspace_id=?2 AND id=?3",
                params![received, r.workspace_id, r.id],
            )
            .map_err(|_| unavailable())?;
            tx.commit().map_err(|_| unavailable())?;
            drop(file);
            if r.finish {
                if received != expected {
                    return Err(invalid());
                }
                let bytes = crate::snapshots::limited(&path, MAX_BYTES)?;
                if content::hash(&bytes) != hash || bytes.len() != expected as usize {
                    return Err(content::integrity());
                }
                let verified = storage.verify_snapshot_bytes(&bytes)?;
                let artifact = content::random_id()?;
                // Hard link publishes an immutable name without overwriting any file.
                std::fs::hard_link(&path, directory.join(format!("{artifact}.tar")))
                    .map_err(|_| unavailable())?;
                let mut db = storage.connection(false)?.ok_or_else(unavailable)?;
                let tx = db
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|_| unavailable())?;
                let id = content::random_id()?;
                let selection = serde_json::to_string(&ExportSelection::Backup {
                    application: verified.info.application,
                })
                .map_err(|_| invalid())?;
                let removed=tx.execute("DELETE FROM backup_uploads WHERE workspace_id=?1 AND id=?2 AND received=?3",params![r.workspace_id,r.id,received]).map_err(|_|unavailable())?;
                if removed != 1 {
                    return Err(conflict());
                }
                tx.execute("INSERT INTO jobs(workspace_id,id,state,version,checkpoint_version,checkpoint,selection,artifact_id,artifact_bytes,artifact_hash,created_at,updated_at,log) VALUES(?1,?2,'Completed',1,1,'verified import',?3,?4,?5,?6,?7,?7,'Imported backup integrity verified')",params![r.workspace_id,id,selection,artifact,expected,hash,chrono::Utc::now().to_rfc3339()]).map_err(|_|unavailable())?;
                tx.commit().map_err(|_| unavailable())?;
                UploadStatus {
                    contract_version: 1,
                    id: r.id,
                    received,
                    job: Some(storage.read_job(&r.workspace_id, &id)?),
                }
            } else {
                UploadStatus {
                    contract_version: 1,
                    id: r.id,
                    received,
                    job: None,
                }
            }
        };
        serde_json::to_value(result).map_err(|_| unavailable())
    }
}
