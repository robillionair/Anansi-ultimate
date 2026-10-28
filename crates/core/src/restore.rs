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

/// Reference rewriting for one artifact_versions row, after
/// the generic id columns have been remapped: contained
/// `artifact://` source references point at the restored
/// artifacts, and evaluation bodies have their typed source
/// and rubric ids remapped with the content hash recomputed
/// over the rewritten body. Opportunity set bodies (type
/// `idea`) likewise have their embedded memory source ids
/// remapped, and their pinned source revision ids are
/// validated against the package's `memory_revisions` rows
/// (the revision must belong to the referenced memory in
/// the artifact's own workspace) and remapped with it. A
/// reference to an artifact that is not in the package
/// fails closed - a restored artifact must never link back
/// into the source workspace. Bodies of every other type
/// are opaque and preserved exactly.
/// Fix-up pass over the prepared `artifact_versions`
/// rows, after every row's references have been remapped:
/// a restored specification approval must approve the
/// restored specification version's recomputed content
/// hash (its body changed when its own source reference
/// was remapped), not the source workspace's hash. An
/// approval whose specification version is absent from
/// the package fails the restore closed.
fn fix_approval_hashes(
    names: &[String],
    values: &mut [Vec<rusqlite::types::Value>],
) -> Result<(), CommandError> {
    use rusqlite::types::Value;
    let index = |name: &str| -> Result<usize, CommandError> {
        names
            .iter()
            .position(|n| n == name)
            .ok_or_else(crate::jobs::invalid)
    };
    let type_i = index("artifact_type")?;
    let body_i = index("body")?;
    let hash_i = index("content_hash")?;
    let id_i = index("artifact_id")?;
    let version_i = index("version")?;
    let mut spec_hashes: std::collections::BTreeMap<(String, i64), String> =
        std::collections::BTreeMap::new();
    for row in values.iter() {
        if matches!(&row[type_i], Value::Text(t) if t == "specification")
            && let (Value::Text(id), Value::Integer(version), Value::Text(hash)) =
                (&row[id_i], &row[version_i], &row[hash_i])
        {
            spec_hashes.insert((id.clone(), *version), hash.clone());
        }
    }
    for row in values.iter_mut() {
        if !matches!(&row[type_i], Value::Text(t) if t == "specification-approval") {
            continue;
        }
        let Value::Text(body) = &row[body_i].clone() else {
            continue;
        };
        let mut doc: serde_json::Value =
            serde_json::from_str(body).map_err(|_| crate::jobs::invalid())?;
        if doc.get("format").and_then(|v| v.as_str()) != Some("anansi-specification-approval") {
            continue;
        }
        let spec_id = doc
            .get("specArtifactId")
            .and_then(|v| v.as_str())
            .ok_or_else(crate::jobs::invalid)?;
        let spec_version = doc
            .get("specVersion")
            .and_then(|v| v.as_u64())
            .ok_or_else(crate::jobs::invalid)? as i64;
        let hash = spec_hashes
            .get(&(spec_id.to_string(), spec_version))
            .ok_or_else(crate::jobs::invalid)?;
        doc["specContentHash"] = serde_json::Value::String(hash.clone());
        let rewritten = serde_json::to_string(&doc).map_err(|_| crate::jobs::invalid())?;
        row[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
        row[body_i] = Value::Text(rewritten);
    }
    Ok(())
}

/// Fix-up pass over the prepared `artifact_versions`
/// rows, after every row's references have been remapped:
/// a restored deliverable must pin the restored package
/// and specification versions' recomputed content hashes
/// (their bodies changed when their own references were
/// remapped), not the source workspace's hashes. A
/// deliverable whose pinned package or specification
/// version is absent from the package fails the restore
/// closed. Verified-facts text needs no rewrite: it
/// contains no artifact ids or body hashes by design.
fn fix_deliverable_hashes(
    names: &[String],
    values: &mut [Vec<rusqlite::types::Value>],
) -> Result<(), CommandError> {
    use rusqlite::types::Value;
    let index = |name: &str| -> Result<usize, CommandError> {
        names
            .iter()
            .position(|n| n == name)
            .ok_or_else(crate::jobs::invalid)
    };
    let type_i = index("artifact_type")?;
    let body_i = index("body")?;
    let hash_i = index("content_hash")?;
    let id_i = index("artifact_id")?;
    let version_i = index("version")?;
    let mut row_hashes: std::collections::BTreeMap<(String, String, i64), String> =
        std::collections::BTreeMap::new();
    for row in values.iter() {
        if let (Value::Text(kind), Value::Text(id), Value::Integer(version), Value::Text(hash)) =
            (&row[type_i], &row[id_i], &row[version_i], &row[hash_i])
            && (kind == "source-package" || kind == "specification")
        {
            row_hashes.insert((kind.clone(), id.clone(), *version), hash.clone());
        }
    }
    for row in values.iter_mut() {
        if !matches!(&row[type_i], Value::Text(t) if t == "deliverable") {
            continue;
        }
        let Value::Text(body) = &row[body_i].clone() else {
            continue;
        };
        let mut doc: serde_json::Value =
            serde_json::from_str(body).map_err(|_| crate::jobs::invalid())?;
        if doc.get("format").and_then(|v| v.as_str()) != Some("anansi-deliverable") {
            continue;
        }
        let package_id = doc
            .get("packageArtifactId")
            .and_then(|v| v.as_str())
            .ok_or_else(crate::jobs::invalid)?;
        let package_version = doc
            .get("packageVersion")
            .and_then(|v| v.as_u64())
            .ok_or_else(crate::jobs::invalid)? as i64;
        let spec_id = doc
            .get("specArtifactId")
            .and_then(|v| v.as_str())
            .ok_or_else(crate::jobs::invalid)?;
        let spec_version = doc
            .get("specVersion")
            .and_then(|v| v.as_u64())
            .ok_or_else(crate::jobs::invalid)? as i64;
        let package_hash = row_hashes
            .get(&(
                "source-package".to_string(),
                package_id.to_string(),
                package_version,
            ))
            .ok_or_else(crate::jobs::invalid)?;
        let spec_hash = row_hashes
            .get(&(
                "specification".to_string(),
                spec_id.to_string(),
                spec_version,
            ))
            .ok_or_else(crate::jobs::invalid)?;
        doc["packageContentHash"] = serde_json::Value::String(package_hash.clone());
        doc["specContentHash"] = serde_json::Value::String(spec_hash.clone());
        let rewritten = serde_json::to_string(&doc).map_err(|_| crate::jobs::invalid())?;
        row[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
        row[body_i] = Value::Text(rewritten);
    }
    Ok(())
}

/// Fix-up pass over the prepared `artifact_versions`
/// rows, after every row's references have been remapped:
/// a restored source review must pin the restored package
/// and specification versions' recomputed content hashes
/// (their bodies changed when their own references were
/// remapped), not the source workspace's hashes. A review
/// whose pinned package or specification version is
/// absent from the package fails the restore closed.
/// Rewrite each transfer-approval row's pinned
/// concept content hash to the restored concept row's
/// recomputed hash, mirroring the specification
/// approval fix-up. The concept body changes on remap
/// (its experiment id is rewritten), so the original
/// pinned hash would no longer match; the fix-up
/// transfers approval only to the restored equivalent
/// of the same concept version, never to a different
/// version or a fabricated body (pre-validation has
/// already refused those).
fn fix_transfer_receipt_hashes(
    names: &[String],
    values: &mut [Vec<rusqlite::types::Value>],
) -> Result<(), CommandError> {
    use rusqlite::types::Value;
    let index = |name: &str| -> Result<usize, CommandError> {
        names
            .iter()
            .position(|n| n == name)
            .ok_or_else(crate::jobs::invalid)
    };
    let type_i = index("artifact_type")?;
    let body_i = index("body")?;
    let hash_i = index("content_hash")?;
    let id_i = index("artifact_id")?;
    let version_i = index("version")?;
    let mut concept_hashes: std::collections::BTreeMap<(String, i64), String> =
        std::collections::BTreeMap::new();
    for row in values.iter() {
        if matches!(&row[type_i], Value::Text(t) if t == "concept")
            && let (Value::Text(id), Value::Integer(version), Value::Text(hash)) =
                (&row[id_i], &row[version_i], &row[hash_i])
        {
            concept_hashes.insert((id.clone(), *version), hash.clone());
        }
    }
    for row in values.iter_mut() {
        if !matches!(&row[type_i], Value::Text(t) if t == "transfer-approval") {
            continue;
        }
        let Value::Text(body) = &row[body_i].clone() else {
            continue;
        };
        let mut doc: serde_json::Value =
            serde_json::from_str(body).map_err(|_| crate::jobs::invalid())?;
        if doc.get("format").and_then(|v| v.as_str()) != Some("anansi-transfer-approval") {
            continue;
        }
        let concept_id = doc
            .get("conceptArtifactId")
            .and_then(|v| v.as_str())
            .ok_or_else(crate::jobs::invalid)?;
        let concept_version = doc
            .get("conceptVersion")
            .and_then(|v| v.as_u64())
            .ok_or_else(crate::jobs::invalid)? as i64;
        let hash = concept_hashes
            .get(&(concept_id.to_string(), concept_version))
            .ok_or_else(crate::jobs::invalid)?;
        doc["conceptContentHash"] = serde_json::Value::String(hash.clone());
        let rewritten = serde_json::to_string(&doc).map_err(|_| crate::jobs::invalid())?;
        row[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
        row[body_i] = Value::Text(rewritten);
    }
    Ok(())
}

/// Fix-up pass over the prepared `artifact_versions`
/// rows, after every row's references have been
/// remapped: a transferred concept — and its transfer
/// approval receipt — pin their source experiment by
/// content hash, and a context-bearing experiment's body
/// changes on restore (its memory ids remap), so its
/// hash changes too. The pinned experiment hash is
/// rewritten to the restored experiment row's recomputed
/// hash for the pinned version, so the binding follows
/// the restored equivalent instead of breaking. This
/// mirrors the specification-approval and receipt
/// concept-hash fix-ups; it never recomputes a hash
/// from an unvalidated original, because pre-validation
/// already approved the original bindings.
fn fix_transfer_experiment_hashes(
    names: &[String],
    values: &mut [Vec<rusqlite::types::Value>],
) -> Result<(), CommandError> {
    use rusqlite::types::Value;
    let index = |name: &str| -> Result<usize, CommandError> {
        names
            .iter()
            .position(|n| n == name)
            .ok_or_else(crate::jobs::invalid)
    };
    let type_i = index("artifact_type")?;
    let body_i = index("body")?;
    let hash_i = index("content_hash")?;
    let id_i = index("artifact_id")?;
    let version_i = index("version")?;
    let mut experiment_hashes: std::collections::BTreeMap<(String, i64), String> =
        std::collections::BTreeMap::new();
    for row in values.iter() {
        if matches!(&row[type_i], Value::Text(t) if t == "experiment")
            && let (Value::Text(id), Value::Integer(version), Value::Text(hash)) =
                (&row[id_i], &row[version_i], &row[hash_i])
        {
            experiment_hashes.insert((id.clone(), *version), hash.clone());
        }
    }
    for row in values.iter_mut() {
        let is_concept = matches!(&row[type_i], Value::Text(t) if t == "concept");
        let is_receipt = matches!(&row[type_i], Value::Text(t) if t == "transfer-approval");
        if !is_concept && !is_receipt {
            continue;
        }
        let Value::Text(body) = &row[body_i].clone() else {
            continue;
        };
        let mut doc: serde_json::Value =
            serde_json::from_str(body).map_err(|_| crate::jobs::invalid())?;
        let format = doc.get("format").and_then(|v| v.as_str());
        if (is_concept && format != Some("anansi-forge-concept"))
            || (is_receipt && format != Some("anansi-transfer-approval"))
        {
            continue;
        }
        let experiment_id = doc
            .get("sourceExperimentArtifactId")
            .and_then(|v| v.as_str())
            .ok_or_else(crate::jobs::invalid)?;
        let experiment_version = doc
            .get("sourceExperimentVersion")
            .and_then(|v| v.as_u64())
            .ok_or_else(crate::jobs::invalid)? as i64;
        let hash = experiment_hashes
            .get(&(experiment_id.to_string(), experiment_version))
            .ok_or_else(crate::jobs::invalid)?;
        doc["sourceExperimentContentHash"] = serde_json::Value::String(hash.clone());
        let rewritten = serde_json::to_string(&doc).map_err(|_| crate::jobs::invalid())?;
        row[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
        row[body_i] = Value::Text(rewritten);
    }
    Ok(())
}

fn fix_review_hashes(
    names: &[String],
    values: &mut [Vec<rusqlite::types::Value>],
) -> Result<(), CommandError> {
    use rusqlite::types::Value;
    let index = |name: &str| -> Result<usize, CommandError> {
        names
            .iter()
            .position(|n| n == name)
            .ok_or_else(crate::jobs::invalid)
    };
    let type_i = index("artifact_type")?;
    let body_i = index("body")?;
    let hash_i = index("content_hash")?;
    let id_i = index("artifact_id")?;
    let version_i = index("version")?;
    let mut row_hashes: std::collections::BTreeMap<(String, String, i64), String> =
        std::collections::BTreeMap::new();
    for row in values.iter() {
        if let (Value::Text(kind), Value::Text(id), Value::Integer(version), Value::Text(hash)) =
            (&row[type_i], &row[id_i], &row[version_i], &row[hash_i])
            && (kind == "source-package" || kind == "specification")
        {
            row_hashes.insert((kind.clone(), id.clone(), *version), hash.clone());
        }
    }
    for row in values.iter_mut() {
        if !matches!(&row[type_i], Value::Text(t) if t == "source-review") {
            continue;
        }
        let Value::Text(body) = &row[body_i].clone() else {
            continue;
        };
        let mut doc: serde_json::Value =
            serde_json::from_str(body).map_err(|_| crate::jobs::invalid())?;
        if doc.get("format").and_then(|v| v.as_str()) != Some("anansi-source-review") {
            continue;
        }
        let package_id = doc
            .get("packageArtifactId")
            .and_then(|v| v.as_str())
            .ok_or_else(crate::jobs::invalid)?;
        let package_version = doc
            .get("packageVersion")
            .and_then(|v| v.as_u64())
            .ok_or_else(crate::jobs::invalid)? as i64;
        let spec_id = doc
            .get("specArtifactId")
            .and_then(|v| v.as_str())
            .ok_or_else(crate::jobs::invalid)?;
        let spec_version = doc
            .get("specVersion")
            .and_then(|v| v.as_u64())
            .ok_or_else(crate::jobs::invalid)? as i64;
        let package_hash = row_hashes
            .get(&(
                "source-package".to_string(),
                package_id.to_string(),
                package_version,
            ))
            .ok_or_else(crate::jobs::invalid)?;
        let spec_hash = row_hashes
            .get(&(
                "specification".to_string(),
                spec_id.to_string(),
                spec_version,
            ))
            .ok_or_else(crate::jobs::invalid)?;
        doc["packageContentHash"] = serde_json::Value::String(package_hash.clone());
        doc["specContentHash"] = serde_json::Value::String(spec_hash.clone());
        let rewritten = serde_json::to_string(&doc).map_err(|_| crate::jobs::invalid())?;
        row[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
        row[body_i] = Value::Text(rewritten);
    }
    Ok(())
}

/// Slice 3 repair: validate original deliverable
/// bindings in the source catalog before any id or
/// hash is changed. A deliverable's pinned package
/// and specification must exist in the backup, be of
/// the right types, live in the deliverable's own
/// workspace and project, and carry the original
/// pinned hashes — otherwise the hash fix-up pass
/// would silently repair an invalid original binding
/// into a valid restored one. This runs before the
/// prepared-row loop and before any destination state
/// is written, so a refusal publishes nothing.
fn validate_original_deliverable_bindings(
    source: &rusqlite::Connection,
) -> Result<(), CommandError> {
    struct Row {
        workspace_id: String,
        project_id: String,
        kind: String,
        body: String,
        content_hash: String,
    }
    let mut pinned: std::collections::BTreeMap<(String, String, i64), Row> =
        std::collections::BTreeMap::new();
    let mut deliverables: Vec<Row> = Vec::new();
    {
        let mut q = source
            .prepare(
                "SELECT workspace_id, artifact_id, project_id, version, artifact_type, body, content_hash FROM artifact_versions",
            )
            .map_err(|_| crate::jobs::invalid())?;
        let mut rows = q.query([]).map_err(|_| crate::jobs::invalid())?;
        while let Some(row) = rows.next().map_err(|_| crate::jobs::invalid())? {
            let workspace_id: String = row.get(0).map_err(|_| crate::jobs::invalid())?;
            let artifact_id: String = row.get(1).map_err(|_| crate::jobs::invalid())?;
            let project_id: String = row.get(2).map_err(|_| crate::jobs::invalid())?;
            let version: i64 = row.get(3).map_err(|_| crate::jobs::invalid())?;
            let kind: String = row.get(4).map_err(|_| crate::jobs::invalid())?;
            let body: String = row.get(5).map_err(|_| crate::jobs::invalid())?;
            let content_hash: String = row.get(6).map_err(|_| crate::jobs::invalid())?;
            let entry = Row {
                workspace_id,
                project_id,
                kind: kind.clone(),
                body,
                content_hash,
            };
            if kind == "deliverable" {
                deliverables.push(entry);
            } else if kind == "source-package" || kind == "specification" {
                pinned.insert((entry.workspace_id.clone(), artifact_id, version), entry);
            }
        }
    }
    for deliverable in &deliverables {
        let value: serde_json::Value =
            serde_json::from_str(&deliverable.body).map_err(|_| crate::jobs::invalid())?;
        if value.get("format").and_then(|v| v.as_str()) != Some("anansi-deliverable") {
            continue;
        }
        let doc: crate::forge::DeliverableDoc =
            serde_json::from_str(&deliverable.body).map_err(|_| crate::jobs::invalid())?;
        if crate::content::hash(deliverable.body.as_bytes()) != deliverable.content_hash {
            return Err(crate::jobs::invalid());
        }
        let package = pinned
            .get(&(
                deliverable.workspace_id.clone(),
                doc.package_artifact_id.clone(),
                doc.package_version as i64,
            ))
            .ok_or_else(crate::jobs::invalid)?;
        if package.kind != "source-package"
            || package.workspace_id != deliverable.workspace_id
            || package.project_id != deliverable.project_id
            || crate::content::hash(package.body.as_bytes()) != package.content_hash
        {
            return Err(crate::jobs::invalid());
        }
        let package_doc: crate::forge::SourcePackageDoc =
            serde_json::from_str(&package.body).map_err(|_| crate::jobs::invalid())?;
        if package_doc.format != "anansi-source-package" {
            return Err(crate::jobs::invalid());
        }
        let spec = pinned
            .get(&(
                deliverable.workspace_id.clone(),
                doc.spec_artifact_id.clone(),
                doc.spec_version as i64,
            ))
            .ok_or_else(crate::jobs::invalid)?;
        if spec.kind != "specification"
            || spec.workspace_id != deliverable.workspace_id
            || spec.project_id != deliverable.project_id
            || crate::content::hash(spec.body.as_bytes()) != spec.content_hash
        {
            return Err(crate::jobs::invalid());
        }
        crate::forge::validate_deliverable_record(
            &doc,
            &package_doc,
            &crate::content::hash(package.body.as_bytes()),
            &spec.content_hash,
        )?;
    }
    Ok(())
}

/// Slice 4: validate original source-review bindings
/// in the source catalog before any id or hash is
/// changed, mirroring the deliverable pre-validation.
/// A review's pinned package and specification must
/// exist in the backup, be of the right types, live
/// in the review's own workspace and project, carry
/// the original pinned hashes, and validate as a
/// record against the original package/spec bodies —
/// otherwise the hash fix-up pass would silently
/// repair an invalid original binding into a valid
/// restored one. This runs before the prepared-row
/// loop and before any destination state is written,
/// so a refusal publishes nothing.
fn validate_original_review_bindings(source: &rusqlite::Connection) -> Result<(), CommandError> {
    struct Row {
        workspace_id: String,
        project_id: String,
        kind: String,
        body: String,
        content_hash: String,
    }
    let mut pinned: std::collections::BTreeMap<(String, String, i64), Row> =
        std::collections::BTreeMap::new();
    let mut reviews: Vec<Row> = Vec::new();
    {
        let mut q = source
            .prepare(
                "SELECT workspace_id, artifact_id, project_id, version, artifact_type, body, content_hash FROM artifact_versions",
            )
            .map_err(|_| crate::jobs::invalid())?;
        let mut rows = q.query([]).map_err(|_| crate::jobs::invalid())?;
        while let Some(row) = rows.next().map_err(|_| crate::jobs::invalid())? {
            let workspace_id: String = row.get(0).map_err(|_| crate::jobs::invalid())?;
            let artifact_id: String = row.get(1).map_err(|_| crate::jobs::invalid())?;
            let project_id: String = row.get(2).map_err(|_| crate::jobs::invalid())?;
            let version: i64 = row.get(3).map_err(|_| crate::jobs::invalid())?;
            let kind: String = row.get(4).map_err(|_| crate::jobs::invalid())?;
            let body: String = row.get(5).map_err(|_| crate::jobs::invalid())?;
            let content_hash: String = row.get(6).map_err(|_| crate::jobs::invalid())?;
            let entry = Row {
                workspace_id,
                project_id,
                kind: kind.clone(),
                body,
                content_hash,
            };
            if kind == "source-review" {
                reviews.push(entry);
            } else if kind == "source-package" || kind == "specification" {
                pinned.insert((entry.workspace_id.clone(), artifact_id, version), entry);
            }
        }
    }
    for review in &reviews {
        let value: serde_json::Value =
            serde_json::from_str(&review.body).map_err(|_| crate::jobs::invalid())?;
        if value.get("format").and_then(|v| v.as_str()) != Some("anansi-source-review") {
            // The registry type already identifies this as a source review.
            // A changed discriminator must not bypass validation of its pins.
            return Err(crate::jobs::invalid());
        }
        let doc: crate::forge::SourceReviewDoc =
            serde_json::from_str(&review.body).map_err(|_| crate::jobs::invalid())?;
        if crate::content::hash(review.body.as_bytes()) != review.content_hash {
            return Err(crate::jobs::invalid());
        }
        let package = pinned
            .get(&(
                review.workspace_id.clone(),
                doc.package_artifact_id.clone(),
                doc.package_version as i64,
            ))
            .ok_or_else(crate::jobs::invalid)?;
        if package.kind != "source-package"
            || package.workspace_id != review.workspace_id
            || package.project_id != review.project_id
            || crate::content::hash(package.body.as_bytes()) != package.content_hash
        {
            return Err(crate::jobs::invalid());
        }
        let package_doc: crate::forge::SourcePackageDoc =
            serde_json::from_str(&package.body).map_err(|_| crate::jobs::invalid())?;
        if package_doc.format != "anansi-source-package" {
            return Err(crate::jobs::invalid());
        }
        let spec = pinned
            .get(&(
                review.workspace_id.clone(),
                doc.spec_artifact_id.clone(),
                doc.spec_version as i64,
            ))
            .ok_or_else(crate::jobs::invalid)?;
        if spec.kind != "specification"
            || spec.workspace_id != review.workspace_id
            || spec.project_id != review.project_id
            || crate::content::hash(spec.body.as_bytes()) != spec.content_hash
        {
            return Err(crate::jobs::invalid());
        }
        let spec_doc: crate::forge::ProductSpecDoc =
            serde_json::from_str(&spec.body).map_err(|_| crate::jobs::invalid())?;
        if spec_doc.format != "anansi-product-spec" {
            return Err(crate::jobs::invalid());
        }
        crate::forge::validate_review_record(
            &doc,
            &package_doc,
            &spec_doc,
            &crate::content::hash(package.body.as_bytes()),
            &spec.content_hash,
        )?;
    }
    Ok(())
}

/// M8 Slice 2, acceptance repair: a transferred concept
/// must validate its original experiment binding before
/// any destination write. The pinned experiment row
/// must exist in the backup in the same workspace and
/// project, be a collider experiment, and recompute to
/// both its row hash and the pinned source hash; every
/// model-derived field — selected combination,
/// assumptions, ordered unknowns, productization brief,
/// provider and model — must match the pinned
/// experiment version. A transfer whose experiment is
/// absent, tampered or mismatched refuses the whole
/// restore with nothing published, and a fabricated
/// original is never repaired by recomputing hashes.
/// The transfer's version-1 approval receipt is
/// validated the same way: its pinned concept row must
/// exist with a matching hash, and every receipt field
/// must equal the concept body. Generic `concept` rows
/// (manual artifacts with other bodies) restore
/// untouched: only format-matching rows are gated.
fn validate_original_transfer_bindings(source: &rusqlite::Connection) -> Result<(), CommandError> {
    struct Row {
        workspace_id: String,
        project_id: String,
        body: String,
        content_hash: String,
    }
    let mut experiments: std::collections::BTreeMap<(String, String, i64), Row> =
        std::collections::BTreeMap::new();
    let mut transfers: Vec<(String, i64, Row)> = Vec::new();
    let mut receipts: Vec<Row> = Vec::new();
    {
        let mut q = source
            .prepare(
                "SELECT workspace_id, artifact_id, project_id, version, artifact_type, body, content_hash FROM artifact_versions",
            )
            .map_err(|_| crate::jobs::invalid())?;
        let mut rows = q.query([]).map_err(|_| crate::jobs::invalid())?;
        while let Some(row) = rows.next().map_err(|_| crate::jobs::invalid())? {
            let workspace_id: String = row.get(0).map_err(|_| crate::jobs::invalid())?;
            let artifact_id: String = row.get(1).map_err(|_| crate::jobs::invalid())?;
            let project_id: String = row.get(2).map_err(|_| crate::jobs::invalid())?;
            let version: i64 = row.get(3).map_err(|_| crate::jobs::invalid())?;
            let kind: String = row.get(4).map_err(|_| crate::jobs::invalid())?;
            let body: String = row.get(5).map_err(|_| crate::jobs::invalid())?;
            let content_hash: String = row.get(6).map_err(|_| crate::jobs::invalid())?;
            let entry = Row {
                workspace_id: workspace_id.clone(),
                project_id,
                body,
                content_hash,
            };
            if kind == "concept" {
                transfers.push((artifact_id, version, entry));
            } else if kind == "experiment" {
                experiments.insert((workspace_id, artifact_id, version), entry);
            } else if kind == "transfer-approval" {
                receipts.push(entry);
            }
        }
    }
    let mut concepts: std::collections::BTreeMap<(String, i64), &Row> =
        std::collections::BTreeMap::new();
    for transfer in &transfers {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&transfer.2.body) else {
            continue;
        };
        if value.get("format").and_then(|v| v.as_str()) != Some("anansi-forge-concept") {
            continue;
        }
        if crate::content::hash(transfer.2.body.as_bytes()) != transfer.2.content_hash {
            return Err(crate::jobs::invalid());
        }
        let doc: crate::transfer::TransferConceptDoc =
            serde_json::from_str(&transfer.2.body).map_err(|_| crate::jobs::invalid())?;
        crate::transfer::validate_transfer_concept(&transfer.2.body)
            .map_err(|_| crate::jobs::invalid())?;
        let experiment = experiments
            .get(&(
                transfer.2.workspace_id.clone(),
                doc.source_experiment_artifact_id.clone(),
                doc.source_experiment_version as i64,
            ))
            .ok_or_else(crate::jobs::invalid)?;
        if experiment.workspace_id != transfer.2.workspace_id
            || experiment.project_id != transfer.2.project_id
            || crate::content::hash(experiment.body.as_bytes()) != experiment.content_hash
            || experiment.content_hash != doc.source_experiment_content_hash
        {
            return Err(crate::jobs::invalid());
        }
        let experiment_doc: crate::collider::ColliderExperimentDoc =
            serde_json::from_str(&experiment.body).map_err(|_| crate::jobs::invalid())?;
        crate::collider::validate_experiment_doc(&experiment_doc)
            .map_err(|_| crate::jobs::invalid())?;
        let expected = crate::transfer::expected_transfer_fields(
            &experiment_doc,
            doc.selected_combination_index,
        )
        .map_err(|_| crate::jobs::invalid())?;
        if expected.combination.name != doc.selected_combination.name
            || expected.combination.description != doc.selected_combination.description
            || expected.assumptions != doc.assumptions
            || expected.unknowns != doc.unknowns
            || expected.productization_brief != doc.productization_brief
            || expected.provider != doc.provider
            || expected.model != doc.model
        {
            return Err(crate::jobs::invalid());
        }
        concepts.insert((transfer.0.clone(), transfer.1), &transfer.2);
    }
    for receipt in &receipts {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&receipt.body) else {
            continue;
        };
        if value.get("format").and_then(|v| v.as_str()) != Some("anansi-transfer-approval") {
            continue;
        }
        if crate::content::hash(receipt.body.as_bytes()) != receipt.content_hash {
            return Err(crate::jobs::invalid());
        }
        let approval: crate::transfer::TransferApprovalDoc =
            serde_json::from_str(&receipt.body).map_err(|_| crate::jobs::invalid())?;
        crate::transfer::validate_transfer_approval_doc(&receipt.body)
            .map_err(|_| crate::jobs::invalid())?;
        let concept = concepts
            .get(&(
                approval.concept_artifact_id.clone(),
                approval.concept_version as i64,
            ))
            .ok_or_else(crate::jobs::invalid)?;
        if concept.content_hash != approval.concept_content_hash
            || receipt.workspace_id != concept.workspace_id
            || receipt.project_id != concept.project_id
        {
            return Err(crate::jobs::invalid());
        }
        let doc: crate::transfer::TransferConceptDoc =
            serde_json::from_str(&concept.body).map_err(|_| crate::jobs::invalid())?;
        if approval.approved_brief != doc.approved_brief
            || approval.source_experiment_artifact_id != doc.source_experiment_artifact_id
            || approval.source_experiment_version != doc.source_experiment_version
            || approval.source_experiment_content_hash != doc.source_experiment_content_hash
            || approval.selected_combination_index != doc.selected_combination_index
            || approval.selected_combination.name != doc.selected_combination.name
            || approval.selected_combination.description != doc.selected_combination.description
            || approval.assumptions != doc.assumptions
            || approval.unknowns != doc.unknowns
            || approval.productization_brief != doc.productization_brief
            || approval.provider != doc.provider
            || approval.model != doc.model
        {
            return Err(crate::jobs::invalid());
        }
    }
    Ok(())
}

/// M8 Slice 1: a saved collider experiment whose body
/// declares the collider format must fully validate
/// before any destination write, mirroring the review
/// format guard. Generic `experiment` rows (manual
/// artifacts with other bodies) restore untouched: the
/// registry type alone does not identify a collider
/// experiment, so only format-matching rows are gated.
/// A tampered collider body refuses the whole restore
/// with nothing published.
fn validate_original_experiment_bindings(
    source: &rusqlite::Connection,
) -> Result<(), CommandError> {
    let mut q = source
        .prepare(
            "SELECT workspace_id, body, content_hash FROM artifact_versions WHERE artifact_type = 'experiment'",
        )
        .map_err(|_| crate::jobs::invalid())?;
    let mut rows = q.query([]).map_err(|_| crate::jobs::invalid())?;
    while let Some(row) = rows.next().map_err(|_| crate::jobs::invalid())? {
        let artifact_workspace: String = row.get(0).map_err(|_| crate::jobs::invalid())?;
        let body: String = row.get(1).map_err(|_| crate::jobs::invalid())?;
        let content_hash: String = row.get(2).map_err(|_| crate::jobs::invalid())?;
        // A body that is not JSON at all cannot be a
        // collider record: generic manual experiments
        // restore untouched.
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) else {
            continue;
        };
        if value.get("format").and_then(|v| v.as_str()) != Some("anansi-collider-experiment") {
            continue;
        }
        if crate::content::hash(body.as_bytes()) != content_hash {
            return Err(crate::jobs::invalid());
        }
        let doc: crate::collider::ColliderExperimentDoc =
            serde_json::from_str(&body).map_err(|_| crate::jobs::invalid())?;
        crate::collider::validate_experiment_doc(&doc).map_err(|_| crate::jobs::invalid())?;
        // M8 Slice 3: every pinned memory context entry
        // must resolve to a real revision row in the
        // source package — same memory, same workspace,
        // matching body hash, non-Sealed metadata — and
        // the snapshot content must hash to the pinned
        // hash. A fabricated or Sealed original refuses
        // the whole restore before any destination state
        // is published; nothing here repairs a hash.
        for entry in &doc.memory_context {
            let found: Option<(String, String, String, String)> = source
                .query_row(
                    "SELECT memory_id, workspace_id, body_hash, metadata FROM memory_revisions WHERE id = ?1",
                    [entry.revision_id.clone()],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, String>(3)?,
                        ))
                    },
                )
                .ok();
            let Some((owner_memory, owner_workspace, row_hash, raw_meta)) = found else {
                return Err(crate::jobs::invalid());
            };
            if owner_memory != entry.memory_id
                || owner_workspace != artifact_workspace
                || row_hash != entry.content_hash
                || crate::content::hash(entry.content.as_bytes()) != entry.content_hash
            {
                return Err(crate::jobs::invalid());
            }
            let meta: serde_json::Value =
                serde_json::from_str(&raw_meta).map_err(|_| crate::jobs::invalid())?;
            if meta.get("privacy").and_then(|v| v.as_str()) == Some("sealed") {
                return Err(crate::jobs::invalid());
            }
        }
    }
    Ok(())
}

