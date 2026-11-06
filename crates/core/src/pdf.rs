//! Bounded PDF text extraction (M13, text-layer PDFs only).
//!
//! A standalone, pure function over PDF bytes: [`extract_pdf_text`]
//! takes the raw file and returns the extracted pages, the stored
//! body, and per-page provenance — or a clear refusal. It does not
//! touch the upload path, the ingestion job, or the workspace; the
//! integration step wires it into `execute_ingestion`.
//!
//! # The bound
//!
//! Every byte the module decodes — structural streams (compressed
//! object streams, cross-reference streams), page content streams,
//! and Form XObject streams — is charged against one cumulative
//! budget, [`MAX_DECODED_BYTES`] (16 MiB). The budget is enforced
//! *during* decoding, by capped readers that abort mid-stream, so a
//! decompression bomb is refused in milliseconds at a few megabytes
//! of peak memory instead of being decoded. The text extractor
//! (pdf-extract) decodes exactly the streams the pre-scans already
//! measured, so its decoding is bounded by the same budget.
//!
//! # Order of checks
//!
//! 1. Pre-load structural scan: find every `/ObjStm` and `/XRef`
//!    stream in the raw bytes and decode each through the capped
//!    decoder. This closes the load-time gap — lopdf decompresses
//!    every object stream during `Document::load_mem`, before any
//!    of this module's code could run — so the scan must prove the
//!    structural streams fit the budget *before* the load.
//! 2. Single `Document::load_mem`, shared by the pre-scan and the
//!    extraction (the load-time decoding happens once).
//! 3. `/Encrypt` check: any encryption dictionary is refused
//!    (empty-password documents pass through — lopdf strips the
//!    dictionary after a successful empty-password decryption —
//!    and are treated as unencrypted).
//! 4. Content pre-scan: every page content stream and Form XObject
//!    stream (walked recursively, with a visited set) decoded
//!    through the capped decoder.
//! 5. Page count: 0 pages is a parse error (never "empty"); more
//!    than [`MAX_PDF_PAGES`] is refused before any text work.
//! 6. Per-page extraction with pdf-extract (`output_doc_page` +
//!    `PlainTextOutput`), stopping at the per-page and total text
//!    caps. Empty total text is refused as no text layer.
//!
//! # What is refused, explicitly
//!
//! - Any `/Filter` outside {none, FlateDecode, LZWDecode,
//!   ASCII85Decode} — including DCTDecode, JPXDecode, CCITTFax,
//!   JBIG2, Crypt, ASCIIHexDecode, and RunLengthDecode. lopdf's own
//!   decoder supports exactly the same three, so a refusal here is
//!   consistent with what the extractor would do.
//! - Any `/DecodeParms` with a `/Predictor` greater than 1 (predictor
//!   decoding is image-data machinery; it does not occur in text
//!   content streams).
//! - Any filter chain the capped decoder cannot bound (it decodes
//!   stage by stage, charging each stage's output to the budget).
//! - A stream dictionary this module cannot parse after an object
//!   header (fail closed: it might be a structural stream).
//! - Encrypted documents, scanned/image-only documents (zero text),
//!   malformed documents, and documents over any cap.
//!
//! # Non-goals
//!
//! OCR (separate slice), DOCX, password entry, layout
//! reconstruction, PDF/A, annotations/forms. lopdf is structure-only
//! here: its own text extractor drops line breaks and ignores
//! ToUnicode, so pdf-extract is the sole text extractor.

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::io::Read;

use flate2::read::{DeflateDecoder, ZlibDecoder};
use lopdf::{Dictionary, Document, Object, ObjectId};
use pdf_extract::{PlainTextOutput, output_doc_page};
use weezl::{BitOrder, decode::Decoder as LzwDecoder};

use crate::content;

/// Maximum raw PDF size accepted (the ingestion layer enforces the
/// same cap at `ingest-begin`; the module re-checks it so it is safe
/// to call directly).
pub const MAX_PDF_BYTES: usize = 1_048_576;

/// Maximum number of pages.
pub const MAX_PDF_PAGES: usize = 100;

/// Maximum extracted characters on one page.
pub const MAX_PDF_PAGE_CHARS: usize = 8_000;

/// Maximum extracted characters in the whole document. This is a
/// *storage* cap (worker time, memory, FTS index size, UI rendering),
/// not an Ask-retrieval guarantee: Ask's default budget is 4,000
/// chars per query, shared across all evidence, spent on top-ranked
/// 1,000-char chunks — a max-size document is never fully retrieved
/// by default.
pub const MAX_PDF_CHARS: usize = 32_000;

/// Cumulative decoded-bytes budget for every stream this module
/// decodes (structural + content + form streams, across all filter
/// stages). Enforced during decoding by capped readers.
pub const MAX_DECODED_BYTES: usize = 16 * 1024 * 1024;

/// Separator joining page texts in the stored body (form feed on its
/// own line): 3 characters, `\n\x0c\n`.
pub const PAGE_SEPARATOR: &str = "\n\x0c\n";

/// One extracted page: the page number (1-based) and its text, with
/// the trailing newline pdf-extract appends per page trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedPage {
    pub number: u32,
    pub text: String,
}

/// Page-level provenance for one stored page: the page number, the
/// character range of that page's text inside the stored body, and
/// the SHA-256 of the page's text (via `crate::content::hash`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfPageProvenance {
    pub page: u32,
    pub start_char: u64,
    pub chars: u64,
    pub hash: String,
}

/// The extraction result: pages in order, the stored body (pages
/// joined by [`PAGE_SEPARATOR`]), and per-page provenance ranges
/// that are contiguous, in order, and cover the body exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedPdf {
    pub pages: Vec<ExtractedPage>,
    pub body: String,
    pub page_provenance: Vec<PdfPageProvenance>,
}

