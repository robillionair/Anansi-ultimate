# M13 DOCX Ingestion — design + acceptance criteria

Branch: `muse/m13-docx-ingestion`, from the expected-target
contract repair tip `af2d4b1646bb7c5a7712efcfd4ee2b95f2e4703d`.
This document is committed BEFORE the spike and before any
integration code. The spike may amend it (like the PDF slice's
§10); amendments are recorded in the spike-results doc, never
silently.

Antigravity owns Windows integration. This slice is Linux
implementation + verification only.

## §1 Goal and scope

Import an ordinary text-bearing DOCX through the existing
resumable ingestion job and publication guard, preserving heading
and paragraph provenance, and make the imported text searchable
and citable through the existing document and Ask workflows.

In scope: DOCX (Office Open XML, `.docx`) text extraction only.
Out of scope: PPTX, XLSX, OCR, scanned/image-only documents,
headers/footers/footnotes/endnotes/comments text, list-number
expansion, embedded-object text. Those stay refused or ignored
honestly, as §4 states.

Master Spec §12 lists DOCX among the formats, requires source
identifiers preserved, local processing when possible, no automatic
sending of private documents to cloud models, and ingestion that is
asynchronous, resumable, and cancellable. This slice satisfies
those for DOCX's text layer.

## §2 The parser candidate (to be spiked, not assumed)

DOCX is a ZIP archive (deflate) containing XML parts. The
candidate parser is a **bounded ZIP walk + streaming XML parse**,
built on two crates:

- `zip` — ZIP archive reading (central directory for the entry
  list; per-entry inflation through a capped reader).
- `quick-xml` — streaming (event-based, never a DOM) parse of
  `word/document.xml` only.

Why this shape, and not a DOCX DOM library (e.g. `docx-rs`): the
M13 bounds philosophy (PDF precedent) requires the expansion
budget to be enforced *during* inflation and the text caps to
abort parsing mid-document. A DOM parser materialises the whole
document before any cap can fire, so it cannot meet the bounds on
its own; it would need the same bounded ZIP pre-scan in front of
it, at which point the streaming parser does the whole job with
fewer dependencies. That is a design judgment, stated as such —
the spike measures the streaming parser; it does not re-litigate
`docx-rs`.

Exact crate versions are resolved and pinned in the spike
(`Cargo.toml` exact pins, one version each in the graph), and
`cargo audit` runs on the spike graph, following the PDF spike.

## §3 Provisional limits (spike confirms or amends)

| Bound | Value | Enforced |
|---|---|---|
| Raw file bytes | ≤ 1 MiB | `max_bytes_for_kind("docx")`, checked at ingest-begin and at the staged-file gate |
| Extracted text | ≤ 32,000 chars | capped during parsing; abort mid-paragraph |
| Archive entries | ≤ 128 | central directory, before reading any entry |
| Cumulative inflated bytes | ≤ 16 MiB | capped reader during inflation, abort mid-entry |
| Single entry inflated | ≤ 8 MiB | per-entry declared size checked first, capped reader second |
| Paragraphs parsed | ≤ 10,000 | parse loop; bounds work on pathological many-empty-paragraph files |
| Heading levels | 1–3 | styles Heading1–3 + Title (→ `#`), `w:outlineLvl` 0–2 as fallback |

A normal DOCX has ~10–20 entries; the caps are generous for
real files and tight for abuse. The spike measures the bomb case
for elapsed time and peak RSS, as the PDF spike did.

## §4 Extraction rules (the contract the spike must prove)

**Archive walk.** Read the central directory; refuse when the
entry count exceeds the cap, when any entry is encrypted
(ZipCrypto/AES flag — refuse, never attempt), when
`word/document.xml` is absent, or when `word/vbaProject.bin` is
present (macros = active content: refuse honestly). Entry names
are matched exactly; no entry is ever written to disk and no path
is ever constructed from an entry name. Only `word/document.xml`
is read; all other parts are ignored as data (images, embeddings,
headers, footers, footnotes, comments, styles, numbering).

**XML safety.** The `word/document.xml` bytes are pre-scanned for
`<!DOCTYPE` and refused when present (no DTD, no entity
expansion, no billion-laughs class). Parsing is streaming via
quick-xml; a malformed document refuses (never a partial body).
No external entity is ever resolved, because nothing is fetched:
relationship targets with `TargetMode="External"` are ignored
entirely — not fetched, not recorded.

