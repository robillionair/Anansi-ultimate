//! DOCX parser spike harness (M13, spike branch only).
//!
//! Bounded ZIP walk + streaming XML parse of `word/document.xml`,
//! implementing design §3 limits and §4 extraction rules from
//! `docs/handoffs/m13-docx-ingestion-design.md`. Not production
//! code: it exists to produce the evidence for
//! `docs/handoffs/m13-docx-spike-results.md`.
//!
//! Usage (one fixture per process for a meaningful peak RSS):
//!   cargo run -p anansi-core --example docx_spike -- all
//!   cargo run -p anansi-core --example docx_spike -- <fixture.docx>
//!   cargo run -p anansi-core --example docx_spike -- <fixture.docx> uncapped
//!   cargo run -p anansi-core --example docx_spike -- worstbomb
//! Peak RSS (Linux VmHWM) and elapsed time are printed per run.

use quick_xml::events::Event;
use quick_xml::Reader;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::time::Instant;
use zip::ZipArchive;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/docx");
const MAX_RAW_BYTES: usize = 1 << 20;
const MAX_TEXT_CHARS: usize = 32_000;
const MAX_ENTRIES: usize = 128;
const MAX_INFLATED_TOTAL: usize = 16 * 1024 * 1024;
const MAX_ENTRY_INFLATED: usize = 8 * 1024 * 1024;
const MAX_PARAGRAPHS: usize = 10_000;

fn peak_rss_kb() -> u64 {
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            return rest.trim().trim_end_matches("kB").trim().parse().unwrap_or(0);
        }
    }
    0
}