/// Why a PDF was refused. The message is the honest failure the
/// worker surfaces on the job; no silent partial import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractError {
    /// Malformed or unreadable document (including 0 pages, which
    /// is never reported as "empty").
    Parse(String),
    /// A password-protected document (`/Encrypt` present and the
    /// document did not open without a password).
    Encrypted,
    /// A decode expanded past the cumulative budget.
    Bomb,
    /// A filter (or predictor) this module does not support.
    Unsupported(String),
    /// More than [`MAX_PDF_PAGES`] pages.
    PageCap,
    /// A page over [`MAX_PDF_PAGE_CHARS`] or the document over
    /// [`MAX_PDF_CHARS`].
    TextCap,
    /// A valid document with no readable text (scanned / blank).
    Empty,
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractError::Parse(m) => write!(f, "PDF could not be parsed: {m}"),
            ExtractError::Encrypted => {
                write!(f, "PDF is encrypted and no password was supplied")
            }
            ExtractError::Bomb => write!(
                f,
                "PDF decoding exceeds the {MAX_DECODED_BYTES}-byte expansion budget"
            ),
            ExtractError::Unsupported(m) => write!(f, "PDF uses an unsupported encoding: {m}"),
            ExtractError::PageCap => {
                write!(f, "PDF has more than {MAX_PDF_PAGES} pages")
            }
            ExtractError::TextCap => write!(
                f,
                "PDF text exceeds the limit ({MAX_PDF_PAGE_CHARS} chars per page, {MAX_PDF_CHARS} total)"
            ),
            ExtractError::Empty => write!(f, "PDF has no readable text layer"),
        }
    }
}

impl std::error::Error for ExtractError {}

/// Extract the text layer of a PDF, with every decode bounded.
///
/// Returns the pages, the stored body, and per-page provenance, or
/// a refusal. The raw size is checked first; the structural scan
/// (step 1) runs before the document is loaded.
pub fn extract_pdf_text(bytes: &[u8]) -> Result<ExtractedPdf, ExtractError> {
    if bytes.len() > MAX_PDF_BYTES {
        return Err(ExtractError::Parse(format!(
            "PDF is {} bytes, over the {MAX_PDF_BYTES}-byte raw cap",
            bytes.len()
        )));
    }
    let mut budget = DecodeBudget::new();
    // Step 1: bound the structural streams before lopdf can decode
    // them at load.
    scan_structural_streams(bytes, &mut budget)?;
    // Step 2: one load, shared by the pre-scan and the extraction.
    let doc = Document::load_mem(bytes)
        .map_err(|e| ExtractError::Parse(format!("lopdf: {e}")))?;
    // Step 3: encryption is refused. (An empty-password document
    // arrives here with the /Encrypt entry already stripped by
    // lopdf, so it is treated as unencrypted.)
    if doc.trailer.get(b"Encrypt").is_ok() {
        return Err(ExtractError::Encrypted);
    }
    // Step 5 (page count) before any text work. Zero pages with no
    // error is a parse error, never "empty" — a broken /Root or an
    // undecryptable page tree both present this way.
    let page_map = doc.get_pages();
    if page_map.is_empty() {
        return Err(ExtractError::Parse(
            "document has no pages".to_string(),
        ));
    }
    if page_map.len() > MAX_PDF_PAGES {
        return Err(ExtractError::PageCap);
    }
    // Step 4: bound every content and form stream.
    prescan_content_streams(&doc, &page_map, &mut budget)?;
    // Step 6: per-page extraction, stopping at the text caps.
    let mut extracted = Vec::with_capacity(page_map.len());
    let mut total_chars = 0usize;
    for (number, _id) in &page_map {
        let number = *number;
        let mut text = String::new();
        {
            let mut output = PlainTextOutput::new(&mut text);
            output_doc_page(&doc, &mut output, number)
                .map_err(|e| ExtractError::Parse(format!("pdf-extract page {number}: {e}")))?;
        }
        // pdf-extract adds framing whitespace per page — two leading
        // newlines and one trailing newline (the spike's debug output
        // confirms the leading pair; its `trim_end()` kept them, so
        // the design's "trailing newline only" description is
        // incomplete). The stored page text is the trimmed form,
        // matching the design's page manifest exactly.
        let text = text.trim().to_string();
        let chars = text.chars().count();
        if chars > MAX_PDF_PAGE_CHARS {
            return Err(ExtractError::TextCap);
        }
        total_chars += chars;
        if total_chars > MAX_PDF_CHARS {
            return Err(ExtractError::TextCap);
        }
        extracted.push(ExtractedPage { number, text });
    }
    if total_chars == 0 {
        return Err(ExtractError::Empty);
    }
    Ok(build_result(extracted))
}

/// Join the pages into the stored body and compute per-page
/// provenance. Ranges are contiguous, in order, and cover the body
/// exactly (the separator is 3 characters).
fn build_result(pages: Vec<ExtractedPage>) -> ExtractedPdf {
    let mut body = String::new();
    let mut provenance = Vec::with_capacity(pages.len());
    for (i, page) in pages.iter().enumerate() {
        if i > 0 {
            body.push_str(PAGE_SEPARATOR);
        }
        let start_char = body.chars().count() as u64;
        let chars = page.text.chars().count() as u64;
        provenance.push(PdfPageProvenance {
            page: page.number,
            start_char,
            chars,
            hash: content::hash(page.text.as_bytes()),
        });
        body.push_str(&page.text);
    }
    ExtractedPdf {
        pages,
        body,
        page_provenance: provenance,
    }
}

// ---------------------------------------------------------------- budget

/// The cumulative decode budget. Every decoded byte — across all
/// streams and all filter stages — is charged here; exceeding it is
/// a [`ExtractError::Bomb`].
struct DecodeBudget {
    remaining: usize,
}

impl DecodeBudget {
    fn new() -> Self {
        Self {
            remaining: MAX_DECODED_BYTES,
        }
    }

