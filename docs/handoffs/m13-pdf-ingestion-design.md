# M13 Document Ingestion Slice 2 — Design: PDF text extraction (text layer only)

Branch: `muse/m13-pdf-ingestion-design`, from the verified Linux repair
`dd0423ced7761386db1ee2cc3f0bc19339f3b19d`. Design only — no
implementation in this branch. Implementation starts only once
Antigravity confirms the Windows ingestion tests pass and shares the
accepted integration commit; that commit becomes the implementation
baseline (§8).

## 0. What this slice is — and what it is not

One complete PDF workflow on the Slice 1 shape: select a PDF → review
and approve import → bounded chunked upload → persistent job → restart/
resume → open and search the extracted text → Ask citations with page
numbers → backup/fresh restore. Text-layer PDFs only: a PDF whose pages
carry embedded text. Scanned/image-only PDFs are refused honestly
("no text layer") — **OCR is a separate slice**, not this one. No
password entry: encrypted PDFs are refused honestly. No cloud, no
model calls, no subprocess, no native library, no network at runtime.
M13 is not declared complete by this slice; DOCX follows PDF.

## 1. Inventory — what already exists (verified in the tree)

- `crates/core/src/ingestion.rs` — `enqueue_ingestion` (begin: validate,
  Immediate-tx insert as Paused, staged file created after commit, sweep
  inside the tx), `ingestion_operation` (`ingest-part`: offset-exact,
  sequential, hash-verified at finish, wakes the worker),
  `execute_ingestion` (worker: staged file is ground truth — length +
  SHA-256 re-verified — then decode, validate, publish through
  `memory_write` with `JobPublishGuard`). **The extraction plug-in
  point is the `String::from_utf8` step in `execute_ingestion`**: for
  kind `pdf` the bytes go to the extractor instead of the UTF-8 decoder.
  Everything before it (caps, hash gate, staging) and after it (source
  validation, dedup guard, publish, completion log) is reused unchanged.
- `crates/core/src/document.rs` — limits (`MAX_DOCUMENT_BYTES` 65,536,
  `MAX_DOCUMENT_CHARS` 8,000, `MAX_LABEL_CHARS` 255,
  `MAX_UPLOAD_PART_BYTES` 65,536, `MAX_ACTIVE_UPLOADS` 8),
  `DocumentSource::validate`, `DocumentProvenance` (file name, media
  type, raw/text hashes, byte/char counts, BOM), `media_type_for`.
- `crates/core/src/memory.rs` — `memory_write` re-validates provenance
  against the body (hashes recomputed, never trusted); the dedup guard
  matches (title, text hash) inside the publish transaction.
- `crates/core/src/context.rs` — `chunk_markdown_sections` splits a body
  into `ContextChunk`s (≤ `MAX_CHUNK_CHARS` 1,000) carrying
  `section_heading: Option<String>` and `line_number: Option<u32>`;
  citations copy those fields verbatim, and `AskAnansi.tsx` already
  renders `{kind} · {sectionHeading ?? 'General'}{lineNumber ?
  ' (line N)' : ''}`. **Page-level citations need no contract or UI
  change** — the chunker gains a page-aware path (§3.4).
- Search: full-body FTS over `memory_body_fts` (`fullBody: true`);
  `Documents.tsx` lists `kind='document'`, shows provenance in
  `<details>`, renders Markdown inertly or `<pre>` source. Backup/
  restore already preserves document memories with provenance and
  re-validates on restore.
- Ask budgets: default evidence budget 4,000 chars, max
  `MAX_ALLOWED_BUDGET_CHARS` 32,000; memory body cap 256 KiB
  (`content::MAX_BODY`); job lease 120 s (`LEASE_SECONDS`).
- UI `documentInput.ts` (`decodeDocumentBytes`) refuses non-`.txt`/`.md`
  before reading. For PDFs the client can only pre-refuse by extension
  and size; all content validation is server-side (worker).

## 2. Parser selection

### 2.1 Candidates (evaluated)

