# M13 PDF extractor module — handoff

Standalone bounded PDF text extractor (`crates/core/src/pdf.rs`), built on a
separate branch from the PDF spike tip `464678d7`. No production upload or
ingestion-job wiring — that is the integration step, gated on Antigravity's
accepted Windows ingestion commit (see §7). The module is the extraction
engine the integration will call; it is complete and tested on its own.

## 1. What the module is

Public API (one function, one result type):

```rust
pub fn extract_pdf_text(bytes: &[u8]) -> Result<ExtractedPdf, ExtractError>
pub struct ExtractedPdf {
    pub pages: Vec<ExtractedPage { number: u32, text: String }>,
    pub body: String,                        // pages joined by "\n\x0c\n"
    pub page_provenance: Vec<PdfPageProvenance { page, start_char, chars, hash }>,
}
```

Limits (constants in `pdf.rs`, matching the design):

| Limit | Value |
|---|---|
| Raw file (`MAX_PDF_BYTES`) | 1 MiB |
| Pages (`MAX_PDF_PAGES`) | 100 |
| Per-page text (`MAX_PDF_PAGE_CHARS`) | 8,000 chars |
| Total text (`MAX_PDF_CHARS`) | 32,000 chars |
| Cumulative decoded expansion (`MAX_DECODED_BYTES`) | 16 MiB |

`ExtractError` variants: `Parse(String)`, `Encrypted`, `Bomb`,
`Unsupported(String)`, `PageCap`, `TextCap`, `Empty`. Page hashes are
`content::hash` (SHA-256) of the stored page text; provenance ranges are
contiguous and cover the joined body exactly (verified by the test harness
for every imported fixture).

Flow, in order:

0. Raw-size check (1 MiB) before any parsing.
1. **Pre-load structural scan** over the raw bytes — the step that closes
   the load-time hole. `Document::load_mem` decompresses object streams
   (`ObjStm`) at load, before module code can run, so the module scans
   first: it finds every `N G obj` header, parses the following dict with
   a minimal bounded parser (8,192 B window, depth ≤ 8, `#XX` name
   escapes decoded so `/Obj#53tm` cannot evade), and if `/Type` is
   `ObjStm` or `XRef` it decodes that stream capped (the stream data is
   located by `stream` EOL → `endstream`; `/Length` is never trusted).
   A `<<` after a header that fails to parse is a **Parse refusal**
   (fail closed). The scan charges every decoded byte to the one
   cumulative `DecodeBudget`.
2. A single `Document::load_mem`, shared by the pre-scan and extraction
   (load-time decoding happens once, now proven bounded).
3. `/Encrypt` in the trailer → `Encrypted`. Empty-password documents
   arrive with `/Encrypt` already stripped by lopdf and are treated as
   unencrypted (spike finding, confirmed).
4. Page count: 0 → `Parse("document has no pages")` (never "empty");
   > 100 → `PageCap`.
5. Content pre-scan: every page's content streams plus Form XObjects,
   walked **recursively** (visited `HashSet<ObjectId>`, depth cap 32),
   decoded capped.
6. Per-page `output_doc_page` + `PlainTextOutput` (pdf-extract), checking
   the per-page and total char caps as it goes. Total 0 chars → `Empty`
   (scanned / no text layer).
7. Body and provenance are computed over the joined body.

One cumulative `DecodeBudget` (16 MiB) is charged for every decoded byte
across the structural scan, the content pre-scan, and every filter stage.
The extractor decodes exactly the streams the pre-scan already measured,
so its decode is bounded by the same budget — that is the proof the
assignment asked for: the 16 MiB limit applies during decoding, including
any decoding the text extractor performs after the pre-scan.

## 2. Supported and refused filter sets

`decode_stream` supports exactly the filters lopdf's
`decompressed_content` supports, in chains, applied in array order:
**FlateDecode, LZWDecode, ASCII85Decode**. Anything else is refused
with `Unsupported` (consistent with pdf-extract, which would fail the
same way). Refused explicitly:

- Any `/Predictor` > 1 (`/DecodeParms /Predictor`) — `Unsupported`.
- Any filter outside the three above (e.g. `DCTDecode`, `JPXDecode`,
  `JBIG2Decode`, `CCITTFaxDecode`, `RunLengthDecode`,
  `ASCIIHexDecode`) — `Unsupported`. Note lopdf's own
  `decompressed_content` does not implement ASCIIHexDecode or
  RunLengthDecode either, so refusing them matches the parser.
- Non-name `/Filter` entries — `Unsupported`.

Decoder notes (verified against lopdf 0.42.0 / weezl 0.1.12 sources):

- Flate: `ZlibDecoder` in 8 KiB chunks charged to the budget, with the
  raw-`DeflateDecoder` fallback on `input[2..]` when zlib fails with
  empty output (corrupt adler32), mirroring lopdf.
- LZW: weezl `IntoVec`, fed in 16-byte input slices, output length
  charged after each slice, stop on `decoder.has_ended()`, `InvalidCode`
  → `Parse` (lopdf tolerates it with partial output; the module is
  stricter, fail-closed). EarlyChange defaults to true (PDF default);
  `/DecodeParms /EarlyChange 0` selects the late-change decoder.
- ASCII85: own decoder; output ≤ 4/5 of input, so a pre-charge of
  `len*4/5 + 4` **is** the bound. Handles `z`, whitespace, `~>`
  terminator, short final group padded with `u`, overflow → `Parse`.

Known limitation (accepted, documented in the module): the per-page text
cap is enforced after `output_doc_page` extracts the page into a String.
A capped-writer `OutputDev` was considered and rejected as
over-engineering; the allocation is bounded by the 16 MiB decode budget.

## 3. Corrections to spike-era claims

**Leading newlines (the spike's "one trailing newline" claim is wrong).**
Direct code-point inspection (`pdf_spike normal-3page.pdf d debug`)
shows pdf-extract's per-page output **starts** with `U+000A U+000A`
(two leading newlines) and ends with `.` (no trailing newline). Page 1
is 114 chars = 112 manifest chars + 2 leading newlines. The spike's
`trim_end()` kept the leading pair, so its "after the worker's trim,
pages equal the manifest exactly" claim was inaccurate. The module trims
**both** ends (`text.trim()`) when forming the stored page text, and its
doc comment records the correction. The design doc §10 / spike-results
"one trailing newline per page" wording should be amended to match.

**LZW width-bump timing (fixture generator).** The generator's
`lzw_encode` originally bumped the code size when the post-increment
`next_code` reached `(1 << width) - 1` (assigned code 510 for width 9).
That desyncs weezl's TIFF decoder with `InvalidCode` at ~32k 'A's
(the first width bump). Root cause: the decoder derives each entry one
code **after** the encoder adds it, so the encoder's bump threshold is
one higher than the decoder's (`(1 << width) - 1` assigned, i.e.
post-increment `next_code == (1 << width)`). The generator is fixed and
commented; the fix was verified by decoding 40,000 'A's cleanly through
weezl. This is the correct early-change encoder behaviour, not a weezl
quirk — the threshold is defined by the decoder's table state and the
encoder's table leads by one.

## 4. Fixtures (22 total)

`tests/fixtures/pdf/` — the 12 original fixtures are byte-identical
(their MANIFEST hashes are unchanged; the generator is deterministic).
10 new adversarial fixtures, all in `MANIFEST.json` (0 hash mismatches):

