//! Safe validation and fresh-target materialization for backup packages.

use crate::{
    CommandError,
    backup::{BackupPreview, read_and_verify},
    storage::reject_links,
};
use std::{fs, path::Path};

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreResult {
    pub contract_version: u8,
    pub workspaces: Vec<crate::workspace::Workspace>,
    pub backup_hash: String,
}

fn remap_metadata(
    raw: &str,
    map: &std::collections::BTreeMap<String, String>,
) -> Result<String, CommandError> {
    let mut value: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| crate::jobs::invalid())?;
    let obj = value.as_object_mut().ok_or_else(crate::jobs::invalid)?;
    for key in [
        "id",
        "workspaceId",
        "projectId",
        "revisionId",
        "sourceId",
        "targetId",
        "supersededBy",
        "restoredFrom",
        "supersedes",
    ] {
        if let Some(serde_json::Value::String(id)) = obj.get_mut(key) {
            *id = map.get(id).ok_or_else(crate::jobs::invalid)?.clone();
        }
    }
    if let Some(serde_json::Value::Array(ids)) = obj.get_mut("relationships") {
        for id in ids {
            let old = id.as_str().ok_or_else(crate::jobs::invalid)?;
            *id = serde_json::Value::String(map.get(old).ok_or_else(crate::jobs::invalid)?.clone());
        }
    }
    serde_json::to_string(&value).map_err(|_| crate::jobs::invalid())
}

