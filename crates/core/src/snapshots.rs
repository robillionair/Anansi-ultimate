//! Managed backup assembly and strict archive verification. No client paths.
use crate::{
    CommandError,
    backup::{BACKUP_FORMAT, BACKUP_VERSION, BackupManifest, BackupMember},
    content,
    export::{Archive, directory},
    jobs::{ExportSelection, JobLease, MAX_BYTES, MAX_RECORDS, conflict, invalid},
    storage::{
        DATABASE_NAME, SCHEMA_VERSION, Storage, private_permissions, reject_links, unavailable,
    },
};
use chrono::Utc;
use rusqlite::{Connection, OpenFlags, params, types::Value};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const TABLES: &[&str] = &[
    "workspaces",
    "workspace_settings",
    "memories",
    "memory_revisions",
    "relationships",
    "relationship_revisions",
    "artifact_versions",
    "restore_origins",
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SnapshotInfo {
    pub schema_version: i64,
    pub application: bool,
    pub workspaces: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotPreview {
    pub contract_version: u8,
    pub workspace_id: String,
    pub workspace_name: String,
    pub schema_version: i64,
    pub application: bool,
    pub workspace_count: u32,
    pub records: u32,
    pub revisions: u32,
    pub relationships: u32,
    pub total_bytes: u64,
    pub sha256: String,
    pub privacy: String,
    pub conflicts: Vec<String>,
}

pub(crate) fn new_file(path: &Path) -> Result<File, CommandError> {
    reject_links(path).map_err(|_| unavailable())?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(|_| unavailable())?;
    private_permissions(path, false, false).map_err(|_| unavailable())?;
    Ok(file)
}
pub(crate) fn stage(storage: &Storage) -> Result<Storage, CommandError> {
    // connection initializes/validates the trusted root before any stage paths are made.
    storage.connection(false)?.ok_or_else(unavailable)?;
    let parent = storage.root.join("backup-staging");
    reject_links(&parent).map_err(|_| unavailable())?;
    match fs::create_dir(&parent) {
        Ok(()) => private_permissions(&parent, true, false).map_err(|_| unavailable())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err(unavailable()),
    }
    private_permissions(&parent, true, true).map_err(|_| unavailable())?;
    let root = parent.join(content::random_id()?);
    fs::create_dir(&root).map_err(|_| unavailable())?;
    private_permissions(&root, true, false).map_err(|_| unavailable())?;
    Storage::new(root)
}
pub(crate) fn limited(path: &Path, max: u64) -> Result<Vec<u8>, CommandError> {
    reject_links(path).map_err(|_| invalid())?;
    let mut file = File::open(path).map_err(|_| invalid())?;
    let size = file.metadata().map_err(|_| invalid())?.len();
    if size > max {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() as u64 > max {
        return Err(invalid());
    }
    Ok(bytes)
}
pub(crate) fn copy_rows(
    source: &Connection,
    target: &Connection,
    table: &str,
    workspace: &str,
    budget: &mut u64,
) -> Result<(), CommandError> {
    // table identifiers are fixed by callers, never renderer input.
    if !TABLES.contains(&table) {
        return Err(invalid());
    }
    let key = if table == "workspaces" {
        "id"
    } else {
        "workspace_id"
    };
    let mut query = source
        .prepare(&format!("SELECT * FROM {table} WHERE {key}=?1"))
        .map_err(|_| unavailable())?;
    let count = query.column_count();
    let placeholders = (1..=count)
        .map(|n| format!("?{n}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut insert = target
        .prepare(&format!("INSERT INTO {table} VALUES({placeholders})"))
        .map_err(|_| unavailable())?;
    let mut rows = query.query([workspace]).map_err(|_| unavailable())?;
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        let values = (0..count)
            .map(|i| row.get::<_, Value>(i))
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| unavailable())?;
        for value in &values {
            let size = match value {
                Value::Text(text) => text.len() as u64,
                Value::Blob(bytes) => bytes.len() as u64,
                _ => 16,
            };
            *budget = budget.checked_add(size + 32).ok_or_else(invalid)?;
            if *budget > MAX_BYTES {
                return Err(invalid());
            }
        }
        insert
            .execute(rusqlite::params_from_iter(values))
            .map_err(|_| unavailable())?;
    }
    Ok(())
}

impl Storage {
    pub fn execute_backup(&self, lease: &JobLease) -> Result<(), CommandError> {
        self.execute_backup_with_snapshot_hook(lease, || Ok(()))
    }
    fn execute_backup_with_snapshot_hook(
        &self,
        lease: &JobLease,
        after_snapshot: impl FnOnce() -> Result<(), CommandError>,
    ) -> Result<(), CommandError> {
        let ExportSelection::Backup { application } = lease.job.selection else {
            return Err(invalid());
        };
        self.checkpoint_export(lease, 0)?;
        let temporary = stage(self)?;
        // Initialize an empty constrained standalone catalog. It has never held another workspace.
        let mut target = temporary.connection(true)?.ok_or_else(unavailable)?;
        target
            .execute_batch("PRAGMA journal_mode=DELETE;")
            .map_err(|_| unavailable())?;
        let mut connection = self.connection(false)?.ok_or_else(unavailable)?;
        let snapshot = connection.transaction().map_err(|_| unavailable())?;
        let count: u32 = if application {
            snapshot.query_row("SELECT count(*) FROM memories", [], |r| r.get(0))
        } else {
            snapshot.query_row(
                "SELECT count(*) FROM memories WHERE workspace_id=?1",
                [&lease.job.workspace_id],
                |r| r.get(0),
            )
        }
        .map_err(|_| unavailable())?;
        if count > MAX_RECORDS {
            return Err(invalid());
        }
        let target_page_size: u64 = target
            .pragma_query_value(None, "page_size", |r| r.get::<_, u32>(0))
            .map(u64::from)
            .map_err(|_| unavailable())?;
        target
            .pragma_update(
                None,
                "max_page_count",
                i64::try_from(MAX_BYTES / target_page_size).map_err(|_| invalid())?,
            )
            .map_err(|_| unavailable())?;
        let mut ids = vec![lease.job.workspace_id.clone()];
        if application {
            let mut statement = snapshot
                .prepare("SELECT id FROM workspaces ORDER BY id")
                .map_err(|_| unavailable())?;
            let source_page_size: u32 = snapshot
                .pragma_query_value(None, "page_size", |r| r.get(0))
                .map_err(|_| unavailable())?;
            let source_page_count: u32 = snapshot
                .pragma_query_value(None, "page_count", |r| r.get(0))
                .map_err(|_| unavailable())?;
            if u64::from(source_page_size) * u64::from(source_page_count) > MAX_BYTES {
                return Err(invalid());
            }
            ids = statement
                .query_map([], |r| r.get(0))
                .map_err(|_| unavailable())?
                .collect::<rusqlite::Result<_>>()
                .map_err(|_| unavailable())?;
            if ids.len() > MAX_RECORDS as usize {
                return Err(invalid());
            }
            // SQLite's online backup API includes committed WAL pages from this read snapshot.
            {
                let backup = rusqlite::backup::Backup::new(&snapshot, &mut target)
                    .map_err(|_| unavailable())?;
                let started = std::time::Instant::now();
                loop {
                    self.checkpoint_export(lease, 0)?;
                    if started.elapsed() > std::time::Duration::from_secs(60) {
                        return Err(unavailable());
                    }
                    match backup.step(64).map_err(|_| unavailable())? {
                        rusqlite::backup::StepResult::Done => break,
                        rusqlite::backup::StepResult::More => {}
                        _ => std::thread::sleep(std::time::Duration::from_millis(10)),
                    }
                }
            }
            // Schedules remain opt-in after recovery. Never run imported scheduled work automatically.
            target
                .execute("UPDATE backup_schedules SET enabled=0", [])
                .map_err(|_| unavailable())?;
            target.execute("UPDATE jobs SET state='Paused',owner=NULL,lease_until=NULL WHERE state IN ('Running','Queued')",[]).map_err(|_|unavailable())?;
            target
                .execute("DELETE FROM backup_uploads", [])
                .map_err(|_| unavailable())?;
        } else {
            let tx = target.transaction().map_err(|_| unavailable())?;
            tx.execute_batch("PRAGMA defer_foreign_keys=ON;")
                .map_err(|_| unavailable())?;
            let mut budget = 0;
            for table in TABLES {
                copy_rows(&snapshot, &tx, table, &lease.job.workspace_id, &mut budget)?;
            }
            tx.commit().map_err(|_| unavailable())?;
        }
        after_snapshot()?;
        let info = SnapshotInfo {
            schema_version: SCHEMA_VERSION,
            application,
            workspaces: ids.clone(),
        };
        let info_bytes = serde_json::to_vec(&info).map_err(|_| invalid())?;
        let mut members = vec![];
        // Build archive directly from the immutable database snapshot and referenced body files.
        let artifact = content::random_id()?;
        let mut archive = Archive {
            file: new_file(
                &directory(self, &lease.job.workspace_id)?.join(format!("{artifact}.tar")),
            )?,
            bytes: 0,
            hash: Sha256::new(),
        };
        let mut total_records = 0u32;
        for workspace in &ids {
            let n: u32 = snapshot
                .query_row(
                    "SELECT count(*) FROM memories WHERE workspace_id=?1",
                    [workspace],
                    |r| r.get(0),
                )
                .map_err(|_| unavailable())?;
            total_records = total_records.checked_add(n).ok_or_else(invalid)?;
            if total_records > MAX_RECORDS {
                return Err(invalid());
            }
            let mut query=snapshot.prepare("SELECT memory_id,id,body_hash,metadata FROM memory_revisions WHERE workspace_id=?1 ORDER BY memory_id,version").map_err(|_|unavailable())?;
            let mut rows = query.query([workspace]).map_err(|_| unavailable())?;
            let mut revisions = 0u32;
            while let Some(row) = rows.next().map_err(|_| unavailable())? {
                let (record, revision, hash, raw): (String, String, String, String) = (
                    row.get(0).map_err(|_| invalid())?,
                    row.get(1).map_err(|_| invalid())?,
                    row.get(2).map_err(|_| invalid())?,
                    row.get(3).map_err(|_| invalid())?,
                );
                let meta: crate::memory::MemoryMetadata =
                    serde_json::from_str(&raw).map_err(|_| content::integrity())?;
                meta.privacy.allow_plaintext()?;
                let body = content::read(&self.root, workspace, &record, &revision, &hash)?;
                let path = format!("bodies/{revision}.md");
                archive.member(&path, body.as_bytes())?;
                members.push(BackupMember {
                    path,
                    bytes: body.len() as u64,
                    sha256: hash,
                });
                revisions += 1;
                if revisions.is_multiple_of(8) {
                    self.checkpoint_export(lease, total_records)?;
                }
            }
        }
        // Full application backups also retain every owned completed job artifact.
        if application {
            let mut query = snapshot
                .prepare("SELECT workspace_id,id,artifact_id FROM jobs WHERE state='Completed'")
                .map_err(|_| unavailable())?;
            let mut rows = query.query([]).map_err(|_| unavailable())?;
            while let Some(row) = rows.next().map_err(|_| unavailable())? {
                let (workspace, job, id): (String, String, String) = (
                    row.get(0).map_err(|_| invalid())?,
                    row.get(1).map_err(|_| invalid())?,
                    row.get(2).map_err(|_| invalid())?,
                );
                let details = self.read_job(&workspace, &job)?;
                let data = limited(
                    &directory(self, &workspace)?.join(format!("{id}.tar")),
                    MAX_BYTES,
                )?;
                if Some(content::hash(&data)) != details.artifact_hash
                    || Some(data.len() as u64) != details.artifact_bytes
                {
                    return Err(content::integrity());
                }
                let path = format!("artifacts/{id}.tar");
                archive.member(&path, &data)?;
                members.push(BackupMember {
                    path,
                    bytes: data.len() as u64,
                    sha256: content::hash(&data),
                });
                self.checkpoint_export(lease, total_records)?;
            }
        }
        target
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
            .map_err(|_| unavailable())?;
        drop(target);
        let catalog = limited(&temporary.root.join(DATABASE_NAME), MAX_BYTES)?;
        for (path, bytes) in [("catalog.sqlite3", catalog), ("snapshot.json", info_bytes)] {
            archive.member(path, &bytes)?;
            members.push(BackupMember {
                path: path.into(),
                bytes: bytes.len() as u64,
                sha256: content::hash(&bytes),
            });
        }
        let name: String = snapshot
            .query_row(
                "SELECT name FROM workspaces WHERE id=?1",
                [&lease.job.workspace_id],
                |r| r.get(0),
            )
            .map_err(|_| invalid())?;
        let manifest = BackupManifest {
            format: BACKUP_FORMAT.into(),
            format_version: BACKUP_VERSION,
            workspace_id: lease.job.workspace_id.clone(),
            workspace_name: name,
            created_at: Utc::now().to_rfc3339(),
            members,
        };
        let bytes = serde_json::to_vec(&manifest).map_err(|_| invalid())?;
        if bytes.len() as u64 > crate::backup::MAX_MANIFEST_BYTES {
            return Err(invalid());
        }
        archive.member("manifest.json", &bytes)?;
        archive.write(&[0; 1024])?;
        archive.file.sync_all().map_err(|_| unavailable())?;
        let size = archive.bytes;
        use sha2::Digest;
        let hash = format!("{:x}", archive.hash.finalize());
        drop(archive.file);
        let completed = limited(
            &directory(self, &lease.job.workspace_id)?.join(format!("{artifact}.tar")),
            MAX_BYTES,
        )?;
        self.verify_snapshot_bytes_with_tick(&completed, || {
            self.checkpoint_export(lease, total_records)
        })?;
        snapshot.commit().map_err(|_| unavailable())?;
        self.checkpoint_export(lease, total_records)?;
        let mut connection = self.connection(false)?.ok_or_else(unavailable)?;
        let publish = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        // Privacy may change after the read snapshot. Recheck all current classifications under lock.
        for workspace in &ids {
            let sealed_artifacts: bool = publish
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM artifact_versions WHERE workspace_id=?1 AND privacy='sealed')",
                    [workspace],
                    |r| r.get(0),
                )
                .map_err(|_| unavailable())?;
            if sealed_artifacts {
                return Err(CommandError::new(
                    crate::ErrorCode::VaultUnavailable,
                    "Sealed artifacts cannot be included in a plaintext backup.",
                ));
            }
            let mut q = publish
                .prepare("SELECT metadata FROM memory_revisions WHERE workspace_id=?1")
                .map_err(|_| unavailable())?;
            let rows = q
                .query_map([workspace], |r| r.get::<_, String>(0))
                .map_err(|_| unavailable())?;
            for raw in rows {
                let m: crate::memory::MemoryMetadata =
                    serde_json::from_str(&raw.map_err(|_| invalid())?).map_err(|_| invalid())?;
                m.privacy.allow_plaintext()?;
            }
        }
        let n=publish.execute("UPDATE jobs SET state='Completed',version=version+1,checkpoint_version=checkpoint_version+1,checkpoint='published',owner=NULL,lease_until=NULL,artifact_id=?1,artifact_bytes=?2,artifact_hash=?3,log='Complete backup ready' WHERE workspace_id=?4 AND id=?5 AND state='Running' AND owner=?6 AND lease_until>?7",params![artifact,size as i64,hash,lease.job.workspace_id,lease.job.id,lease.owner,Utc::now().timestamp()]).map_err(|_|unavailable())?;
        if n != 1 {
            return Err(conflict());
        }
        publish.commit().map_err(|_| unavailable())?;
        Ok(())
    }
}

fn octal(bytes: &[u8]) -> Result<usize, CommandError> {
    usize::from_str_radix(
        std::str::from_utf8(bytes)
            .map_err(|_| invalid())?
            .trim_matches(['\0', ' ']),
        8,
    )
    .map_err(|_| invalid())
}
type SchemaRow = (String, String, String, Option<String>);

fn schema_objects(
    db: &Connection,
) -> Result<std::collections::BTreeMap<String, SchemaRow>, CommandError> {
    let mut q = db
        .prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY name")
        .map_err(|_| invalid())?;
    let rows = q
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .map_err(|_| invalid())?
        .collect::<rusqlite::Result<Vec<SchemaRow>>>()
        .map_err(|_| invalid())?;
    Ok(rows
        .into_iter()
        .map(|row| (row.1.clone(), row))
        .collect())
}

/// The schema of a pre-artifact catalog (migrations
/// 0001-0008), exactly as the older releases created it.
const LEGACY_SCHEMA_8: &str = concat!(
    include_str!("../migrations/0001_workspace.sql"),
    "\n",
    include_str!("../migrations/0002_onboarding.sql"),
    "\n",
    include_str!("../migrations/0003_memory.sql"),
    include_str!("../migrations/0004_lifecycle.sql"),
    include_str!("../migrations/0005_entities.sql"),
    include_str!("../migrations/0006_relationship_history.sql"),
    include_str!("../migrations/0007_jobs.sql"),
    include_str!("../migrations/0008_backups.sql")
);
const VAULT_SCHEMA_9: &str = include_str!("../migrations/0009_vault.sql");
const ARTIFACTS_SCHEMA_10: &str = include_str!("../migrations/0010_artifacts.sql");

fn in_memory_schema(ddl: &[&str]) -> Result<Connection, CommandError> {
    let db = Connection::open_in_memory().map_err(|_| invalid())?;
    for batch in ddl {
        db.execute_batch(batch).map_err(|_| invalid())?;
    }
    Ok(db)
}

/// The embeddings table is the one object whose stored
/// CREATE text has a supported in-place ALTER history (see
/// vector::ensure_embeddings_table), so its identity is its
/// column fingerprint rather than its CREATE text.
fn embeddings_fingerprint(db: &Connection) -> Result<String, CommandError> {
    let mut q = db
        .prepare(
            "SELECT name,type,\"notnull\",dflt_value,pk FROM pragma_table_info('memory_embeddings') ORDER BY name",
        )
        .map_err(|_| invalid())?;
    let cols = q
        .query_map([], |r| {
            Ok(format!(
                "{}:{}:{}:{}:{}",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                r.get::<_, i64>(4)?
            ))
        })
        .map_err(|_| invalid())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| invalid())?;
    if cols.is_empty() {
        return Err(invalid());
    }
    Ok(cols.join("|"))
}

