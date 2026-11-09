//! Bounded DOCX text extraction (M13, text-bearing DOCX only).
//!
//! A standalone, pure function over DOCX bytes: [`extract_docx_text`]
//! takes the raw file and returns the extracted body and per-section
//! provenance — or a clear refusal. The integration wires it into
//! `execute_ingestion` (`crates/core/src/ingestion.rs`): the worker
//! calls it for `kind == "docx"` instead of decoding the staged
//! bytes as UTF-8, and stores the section provenance on the
//! published document.
//!
//! The design (`docs/handoffs/m13-docx-ingestion-design.md` §2–§4,
//! amended in §10) and the spike that proved it
//! (`docs/handoffs/m13-docx-spike-results.md`, 23 deterministic
//! fixtures, `crates/core/examples/docx_spike.rs`) use a bounded ZIP
//! walk + streaming XML parse: `zip` reads the archive (central
//! directory for the entry list, per-entry inflation through a
//! capped reader), `quick-xml` streams `word/document.xml` only
//! (never a DOM, never another part).
//!
//! # The bounds
//!
//! * Raw file at most [`MAX_DOCX_BYTES`] (1 MiB) — enforced by the
//!   caller (`document::max_bytes_for_kind`, ingest-begin, and the
//!   staged-file read), not by this module, exactly like PDF.
//! * Archive entries at most [`MAX_DOCX_ENTRIES`] (128), checked
//!   from the central directory before any entry is read.
//! * One entry inflated at most [`MAX_DOCX_ENTRY_INFLATED`] (8 MiB):
//!   the declared size is checked first; a lying declared size is
//!   refused by the zip crate's read-time size check during
//!   inflation.
//! * Cumulative inflation at most [`MAX_DOCX_INFLATED_TOTAL`]
//!   (16 MiB), enforced *during* inflation by a capped reader that
//!   aborts mid-entry — the second layer, covering entries whose
//!   declared size the pre-check cannot trust (e.g. data-descriptor
//!   entries with unknown central-directory size).
//! * Extracted text at most [`MAX_DOCX_CHARS`] (32,000 chars),
//!   enforced mid-paragraph while parsing, so a huge document
//!   cannot grow a huge `String` before being refused.
//! * Paragraphs at most [`MAX_DOCX_PARAGRAPHS`] (10,000), bounding
//!   parse work on pathological many-empty-paragraph files.
//!
//! # What is refused, explicitly
//!
//! * Not a ZIP, truncated archive, bad CRC, malformed XML, missing
//!   `word/document.xml` or `[Content_Types].xml` — parse errors.
//! * Any encrypted archive entry (ZipCrypto/AES flag) — refused,
//!   never attempted. The walk uses metadata-only entry access
//!   (`by_index_data`): opening an entry errors on encrypted ones
//!   before `encrypted()` is reached.
//! * `word/vbaProject.bin` present (macros = active content).
//! * `w:altChunk` present (its text would be missing, so importing
//!   would publish a partial document).
//! * `<!DOCTYPE` anywhere in `word/document.xml` (no DTD, no entity
//!   expansion).
//! * Empty total text (a valid document with no readable text).
//!
//! # What is never done
//!
//! Nothing is fetched: only `word/document.xml` is read, and
//! relationship targets with `TargetMode="External"` are ignored
//! entirely — not fetched, not recorded. No field code is executed:
//! `w:instrText` is skipped (only field *results*, ordinary `w:t`
//! runs, are included). No macro, embedded object, or drawing is
//! executed or extracted. No entry is written to disk and no path
//! is constructed from an entry name.
//!
//! # Body and provenance
//!
//! Paragraphs become body lines in document order; headings
//! (`w:pStyle` Heading1–3, Title, or `w:outlineLvl` 0–2 as a
//! fallback) become `#`/`##`/`###` lines. The `#` prefix is body
//! formatting only: `DocxSectionProvenance.heading` stores the
//! plain heading text, while the Ask/search citation heading comes
//! from the body line via `context.rs` chunking (which stores the
//! `#` line verbatim, as for Markdown), so citations read
//! `document · # <Heading> (line N)`. Section ranges are
//! contiguous, in order, and cover the body exactly (the single
//! `\n` between two sections belongs to neither section's text).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{Cursor, Read};