    /// Charge `n` decoded bytes. Refuses when the charge would take
    /// the total past the budget.
    fn charge(&mut self, n: usize) -> Result<(), ExtractError> {
        if n > self.remaining {
            return Err(ExtractError::Bomb);
        }
        self.remaining -= n;
        Ok(())
    }
}

// ------------------------------------------------------- filter decoding

/// A stream's filter chain, read from its dictionary: the filter
/// names in decoding order, and whether any DecodeParms requests a
/// predictor (which is refused).
struct FilterChain {
    filters: Vec<Vec<u8>>,
    has_predictor: bool,
}

fn filter_chain(dict: &Dictionary) -> Result<FilterChain, ExtractError> {
    let mut filters = Vec::new();
    if let Ok(filter) = dict.get(b"Filter") {
        match filter {
            Object::Name(name) => filters.push(name.clone()),
            Object::Array(items) => {
                for item in items {
                    match item {
                        Object::Name(name) => filters.push(name.clone()),
                        _ => {
                            return Err(ExtractError::Unsupported(
                                "non-name entry in /Filter".to_string(),
                            ));
                        }
                    }
                }
            }
            _ => {
                return Err(ExtractError::Unsupported(
                    "malformed /Filter entry".to_string(),
                ));
            }
        }
    }
    // A predictor is image-data machinery; text content streams do
    // not use one, and this module does not implement it.
    let mut has_predictor = false;
    if let Ok(parms) = dict.get(b"DecodeParms") {
        let dicts: Vec<&Dictionary> = match parms {
            Object::Dictionary(d) => vec![d],
            Object::Array(items) => items
                .iter()
                .filter_map(|item| match item {
                    Object::Dictionary(d) => Some(d),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        };
        for d in dicts {
            if let Ok(Object::Integer(p)) = d.get(b"Predictor") {
                if *p > 1 {
                    has_predictor = true;
                }
            }
        }
    }
    Ok(FilterChain {
        filters,
        has_predictor,
    })
}

/// Decode a stream's raw bytes through its filter chain, charging
/// every stage's output to the budget. Returns the decoded bytes
/// (bounded by the budget, so holding them is safe). Mirrors lopdf's
/// `decompressed_content` filter set and its zlib fallback (raw
/// deflate on a checksum failure), but every stage is capped — the
/// uncapped original is what the bomb exploits.
fn decode_stream(
    data: &[u8],
    dict: &Dictionary,
    budget: &mut DecodeBudget,
) -> Result<Vec<u8>, ExtractError> {
    let chain = filter_chain(dict)?;
    if chain.has_predictor {
        return Err(ExtractError::Unsupported(
            "predictor decoding (/DecodeParms /Predictor) is not supported".to_string(),
        ));
    }
    // No filter: the raw bytes are the decoded size.
    if chain.filters.is_empty() {
        budget.charge(data.len())?;
        return Ok(data.to_vec());
    }
    let mut input: Vec<u8> = data.to_vec();
    for filter in &chain.filters {
        input = match filter.as_slice() {
            b"FlateDecode" => decode_flate(&input, budget)?,
            b"LZWDecode" => decode_lzw(&input, dict, budget)?,
            b"ASCII85Decode" => decode_ascii85(&input, budget)?,
            _ => {
                return Err(ExtractError::Unsupported(format!(
                    "filter {} is not supported",
                    String::from_utf8_lossy(filter)
                )));
            }
        };
    }
    Ok(input)
}

/// Capped FlateDecode. Reads through a decoder in 8 KiB chunks,
/// charging each chunk to the budget, so the decoder aborts
/// mid-stream the moment the output would exceed it. On a zlib
/// failure with empty output (corrupt adler32, as in encrypted
/// PDFs), falls back to raw deflate on `input[2..]`, mirroring lopdf.
fn decode_flate(input: &[u8], budget: &mut DecodeBudget) -> Result<Vec<u8>, ExtractError> {
    let mut out = Vec::new();
    let mut decoder = ZlibDecoder::new(input);
    match read_capped(&mut decoder, &mut out, budget) {
        Ok(()) => Ok(out),
        Err(ExtractError::Parse(_)) if out.is_empty() && input.len() > 2 => {
            let mut raw = DeflateDecoder::new(&input[2..]);
            let mut out2 = Vec::new();
            read_capped(&mut raw, &mut out2, budget)?;
            Ok(out2)
        }
        Err(e) => Err(e),
    }
}

/// Read a decoder to the end, charging each chunk to the budget and
/// refusing the moment the output would exceed it. A decode error is
/// a parse error (the stream is malformed); a budget refusal is a
/// bomb.
fn read_capped<R: Read>(
    reader: &mut R,
    out: &mut Vec<u8>,
    budget: &mut DecodeBudget,
) -> Result<(), ExtractError> {
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                budget.charge(n)?;
                out.extend_from_slice(&chunk[..n]);
            }
            Err(e) => {
                return Err(ExtractError::Parse(format!("decode error: {e}")));
            }
        }
    }
}