/// Named schema groups a catalog may contain. Derived
/// structures (vault, full-text index, embeddings) are
/// recognised exactly - built from the same DDL the runtime
/// executes - never ignored wholesale.
struct SchemaGroups {
    core10: std::collections::BTreeMap<String, SchemaRow>,
    core8: std::collections::BTreeMap<String, SchemaRow>,
    vault: std::collections::BTreeMap<String, SchemaRow>,
    fts: std::collections::BTreeMap<String, SchemaRow>,
    embeddings: std::collections::BTreeMap<String, SchemaRow>,
    embeddings_fingerprint: String,
}

fn schema_groups() -> Result<SchemaGroups, CommandError> {
    let core10 = schema_objects(&in_memory_schema(&[crate::storage::CURRENT_SCHEMA])?)?;
    let core8 = schema_objects(&in_memory_schema(&[LEGACY_SCHEMA_8])?)?;
    let v9 = schema_objects(&in_memory_schema(&[LEGACY_SCHEMA_8, VAULT_SCHEMA_9])?)?;
    let vault: std::collections::BTreeMap<_, _> = v9
        .into_iter()
        .filter(|(name, _)| !core8.contains_key(name))
        .collect();
    let fts = schema_objects(&in_memory_schema(&[crate::search::FTS_DDL])?)?;
    let emb_db = Connection::open_in_memory().map_err(|_| invalid())?;
    crate::vector::ensure_embeddings_table(&emb_db).map_err(|_| invalid())?;
    let embeddings_fingerprint = embeddings_fingerprint(&emb_db)?;
    let embeddings = schema_objects(&emb_db)?;
    Ok(SchemaGroups {
        core10,
        core8,
        vault,
        fts,
        embeddings,
        embeddings_fingerprint,
    })
}