| Fixture | Outcome (module) |
|---|---|
| `object-stream-ok.pdf` (PDF 1.5, /Pages tree as compressed object 2, xref stream) | Imported, 1 p, 47 chars |
| `object-stream-bomb.pdf` (unreferenced ObjStm expanding to ~20 MB) | `Bomb` |
| `xref-stream-ok.pdf` (PDF 1.5, xref stream, no compressed objects) | Imported, 1 p, 47 chars |
| `xref-stream-bomb.pdf` (xref stream decoded = 3M × 7 B = 21 MB) | `Bomb` |
| `chained-filter-ok.pdf` (`/Filter [/ASCII85Decode /FlateDecode]`) | Imported, 1 p, 50 chars |
| `chained-filter-bomb.pdf` (`/Filter [/FlateDecode /FlateDecode]`, 20 MB) | `Bomb` |
| `lzw-ok.pdf` (`/Filter /LZWDecode`; doubles as generator↔weezl round-trip check) | Imported, 1 p, 47 chars |
| `lzw-bomb.pdf` (LZW of 20 MB 'A' text) | `Bomb` |
| `nested-form-ok.pdf` (page → Form /Fm1 → Form /Fm2 with text) | Imported, 1 p, 50 chars |
| `nested-form-bomb.pdf` (same nesting, /Fm2 content is the 20 MB FlateDecode bomb) | `Bomb` |