impl crate::storage::Storage {
    pub fn restore_snapshot(
        &self,
        workspace: &str,
        job: &str,
        artifact: &str,
        name: &str,
        expected_hash: &str,
    ) -> Result<RestoreResult, CommandError> {
        crate::workspace::validate_name(name)?;
        let verified = self.verify_snapshot(workspace, job, artifact)?;
        if verified.preview.sha256 != expected_hash {
            return Err(crate::jobs::conflict());
        }
        let source = rusqlite::Connection::open_with_flags(
            verified.stage.root.join("catalog.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|_| crate::jobs::invalid())?;
        source
            .execute_batch("PRAGMA trusted_schema=OFF;")
            .map_err(|_| crate::jobs::invalid())?;
        let mut ids = std::collections::BTreeMap::new();
        let mut owners = std::collections::BTreeMap::new();
        let mut identity_queries = vec![
            "SELECT id,id FROM workspaces",
            "SELECT id,workspace_id FROM memories",
            "SELECT id,workspace_id FROM memory_revisions",
            "SELECT id,workspace_id FROM relationships",
        ];
        if verified.info.application {
            identity_queries.extend([
                "SELECT id,workspace_id FROM jobs",
                "SELECT artifact_id,workspace_id FROM jobs WHERE state='Completed'",
            ]);
        }
        for query in identity_queries {
            let mut q = source.prepare(query).map_err(|_| crate::jobs::invalid())?;
            for old in q
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .map_err(|_| crate::jobs::invalid())?
            {
                let (old, owner) = old.map_err(|_| crate::jobs::invalid())?;
                owners.insert(old.clone(), owner);
                crate::workspace::validate_id(&old)?;
                if ids.insert(old, crate::content::random_id()?).is_some() {
                    return Err(crate::jobs::invalid());
                }
            }
        }
        // Validate and prepare every mapped row before any managed content is written.
        let mut tables = vec![
            "workspaces",
            "workspace_settings",
            "memories",
            "memory_revisions",
            "relationships",
            "relationship_revisions",
        ];
        if verified.info.application {
            tables.extend(["jobs", "backup_schedules"]);
        }
        let mut prepared = Vec::new();
        let mut workspaces = Vec::new();
        for table in tables {
            let mut q = source
                .prepare(&format!("SELECT * FROM {table}"))
                .map_err(|_| crate::jobs::invalid())?;
            let names = q
                .column_names()
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>();
            let mut rows = q.query([]).map_err(|_| crate::jobs::invalid())?;
            let mut values = Vec::new();
            while let Some(row) = rows.next().map_err(|_| crate::jobs::invalid())? {
                let mut columns = (0..names.len())
                    .map(|i| row.get::<_, rusqlite::types::Value>(i))
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|_| crate::jobs::invalid())?;
                for (column, value) in names.iter().zip(columns.iter_mut()) {
                    if let rusqlite::types::Value::Text(text) = value {
                        if [
                            "id",
                            "workspace_id",
                            "memory_id",
                            "current_revision",
                            "relationship_id",
                            "source_id",
                            "target_id",
                            "artifact_id",
                        ]
                        .contains(&column.as_str())
                        {
                            *text = ids.get(text).ok_or_else(crate::jobs::invalid)?.clone();
                        } else if column == "selection" && table == "jobs" {
                            let mut selection: crate::jobs::ExportSelection =
                                serde_json::from_str(text).map_err(|_| crate::jobs::invalid())?;
                            match &mut selection {
                                crate::jobs::ExportSelection::Project { id } => {
                                    *id = ids.get(id).ok_or_else(crate::jobs::invalid)?.clone()
                                }
                                crate::jobs::ExportSelection::Records { ids: records } => {
                                    for id in records {
                                        *id = ids.get(id).ok_or_else(crate::jobs::invalid)?.clone();
                                    }
                                }
                                _ => {}
                            }
                            *text = serde_json::to_string(&selection)
                                .map_err(|_| crate::jobs::invalid())?;
                        } else if column == "metadata" && table == "memory_revisions"
                            || column == "snapshot"
                        {
                            *text = remap_metadata(text, &ids)?;
                        }
                    }
                }
                if table == "workspaces" {
                    let id = if let rusqlite::types::Value::Text(id) = &columns[0] {
                        id.clone()
                    } else {
                        return Err(crate::jobs::invalid());
                    };
                    let renamed = if verified.info.workspaces.len() == 1 {
                        name.to_string()
                    } else {
                        format!("{} {}", name.chars().take(54).collect::<String>(), &id[..8])
                    };
                    columns[1] = rusqlite::types::Value::Text(renamed.clone());
                    columns[2] = rusqlite::types::Value::Text(renamed.to_ascii_lowercase());
                    workspaces.push(crate::workspace::Workspace { id, name: renamed });
                }
                values.push(columns);
            }
            prepared.push((table, names.len(), values));
        }
        let mut connection = self
            .connection(false)?
            .ok_or_else(crate::storage::unavailable)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| crate::storage::unavailable())?;
        tx.execute_batch("PRAGMA defer_foreign_keys=ON;")
            .map_err(|_| crate::storage::unavailable())?;
        for w in &workspaces {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM workspaces WHERE name_key=?1)",
                    [w.name.to_ascii_lowercase()],
                    |r| r.get(0),
                )
                .map_err(|_| crate::storage::unavailable())?;
            if exists {
                return Err(CommandError::new(
                    crate::ErrorCode::WorkspaceExists,
                    "Choose an unused restore workspace name.",
                ));
            }
        }
        let mut q = source
            .prepare("SELECT workspace_id,memory_id,id,body_hash FROM memory_revisions")
            .map_err(|_| crate::jobs::invalid())?;
        let mut rows = q.query([]).map_err(|_| crate::jobs::invalid())?;
        while let Some(row) = rows.next().map_err(|_| crate::jobs::invalid())? {
            let w: String = row.get(0).map_err(|_| crate::jobs::invalid())?;
            let m: String = row.get(1).map_err(|_| crate::jobs::invalid())?;
            let r: String = row.get(2).map_err(|_| crate::jobs::invalid())?;
            let hash: String = row.get(3).map_err(|_| crate::jobs::invalid())?;
            let bytes = crate::snapshots::limited(
                &verified.stage.root.join(format!("bodies/{r}.md")),
                crate::content::MAX_BODY as u64,
            )?;
            if crate::content::hash(&bytes) != hash {
                return Err(crate::content::integrity());
            }
            let body = std::str::from_utf8(&bytes).map_err(|_| crate::content::integrity())?;
            crate::content::write(&self.root, &ids[&w], &ids[&m], &ids[&r], body)?;
        }
        if verified.info.application {
            let mut q=source.prepare("SELECT workspace_id,artifact_id,artifact_hash FROM jobs WHERE state='Completed'").map_err(|_|crate::jobs::invalid())?;
            let mut rows = q.query([]).map_err(|_| crate::jobs::invalid())?;
            while let Some(row) = rows.next().map_err(|_| crate::jobs::invalid())? {
                let w: String = row.get(0).map_err(|_| crate::jobs::invalid())?;
                let artifact: String = row.get(1).map_err(|_| crate::jobs::invalid())?;
                let hash: String = row.get(2).map_err(|_| crate::jobs::invalid())?;
                let data = crate::snapshots::limited(
                    &verified
                        .stage
                        .root
                        .join(format!("artifacts/{artifact}.tar")),
                    crate::jobs::MAX_BYTES,
                )?;
                if crate::content::hash(&data) != hash {
                    return Err(crate::content::integrity());
                }
                use std::io::Write;
                let destination = crate::export::directory(self, &ids[&w])?
                    .join(format!("{}.tar", ids[&artifact]));
                crate::snapshots::new_file(&destination)?
                    .write_all(&data)
                    .map_err(|_| crate::storage::unavailable())?;
            }
        }
        for (table, count, rows) in prepared {
            let slots = (1..=count)
                .map(|i| format!("?{i}"))
                .collect::<Vec<_>>()
                .join(",");
            let mut insert = tx
                .prepare(&format!("INSERT INTO {table} VALUES({slots})"))
                .map_err(|_| crate::storage::unavailable())?;
            for row in rows {
                insert
                    .execute(rusqlite::params_from_iter(row))
                    .map_err(|_| crate::jobs::invalid())?;
            }
        }
        for old_workspace in &verified.info.workspaces {
            for (original, new) in &ids {
                if owners.get(original) != Some(old_workspace) {
                    continue;
                }
                tx.execute("INSERT INTO restore_origins(workspace_id,new_id,original_workspace,original_id,backup_id) VALUES(?1,?2,?3,?4,?5)",rusqlite::params![ids[old_workspace],new,old_workspace,original,artifact]).map_err(|_|crate::storage::unavailable())?;
            }
        }
        tx.commit().map_err(|_| crate::jobs::invalid())?;
        Ok(RestoreResult {
            contract_version: 1,
            workspaces,
            backup_hash: verified.preview.sha256,
        })
    }
}

