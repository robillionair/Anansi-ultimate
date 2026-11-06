//! Shared local-document rules for TXT/Markdown ingestion (M13 Slice 1).
//!
//! The validation contract here is the one the collider beam-text loader
//! already enforced (`validate_document_source` in `collider.rs`): kind
//! `txt`/`md` only, basename label whose extension matches the kind, raw
//! file at most 64 KiB, decoded text 1–8,000 chars, strict UTF-8, one
//! leading BOM stripped and recorded, and both hashes recomputed against
//! the submitted text. The collider keeps its own `ColliderDocumentSource`
//! contract type (its name is part of the generated contracts); the shared
//! logic below takes plain fields so both callers validate identically
//! without a module cycle. See `docs/handoffs/m13-document-ingestion-design.md`
//! §3.1 and the closure addendum for the shape note.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Largest accepted document file, in bytes. A document body holds at most
/// 8,000 characters, so 64 KiB bounds the request payload while leaving
/// ample room for multi-byte UTF-8 text.
pub const MAX_DOCUMENT_BYTES: u64 = 65_536;
/// Largest accepted decoded document text, in characters.
pub const MAX_DOCUMENT_CHARS: u64 = 8_000;
/// Largest accepted file-name label, in characters.
pub const MAX_LABEL_CHARS: usize = 255;
/// Largest single upload part, in bytes.
pub const MAX_UPLOAD_PART_BYTES: usize = 65_536;
/// Largest number of concurrent ingestion uploads per workspace.
pub const MAX_ACTIVE_UPLOADS: u32 = 8;

/// The exact-source record for one beam/document text. Mirrors
/// `ColliderDocumentSource` field for field; ingestion builds this from
/// the staged upload, the collider builds it from the picked file.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentSource {
    pub label: String,
    pub kind: String,
    pub bom: bool,
    pub raw_bytes: u64,
    pub raw_hash: String,
    pub text_hash: String,
    pub text_chars: u64,
}

/// Exact-source provenance stored on a published `document` memory.
/// `media_type` is derived from the kind, never trusted from the client.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentProvenance {
    pub file_name: String,
    pub media_type: String,
    pub raw_hash: String,
    pub text_hash: String,
    pub raw_bytes: u64,
    pub text_chars: u64,
    pub bom: bool,
}

impl DocumentProvenance {
    pub fn media_type_for(kind: &str) -> &'static str {
        match kind {
            "md" => "text/markdown",
            _ => "text/plain",
        }
    }
}

/// Internal-only publish guard for the ingestion worker. It is never
/// deserialized from JSON (`#[serde(skip)]` on the request field), so
/// only Rust callers can set it. Carried into `memory_write`'s publish
/// transaction, where it atomically verifies the job is still Running
/// under this lease and, when dedup fields are set, returns an existing
/// document with the same title and text hash instead of publishing a
/// duplicate.
#[derive(Debug, Clone)]
pub struct JobPublishGuard {
    pub job_id: String,
    pub owner: String,
    pub dedup_title: Option<String>,
    pub dedup_text_hash: Option<String>,
}

