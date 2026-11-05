//! PDF parser spike harness (M13 Slice 2, spike branch only).
//!
//! Runs the 12 committed fixtures through three extraction paths and
//! prints a machine-readable report. Not production code: it exists to
//! produce the evidence for `docs/handoffs/m13-pdf-spike-results.md`.
//!
//! Paths:
//!   a — pdf-extract alone (`extract_text_from_mem_by_pages`).
//!   b — bounded pre-scan (lopdf + capped FlateDecode, 16 MiB total
//!       decoded budget) then pdf-extract. The pre-scan is the
//!       prototype of the production decode bound.
//!   c — lopdf per-page incremental extraction (`extract_text(&[n])`
//!       one page at a time, accumulating chars, stopping early when
//!       the running total exceeds the 32,000-char document cap).
//!
//! Usage:
//!   cargo run -p anansi-core --example pdf_spike -- <fixture> <a|b|c>
//!   cargo run -p anansi-core --example pdf_spike -- all
//!   cargo run -p anansi-core --example pdf_spike -- worstbomb <b>
//! Peak RSS (Linux VmHWM) and elapsed time are printed per run; run
//! one fixture per process for a meaningful peak.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/pdf");
const DECODE_BUDGET: usize = 16 * 1024 * 1024;
const MAX_PAGES: usize = 100;
const MAX_PAGE_CHARS: usize = 8_000;
const MAX_DOC_CHARS: usize = 32_000;

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

/// Page count without extracting any text: the page-cap check precedes
/// all extraction work.
fn page_count(bytes: &[u8]) -> Result<usize, String> {
    let doc = lopdf::Document::load_mem(bytes).map_err(|e| format!("load: {e}"))?;
    Ok(doc.get_pages().len())
}