fn remap_artifact_row(
    names: &[String],
    columns: &mut [rusqlite::types::Value],
    ids: &std::collections::BTreeMap<String, String>,
    revision_owners: &std::collections::BTreeMap<String, (String, String)>,
    old_workspace: &str,
) -> Result<(), CommandError> {
    use rusqlite::types::Value;
    let index = |name: &str| -> Result<usize, CommandError> {
        names
            .iter()
            .position(|n| n == name)
            .ok_or_else(crate::jobs::invalid)
    };
    let refs_i = index("source_references")?;
    if let Value::Text(raw) = &columns[refs_i] {
        let refs: Vec<String> = serde_json::from_str(raw).map_err(|_| crate::jobs::invalid())?;
        let mut out = Vec::new();
        for reference in refs {
            if let Some(rest) = reference.strip_prefix("artifact://") {
                let (id, version) = rest.rsplit_once("@v").ok_or_else(crate::jobs::invalid)?;
                let new = ids.get(id).ok_or_else(crate::jobs::invalid)?;
                out.push(format!("artifact://{new}@v{version}"));
            } else if reference.starts_with("artifact:") {
                return Err(crate::jobs::invalid());
            } else {
                out.push(reference);
            }
        }
        columns[refs_i] =
            Value::Text(serde_json::to_string(&out).map_err(|_| crate::jobs::invalid())?);
    }
    let type_i = index("artifact_type")?;
    let is_evaluation = matches!(&columns[type_i], Value::Text(t) if t == "evaluation");
    if is_evaluation {
        let body_i = index("body")?;
        let hash_i = index("content_hash")?;
        if let Value::Text(body) = &columns[body_i] {
            let rewritten = crate::evaluations::remap_evaluation_references(body, ids)?;
            columns[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
            columns[body_i] = Value::Text(rewritten);
        }
    }
    // Product Forge bodies pin other artifacts by id
    // (specification → its source set; source-package → its
    // specification and source set): those resolve to the
    // restored artifacts, fail closed when a reference is
    // not in the package, and the hash is recomputed when
    // the body changed.
    let is_specification = matches!(&columns[type_i], Value::Text(t) if t == "specification");
    let is_package = matches!(&columns[type_i], Value::Text(t) if t == "source-package");
    if is_specification || is_package {
        let body_i = index("body")?;
        let hash_i = index("content_hash")?;
        if let Value::Text(body) = &columns[body_i] {
            let rewritten = if is_specification {
                crate::forge::remap_specification_references(body, ids)?
            } else {
                crate::forge::remap_source_package_references(body, ids)?
            };
            if let Some(rewritten) = rewritten {
                columns[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
                columns[body_i] = Value::Text(rewritten);
            }
        }
    }
    // Slice 3 deliverables (type `deliverable`) pin a
    // source package and a specification by id: both
    // resolve to the restored artifacts, fail closed when
    // either is not in the package, and the pinned hashes
    // are rewritten afterwards (see the fix-up pass) to
    // the restored rows' recomputed hashes.
    let is_deliverable = matches!(&columns[type_i], Value::Text(t) if t == "deliverable");
    if is_deliverable {
        let body_i = index("body")?;
        let hash_i = index("content_hash")?;
        if let Value::Text(body) = &columns[body_i]
            && let Some(rewritten) = crate::forge::remap_deliverable_references(body, ids)?
        {
            columns[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
            columns[body_i] = Value::Text(rewritten);
        }
    }
    // Slice 4 source reviews (type `source-review`) pin a
    // source package and a specification by id: both
    // resolve to the restored artifacts, fail closed when
    // either is not in the package, and the pinned hashes
    // are rewritten afterwards (see the fix-up pass) to
    // the restored rows' recomputed hashes.
    let is_review = matches!(&columns[type_i], Value::Text(t) if t == "source-review");
    if is_review {
        let body_i = index("body")?;
        let hash_i = index("content_hash")?;
        if let Value::Text(body) = &columns[body_i]
            && let Some(rewritten) = crate::forge::remap_review_references(body, ids)?
        {
            columns[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
            columns[body_i] = Value::Text(rewritten);
        }
    }
    // M8 Slice 2 transferred concepts (type `concept`,
    // body format `anansi-forge-concept`) pin a collider
    // experiment by id: it resolves to the restored
    // experiment, fail closed when it is not in the
    // package. The experiment's own content hash is
    // unchanged by restore (its body carries no self-id),
    // so the pinned source hash stays valid after remap;
    // the row hash is recomputed here because the body
    // changed.
    let is_transfer = matches!(&columns[type_i], Value::Text(t) if t == "concept");
    if is_transfer {
        let body_i = index("body")?;
        let hash_i = index("content_hash")?;
        if let Value::Text(body) = &columns[body_i]
            && let Some(rewritten) = crate::transfer::remap_transfer_references(body, ids)?
        {
            columns[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
            columns[body_i] = Value::Text(rewritten);
        }
    }
    // Transfer approvals (type `transfer-approval`)
    // pin a concept and an experiment by id: both
    // resolve to their restored equivalents, fail
    // closed when either is not in the package. The
    // pinned concept content hash is rewritten
    // afterwards (see the fix-up pass) because the
    // concept body changes on remap; the row hash is
    // recomputed here because the body changed.
    let is_receipt = matches!(&columns[type_i], Value::Text(t) if t == "transfer-approval");
    if is_receipt {
        let body_i = index("body")?;
        let hash_i = index("content_hash")?;
        if let Value::Text(body) = &columns[body_i]
            && let Some(rewritten) = crate::transfer::remap_transfer_approval_references(body, ids)?
        {
            columns[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
            columns[body_i] = Value::Text(rewritten);
        }
    }
    // Specification approvals (type
    // `specification-approval`) pin a specification by id:
    // that resolves to the restored specification, fail
    // closed when it is not in the package. The approval's
    // content hash is rewritten afterwards (see the
    // fix-up pass over the prepared artifact rows) to the
    // restored specification version's recomputed hash, so
    // the approval transfers only to the restored
    // equivalent of the same version.
    let is_approval = matches!(&columns[type_i], Value::Text(t) if t == "specification-approval");
    if is_approval {
        let body_i = index("body")?;
        let hash_i = index("content_hash")?;
        if let Value::Text(body) = &columns[body_i]
            && let Some(rewritten) =
                crate::forge::remap_specification_approval_references(body, ids)?
        {
            columns[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
            columns[body_i] = Value::Text(rewritten);
        }
    }
    // M8 Slice 3 collider experiments (type `experiment`,
    // body format `anansi-collider-experiment`) pin
    // memory context by memory and revision id: both
    // resolve to the restored memory/revision, fail
    // closed when either is not in the package, and the
    // revision's owner must match the pinned memory in
    // the artifact's own workspace. Snapshot content and
    // pinned hashes stay verbatim; the row hash is
    // recomputed when the body changed.
    let is_experiment = matches!(&columns[type_i], Value::Text(t) if t == "experiment");
    if is_experiment {
        let body_i = index("body")?;
        let hash_i = index("content_hash")?;
        if let Value::Text(body) = &columns[body_i]
            && let Some(rewritten) = crate::collider::remap_experiment_memory_references(
                body,
                ids,
                revision_owners,
                old_workspace,
            )?
        {
            columns[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
            columns[body_i] = Value::Text(rewritten);
        }
    }
    // Opportunity sets (type `idea`, body format
    // `anansi-opportunity-set`) embed the memory ids of their
    // memory sources: those resolve to the restored memories,
    // with the hash recomputed when the body changed. Their
    // exact-revision provenance is validated against the
    // package's revision rows and remapped with the memory.
    let is_idea = matches!(&columns[type_i], Value::Text(t) if t == "idea");
    if is_idea {
        let body_i = index("body")?;
        let hash_i = index("content_hash")?;
        if let Value::Text(body) = &columns[body_i]
            && let Some(rewritten) = crate::opportunities::remap_opportunity_references(
                body,
                ids,
                revision_owners,
                old_workspace,
            )?
        {
            columns[hash_i] = Value::Text(crate::content::hash(rewritten.as_bytes()));
            columns[body_i] = Value::Text(rewritten);
        }
    }
    Ok(())
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
        {
            let connection = self
                .connection(false)?
                .ok_or_else(crate::storage::unavailable)?;
            let prior: Option<String> = connection
                .query_row(
                    "SELECT workspace_id FROM restore_origins WHERE backup_id=?1 LIMIT 1",
                    [artifact],
                    |r| r.get(0),
                )
                .ok();
            if let Some(existing) = prior {
                let existing_name: String = connection
                    .query_row(
                        "SELECT name FROM workspaces WHERE id=?1",
                        [&existing],
                        |r| r.get(0),
                    )
                    .unwrap_or_else(|_| existing.clone());
                return Err(CommandError::new(
                    crate::ErrorCode::RevisionConflict,
                    &format!(
                        "This backup was already restored as workspace '{existing_name}'. Import the package again as a new backup to restore another copy."
                    ),
                ));
            }
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
            "SELECT DISTINCT artifact_id,workspace_id FROM artifact_versions",
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
        // Revision ownership from the package's own rows:
        // old revision id -> (old memory id, old workspace
        // id). Body-level revision references (opportunity
        // source provenance) are validated against this, not
        // merely against map membership.
        let mut revision_owners: std::collections::BTreeMap<String, (String, String)> =
            std::collections::BTreeMap::new();
        {
            let mut q = source
                .prepare("SELECT id,memory_id,workspace_id FROM memory_revisions")
                .map_err(|_| crate::jobs::invalid())?;
            for row in q
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .map_err(|_| crate::jobs::invalid())?
            {
                let (revision, memory, workspace) = row.map_err(|_| crate::jobs::invalid())?;
                revision_owners.insert(revision, (memory, workspace));
            }
        }
        // Slice 3 repair: validate original deliverable
        // bindings before any id or hash is changed.
        // A refusal here publishes no destination state.
        validate_original_deliverable_bindings(&source)?;
        validate_original_review_bindings(&source)?;
        validate_original_experiment_bindings(&source)?;
        validate_original_transfer_bindings(&source)?;
        // Validate and prepare every mapped row before any managed content is written.
        let mut tables = vec![
            "workspaces",
            "workspace_settings",
            "memories",
            "memory_revisions",
            "relationships",
            "relationship_revisions",
            "artifact_versions",
        ];
        if verified.info.application {
            tables.extend(["jobs", "backup_schedules"]);
        }
        let mut prepared = Vec::new();
        let mut workspaces = Vec::new();
        let mut revision_meta: std::collections::BTreeMap<
            String,
            (String, crate::privacy::Privacy),
        > = std::collections::BTreeMap::new();
        let mut memory_current: Vec<(String, String, String)> = Vec::new();
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
                // Artifact bodies are validated against the
                // row's ORIGINAL workspace; capture it before
                // the generic id remap rewrites the column.
                let artifact_workspace = if table == "artifact_versions" {
                    let i = names
                        .iter()
                        .position(|n| n == "workspace_id")
                        .ok_or_else(crate::jobs::invalid)?;
                    match &columns[i] {
                        rusqlite::types::Value::Text(t) => Some(t.clone()),
                        _ => return Err(crate::jobs::invalid()),
                    }
                } else {
                    None
                };
                for (column, value) in names.iter().zip(columns.iter_mut()) {
                    if let rusqlite::types::Value::Text(text) = value {
                        if [
                            "id",
                            "workspace_id",
                            "memory_id",
                            "current_revision",
                            "relationship_id",
                            "project_id",
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
                                    if let Some(mapped) = ids.get(id) {
                                        *id = mapped.clone();
                                    }
                                }
                                crate::jobs::ExportSelection::Records { ids: records } => {
                                    for id in records {
                                        if let Some(mapped) = ids.get(id) {
                                            *id = mapped.clone();
                                        }
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
                if table == "artifact_versions" {
                    remap_artifact_row(
                        &names,
                        &mut columns,
                        &ids,
                        &revision_owners,
                        artifact_workspace
                            .as_deref()
                            .ok_or_else(crate::jobs::invalid)?,
                    )?;
                }
                if table == "memory_revisions" {
                    let id_i = names
                        .iter()
                        .position(|n| n == "id")
                        .ok_or_else(crate::jobs::invalid)?;
                    let meta_i = names
                        .iter()
                        .position(|n| n == "metadata")
                        .ok_or_else(crate::jobs::invalid)?;
                    if let (
                        rusqlite::types::Value::Text(revision),
                        rusqlite::types::Value::Text(raw),
                    ) = (&columns[id_i], &columns[meta_i])
                    {
                        let meta: crate::memory::MemoryMetadata =
                            serde_json::from_str(raw).map_err(|_| crate::jobs::invalid())?;
                        revision_meta.insert(revision.clone(), (meta.title, meta.privacy));
                    }
                }
                if table == "memories" {
                    let text = |name: &str,
                                columns: &[rusqlite::types::Value]|
                     -> Result<String, CommandError> {
                        let i = names
                            .iter()
                            .position(|n| n == name)
                            .ok_or_else(crate::jobs::invalid)?;
                        match &columns[i] {
                            rusqlite::types::Value::Text(t) => Ok(t.clone()),
                            _ => Err(crate::jobs::invalid()),
                        }
                    };
                    memory_current.push((
                        text("workspace_id", &columns)?,
                        text("id", &columns)?,
                        text("current_revision", &columns)?,
                    ));
                }
                values.push(columns);
            }
            if table == "artifact_versions" {
                fix_approval_hashes(&names, &mut values)?;
                fix_deliverable_hashes(&names, &mut values)?;
                fix_review_hashes(&names, &mut values)?;
                // Order matters: the experiment fix-up
                // rewrites concept bodies (their pinned
                // experiment hash follows the restored
                // experiment row), which changes concept
                // row hashes. The receipt fix-up must run
                // after it so the receipt pins the final
                // concept hash; the reverse order leaves
                // a stale conceptContentHash whenever
                // the experiment hash changes on restore
                // (e.g. remapped memory references).
                fix_transfer_experiment_hashes(&names, &mut values)?;
                fix_transfer_receipt_hashes(&names, &mut values)?;
            }
            prepared.push((table, names, values));
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
        let mut restored_bodies: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
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
            restored_bodies.insert(ids[&r].clone(), body.to_string());
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
                let mut file = crate::snapshots::new_file(&destination)?;
                file.write_all(&data)
                    .and_then(|_| file.sync_all())
                    .map_err(|_| crate::storage::unavailable())?;
            }
        }
        for (table, names, rows) in prepared {
            let count = names.len();
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
        // Rebuild derived search state from the restored
        // canonical content; archived index rows are never
        // imported. sync_index_tx applies the search
        // boundary, so Sealed content is never indexed (and
        // verified packages cannot contain any).
        crate::search::ensure_fts_table(&tx).map_err(|_| crate::storage::unavailable())?;
        for (new_workspace, new_memory, new_revision) in &memory_current {
            let (title, privacy) = revision_meta
                .get(new_revision)
                .ok_or_else(crate::jobs::invalid)?;
            let body = restored_bodies
                .get(new_revision)
                .ok_or_else(crate::jobs::invalid)?;
            crate::search::sync_index_tx(
                &tx,
                new_workspace,
                new_memory,
                new_revision,
                title,
                body,
                privacy,
            )
            .map_err(|_| crate::storage::unavailable())?;
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

#[cfg(test)]
mod experiment_binding_tests {
    use super::*;
    use rusqlite::Connection;

    fn experiment_body(content: &str, content_hash: &str, memory_id: &str) -> String {
        serde_json::json!({
            "format": "anansi-collider-experiment",
            "version": 1,
            "beamA": "Trading.",
            "beamB": "Mycelium.",
            "controls": {
                "creativity": "balanced",
                "technicalDepth": "standard",
                "practicality": "balanced",
                "innovationDistance": "moderate",
                "budget": "",
                "preferredOutputType": "concept"
            },
            "memoryContext": [{
                "memoryId": memory_id,
                "revisionId": "rev-1",
                "version": 1,
                "title": "Notes",
                "content": content,
                "contentHash": content_hash,
                "privacy": "private"
            }],
            "result": {
                "sourceUnderstanding": "Two domains.",
                "conceptualPrimitives": ["a", "b", "c"],
                "crossDomainMappings": ["m1", "m2"],
                "combinations": [
                    { "name": "one", "description": "first" },
                    { "name": "two", "description": "second" }
                ],
                "feasibility": {
                    "hypotheses": ["h"],
                    "dependencies": [],
                    "unknownCosts": [],
                    "risks": ["r"],
                    "testability": "testable"
                },
                "productization": {
                    "conceptBrief": "brief",
                    "architecture": "arch",
                    "roadmap": "road",
                    "testPlan": "plan",
                    "prototypeProposal": "proto"
                }
            },
            "provider": "ollama",
            "model": "llama-x",
            "disclosure": {
                "provider": "ollama",
                "model": "llama-x",
                "endpointHost": "127.0.0.1",
                "local": true,
                "routingMode": "auto",
                "routingReason": "test",
                "messageCount": 1,
                "segmentCount": 1,
                "redactionCount": 0,
                "estimatedInputTokens": 10,
                "costClass": "free",
                "usage": null,
                "attempts": []
            }
        })
        .to_string()
    }

    fn setup(
        body: &str,
        revision_meta: &str,
        row_hash: &str,
        owner_memory: &str,
    ) -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE artifact_versions (workspace_id TEXT, body TEXT, content_hash TEXT, artifact_type TEXT);
             CREATE TABLE memory_revisions (id TEXT, memory_id TEXT, workspace_id TEXT, body_hash TEXT, metadata TEXT);",
        )
        .unwrap();
        let artifact_hash = crate::content::hash(body.as_bytes());
        db.execute(
            "INSERT INTO artifact_versions VALUES (?1, ?2, ?3, 'experiment')",
            rusqlite::params!["ws-1", body, artifact_hash],
        )
        .unwrap();
        db.execute(
            "INSERT INTO memory_revisions VALUES ('rev-1', ?1, 'ws-1', ?2, ?3)",
            rusqlite::params![owner_memory, row_hash, revision_meta],
        )
        .unwrap();
        db
    }

    #[test]
    fn experiment_memory_bindings_accept_valid_and_refuse_bad_originals() {
        let content = "hello context";
        let hash = crate::content::hash(content.as_bytes());
        let body = experiment_body(content, &hash, "mem-1");
        // Valid original passes.
        let db = setup(&body, r#"{"privacy":"private"}"#, &hash, "mem-1");
        assert!(validate_original_experiment_bindings(&db).is_ok());
        // Sealed original refuses (defense in depth: the
        // supported backup pipeline already refuses sealed
        // packages at export, so this branch guards
        // foreign or hand-crafted packages).
        let db = setup(&body, r#"{"privacy":"sealed"}"#, &hash, "mem-1");
        assert!(validate_original_experiment_bindings(&db).is_err());
        // Wrong owner memory refuses.
        let db = setup(&body, r#"{"privacy":"private"}"#, &hash, "mem-2");
        assert!(validate_original_experiment_bindings(&db).is_err());
        // Wrong revision-row hash refuses.
        let db = setup(
            &body,
            r#"{"privacy":"private"}"#,
            &crate::content::hash("different".as_bytes()),
            "mem-1",
        );
        assert!(validate_original_experiment_bindings(&db).is_err());
        // Snapshot content that does not hash to the
        // pinned hash refuses.
        let bad_body = experiment_body("other content", &hash, "mem-1");
        let db = setup(&bad_body, r#"{"privacy":"private"}"#, &hash, "mem-1");
        assert!(validate_original_experiment_bindings(&db).is_err());
        // Non-collider and non-JSON experiment bodies
        // restore untouched, as before.
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE artifact_versions (workspace_id TEXT, body TEXT, content_hash TEXT, artifact_type TEXT);
             CREATE TABLE memory_revisions (id TEXT, memory_id TEXT, workspace_id TEXT, body_hash TEXT, metadata TEXT);",
        )
        .unwrap();
        db.execute(
            "INSERT INTO artifact_versions VALUES ('ws-1', 'Just notes.', 'x', 'experiment')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO artifact_versions VALUES ('ws-1', '{\"format\":\"other\"}', 'y', 'experiment')",
            [],
        )
        .unwrap();
        assert!(validate_original_experiment_bindings(&db).is_ok());
    }
}