/// Capped LZWDecode via weezl, mirroring lopdf's decoder setup
/// (MSB-first, min code size 8, TIFF early-change when the
/// /DecodeParms /EarlyChange is not 0 — the PDF default). The input
/// is fed in 16-byte slices and the output is counted after each
/// one, so a bomb is refused mid-decode with at most one slice's
/// output past the budget. A weezl `InvalidCode` is a parse error
/// (malformed stream); lopdf tolerates it with partial output, this
/// module refuses it.
fn decode_lzw(
    input: &[u8],
    dict: &Dictionary,
    budget: &mut DecodeBudget,
) -> Result<Vec<u8>, ExtractError> {
    let early_change = dict
        .get(b"DecodeParms")
        .ok()
        .and_then(|p| match p {
            Object::Dictionary(d) => d.get(b"EarlyChange").ok(),
            _ => None,
        })
        .and_then(|v| v.as_i64().ok())
        .map(|v| v != 0)
        .unwrap_or(true);
    let mut decoder = if early_change {
        LzwDecoder::with_tiff_size_switch(BitOrder::Msb, 8)
    } else {
        LzwDecoder::new(BitOrder::Msb, 8)
    };
    let mut out: Vec<u8> = Vec::new();
    let mut offset = 0usize;
    let mut iterations = 0usize;
    loop {
        if offset >= input.len() {
            break;
        }
        // Defensive cap: each iteration consumes input, produces
        // output (charged to the finite budget), or ends — so the
        // loop cannot spin forever on a hostile stream.
        iterations += 1;
        if iterations > input.len() + 4096 {
            return Err(ExtractError::Parse(
                "LZW decode did not terminate".to_string(),
            ));
        }
        let end = (offset + 16).min(input.len());
        let before = out.len();
        let result = {
            let mut stream = decoder.into_vec(&mut out);
            stream.decode(&input[offset..end])
        };
        let produced = out.len() - before;
        if produced > 0 {
            budget.charge(produced)?;
        }
        if let Err(e) = result.status {
            return Err(ExtractError::Parse(format!("LZW decode error: {e:?}")));
        }
        if decoder.has_ended() {
            break;
        }
        if result.consumed_in == 0 && produced == 0 {
            // No progress on a non-empty slice: the decoder is stuck.
            return Err(ExtractError::Parse(
                "LZW decode made no progress".to_string(),
            ));
        }
        // Advance by exactly what was consumed. The decoder may emit
        // output for a code buffered from the previous slice without
        // consuming input; advancing by more would skip a byte.
        offset += result.consumed_in;
    }
    Ok(out)
}

/// Capped ASCII85Decode. ASCII85 output is at most 4/5 of the input
/// (5 chars -> 4 bytes), so a pre-check against the remaining budget
/// *is* the bound: if the input cannot expand past the budget, the
/// decode cannot either. `z` is the 4-zero-bytes shorthand; `~>`
/// terminates the data.
fn decode_ascii85(input: &[u8], budget: &mut DecodeBudget) -> Result<Vec<u8>, ExtractError> {
    let max_out = input.len().saturating_mul(4) / 5 + 4;
    budget.charge(max_out)?;
    let mut out = Vec::with_capacity(max_out.min(1 << 20));
    let mut group: Vec<u8> = Vec::with_capacity(5);
    let mut i = 0usize;
    while i < input.len() {
        let b = input[i];
        i += 1;
        match b {
            b'~' => {
                // `~>` ends the data; anything else is malformed.
                if i < input.len() && input[i] == b'>' {
                    break;
                }
                return Err(ExtractError::Parse(
                    "malformed ASCII85 data".to_string(),
                ));
            }
            b'z' if group.is_empty() => out.extend_from_slice(&[0, 0, 0, 0]),
            b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x00 => {}
            33..=117 => group.push(b),
            _ => {
                return Err(ExtractError::Parse(
                    "malformed ASCII85 data".to_string(),
                ));
            }
        }
        if group.len() == 5 {
            let mut value: u32 = 0;
            for &c in &group {
                value = value
                    .checked_mul(85)
                    .and_then(|v| v.checked_add((c - 33) as u32))
                    .ok_or_else(|| ExtractError::Parse("ASCII85 group overflow".to_string()))?;
            }
            out.extend_from_slice(&value.to_be_bytes());
            group.clear();
        }
    }
    if !group.is_empty() {
        // A short final group is padded with `u` (84) and only the
        // first (len - 1) bytes are kept.
        if group.len() == 1 {
            return Err(ExtractError::Parse(
                "malformed ASCII85 data".to_string(),
            ));
        }
        let n = group.len();
        while group.len() < 5 {
            group.push(b'u');
        }
        let mut value: u32 = 0;
        for &c in &group {
            value = value
                .checked_mul(85)
                .and_then(|v| v.checked_add((c - 33) as u32))
                .ok_or_else(|| ExtractError::Parse("ASCII85 group overflow".to_string()))?;
        }
        out.extend_from_slice(&value.to_be_bytes()[..n - 1]);
    }
    Ok(out)
}

// --------------------------------------------- pre-load structural scan

/// The largest stream dictionary this module will parse (stream
/// dictionaries are small; a larger one is malformed or hostile).
const MAX_DICT_BYTES: usize = 8192;

/// Scan the raw file for compressed-structure streams — `/ObjStm`
/// and `/XRef` — and decode each through the capped decoder, charging
/// the cumulative budget. This runs *before* `Document::load_mem`,
/// because lopdf decompresses every object stream during the load
/// itself; a bomb there would detonate before any module code ran.
/// The scan finds every such stream in the file (any revision), a
/// superset of the ones the load decodes, so bounding all of them
/// bounds the load.
fn scan_structural_streams(bytes: &[u8], budget: &mut DecodeBudget) -> Result<(), ExtractError> {
    let mut offset = 0usize;
    while let Some(header) = find_object_header(bytes, offset) {
        offset = header.after_header;
        // The object body must start with a dictionary for a stream;
        // anything else is not a stream object and is skipped.
        let body_start = skip_ws(bytes, offset);
        if body_start + 2 > bytes.len() || &bytes[body_start..body_start + 2] != b"<<" {
            continue;
        }
        let dict_end = body_start + MAX_DICT_BYTES.min(bytes.len() - body_start);
        let dict = match parse_dict(&bytes[body_start..dict_end]) {
            Ok((dict, _consumed)) => dict,
            Err(_) => {
                return Err(ExtractError::Parse(
                    "unparseable stream dictionary".to_string(),
                ));
            }
        };
        let is_structural = matches!(
            dict.get(b"Type").ok().and_then(|v| match v {
                Object::Name(n) => Some(n.as_slice()),
                _ => None,
            }),
            Some(b"ObjStm") | Some(b"XRef")
        );
        if !is_structural {
            continue;
        }
        // Locate the stream data: the `stream` keyword after the
        // dictionary, then the bytes up to `endstream`. The /Length
        // entry is not trusted (it may be an indirect reference);
        // the data is whatever lies between the markers.
        let stream_data = match locate_stream_data(bytes, body_start) {
            Some(data) => data,
            None => {
                return Err(ExtractError::Parse(
                    "structural stream without stream data".to_string(),
                ));
            }
        };
        decode_stream(stream_data, &dict, budget)?;
    }
    Ok(())
}