fn definition_matches(
    groups: &SchemaGroups,
    name: &str,
    actual: &SchemaRow,
    expected: &SchemaRow,
    db: &Connection,
) -> Result<bool, CommandError> {
    if name == "memory_embeddings" {
        return Ok(actual.0 == expected.0
            && embeddings_fingerprint(db)? == groups.embeddings_fingerprint);
    }
    Ok(actual == expected)
}

/// Every required object present with an identical
/// definition; every catalog object accounted for by a
/// group; each optional group complete or absent.
fn check_schema(
    db: &Connection,
    groups: &SchemaGroups,
    required: &std::collections::BTreeMap<String, SchemaRow>,
    optional: &[&std::collections::BTreeMap<String, SchemaRow>],
) -> Result<(), CommandError> {
    let actual = schema_objects(db)?;
    for (name, def) in required {
        match actual.get(name) {
            Some(found) if definition_matches(groups, name, found, def, db)? => {}
            _ => return Err(invalid()),
        }
    }
    for group in optional {
        let present = group.keys().filter(|name| actual.contains_key(*name)).count();
        if present != 0 && present != group.len() {
            return Err(invalid());
        }
    }
    for (name, found) in &actual {
        let known = required
            .get(name)
            .or_else(|| optional.iter().find_map(|g| g.get(name)));
        match known {
            Some(def) if definition_matches(groups, name, found, def, db)? => {}
            _ => return Err(invalid()),
        }
    }
    Ok(())
}

