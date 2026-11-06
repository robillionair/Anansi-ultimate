//! Shared local-document rules for TXT/Markdown/PDF ingestion (M13).
//!
//! The validation contract here is the one the collider beam-text loader
//! already enforced (`validate_document_source` in `collider.rs`): kind
//! `txt`/`md` only, basename label whose extension matches the kind, raw
//! file at most 64 KiB, decoded text 1–8,000 chars, strict UTF-8, one
//! leading BOM stripped and recorded, and both hashes recomputed against
//! the submitted text. PDF extends it: kind `pdf`, raw file at most
//! 1 MiB, extracted text 1–32,000 chars, no BOM, and the raw hash's
//! *value* verified at ingestion against the staged file (the binary
//! PDF is not recoverable from the extracted text, so the shared
//! validator checks only its shape). The collider keeps its own
//! `ColliderDocumentSource` contract type (its name is part of the
//! generated contracts); the shared logic below takes plain fields so
//! both callers validate identically without a module cycle. See
//! `docs/handoffs/m13-document-ingestion-design.md` §3.1 and the
//! closure addendum for the shape note.
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

/// Largest accepted raw file, in bytes, for a given document kind.
/// PDF is binary and carries layout/fonts/images, so its raw cap is
/// 1 MiB (the extractor's [`crate::pdf::MAX_PDF_BYTES`]); TXT/Markdown
/// stays at 64 KiB.
pub fn max_bytes_for_kind(kind: &str) -> u64 {
    if kind == "pdf" {
        crate::pdf::MAX_PDF_BYTES as u64
    } else {
        MAX_DOCUMENT_BYTES
    }
}

/// Largest accepted decoded text, in characters, for a given kind.
/// PDF text is capped at 32,000 chars (the extractor's
/// [`crate::pdf::MAX_PDF_CHARS`], the Ask evidence budget); TXT/Markdown
/// stays at 8,000.
pub fn max_chars_for_kind(kind: &str) -> u64 {
    if kind == "pdf" {
        crate::pdf::MAX_PDF_CHARS as u64
    } else {
        MAX_DOCUMENT_CHARS
    }
}

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
/// `pages` is present only for PDF documents: one entry per stored page
/// (page number, character range inside the stored body, SHA-256 of the
/// page's text), so a reader, a search hit, and an Ask citation can all
/// name the page a passage came from. TXT/Markdown documents omit it.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<crate::pdf::PdfPageProvenance>>,
}