/// Largest accepted raw DOCX file, in bytes (mirrors the PDF raw
/// cap: binary, carries layout/fonts/images).
pub const MAX_DOCX_BYTES: usize = 1_048_576;
/// Largest accepted extracted text, in characters.
pub const MAX_DOCX_CHARS: usize = 32_000;
/// Largest accepted archive entry count.
pub const MAX_DOCX_ENTRIES: usize = 128;
/// Largest cumulative inflated bytes across the archive.
pub const MAX_DOCX_INFLATED_TOTAL: usize = 16 * 1024 * 1024;
/// Largest single entry inflated, in bytes.
pub const MAX_DOCX_ENTRY_INFLATED: usize = 8 * 1024 * 1024;
/// Largest accepted paragraph count.
pub const MAX_DOCX_PARAGRAPHS: usize = 10_000;

/// Section-level provenance for one stored section: the heading
/// text (plain, without the `# ` body prefix; `None` for leading
/// paragraphs before the first heading), its level (0 for the
/// leading section, else 1–3), the character range of that
/// section's text inside the stored body, and the SHA-256 of the
/// section's text (via `crate::content::hash`). Serialized into
/// `DocumentProvenance.sections` (a generated contract), so the
/// field names are camelCase on the wire.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocxSectionProvenance {
    pub heading: Option<String>,
    pub level: u8,
    pub start_char: u64,
    pub chars: u64,
    pub hash: String,
}

/// The extraction result: the stored body (paragraphs joined by
/// `\n`, headings as `#` lines) and per-section provenance ranges
/// that are contiguous, in order, and cover the body exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedDocx {
    pub body: String,
    pub section_provenance: Vec<DocxSectionProvenance>,
}

/// Why a DOCX was refused. The message is the honest failure the
/// worker surfaces on the job; no silent partial import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractError {
    /// Malformed or unreadable document (not a ZIP, bad CRC,
    /// malformed XML, missing required part, unclosed paragraph).
    Parse(String),
    /// An encrypted archive entry.
    Encrypted,
    /// Inflation expanded past the cumulative budget, or a single
    /// entry declared past its cap.
    Bomb,
    /// Content this importer does not support (macros, altChunk,
    /// DOCTYPE).
    Unsupported(String),
    /// More than [`MAX_DOCX_ENTRIES`] archive entries.
    EntryCap,
    /// More than [`MAX_DOCX_PARAGRAPHS`] paragraphs.
    ParagraphCap,
    /// Extracted text over [`MAX_DOCX_CHARS`] characters.
    TextCap,
    /// A valid document with no readable text.
    Empty,
}