**Text rules (WordprocessingML, `w:` namespace; match on local
names after the prefix).**
- `w:p` → one body line; whitespace-only paragraphs skipped.
- `w:t` runs concatenated; `xml:space="preserve"` honoured
  (text taken as-is, paragraph trailing whitespace trimmed).
- `w:tab` → `\t`; `w:br` → `\n` (soft break); `w:br`
  `type="page"` → paragraph break (never `\x0c`: a form feed
  would trigger the PDF page-chunking branch in `context.rs`
  and lose heading citations).
- `w:instrText` (field codes: HYPERLINK, PAGEREF, TOC, …) skipped
  entirely — never executed, never included. Field *results*
  (ordinary `w:t` runs) are included: a hyperlink yields its
  display text, never its URL.
- `w:hyperlink`: inner runs are text; its `r:id` is ignored.
- `w:sdt` (structured tags): inner runs are text. `w:ins`:
  included (current text). `w:del`: skipped (deleted text is not
  current). `w:smartTag`: inner runs are text.
- `w:tbl`: cell paragraphs in document order, each its own body
  line; no row markers.
- `w:drawing` / `w:pict` / `w:object`: skipped (no text).
- `w:altChunk`: refuse the document (its text would be missing,
  so importing would publish a partial document).
- List numbering (`w:numPr`): NOT expanded — list paragraphs are
  plain paragraphs; the bullet/number glyph is not reproduced.
  Recorded as a known limit, not a defect.

**Headings.** `w:pStyle` `Heading1`/`Heading2`/`Heading3` →
`#`/`##`/`###` body lines; `Title` → `#`. `w:outlineLvl` 0–2
is the fallback when no style names a heading. A heading line is
a body line like any other, so the existing Markdown-heading
chunking in `context.rs` gives section headings for free: Ask
citations read `document · <Heading> (line N)` with no
`context.rs` change (verified by test, AC-D1/D2). If the spike
shows `outlineLvl` is unreliable, the fallback is dropped and
the amendment recorded.

**Body format.** Paragraphs joined by `\n`; headings as `#`
lines. No form feeds, no trailing blank lines.

## §5 Provenance