/// Find the next indirect-object header (`N G obj`) at or after
/// `from`, returning the offset just past the header. Scans the whole
/// file (not just line starts): an object header mid-line is legal,
/// and a bomb hidden there must still be found. A match inside a
/// stream's text is harmless — the dictionary check after it either
/// skips it (not a stream) or refuses it (fail closed).
fn find_object_header(bytes: &[u8], from: usize) -> Option<ObjectHeader> {
    let mut i = from;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let num_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i >= bytes.len() || !is_ws(bytes[i]) {
            i = num_start + 1;
            continue;
        }
        let gen_start = skip_ws(bytes, i);
        let mut j = gen_start;
        if j >= bytes.len() || !bytes[j].is_ascii_digit() {
            i = num_start + 1;
            continue;
        }
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j >= bytes.len() || !is_ws(bytes[j]) {
            i = num_start + 1;
            continue;
        }
        let obj_start = skip_ws(bytes, j);
        if bytes.len() - obj_start >= 3 && &bytes[obj_start..obj_start + 3] == b"obj" {
            let after = obj_start + 3;
            if after >= bytes.len() || is_ws(bytes[after]) {
                return Some(ObjectHeader {
                    after_header: after,
                });
            }
        }
        i = num_start + 1;
    }
    None
}

struct ObjectHeader {
    after_header: usize,
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x00)
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && is_ws(bytes[i]) {
        i += 1;
    }
    i
}

/// Locate a stream's data: the `stream` keyword after the dictionary
/// (followed by CRLF or LF), up to the `endstream` keyword. Returns
/// the raw stream bytes (still filtered).
fn locate_stream_data<'a>(bytes: &'a [u8], dict_start: usize) -> Option<&'a [u8]> {
    // The dictionary ends at the matching `>>`; `stream` follows.
    let mut depth = 0usize;
    let mut i = dict_start;
    let mut dict_end = None;
    while i + 1 < bytes.len() {
        if &bytes[i..i + 2] == b"<<" {
            depth += 1;
            i += 2;
        } else if &bytes[i..i + 2] == b">>" {
            depth -= 1;
            i += 2;
            if depth == 0 {
                dict_end = Some(i);
                break;
            }
        } else {
            i += 1;
        }
    }
    let mut pos = skip_ws(bytes, dict_end?);
    if bytes.len() - pos < 6 || &bytes[pos..pos + 6] != b"stream" {
        return None;
    }
    pos += 6;
    // `stream` must be followed by CRLF or LF (a lone CR is also
    // accepted by lopdf; accept it here too).
    if pos < bytes.len() && bytes[pos] == b'\r' {
        pos += 1;
    }
    if pos < bytes.len() && bytes[pos] == b'\n' {
        pos += 1;
    }
    let data_start = pos;
    // Find `endstream` from here (bounded by the file end).
    let mut j = data_start;
    while j + 9 <= bytes.len() {
        if &bytes[j..j + 9] == b"endstream" {
            let mut end = j;
            // The data ends at the EOL before `endstream`.
            if end > data_start && bytes[end - 1] == b'\n' {
                end -= 1;
            }
            if end > data_start && bytes[end - 1] == b'\r' {
                end -= 1;
            }
            return Some(&bytes[data_start..end]);
        }
        j += 1;
    }
    None
}

// ------------------------------------------------- minimal dict parser

/// A parsed PDF dictionary value (the subset the structural scan
/// needs). Names decode `#XX` hex escapes, so `/Obj#53tm` is the
/// same name as `/ObjStm` and cannot evade the scan.
#[derive(Debug, Clone, PartialEq)]
enum PdfValue {
    Name(Vec<u8>),
    Integer(i64),
    Real(f32),
    Reference(u32, u16),
    Array(Vec<PdfValue>),
    Dict(BTreeMap<Vec<u8>, PdfValue>),
    Str(Vec<u8>),
    Bool(bool),
    Null,
}

/// Parse a `<< ... >>` dictionary starting at `bytes[0]`, returning
/// the dictionary and the number of bytes consumed. Bounded: the
/// caller caps the input window, and nesting is limited to depth 8.
fn parse_dict(bytes: &[u8]) -> Result<(Dictionary, usize), ExtractError> {
    let (value, consumed) = parse_value(bytes, 0, 0)?;
    match value {
        PdfValue::Dict(entries) => {
            let mut dict = Dictionary::new();
            for (key, val) in entries {
                dict.set(key, pdf_value_to_object(val));
            }
            Ok((dict, consumed))
        }
        _ => Err(ExtractError::Parse("expected a dictionary".to_string())),
    }
}

fn pdf_value_to_object(value: PdfValue) -> Object {
    match value {
        PdfValue::Name(n) => Object::Name(n),
        PdfValue::Integer(i) => Object::Integer(i),
        PdfValue::Real(r) => Object::Real(r),
        PdfValue::Reference(id, generation) => Object::Reference((id, generation)),
        PdfValue::Array(items) => {
            Object::Array(items.into_iter().map(pdf_value_to_object).collect())
        }
        PdfValue::Dict(entries) => {
            let mut dict = Dictionary::new();
            for (key, val) in entries {
                dict.set(key, pdf_value_to_object(val));
            }
            Object::Dictionary(dict)
        }
        PdfValue::Str(s) => Object::String(s, lopdf::StringFormat::Literal),
        PdfValue::Bool(b) => Object::Boolean(b),
        PdfValue::Null => Object::Null,
    }
}