Generator fixes in this round: (a) `object_stream_ok` reserved id 2 for
the compressed /Pages tree **before** adding any normal object — the
original version let the content stream take id 2 and then overwrote it
with the Pages tree, so the page had no contents (pdftotext: "Weird page
contents"); (b) LZW width-bump rule (see §3). `lzw-ok` / `lzw-bomb`
were regenerated with the fixed encoder; the other 8 new fixtures are
unchanged from their first generation.

## 5. Test results

`node scripts/cargo.mjs test --locked -p anansi-core --lib pdf::` —
**16 passed, 0 failed** (anansi-core lib 200 → 216 tests with the module):

- `normal_three_pages`, `unicode_cmap_pages` — exact page texts +
  provenance contiguity/coverage/hash.
- `scanned_and_empty_are_refused`, `malformed_are_refused`,
  `encrypted_is_refused`, `caps_are_refused` (incl. oversized-raw),
  `decompression_bomb_is_refused`, `raw_cap_is_checked`,
  `unsupported_filter_is_refused` (DCTDecode → `Unsupported`).
- `object_streams`, `xref_streams`, `chained_filters`, `lzw_streams`,
  `nested_forms` — ok fixture imports + bomb fixture refuses `Bomb`.
- `dict_parser_decodes_name_escapes` (`/Obj#53tm` → ObjStm),
  `ascii85_known_vector` (independent Python-a85encode vector).

All refusal/bound paths pass; the 8 initial failures were the
leading-newline trim (§3), a truncated ASCII85 test vector (replaced
with a complete independent vector), the LZW width-bump desync (§3),
and the object-stream id collision (§4) — all fixed in the fixture or
the module, no assertion was weakened.

## 6. Bomb measurements (per-process, debug build, module)

Measured with a throwaway per-process runner calling
`anansi_core::pdf::extract_pdf_text` (deleted before commit). Peak RSS
is Linux VmHWM.

| Fixture | Module outcome | Elapsed | Peak RSS |
|---|---|---|---|
| decompression-bomb (20 MB) | `Bomb` | 175 ms | 22,544 KB |
| object-stream-bomb (~20 MB) | `Bomb` | 172 ms | 21,952 KB |
| xref-stream-bomb (21 MB) | `Bomb` | 178 ms | 21,860 KB |
| chained-filter-bomb (20 MB) | `Bomb` | 175 ms | 22,208 KB |
| lzw-bomb (20 MB) | `Bomb` | 125 ms | 22,164 KB |
| nested-form-bomb (20 MB) | `Bomb` | 174 ms | 22,552 KB |

In-cap fixtures through the module: normal-3page 12 ms / 6,772 KB,
unicode-cmap 6 ms / 6,980 KB, lzw-ok 5 ms / 6,640 KB.

Comparison (spike, decompression bomb): pdf-extract alone (path A)
**45,463 ms / 65,448 KB** (fully decodes 20,000,002 chars); spike
pre-scan + pdf-extract (path B) 187 ms / 7,168 KB; spike production
flow (path D) 169 ms / 7,064 KB. Worst-case 1 GiB bomb (synthetic,
/tmp-only): path B 163 ms / 11,656 KB; path D 182 ms / 11,496 KB.

**Why the module peaks at ~22 MB vs the spike's ~7 MB:** the module
materializes decoded output into a `Vec` (up to the 16 MiB budget)
before the budget trips, so its peak is the budget + ~6 MB baseline —
bounded by construction, and the bound is what the measurement proves.
The spike prototype streamed into a discard buffer (reading one byte
past the remaining budget to prove the excess) and never materialized
the expansion. A discard sink for the content pre-scan (which drops the
decoded bytes anyway) would bring the module's peak to baseline; that
is a recorded refinement, not a correctness gap — the allocation never
exceeds the budget. The structural scan must retain decoded ObjStm /
xref bytes to parse them, so only the content pre-scan is affected.

## 7. Integration contract (when Antigravity's accepted commit arrives)

The Windows ingestion rerun is still pending at delivery. When
Antigravity shares the accepted integration commit:

1. Use that commit as the baseline (rebase / cherry-pick this module
   onto it). If the rerun finds a regression, repair that first.
2. Wire into `execute_ingestion` per design §3: per-kind raw limit in
   `snapshots::limited` (PDF 1 MiB, TXT/MD 64 KiB); replace the
   `String::from_utf8` step with `extract_pdf_text` for `kind == "pdf"`;
   `DocumentProvenance.pages` (contract regen — the field is
   optional/defaulted so TXT/MD provenance stays compatible);
   `media_type_for("pdf")` → `application/pdf`.
3. Documents view: plain-text reader with page navigation (never the
   Markdown renderer); search shows the page of first match (computed
   client-side from provenance); Ask cites `Page N (line M)` via
   `section_heading = "Page N"` (no contract change, no UI change —
   the Ask citation renderer already prints it).
4. Backup/restore unchanged (provenance rides with the memory body).
5. Run the end-to-end checks: documents e2e (PDF variants), ingestion
   e2e, full battery, docs validator.

## 8. Advisory coverage (unchanged)

`cargo-audit` on the spike branch (2026-11-05): 1 vulnerability +
7 warnings. The vulnerability is **ctutils 0.4.3, RUSTSEC-2026-0385**
(non-constant-time mask generation) — **pre-existing** in the
`dd0423ced` baseline lock, pulled by argon2 + chacha20poly1305, not by
the PDF crates; out of this slice's scope, flagged for the owner. The
one new warning is ttf-parser 0.25.1 unmaintained (RUSTSEC-2026-0192)
via lopdf ← pdf-extract — warning only. lopdf is pinned `=0.42.0`
(RUSTSEC-2026-0187 floor). Dependency changes in this round:
`lopdf` + `flate2` moved from dev-dependencies to dependencies,
`weezl = "=0.1.12"` added (already in the graph — **no new crates**).

## 9. Proposed global status updates (for Antigravity; not applied)

- None to AN-025 / SPEC_COVERAGE §12: the module is standalone, no
  production wiring exists yet, so the ingestion milestone state does
  not change.
- Design doc §10 / spike-results "one trailing newline per page"
  wording should be corrected to "two leading newlines per page,
  trimmed at both ends" (§3 above).
- The LZW early-change encoder rule (§3) is a fixture-generator
  correction, not a design change; no design amendment is needed beyond
  the newline correction.

## 10. Files in this delivery

Branch `muse/m13-pdf-extractor` from spike tip `464678d7` (single
commit): `crates/core/src/pdf.rs` (new), `crates/core/src/lib.rs`
(`pub mod pdf;`), `crates/core/Cargo.toml` + `Cargo.lock` (lopdf/flate2
to dependencies, weezl added), `tests/fixtures/pdf/generate-pdf-fixtures.py`
(LZW bump rule + object-stream id reservation), 10 new fixtures +
`MANIFEST.json`, this handoff doc. No TS, contract, or UI changes.
No Windows run. No ingestion wiring.