impl ExtractError {
    /// The honest failure message the ingestion worker surfaces on
    /// the job. No partial import is published for any of these.
    pub fn message(&self) -> String {
        match self {
            ExtractError::Parse(detail) => {
                format!("DOCX could not be parsed: {detail}")
            }
            ExtractError::Encrypted => {
                "DOCX is password-protected. Encrypted documents cannot be imported; remove the password and try again.".to_string()
            }
            ExtractError::Bomb => {
                "DOCX expands too far when decoded (over the 16 MiB decode budget). The file is refused.".to_string()
            }
            ExtractError::Unsupported(detail) => {
                format!("DOCX uses content this importer does not support: {detail}")
            }
            ExtractError::EntryCap => {
                format!("DOCX has more than {MAX_DOCX_ENTRIES} archive entries. The file is refused.")
            }
            ExtractError::ParagraphCap => {
                format!("DOCX has more than {MAX_DOCX_PARAGRAPHS} paragraphs. Split it and try again.")
            }
            ExtractError::TextCap => {
                format!("DOCX text exceeds the limit ({MAX_DOCX_CHARS} characters). Split it and try again.")
            }
            ExtractError::Empty => {
                "This DOCX contains no readable text.".to_string()
            }
        }
    }
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractError::Parse(m) => write!(f, "DOCX could not be parsed: {m}"),
            ExtractError::Encrypted => {
                write!(f, "DOCX is encrypted and no password was supplied")
            }
            ExtractError::Bomb => write!(f, "DOCX expands past the decode budget"),
            ExtractError::Unsupported(m) => {
                write!(f, "DOCX uses unsupported content: {m}")
            }
            ExtractError::EntryCap => {
                write!(f, "DOCX has too many archive entries")
            }
            ExtractError::ParagraphCap => {
                write!(f, "DOCX has too many paragraphs")
            }
            ExtractError::TextCap => write!(f, "DOCX text exceeds the character limit"),
            ExtractError::Empty => write!(f, "DOCX contains no readable text"),
        }
    }
}

impl std::error::Error for ExtractError {}

/// A reader that errors the moment the cumulative inflated budget
/// is exceeded, mid-entry, before any of the over-budget bytes are
/// retained.
struct CappedReader<R: Read> {
    inner: R,
    remaining: usize,
}

impl<R: Read> CappedReader<R> {
    fn new(inner: R, budget: usize) -> Self {
        Self {
            inner,
            remaining: budget,
        }
    }
}

impl<R: Read> Read for CappedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "expansion budget exceeded",
            ));
        }
        let limit = buf.len().min(self.remaining);
        let n = self.inner.read(&mut buf[..limit])?;
        self.remaining -= n;
        Ok(n)
    }
}

struct ParaState {
    buf: String,
    style: Option<String>,
    outline: Option<u8>,
    heading_used: bool,
}

impl ParaState {
    fn new() -> Self {
        Self {
            buf: String::new(),
            style: None,
            outline: None,
            heading_used: false,
        }
    }
    fn heading_level(&self) -> Option<u8> {
        match self.style.as_deref() {
            Some("Heading1") | Some("Title") => Some(1),
            Some("Heading2") => Some(2),
            Some("Heading3") => Some(3),
            _ => self.outline.map(|o| o + 1),
        }
    }
}

/// Lines are (plain paragraph text, heading level). The `# ` prefix
/// is body formatting, rendered only when joining the body and the
/// section text; `DocxSectionProvenance.heading` is the plain
/// heading text.
fn render_line(text: &str, level: Option<u8>) -> String {
    match level {
        Some(l) => format!("{} {text}", "#".repeat(l as usize)),
        None => text.to_string(),
    }
}

fn sections_for(lines: &[(String, Option<u8>)]) -> Vec<DocxSectionProvenance> {
    // Groups split at heading lines; a leading null-heading group
    // exists only when lines precede the first heading. A section's
    // text is its rendered lines joined by "\n"; the single "\n"
    // between two sections belongs to neither section's text.
    let mut groups: Vec<(Option<String>, u8, Vec<(String, Option<u8>)>)> = Vec::new();
    let mut cur: (Option<String>, u8, Vec<(String, Option<u8>)>) = (None, 0, Vec::new());
    for (text, level) in lines {
        if let Some(lvl) = level {
            if !cur.2.is_empty() {
                groups.push(std::mem::replace(&mut cur, (None, 0, Vec::new())));
            }
            cur = (Some(text.clone()), *lvl, vec![(text.clone(), *level)]);
        } else {
            cur.2.push((text.clone(), *level));
        }
    }
    if !cur.2.is_empty() {
        groups.push(cur);
    }
    let mut sections = Vec::new();
    let mut pos = 0u64;
    for (heading, level, group_lines) in groups {
        let rendered: Vec<String> = group_lines
            .iter()
            .map(|(t, l)| render_line(t, *l))
            .collect();
        let text = rendered.join("\n");
        let chars = text.chars().count() as u64;
        let hash = crate::content::hash(text.as_bytes());
        sections.push(DocxSectionProvenance {
            heading,
            level,
            start_char: pos,
            chars,
            hash,
        });
        pos += chars + 1;
    }
    sections
}