fn parse_value(
    bytes: &[u8],
    mut i: usize,
    depth: usize,
) -> Result<(PdfValue, usize), ExtractError> {
    if depth > 8 {
        return Err(ExtractError::Parse(
            "dictionary nesting too deep".to_string(),
        ));
    }
    i = skip_ws(bytes, i);
    if i >= bytes.len() {
        return Err(ExtractError::Parse(
            "unexpected end of dictionary".to_string(),
        ));
    }
    match bytes[i] {
        b'<' if i + 1 < bytes.len() && bytes[i + 1] == b'<' => {
            // Dictionary.
            i += 2;
            let mut entries = BTreeMap::new();
            loop {
                i = skip_ws(bytes, i);
                if i + 1 < bytes.len() && &bytes[i..i + 2] == b">>" {
                    return Ok((PdfValue::Dict(entries), i + 2));
                }
                if i >= bytes.len() || bytes[i] != b'/' {
                    return Err(ExtractError::Parse("malformed dictionary".to_string()));
                }
                let (name, next) = parse_name(bytes, i)?;
                i = skip_ws(bytes, next);
                let (value, next) = parse_value(bytes, i, depth + 1)?;
                entries.insert(name, value);
                i = next;
            }
        }
        b'<' => {
            // Hex string.
            i += 1;
            let mut hex = Vec::new();
            while i < bytes.len() && bytes[i] != b'>' {
                if !is_ws(bytes[i]) {
                    hex.push(bytes[i]);
                }
                i += 1;
            }
            if i >= bytes.len() {
                return Err(ExtractError::Parse(
                    "unterminated hex string".to_string(),
                ));
            }
            if hex.len() % 2 == 1 {
                hex.push(b'0');
            }
            let mut out = Vec::with_capacity(hex.len() / 2);
            for pair in hex.chunks_exact(2) {
                let hi = hex_val(pair[0])
                    .ok_or_else(|| ExtractError::Parse("bad hex digit".to_string()))?;
                let lo = hex_val(pair[1])
                    .ok_or_else(|| ExtractError::Parse("bad hex digit".to_string()))?;
                out.push((hi << 4) | lo);
            }
            Ok((PdfValue::Str(out), i + 1))
        }
        b'(' => parse_literal_string(bytes, i),
        b'[' => {
            i += 1;
            let mut items = Vec::new();
            loop {
                i = skip_ws(bytes, i);
                if i >= bytes.len() {
                    return Err(ExtractError::Parse("unterminated array".to_string()));
                }
                if bytes[i] == b']' {
                    return Ok((PdfValue::Array(items), i + 1));
                }
                let (value, next) = parse_value(bytes, i, depth + 1)?;
                items.push(value);
                i = next;
            }
        }
        b'/' => {
            let (name, next) = parse_name(bytes, i)?;
            Ok((PdfValue::Name(name), next))
        }
        b't' if bytes[i..].starts_with(b"true") => Ok((PdfValue::Bool(true), i + 4)),
        b'f' if bytes[i..].starts_with(b"false") => Ok((PdfValue::Bool(false), i + 5)),
        b'n' if bytes[i..].starts_with(b"null") => Ok((PdfValue::Null, i + 4)),
        b'-' | b'+' | b'.' | b'0'..=b'9' => parse_number_or_reference(bytes, i),
        _ => Err(ExtractError::Parse(
            "unexpected token in dictionary".to_string(),
        )),
    }
}

/// Parse a name (after the `/`), decoding `#XX` hex escapes.
fn parse_name(bytes: &[u8], mut i: usize) -> Result<(Vec<u8>, usize), ExtractError> {
    // Caller guarantees bytes[i] == b'/'
    i += 1;
    let mut name = Vec::new();
    while i < bytes.len() {
        let b = bytes[i];
        if is_ws(b)
            || matches!(
                b,
                b'/' | b'<' | b'>' | b'[' | b']' | b'(' | b')' | b'{' | b'}' | b'%'
            )
        {
            break;
        }
        if b == b'#' {
            if i + 2 >= bytes.len() {
                return Err(ExtractError::Parse("bad name escape".to_string()));
            }
            let hi = hex_val(bytes[i + 1])
                .ok_or_else(|| ExtractError::Parse("bad name escape".to_string()))?;
            let lo = hex_val(bytes[i + 2])
                .ok_or_else(|| ExtractError::Parse("bad name escape".to_string()))?;
            name.push((hi << 4) | lo);
            i += 3;
        } else {
            name.push(b);
            i += 1;
        }
    }
    Ok((name, i))
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parse a literal string `( ... )` with nesting and escapes.
fn parse_literal_string(bytes: &[u8], mut i: usize) -> Result<(PdfValue, usize), ExtractError> {
    // Caller guarantees bytes[i] == b'('
    i += 1;
    let mut out = Vec::new();
    let mut depth = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    break;
                }
                match bytes[i] {
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'b' => out.push(8),
                    b'f' => out.push(12),
                    b'(' => out.push(b'('),
                    b')' => out.push(b')'),
                    b'\\' => out.push(b'\\'),
                    b'\n' => {}
                    b'\r' => {
                        if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                            i += 1;
                        }
                    }
                    b'0'..=b'7' => {
                        let mut val: u32 = 0;
                        let mut n = 0;
                        while n < 3 && i < bytes.len() && matches!(bytes[i], b'0'..=b'7') {
                            val = val * 8 + (bytes[i] - b'0') as u32;
                            i += 1;
                            n += 1;
                        }
                        if val <= 255 {
                            out.push(val as u8);
                        }
                        continue;
                    }
                    other => out.push(other),
                }
                i += 1;
            }
            b'(' => {
                depth += 1;
                out.push(b'(');
                i += 1;
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((PdfValue::Str(out), i + 1));
                }
                out.push(b')');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    Err(ExtractError::Parse(
        "unterminated literal string".to_string(),
    ))
}