pub fn preview(root: &Path) -> Result<BackupPreview, CommandError> {
    read_and_verify(root)
}

pub fn restore_to_new_target(source: &Path, target: &Path) -> Result<BackupPreview, CommandError> {
    reject_links(source).map_err(|_| crate::jobs::invalid())?;
    reject_links(target).map_err(|_| crate::jobs::invalid())?;
    let source = source.canonicalize().map_err(|_| crate::jobs::invalid())?;
    let parent = target
        .parent()
        .ok_or_else(crate::jobs::invalid)?
        .canonicalize()
        .map_err(|_| crate::jobs::invalid())?;
    let target = parent.join(target.file_name().ok_or_else(crate::jobs::invalid)?);
    if source.starts_with(&target) || target.starts_with(&source) {
        return Err(crate::CommandError::new(
            crate::ErrorCode::InvalidRequest,
            "Restore source and target must be different.",
        ));
    }
    let preview = read_and_verify(&source)?;
    reject_links(&target).map_err(|_| {
        crate::CommandError::new(
            crate::ErrorCode::InvalidRequest,
            "Restore target contains a link.",
        )
    })?;
    if target.exists()
        && fs::read_dir(&target)
            .map_err(|_| crate::storage::unavailable())?
            .next()
            .is_some()
    {
        return Err(crate::CommandError::new(
            crate::ErrorCode::InvalidRequest,
            "Restore target must be new and empty.",
        ));
    }
    fs::create_dir(&target).map_err(|_| crate::storage::unavailable())?;
    crate::storage::private_permissions(&target, true, false)
        .map_err(|_| crate::storage::unavailable())?;
    let mut restored_manifest = preview.manifest.clone();
    let mut id = [0_u8; 16];
    getrandom::fill(&mut id).map_err(|_| crate::storage::unavailable())?;
    restored_manifest.workspace_id = id.iter().map(|byte| format!("{byte:02x}")).collect();
    let manifest_bytes =
        serde_json::to_vec_pretty(&restored_manifest).map_err(|_| crate::storage::unavailable())?;
    use std::io::Write;
    crate::snapshots::new_file(&target.join("manifest.json"))?
        .write_all(&manifest_bytes)
        .map_err(|_| crate::storage::unavailable())?;
    for member in &preview.manifest.members {
        let relative = Path::new(&member.path);
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|_| crate::storage::unavailable())?;
        }
        reject_links(&destination).map_err(|_| crate::jobs::invalid())?;
        let bytes = crate::snapshots::limited(&source.join(relative), crate::jobs::MAX_BYTES)?;
        crate::snapshots::new_file(&destination)?
            .write_all(&bytes)
            .map_err(|_| crate::storage::unavailable())?;
    }
    let verified = read_and_verify(&target)?;
    if verified.manifest.workspace_id == preview.manifest.workspace_id {
        return Err(crate::CommandError::new(
            crate::ErrorCode::InvalidRequest,
            "Restore reused the source workspace identity.",
        ));
    }
    Ok(verified)
}