/// Migrate an isolated staging catalog from schema 8/9 to
/// 10 with the same step the storage migrator uses. Only
/// ever called after the legacy structure has been fully
/// validated; the source package is never touched.
fn migrate_staging_catalog(path: &Path) -> Result<(), CommandError> {
    let mut connection = Connection::open(path).map_err(|_| invalid())?;
    connection
        .execute_batch("PRAGMA trusted_schema=OFF;")
        .map_err(|_| invalid())?;
    let tx = connection.transaction().map_err(|_| invalid())?;
    tx.execute_batch(ARTIFACTS_SCHEMA_10)
        .map_err(|_| invalid())?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(|_| invalid())?;
    tx.commit().map_err(|_| invalid())?;
    Ok(())
}

/// Strict USTAR subset: regular files, exact portable names, zero extension fields and no duplicates.
#[cfg(test)]
pub(crate) fn unpack(data: &[u8], root: &Path) -> Result<(), CommandError> {
    unpack_with_tick(data, root, || Ok(()))
}
fn unpack_with_tick(
    data: &[u8],
    root: &Path,
    mut tick: impl FnMut() -> Result<(), CommandError>,
) -> Result<(), CommandError> {
    if data.len() as u64 > MAX_BYTES || !data.len().is_multiple_of(512) {
        return Err(invalid());
    }
    let mut offset = 0usize;
    let mut seen = BTreeSet::new();
    while offset + 512 <= data.len() {
        if seen.len().is_multiple_of(8) {
            tick()?;
        }
        let h = &data[offset..offset + 512];
        if h.iter().all(|b| *b == 0) {
            if data.len() - offset < 1024 || data[offset..].iter().any(|b| *b != 0) {
                return Err(invalid());
            }
            return Ok(());
        }
        let sum: usize = h
            .iter()
            .enumerate()
            .map(|(i, b)| {
                if (148..156).contains(&i) {
                    32
                } else {
                    usize::from(*b)
                }
            })
            .sum();
        if octal(&h[148..156])? != sum
            || &h[257..263] != b"ustar\0"
            || &h[263..265] != b"00"
            || h[156] != b'0'
            || h[157..257].iter().any(|b| *b != 0)
            || h[345..500].iter().any(|b| *b != 0)
        {
            return Err(invalid());
        }
        let end = h[..100].iter().position(|b| *b == 0).ok_or_else(invalid)?;
        if h[end..100].iter().any(|b| *b != 0) {
            return Err(invalid());
        }
        let name = std::str::from_utf8(&h[..end]).map_err(|_| invalid())?;
        let valid = name == "manifest.json"
            || name == "snapshot.json"
            || name == "catalog.sqlite3"
            || ((name.starts_with("bodies/") && name.ends_with(".md"))
                || (name.starts_with("artifacts/") && name.ends_with(".tar")))
                && {
                    let id = name
                        .split('/')
                        .nth(1)
                        .and_then(|p| p.split('.').next())
                        .ok_or_else(invalid)?;
                    crate::workspace::validate_id(id).is_ok() && name.matches('/').count() == 1
                };
        if !valid || !seen.insert(name.to_string()) {
            return Err(invalid());
        }
        let size = octal(&h[124..136])?;
        let start = offset + 512;
        let finish = start.checked_add(size).ok_or_else(invalid)?;
        if finish > data.len()
            || (name == "manifest.json" || name == "snapshot.json")
                && size as u64 > crate::backup::MAX_MANIFEST_BYTES
        {
            return Err(invalid());
        }
        let path = root.join(name);
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            fs::create_dir(parent).map_err(|_| unavailable())?;
            private_permissions(parent, true, false).map_err(|_| unavailable())?;
        }
        let mut file = new_file(&path)?;
        file.write_all(&data[start..finish])
            .and_then(|_| file.sync_all())
            .map_err(|_| unavailable())?;
        offset = finish.div_ceil(512) * 512;
    }
    Err(invalid())
}

pub(crate) struct VerifiedSnapshot {
    pub stage: Storage,
    pub info: SnapshotInfo,
    pub preview: SnapshotPreview,
}