`DocumentProvenance` gains `sections:
Option<Vec<DocxSectionProvenance>>` (contract regeneration, like
PDF's `pages`): one entry per heading section, covering the body
contiguously — a leading entry with `heading: null, level: 0`
for paragraphs before the first heading, then one entry per
heading. Each entry: `heading` (Option<String>), `level` (u8),
`startChar`, `chars`, `textHash` (SHA-256 of the section text,
via the existing `content::hash`). The reader/search/Ask paths
use the body; the sections list is the explicit, checkable
provenance record.

**Raw-hash verification:** same as PDF — the binary DOCX is not
recoverable from the extracted text, so
`validate_document_source` checks the raw hash's shape only; its
value is verified at ingestion against the staged file (length +
SHA-256 gate before extraction). `memory.rs` `validate_import`
gains the media-type mapping
`application/vnd.openxmlformats-officedocument.wordprocessingml.document`
→ `docx`.

## §6 Integration plan (after the spike passes)

- `crates/core/src/docx.rs`: `extract_docx_text(&bytes) ->
  Result<ExtractedDocx, ExtractError>` mirroring `pdf.rs`
  (caps, `ExtractError::message()` user-facing refusals).
- `crates/core/src/document.rs`: kind `docx`, raw 1 MiB / text
  32,000 chars, media type above, `kind_for_label` accepts
  `docx`, validate with shape-only raw hash like PDF.
- `crates/core/src/ingestion.rs`: `kind == "docx"` branch in
  `execute_ingestion` (extract after the staged-file gate;
  refusal fails the job with the extractor's message; no partial
  publish). `enqueue_ingestion` and ingest-part already read the
  staged file with the per-kind limit; `jobs.rs` validate already
  uses `max_bytes_for_kind` + `kind_for_label`, so DOCX flows
  through with no change there.
- `crates/core/src/memory.rs`: media-type mapping (§5).
- `context.rs`: no change expected (headings in body); verified
  by test.
- UI: `packages/ui/src/documents.ts` `prepareDocument` accepts
  `.docx` (binary, server-side extraction, raw hash, 1 MiB cap);
  `Documents.tsx` file input + review line ("DOCX — text
  extracted on the server during import"); Markdown rendering
  for DOCX bodies (headings render; no new reader component);
  search shows the section heading of the first match
  (client-side, mirroring the PDF "· Page N" display);
  `packages/contracts/src/client.ts` `ingestDocument` size limit
  per-kind.
- Contracts regenerated; docs validator green.

## §7 Spike plan

- Example binary `crates/core/examples/docx_spike.rs` (spike
  only, not production code), following the PDF spike.
- Fixtures under `tests/fixtures/docx/` with
  `generate-docx-fixtures.py` (python-docx 1.1.0, deterministic)
  + `MANIFEST.json` (name, SHA-256, expected text/sections).
  Fixtures: normal (headings + paragraphs + table + hyperlink +
  list), unicode-headings (CJK, emoji, accents), field-codes
  (hyperlink field, pageref), track-changes (ins/del),
  empty (no text), whitespace-only, corrupt (truncated ZIP, bad
  CRC, not-a-ZIP, bad XML, missing document.xml, missing
  content-types), oversized-raw (> 1 MiB), oversized-text
  (> 32,000 chars), zip-bomb (small raw, > 16 MiB expansion),
  many-entries (> 128), encrypted-entry (flag-only synthetic),
  macro (vbaProject.bin), altChunk, external-hyperlink.
  One LibreOffice-generated .docx as a real-world cross-check
  (spike output vs `libreoffice --convert-to txt`).
- Spike measurements per fixture: outcome (ok/refused + reason),
  extracted chars, sections, elapsed, peak RSS for the bomb.
  Dependency versions resolved + `cargo audit` on the spike
  graph.
- **Spike gate:** if the parser cannot meet the bounds or
  preserve provenance, the slice delivers the spike + the exact
  blocker instead of integrating.

## §8 Acceptance criteria

- **AC-D1** PASS. A normal DOCX imports end to end; headings
  preserved as `#` lines; paragraphs in order; search finds the
  text; Ask cites `document · <Heading> (line N)`.
- **AC-D2** PASS. Unicode headings (CJK, emoji, accents)
  preserved exactly; citations resolve to them.
- **AC-D3** PASS. Corrupt files (truncated ZIP, bad CRC,
  not-a-ZIP, bad XML, missing document.xml) refused with honest
  messages; no document published.
- **AC-D4** PASS. Oversized files refused: raw > 1 MiB at
  ingest-begin; text > 32,000 chars fails the job; no partial
  publish.
- **AC-D5** PASS. Archive expansion bounded: zip bomb refused at
  the 16 MiB cumulative budget during inflation (fast, low
  memory); entry-count cap refused; encrypted entries refused;
  macro-bearing files refused; altChunk refused.
- **AC-D6** PASS. External content never fetched: hyperlinks /
  relationships / external targets ignored; no network calls;
  display text imported, URLs not included.
- **AC-D7** PASS. Field codes not executed: `w:instrText`
  excluded, field results included; deleted text excluded,
  inserted text included.
- **AC-D8** PASS. Cancellation during a DOCX upload publishes
  nothing and removes staging; killing the API mid-upload,
  restarting, and resuming imports exactly once.
- **AC-D9** PASS. Backup + fresh restore preserves the document,
  its sections provenance, and its searchability.
- **AC-D10** PASS. Search shows the section heading of the first
  match; Ask citations resolve to the imported source.
- **AC-D11** PASS. Spike gate: the parser spike meets the bounds
  and preserves provenance, measured on deterministic fixtures,
  before integration (or the slice delivers spike + blocker).
- **AC-D12** PASS. No regressions: TXT/Markdown e2e 9/9, PDF e2e
  5/5, Ask e2e 1/1, Rust lib, tsc, contracts (regenerated with
  the `sections` addition), docs, UI.

## §9 Out of scope

PPTX, XLSX, OCR, scanned/image-only documents, headers /
footnotes / comments text, list-number expansion, embedded-object
text, preview promotion, pushes, remote CI. No cloud calls, no
paid inference, no model downloads.