/// Validate one document's provenance against its exact decoded text.
/// Every field is recomputed here — nothing about the record is trusted —
/// so a forged, stale or inconsistent record refuses. The label must be a
/// basename whose extension matches the kind; the raw hash covers the
/// original file bytes (a stripped BOM included), the text hash covers the
/// decoded text bytes, and both counts must match the text exactly.
#[allow(clippy::too_many_arguments)]
pub fn validate_document_source(
    label: &str,
    kind: &str,
    bom: bool,
    raw_bytes: u64,
    raw_hash: &str,
    text_hash: &str,
    text_chars: u64,
    text: &str,
) -> Result<(), String> {
    if kind != "txt" && kind != "md" {
        return Err(format!("document kind must be txt or md, got {kind}"));
    }
    let label_chars = label.chars().count();
    if label_chars == 0 || label_chars > MAX_LABEL_CHARS {
        return Err("document label must be 1 to 255 characters.".to_string());
    }
    if label.contains('/') || label.contains('\\') || label.contains('\0') {
        return Err("document label must be a file name, not a path.".to_string());
    }
    let suffix = label.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if (kind == "txt" && suffix != "txt") || (kind == "md" && suffix != "md") {
        return Err("document label extension must match its kind.".to_string());
    }
    if raw_bytes > MAX_DOCUMENT_BYTES {
        return Err(format!(
            "document file must be at most {MAX_DOCUMENT_BYTES} bytes, got {raw_bytes}"
        ));
    }
    let text_bytes = text.as_bytes();
    let actual_chars = text.chars().count() as u64;
    if text_chars != actual_chars || actual_chars > MAX_DOCUMENT_CHARS {
        return Err("document text length must match the submitted text.".to_string());
    }
    if crate::content::hash(text_bytes) != text_hash {
        return Err("document text hash must match the submitted text.".to_string());
    }
    let mut raw: Vec<u8> = Vec::with_capacity(text_bytes.len() + 3);
    if bom {
        raw.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    raw.extend_from_slice(text_bytes);
    if crate::content::hash(&raw) != raw_hash {
        return Err("document file hash must match the submitted text.".to_string());
    }
    if raw_bytes != raw.len() as u64 {
        return Err("document file size must match the submitted text.".to_string());
    }
    Ok(())
}

impl DocumentSource {
    pub fn validate(&self, text: &str) -> Result<(), String> {
        validate_document_source(
            &self.label,
            &self.kind,
            self.bom,
            self.raw_bytes,
            &self.raw_hash,
            &self.text_hash,
            self.text_chars,
            text,
        )
    }

    /// Derive the kind from a file-name label, or refuse when the
    /// extension is not `.txt` / `.md`.
    pub fn kind_for_label(label: &str) -> Result<&'static str, String> {
        let suffix = label.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        match suffix.as_str() {
            "txt" => Ok("txt"),
            "md" => Ok("md"),
            other => Err(format!(
                "document format '{other}' is not supported in this slice. Only .txt and .md files can be imported; PDF, DOCX and other formats are tracked as follow-up work."
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> DocumentSource {
        let text = "Hello local notes.";
        let mut raw = text.as_bytes().to_vec();
        let text_hash = crate::content::hash(text.as_bytes());
        let raw_hash = crate::content::hash(&raw);
        raw.clear();
        DocumentSource {
            label: "notes.txt".to_string(),
            kind: "txt".to_string(),
            bom: false,
            raw_bytes: text.len() as u64,
            raw_hash,
            text_hash,
            text_chars: text.chars().count() as u64,
        }
    }

    #[test]
    fn valid_source_passes() {
        assert!(source().validate("Hello local notes.").is_ok());
    }

    #[test]
    fn wrong_text_hash_refuses() {
        let mut s = source();
        s.text_hash = "0".repeat(64);
        assert!(s.validate("Hello local notes.").is_err());
    }

    #[test]
    fn wrong_kind_refuses() {
        assert!(DocumentSource::kind_for_label("report.pdf").is_err());
        assert_eq!(DocumentSource::kind_for_label("a.md").unwrap(), "md");
        assert_eq!(DocumentSource::kind_for_label("A.TXT").unwrap(), "txt");
    }

    #[test]
    fn bom_source_validates_with_bom_bytes() {
        let text = "With BOM.";
        let mut raw = vec![0xEF, 0xBB, 0xBF];
        raw.extend_from_slice(text.as_bytes());
        let s = DocumentSource {
            label: "bom.md".to_string(),
            kind: "md".to_string(),
            bom: true,
            raw_bytes: raw.len() as u64,
            raw_hash: crate::content::hash(&raw),
            text_hash: crate::content::hash(text.as_bytes()),
            text_chars: text.chars().count() as u64,
        };
        assert!(s.validate(text).is_ok());
        // The same record without the BOM flag must refuse: the raw hash
        // would be computed without the BOM bytes.
        let mut no_bom = s.clone();
        no_bom.bom = false;
        assert!(no_bom.validate(text).is_err());
    }
}