impl Storage {
    pub(crate) fn verify_snapshot(
        &self,
        workspace: &str,
        job: &str,
        artifact: &str,
    ) -> Result<VerifiedSnapshot, CommandError> {
        let owned = self.read_job(workspace, job)?;
        if !matches!(owned.selection, ExportSelection::Backup { .. })
            || owned.state != crate::jobs::JobState::Completed
            || owned.artifact_id.as_deref() != Some(artifact)
        {
            return Err(invalid());
        }
        crate::workspace::validate_id(artifact)?;
        let bytes = limited(
            &directory(self, workspace)?.join(format!("{artifact}.tar")),
            MAX_BYTES,
        )?;
        if Some(content::hash(&bytes)) != owned.artifact_hash
            || Some(bytes.len() as u64) != owned.artifact_bytes
        {
            return Err(content::integrity());
        }
        self.verify_snapshot_bytes(&bytes)
    }
    pub(crate) fn verify_snapshot_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<VerifiedSnapshot, CommandError> {
        self.verify_snapshot_bytes_with_tick(bytes, || Ok(()))
    }
    fn verify_snapshot_bytes_with_tick(
        &self,
        bytes: &[u8],
        mut tick: impl FnMut() -> Result<(), CommandError>,
    ) -> Result<VerifiedSnapshot, CommandError> {
        let temporary = stage(self)?;
        unpack_with_tick(bytes, &temporary.root, &mut tick)?;
        let checked = crate::backup::read_and_verify(&temporary.root)?;
        let info: SnapshotInfo = serde_json::from_slice(&limited(
            &temporary.root.join("snapshot.json"),
            crate::backup::MAX_MANIFEST_BYTES,
        )?)
        .map_err(|_| invalid())?;
        if !matches!(info.schema_version, 8 | 9 | SCHEMA_VERSION)
            || info.workspaces.is_empty()
            || info.workspaces.len() > MAX_RECORDS as usize
            || !info.workspaces.contains(&checked.manifest.workspace_id)
            || !info.application && info.workspaces != vec![checked.manifest.workspace_id.clone()]
        {
            return Err(invalid());
        }
        let expected: BTreeSet<_> = checked
            .manifest
            .members
            .iter()
            .map(|m| m.path.clone())
            .chain(std::iter::once("manifest.json".into()))
            .collect();
        let mut actual = BTreeSet::new();
        for entry in fs::read_dir(&temporary.root).map_err(|_| invalid())? {
            let e = entry.map_err(|_| invalid())?;
            let name = e.file_name().into_string().map_err(|_| invalid())?;
            if e.file_type().map_err(|_| invalid())?.is_dir() {
                for f in fs::read_dir(e.path()).map_err(|_| invalid())? {
                    actual.insert(format!(
                        "{name}/{}",
                        f.map_err(|_| invalid())?
                            .file_name()
                            .to_str()
                            .ok_or_else(invalid)?
                    ));
                }
            } else {
                actual.insert(name);
            }
        }
        if expected != actual {
            return Err(invalid());
        }
        let groups = schema_groups()?;
        let open_catalog = |root: &Path| -> Result<Connection, CommandError> {
            let db = Connection::open_with_flags(
                root.join("catalog.sqlite3"),
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .map_err(|_| invalid())?;
            db.execute_batch("PRAGMA trusted_schema=OFF;")
                .map_err(|_| invalid())?;
            Ok(db)
        };
        // Identity, integrity and foreign keys must hold for
        // the catalog exactly as packaged - for legacy
        // packages this runs BEFORE any staging migration.
        let catalog_version = |db: &Connection| -> Result<i64, CommandError> {
            let version: i64 = db
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .map_err(|_| invalid())?;
            let identity: i64 = db
                .pragma_query_value(None, "application_id", |r| r.get(0))
                .map_err(|_| invalid())?;
            let integrity: String = db
                .query_row("PRAGMA integrity_check", [], |r| r.get(0))
                .map_err(|_| invalid())?;
            let fk_bad = db
                .prepare("PRAGMA foreign_key_check")
                .map_err(|_| invalid())?
                .query([])
                .map_err(|_| invalid())?
                .next()
                .map_err(|_| invalid())?
                .is_some();
            if identity != 0x414e414e || integrity != "ok" || fk_bad {
                return Err(invalid());
            }
            Ok(version)
        };
        let mut db = open_catalog(&temporary.root)?;
        let actual_version = catalog_version(&db)?;
        let optional_current = [&groups.vault, &groups.fts, &groups.embeddings];
        match info.schema_version {
            SCHEMA_VERSION => {
                if actual_version != SCHEMA_VERSION {
                    return Err(invalid());
                }
                check_schema(&db, &groups, &groups.core10, &optional_current)?;
            }
            8 | 9 => {
                // Bounded legacy path: the declared version
                // must agree with the catalog, the legacy
                // structure must be exact, and only then is
                // the isolated staging copy migrated - the
                // package itself is never modified.
                if actual_version != info.schema_version {
                    return Err(invalid());
                }
                if info.schema_version == 9 {
                    check_schema(
                        &db,
                        &groups,
                        &groups.core8,
                        &[&groups.vault, &groups.fts, &groups.embeddings],
                    )?;
                } else {
                    check_schema(
                        &db,
                        &groups,
                        &groups.core8,
                        &[&groups.fts, &groups.embeddings],
                    )?;
                }
                drop(db);
                migrate_staging_catalog(&temporary.root.join("catalog.sqlite3"))?;
                db = open_catalog(&temporary.root)?;
                if catalog_version(&db)? != SCHEMA_VERSION {
                    return Err(invalid());
                }
                check_schema(&db, &groups, &groups.core10, &optional_current)?;
            }
            _ => return Err(invalid()),
        }
        let schema = SCHEMA_VERSION;
        let unsafe_policy:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM backup_schedules WHERE enabled<>0) OR EXISTS(SELECT 1 FROM jobs WHERE state IN ('Queued','Running')) OR EXISTS(SELECT 1 FROM backup_uploads)",[],|r|r.get(0)).map_err(|_|invalid())?;
        if unsafe_policy {
            return Err(invalid());
        }
        if !info.application {
            let unexpected: bool = db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM jobs) OR EXISTS(SELECT 1 FROM backup_schedules)",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| invalid())?;
            if unexpected {
                return Err(invalid());
            }
        }
        let mut query = db
            .prepare("SELECT id FROM workspaces ORDER BY id")
            .map_err(|_| invalid())?;
        let actual = query
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|_| invalid())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| invalid())?;
        let mut claimed = info.workspaces.clone();
        claimed.sort();
        if actual != claimed {
            return Err(invalid());
        }
        let records: u32 = db
            .query_row("SELECT count(*) FROM memories", [], |r| r.get(0))
            .map_err(|_| invalid())?;
        if records > MAX_RECORDS {
            return Err(invalid());
        }
        let revisions: u32 = db
            .query_row("SELECT count(*) FROM memory_revisions", [], |r| r.get(0))
            .map_err(|_| invalid())?;
        let relationships: u32 = db
            .query_row("SELECT count(*) FROM relationships", [], |r| r.get(0))
            .map_err(|_| invalid())?;
        let mut referenced = BTreeSet::from(["catalog.sqlite3".into(), "snapshot.json".into()]);
        let mut query = db
            .prepare("SELECT workspace_id,memory_id,id,metadata,body_hash FROM memory_revisions")
            .map_err(|_| invalid())?;
        let mut rows = query.query([]).map_err(|_| invalid())?;
        while let Some(row) = rows.next().map_err(|_| invalid())? {
            if referenced.len().is_multiple_of(8) {
                tick()?;
            }
            let (w, m, r, raw, hash): (String, String, String, String, String) = (
                row.get(0).map_err(|_| invalid())?,
                row.get(1).map_err(|_| invalid())?,
                row.get(2).map_err(|_| invalid())?,
                row.get(3).map_err(|_| invalid())?,
                row.get(4).map_err(|_| invalid())?,
            );
            for id in [&w, &m, &r] {
                crate::workspace::validate_id(id)?;
            }
            let meta: crate::memory::MemoryMetadata =
                serde_json::from_str(&raw).map_err(|_| invalid())?;
            meta.privacy.allow_plaintext()?;
            if meta.id != m || meta.workspace_id != w || meta.revision_id != r {
                return Err(invalid());
            }
            let path = format!("bodies/{r}.md");
            let body = limited(&temporary.root.join(&path), crate::content::MAX_BODY as u64)?;
            if content::hash(&body) != hash || std::str::from_utf8(&body).is_err() {
                return Err(content::integrity());
            }
            crate::memory::validate(&crate::memory::MemoryDraft {
                title: meta.title.clone(),
                body: std::str::from_utf8(&body).map_err(|_| invalid())?.into(),
                privacy: meta.privacy,
                event_date: meta.event_date.clone(),
                valid_from: meta.valid_from.clone(),
                valid_until: meta.valid_until.clone(),
                confidence: meta.confidence,
                source_references: meta.source_references.clone(),
                project_id: Some(meta.project_id.clone()),
                due_date: meta.due_date.clone(),
                completed: meta.completed,
            })?;
            crate::entities::validate_entity_fields(
                &meta.kind,
                meta.project_id.as_deref(),
                meta.due_date.as_deref(),
                meta.event_date.as_deref(),
            )?;
            for reference in [&meta.project_id, &meta.superseded_by, &meta.supersedes]
                .into_iter()
                .flatten()
            {
                let exists: bool = db
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM memories WHERE workspace_id=?1 AND id=?2)",
                        params![w, reference],
                        |r| r.get(0),
                    )
                    .map_err(|_| invalid())?;
                if !exists {
                    return Err(invalid());
                }
            }
            for reference in &meta.relationships {
                let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM relationships WHERE workspace_id=?1 AND id=?2)",params![w,reference],|r|r.get(0)).map_err(|_|invalid())?;
                if !exists {
                    return Err(invalid());
                }
            }
            referenced.insert(path);
        }
        // Artifact registry rows: structure, integrity and
        // references are verified before anything is imported.
        {
            let mut artifact_keys = BTreeSet::new();
            let mut version_sets: std::collections::BTreeMap<(String, String), Vec<i64>> =
                std::collections::BTreeMap::new();
            let mut pending_refs: Vec<(String, String, i64)> = Vec::new();
            let mut query = db
                .prepare("SELECT workspace_id,artifact_id,project_id,version,artifact_type,body,content_hash,source_references,generator,privacy FROM artifact_versions")
                .map_err(|_| invalid())?;
            let mut rows = query.query([]).map_err(|_| invalid())?;
            while let Some(row) = rows.next().map_err(|_| invalid())? {
                let (w, a, project, v, kind, body, hash, refs_raw, generator, privacy): (
                    String,
                    String,
                    String,
                    i64,
                    String,
                    String,
                    String,
                    String,
                    Option<String>,
                    String,
                ) = (
                    row.get(0).map_err(|_| invalid())?,
                    row.get(1).map_err(|_| invalid())?,
                    row.get(2).map_err(|_| invalid())?,
                    row.get(3).map_err(|_| invalid())?,
                    row.get(4).map_err(|_| invalid())?,
                    row.get(5).map_err(|_| invalid())?,
                    row.get(6).map_err(|_| invalid())?,
                    row.get(7).map_err(|_| invalid())?,
                    row.get(8).map_err(|_| invalid())?,
                    row.get(9).map_err(|_| invalid())?,
                );
                for id in [&w, &a, &project] {
                    crate::workspace::validate_id(id)?;
                }
                if !info.workspaces.contains(&w) || v < 1 {
                    return Err(invalid());
                }
                let project_exists: bool = db
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM memories WHERE workspace_id=?1 AND id=?2)",
                        params![w, project],
                        |r| r.get(0),
                    )
                    .map_err(|_| invalid())?;
                if !project_exists {
                    return Err(invalid());
                }
                if privacy != "standard" && privacy != "private" {
                    return Err(invalid());
                }
                if content::hash(body.as_bytes()) != hash {
                    return Err(content::integrity());
                }
                if let Some(raw) = &generator {
                    let parsed: serde_json::Value =
                        serde_json::from_str(raw).map_err(|_| invalid())?;
                    if !parsed.is_object() {
                        return Err(invalid());
                    }
                }
                let refs: Vec<String> =
                    serde_json::from_str(&refs_raw).map_err(|_| invalid())?;
                for reference in &refs {
                    if let Some(rest) = reference.strip_prefix("artifact://") {
                        let (id, version) = rest.rsplit_once("@v").ok_or_else(invalid)?;
                        crate::workspace::validate_id(id)?;
                        let version: i64 = version.parse().map_err(|_| invalid())?;
                        pending_refs.push((w.clone(), id.to_string(), version));
                    } else if reference.starts_with("artifact:") {
                        return Err(invalid());
                    }
                }
                if kind == "evaluation" {
                    let (source, source_v, rubric, rubric_v) =
                        crate::evaluations::evaluation_references(&body)
                            .ok_or_else(invalid)?;
                    pending_refs.push((w.clone(), source, i64::from(source_v)));
                    pending_refs.push((w.clone(), rubric, i64::from(rubric_v)));
                }
                version_sets
                    .entry((w.clone(), a.clone()))
                    .or_default()
                    .push(v);
                artifact_keys.insert((w, a, v));
            }
            for versions in version_sets.values() {
                let mut sorted = versions.clone();
                sorted.sort_unstable();
                let expected: Vec<i64> = (1..=versions.len() as i64).collect();
                if sorted != expected {
                    return Err(invalid());
                }
            }
            for key in &pending_refs {
                if !artifact_keys.contains(key) {
                    return Err(invalid());
                }
            }
        }
        if info.application {
            let mut jobs = db
                .prepare("SELECT selection,log FROM jobs")
                .map_err(|_| invalid())?;
            let mut entries = jobs.query([]).map_err(|_| invalid())?;
            while let Some(row) = entries.next().map_err(|_| invalid())? {
                let selection: String = row.get(0).map_err(|_| invalid())?;
                let log: String = row.get(1).map_err(|_| invalid())?;
                serde_json::from_str::<ExportSelection>(&selection)
                    .map_err(|_| invalid())?
                    .validate()?;
                if log.len() > 512 {
                    return Err(invalid());
                }
            }
            let mut q=db.prepare("SELECT artifact_id,artifact_hash,artifact_bytes FROM jobs WHERE state='Completed'").map_err(|_|invalid())?;
            let mut rows = q.query([]).map_err(|_| invalid())?;
            while let Some(r) = rows.next().map_err(|_| invalid())? {
                let id: String = r.get(0).map_err(|_| invalid())?;
                crate::workspace::validate_id(&id)?;
                let p = format!("artifacts/{id}.tar");
                let data = limited(&temporary.root.join(&p), MAX_BYTES)?;
                let hash: String = r.get(1).map_err(|_| invalid())?;
                let size: i64 = r.get(2).map_err(|_| invalid())?;
                if content::hash(&data) != hash || i64::try_from(data.len()).ok() != Some(size) {
                    return Err(content::integrity());
                }
                referenced.insert(p);
            }
        }
        if referenced
            != checked
                .manifest
                .members
                .iter()
                .map(|m| m.path.clone())
                .collect()
        {
            return Err(invalid());
        }
        let preview = SnapshotPreview {
            contract_version: 1,
            workspace_id: checked.manifest.workspace_id,
            workspace_name: checked.manifest.workspace_name,
            schema_version: schema,
            application: info.application,
            workspace_count: info.workspaces.len() as u32,
            records,
            revisions,
            relationships,
            total_bytes: bytes.len() as u64,
            sha256: content::hash(bytes),
            privacy: "Plaintext; Sealed and credentials excluded".into(),
            conflicts: vec!["Restore creates fresh IDs; original workspaces are preserved".into()],
        };
        Ok(VerifiedSnapshot {
            stage: temporary,
            info,
            preview,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sha2::Digest;
    #[test]
    fn standalone_catalog_budget_is_checked_before_row_insertion() {
        let source = Connection::open_in_memory().unwrap();
        let target = Connection::open_in_memory().unwrap();
        source.execute_batch("CREATE TABLE workspaces(id TEXT,name TEXT,name_key TEXT); INSERT INTO workspaces VALUES('workspace','title','title')").unwrap();
        target
            .execute_batch("CREATE TABLE workspaces(id TEXT,name TEXT,name_key TEXT)")
            .unwrap();
        let mut budget = MAX_BYTES;
        assert!(copy_rows(&source, &target, "workspaces", "workspace", &mut budget).is_err());
        assert_eq!(
            target
                .query_row("SELECT count(*) FROM workspaces", [], |r| r
                    .get::<_, u32>(0))
                .unwrap(),
            0
        );
    }
    fn archive_members(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut result = std::collections::BTreeMap::new();
        let mut offset = 0;
        while offset + 512 <= bytes.len() && bytes[offset..offset + 512].iter().any(|b| *b != 0) {
            let h = &bytes[offset..offset + 512];
            let end = h[..100].iter().position(|b| *b == 0).unwrap();
            let name = std::str::from_utf8(&h[..end]).unwrap().to_string();
            let size = octal(&h[124..136]).unwrap();
            result.insert(name, bytes[offset + 512..offset + 512 + size].to_vec());
            offset = (offset + 512 + size).div_ceil(512) * 512;
        }
        result
    }
    fn make_archive(root: &Path, members: &std::collections::BTreeMap<String, Vec<u8>>) -> Vec<u8> {
        let path = root.join(content::random_id().unwrap());
        let mut archive = Archive {
            file: File::create(&path).unwrap(),
            bytes: 0,
            hash: Sha256::new(),
        };
        for (name, data) in members {
            archive.member(name, data).unwrap();
        }
        archive.write(&[0; 1024]).unwrap();
        drop(archive);
        fs::read(path).unwrap()
    }
    #[test]
    fn semantic_package_corruption_is_refused_before_any_workspace_import() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        let core = crate::Core::with_storage_root(root.clone());
        let w = core
            .workspace_command(
                "create",
                json!({"contractVersion":1,"name":"Corruption fixture"}),
            )
            .unwrap()["workspace"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"draft":{"title":"Preserved","body":"untouched source"}})).unwrap();
        let store = Storage::new(root).unwrap();
        let job = store
            .enqueue_export(&w, ExportSelection::Backup { application: false })
            .unwrap();
        let lease = store.claim_export().unwrap().unwrap();
        store.execute_backup(&lease).unwrap();
        let completed = store.read_job(&w, &job.id).unwrap();
        let original = limited(
            &directory(&store, &w)
                .unwrap()
                .join(format!("{}.tar", completed.artifact_id.unwrap())),
            MAX_BYTES,
        )
        .unwrap();
        let members = archive_members(&original);
        for mode in [
            "missing",
            "version",
            "traversal",
            "duplicate",
            "hash",
            "schema",
            "trigger",
            "sealed",
            "activeSchedule",
            "oversized",
        ] {
            let mut changed = members.clone();
            let mut manifest: BackupManifest =
                serde_json::from_slice(&changed["manifest.json"]).unwrap();
            match mode {
                "missing" => {
                    changed.remove("snapshot.json");
                }
                "version" => manifest.format_version = 99,
                "traversal" => manifest.members[0].path = "../escape".into(),
                "duplicate" => manifest.members.push(manifest.members[0].clone()),
                "hash" => manifest.members[0].sha256 = "0".repeat(64),
                "schema" | "trigger" | "sealed" | "activeSchedule" => {
                    let path = temp.path().join(content::random_id().unwrap());
                    fs::write(&path, &changed["catalog.sqlite3"]).unwrap();
                    let db = Connection::open(&path).unwrap();
                    match mode {
                        "schema"=>db.execute_batch("PRAGMA user_version=99").unwrap(),
                        "trigger"=>db.execute_batch("CREATE TRIGGER unexpected AFTER INSERT ON memories BEGIN SELECT 1; END").unwrap(),
                        "activeSchedule"=>{db.execute("INSERT INTO backup_schedules(workspace_id,enabled,interval_seconds,next_due,consent_version) VALUES(?1,1,3600,0,1)",[&w]).unwrap();},
                        _=>{db.execute("UPDATE memory_revisions SET metadata=json_set(metadata,'$.privacy','sealed')",[]).unwrap();}
                    }
                    drop(db);
                    let data = fs::read(path).unwrap();
                    let member = manifest
                        .members
                        .iter_mut()
                        .find(|m| m.path == "catalog.sqlite3")
                        .unwrap();
                    member.bytes = data.len() as u64;
                    member.sha256 = content::hash(&data);
                    changed.insert("catalog.sqlite3".into(), data);
                }
                _ => {}
            }
            changed.insert(
                "manifest.json".into(),
                if mode == "oversized" {
                    vec![b' '; crate::backup::MAX_MANIFEST_BYTES as usize + 1]
                } else {
                    serde_json::to_vec(&manifest).unwrap()
                },
            );
            let bytes = make_archive(temp.path(), &changed);
            assert!(
                store.verify_snapshot_bytes(&bytes).is_err(),
                "{mode} must be refused"
            );
            assert_eq!(store.list().unwrap().len(), 1);
        }
        assert_eq!(
            content::hash(
                &limited(
                    &directory(&store, &w).unwrap().join(format!(
                        "{}.tar",
                        store.read_job(&w, &job.id).unwrap().artifact_id.unwrap()
                    )),
                    MAX_BYTES
                )
                .unwrap()
            ),
            content::hash(&original)
        );
    }
    #[test]
    fn cancelled_or_interrupted_backup_never_publishes_and_retry_is_explicit() {
        use crate::jobs::{JobAction, JobState};
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        let store = Storage::new(root).unwrap();
        let w = store.create("Failure fixture").unwrap().workspace.id;
        let job = store
            .enqueue_export(&w, ExportSelection::Backup { application: false })
            .unwrap();
        let lease = store.claim_export().unwrap().unwrap();
        let result = store.execute_backup_with_snapshot_hook(&lease, || {
            let current = store.read_job(&w, &job.id)?;
            store.control_job(&w, &job.id, current.version, JobAction::Cancel)?;
            Ok(())
        });
        assert!(result.is_err());
        let cancelled = store.read_job(&w, &job.id).unwrap();
        assert_eq!(cancelled.state, JobState::Cancelled);
        assert!(cancelled.artifact_id.is_none());
        store
            .control_job(&w, &job.id, cancelled.version, JobAction::Retry)
            .unwrap();
        let lease = store.claim_export().unwrap().unwrap();
        assert!(
            store
                .execute_backup_with_snapshot_hook(&lease, || Err(unavailable()))
                .is_err()
        );
        store.fail_export(&lease).unwrap();
        let failed = store.read_job(&w, &job.id).unwrap();
        assert_eq!(failed.state, JobState::Failed);
        assert!(failed.artifact_id.is_none());
        store
            .control_job(&w, &job.id, failed.version, JobAction::Retry)
            .unwrap();
        let lease = store.claim_export().unwrap().unwrap();
        store.execute_backup(&lease).unwrap();
        assert_eq!(
            store.read_job(&w, &job.id).unwrap().state,
            JobState::Completed
        );
    }
    #[test]
    fn snapshot_is_consistent_during_concurrent_edit_and_saved_file_recovers_in_fresh_catalog() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("original");
        let core = crate::Core::with_storage_root(root.clone());
        let w = core
            .workspace_command(
                "create",
                json!({"contractVersion":1,"name":"Snapshot consistency"}),
            )
            .unwrap()["workspace"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let first=core.memory_command("create",json!({"contractVersion":1,"workspaceId":w,"draft":{"title":"Snapshot fixture","body":"before concurrent edit"}})).unwrap();
        let store = Storage::new(root.clone()).unwrap();
        let job = store
            .enqueue_export(&w, ExportSelection::Backup { application: false })
            .unwrap();
        let lease = store.claim_export().unwrap().unwrap();
        store.execute_backup_with_snapshot_hook(&lease,||{
            core.memory_command("revise",json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"],"expectedRevision":first["memory"]["revisionId"],"draft":{"title":"Snapshot fixture","body":"after concurrent edit"}}))?;Ok(())
        }).unwrap();
        let completed = store.read_job(&w, &job.id).unwrap();
        let artifact = completed.artifact_id.unwrap();
        let snapshot = store.verify_snapshot(&w, &job.id, &artifact).unwrap();
        assert_eq!(snapshot.preview.revisions, 1);
        let data = limited(
            &directory(&store, &w)
                .unwrap()
                .join(format!("{artifact}.tar")),
            MAX_BYTES,
        )
        .unwrap();
        let fresh = crate::Core::with_storage_root(temp.path().join("fresh"));
        let recovery = fresh
            .workspace_command(
                "create",
                json!({"contractVersion":1,"name":"Recovery landing"}),
            )
            .unwrap()["workspace"]["id"]
            .clone();
        let begin=fresh.job_command("upload-begin",json!({"contractVersion":1,"workspaceId":recovery,"bytes":data.len(),"sha256":content::hash(&data),"plaintextConsent":true})).unwrap();
        let mut result = serde_json::Value::Null;
        for (index, chunk) in data.chunks(65536).enumerate() {
            let offset = index * 65536;
            result=fresh.job_command("upload-part",json!({"contractVersion":1,"workspaceId":recovery,"id":begin["id"],"offset":offset,"bytes":chunk,"finish":offset+chunk.len()==data.len()})).unwrap();
        }
        let imported = &result["job"];
        let preview=fresh.job_command("preview",json!({"contractVersion":1,"workspaceId":recovery,"jobId":imported["id"],"artifactId":imported["artifactId"]})).unwrap();
        let restored=fresh.job_command("restore",json!({"contractVersion":1,"workspaceId":recovery,"jobId":imported["id"],"artifactId":imported["artifactId"],"name":"Independent recovered","expectedHash":preview["sha256"],"confirm":true})).unwrap();
        let new = restored["workspaces"][0]["id"].clone();
        let list = fresh
            .memory_command("list", json!({"contractVersion":1,"workspaceId":new}))
            .unwrap();
        let read = fresh
            .memory_command(
                "read",
                json!({"contractVersion":1,"workspaceId":new,"id":list["memories"][0]["id"]}),
            )
            .unwrap();
        assert_eq!(read["body"], "before concurrent edit");
        let original = core
            .memory_command(
                "read",
                json!({"contractVersion":1,"workspaceId":w,"id":first["memory"]["id"]}),
            )
            .unwrap();
        assert_eq!(original["body"], "after concurrent edit");
    }
    #[test]
    fn strict_tar_refuses_traversal_duplicates_links_and_incomplete_archives() {
        let temp = tempfile::tempdir().unwrap();
        for name in [
            "../escape",
            "/absolute",
            "bodies/../../x",
            "bodies/C:evil.md",
            "manifest.json",
        ] {
            let file = temp.path().join(crate::content::random_id().unwrap());
            let mut archive = Archive {
                file: File::create(&file).unwrap(),
                bytes: 0,
                hash: Sha256::new(),
            };
            if name == "manifest.json" {
                archive.member(name, b"{}").unwrap();
                archive.member(name, b"{}").unwrap();
            } else if archive.member(name, b"x").is_err() {
                continue;
            }
            archive.write(&[0; 1024]).unwrap();
            drop(archive);
            let bytes = fs::read(file).unwrap();
            let target = temp.path().join(crate::content::random_id().unwrap());
            fs::create_dir(&target).unwrap();
            assert!(unpack(&bytes, &target).is_err());
        }
        assert!(unpack(&[0; 512], temp.path()).is_err());
        let mut members = std::collections::BTreeMap::new();
        members.insert(format!("bodies/{}.md", "a".repeat(32)), b"x".to_vec());
        let regular = make_archive(temp.path(), &members);
        for flag in *b"125x" {
            let mut linked = regular.clone();
            linked[156] = flag;
            linked[148..156].fill(b' ');
            let sum: u32 = linked[..512].iter().map(|b| u32::from(*b)).sum();
            linked[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
            assert!(
                unpack(&linked, temp.path()).is_err(),
                "link/directory/extension flag must be rejected"
            );
        }
    }
}