fn fixture_bytes(name: &str) -> Vec<u8> {
    fs::read(PathBuf::from(FIXTURES).join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn manifest() -> serde_json::Value {
    let text = fs::read_to_string(PathBuf::from(FIXTURES).join("MANIFEST.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

/// A reader that errors the moment the cumulative inflated budget is
/// exceeded, so a lying declared size cannot smuggle a bomb past the
/// pre-check. The error surfaces mid-entry, before any of the
/// over-budget bytes are retained.
struct CappedReader<R: Read> {
    inner: R,
    remaining: usize,
}

impl<R: Read> CappedReader<R> {
    fn new(inner: R, budget: usize) -> Self {
        Self { inner, remaining: budget }
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

#[derive(Debug, Clone)]
struct Section {
    heading: Option<String>,
    level: u8,
    start_char: usize,
    chars: usize,
    hash: String,
}

#[derive(Debug)]
struct Extracted {
    body: String,
    sections: Vec<Section>,
    lines: usize,
}

/// Lines are (plain paragraph text, heading level). The "# " prefix
/// is body formatting, rendered only when joining the body and the
/// section text; `Section.heading` is the plain heading text.
fn render_line(text: &str, level: Option<u8>) -> String {
    match level {
        Some(l) => format!("{} {}", "#".repeat(l as usize), text),
        None => text.to_string(),
    }
}

fn sections_for(lines: &[(String, Option<u8>)]) -> Vec<Section> {
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
    let mut pos = 0usize;
    for (heading, level, group_lines) in groups {
        let rendered: Vec<String> = group_lines
            .iter()
            .map(|(t, l)| render_line(t, *l))
            .collect();
        let text = rendered.join("\n");
        let chars = text.chars().count();
        let hash = sha256_hex(text.as_bytes());
        sections.push(Section { heading, level, start_char: pos, chars, hash });
        pos += chars + 1;
    }
    sections
}

struct ParaState {
    buf: String,
    style: Option<String>,
    outline: Option<u8>,
    heading_used: bool,
}

impl ParaState {
    fn new() -> Self {
        Self { buf: String::new(), style: None, outline: None, heading_used: false }
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

/// Extract per design §3/§4. `caps=false` is the unbounded baseline:
/// it skips only the inflation bound (declared-size pre-check and
/// capped reader); every other rule still applies.
fn extract(bytes: &[u8], caps: bool) -> Result<Extracted, String> {
    if caps && bytes.len() > MAX_RAW_BYTES {
        return Err(format!("raw cap: file is {} bytes (max 1 MiB)", bytes.len()));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("parse error: not a zip archive: {e}"))?;
    if caps && archive.len() > MAX_ENTRIES {
        return Err(format!("entry cap: {} entries (max {MAX_ENTRIES})", archive.len()));
    }
    let mut doc_index: Option<usize> = None;
    let mut has_content_types = false;
    for i in 0..archive.len() {
        // Metadata only: by_index would try to open the entry and
        // errors on an encrypted one before encrypted() is reached.
        let file = archive
            .by_index_data(i)
            .map_err(|e| format!("parse error: entry {i}: {e}"))?;
        let name = file
            .name()
            .map_err(|e| format!("parse error: entry {i} name: {e}"))?
            .into_owned();
        if name == "[Content_Types].xml" {
            has_content_types = true;
        }
        if name == "word/vbaProject.bin" {
            return Err("macro: word/vbaProject.bin present".to_string());
        }
        if file.encrypted() {
            return Err(format!("encrypted entry: {name}"));
        }
        if caps && file.size() > MAX_ENTRY_INFLATED as u64 {
            return Err(format!(
                "single entry exceeds 8 MiB: {name} declares {} bytes",
                file.size()
            ));
        }
        if name == "word/document.xml" {
            doc_index = Some(i);
        }
    }
    if !has_content_types {
        return Err("missing [Content_Types].xml".to_string());
    }
    let doc_index = doc_index.ok_or_else(|| "missing word/document.xml".to_string())?;
    let mut file = archive
        .by_index(doc_index)
        .map_err(|e| format!("parse error: word/document.xml: {e}"))?;
    let mut doc_bytes: Vec<u8> = Vec::new();
    if caps {
        let mut capped = CappedReader::new(&mut file, MAX_INFLATED_TOTAL);
        capped
            .read_to_end(&mut doc_bytes)
            .map_err(|e| {
                if e.to_string().contains("expansion budget exceeded") {
                    "expansion budget exceeded: word/document.xml expands past 16 MiB".to_string()
                } else {
                    format!("parse error: reading word/document.xml: {e}")
                }
            })?;
    } else {
        file.read_to_end(&mut doc_bytes)
            .map_err(|e| format!("parse error: reading word/document.xml: {e}"))?;
    }
    // No DTD, no entity expansion: refuse before parsing.
    if doc_bytes.windows(9).any(|w| w == b"<!DOCTYPE" || w == b"<!doctype") {
        return Err("refused: <!DOCTYPE present".to_string());
    }
    parse_document_xml(&doc_bytes)
}

fn parse_document_xml(doc_bytes: &[u8]) -> Result<Extracted, String> {
    // quick-xml 0.42: the slice reader (read_event, no buffer) needs
    // &str, so UTF-8 is validated up front — a non-UTF-8 document.xml
    // is a parse error, not a transcode attempt.
    let xml_text = std::str::from_utf8(doc_bytes)
        .map_err(|e| format!("parse error: word/document.xml is not UTF-8: {e}"))?;
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
        let level = if para.heading_used { None } else { para.heading_level() };
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
     -> Result<(), String> {
        let n = text.chars().count();
        if *appended_chars + n > MAX_TEXT_CHARS {
            return Err(format!(
                "text cap: exceeded ({} chars, cap {MAX_TEXT_CHARS})",
                *appended_chars + n
            ));
        }
        *appended_chars += n;
        para.buf.push_str(text);
        Ok(())
    };

    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("parse error: malformed XML: {e}"))?;
        match event {
            Event::Eof => break,
            Event::DocType(_) => {
                return Err("refused: <!DOCTYPE present".to_string());
            }
            Event::Start(e) => {
                match e.local_name().as_ref() {
                    "p" => {
                        para_count += 1;
                        if para_count > MAX_PARAGRAPHS {
                            return Err(format!("paragraph cap: {para_count} paragraphs"));
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
                        return Err("refused: w:altChunk present".to_string());
                    }
                    _ => {}
                }
            }
            Event::Empty(e) => {
                match e.local_name().as_ref() {
                    "p" => {
                        para_count += 1;
                        if para_count > MAX_PARAGRAPHS {
                            return Err(format!("paragraph cap: {para_count} paragraphs"));
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
                        return Err("refused: w:altChunk present".to_string());
                    }
                    _ => {}
                }
            }
            Event::Text(e) => {
                if in_t && del_depth == 0 && instr_depth == 0 && skip_depth == 0 && in_para {
                    let raw = e.into_inner();
                    let text = quick_xml::escape::unescape(&raw)
                        .map_err(|err| format!("parse error: bad text: {err}"))?;
                    append_capped(&mut para, &mut appended_chars, &text)?;
                }
            }
            Event::CData(e) => {
                if del_depth == 0 && instr_depth == 0 && skip_depth == 0 && in_para {
                    let text = e.into_inner();
                    append_capped(&mut para, &mut appended_chars, &text)?;
                }
            }
            Event::End(e) => {
                match e.local_name().as_ref() {
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
                }
            }
            _ => {}
        }
    }
    if in_para {
        // Well-formed XML always closes w:p; reaching EOF inside one
        // means the document was truncated mid-paragraph.
        return Err("parse error: unclosed paragraph at end of document".to_string());
    }
    let body = lines
        .iter()
        .map(|(text, level)| render_line(text, *level))
        .collect::<Vec<_>>()
        .join("\n");
    let sections = sections_for(&lines);
    Ok(Extracted { body, sections, lines: lines.len() })
}

fn run_one(name: &str, caps: bool) -> String {
    let bytes = fixture_bytes(name);
    let start = Instant::now();
    let result = extract(&bytes, caps);
    let elapsed = start.elapsed();
    let out = match &result {
        Ok(ex) => format!(
            "OK chars={} lines={} sections={}",
            ex.body.chars().count(),
            ex.lines,
            ex.sections.len()
        ),
        Err(e) => format!("REFUSED {e}"),
    };
    println!(
        "fixture={name} caps={caps}\n{out}\nELAPSED_MS={} PEAK_RSS_KB={}",
        elapsed.as_millis(),
        peak_rss_kb()
    );
    if let Ok(ex) = &result {
        for s in &ex.sections {
            println!(
                "  section heading={:?} level={} start={} chars={} hash={}",
                s.heading, s.level, s.start_char, s.chars, s.hash
            );
        }
    }
    out
}

fn check_manifest(name: &str, result: &Result<Extracted, String>, bytes: &[u8]) -> String {
    let manifest = manifest();
    let entry = &manifest["fixtures"][name];
    if entry.is_null() {
        return "NO-MANIFEST-ENTRY".to_string();
    }
    let want_sha = entry["sha256"].as_str().unwrap_or("");
    if sha256_hex(bytes) != want_sha {
        return format!("MISMATCH sha256: file != manifest");
    }
    let want_outcome = entry["expected_outcome"].as_str().unwrap_or("");
    if let Some(reason) = want_outcome.strip_prefix("refused: ") {
        match result {
            Ok(_) => return format!("MISMATCH expected refused ({reason}), got OK"),
            Err(e) => {
                if !e.to_lowercase().contains(&reason.to_lowercase()) {
                    return format!("MISMATCH refused reason: want '{reason}', got '{e}'");
                }
            }
        }
        return "MANIFEST-OK".to_string();
    }
    match result {
        Err(e) => return format!("MISMATCH expected ok, got REFUSED {e}"),
        Ok(ex) => {
            let want_body = entry["expected_body"].as_str().unwrap_or("");
            if ex.body != want_body {
                return format!(
                    "MISMATCH body: want {:?} ({} chars), got {:?} ({} chars)",
                    want_body,
                    want_body.chars().count(),
                    ex.body,
                    ex.body.chars().count()
                );
            }
            let want_sections = entry["expected_sections"].as_array().cloned().unwrap_or_default();
            if ex.sections.len() != want_sections.len() {
                return format!(
                    "MISMATCH sections: want {} got {}",
                    want_sections.len(),
                    ex.sections.len()
                );
            }
            for (got, want) in ex.sections.iter().zip(want_sections.iter()) {
                let w_heading = want["heading"].as_str().map(|s| s.to_string());
                let w_level = want["level"].as_u64().unwrap_or(0) as u8;
                let w_start = want["startChar"].as_u64().unwrap_or(0) as usize;
                let w_chars = want["chars"].as_u64().unwrap_or(0) as usize;
                if got.heading != w_heading
                    || got.level != w_level
                    || got.start_char != w_start
                    || got.chars != w_chars
                {
                    return format!(
                        "MISMATCH section: want {:?}/{}/{}/{} got {:?}/{}/{}/{}",
                        w_heading, w_level, w_start, w_chars, got.heading, got.level,
                        got.start_char, got.chars
                    );
                }
            }
            "MANIFEST-OK".to_string()
        }
    }
}

fn run_all() {
    let manifest = manifest();
    let fixtures = manifest["fixtures"].as_object().unwrap();
    println!("fixture | outcome | chars | lines | sections | elapsed_ms | manifest");
    let mut failures = 0;
    for name in fixtures.keys() {
        let bytes = fixture_bytes(name);
        let start = Instant::now();
        let result = extract(&bytes, true);
        let elapsed = start.elapsed();
        let check = check_manifest(name, &result, &bytes);
        if check != "MANIFEST-OK" {
            failures += 1;
        }
        let summary = match &result {
            Ok(ex) => format!(
                "OK | {} | {} | {} | {} | {check}",
                ex.body.chars().count(),
                ex.lines,
                ex.sections.len(),
                elapsed.as_millis()
            ),
            Err(e) => format!("REFUSED {e} | 0 | 0 | 0 | {} | {check}", elapsed.as_millis()),
        };
        println!("{name} | {summary}");
    }
    println!("MANIFEST failures: {failures}");
    if failures > 0 {
        std::process::exit(1);
    }
}

/// In-memory worst-case bombs (NOT committed fixtures): a minimal
/// docx whose word/document.xml expands to ~20 MB, then ~1 GiB.
/// Variant (a) declares the honest size (the 8 MiB single-entry
/// pre-check must refuse it); variant (b) lies about the declared
/// size (1000) so only the streaming capped reader can stop it.
/// Uncapped is run only on the 20 MB variant — never on 1 GiB.
fn run_worstbomb() {
    // Direct proof that the capped reader itself stops at the budget:
    // a synthetic 20 MB source through a 16 MiB cap must error after
    // exactly 16 MiB, without retaining the rest.
    {
        let source = vec![b'A'; 20 * (1 << 20)];
        let mut capped = CappedReader::new(&source[..], MAX_INFLATED_TOTAL);
        let mut sink = Vec::new();
        let err = capped.read_to_end(&mut sink).unwrap_err();
        println!(
            "capped-reader unit: read {} bytes then err {:?} (budget {MAX_INFLATED_TOTAL})",
            sink.len(),
            err.to_string()
        );
        assert_eq!(sink.len(), MAX_INFLATED_TOTAL);
        assert!(err.to_string().contains("expansion budget exceeded"));
    }
    for (label, inflated_mb, honest) in [
        ("worstbomb-20mb-honest", 20usize, true),
        ("worstbomb-20mb-lying", 20usize, false),
        ("worstbomb-1gib-lying", 1024usize, false),
    ] {
        let bytes = build_bomb_docx(inflated_mb, honest);
        println!(
            "{label}: raw={} bytes declared_size={}",
            bytes.len(),
            if honest { format!("{inflated_mb} MiB") } else { "1000 (lying)".to_string() }
        );
        let start = Instant::now();
        let capped = extract(&bytes, true);
        println!(
            "  capped: {:?} ELAPSED_MS={} PEAK_RSS_KB={}",
            capped.map(|ex| format!("OK {} chars", ex.body.chars().count())),
            start.elapsed().as_millis(),
            peak_rss_kb()
        );
        if label != "worstbomb-1gib-lying" {
            // Uncapped on the lying variant is still bounded by the
            // crate's own declared-size check; the honest variant is
            // the one that shows the unbounded read (20 MB inflated,
            // then the text cap trips).
            let start = Instant::now();
            let uncapped = extract(&bytes, false);
            println!(
                "  uncapped: {:?} ELAPSED_MS={} PEAK_RSS_KB={}",
                uncapped.map(|ex| format!("OK {} chars", ex.body.chars().count())),
                start.elapsed().as_millis(),
                peak_rss_kb()
            );
        }
    }
}

fn build_bomb_docx(inflated_mb: usize, honest: bool) -> Vec<u8> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    // Stream the 'A' run through the deflater: the generator never
    // holds the full inflated form (the 1 GiB variant would otherwise
    // allocate 1 GiB before the measurement starts).
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>").unwrap();
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(
            b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body><w:p><w:r><w:t>",
        )
        .unwrap();
        let chunk = vec![b'A'; 1 << 20];
        let mut remaining = inflated_mb * (1 << 20);
        while remaining > 0 {
            let n = remaining.min(chunk.len());
            zip.write_all(&chunk[..n]).unwrap();
            remaining -= n;
        }
        zip.write_all(b"</w:t></w:r></w:p></w:body></w:document>").unwrap();
        zip.finish().unwrap();
    }
    let mut bytes = buf.into_inner();
    if !honest {
        patch_declared_size(&mut bytes, "word/document.xml", 1000);
    }
    bytes
}

fn patch_declared_size(data: &mut [u8], name: &str, new_size: u32) {
    // Central directory.
    let eocd = data
        .windows(22)
        .rposition(|w| w[..4] == *b"PK\x05\x06")
        .expect("EOCD");
    let cd_offset = u32::from_le_bytes(data[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let cd_entries = u16::from_le_bytes(data[eocd + 10..eocd + 12].try_into().unwrap()) as usize;
    let mut pos = cd_offset;
    for _ in 0..cd_entries {
        assert_eq!(&data[pos..pos + 4], b"PK\x01\x02");
        let name_len = u16::from_le_bytes(data[pos + 28..pos + 30].try_into().unwrap()) as usize;
        let extra_len = u16::from_le_bytes(data[pos + 30..pos + 32].try_into().unwrap()) as usize;
        let comment_len =
            u16::from_le_bytes(data[pos + 32..pos + 34].try_into().unwrap()) as usize;
        let entry_name = std::str::from_utf8(&data[pos + 46..pos + 46 + name_len]).unwrap();
        if entry_name == name {
            data[pos + 24..pos + 28].copy_from_slice(&new_size.to_le_bytes());
        }
        pos += 46 + name_len + extra_len + comment_len;
    }
    // Local header.
    let mut lpos = 0usize;
    while let Some(found) = data[lpos..]
        .windows(4)
        .position(|w| w == b"PK\x03\x04")
        .map(|p| p + lpos)
    {
        let name_len =
            u16::from_le_bytes(data[found + 26..found + 28].try_into().unwrap()) as usize;
        let entry_name =
            std::str::from_utf8(&data[found + 30..found + 30 + name_len]).unwrap();
        if entry_name == name {
            data[found + 18..found + 22].copy_from_slice(&new_size.to_le_bytes());
            break;
        }
        lpos = found + 4;
    }
}

/// Honest-size 20 MB bomb only, in a fresh process, so the
/// uncapped peak RSS is not contaminated by the CappedReader unit
/// test's 37 MB allocation (VmHWM is process-wide).
fn run_worstbomb_honest() {
    let bytes = build_bomb_docx(20, true);
    println!("worstbomb-20mb-honest: raw={} bytes", bytes.len());
    let start = Instant::now();
    let capped = extract(&bytes, true);
    println!(
        "  capped: {:?} ELAPSED_MS={} PEAK_RSS_KB={}",
        capped.map(|ex| format!("OK {} chars", ex.body.chars().count())),
        start.elapsed().as_millis(),
        peak_rss_kb()
    );
    let start = Instant::now();
    let uncapped = extract(&bytes, false);
    println!(
        "  uncapped: {:?} ELAPSED_MS={} PEAK_RSS_KB={}",
        uncapped.map(|ex| format!("OK {} chars", ex.body.chars().count())),
        start.elapsed().as_millis(),
        peak_rss_kb()
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("all");
    match mode {
        "all" => run_all(),
        "dump" => {
            let name = args.get(2).expect("dump <fixture.docx>");
            let path = format!("{FIXTURES}/{name}");
            let bytes = std::fs::read(&path).expect("fixture readable");
            let ex = extract(&bytes, true).expect("fixture extracts");
            println!("{}", ex.body);
        }
        "worstbomb" => run_worstbomb(),
        "worstbomb-honest" => run_worstbomb_honest(),
        fixture => {
            let caps = args.get(2).map(|s| s.as_str()) != Some("uncapped");
            run_one(fixture, caps);
        }
    }
}