/// Parse a number, or a reference `N G R` (the two integers followed
/// by `R`). Only integers are references; reals are plain numbers.
fn parse_number_or_reference(bytes: &[u8], i: usize) -> Result<(PdfValue, usize), ExtractError> {
    let mut j = i;
    if j < bytes.len() && (bytes[j] == b'-' || bytes[j] == b'+') {
        j += 1;
    }
    let int_start = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j == int_start {
        return Err(ExtractError::Parse("expected a number".to_string()));
    }
    let int_text = std::str::from_utf8(&bytes[int_start..j])
        .map_err(|_| ExtractError::Parse("bad number".to_string()))?;
    if j < bytes.len() && bytes[j] == b'.' {
        // Real number.
        j += 1;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        let text = std::str::from_utf8(&bytes[i..j])
            .map_err(|_| ExtractError::Parse("bad number".to_string()))?;
        let value: f32 = text
            .parse()
            .map_err(|_| ExtractError::Parse("bad number".to_string()))?;
        return Ok((PdfValue::Real(value), j));
    }
    let first: i64 = int_text
        .parse()
        .map_err(|_| ExtractError::Parse("bad number".to_string()))?;
    // Maybe a reference: `N G R`.
    let k = skip_ws(bytes, j);
    let mut l = k;
    if l < bytes.len() && bytes[l].is_ascii_digit() {
        while l < bytes.len() && bytes[l].is_ascii_digit() {
            l += 1;
        }
        let gen_text = std::str::from_utf8(&bytes[k..l])
            .map_err(|_| ExtractError::Parse("bad number".to_string()))?;
        let m = skip_ws(bytes, l);
        if m < bytes.len() && bytes[m] == b'R' {
            let generation: u16 = gen_text
                .parse()
                .map_err(|_| ExtractError::Parse("bad generation".to_string()))?;
            if first >= 0 && first <= u32::MAX as i64 {
                return Ok((PdfValue::Reference(first as u32, generation), m + 1));
            }
        }
    }
    Ok((PdfValue::Integer(first), j))
}

// --------------------------------------------- content pre-scan (forms)

/// Maximum form-nesting depth the pre-scan follows (plus the visited
/// set, which stops cycles). Real documents nest forms a few levels
/// deep; deeper nesting is hostile or broken.
const MAX_FORM_DEPTH: usize = 32;

