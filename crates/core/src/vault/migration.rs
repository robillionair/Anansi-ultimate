//! Authenticated shadow-stage preparation only. Never changes canonical pointers,
//! deletes originals, executes a journal at startup, or upgrades a live schema.
//! Domain snapshot acquisition and fenced publication remain integration owners.
use super::{
    Result, VaultError,
    envelope::{self, KdfProfile, ParsedEnvelope, Purpose},
    keys::Context,
};
use crate::{
    content::hash,
    storage::{private_permissions, reject_links},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

pub struct StageInput<'a> {
    pub record_id: &'a str,
    pub revision_id: &'a str,
    /// Complete, already domain-validated revision/relationship snapshot bytes.
    /// This storage layer does not certify domain/history completeness.
    pub payload: &'a [u8],
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Member {
    version: u8,
    purpose: String,
    workspace_id: String,
    vault_id: String,
    generation_id: String,
    record_id: String,
    revision_id: String,
    payload: Vec<u8>,
}
impl Drop for Member {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.payload.zeroize();
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    record_id: String,
    revision_id: String,
    envelope_hash: String,
    byte_length: usize,
    source_hash: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Journal {
    version: u8,
    purpose: String,
    workspace_id: String,
    vault_id: String,
    source_generation_id: String,
    target_generation_id: String,
    journal_id: String,
    entries: Vec<Entry>,
}
fn valid_id(id: &str) -> Result<()> {
    if id.len() != 32
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(VaultError::InvalidContext);
    }
    Ok(())
}
fn failure<E>(_: E) -> VaultError {
    VaultError::StorageUnavailable
}
fn stage_directory(root: &Path, context: &Context, create: bool) -> Result<PathBuf> {
    reject_links(root).map_err(failure)?;
    let mut path = root.to_path_buf();
    for part in ["vault-staging", context.vault_id(), context.generation_id()] {
        path.push(part);
        reject_links(&path).map_err(failure)?;
        if create && !path.exists() {
            fs::create_dir(&path).map_err(failure)?;
            private_permissions(&path, true, false).map_err(failure)?;
        }
        private_permissions(&path, true, true).map_err(failure)?;
    }
    Ok(path)
}
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    reject_links(path).map_err(failure)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(failure)?;
    private_permissions(path, false, false).map_err(failure)?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(failure)
}
pub(crate) fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    reject_links(path).map_err(failure)?;
    private_permissions(path, false, true).map_err(failure)?;
    let mut out = Vec::new();
    fs::File::open(path)
        .map_err(failure)?
        .take((maximum + 1) as u64)
        .read_to_end(&mut out)
        .map_err(failure)?;
    if out.len() > maximum {
        return Err(VaultError::ResourceLimit);
    }
    Ok(out)
}

