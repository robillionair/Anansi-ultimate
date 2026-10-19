//! Explicit schema-9 core boundary. Never called by schema-8 application adapters.
use super::{
    Result, VaultError,
    admission::Admission,
    envelope::{self, KdfProfile, ParsedEnvelope, Purpose},
    io::{VerifiedFiles, write_cipher},
    keys::{Context, KeySlots},
    migration::read_bounded,
    payload::{Origin, RecordPayload, Snapshot, id, invalid},
    recovery::{AcknowledgedSetup, PendingSetup},
    session::{Caller, Sessions},
};
use crate::{
    content,
    storage::{DATABASE_NAME, Storage, private_permissions, reject_links},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    PreJournal,
    MidStage,
    PreSwitch,
    PostSwitch,
    PreRollback,
    PostRollback,
}
pub struct BackupReference {
    pub job_id: String,
    pub artifact_id: String,
    pub sha256: String,
}
pub struct VaultStore {
    pub(crate) root: PathBuf,
    pub(crate) host_namespace: PathBuf,
}
pub(crate) struct Generation {
    pub context: Context,
    pub source: String,
    pub manifest_hash: String,
    pub slots: KeySlots,
    pub epoch: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MemberEntry {
    pub token: String,
    pub record_id: String,
    pub revision_id: String,
    pub envelope_hash: String,
    pub byte_length: usize,
    pub source_hash: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Manifest {
    pub version: u8,
    pub purpose: String,
    pub workspace_id: String,
    pub vault_id: String,
    pub source_generation_id: String,
    pub target_generation_id: String,
    pub journal_id: String,
    pub entries: Vec<MemberEntry>,
    pub current_revisions: BTreeMap<String, String>,
    pub graph: Vec<crate::relationships::RelationshipRecord>,
    pub current_edges: BTreeMap<String, u32>,
    pub origin: Option<Origin>,
}

impl Drop for Manifest {
    fn drop(&mut self) {
        self.graph.iter_mut().for_each(super::payload::wipe_edge);
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestWrite<'a> {
    version: u8,
    purpose: &'a str,
    workspace_id: &'a str,
    vault_id: &'a str,
    source_generation_id: &'a str,
    target_generation_id: &'a str,
    journal_id: String,
    entries: &'a [MemberEntry],
    current_revisions: &'a BTreeMap<String, String>,
    graph: &'a [crate::relationships::RelationshipRecord],
    current_edges: &'a BTreeMap<String, u32>,
    origin: &'a Option<Origin>,
}

impl VaultStore {
    pub fn new(root: PathBuf, host_namespace: PathBuf) -> Result<Self> {
        let storage = Storage::new(root.clone()).map_err(invalid)?;
        if !root.join(".anansi-root").exists() {
            storage.connection(true).map_err(invalid)?;
        }
        if !root.is_dir() || !host_namespace.is_absolute() {
            return Err(VaultError::StorageUnavailable);
        }
        private_permissions(&root, true, true).map_err(invalid)?;
        let marker = root.join(".anansi-root");
        reject_links(&marker).map_err(invalid)?;
        if read_bounded(&marker, 64)? != b"ANANSI workspace root v1\n" {
            return Err(VaultError::InvalidContext);
        }
        Ok(Self {
            root,
            host_namespace,
        })
    }
    pub fn root_id(&self) -> Result<String> {
        Ok(content::hash(
            self.root
                .canonicalize()
                .map_err(invalid)?
                .to_string_lossy()
                .as_bytes(),
        ))
    }
    pub(crate) fn connection(&self) -> Result<Connection> {
        let db = self.root.join(DATABASE_NAME);
        for path in [
            db.clone(),
            self.root.join(format!("{DATABASE_NAME}-wal")),
            self.root.join(format!("{DATABASE_NAME}-shm")),
            self.root.join(format!("{DATABASE_NAME}-journal")),
        ] {
            reject_links(&path).map_err(invalid)?;
        }
        private_permissions(&db, false, true).map_err(invalid)?;
        let probe = Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(invalid)?;
        let version: i64 = probe
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(invalid)?;
        let app: i64 = probe
            .pragma_query_value(None, "application_id", |r| r.get(0))
            .map_err(invalid)?;
        if app != 0x414e414e || ![8, 9].contains(&version) {
            return Err(VaultError::UnsupportedSchema);
        }
        drop(probe);
        let db = Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(invalid)?;
        db.busy_timeout(Duration::from_secs(5)).map_err(invalid)?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA synchronous=FULL;",
        )
        .map_err(invalid)?;
        let integrity: String = db
            .query_row("PRAGMA quick_check(1)", [], |r| r.get(0))
            .map_err(invalid)?;
        if integrity != "ok"
            || db
                .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
                .optional()
                .map_err(invalid)?
                .is_some()
        {
            return Err(VaultError::InvalidEnvelope);
        }
        Ok(db)
    }
    pub fn begin(&self, workspace: &str, password: &[u8]) -> Result<PendingSetup> {
        id(workspace)?;
        let _root = Admission::root(&self.root)?;
        let _kdf = Admission::kdf(&self.host_namespace)?;
        let db = self.connection()?;
        if !db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id=?1)",
                [workspace],
                |r| r.get::<_, bool>(0),
            )
            .map_err(invalid)?
        {
            return Err(VaultError::InvalidContext);
        }
        let context = Context::new(
            &content::random_id().map_err(invalid)?,
            workspace,
            &content::random_id().map_err(invalid)?,
        )?;
        PendingSetup::new(context, password, self.root_id()?)
    }
    pub fn seal_workspace(
        &self,
        setup: AcknowledgedSetup,
        backup: &BackupReference,
        retain_plaintext_backup: bool,
    ) -> Result<()> {
        self.seal_observed(setup, backup, retain_plaintext_backup, |_| Ok(()))
    }
    pub fn seal_observed(
        &self,
        setup: AcknowledgedSetup,
        backup: &BackupReference,
        retain_plaintext_backup: bool,
        mut observe: impl FnMut(Phase) -> Result<()>,
    ) -> Result<()> {
        if !retain_plaintext_backup || setup.root_id != self.root_id()? {
            return Err(VaultError::AcknowledgementRequired);
        }
        let _root = Admission::root(&self.root)?;
        let storage = Storage::new(self.root.clone()).map_err(invalid)?;
        let verified = storage
            .verify_snapshot(
                setup.context.workspace_id(),
                &backup.job_id,
                &backup.artifact_id,
            )
            .map_err(invalid)?;
        if verified.preview.sha256 != backup.sha256 {
            return Err(VaultError::Conflict);
        }
        let mut db = self.connection()?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(invalid)?;
        if tx
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .map_err(invalid)?
            != 8
        {
            return Err(VaultError::UnsupportedSchema);
        }
        let snapshot = Snapshot::capture(&tx, &self.root, &setup.context, false)?;
        let old = Connection::open_with_flags(
            verified.stage.root.join("catalog.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(invalid)?;
        let recovered = Snapshot::capture(&old, &verified.stage.root, &setup.context, true)?;
        if snapshot.fingerprint()? != recovered.fingerprint()? {
            return Err(VaultError::Conflict);
        }
        let manifest_hash = self.stage(
            &snapshot,
            &setup.context,
            &backup.artifact_id,
            &setup.dek,
            &mut observe,
        )?;
        Self::upgrade(&tx)?;
        tx.execute("UPDATE vault_maintenance SET enabled=1", [])
            .map_err(invalid)?;
        // Deferred circular record/revision pointers are checked at commit.
        tx.execute_batch("PRAGMA defer_foreign_keys=ON;")
            .map_err(invalid)?;
        for table in [
            "relationship_revisions",
            "relationships",
            "memory_revisions",
            "memories",
        ] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE workspace_id=?1"),
                [setup.context.workspace_id()],
            )
            .map_err(invalid)?;
        }
        tx.execute("UPDATE vault_maintenance SET enabled=0", [])
            .map_err(invalid)?;
        self.publish_new(&tx, &setup, &backup.artifact_id, &manifest_hash, &snapshot)?;
        observe(Phase::PreSwitch)?;
        tx.commit().map_err(invalid)?;
        observe(Phase::PostSwitch)?;
        Ok(())
    }
    pub(crate) fn upgrade(db: &Connection) -> Result<()> {
        let version: i64 = db
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(invalid)?;
        if version == 8 {
            db.execute_batch(include_str!("../../migrations/0009_vault.sql"))
                .map_err(invalid)?;
            db.pragma_update(None, "user_version", 9).map_err(invalid)?;
        } else if version != 9 {
            return Err(VaultError::UnsupportedSchema);
        }
        Ok(())
    }
    pub(crate) fn publish_new(
        &self,
        db: &Connection,
        setup: &AcknowledgedSetup,
        source: &str,
        hash: &str,
        snapshot: &Snapshot,
    ) -> Result<()> {
        let context = &setup.context;
        db.execute("INSERT INTO vaults(workspace_id,vault_id,active_generation,epoch,recovery_acknowledged) VALUES(?1,?2,?3,1,1)",params![context.workspace_id(),context.vault_id(),context.generation_id()]).map_err(invalid)?;
        self.add_generation(
            db,
            &Generation {
                context: context.clone(),
                source: source.into(),
                manifest_hash: hash.into(),
                slots: KeySlots {
                    passphrase: setup.slots.passphrase.clone(),
                    recovery: setup.slots.recovery.clone(),
                },
                epoch: 1,
            },
            snapshot,
            &setup.dek,
        )
    }
    fn add_generation(
        &self,
        db: &Connection,
        descriptor: &Generation,
        snapshot: &Snapshot,
        key: &[u8; 32],
    ) -> Result<()> {
        let context = &descriptor.context;
        let slots = &descriptor.slots;
        let directory = self.directory(context, false)?;
        let bytes = read_bounded(
            &directory.join("manifest.ansv"),
            envelope::MAX_JOURNAL_BYTES + 80,
        )?;
        if content::hash(&bytes) != descriptor.manifest_hash {
            return Err(VaultError::AuthenticationFailed);
        }
        let plain = ParsedEnvelope::parse(&bytes, Purpose::Journal)?.open(key)?;
        super::json::validate(&plain, envelope::MAX_JOURNAL_BYTES)?;
        let manifest: Manifest = serde_json::from_slice(&plain).map_err(invalid)?;
        db.execute(
            "INSERT INTO vault_generations VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                context.vault_id(),
                context.generation_id(),
                descriptor.source,
                descriptor.manifest_hash,
                slots.passphrase,
                slots.recovery
            ],
        )
        .map_err(invalid)?;
        for record in snapshot.current_revisions.keys() {
            db.execute(
                "INSERT INTO vault_members VALUES(?1,?2,?3)",
                params![context.vault_id(), context.generation_id(), record],
            )
            .map_err(invalid)?;
        }
        db.execute(
            "INSERT INTO vault_files VALUES(?1,?2,'manifest',?3,?4)",
            params![
                context.vault_id(),
                context.generation_id(),
                descriptor.manifest_hash,
                bytes.len() as i64
            ],
        )
        .map_err(invalid)?;
        for entry in &manifest.entries {
            db.execute(
                "INSERT INTO vault_files VALUES(?1,?2,?3,?4,?5)",
                params![
                    context.vault_id(),
                    context.generation_id(),
                    entry.token,
                    entry.envelope_hash,
                    entry.byte_length as i64
                ],
            )
            .map_err(invalid)?;
        }
        Ok(())
    }
    pub(crate) fn generation(
        db: &Connection,
        workspace: &str,
        generation: Option<&str>,
    ) -> Result<Generation> {
        id(workspace)?;
        let row=db.query_row("SELECT v.vault_id,g.generation_id,g.source_generation_id,g.manifest_hash,g.passphrase_slot,g.recovery_slot,v.epoch FROM vaults v JOIN vault_generations g ON g.vault_id=v.vault_id AND g.generation_id=COALESCE(?2,v.active_generation) WHERE v.workspace_id=?1",params![workspace,generation],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,Vec<u8>>(4)?,r.get::<_,Vec<u8>>(5)?,r.get::<_,i64>(6)?))).map_err(invalid)?;
        Ok(Generation {
            context: Context::new(&row.0, workspace, &row.1)?,
            source: row.2,
            manifest_hash: row.3,
            slots: KeySlots {
                passphrase: row.4,
                recovery: row.5,
            },
            epoch: u64::try_from(row.6).map_err(invalid)?,
        })
    }
    pub fn context(&self, workspace: &str) -> Result<(Context, u64)> {
        let _root = Admission::root(&self.root)?;
        let descriptor = Self::generation(&self.connection()?, workspace, None)?;
        Ok((descriptor.context, descriptor.epoch))
    }
    pub fn unlock(
        &self,
        sessions: &mut Sessions,
        caller: &Caller,
        password: &[u8],
        now: Instant,
    ) -> Result<Zeroizing<[u8; 16]>> {
        let _root = Admission::root(&self.root)?;
        let _kdf = Admission::kdf(&self.host_namespace)?;
        let db = self.connection()?;
        let descriptor = Self::generation(&db, caller.context.workspace_id(), None)?;
        if descriptor.context != caller.context || caller.root_id != self.root_id()? {
            return Err(VaultError::InvalidContext);
        }
        let dek = descriptor
            .slots
            .unlock(&descriptor.context, password, false)?;
        self.load_generation(&descriptor, &dek)?;
        sessions.establish(caller.clone(), dek, descriptor.epoch, now)
    }
    pub fn read(
        &self,
        sessions: &mut Sessions,
        caller: &Caller,
        token: &[u8; 16],
        now: Instant,
    ) -> Result<Snapshot> {
        let started = Instant::now();
        let _root = Admission::root(&self.root)?;
        let db = self.connection()?;
        let descriptor = Self::generation(&db, caller.context.workspace_id(), None)?;
        if descriptor.context != caller.context || caller.root_id != self.root_id()? {
            sessions.lock();
            return Err(VaultError::Locked);
        }
        let snapshot = sessions.authorize(caller, token, descriptor.epoch, now, |key| {
            self.load_generation(&descriptor, key)
        })?;
        sessions.authorize(
            caller,
            token,
            descriptor.epoch,
            now.checked_add(started.elapsed())
                .ok_or(VaultError::Locked)?,
            |_| Ok(()),
        )?;
        Ok(snapshot)
    }
    pub fn lock(&self, workspace: &str, sessions: &mut Sessions) -> Result<u64> {
        let _root = Admission::root(&self.root)?;
        let mut db = self.connection()?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(invalid)?;
        Self::bump(&tx, workspace)?;
        let epoch = Self::generation(&tx, workspace, None)?.epoch;
        tx.commit().map_err(invalid)?;
        sessions.lock();
        Ok(epoch)
    }
    fn bump(db: &Connection, workspace: &str) -> Result<()> {
        if db.execute("UPDATE vaults SET epoch=epoch+1 WHERE workspace_id=?1 AND epoch<9223372036854775807",[workspace]).map_err(invalid)?!=1 { return Err(VaultError::Conflict); }
        Ok(())
    }
    pub fn rotate(
        &self,
        sessions: &mut Sessions,
        caller: &Caller,
        token: &[u8; 16],
        password: &[u8],
        now: Instant,
    ) -> Result<PendingRotation> {
        let _root = Admission::root(&self.root)?;
        let _kdf = Admission::kdf(&self.host_namespace)?;
        let db = self.connection()?;
        let descriptor = Self::generation(&db, caller.context.workspace_id(), None)?;
        if descriptor.context != caller.context || caller.root_id != self.root_id()? {
            return Err(VaultError::Locked);
        }
        let snapshot = sessions.authorize(caller, token, descriptor.epoch, now, |key| {
            self.load_generation(&descriptor, key)
        })?;
        let context = Context::new(
            descriptor.context.vault_id(),
            descriptor.context.workspace_id(),
            &content::random_id().map_err(invalid)?,
        )?;
        let pending = PendingSetup::new(context.clone(), password, self.root_id()?)?;
        Ok(PendingRotation {
            pending: Some(pending),
            snapshot,
            expected_epoch: descriptor.epoch,
            source: descriptor.context.generation_id().into(),
            target: context,
            caller: caller.clone(),
            token: Zeroizing::new(*token),
        })
    }
    pub fn finish_rotation(
        &self,
        rotation: AcknowledgedRotation,
        sessions: &mut Sessions,
        now: Instant,
        mut observe: impl FnMut(Phase) -> Result<()>,
    ) -> Result<()> {
        let started = Instant::now();
        let AcknowledgedRotation { pending, ack } = rotation;
        if ack.root_id != self.root_id()? {
            return Err(VaultError::InvalidContext);
        }
        let _root = Admission::root(&self.root)?;
        let mut db = self.connection()?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(invalid)?;
        let descriptor = Self::generation(&tx, ack.context.workspace_id(), None)?;
        if descriptor.epoch != pending.expected_epoch
            || descriptor.context.generation_id() != pending.source
            || descriptor.context.vault_id() != ack.context.vault_id()
            || pending.target != ack.context
        {
            return Err(VaultError::Conflict);
        }
        sessions.authorize(
            &pending.caller,
            &pending.token,
            descriptor.epoch,
            now,
            |_| Ok(()),
        )?;
        let mut snapshot = pending.snapshot;
        snapshot.rebind(&ack.context);
        snapshot.validate(&ack.context)?;
        let hash = self.stage(
            &snapshot,
            &ack.context,
            &pending.source,
            &ack.dek,
            &mut observe,
        )?;
        self.add_generation(
            &tx,
            &Generation {
                context: ack.context.clone(),
                source: pending.source.clone(),
                manifest_hash: hash,
                slots: KeySlots {
                    passphrase: ack.slots.passphrase.clone(),
                    recovery: ack.slots.recovery.clone(),
                },
                epoch: descriptor.epoch,
            },
            &snapshot,
            &ack.dek,
        )?;
        tx.execute(
            "UPDATE vaults SET active_generation=?2 WHERE workspace_id=?1",
            params![ack.context.workspace_id(), ack.context.generation_id()],
        )
        .map_err(invalid)?;
        Self::bump(&tx, ack.context.workspace_id())?;
        observe(Phase::PreSwitch)?;
        sessions.authorize(
            &pending.caller,
            &pending.token,
            descriptor.epoch,
            now.checked_add(started.elapsed())
                .ok_or(VaultError::Locked)?,
            |_| Ok(()),
        )?;
        tx.commit().map_err(invalid)?;
        sessions.lock();
        observe(Phase::PostSwitch)?;
        Ok(())
    }
    pub fn rollback(
        &self,
        workspace: &str,
        generation: &str,
        recovery: &[u8; 32],
        sessions: &mut Sessions,
        mut observe: impl FnMut(Phase) -> Result<()>,
    ) -> Result<()> {
        id(generation)?;
        let _root = Admission::root(&self.root)?;
        let mut db = self.connection()?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(invalid)?;
        let descriptor = Self::generation(&tx, workspace, Some(generation))?;
        let key = descriptor.slots.recover(&descriptor.context, recovery)?;
        self.load_generation(&descriptor, &key)?;
        observe(Phase::PreRollback)?;
        tx.execute(
            "UPDATE vaults SET active_generation=?2 WHERE workspace_id=?1",
            params![workspace, generation],
        )
        .map_err(invalid)?;
        Self::bump(&tx, workspace)?;
        tx.commit().map_err(invalid)?;
        sessions.lock();
        observe(Phase::PostRollback)?;
        Ok(())
    }
    pub(crate) fn directory(&self, context: &Context, create: bool) -> Result<PathBuf> {
        let mut path = self.root.clone();
        for part in [
            "vault-generations",
            context.vault_id(),
            context.generation_id(),
        ] {
            path.push(part);
            reject_links(&path).map_err(invalid)?;
            if create && !path.exists() {
                std::fs::create_dir(&path).map_err(invalid)?;
                private_permissions(&path, true, false).map_err(invalid)?;
            }
            private_permissions(&path, true, true).map_err(invalid)?;
        }
        Ok(path)
    }
    pub(crate) fn stage(
        &self,
        snapshot: &Snapshot,
        context: &Context,
        source: &str,
        key: &[u8; 32],
        observe: &mut impl FnMut(Phase) -> Result<()>,
    ) -> Result<String> {
        snapshot.validate(context)?;
        id(source)?;
        let directory = self.directory(context, true)?;
        if std::fs::read_dir(&directory)
            .map_err(invalid)?
            .next()
            .is_some()
        {
            return Err(VaultError::Conflict);
        }
        let mut entries = Vec::new();
        let mut protected_files = Vec::new();
        let mut total = 0usize;
        for (i, record) in snapshot.records.iter().enumerate() {
            let plain = record.encode(context)?;
            total = total
                .checked_add(plain.len())
                .ok_or(VaultError::ResourceLimit)?;
            if total > envelope::MAX_ARCHIVE_BYTES {
                return Err(VaultError::ResourceLimit);
            }
            let token = content::random_id().map_err(invalid)?;
            let salt = envelope::random::<16>()?;
            let bytes = envelope::seal(key, Purpose::Record, &plain, KdfProfile::default(), &salt)?;
            let path = directory.join(format!("{token}.ansv"));
            write_cipher(&path, &bytes)?;
            protected_files.push((path, false));
            entries.push(MemberEntry {
                token,
                record_id: record.record_id.clone(),
                revision_id: record.revision_id.clone(),
                envelope_hash: content::hash(&bytes),
                byte_length: bytes.len(),
                source_hash: content::hash(&plain),
            });
            if i == snapshot.records.len() / 2 {
                observe(Phase::MidStage)?;
            }
        }
        observe(Phase::PreJournal)?;
        // Serialize graph only inside guarded manifest plaintext; no SQL graph shadow.
        let manifest = ManifestWrite {
            version: 1,
            purpose: "vault_migration",
            workspace_id: context.workspace_id(),
            vault_id: context.vault_id(),
            source_generation_id: source,
            target_generation_id: context.generation_id(),
            journal_id: content::random_id().map_err(invalid)?,
            entries: &entries,
            current_revisions: &snapshot.current_revisions,
            graph: &snapshot.graph,
            current_edges: &snapshot.current_edges,
            origin: &snapshot.origin,
        };
        let plain = Zeroizing::new(serde_json::to_vec(&manifest).map_err(invalid)?);
        if plain.len() > envelope::MAX_JOURNAL_BYTES
            || total
                .checked_add(plain.len())
                .is_none_or(|n| n > envelope::MAX_ARCHIVE_BYTES)
        {
            return Err(VaultError::ResourceLimit);
        }
        let salt = envelope::random::<16>()?;
        let bytes = envelope::seal(key, Purpose::Journal, &plain, KdfProfile::default(), &salt)?;
        let path = directory.join("manifest.ansv");
        write_cipher(&path, &bytes)?;
        protected_files.push((path, false));
        VerifiedFiles::verify(&protected_files, true)?;
        let hash = content::hash(&bytes);
        let probe = Generation {
            context: context.clone(),
            source: source.into(),
            manifest_hash: hash.clone(),
            slots: KeySlots {
                passphrase: Vec::new(),
                recovery: Vec::new(),
            },
            epoch: 0,
        };
        self.load_generation(&probe, key)?;
        Ok(hash)
    }
    pub(crate) fn load_generation(
        &self,
        descriptor: &Generation,
        key: &[u8; 32],
    ) -> Result<Snapshot> {
        let directory = self.directory(&descriptor.context, false)?;
        let bytes = read_bounded(
            &directory.join("manifest.ansv"),
            envelope::MAX_JOURNAL_BYTES + 80,
        )?;
        if content::hash(&bytes) != descriptor.manifest_hash {
            return Err(VaultError::AuthenticationFailed);
        }
        let plain = ParsedEnvelope::parse(&bytes, Purpose::Journal)?.open(key)?;
        super::json::validate(&plain, envelope::MAX_JOURNAL_BYTES)?;
        let manifest: Manifest = serde_json::from_slice(&plain).map_err(invalid)?;
        if manifest.entries.len() > 100_000 {
            return Err(VaultError::ResourceLimit);
        }
        let mut paths = Vec::new();
        for entry in &manifest.entries {
            id(&entry.token)?;
            paths.push((directory.join(format!("{}.ansv", entry.token)), false));
        }
        let verified = VerifiedFiles::verify(&paths, false)?;
        Self::decode_generation(descriptor, key, &bytes, |token| {
            verified.read(
                &directory.join(format!("{token}.ansv")),
                envelope::MAX_RECORD_BYTES + 80,
            )
        })
    }

    pub(crate) fn decode_generation(
        descriptor: &Generation,
        key: &[u8; 32],
        bytes: &[u8],
        mut member: impl FnMut(&str) -> Result<Vec<u8>>,
    ) -> Result<Snapshot> {
        let plain = ParsedEnvelope::parse(bytes, Purpose::Journal)?.open(key)?;
        super::json::validate(&plain, envelope::MAX_JOURNAL_BYTES)?;
        let mut manifest: Manifest = serde_json::from_slice(&plain).map_err(invalid)?;
        if manifest.version != 1
            || manifest.purpose != "vault_migration"
            || manifest.workspace_id != descriptor.context.workspace_id()
            || manifest.vault_id != descriptor.context.vault_id()
            || manifest.target_generation_id != descriptor.context.generation_id()
            || manifest.source_generation_id != descriptor.source
            || manifest.entries.len() + manifest.graph.len() > 100_000
        {
            return Err(VaultError::InvalidContext);
        }
        id(&manifest.journal_id)?;
        let mut tokens = BTreeSet::new();
        let mut records = Vec::new();
        let mut total = plain.len();
        for entry in &manifest.entries {
            id(&entry.token)?;
            id(&entry.record_id)?;
            id(&entry.revision_id)?;
            if !tokens.insert(&entry.token) || entry.byte_length > envelope::MAX_RECORD_BYTES + 80 {
                return Err(VaultError::InvalidEnvelope);
            }
            let bytes = member(&entry.token)?;
            if bytes.len() != entry.byte_length || content::hash(&bytes) != entry.envelope_hash {
                return Err(VaultError::AuthenticationFailed);
            }
            let plain = ParsedEnvelope::parse(&bytes, Purpose::Record)?.open(key)?;
            total = total
                .checked_add(plain.len())
                .ok_or(VaultError::ResourceLimit)?;
            if total > envelope::MAX_ARCHIVE_BYTES || content::hash(&plain) != entry.source_hash {
                return Err(VaultError::ResourceLimit);
            }
            let payload = RecordPayload::decode(&plain, &descriptor.context)?;
            if payload.record_id != entry.record_id || payload.revision_id != entry.revision_id {
                return Err(VaultError::InvalidContext);
            }
            records.push(payload);
        }
        let result = Snapshot {
            version: 1,
            workspace_id: std::mem::take(&mut manifest.workspace_id),
            records,
            current_revisions: std::mem::take(&mut manifest.current_revisions),
            graph: std::mem::take(&mut manifest.graph),
            current_edges: std::mem::take(&mut manifest.current_edges),
            origin: manifest.origin.take(),
        };
        result.validate(&descriptor.context)?;
        Ok(result)
    }
}
pub struct PendingRotation {
    pending: Option<PendingSetup>,
    pub(crate) snapshot: Snapshot,
    expected_epoch: u64,
    source: String,
    target: Context,
    caller: Caller,
    token: Zeroizing<[u8; 16]>,
}
pub struct AcknowledgedRotation {
    pending: PendingRotation,
    ack: AcknowledgedSetup,
}
impl PendingRotation {
    pub fn display_once(&mut self) -> Result<super::recovery::RecoveryDisplay> {
        self.pending
            .as_mut()
            .ok_or(VaultError::AcknowledgementRequired)?
            .display_once()
    }
    pub fn acknowledge(
        mut self,
        typed: &str,
        challenge: &[u8; 16],
        saved: bool,
    ) -> Result<AcknowledgedRotation> {
        let ack = self
            .pending
            .take()
            .ok_or(VaultError::AcknowledgementRequired)?
            .acknowledge(typed, challenge, saved)?;
        Ok(AcknowledgedRotation { pending: self, ack })
    }
}