impl DocumentProvenance {
    pub fn media_type_for(kind: &str) -> &'static str {
        match kind {
            "md" => "text/markdown",
            "pdf" => "application/pdf",
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
///
/// For PDF the raw hash is the one exception: the raw file is binary
/// and is not recoverable from the extracted text, so its *value*
/// cannot be recomputed here. This function checks only that it is a
/// well-formed 64-character lowercase hex string; the value itself is
/// verified at ingestion, in `execute_ingestion`, where the staged
/// file's bytes are the ground truth (length + hash checked against
/// the declared values before extraction runs). The text hash, text
/// count, label, media type, and both limits are still recomputed
/// here, so a forged PDF provenance cannot publish a document whose
/// text does not match.
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
    if kind != "txt" && kind != "md" && kind != "pdf" {
        return Err(format!("document kind must be txt, md or pdf, got {kind}"));
    }
    let label_chars = label.chars().count();
    if label_chars == 0 || label_chars > MAX_LABEL_CHARS {
        return Err("document label must be 1 to 255 characters.".to_string());
    }
    if label.contains('/') || label.contains('\\') || label.contains('\0') {
        return Err("document label must be a file name, not a path.".to_string());
    }
    let suffix = label.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if (kind == "txt" && suffix != "txt")
        || (kind == "md" && suffix != "md")
        || (kind == "pdf" && suffix != "pdf")
    {
        return Err("document label extension must match its kind.".to_string());
    }
    let max_bytes = max_bytes_for_kind(kind);
    if raw_bytes > max_bytes {
        return Err(format!(
            "document file must be at most {max_bytes} bytes, got {raw_bytes}"
        ));
    }
    let max_chars = max_chars_for_kind(kind);
    let text_bytes = text.as_bytes();
    let actual_chars = text.chars().count() as u64;
    if text_chars != actual_chars || actual_chars > max_chars {
        return Err("document text length must match the submitted text.".to_string());
    }
    if crate::content::hash(text_bytes) != text_hash {
        return Err("document text hash must match the submitted text.".to_string());
    }
    if kind == "pdf" {
        // The raw PDF is binary; only the hash's shape can be checked
        // here. Its value is verified at ingestion against the staged
        // file (see the doc comment above).
        if raw_hash.len() != 64
            || !raw_hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err("document file hash must be 64 lowercase hex characters.".to_string());
        }
        return Ok(());
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
    /// extension is not `.txt` / `.md` / `.pdf`.
    pub fn kind_for_label(label: &str) -> Result<&'static str, String> {
        let suffix = label.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        match suffix.as_str() {
            "txt" => Ok("txt"),
            "md" => Ok("md"),
            "pdf" => Ok("pdf"),
            other => Err(format!(
                "document format '{other}' is not supported in this slice. Only .txt, .md and .pdf files can be imported; DOCX and other formats are tracked as follow-up work."
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
        assert!(DocumentSource::kind_for_label("report.docx").is_err());
        assert_eq!(DocumentSource::kind_for_label("a.md").unwrap(), "md");
        assert_eq!(DocumentSource::kind_for_label("A.TXT").unwrap(), "txt");
        assert_eq!(DocumentSource::kind_for_label("report.PDF").unwrap(), "pdf");
    }

    #[test]
    fn pdf_source_validates_with_format_only_raw_hash() {
        // The raw PDF is binary, so the validator cannot recompute
        // its hash from the text: it checks the hash's shape, and the
        // value is verified at ingestion against the staged file.
        let text = "Page one text.";
        let s = DocumentSource {
            label: "report.pdf".to_string(),
            kind: "pdf".to_string(),
            bom: false,
            raw_bytes: 1_479,
            raw_hash: "a".repeat(64),
            text_hash: crate::content::hash(text.as_bytes()),
            text_chars: text.chars().count() as u64,
        };
        assert!(s.validate(text).is_ok());
        // A malformed raw hash refuses even for PDF.
        let mut bad_hash = s.clone();
        bad_hash.raw_hash = "not-a-hash".to_string();
        assert!(bad_hash.validate(text).is_err());
        // A wrong text hash still refuses: the text is recomputed.
        let mut bad_text = s.clone();
        bad_text.text_hash = "0".repeat(64);
        assert!(bad_text.validate(text).is_err());
        // The wrong extension for the kind refuses.
        let mut bad_label = s.clone();
        bad_label.label = "report.txt".to_string();
        assert!(bad_label.validate(text).is_err());
        // The PDF text cap is 32,000, not 8,000: a 9,000-char text
        // validates for PDF but would refuse for txt.
        let long = "x".repeat(9_000);
        let mut long_pdf = s.clone();
        long_pdf.text_hash = crate::content::hash(long.as_bytes());
        long_pdf.text_chars = 9_000;
        assert!(long_pdf.validate(&long).is_ok());
        let mut long_txt = long_pdf.clone();
        long_txt.kind = "txt".to_string();
        long_txt.label = "report.txt".to_string();
        assert!(long_txt.validate(&long).is_err());
    }

    #[test]
    fn per_kind_limits() {
        assert_eq!(max_bytes_for_kind("txt"), 65_536);
        assert_eq!(max_bytes_for_kind("md"), 65_536);
        assert_eq!(max_bytes_for_kind("pdf"), 1_048_576);
        assert_eq!(max_chars_for_kind("txt"), 8_000);
        assert_eq!(max_chars_for_kind("pdf"), 32_000);
        assert_eq!(DocumentProvenance::media_type_for("pdf"), "application/pdf");
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