/// An explicit callback can cancel before each member; every interruption retains
/// the original data and partial encrypted stage. No plaintext or filesystem path
/// is accepted from a renderer, since there is no adapter entry point.
pub fn prepare(
    root: &Path,
    context: &Context,
    source_generation: &str,
    key: &[u8; 32],
    inputs: &[StageInput<'_>],
    mut progress: impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    valid_id(source_generation)?;
    if source_generation == context.generation_id() || inputs.is_empty() || inputs.len() > 100_000 {
        return Err(VaultError::InvalidContext);
    }
    let mut identities = HashSet::new();
    let mut records = HashSet::new();
    for input in inputs {
        valid_id(input.record_id)?;
        valid_id(input.revision_id)?;
        if input.payload.len() > crate::content::MAX_BODY {
            return Err(VaultError::ResourceLimit);
        }
        if !identities.insert((input.record_id, input.revision_id)) {
            return Err(VaultError::InvalidContext);
        }
        records.insert(input.record_id);
    }
    if records.len() > 10_000 {
        return Err(VaultError::ResourceLimit);
    }
    let path = stage_directory(root, context, true)?;
    // Refuse overwriting or guessing how an existing incomplete generation resumes.
    if fs::read_dir(&path).map_err(failure)?.next().is_some() {
        return Err(VaultError::StorageUnavailable);
    }
    let journal_id = crate::content::random_id().map_err(failure)?;
    let mut journal = Journal {
        version: 1,
        purpose: "vault_migration".into(),
        workspace_id: context.workspace_id().into(),
        vault_id: context.vault_id().into(),
        source_generation_id: source_generation.into(),
        target_generation_id: context.generation_id().into(),
        journal_id,
        entries: Vec::new(),
    };
    let mut total = 0usize;
    for (index, input) in inputs.iter().enumerate() {
        progress(index)?;
        let member = Member {
            version: 1,
            purpose: "sealed_revision".into(),
            workspace_id: context.workspace_id().into(),
            vault_id: context.vault_id().into(),
            generation_id: context.generation_id().into(),
            record_id: input.record_id.into(),
            revision_id: input.revision_id.into(),
            payload: input.payload.to_vec(),
        };
        let plain = Zeroizing::new(serde_json::to_vec(&member).map_err(failure)?);
        total = total
            .checked_add(plain.len())
            .ok_or(VaultError::ResourceLimit)?;
        if total > envelope::MAX_ARCHIVE_BYTES {
            return Err(VaultError::ResourceLimit);
        }
        let salt = envelope::random::<16>()?;
        let encrypted = envelope::seal(key, Purpose::Record, &plain, KdfProfile::default(), &salt)?;
        let file = path.join(format!("{}-{}.ansv", input.record_id, input.revision_id));
        write_private(&file, &encrypted)?;
        let readback = read_bounded(&file, envelope::MAX_RECORD_BYTES + 80)?;
        if readback != encrypted
            || *ParsedEnvelope::parse(&readback, Purpose::Record)?.open(key)? != *plain
        {
            return Err(VaultError::AuthenticationFailed);
        }
        journal.entries.push(Entry {
            record_id: input.record_id.into(),
            revision_id: input.revision_id.into(),
            envelope_hash: hash(&encrypted),
            byte_length: encrypted.len(),
            source_hash: hash(input.payload),
        });
    }
    progress(inputs.len())?;
    let manifest = Zeroizing::new(serde_json::to_vec(&journal).map_err(failure)?);
    if manifest.len() > envelope::MAX_JOURNAL_BYTES
        || total
            .checked_add(manifest.len())
            .is_none_or(|n| n > envelope::MAX_ARCHIVE_BYTES)
    {
        return Err(VaultError::ResourceLimit);
    }
    let salt = envelope::random::<16>()?;
    let encrypted = envelope::seal(
        key,
        Purpose::Journal,
        &manifest,
        KdfProfile::default(),
        &salt,
    )?;
    write_private(&path.join("manifest.ansv"), &encrypted)?;
    verify(root, context, source_generation, key)?;
    Ok(())
}

/// Must be explicitly called AFTER unlock. Verifies a stage, never publishes it.
pub fn verify(
    root: &Path,
    context: &Context,
    source_generation: &str,
    key: &[u8; 32],
) -> Result<usize> {
    valid_id(source_generation)?;
    if source_generation == context.generation_id() {
        return Err(VaultError::InvalidContext);
    }
    let path = stage_directory(root, context, false)?;
    let encrypted = read_bounded(
        &path.join("manifest.ansv"),
        envelope::MAX_JOURNAL_BYTES + 80,
    )?;
    let manifest = ParsedEnvelope::parse(&encrypted, Purpose::Journal)?.open(key)?;
    let journal: Journal =
        serde_json::from_slice(&manifest).map_err(|_| VaultError::InvalidEnvelope)?;
    if journal.version != 1
        || journal.purpose != "vault_migration"
        || journal.workspace_id != context.workspace_id()
        || journal.vault_id != context.vault_id()
        || journal.target_generation_id != context.generation_id()
        || journal.source_generation_id != source_generation
        || journal.entries.is_empty()
        || journal.entries.len() > 100_000
    {
        return Err(VaultError::InvalidContext);
    }
    valid_id(&journal.journal_id)?;
    let mut identities = HashSet::new();
    let mut records = HashSet::new();
    let mut total = manifest.len();
    for entry in &journal.entries {
        valid_id(&entry.record_id)?;
        valid_id(&entry.revision_id)?;
        if !identities.insert((&entry.record_id, &entry.revision_id)) {
            return Err(VaultError::InvalidContext);
        }
        records.insert(&entry.record_id);
        if entry.byte_length > envelope::MAX_RECORD_BYTES + 80 {
            return Err(VaultError::ResourceLimit);
        }
        let file = path.join(format!("{}-{}.ansv", entry.record_id, entry.revision_id));
        let bytes = read_bounded(&file, envelope::MAX_RECORD_BYTES + 80)?;
        if bytes.len() != entry.byte_length || hash(&bytes) != entry.envelope_hash {
            return Err(VaultError::AuthenticationFailed);
        }
        let plain = ParsedEnvelope::parse(&bytes, Purpose::Record)?.open(key)?;
        total = total
            .checked_add(plain.len())
            .ok_or(VaultError::ResourceLimit)?;
        if total > envelope::MAX_ARCHIVE_BYTES {
            return Err(VaultError::ResourceLimit);
        }
        let member: Member =
            serde_json::from_slice(&plain).map_err(|_| VaultError::InvalidEnvelope)?;
        if member.version != 1
            || member.purpose != "sealed_revision"
            || member.workspace_id != context.workspace_id()
            || member.vault_id != context.vault_id()
            || member.generation_id != context.generation_id()
            || member.record_id != entry.record_id
            || member.revision_id != entry.revision_id
            || member.payload.len() > crate::content::MAX_BODY
            || hash(&member.payload) != entry.source_hash
        {
            return Err(VaultError::InvalidContext);
        }
    }
    if records.len() > 10_000 {
        return Err(VaultError::ResourceLimit);
    }
    let mut expected: HashSet<String> = journal
        .entries
        .iter()
        .map(|entry| format!("{}-{}.ansv", entry.record_id, entry.revision_id))
        .collect();
    expected.insert("manifest.ansv".into());
    for file in fs::read_dir(&path).map_err(failure)? {
        let file = file.map_err(failure)?;
        let name = file
            .file_name()
            .into_string()
            .map_err(|_| VaultError::InvalidEnvelope)?;
        if !expected.remove(&name) {
            return Err(VaultError::InvalidEnvelope);
        }
        reject_links(&file.path()).map_err(failure)?;
    }
    if !expected.is_empty() {
        return Err(VaultError::InvalidEnvelope);
    }
    Ok(journal.entries.len())
}