| Candidate | Form | Verdict |
|---|---|---|
| **pdf-extract 0.12.x** (over lopdf) | Pure Rust, MIT/Apache-2.0, active (latest 0.12.1, 2026-09-16) | **Selected** — see §2.2 |
| lopdf 0.42+ alone | Pure Rust, low-level | Fallback only: the extractor (content-stream operators, font encodings, ToUnicode CMaps) would have to be hand-written on top of it; that is more code, more encoding risk, and no gain over pdf-extract, which already wraps it |
| pdf-text-extract / pdf_parser | Pure Rust, newer | Rejected: no xref streams (PDF 1.5+) — most real-world PDFs are 1.5+ |
| folio_pdf | Pure Rust, comprehensive | Rejected for this slice: newer, smaller extraction track record than the lopdf lineage; revisit if the spike fails |
| MuPDF / PDFium bindings, poppler CLI | Native C library / subprocess | Rejected: native libraries break Windows/Linux build parity (vendored binaries, DLLs); a subprocess is forbidden by the Slice 1 rules (no shell, no external process execution from the app); both would need downloads the owner did not authorize |

### 2.2 Decision: pdf-extract 0.12.x, `extract_text_from_mem_by_pages`

- **API fit.** The worker holds verified staged bytes in memory, and
  page-level provenance needs per-page text. pdf-extract provides
  exactly that: `extract_text_from_mem(&bytes) -> Result<String, _>`
  and `extract_text_from_mem_by_pages(&bytes) -> Result<Vec<String>, _>`
  (one string per page, in page order), plus `_encrypted` password
  variants that this slice will **not** call (no password entry).
  All encodings specified in PDF 2.0 are supported (PDFDocEncoding,
  UTF-16BE, ToUnicode CMaps) — the fixture `unicode-cmap.pdf` proves
  the CMap path (§6).
- **Windows/Linux compatibility.** Pure Rust, no `build.rs` system
  dependency, no subprocess, no temp files, no network. The
  implementation spike must confirm with `cargo tree` that the resolved
  graph contains no C library (notably the `flate2` backend must be
  the Rust backend, `miniz_oxide`, not a system zlib) and that the
  crate builds on this toolchain (rustc 1.99.1 here; lopdf requires
  Rust 1.85+). The crate is added as a normal `crates.io` dependency
  of `anansi-core`; `Cargo.lock` is updated by the spike and committed
  with the implementation.
- **Determinism.** Single-threaded extraction, page order ascending,
  byte-identical output for byte-identical input (required: the dedup
  guard hashes the stored body, so nondeterministic extraction would
  manufacture duplicate records).
- **Security (disclosed).** RUSTSEC-2026-0187: stack overflow in lopdf
  via deeply nested PDF objects (CVSS 7.5, published 2026-06-21),
  fixed in lopdf 0.42.0. The design therefore pins **pdf-extract
  =0.12.1** (resolving lopdf =0.42.0, a single version — the direct
  lopdf dependency is pinned to the same version so the graph holds
  exactly one lopdf) and requires the spike to record the resolved
  lopdf version from `Cargo.lock`. Residual risk, stated honestly: a
  stack overflow aborts the process and cannot be caught in Rust;
  the version floor is the primary mitigation, and extraction runs
  in the background worker (never on an API request thread), so a
  hostile PDF cannot stall a user-facing request. A
  spawned-thread-with-guard hardening is follow-up work, not claimed
  here.
- **Spike result (2026-11-05): the bounded pre-scan is mandatory.**
  pdf-extract alone does NOT stop the decompression bomb during
  decoding — it fully decodes the 20 MB fixture (~45 s, ~64 MB peak)
  and would take ~35 minutes / ~3 GB on a 1 GiB expansion. The
  pre-scan (lopdf + capped FlateDecode, 16 MiB total decoded
  budget) stops the fixture bomb in ~190 ms / ~7 MB and a 1 GiB
  worst-case expansion in ~160–180 ms / ~11 MB, and must run before
  pdf-extract on every PDF (§3.2, path D). lopdf is therefore a
  **production** dependency (structure + pre-scan), not just
  pdf-extract's transitive dep.
- **Audit (2026-11-05, cargo-audit 0.22.2).** 1 vulnerability,
  7 warnings. The vulnerability (`ctutils` 0.4.3,
  RUSTSEC-2026-0385) is **pre-existing** — already in the baseline
  lock via `argon2`/`chacha20poly1305`, not introduced by this
  slice; flagged for the owner, out of scope. Of the warnings, 1 is
  new: `ttf-parser` 0.25.1 is unmaintained (RUSTSEC-2026-0192),
  pulled in by lopdf ← pdf-extract for font parsing — a warning,
  not a vulnerability; if a future advisory appears against it, the
  parser choice is revisited. The other 6 warnings are
  pre-existing. Lock grows 551 → 580 crates (+29, all pure Rust).

### 2.3 Spike gate (before integration)

Before the extractor is wired into `execute_ingestion`, a scratch
spike (not committed as production code) must prove, against the
committed fixtures in `tests/fixtures/pdf/`:

1. `extract_text_from_mem_by_pages` returns the manifest's expected
   page text for `normal-3page.pdf` and `unicode-cmap.pdf` exactly
   (modulo the extractor's own trailing-whitespace normalization,
   which the implementation must then pin down in the worker).
2. It returns an error (not empty text, not a panic) for
   `malformed-truncated.pdf`, `malformed-xref.pdf`, and
   `encrypted-user.pdf` (no password passed).
3. It returns an empty/whitespace-only result for `scanned-image.pdf`
   and `empty-text.pdf`.
4. The decompression bomb (§6) does not grow the worker's memory
   without bound: measure peak RSS while extracting it; if the
   extractor cannot bound the decoded output, the implementation
   must add a bounded decode path (lopdf per-stream decode with a
   `Read::take` cap) before integration — the slice does not land on
   an unmeasured bomb.
5. `malformed-header.pdf`: poppler recovers this input (warning +
   text). The spike must confirm lopdf refuses it; if lopdf also
   recovers it, the fixture is hardened (corrupt the first object
   too) before the slice lands — the acceptance criterion is refusal,
   and a parser that silently repairs a bad header does not meet it.

If any spike check fails, the slice stops and reports; no weaker
parser is silently substituted.

**Spike outcomes (2026-11-05, `docs/handoffs/m13-pdf-spike-results.md`):**
1. PASS — by-pages returns the manifest text exactly (modulo one
   trailing newline per page, removed by the worker's trim).
2. PARTIAL — truncated and encrypted error ✓; the xref fixture
   yields 0 pages with no error, corrected by the worker's
   0-pages→parse-error mapping (§3.2, correction 4).
3. PASS — scanned + empty yield 0 chars.
4. FAIL for pdf-extract alone (~45 s / ~64 MB on the fixture bomb);
   PASS with the pre-scan (~190 ms / ~7 MB fixture, ~160–180 ms / ~11 MB
   worst case). The pre-scan is now mandatory (§3.2).
5. PASS — lopdf refuses the bad-header fixture (`invalid file
   header`); the fixture stands as-is.

## 3. Design

### 3.1 Limits (locked before coding)

| Limit | Value | Enforced where |
|---|---|---|
| Raw PDF file | 1–1,048,576 bytes (1 MiB) | `ingest-begin` validation (before upload); worker re-read via `snapshots::limited` with the PDF cap |
| Upload part | ≤ 64 KiB, offset-exact, sequential | `ingest-part` (unchanged) — a 1 MiB PDF is 16 parts, so multi-part resume is finally exercised |
| Pages | 1–100 | Worker, after extraction (page count from the extractor) |
| Extracted text, per page | 1–8,000 chars | Worker, per page (a page holding more text than a whole TXT document is pathological) |
| Extracted text, per document | 1–32,000 chars | Worker, running total. The 32,000-char cap is a **storage** cap, justified on its own grounds (worker time, memory, FTS index size, UI rendering, 4× the TXT cap for multi-page documents). It is **not** an Ask-retrieval guarantee: Ask retrieves top-ranked chunks within a separate per-query budget (default 4,000 chars, max 32,000 only if requested — §3.4). A max-size document is never fully retrieved by default. |
| Single stream, decompressed | ≤ 16 MiB | Enforced by the mandatory pre-scan before any extraction (see §3.2) |
| Active ingestion uploads / jobs | 8 / 32 per workspace | Unchanged (Slice 1 caps) |
| File name | 1–255 chars, basename, `.pdf` | `ingest-begin` (extension must match kind `pdf`) |

Refusals are fail-closed and named: oversize raw → begin 400 ("Document
file must be 1 to 1048576 bytes…"); over page/text cap, malformed,
encrypted, no-text-layer, empty → job Failed with the exact message
(§5). Nothing is truncated silently: a document that exceeds a cap is
refused whole, never published partial — partial publication would be
dishonest provenance.

### 3.2 Extraction (`crates/core/src/pdf.rs`, new)

New module, pure function over bytes (no I/O, no threads, no network):

```rust
pub struct ExtractedPage { pub number: u32, pub text: String }
pub struct ExtractedPdf { pub pages: Vec<ExtractedPage> }
pub fn extract_pdf_text(bytes: &[u8]) -> Result<ExtractedPdf, ExtractError>
```

- **Check ordering (spike-proven).** lopdf/pdf-extract return 0 pages
  with *no error* for a broken document (bad `/Root`) and for an
  encrypted document (undecryptable page tree), so the worker must
  check in this order: (1) `/Encrypt` present in the trailer →
  `ExtractError::Encrypted`; (2) load error → `ExtractError::Parse`;
  (3) **0 pages → `ExtractError::Parse`** (never "empty"); (4)
  per-page extraction + caps; (5) empty total → `ExtractError::Empty`.
  A page-count-first flow misclassifies encrypted documents as empty.
- **Bounded pre-scan (mandatory, runs first).** Before any text
  extraction, lopdf loads the document structure and decodes every
  page content stream (and Form XObject streams, one level) through
  a capped FlateDecode reader with a 16 MiB **total** decoded
  budget, aborting mid-stream when the budget is exceeded
  (`ExtractError::Bomb`). Non-content streams are never decoded
  (the raw cap's job). The spike measured: fixture bomb refused in
  ~190 ms / ~7 MB peak; a 1 GiB worst-case expansion refused in
  ~160–180 ms / ~11 MB peak. Without the pre-scan, pdf-extract fully
  decodes the bomb (~45 s / ~64 MB for the 20 MB fixture).
  Prototype gaps for integration: filter chains are refused;
  non-Flate expanding filters (LZW) are refused as unsupported
  (integration adds a capped LZW path via `weezl`, already in the
  graph, or refuses LZW content streams); nested Form XObjects are
  walked one level only. Object streams (PDF 1.5+ compressed
  objects) are decoded by lopdf at load, outside the pre-scan's
  budget — residual risk, same class as RUSTSEC-2026-0187,
  mitigated by the version floor + raw cap; a dedicated
  object-stream bomb fixture is follow-up work.
- **Per-page extraction with pdf-extract's quality.** The worker
  calls the public `pdf_extract::output_doc_page` + `PlainTextOutput`
  API (exactly what `extract_text_by_page` does internally), one
  page at a time, accumulating chars and stopping early when a cap
  trips (a 20-page / 141,400-char document stops at page 5 in
  115 ms, versus 430 ms for all-pages extraction). The output is
  identical to `extract_text_from_mem_by_pages` (270 chars on the
  normal fixture, 5 on the CMap fixture). An error maps to
  `ExtractError::Parse` ("This PDF could not be read. It may be
  damaged or use features this importer does not support.").
  **lopdf's own extractor must never supply text**: it drops line
  breaks entirely and uses glyph names over ToUnicode (capitals
  U+0391/U+0392 instead of the mapped lowercase U+03B1/U+03B2).
  lopdf is structure-only (page count, content-stream enumeration
  for the pre-scan); pdf-extract is the sole text extractor.
- **Encrypted.** A document whose trailer carries `/Encrypt` and
  does not open with an empty password maps to
  `ExtractError::Encrypted` ("This PDF is encrypted.
  Password-protected PDFs cannot be imported in this slice.").
  A document that opens with an **empty password** is treated as
  unencrypted and imports normally (lopdf strips `/Encrypt` after
  a successful empty-password decryption; no credential is needed).
- Normalizes each page: trims trailing whitespace per line, drops
  empty trailing lines, joins lines with `\n`. No layout heuristics,
  no reading-order repair, no Markdown conversion: the stored text is
  the extractor's text, and the provenance records exactly that.
  (pdf-extract returns one trailing newline per page; the trim
  removes it, so stored pages equal the fixture manifest exactly.)
- Enforces the page cap (error if the page count > 100, checked
  before any text extraction), the per-page cap, and the running
  total cap; empty total (all pages whitespace) maps to
  `ExtractError::Empty` ("This PDF contains no readable text.
  Scanned or image-only PDFs need OCR, which is a separate slice.").
  A page with zero text is kept as an empty page (it still counts
  toward the page cap and appears in provenance with 0 chars) — page
  numbers must stay aligned with the file.

### 3.3 Provenance and stored body

`DocumentProvenance` gains an optional, serde-defaulted field (contract
change, regenerated — no new DTO, no migration):

```rust
pub struct PdfPageProvenance { pub page: u32, pub start_char: u64, pub chars: u64, pub hash: String }
pub struct DocumentProvenance {
    // ... existing fields, unchanged ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<PdfPageProvenance>>,
}
```

- `media_type_for("pdf")` = `"application/pdf"`; `bom` is always
  `false` for PDFs (no BOM concept).
- The stored body is the pages joined by a form feed on its own line
  (`\n\x0c\n`). Per-page provenance records `start_char` (char offset
  of the page's first character in the stored body), `chars`, and
  `hash` (SHA-256 of that page's text, without the separator). Page
  ranges are contiguous, in order, and cover the body minus
  separators — the worker asserts this before publishing, and
  `memory_write`'s provenance re-validation recomputes the page hashes
  from the body (forged or stale page metadata is refused exactly like
  forged text hashes are today).
- `text_hash` is the SHA-256 of the **stored body** (with separators),
  so the existing (title, text hash) dedup guard works unchanged: two
  identical PDFs publish once; the same file name with different
  content publishes a second record.
- `DocumentSource::kind_for_label` gains `"pdf"`; the `ExportSelection::
  Ingestion.format` value is `"pdf"`.

### 3.4 Search, Ask citations, backup/restore

- **Search.** FTS indexes the stored body (form feeds tokenize as
  whitespace). No server change. The Documents search lists the
  document; the UI shows the page containing the first match by
  scanning the fetched body for the query term and mapping the offset
  through the provenance ranges (client-side; the body is already
  fetched for the reader).
- **Ask citations.** The chunker gains a page-aware path: for a
  `document` memory whose provenance carries `pages`, chunks are
  built per page with `section_heading = "Page N"` (1-based) and
  `line_number` = the line number *within that page*. Citations then
  render as `document · Page 3 (line 12)` with no contract or UI
  change (§1). The compiled prompt's chunk header includes the page
  label, so the model cites the page it used. Non-PDF documents keep
  the existing heading-based chunking.
  **Storage cap vs retrieval budget (corrected 2026-11-05 — see
  §10).** The 32,000-char document cap is a *storage* cap: how much
  text a document may hold. What Ask *retrieves* is a separate
  per-query budget — default 4,000 chars, max 32,000 only if the
  caller requests it — shared across **all** evidence and spent on
  top-ranked 1,000-char chunks. A 32,000-char document is therefore
  never fully retrieved by default; it contributes at most a few
  1,000-char chunks to any one answer (fewer if other evidence
  competes), and the page-level citation points at the chunk's page.
  No claim is made that a max-size document is "fully used as
  evidence".
- **Backup/restore.** Provenance (including `pages`) lives in the
  memory metadata that restore already preserves and re-validates;
  the body is restored verbatim. No restore change. The e2e proves a
  restored PDF document keeps its page provenance, full-body search
  finds it, and Ask cites its page.

### 3.5 Cancellation and restart

- **Upload phase:** identical to TXT/MD. The job is Paused with
  checkpoint `uploading:<received>`; cancel removes staging; a killed
  API leaves the staged file, and resume continues from the checkpoint
  offset via `ingest-part` (multi-part resumes now genuinely happen:
  a 1 MiB PDF is 16 parts).
- **Extraction phase (worker):** the worker claims the job (Running,
  120 s lease) and extracts page by page from the staged file. A
  cancel arriving mid-extraction takes effect at the publish guard:
  `memory_write` refuses to publish for a job that is no longer
  Running under the lease, so a cancelled-while-extracting job
  publishes nothing and its staging is removed by the cancel path.
  No mid-extraction abort is needed: extraction is bounded by §3.1
  (≤ 100 pages, ≤ 32,000 chars) and completes in milliseconds for
  in-cap files. If the API dies mid-extraction, the stale lease
  recovers to Paused ("Worker interrupted; resume explicitly"); Resume
  re-queues, and the worker re-runs extraction from the staged file —
  idempotent, and the dedup guard covers the crash-after-publish
  window (nothing is published twice).
- **Retry:** a Failed extraction keeps staging (`staged` checkpoint)
  for Retry, exactly as Slice 1. Retry is for transient failures
  (e.g. a storage hiccup), not for refused files: retrying the same
  bytes fails identically with the same honest message.

### 3.6 UI (`Documents.tsx`, `documents.ts`, `Navigation.tsx` — no new view)

- The file picker accepts `.pdf,application/pdf` alongside
  `.txt,.md`; the review step shows file name, bytes, declared
  SHA-256, kind `pdf`, privacy (Private default, Sealed disabled).
  The client pre-refuses by extension and raw size only.
- The reader renders PDF documents as **plain text with page tabs**
  (split on `\x0c`), never through the Markdown renderer (PDF text
  is not Markdown; a `#` line in a PDF must not become a heading).
  The provenance `<details>` shows file name, media type, raw/text
  hashes, byte/char counts, page count, and the per-page table
  (page, chars, hash prefix).
- Search results for PDF documents show the page of the first match
  (§3.4). No change to `Jobs.tsx`, the navigation, or the Ask view.

## 4. Requirement mapping

- **§12 Document intelligence** — this slice delivers the PDF
  text-layer format (text-based PDFs; scanned/image PDFs refused
  honestly pending the OCR slice). DOCX/PPTX/XLSX/HTML/EML/audio/
  video transcripts remain follow-ups; AN-025 stays `in progress`.
- **§66 Background jobs** — same persistent job shape (Paused upload,
  Queued, Running, Completed/Failed/Cancelled, explicit Resume, Retry,
  120 s lease, stale-lease recovery). No new job category.
- **§68 MCP** — no MCP change. (A future slice may expose a read-only
  PDF text tool; not this one.)
- **§71 Security** — parser runs in-process, pure Rust, no subprocess,
  no network, no shell, no fetched scripts; hostile inputs are
  refused fail-closed (malformed, encrypted, bomb); the lopdf
  advisory floor (§2.2) is pinned; staged bytes are hash-verified
  before extraction; provenance is re-validated server-side.
- **§74 Privacy** — Private by default, Sealed refused (no encrypted
  vault path for imports); workspace/project binding unchanged;
  extracted text is indexed only through the authorized search
  pipeline; Ask uses the same evidence/disclosure path.

## 5. Acceptance criteria (named)

- **AC-PDF-01** Normal 3-page PDF imports end-to-end (select → approve
  → upload → job → open → search → Ask → backup/restore) with exact
  per-page text and provenance.
- **AC-PDF-02** A ToUnicode CMap PDF extracts the mapped Unicode text
  (αβ€), not raw bytes.
- **AC-PDF-03** Encrypted PDF (user password) is refused with the
  encrypted message; nothing is published; no password prompt exists.
- **AC-PDF-04** Truncated, bad-header, and bad-xref PDFs are refused
  with a parse error; nothing is published.
- **AC-PDF-05** A PDF over the 1 MiB raw cap is refused at
  `ingest-begin` (400) before any upload.
- **AC-PDF-06** A PDF under the raw cap but over the 32,000-char text
  cap (or over the 100-page cap, or a page over 8,000 chars) is refused
  at extraction; nothing is published, nothing truncated.
- **AC-PDF-07** A scanned/image-only PDF is refused as "no text layer"
  (OCR named as the separate slice); a blank-page PDF is refused as
  empty.
- **AC-PDF-08** The decompression bomb is refused without unbounded
  memory growth (spike-measured bound, §2.3.4).
- **AC-PDF-09** Concurrent duplicate PDF imports publish exactly one
  document (same dedup guard, same artifact).
- **AC-PDF-10** Cancel during upload publishes nothing and removes
  staging; cancel during extraction publishes nothing.
- **AC-PDF-11** Kill the API mid-upload, restart, resume from the
  checkpoint offset (a multi-part resume: a >64 KiB PDF), exactly one
  document.
- **AC-PDF-12** Ask cites the PDF with a page number
  (`document · Page N (line M)`); the citation's snippet comes from
  that page.
- **AC-PDF-13** Search finds PDF content through full-body FTS and the
  UI shows the page of the first match.
- **AC-PDF-14** Backup → fresh restore preserves the PDF document,
  its body, and its page provenance; search and Ask work on the
  restored copy.
- **AC-PDF-15** Forged page provenance (wrong page hash or range) is
  refused by `memory_write` exactly like forged text hashes.
- **AC-PDF-16** No new contract drift, tsc 0, docs validator passes,
  UI + Rust suites green, documents e2e 9/9 + new PDF spec green,
  full regression battery green except the known pre-existing
  dashboard failure.

## 6. Fixtures

Committed in `tests/fixtures/pdf/` with a deterministic stdlib-only
generator (`generate-pdf-fixtures.py`, no network, no system PDF
tools) and a self-checking `MANIFEST.json` (SHA-256 + expected
per-page text + expected outcome). Reference cross-checks were done
with `pdftotext` (poppler, dev-time only — never the app's parser).

| Fixture | Size | Expected outcome |
|---|---|---|
| `normal-3page.pdf` | 1,479 B, 3 pp. | Imported; exact per-page text (WinAnsi é on p. 2) |
| `unicode-cmap.pdf` | 1,079 B, 1 p. | Imported; extracts `αβ€` via ToUnicode CMap |
| `malformed-truncated.pdf` | 887 B | Refused: parse error (cut at 60%) |
| `malformed-header.pdf` | 1,479 B | Refused: parse error (bad `%PDF` magic; poppler recovers — spike item §2.3.5) |
| `malformed-xref.pdf` | 1,479 B | Refused: parse error (xref offsets zeroed + trailer `/Root` broken; the parser reports **0 pages with no error**, so the worker maps 0 pages to a parse failure — §3.2) |
| `encrypted-user.pdf` | 1,030 B, 1 p. | Refused: encrypted (R=2 RC4, user password `secret`; `pdftotext -upw secret` extracts it) |
| `oversized-raw.pdf` | 1,100,767 B | Refused: raw cap (unreferenced 1.1 MB stream) |
| `oversized-text.pdf` | 2,058 B, 5 pp. | Refused: text cap (35,360 chars over 32,000; raw under cap) |
| `too-many-pages.pdf` | 30,070 B, 101 pp. | Refused: page cap |
| `scanned-image.pdf` | 878 B, 1 p. | Refused: no text layer (image XObject only) |
| `empty-text.pdf` | 625 B, 1 p. | Refused: empty (blank page) |
| `decompression-bomb.pdf` | 20,113 B, 1 p. | Refused: extraction bound (one FlateDecode stream expands to 20,000,031 bytes; verified by direct decompression) |

Fixture verification evidence (this design): `pdftotext` extracts the
manifest's expected text for `normal-3page.pdf` and `unicode-cmap.pdf`
exactly; `scanned-image.pdf` and `empty-text.pdf` yield no text;
`encrypted-user.pdf` fails without a password ("Incorrect password")
and extracts with `-upw secret`; `malformed-truncated.pdf` and
`malformed-xref.pdf` fail in pdftotext; `too-many-pages.pdf` reports
101 pages; `oversized-text.pdf` extracts 35,360 chars. The one
fixture whose reference behavior differs from the expected slice
outcome is `malformed-header.pdf` (poppler recovers it) — see §2.3.5.

## 7. Files to change (planned)

- `crates/core/src/pdf.rs` (new) — extraction + limits + error mapping.
- `crates/core/src/document.rs` — PDF limits, `kind_for_label("pdf")`,
  `media_type_for("pdf")`, `DocumentProvenance.pages` +
  `PdfPageProvenance`.
- `crates/core/src/ingestion.rs` — kind dispatch in
  `execute_ingestion` (PDF → extractor; TXT/MD → UTF-8 path,
  unchanged), PDF byte cap in the staged read, tests.
- `crates/core/src/context.rs` — page-aware chunking for `document`
  memories with PDF provenance; tests.
- `crates/core/src/memory.rs` — provenance re-validation extended to
  page hashes/ranges; forged-provenance test.
- `crates/contracts/src/main.rs` + regenerated contracts — new
  provenance field.
- `crates/core/Cargo.toml` + `Cargo.lock` — `pdf-extract =0.12.1`
  and `lopdf =0.42.0` (production: the pre-scan uses lopdf
  directly; both pinned to the same version so the graph holds
  exactly one lopdf), `flate2 =1.1.10` (miniz_oxide backend).
- `packages/ui/src/Documents.tsx`, `documents.ts`, `documentInput.ts`
  — `.pdf` accept, page tabs, plain-text reader, page-of-match.
- `packages/ui/tests/documents.test.tsx` — PDF UI tests.
- `tests/e2e/documents-pdf.spec.ts` (new) — AC-PDF-01..15 via the
  fixtures (9 TXT/MD tests untouched).
- `docs/handoffs/m13-pdf-ingestion-addendum.md` (new, at delivery) —
  evidence, deviations, Windows checklist, proposed next slice (DOCX).

## 8. Implementation preconditions

1. **Windows rerun first.** Implementation starts only once Antigravity
   confirms the Windows ingestion tests pass (Slice 1 + repair) and
   shares the accepted integration commit. That commit is the
   implementation baseline — not `dd0423ced`, and not the readiness
   branch tip. If the rerun finds a regression, repair that finding
   first; this design waits.
2. **Spike gate (§2.3)** passed with the amendments in §10,
   recorded in `docs/handoffs/m13-pdf-spike-results.md` (resolved
   versions, advisory coverage, bomb memory/time, per-fixture
   outcomes).
3. **Design + acceptance criteria committed before code** (this
   document), per the owner's standing rule.

## 9. Not in this slice

OCR (scanned/image PDFs) — separate slice with its own design
(local OCR engine selection, language data, accuracy bounds).
DOCX — after PDF, same shape. PDF forms/annotations, links, images,
tables, layout reconstruction, password entry, encrypted-PDF support,
and any MCP tool exposure. No paid/cloud inference, no model
downloads, no generated-product execution, no pushes, no CI, no
publication, no preview promotion.

## 10. Spike amendments (2026-11-05)

The parser spike ran on branch `muse/m13-pdf-spike` against the 12
committed fixtures; full evidence in
`docs/handoffs/m13-pdf-spike-results.md`. The amendments below
supersede the original wording where they conflict.

1. **Storage cap vs Ask retrieval budget (§3.1, §3.4).** The original
   §3.1 justified the 32,000-char document cap as "a max-size PDF
   can still be fully used as Ask evidence" because it equals
   `MAX_ALLOWED_BUDGET_CHARS`. That is wrong and is corrected: the
   32,000-char cap is a **storage** cap (worker time, memory, FTS
   index size, UI rendering, 4× the TXT cap). What Ask *retrieves*
   is a separate **per-query** budget — default 4,000 chars, max
   32,000 only if the caller requests it — shared across **all**
   evidence and spent on top-ranked 1,000-char chunks. A 32,000-char
   document is never fully retrieved by default; it contributes at
   most a few 1,000-char chunks per answer, and the page-level
   citation points at the chunk's page. No claim is made that a
   max-size document is "fully used as evidence".
2. **Bounded pre-scan is mandatory (§2.2, §3.2).** pdf-extract alone
   does not stop the decompression bomb during decoding: it fully
   decodes the 20 MB fixture (~45 s, ~64 MB peak) and would take
   ~35 minutes / ~3 GB on a 1 GiB expansion. The pre-scan (lopdf +
   capped FlateDecode, 16 MiB total decoded budget) stops the
   fixture bomb in ~190 ms / ~7 MB and a 1 GiB worst case in
   ~160–180 ms / ~11 MB, and must run before pdf-extract on every PDF. lopdf is
   therefore a **production** dependency, pinned `=0.42.0` (the
   version pdf-extract resolves, so the graph holds exactly one
   lopdf).
3. **Per-page pdf-extract is the extraction path (§3.2).** The
   public `output_doc_page` + `PlainTextOutput` API gives per-page
   extraction with output identical to `extract_text_from_mem_by_pages`
   (270 chars normal, 5 chars CMap), enabling early termination at
   the text cap (a 20-page / 141,400-char document stops at page 5
   in 115 ms, versus 430 ms for all-pages extraction) with no
   quality loss.
4. **Worker check ordering (§3.2, AC-PDF-04).** lopdf/pdf-extract
   return 0 pages with *no error* for a broken document (bad
   `/Root`) and for an encrypted document (undecryptable page
   tree). The worker checks in order: `/Encrypt` → encrypted;
   load error → parse error; **0 pages → parse error** (never
   "empty"); then caps; empty total → empty/no-text-layer. The
   malformed-xref fixture's expected outcome is corrected to
   "refused: parse error (0 pages — the parser reports no error;
   the worker maps 0 pages to a parse failure)".
5. **Empty-password documents (§3.2, AC-PDF-03).** Documents that
   open with an empty password are treated as unencrypted and
   import normally (lopdf strips `/Encrypt` after a successful
   empty-password decryption; no credential is needed).
6. **lopdf is structure-only (§2.2, §3.2).** Its own extractor
   drops line breaks entirely and uses glyph names over ToUnicode
   (capitals U+0391/U+0392 instead of the mapped lowercase
   U+03B1/U+03B2), so it must never supply text; pdf-extract is
   the sole text extractor. This is why the production needs both
   crates, not lopdf alone.
7. **Pre-scan scope and gaps (§3.2).** Decodes page content
   streams + Form XObject streams (one level; nested forms are a
   documented integration gap). Filter chains are refused in the
   prototype (gap). Non-Flate expanding filters (LZW) are refused
   as unsupported in the prototype (gap — integration adds a
   capped LZW path via `weezl`, already in the graph, or refuses
   LZW content streams). Non-content streams are never decoded
   (raw cap's job). Object streams (PDF 1.5+ compressed objects)
   are decoded by lopdf at load, outside the pre-scan's budget —
   residual risk, same class as RUSTSEC-2026-0187, mitigated by
   the version floor + raw cap; a dedicated object-stream bomb
   fixture is follow-up work.
8. **Audit (§2.2).** `ctutils` 0.4.3 / RUSTSEC-2026-0385 is
   pre-existing (baseline lock via `argon2`/`chacha20poly1305`),
   flagged for the owner, out of scope. `ttf-parser` 0.25.1
   unmaintained (RUSTSEC-2026-0192) is new via lopdf ← pdf-extract
   — a warning, not a vulnerability; revisit if an advisory
   appears.