/// Extract the text layer of a DOCX: the stored body plus
/// per-section provenance. The raw-size cap is enforced by the
/// caller (ingest-begin and the staged-file read); every other
/// bound is enforced here, during the walk and the parse.
pub fn extract_docx_text(bytes: &[u8]) -> Result<ExtractedDocx, ExtractError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| ExtractError::Parse(format!("not a zip archive: {e}")))?;
    if archive.len() > MAX_DOCX_ENTRIES {
        return Err(ExtractError::EntryCap);
    }
    let mut doc_index: Option<usize> = None;
    let mut has_content_types = false;
    for i in 0..archive.len() {
        // Metadata only: by_index would try to open the entry and
        // errors on an encrypted one before encrypted() is reached.
        let file = archive
            .by_index_data(i)
            .map_err(|e| ExtractError::Parse(format!("entry {i}: {e}")))?;
        let name = file
            .name()
            .map_err(|e| ExtractError::Parse(format!("entry {i} name: {e}")))?
            .into_owned();
        if name == "[Content_Types].xml" {
            has_content_types = true;
        }
        if name == "word/vbaProject.bin" {
            return Err(ExtractError::Unsupported(
                "macros (word/vbaProject.bin present)".to_string(),
            ));
        }
        if file.encrypted() {
            return Err(ExtractError::Encrypted);
        }
        if file.size() > MAX_DOCX_ENTRY_INFLATED as u64 {
            return Err(ExtractError::Bomb);
        }
        if name == "word/document.xml" {
            doc_index = Some(i);
        }
    }
    if !has_content_types {
        return Err(ExtractError::Parse(
            "missing [Content_Types].xml".to_string(),
        ));
    }
    let doc_index =
        doc_index.ok_or_else(|| ExtractError::Parse("missing word/document.xml".to_string()))?;
    let mut file = archive
        .by_index(doc_index)
        .map_err(|e| ExtractError::Parse(format!("word/document.xml: {e}")))?;
    let mut doc_bytes: Vec<u8> = Vec::new();
    let mut capped = CappedReader::new(&mut file, MAX_DOCX_INFLATED_TOTAL);
    capped.read_to_end(&mut doc_bytes).map_err(|e| {
        if e.to_string().contains("expansion budget exceeded") {
            ExtractError::Bomb
        } else {
            ExtractError::Parse(format!("reading word/document.xml: {e}"))
        }
    })?;
    // No DTD, no entity expansion: refuse before parsing.
    if doc_bytes
        .windows(9)
        .any(|w| w == b"<!DOCTYPE" || w == b"<!doctype")
    {
        return Err(ExtractError::Unsupported(
            "DOCTYPE declarations are not supported".to_string(),
        ));
    }
    let extracted = parse_document_xml(&doc_bytes)?;
    if extracted.body.trim().is_empty() {
        return Err(ExtractError::Empty);
    }
    Ok(extracted)
}