/// Bounded pre-scan prototype: decode every page content stream (and
/// every Form XObject stream reachable from a page) through a capped
/// reader. Returns the total decoded bytes when everything fits the
/// budget, or an error naming the reason. FlateDecode is the only
/// expanding filter handled here; anything else in a content stream is
/// refused as unsupported (recorded as a spike gap, not silently
/// passed to the uncapped decoder).
fn prescan(bytes: &[u8]) -> Result<usize, String> {
    let doc = lopdf::Document::load_mem(bytes).map_err(|e| format!("load: {e}"))?;
    if doc.trailer.get(b"Encrypt").is_ok() {
        return Err("encrypted: /Encrypt present".to_string());
    }
    let mut total: usize = 0;
    let mut seen = std::collections::HashSet::new();
    for (_n, page_id) in doc.get_pages() {
        for content_id in doc.get_page_contents(page_id) {
            total = decode_stream_capped(&doc, content_id, total, &mut seen)?;
        }
        // Form XObjects reachable from this page's resources.
        let page = doc.get_object(page_id).map_err(|e| format!("page: {e}"))?;
        if let Ok(resources) = page.as_dict().and_then(|d| d.get(b"Resources")) {
            if let Ok(xobjects) = resources.as_dict().and_then(|d| d.get(b"XObject")) {
                if let Ok(dict) = xobjects.as_dict() {
                    for (_name, value) in dict.iter() {
                        if let Ok(obj) = doc.get_object(value.as_reference().unwrap_or((0, 0))) {
                            if let Ok(stream) = obj.as_stream() {
                                if stream
                                    .dict
                                    .get(b"Subtype")
                                    .and_then(|s| s.as_name())
                                    .is_ok_and(|n| n == b"Form")
                                {
                                    total = decode_stream_capped(
                                        &doc,
                                        value.as_reference().unwrap_or((0, 0)),
                                        total,
                                        &mut seen,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(total)
}

fn decode_stream_capped(
    doc: &lopdf::Document,
    id: lopdf::ObjectId,
    mut total: usize,
    seen: &mut std::collections::HashSet<lopdf::ObjectId>,
) -> Result<usize, String> {
    if !seen.insert(id) {
        return Ok(total);
    }
    let stream = doc
        .get_object(id)
        .map_err(|e| format!("stream {id:?}: {e}"))?
        .as_stream()
        .map_err(|e| format!("stream {id:?}: {e}"))?;
    let filters = stream_filters(stream)?;
    if filters.len() > 1 {
        // Chained filters are refused in the spike prototype: the
        // capped decode below discards decoded bytes (it counts them),
        // so a second filter would have nothing to decode. Recorded
        // as a spike gap for integration (rare in text content
        // streams; pdf-extract handles chains).
        return Err(format!("filter chain not supported in spike pre-scan: {filters:?}"));
    }
    let mut data: Vec<u8> = stream.content.clone();
    for filter in filters {
        match filter.as_str() {
            "FlateDecode" | "Fl" => {
                let remaining = DECODE_BUDGET.checked_sub(total).ok_or("decode budget exceeded")?;
                let mut decoder = flate2::read::ZlibDecoder::new(&data[..]);
                let mut counted = 0usize;
                let mut buf = [0u8; 8192];
                loop {
                    // Take(remaining+1): reading the (remaining+1)-th byte
                    // proves the budget is exceeded without allocating it.
                    let mut limited = decoder.by_ref().take((remaining - counted + 1) as u64);
                    let n = limited.read(&mut buf).map_err(|e| format!("flate: {e}"))?;
                    if n == 0 {
                        break;
                    }
                    counted += n;
                    if counted > remaining {
                        return Err(format!(
                            "decode budget exceeded: stream {id:?} expands past {DECODE_BUDGET} bytes"
                        ));
                    }
                }
                total += counted;
                data = vec![0u8; 0]; // decoded bytes are counted, not kept
            }
            "ASCIIHexDecode" | "AHx" | "ASCII85Decode" | "A85" | "RunLengthDecode" | "RL" => {
                // Bounded expansion factors (2x, 5:4, 128x): refuse
                // conservatively when even the worst case breaks the
                // budget, else count the raw length as a lower bound.
                let factor: usize = match filter.as_str() {
                    "RunLengthDecode" | "RL" => 128,
                    "ASCIIHexDecode" | "AHx" => 2,
                    _ => 2,
                };
                let worst = data.len().saturating_mul(factor);
                if total.saturating_add(worst) > DECODE_BUDGET {
                    return Err(format!(
                        "decode budget exceeded: {filter} stream {id:?} could expand past {DECODE_BUDGET} bytes"
                    ));
                }
                total += data.len();
                data = vec![0u8; 0];
            }
            other => {
                return Err(format!("unsupported content-stream filter: {other}"));
            }
        }
    }
    // No filter: raw bytes are the decoded size.
    if stream_filters(stream)?.is_empty() {
        if total.saturating_add(stream.content.len()) > DECODE_BUDGET {
            return Err("decode budget exceeded: unfiltered stream".to_string());
        }
        total += stream.content.len();
    }
    Ok(total)
}

fn stream_filters(stream: &lopdf::Stream) -> Result<Vec<String>, String> {
    match stream.dict.get(b"Filter") {
        Ok(obj) => {
            if let Ok(name) = obj.as_name() {
                Ok(vec![String::from_utf8_lossy(name).into_owned()])
            } else if let Ok(arr) = obj.as_array() {
                let mut out = Vec::new();
                for item in arr {
                    out.push(String::from_utf8_lossy(item.as_name().unwrap_or(b"?")).into_owned());
                }
                Ok(out)
            } else {
                Ok(vec![])
            }
        }
        Err(_) => Ok(vec![]),
    }
}

/// Path A: pdf-extract alone.
fn path_a(bytes: &[u8]) -> Result<Vec<String>, String> {
    pdf_extract::extract_text_from_mem_by_pages(bytes).map_err(|e| format!("{e}"))
}

/// Path D — the production-flow prototype: bounded pre-scan first
/// (the decode bound), then per-page extraction with pdf-extract's
/// quality via the public `output_doc_page` + `PlainTextOutput`
/// (exactly what `extract_text_by_page` does internally), stopping
/// early when a cap trips. The /Encrypt and 0-page checks live in the
/// pre-scan / page-count steps: `output_doc_page` itself only logs
/// for encrypted docs and would silently yield empty pages.
fn path_d(bytes: &[u8]) -> Result<(Vec<String>, usize, usize), String> {
    let _decoded = prescan(bytes)?;
    let doc = pdf_extract::Document::load_mem(bytes).map_err(|e| format!("load: {e}"))?;
    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err("parse error: document has no pages".to_string());
    }
    if pages.len() > MAX_PAGES {
        return Err(format!("page cap: {} pages", pages.len()));
    }
    let mut out: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut extracted = 0usize;
    for page_num in pages.keys() {
        let mut text = String::new();
        {
            let mut output = pdf_extract::PlainTextOutput::new(&mut text);
            pdf_extract::output_doc_page(&doc, &mut output, *page_num)
                .map_err(|e| format!("page {page_num}: {e}"))?;
        }
        extracted += 1;
        let chars = text.chars().count();
        if chars > MAX_PAGE_CHARS {
            return Err(format!("page cap: page {page_num} has {chars} chars"));
        }
        total += chars;
        if total > MAX_DOC_CHARS {
            return Err(format!("text cap: exceeded at page {page_num} ({total} chars)"));
        }
        out.push(text);
    }
    Ok((out, extracted, total))
}

/// Path C: lopdf per-page incremental extraction with early stop.
fn path_c(bytes: &[u8]) -> Result<(Vec<String>, usize, usize), String> {
    let doc = lopdf::Document::load_mem(bytes).map_err(|e| format!("load: {e}"))?;
    let pages = doc.get_pages();
    if pages.len() > MAX_PAGES {
        return Err(format!("page cap: {} pages", pages.len()));
    }
    let mut out: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut extracted = 0usize;
    for (number, _id) in pages.iter() {
        let text = doc.extract_text(&[*number]).map_err(|e| format!("page {number}: {e}"))?;
        extracted += 1;
        let chars = text.chars().count();
        if chars > MAX_PAGE_CHARS {
            return Err(format!("page cap: page {number} has {chars} chars"));
        }
        total += chars;
        if total > MAX_DOC_CHARS {
            return Err(format!("text cap: exceeded at page {number} ({total} chars)"));
        }
        out.push(text);
    }
    Ok((out, extracted, total))
}

/// Build the stored body + per-page provenance for the design §3.3 and
/// verify the ranges cover the body exactly.
fn provenance(pages: &[String]) -> (String, Vec<(u32, u64, u64, String)>) {
    let mut body = String::new();
    let mut rows = Vec::new();
    for (i, text) in pages.iter().enumerate() {
        let page_text = text.trim_end().to_string();
        let start = body.chars().count() as u64;
        let chars = page_text.chars().count() as u64;
        let hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            page_text.hash(&mut h);
            format!("{:016x}", h.finish())
        };
        rows.push((i as u32 + 1, start, chars, hash));
        body.push_str(&page_text);
        if i + 1 < pages.len() {
            body.push('\n');
            body.push('\u{000c}');
            body.push('\n');
        }
    }
    (body, rows)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("all");
    if mode == "all" {
        run_all();
        return;
    }
    if mode == "worstbomb" {
        run_worstbomb(args.get(2).map(|s| s.as_str()).unwrap_or("b"));
        return;
    }
    let fixture = mode;
    let path = args.get(2).map(|s| s.as_str()).unwrap_or("a");
    run_one(fixture, path);
}

fn run_all() {
    let manifest = manifest();
    let fixtures = manifest["fixtures"].as_object().unwrap();
    println!("fixture | path A (pdf-extract alone) | path B (pre-scan + pdf-extract) | path C (lopdf per-page) | path D (pre-scan + per-page pdf-extract)");
    for name in fixtures.keys() {
        let bytes = fixture_bytes(name);
        let a = summarize_a(&bytes);
        let b = summarize_b(&bytes);
        let c = summarize_c(&bytes);
        let d = summarize_d(&bytes);
        println!("{name} | {a} | {b} | {c} | {d}");
    }
}

fn summarize_a(bytes: &[u8]) -> String {
    match path_a(bytes) {
        Ok(pages) => {
            let total: usize = pages.iter().map(|p| p.chars().count()).sum();
            format!("ok {} pages, {} chars", pages.len(), total)
        }
        Err(e) => format!("ERR {e}"),
    }
}

fn summarize_b(bytes: &[u8]) -> String {
    match prescan(bytes) {
        Ok(decoded) => match path_a(bytes) {
            Ok(pages) => {
                let total: usize = pages.iter().map(|p| p.chars().count()).sum();
                format!("ok prescan {} decoded bytes; {} pages, {} chars", decoded, pages.len(), total)
            }
            Err(e) => format!("prescan ok; extract ERR {e}"),
        },
        Err(e) => format!("prescan REFUSED {e}"),
    }
}

fn summarize_c(bytes: &[u8]) -> String {
    match path_c(bytes) {
        Ok((pages, extracted, total)) => {
            format!("ok {} pages extracted (of {}), {} chars", extracted, pages.len(), total)
        }
        Err(e) => format!("ERR {e}"),
    }
}

fn summarize_d(bytes: &[u8]) -> String {
    match path_d(bytes) {
        Ok((pages, extracted, total)) => {
            format!("ok {} pages extracted (of {}), {} chars", extracted, pages.len(), total)
        }
        Err(e) => format!("ERR {e}"),
    }
}

fn run_one(fixture: &str, path: &str) {
    let bytes = fixture_bytes(fixture);
    let start = Instant::now();
    let result = match path {
        "a" => match path_a(&bytes) {
            Ok(pages) => {
                let total: usize = pages.iter().map(|p| p.chars().count()).sum();
                let mut s = format!("OK pages={} chars={}", pages.len(), total);
                for (i, p) in pages.iter().enumerate() {
                    s.push_str(&format!("\n  page {}: {} chars {:?}", i + 1, p.chars().count(), first_line(p)));
                }
                if std::env::args().any(|a| a == "debug") {
                    debug_pages("A", &pages);
                }
                s
            }
            Err(e) => format!("ERR {e}"),
        },
        "b" => match prescan(&bytes) {
            Ok(decoded) => match path_a(&bytes) {
                Ok(pages) => {
                    let total: usize = pages.iter().map(|p| p.chars().count()).sum();
                    format!("OK prescan_decoded={} pages={} chars={}", decoded, pages.len(), total)
                }
                Err(e) => format!("ERR prescan ok, extract failed: {e}"),
            },
            Err(e) => format!("REFUSED {e}"),
        },
        "c" => match path_c(&bytes) {
            Ok((pages, extracted, total)) => {
                let mut s = format!("OK extracted_pages={} chars={}", extracted, total);
                for (i, p) in pages.iter().enumerate() {
                    s.push_str(&format!("\n  page {}: {} chars {:?}", i + 1, p.chars().count(), first_line(p)));
                }
                if std::env::args().any(|a| a == "debug") {
                    debug_pages("C", &pages);
                }
                s
            }
            Err(e) => format!("ERR {e}"),
        },
        "d" => match path_d(&bytes) {
            Ok((pages, extracted, total)) => {
                let mut s = format!("OK extracted_pages={} chars={}", extracted, total);
                for (i, p) in pages.iter().enumerate() {
                    s.push_str(&format!("\n  page {}: {} chars {:?}", i + 1, p.chars().count(), first_line(p)));
                }
                if std::env::args().any(|a| a == "debug") {
                    debug_pages("D", &pages);
                }
                s
            }
            Err(e) => format!("ERR {e}"),
        },
        _ => format!("unknown path {path}"),
    };
    let elapsed = start.elapsed();
    println!("fixture={fixture} path={path}\n{result}\nELAPSED_MS={} PEAK_RSS_KB={}", elapsed.as_millis(), peak_rss_kb());
    // Page provenance check for the normal fixture (design §3.3).
    if fixture == "normal-3page.pdf" {
        if let Ok(pages) = path_a(&bytes) {
            let (body, rows) = provenance(&pages);
            println!("PROVENANCE body_chars={} pages={:?}", body.chars().count(), rows);
            // Verify ranges: contiguous, in order, covering the body
            // minus the 2-char separators.
            let mut expect = 0u64;
            for (page, start, chars, _hash) in &rows {
                assert_eq!(*start, expect, "page {page} start");
                expect = start + chars + 3; // separator is "\n\x0c\n" (3 chars)
            }
            assert_eq!(expect - 3, body.chars().count() as u64, "body coverage");
            println!("PROVENANCE ranges verified: contiguous, in order, covering body");
        }
    }
}

fn first_line(text: &str) -> String {
    let line: String = text.lines().next().unwrap_or("").chars().take(60).collect();
    line
}

/// Debug: print a page's exact code points (distinguishes U+03B1 from
/// U+0391, trailing newlines, and missing line breaks).
fn debug_pages(label: &str, pages: &[String]) {
    for (i, p) in pages.iter().enumerate() {
        let points: Vec<String> = p.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        println!("  {label} page {}: {:?} ({} chars)", i + 1, points.join(" "), p.chars().count());
    }
}

/// Synthetic worst-case bomb (NOT a committed fixture): a 1-page PDF
/// whose single FlateDecode content stream expands to ~1 GiB of 'A'
/// text. Built in-memory with streaming compression so the generator
/// never holds the full decoded form. Path A is deliberately never run
/// on it (it would allocate ~1 GiB); path B must stop it at the
/// 16 MiB budget.
fn run_worstbomb(path: &str) {
    if path != "b" && path != "d" {
        println!("worstbomb path={path}: SKIPPED — path A/C are not run on a ~1 GiB expansion (they would allocate it); only the pre-scan-first paths are measured");
        return;
    }
    let mut compressed = Vec::new();
    {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use std::io::Write;
        let mut enc = ZlibEncoder::new(&mut compressed, Compression::default());
        let chunk = vec![b'A'; 1 << 20];
        let mut remaining = 1usize << 30;
        while remaining > 0 {
            let n = remaining.min(chunk.len());
            enc.write_all(&chunk[..n]).unwrap();
            remaining -= n;
        }
        enc.finish().unwrap();
    }
    println!("worstbomb: raw stream {} bytes (compressed 1 GiB of 'A')", compressed.len());
    // Build the minimal PDF by hand: the pre-scan only needs a valid
    // document with one page whose /Contents is the bomb stream.
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let stream_dict = format!("<< /Length {} /Filter /FlateDecode >>", compressed.len());
    let mut offsets: BTreeMap<u32, usize> = BTreeMap::new();
    let mut emit = |out: &mut Vec<u8>, offsets: &mut BTreeMap<u32, usize>, id: u32, body: Vec<u8>| {
        offsets.insert(id, out.len());
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(&body);
        out.extend_from_slice(b"\nendobj\n");
    };
    emit(&mut out, &mut offsets, 3, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    let stream_body = {
        let mut b = stream_dict.into_bytes();
        b.extend_from_slice(b"\nstream\n");
        b.extend_from_slice(&compressed);
        b.extend_from_slice(b"\nendstream");
        b
    };
    emit(&mut out, &mut offsets, 4, stream_body);
    emit(&mut out, &mut offsets, 5, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents 4 0 R >>".to_vec());
    emit(&mut out, &mut offsets, 2, b"<< /Type /Pages /Kids [5 0 R] /Count 1 >>".to_vec());
    emit(&mut out, &mut offsets, 1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let xref_pos = out.len();
    let count = 6;
    out.extend_from_slice(format!("xref\n0 {count}\n").as_bytes());
    for id in 0..count {
        if let Some(off) = offsets.get(&id) {
            out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        } else {
            out.extend_from_slice(b"0000000000 65535 f \n");
        }
    }
    out.extend_from_slice(format!("trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n").as_bytes());
    let start = Instant::now();
    let result = if path == "d" {
        match path_d(&out) {
            Ok((pages, extracted, total)) => format!("Ok({extracted} pages, {total} chars of {} pages)", pages.len()),
            Err(e) => format!("Err({e:?})"),
        }
    } else {
        format!("{:?}", prescan(&out))
    };
    let elapsed = start.elapsed();
    println!("worstbomb path={path} result={result}\nELAPSED_MS={} PEAK_RSS_KB={}", elapsed.as_millis(), peak_rss_kb());
}