/// Decode every page content stream and every Form XObject stream
/// (recursively) through the capped decoder, charging the cumulative
/// budget. This is what bounds the extractor: it decodes exactly
/// these streams, so after this pass its total decode is already
/// proven to fit.
fn prescan_content_streams(
    doc: &Document,
    page_map: &BTreeMap<u32, ObjectId>,
    budget: &mut DecodeBudget,
) -> Result<(), ExtractError> {
    let mut visited: HashSet<ObjectId> = HashSet::new();
    for (_number, page_id) in page_map {
        let page_id = *page_id;
        for content_id in doc.get_page_contents(page_id) {
            prescan_stream(doc, content_id, budget)?;
        }
        let page_dict = doc
            .get_object(page_id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .cloned();
        if let Some(page_dict) = page_dict {
            prescan_resources(doc, &page_dict, budget, &mut visited, 0)?;
        }
    }
    Ok(())
}

/// Decode one stream object through the capped decoder.
fn prescan_stream(
    doc: &Document,
    id: ObjectId,
    budget: &mut DecodeBudget,
) -> Result<(), ExtractError> {
    let object = doc
        .get_object(id)
        .map_err(|_| ExtractError::Parse(format!("missing stream {id:?}")))?;
    let stream = object
        .as_stream()
        .map_err(|_| ExtractError::Parse(format!("object {id:?} is not a stream")))?;
    decode_stream(&stream.content, &stream.dict, budget)?;
    Ok(())
}

/// Walk a dictionary's /Resources /XObject entries, decoding every
/// Form XObject stream recursively. `visited` stops cycles (a form
/// referencing itself, directly or through another form); the depth
/// cap stops pathological nesting.
fn prescan_resources(
    doc: &Document,
    dict: &Dictionary,
    budget: &mut DecodeBudget,
    visited: &mut HashSet<ObjectId>,
    depth: usize,
) -> Result<(), ExtractError> {
    if depth > MAX_FORM_DEPTH {
        return Err(ExtractError::Parse("form nesting too deep".to_string()));
    }
    let resources = match dict.get(b"Resources").and_then(|o| o.as_dict()) {
        Ok(d) => d,
        Err(_) => return Ok(()),
    };
    let xobjects = match resources.get(b"XObject").and_then(|o| o.as_dict()) {
        Ok(d) => d,
        Err(_) => return Ok(()),
    };
    for (_, value) in xobjects.iter() {
        let id = match value {
            Object::Reference((id, generation)) => (*id, *generation),
            _ => continue,
        };
        if !visited.insert(id) {
            continue;
        }
        let object = match doc.get_object(id) {
            Ok(o) => o,
            Err(_) => continue,
        };
        let stream = match object.as_stream() {
            Ok(s) => s,
            Err(_) => continue,
        };
        let is_form = stream
            .dict
            .get(b"Subtype")
            .ok()
            .and_then(|v| match v {
                Object::Name(n) => Some(n.as_slice()),
                _ => None,
            })
            == Some(b"Form");
        if !is_form {
            continue;
        }
        decode_stream(&stream.content, &stream.dict, budget)?;
        prescan_resources(doc, &stream.dict, budget, visited, depth + 1)?;
    }
    Ok(())
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/pdf")
            .join(name);
        std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {name} unreadable: {e}"))
    }

    fn expect_imported(name: &str, expected_pages: &[&str]) {
        let bytes = fixture(name);
        let result =
            extract_pdf_text(&bytes).unwrap_or_else(|e| panic!("fixture {name} should import, got {e}"));
        let got: Vec<&str> = result.pages.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(got, expected_pages, "fixture {name} page text");
        // Provenance covers the body exactly.
        let mut pos = 0u64;
        for (i, prov) in result.page_provenance.iter().enumerate() {
            assert_eq!(prov.start_char, pos, "fixture {name} page {} start", i + 1);
            assert_eq!(
                prov.chars,
                result.pages[i].text.chars().count() as u64,
                "fixture {name} page {} chars",
                i + 1
            );
            assert_eq!(
                prov.hash,
                content::hash(result.pages[i].text.as_bytes()),
                "fixture {name} page {} hash",
                i + 1
            );
            pos += prov.chars;
            if i + 1 < result.pages.len() {
                pos += PAGE_SEPARATOR.chars().count() as u64;
            }
        }
        assert_eq!(
            pos,
            result.body.chars().count() as u64,
            "fixture {name} body coverage"
        );
    }

    fn expect_refused(name: &str, expected: &ExtractError) {
        let bytes = fixture(name);
        let err = extract_pdf_text(&bytes).unwrap_err();
        assert_eq!(
            std::mem::discriminant(&err),
            std::mem::discriminant(expected),
            "fixture {name}: expected {expected:?}, got {err:?}"
        );
    }

    #[test]
    fn normal_three_pages() {
        expect_imported(
            "normal-3page.pdf",
            &[
                "Harbor Ledger\nThe bramblewick shipment cleared customs on Tuesday.\nThree crates of copper fittings, two of rope.",
                "Second page\nThe aurora storage annex holds the spare manifests.\nCafé service ends at noon.",
                "Third page\nTotals: 41 crates, 12 pallets, 1 lighthouse keeper.",
            ],
        );
    }

    #[test]
    fn unicode_cmap_pages() {
        expect_imported("unicode-cmap.pdf", &["\u{3b1}\u{3b2}\u{20ac}"]);
    }

    #[test]
    fn scanned_and_empty_are_refused() {
        expect_refused("scanned-image.pdf", &ExtractError::Empty);
        expect_refused("empty-text.pdf", &ExtractError::Empty);
    }

    #[test]
    fn malformed_are_refused() {
        expect_refused("malformed-truncated.pdf", &ExtractError::Parse(String::new()));
        expect_refused("malformed-header.pdf", &ExtractError::Parse(String::new()));
        expect_refused("malformed-xref.pdf", &ExtractError::Parse(String::new()));
    }

    #[test]
    fn encrypted_is_refused() {
        expect_refused("encrypted-user.pdf", &ExtractError::Encrypted);
    }

    #[test]
    fn caps_are_refused() {
        expect_refused("too-many-pages.pdf", &ExtractError::PageCap);
        expect_refused("oversized-text.pdf", &ExtractError::TextCap);
        let bytes = fixture("oversized-raw.pdf");
        assert!(matches!(
            extract_pdf_text(&bytes),
            Err(ExtractError::Parse(_))
        ));
    }

    #[test]
    fn decompression_bomb_is_refused() {
        expect_refused("decompression-bomb.pdf", &ExtractError::Bomb);
    }

    #[test]
    fn object_streams() {
        expect_imported(
            "object-stream-ok.pdf",
            &["Harbor Ledger\nThe object-stream shipment cleared customs."],
        );
        expect_refused("object-stream-bomb.pdf", &ExtractError::Bomb);
    }

    #[test]
    fn xref_streams() {
        expect_imported(
            "xref-stream-ok.pdf",
            &["Harbor Ledger\nThe xref-stream shipment cleared customs."],
        );
        expect_refused("xref-stream-bomb.pdf", &ExtractError::Bomb);
    }

    #[test]
    fn chained_filters() {
        expect_imported(
            "chained-filter-ok.pdf",
            &["Harbor Ledger\nThe chained-filter shipment cleared customs."],
        );
        expect_refused("chained-filter-bomb.pdf", &ExtractError::Bomb);
    }

    #[test]
    fn lzw_streams() {
        expect_imported(
            "lzw-ok.pdf",
            &["Harbor Ledger\nThe lzw shipment cleared customs."],
        );
        expect_refused("lzw-bomb.pdf", &ExtractError::Bomb);
    }

    #[test]
    fn nested_forms() {
        expect_imported(
            "nested-form-ok.pdf",
            &["Harbor Ledger\nThe nested-form shipment cleared customs."],
        );
        expect_refused("nested-form-bomb.pdf", &ExtractError::Bomb);
    }

    #[test]
    fn raw_cap_is_checked() {
        let mut bytes = fixture("normal-3page.pdf");
        bytes.resize(MAX_PDF_BYTES + 1, 0);
        assert!(matches!(
            extract_pdf_text(&bytes),
            Err(ExtractError::Parse(_))
        ));
    }

    #[test]
    fn dict_parser_decodes_name_escapes() {
        // /Obj#53tm is the name ObjStm; the scan must not be evaded
        // by a hex escape.
        let (dict, _) = parse_dict(b"<< /Type /Obj#53tm /N 1 >>").unwrap();
        let ty = dict.get(b"Type").unwrap();
        assert_eq!(ty, &Object::Name(b"ObjStm".to_vec()));
    }

    #[test]
    fn ascii85_known_vector() {
        // Independent vector: Python's base64.a85encode of
        // "Hello, ASCII85! The quick brown fox." (36 bytes), Adobe
        // form without the <~ prefix, ending with ~>.
        let input = b"87cURD_*\"s;aX,J3&Mj%<+ohcEHPu*CER),Dg-(AAoDoH~>";
        let mut budget = DecodeBudget::new();
        let mut dict = Dictionary::new();
        dict.set("Filter", Object::Name(b"ASCII85Decode".to_vec()));
        let decoded = decode_ascii85(input, &mut budget).unwrap();
        assert_eq!(&decoded, b"Hello, ASCII85! The quick brown fox.");
    }

    #[test]
    fn unsupported_filter_is_refused() {
        let mut budget = DecodeBudget::new();
        let mut dict = Dictionary::new();
        dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
        let err = decode_stream(b"\x00\x01\x02", &dict, &mut budget).unwrap_err();
        assert!(matches!(err, ExtractError::Unsupported(_)));
    }
}
