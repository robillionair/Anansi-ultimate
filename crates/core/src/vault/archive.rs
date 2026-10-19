//! Ciphertext-only portable packages and passphrase-encrypted exports.
//! Authentication precedes every fresh destination write; no path-based members.
use super::{
    Result, VaultError,
    admission::Admission,
    catalog::{Generation, Phase, VaultStore},
    envelope::{self, KdfProfile, ParsedEnvelope, Purpose},
    keys::{Context, KeySlots},
    migration::read_bounded,
    payload::{Origin, Snapshot, id, invalid},
    recovery::{AcknowledgedSetup, PendingSetup, RecoveryDisplay},
    session::{Caller, Sessions},
};
use crate::content;
use rusqlite::{TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"ANCB\0\0\0\x01";
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileInfo {
    token: String,
    bytes: usize,
    sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Header {
    format: String,
    version: u8,
    workspace_id: String,
    vault_id: String,
    generation_id: String,
    source_generation_id: String,
    manifest_hash: String,
    passphrase_slot: Vec<u8>,
    recovery_slot: Vec<u8>,
    files: Vec<FileInfo>,
}
pub struct CiphertextBackup {
    header: Header,
    members: BTreeMap<String, Vec<u8>>,
}
pub struct LockedPreview {
    pub authenticated: bool,
    pub files: usize,
    pub bytes: usize,
}
enum RestoreKey<'a> {
    Passphrase(&'a [u8]),
    Recovery(&'a [u8; 32]),
}
impl CiphertextBackup {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let metadata = serde_json::to_vec(&self.header).map_err(invalid)?;
        let size = self
            .members
            .values()
            .try_fold(12usize + metadata.len(), |n, b| {
                n.checked_add(b.len()).ok_or(VaultError::ResourceLimit)
            })?;
        if size > envelope::MAX_ARCHIVE_BYTES || metadata.len() > envelope::MAX_JOURNAL_BYTES {
            return Err(VaultError::ResourceLimit);
        }
        let mut out = Vec::with_capacity(size);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(metadata.len() as u32).to_be_bytes());
        out.extend_from_slice(&metadata);
        for file in &self.header.files {
            out.extend_from_slice(
                self.members
                    .get(&file.token)
                    .ok_or(VaultError::InvalidEnvelope)?,
            );
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 12 || bytes.len() > envelope::MAX_ARCHIVE_BYTES || &bytes[..8] != MAGIC {
            return Err(VaultError::InvalidEnvelope);
        }
        let size = u32::from_be_bytes(bytes[8..12].try_into().expect("checked header")) as usize;
        if size > envelope::MAX_JOURNAL_BYTES || size > bytes.len() - 12 {
            return Err(VaultError::ResourceLimit);
        }
        super::json::validate(&bytes[12..12 + size], envelope::MAX_JOURNAL_BYTES)?;
        let header: Header = serde_json::from_slice(&bytes[12..12 + size]).map_err(invalid)?;
        if header.format != "anansi-ciphertext-backup"
            || header.version != 1
            || header.files.is_empty()
            || header.files.len() > 100_001
        {
            return Err(VaultError::InvalidEnvelope);
        }
        Context::new(
            &header.vault_id,
            &header.workspace_id,
            &header.generation_id,
        )?;
        id(&header.source_generation_id)?;
        ParsedEnvelope::parse(&header.passphrase_slot, Purpose::PassphraseSlot)?;
        ParsedEnvelope::parse(&header.recovery_slot, Purpose::RecoverySlot)?;
        let mut at = 12 + size;
        let mut members = BTreeMap::new();
        for file in &header.files {
            if file.token != "manifest" {
                id(&file.token)?;
            }
            let limit = if file.token == "manifest" {
                envelope::MAX_JOURNAL_BYTES + 80
            } else {
                envelope::MAX_RECORD_BYTES + 80
            };
            if file.bytes > limit || file.bytes < 80 {
                return Err(VaultError::ResourceLimit);
            }
            let end = at
                .checked_add(file.bytes)
                .ok_or(VaultError::ResourceLimit)?;
            let member = bytes.get(at..end).ok_or(VaultError::InvalidEnvelope)?;
            if content::hash(member) != file.sha256
                || members
                    .insert(file.token.clone(), member.to_vec())
                    .is_some()
            {
                return Err(VaultError::AuthenticationFailed);
            }
            ParsedEnvelope::parse(
                member,
                if file.token == "manifest" {
                    Purpose::Journal
                } else {
                    Purpose::Record
                },
            )?;
            at = end;
        }
        if at != bytes.len()
            || !members.contains_key("manifest")
            || content::hash(&members["manifest"]) != header.manifest_hash
        {
            return Err(VaultError::InvalidEnvelope);
        }
        Ok(Self { header, members })
    }
    pub fn preview(bytes: &[u8]) -> Result<LockedPreview> {
        let package = Self::decode(bytes)?;
        Ok(LockedPreview {
            authenticated: false,
            files: package.members.len(),
            bytes: bytes.len(),
        })
    }
    fn descriptor(&self) -> Result<Generation> {
        Ok(Generation {
            context: Context::new(
                &self.header.vault_id,
                &self.header.workspace_id,
                &self.header.generation_id,
            )?,
            source: self.header.source_generation_id.clone(),
            manifest_hash: self.header.manifest_hash.clone(),
            epoch: 0,
            slots: KeySlots {
                passphrase: self.header.passphrase_slot.clone(),
                recovery: self.header.recovery_slot.clone(),
            },
        })
    }
    fn authenticate(&self, key: &[u8; 32]) -> Result<Snapshot> {
        let descriptor = self.descriptor()?;
        let mut unused: BTreeSet<_> = self
            .members
            .keys()
            .filter(|k| k.as_str() != "manifest")
            .cloned()
            .collect();
        let snapshot =
            VaultStore::decode_generation(&descriptor, key, &self.members["manifest"], |token| {
                if !unused.remove(token) {
                    return Err(VaultError::InvalidEnvelope);
                }
                self.members
                    .get(token)
                    .cloned()
                    .ok_or(VaultError::InvalidEnvelope)
            })?;
        if !unused.is_empty() {
            return Err(VaultError::InvalidEnvelope);
        }
        Ok(snapshot)
    }
}

impl VaultStore {
    fn collect_backup(
        &self,
        db: &rusqlite::Connection,
        descriptor: &Generation,
    ) -> Result<CiphertextBackup> {
        let directory = self.directory(&descriptor.context, false)?;
        let mut query=db.prepare("SELECT token,envelope_hash,byte_length FROM vault_files WHERE vault_id=?1 AND generation_id=?2 ORDER BY token LIMIT 100002").map_err(invalid)?;
        let rows = query
            .query_map(
                params![
                    descriptor.context.vault_id(),
                    descriptor.context.generation_id()
                ],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                },
            )
            .map_err(invalid)?;
        let mut members = BTreeMap::new();
        let mut files = Vec::new();
        let mut total = 0usize;
        for row in rows {
            if files.len() >= 100_001 {
                return Err(VaultError::ResourceLimit);
            }
            let (token, sha256, size) = row.map_err(invalid)?;
            if token != "manifest" {
                id(&token)?;
            }
            let maximum = if token == "manifest" {
                envelope::MAX_JOURNAL_BYTES + 80
            } else {
                envelope::MAX_RECORD_BYTES + 80
            };
            let size = usize::try_from(size).map_err(invalid)?;
            if size > maximum {
                return Err(VaultError::ResourceLimit);
            }
            let bytes = read_bounded(&directory.join(format!("{token}.ansv")), maximum)?;
            if bytes.len() != size || content::hash(&bytes) != sha256 {
                return Err(VaultError::AuthenticationFailed);
            }
            total = total
                .checked_add(bytes.len())
                .ok_or(VaultError::ResourceLimit)?;
            if total > envelope::MAX_ARCHIVE_BYTES {
                return Err(VaultError::ResourceLimit);
            }
            files.push(FileInfo {
                token: token.clone(),
                bytes: size,
                sha256,
            });
            members.insert(token, bytes);
        }
        let result = CiphertextBackup {
            header: Header {
                format: "anansi-ciphertext-backup".into(),
                version: 1,
                workspace_id: descriptor.context.workspace_id().into(),
                vault_id: descriptor.context.vault_id().into(),
                generation_id: descriptor.context.generation_id().into(),
                source_generation_id: descriptor.source.clone(),
                manifest_hash: descriptor.manifest_hash.clone(),
                passphrase_slot: descriptor.slots.passphrase.clone(),
                recovery_slot: descriptor.slots.recovery.clone(),
                files,
            },
            members,
        };
        // Structural/hash checks are all that a locked copy may claim.
        CiphertextBackup::decode(&result.encode()?)
    }
    pub fn ciphertext_backup(&self, workspace: &str) -> Result<Vec<u8>> {
        let _root = Admission::root(&self.root)?;
        let db = self.connection()?;
        self.collect_backup(&db, &Self::generation(&db, workspace, None)?)?
            .encode()
    }
    pub fn encrypted_export(
        &self,
        sessions: &mut Sessions,
        caller: &Caller,
        token: &[u8; 16],
        password: &[u8],
        now: Instant,
    ) -> Result<Vec<u8>> {
        let started = Instant::now();
        let _root = Admission::root(&self.root)?;
        let _kdf = Admission::kdf(&self.host_namespace)?;
        let db = self.connection()?;
        let descriptor = Self::generation(&db, caller.context.workspace_id(), None)?;
        if descriptor.context != caller.context || caller.root_id != self.root_id()? {
            return Err(VaultError::Locked);
        }
        let bytes = sessions.authorize(caller, token, descriptor.epoch, now, |key| {
            let mut package = self.collect_backup(&db, &descriptor)?;
            package.authenticate(key)?;
            // Same export passphrase opens the outer archive and the rewrapped
            // source slot; existing original backups/slots are never modified.
            package.header.passphrase_slot =
                KeySlots::wrap_passphrase(&descriptor.context, key, password)?;
            let plain = Zeroizing::new(package.encode()?);
            let salt = envelope::random::<16>()?;
            let kek = envelope::derive_key(password, &salt, KdfProfile::default(), false)?;
            envelope::seal(&kek, Purpose::Archive, &plain, KdfProfile::default(), &salt)
        })?;
        sessions.authorize(
            caller,
            token,
            descriptor.epoch,
            now.checked_add(started.elapsed())
                .ok_or(VaultError::Locked)?,
            |_| Ok(()),
        )?;
        Ok(bytes)
    }
    /// Native input only. Outer decryption writes nothing and remains KDF-admitted.
    pub fn decrypt_export(&self, bytes: &[u8], password: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        let _root = Admission::root(&self.root)?;
        let _kdf = Admission::kdf(&self.host_namespace)?;
        let plain = envelope::open_passphrase(bytes, Purpose::Archive, password, false)?;
        CiphertextBackup::decode(&plain)?;
        Ok(plain)
    }
    pub fn prepare_restore(
        &self,
        bytes: &[u8],
        source_password: &[u8],
        name: &str,
        destination_password: &[u8],
    ) -> Result<PendingRestore> {
        self.prepare_restore_key(
            bytes,
            RestoreKey::Passphrase(source_password),
            name,
            destination_password,
        )
    }
    /// Native recovery input only; authenticated context-bound source slot unwrap.
    pub fn prepare_restore_recovery(
        &self,
        bytes: &[u8],
        recovery_key: &[u8; 32],
        name: &str,
        destination_password: &[u8],
    ) -> Result<PendingRestore> {
        self.prepare_restore_key(
            bytes,
            RestoreKey::Recovery(recovery_key),
            name,
            destination_password,
        )
    }
    fn prepare_restore_key(
        &self,
        bytes: &[u8],
        credential: RestoreKey<'_>,
        name: &str,
        destination_password: &[u8],
    ) -> Result<PendingRestore> {
        crate::workspace::validate_name(name).map_err(invalid)?;
        let _root = Admission::root(&self.root)?;
        let _kdf = Admission::kdf(&self.host_namespace)?;
        let package = CiphertextBackup::decode(bytes)?;
        let descriptor = package.descriptor()?;
        let key = match credential {
            RestoreKey::Passphrase(password) => {
                descriptor
                    .slots
                    .unlock(&descriptor.context, password, false)?
            }
            RestoreKey::Recovery(key) => descriptor.slots.recover(&descriptor.context, key)?,
        };
        let snapshot = package.authenticate(&key)?;
        let context = Context::new(
            &content::random_id().map_err(invalid)?,
            &content::random_id().map_err(invalid)?,
            &content::random_id().map_err(invalid)?,
        )?;
        let pending = PendingSetup::new(context.clone(), destination_password, self.root_id()?)?;
        Ok(PendingRestore {
            pending: Some(pending),
            snapshot,
            target: context,
            source: descriptor.context,
            name: name.into(),
        })
    }
    pub fn finish_restore(
        &self,
        restore: AcknowledgedRestore,
        mut observe: impl FnMut(Phase) -> Result<()>,
    ) -> Result<String> {
        let AcknowledgedRestore { mut pending, ack } = restore;
        if ack.context != pending.target || ack.root_id != self.root_id()? {
            return Err(VaultError::InvalidContext);
        }
        let _root = Admission::root(&self.root)?;
        let mut db = self.connection()?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(invalid)?;
        let mut ids = BTreeMap::new();
        ids.insert(
            pending.source.workspace_id().to_owned(),
            ack.context.workspace_id().to_owned(),
        );
        for record in &pending.snapshot.records {
            for value in [&record.record_id, &record.revision_id]
                .into_iter()
                .chain(record.metadata.relationships.iter())
                .chain(record.metadata.project_id.iter())
                .chain(record.metadata.supersedes.iter())
                .chain(record.metadata.superseded_by.iter())
                .chain(record.metadata.restored_from.iter())
            {
                if !ids.contains_key(value) {
                    ids.insert(value.clone(), content::random_id().map_err(invalid)?);
                }
            }
        }
        for edge in &pending.snapshot.graph {
            if !ids.contains_key(&edge.id) {
                ids.insert(edge.id.clone(), content::random_id().map_err(invalid)?);
            }
        }
        let remap = |s: &mut String| -> Result<()> {
            *s = ids.get(s).ok_or(VaultError::InvalidContext)?.clone();
            Ok(())
        };
        let edge_remap = |edge: &mut crate::relationships::RelationshipRecord| -> Result<()> {
            remap(&mut edge.id)?;
            remap(&mut edge.source_id)?;
            remap(&mut edge.target_id)?;
            Ok(())
        };
        for record in &mut pending.snapshot.records {
            remap(&mut record.record_id)?;
            remap(&mut record.revision_id)?;
            remap(&mut record.metadata.id)?;
            remap(&mut record.metadata.revision_id)?;
            for value in &mut record.metadata.relationships {
                remap(value)?;
            }
            for value in [
                &mut record.metadata.project_id,
                &mut record.metadata.supersedes,
                &mut record.metadata.superseded_by,
                &mut record.metadata.restored_from,
            ]
            .into_iter()
            .flatten()
            {
                remap(value)?;
            }
            for edge in &mut record.relationship_snapshot {
                edge_remap(edge)?;
            }
        }
        for edge in &mut pending.snapshot.graph {
            edge_remap(edge)?;
        }
        pending.snapshot.current_revisions = pending
            .snapshot
            .current_revisions
            .iter()
            .map(|(id, rev)| {
                Ok((
                    ids.get(id).ok_or(VaultError::InvalidContext)?.clone(),
                    ids.get(rev).ok_or(VaultError::InvalidContext)?.clone(),
                ))
            })
            .collect::<Result<_>>()?;
        pending.snapshot.current_edges = pending
            .snapshot
            .current_edges
            .iter()
            .map(|(id, version)| {
                Ok((
                    ids.get(id).ok_or(VaultError::InvalidContext)?.clone(),
                    *version,
                ))
            })
            .collect::<Result<_>>()?;
        pending.snapshot.origin = Some(Origin {
            workspace_id: pending.source.workspace_id().into(),
            vault_id: pending.source.vault_id().into(),
            generation_id: pending.source.generation_id().into(),
            ids,
            prior: pending.snapshot.origin.take().map(Box::new),
        });
        pending.snapshot.rebind(&ack.context);
        pending.snapshot.validate(&ack.context)?;
        let hash = self.stage(
            &pending.snapshot,
            &ack.context,
            pending.source.generation_id(),
            &ack.dek,
            &mut observe,
        )?;
        Self::upgrade(&tx)?;
        tx.execute(
            "INSERT INTO workspaces(id,name,name_key) VALUES(?1,?2,?3)",
            params![
                ack.context.workspace_id(),
                pending.name,
                pending.name.to_ascii_lowercase()
            ],
        )
        .map_err(invalid)?;
        tx.execute(
            "INSERT INTO workspace_settings(workspace_id) VALUES(?1)",
            [ack.context.workspace_id()],
        )
        .map_err(invalid)?;
        self.publish_new(
            &tx,
            &ack,
            pending.source.generation_id(),
            &hash,
            &pending.snapshot,
        )?;
        observe(Phase::PreSwitch)?;
        tx.commit().map_err(invalid)?;
        observe(Phase::PostSwitch)?;
        Ok(ack.context.workspace_id().into())
    }
}
pub struct PendingRestore {
    pending: Option<PendingSetup>,
    snapshot: Snapshot,
    target: Context,
    source: Context,
    name: String,
}
pub struct AcknowledgedRestore {
    pending: PendingRestore,
    ack: AcknowledgedSetup,
}
impl PendingRestore {
    pub fn display_once(&mut self) -> Result<RecoveryDisplay> {
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
    ) -> Result<AcknowledgedRestore> {
        let ack = self
            .pending
            .take()
            .ok_or(VaultError::AcknowledgementRequired)?
            .acknowledge(typed, challenge, saved)?;
        Ok(AcknowledgedRestore { pending: self, ack })
    }
}
