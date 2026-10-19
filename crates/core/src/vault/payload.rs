//! Canonical authenticated full revision and graph snapshots, never plaintext SQL.
use super::{Result, VaultError, keys::Context};
use crate::{
    content,
    memory::{MemoryDraft, MemoryMetadata},
    privacy::Privacy,
    relationships::RelationshipRecord,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use zeroize::{Zeroize, Zeroizing};

pub(crate) fn invalid<E>(_: E) -> VaultError {
    VaultError::InvalidEnvelope
}
pub(crate) fn id(s: &str) -> Result<()> {
    crate::workspace::validate_id(s).map_err(invalid)
}
fn wipe_option(s: &mut Option<String>) {
    if let Some(s) = s {
        s.zeroize();
    }
}
fn wipe_value(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::String(s) => s.zeroize(),
        serde_json::Value::Array(a) => a.iter_mut().for_each(wipe_value),
        serde_json::Value::Object(o) => o.values_mut().for_each(wipe_value),
        _ => {}
    }
}
pub(crate) fn wipe_edge(r: &mut RelationshipRecord) {
    r.predicate.zeroize();
    r.created_at.zeroize();
    r.updated_at.zeroize();
    r.provenance.zeroize();
    wipe_value(&mut r.metadata);
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordPayload {
    pub version: u8,
    pub purpose: String,
    pub workspace_id: String,
    pub vault_id: String,
    pub generation_id: String,
    pub record_id: String,
    pub revision_id: String,
    pub original_privacy: Privacy,
    pub metadata: MemoryMetadata,
    pub tags: Vec<String>,
    pub body: String,
    pub body_hash: String,
    pub relationship_snapshot: Vec<RelationshipRecord>,
}
impl Drop for RecordPayload {
    fn drop(&mut self) {
        self.body.zeroize();
        self.tags.zeroize();
        self.metadata.title.zeroize();
        self.metadata.source_references.zeroize();
        self.metadata.created_at.zeroize();
        self.metadata.updated_at.zeroize();
        self.metadata.provenance.zeroize();
        self.metadata.validation_status.zeroize();
        wipe_option(&mut self.metadata.generator_model);
        for s in [
            &mut self.metadata.event_date,
            &mut self.metadata.valid_from,
            &mut self.metadata.valid_until,
            &mut self.metadata.due_date,
        ] {
            wipe_option(s);
        }
        self.relationship_snapshot.iter_mut().for_each(wipe_edge);
    }
}
impl RecordPayload {
    pub fn validate(&self, context: &Context) -> Result<()> {
        for s in [&self.record_id, &self.revision_id] {
            id(s)?;
        }
        if self.version != 1
            || self.purpose != "sealed_revision"
            || self.workspace_id != context.workspace_id()
            || self.vault_id != context.vault_id()
            || self.generation_id != context.generation_id()
            || self.metadata.workspace_id != self.workspace_id
            || self.metadata.id != self.record_id
            || self.metadata.revision_id != self.revision_id
            || self.metadata.privacy != Privacy::Sealed
            || self.metadata.version == 0
            || content::hash(self.body.as_bytes()) != self.body_hash
            || self.tags.len() > 64
            || self
                .tags
                .iter()
                .any(|s| s.is_empty() || s.len() > 128 || s.contains('\0'))
        {
            return Err(VaultError::InvalidContext);
        }
        // Reuse existing syntactic validation only. This transient draft never
        // grants plaintext access or changes the Sealed metadata being persisted.
        let mut draft = MemoryDraft {
            title: self.metadata.title.clone(),
            body: self.body.clone(),
            privacy: Privacy::Private,
            event_date: self.metadata.event_date.clone(),
            valid_from: self.metadata.valid_from.clone(),
            valid_until: self.metadata.valid_until.clone(),
            confidence: self.metadata.confidence,
            source_references: self.metadata.source_references.clone(),
            project_id: Some(self.metadata.project_id.clone()),
            due_date: self.metadata.due_date.clone(),
            completed: self.metadata.completed,
        };
        let checked = crate::memory::validate(&draft);
        draft.title.zeroize();
        draft.body.zeroize();
        draft.source_references.zeroize();
        for s in [
            &mut draft.event_date,
            &mut draft.valid_from,
            &mut draft.valid_until,
            &mut draft.due_date,
        ] {
            wipe_option(s);
        }
        checked.map_err(invalid)?;
        crate::entities::validate_entity_fields(
            &self.metadata.kind,
            self.metadata.project_id.as_deref(),
            self.metadata.due_date.as_deref(),
            self.metadata.event_date.as_deref(),
        )
        .map_err(invalid)?;
        for stamp in [&self.metadata.created_at, &self.metadata.updated_at] {
            chrono::DateTime::parse_from_rfc3339(stamp).map_err(invalid)?;
        }
        for value in self
            .metadata
            .relationships
            .iter()
            .chain(self.metadata.project_id.iter())
            .chain(self.metadata.supersedes.iter())
            .chain(self.metadata.superseded_by.iter())
            .chain(self.metadata.restored_from.iter())
        {
            id(value)?;
        }
        for edge in &self.relationship_snapshot {
            validate_edge(edge, &self.workspace_id)?;
        }
        Ok(())
    }
    pub fn encode(&self, context: &Context) -> Result<Zeroizing<Vec<u8>>> {
        self.validate(context)?;
        let bytes = Zeroizing::new(serde_json::to_vec(self).map_err(invalid)?);
        if bytes.len() > super::envelope::MAX_RECORD_BYTES {
            return Err(VaultError::ResourceLimit);
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8], context: &Context) -> Result<Self> {
        if bytes.len() > super::envelope::MAX_RECORD_BYTES {
            return Err(VaultError::ResourceLimit);
        }
        super::json::validate(bytes, super::envelope::MAX_RECORD_BYTES)?;
        let result: Self = serde_json::from_slice(bytes).map_err(invalid)?;
        result.validate(context)?;
        Ok(result)
    }
}
fn validate_edge(edge: &RelationshipRecord, workspace: &str) -> Result<()> {
    for s in [&edge.id, &edge.source_id, &edge.target_id] {
        id(s)?;
    }
    if edge.workspace_id != workspace || edge.source_id == edge.target_id || edge.version == 0 {
        return Err(VaultError::InvalidContext);
    }
    crate::relationships::validate_predicate(&edge.predicate).map_err(invalid)?;
    if !edge.metadata.is_object()
        || Zeroizing::new(serde_json::to_vec(&edge.metadata).map_err(invalid)?).len() > 8192
    {
        return Err(VaultError::ResourceLimit);
    }
    for stamp in [&edge.created_at, &edge.updated_at] {
        chrono::DateTime::parse_from_rfc3339(stamp).map_err(invalid)?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub version: u8,
    pub workspace_id: String,
    pub records: Vec<RecordPayload>,
    pub current_revisions: BTreeMap<String, String>,
    pub graph: Vec<RelationshipRecord>,
    pub current_edges: BTreeMap<String, u32>,
    pub origin: Option<Origin>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Origin {
    pub workspace_id: String,
    pub vault_id: String,
    pub generation_id: String,
    pub ids: BTreeMap<String, String>,
    pub prior: Option<Box<Origin>>,
}
impl Drop for Snapshot {
    fn drop(&mut self) {
        self.graph.iter_mut().for_each(wipe_edge);
    }
}
impl Snapshot {
    pub fn validate(&self, context: &Context) -> Result<()> {
        if self.version != 1
            || self.workspace_id != context.workspace_id()
            || self.current_revisions.len() > 10_000
            || self
                .records
                .len()
                .checked_add(self.graph.len())
                .is_none_or(|n| n > 100_000)
        {
            return Err(VaultError::ResourceLimit);
        }
        let mut total = 0usize;
        let mut revisions = BTreeMap::<String, BTreeMap<u32, String>>::new();
        let mut revision_ids = BTreeSet::new();
        for record in &self.records {
            total = total
                .checked_add(record.encode(context)?.len())
                .ok_or(VaultError::ResourceLimit)?;
            if total > super::envelope::MAX_ARCHIVE_BYTES
                || !revision_ids.insert(&record.revision_id)
                || revisions
                    .entry(record.record_id.clone())
                    .or_default()
                    .insert(record.metadata.version, record.revision_id.clone())
                    .is_some()
            {
                return Err(VaultError::InvalidContext);
            }
        }
        if revisions.len() != self.current_revisions.len() {
            return Err(VaultError::InvalidContext);
        }
        for (record, versions) in &revisions {
            id(record)?;
            if versions.keys().copied().ne(1..=versions.len() as u32)
                || self.current_revisions.get(record) != versions.last_key_value().map(|(_, v)| v)
            {
                return Err(VaultError::InvalidContext);
            }
        }
        let mut edges = BTreeMap::<String, BTreeSet<u32>>::new();
        for edge in &self.graph {
            validate_edge(edge, &self.workspace_id)?;
            if !revisions.contains_key(&edge.source_id)
                || !revisions.contains_key(&edge.target_id)
                || !edges
                    .entry(edge.id.clone())
                    .or_default()
                    .insert(edge.version)
            {
                return Err(VaultError::InvalidContext);
            }
        }
        if edges.len() != self.current_edges.len() {
            return Err(VaultError::InvalidContext);
        }
        for (edge, versions) in edges {
            if versions.iter().copied().ne(1..=versions.len() as u32)
                || self.current_edges.get(&edge) != versions.last()
            {
                return Err(VaultError::InvalidContext);
            }
        }
        // A revision's saved graph must be the complete authenticated graph
        // projection, not an independently editable or truncated shadow.
        for record in &self.records {
            let expected: Vec<_> = self
                .graph
                .iter()
                .filter(|e| e.source_id == record.record_id || e.target_id == record.record_id)
                .collect();
            if Zeroizing::new(serde_json::to_vec(&record.relationship_snapshot).map_err(invalid)?)
                != Zeroizing::new(serde_json::to_vec(&expected).map_err(invalid)?)
            {
                return Err(VaultError::InvalidContext);
            }
        }
        let mut origin = self.origin.as_ref();
        let mut depth = 0;
        while let Some(value) = origin {
            depth += 1;
            if depth > 16 || value.ids.len() > 200_000 {
                return Err(VaultError::ResourceLimit);
            }
            for s in [&value.workspace_id, &value.vault_id, &value.generation_id] {
                id(s)?;
            }
            let mut targets = BTreeSet::new();
            for (source, target) in &value.ids {
                id(source)?;
                id(target)?;
                if !targets.insert(target) {
                    return Err(VaultError::InvalidContext);
                }
            }
            origin = value.prior.as_deref();
        }
        Ok(())
    }
    pub fn fingerprint(&self) -> Result<String> {
        Ok(content::hash(&Zeroizing::new(
            serde_json::to_vec(self).map_err(invalid)?,
        )))
    }
    pub(crate) fn capture(
        db: &rusqlite::Connection,
        root: &Path,
        context: &Context,
        backup: bool,
    ) -> Result<Self> {
        let workspace = context.workspace_id();
        let current_revisions = collect_map(
            db,
            "SELECT id,current_revision FROM memories WHERE workspace_id=?1 ORDER BY id LIMIT 10001",
            workspace,
        )?;
        let current_edges = collect_map(
            db,
            "SELECT id,version FROM relationships WHERE workspace_id=?1 ORDER BY id LIMIT 100001",
            workspace,
        )?;
        let mut query=db.prepare("SELECT snapshot FROM relationship_revisions WHERE workspace_id=?1 ORDER BY relationship_id,version").map_err(invalid)?;
        let mut graph = Vec::<RelationshipRecord>::new();
        let mut graph_bytes = 0usize;
        for raw in query
            .query_map([workspace], |r| r.get::<_, String>(0))
            .map_err(invalid)?
        {
            if graph.len() >= 100_000 {
                return Err(VaultError::ResourceLimit);
            }
            let raw = Zeroizing::new(raw.map_err(invalid)?);
            graph_bytes = graph_bytes
                .checked_add(raw.len())
                .ok_or(VaultError::ResourceLimit)?;
            if graph_bytes > super::envelope::MAX_JOURNAL_BYTES {
                return Err(VaultError::ResourceLimit);
            }
            super::json::validate(raw.as_bytes(), super::envelope::MAX_RECORD_BYTES)?;
            graph.push(serde_json::from_str(&raw).map_err(invalid)?);
        }
        let mut records = Vec::new();
        let mut total = 0;
        let mut query=db.prepare("SELECT memory_id,id,metadata,body_hash,version FROM memory_revisions WHERE workspace_id=?1 ORDER BY memory_id,version").map_err(invalid)?;
        let rows = query
            .query_map([workspace], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, u32>(4)?,
                ))
            })
            .map_err(invalid)?;
        for row in rows {
            if records.len() + graph.len() >= 100_000 {
                return Err(VaultError::ResourceLimit);
            }
            let (record_id, revision_id, raw, body_hash, version) = row.map_err(invalid)?;
            id(&record_id)?;
            id(&revision_id)?;
            let raw = Zeroizing::new(raw);
            if raw.len() > super::envelope::MAX_RECORD_BYTES {
                return Err(VaultError::ResourceLimit);
            }
            super::json::validate(raw.as_bytes(), super::envelope::MAX_RECORD_BYTES)?;
            let mut metadata: MemoryMetadata = serde_json::from_str(&raw).map_err(invalid)?;
            if metadata.id != record_id
                || metadata.revision_id != revision_id
                || metadata.workspace_id != workspace
                || metadata.version != version
            {
                return Err(VaultError::InvalidContext);
            }
            let body = if backup {
                let path = root.join("bodies").join(format!("{revision_id}.md"));
                crate::storage::reject_links(&path).map_err(invalid)?;
                let bytes =
                    crate::snapshots::limited(&path, content::MAX_BODY as u64).map_err(invalid)?;
                if content::hash(&bytes) != body_hash {
                    return Err(VaultError::AuthenticationFailed);
                }
                String::from_utf8(bytes).map_err(invalid)?
            } else {
                content::read(root, workspace, &record_id, &revision_id, &body_hash)
                    .map_err(invalid)?
            };
            let original_privacy = metadata.privacy;
            metadata.privacy = Privacy::Sealed;
            let relationship_snapshot = graph
                .iter()
                .filter(|r| r.source_id == record_id || r.target_id == record_id)
                .cloned()
                .collect();
            let record = RecordPayload {
                version: 1,
                purpose: "sealed_revision".into(),
                workspace_id: workspace.into(),
                vault_id: context.vault_id().into(),
                generation_id: context.generation_id().into(),
                record_id,
                revision_id,
                original_privacy,
                metadata,
                tags: Vec::new(),
                body,
                body_hash,
                relationship_snapshot,
            };
            total += record.encode(context)?.len();
            if total > super::envelope::MAX_ARCHIVE_BYTES {
                return Err(VaultError::ResourceLimit);
            }
            records.push(record);
        }
        let result = Self {
            version: 1,
            workspace_id: workspace.into(),
            records,
            current_revisions,
            graph,
            current_edges,
            origin: None,
        };
        result.validate(context)?;
        Ok(result)
    }
    pub(crate) fn rebind(&mut self, context: &Context) {
        self.workspace_id = context.workspace_id().into();
        for record in &mut self.records {
            record.workspace_id = context.workspace_id().into();
            record.vault_id = context.vault_id().into();
            record.generation_id = context.generation_id().into();
            record.metadata.workspace_id = context.workspace_id().into();
            for edge in &mut record.relationship_snapshot {
                edge.workspace_id = context.workspace_id().into();
            }
        }
        for edge in &mut self.graph {
            edge.workspace_id = context.workspace_id().into();
        }
    }
}
fn collect_map<V: rusqlite::types::FromSql>(
    db: &rusqlite::Connection,
    sql: &str,
    workspace: &str,
) -> Result<BTreeMap<String, V>> {
    let mut query = db.prepare(sql).map_err(invalid)?;
    query
        .query_map([workspace], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(invalid)?
        .collect::<rusqlite::Result<_>>()
        .map_err(invalid)
}