fn parse_document_xml(doc_bytes: &[u8]) -> Result<ExtractedDocx, ExtractError> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    // quick-xml 0.42: the slice reader (read_event, no buffer) needs
    // &str, so UTF-8 is validated up front — a non-UTF-8 document.xml
    // is a parse error, not a transcode attempt.
    let xml_text = std::str::from_utf8(doc_bytes)
        .map_err(|e| ExtractError::Parse(format!("word/document.xml is not UTF-8: {e}")))?;
    let mut reader = Reader::from_str(xml_text);
    reader.config_mut().trim_text_start = false;
    reader.config_mut().trim_text_end = false;
    reader.config_mut().check_end_names = true;

    let mut lines: Vec<(String, Option<u8>)> = Vec::new();
    let mut para = ParaState::new();
    let mut in_para = false;
    let mut para_count = 0usize;
    let mut in_t = false;
    let mut del_depth = 0usize;
    let mut instr_depth = 0usize;
    let mut skip_depth = 0usize; // w:drawing / w:pict / w:object
    // Chars appended to paragraph buffers (before trimming): the
    // single text-cap counter, enforced mid-paragraph during parsing.
    // flush_line does NOT touch it — the body is only shorter than
    // this (trimming, "# " prefixes excluded).
    let mut appended_chars = 0usize;

    let flush_line = |para: &mut ParaState, lines: &mut Vec<(String, Option<u8>)>| {
        let text = para.buf.trim_end().to_string();
        if text.is_empty() {
            para.buf.clear();
            return;
        }
        let level = if para.heading_used {
            None
        } else {
            para.heading_level()
        };
        para.heading_used = true;
        lines.push((text, level));
        para.buf.clear();
    };

    // Shared attribute readers: pStyle / outlineLvl (w:val) and
    // w:br (type="page"). Attribute values are Cow<str> in 0.42.
    let take_val = |e: &quick_xml::events::BytesStart<'_>| -> Option<String> {
        for attr in e.attributes().flatten() {
            if attr.key.local_name().as_ref() == "val" {
                return Some(attr.value.into_owned());
            }
        }
        None
    };
    let br_is_page = |e: &quick_xml::events::BytesStart<'_>| -> bool {
        for attr in e.attributes().flatten() {
            if attr.key.local_name().as_ref() == "type" && attr.value.as_ref() == "page" {
                return true;
            }
        }
        false
    };
    // Appending with the text cap enforced mid-paragraph.
    let append_capped = |para: &mut ParaState,
                         appended_chars: &mut usize,
                         text: &str|
     -> Result<(), ExtractError> {
        let n = text.chars().count();
        if *appended_chars + n > MAX_DOCX_CHARS {
            return Err(ExtractError::TextCap);
        }
        *appended_chars += n;
        para.buf.push_str(text);
        Ok(())
    };

    loop {
        let event = reader
            .read_event()
            .map_err(|e| ExtractError::Parse(format!("malformed XML: {e}")))?;
        match event {
            Event::Eof => break,
            Event::DocType(_) => {
                return Err(ExtractError::Unsupported(
                    "DOCTYPE declarations are not supported".to_string(),
                ));
            }
            Event::Start(e) => match e.local_name().as_ref() {
                "p" => {
                    para_count += 1;
                    if para_count > MAX_DOCX_PARAGRAPHS {
                        return Err(ExtractError::ParagraphCap);
                    }
                    para = ParaState::new();
                    in_para = true;
                }
                "pStyle" => {
                    if let Some(v) = take_val(&e) {
                        para.style = Some(v);
                    }
                }
                "outlineLvl" => {
                    if let Some(v) = take_val(&e) {
                        if let Ok(n) = v.parse::<u8>() {
                            if n <= 2 {
                                para.outline = Some(n);
                            }
                        }
                    }
                }
                "t" => in_t = true,
                "tab" => {
                    if in_para {
                        para.buf.push('\t');
                    }
                }
                "br" => {
                    if in_para {
                        if br_is_page(&e) {
                            flush_line(&mut para, &mut lines);
                        } else {
                            para.buf.push('\n');
                        }
                    }
                }
                "instrText" => instr_depth += 1,
                "del" => del_depth += 1,
                "drawing" | "pict" | "object" => skip_depth += 1,
                "altChunk" => {
                    return Err(ExtractError::Unsupported(
                        "altChunk content is not supported".to_string(),
                    ));
                }
                _ => {}
            },
            Event::Empty(e) => match e.local_name().as_ref() {
                "p" => {
                    para_count += 1;
                    if para_count > MAX_DOCX_PARAGRAPHS {
                        return Err(ExtractError::ParagraphCap);
                    }
                    para = ParaState::new();
                    flush_line(&mut para, &mut lines);
                }
                "pStyle" => {
                    if let Some(v) = take_val(&e) {
                        para.style = Some(v);
                    }
                }
                "outlineLvl" => {
                    if let Some(v) = take_val(&e) {
                        if let Ok(n) = v.parse::<u8>() {
                            if n <= 2 {
                                para.outline = Some(n);
                            }
                        }
                    }
                }
                "tab" => {
                    if in_para {
                        para.buf.push('\t');
                    }
                }
                "br" => {
                    if in_para {
                        if br_is_page(&e) {
                            flush_line(&mut para, &mut lines);
                        } else {
                            para.buf.push('\n');
                        }
                    }
                }
                "altChunk" => {
                    return Err(ExtractError::Unsupported(
                        "altChunk content is not supported".to_string(),
                    ));
                }
                _ => {}
            },
            Event::Text(e) => {
                if in_t && del_depth == 0 && instr_depth == 0 && skip_depth == 0 && in_para {
                    let raw = e.into_inner();
                    let text = quick_xml::escape::unescape(&raw)
                        .map_err(|err| ExtractError::Parse(format!("bad text: {err}")))?;
                    append_capped(&mut para, &mut appended_chars, &text)?;
                }
            }
            Event::CData(e) => {
                if del_depth == 0 && instr_depth == 0 && skip_depth == 0 && in_para {
                    let text = e.into_inner();
                    append_capped(&mut para, &mut appended_chars, &text)?;
                }
            }
            Event::End(e) => match e.local_name().as_ref() {
                "p" => {
                    flush_line(&mut para, &mut lines);
                    in_para = false;
                }
                "t" => in_t = false,
                "instrText" => instr_depth = instr_depth.saturating_sub(1),
                "del" => del_depth = del_depth.saturating_sub(1),
                "drawing" | "pict" | "object" => {
                    skip_depth = skip_depth.saturating_sub(1);
                }
                _ => {}
            },
            _ => {}
        }
    }
    if in_para {
        // Well-formed XML always closes w:p; reaching EOF inside one
        // means the document was truncated mid-paragraph.
        return Err(ExtractError::Parse(
            "unclosed paragraph at end of document".to_string(),
        ));
    }
    let body = lines
        .iter()
        .map(|(text, level)| render_line(text, *level))
        .collect::<Vec<_>>()
        .join("\n");
    let section_provenance = sections_for(&lines);
    Ok(ExtractedDocx {
        body,
        section_provenance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!(
            "{}/../../tests/fixtures/docx/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read(path).unwrap()
    }

    #[test]
    fn normal_docx_extracts_headings_and_sections() {
        let extracted = extract_docx_text(&fixture("normal.docx")).unwrap();
        assert_eq!(extracted.body.chars().count(), 297);
        assert_eq!(extracted.section_provenance.len(), 4);
        let first = &extracted.section_provenance[0];
        assert_eq!(first.heading.as_deref(), Some("Harbor Ledger"));
        assert_eq!(first.level, 1);
        assert_eq!(first.start_char, 0);
        assert_eq!(first.chars, 15);
        assert!(extracted.body.starts_with("# Harbor Ledger\n"));
        assert!(!extracted.body.contains('\x0c'));
        // Section ranges are contiguous and cover the body exactly.
        let mut pos = 0u64;
        for section in &extracted.section_provenance {
            assert_eq!(section.start_char, pos);
            pos += section.chars + 1;
        }
        assert_eq!(pos - 1, extracted.body.chars().count() as u64);
    }

    #[test]
    fn unicode_headings_are_exact() {
        let extracted = extract_docx_text(&fixture("unicode-headings.docx")).unwrap();
        assert_eq!(extracted.section_provenance.len(), 3);
        assert!(extracted.body.contains("海关总署"));
        assert!(extracted.body.contains("Café ☕"));
        assert!(extracted.body.contains("naïve façade — ünïcödé"));
    }

    #[test]
    fn field_codes_excluded_results_included() {
        let extracted = extract_docx_text(&fixture("field-codes.docx")).unwrap();
        assert!(extracted.body.contains("Annex link"));
        assert!(!extracted.body.contains("HYPERLINK"));
        assert!(!extracted.body.contains("PAGEREF"));
    }

    #[test]
    fn track_changes_keep_insertions_drop_deletions() {
        let extracted = extract_docx_text(&fixture("track-changes.docx")).unwrap();
        assert!(extracted.body.contains("Inserted text."));
        assert!(!extracted.body.contains("Deleted text."));
    }

    #[test]
    fn external_hyperlink_keeps_display_text_only() {
        let extracted = extract_docx_text(&fixture("external-hyperlink.docx")).unwrap();
        assert!(extracted.body.contains("External annex link"));
        assert!(!extracted.body.contains("example.invalid"));
        assert!(!extracted.body.contains("https://"));
    }

    #[test]
    fn corrupt_fixtures_are_parse_errors() {
        for name in [
            "corrupt-truncated-zip.docx",
            "corrupt-not-a-zip.docx",
            "corrupt-bad-crc.docx",
            "corrupt-bad-xml.docx",
            "corrupt-missing-document.docx",
            "corrupt-missing-content-types.docx",
        ] {
            let err = extract_docx_text(&fixture(name)).unwrap_err();
            assert!(
                matches!(err, ExtractError::Parse(_)),
                "{name}: expected Parse, got {err:?}"
            );
        }
    }

    #[test]
    fn refusals_have_the_right_kind() {
        assert!(matches!(
            extract_docx_text(&fixture("encrypted-entry.docx")).unwrap_err(),
            ExtractError::Encrypted
        ));
        assert!(matches!(
            extract_docx_text(&fixture("macro.docx")).unwrap_err(),
            ExtractError::Unsupported(_)
        ));
        assert!(matches!(
            extract_docx_text(&fixture("altchunk.docx")).unwrap_err(),
            ExtractError::Unsupported(_)
        ));
        assert!(matches!(
            extract_docx_text(&fixture("doctype.docx")).unwrap_err(),
            ExtractError::Unsupported(_)
        ));
        assert!(matches!(
            extract_docx_text(&fixture("many-entries.docx")).unwrap_err(),
            ExtractError::EntryCap
        ));
        assert!(matches!(
            extract_docx_text(&fixture("oversized-text.docx")).unwrap_err(),
            ExtractError::TextCap
        ));
        assert!(matches!(
            extract_docx_text(&fixture("empty.docx")).unwrap_err(),
            ExtractError::Empty
        ));
        assert!(matches!(
            extract_docx_text(&fixture("whitespace-only.docx")).unwrap_err(),
            ExtractError::Empty
        ));
    }

    #[test]
    fn zip_bomb_is_refused_before_expansion() {
        let err = extract_docx_text(&fixture("zip-bomb.docx")).unwrap_err();
        // A lying declared size is refused by the zip crate's
        // read-time size check; either layer is a Bomb-class refusal.
        assert!(
            matches!(err, ExtractError::Bomb | ExtractError::Parse(_)),
            "zip bomb: expected Bomb or Parse, got {err:?}"
        );
    }

    #[test]
    fn outline_level_fallback_matches_styled_headings() {
        let styled = extract_docx_text(&fixture("normal.docx")).unwrap();
        let outlined = extract_docx_text(&fixture("outline-lvl.docx")).unwrap();
        assert_eq!(styled.body, outlined.body);
        assert_eq!(styled.section_provenance, outlined.section_provenance);
    }
}
