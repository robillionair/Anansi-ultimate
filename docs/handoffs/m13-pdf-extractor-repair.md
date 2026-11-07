# M13 PDF extractor repair: close the pre-scan / parser gap

Branch `muse/m13-pdf-extractor-repair`, from extractor tip
`a14d6c96be8d8072df88ca6afc8aa07e828ffb5d`. Standalone module
only — no production upload or ingestion-job wiring. Integration
still waits for Antigravity's merged baseline (see §8).

## 1. What was wrong

The extractor module (`crates/core/src/pdf.rs`, `a14d6c96b`)
pre-scanned the streams it *thought* the parser would decode,
and charged each once. Tracing each new fixture through the actual
decoder (lopdf 0.42.0 + pdf-extract 0.12.1, sources in
`~/.cargo/registry/src/*/`) showed four ways the parser decodes
more than the pre-scan measured — and one way the text limit was
enforced too late.

### 1.1 Indirect and inherited `/Resources` were never walked

The old `prescan_resources` read only the page dict's *direct*
`/Resources` (`dict.get(b"Resources").and_then(|o| o.as_dict())`).
An indirect `/Resources` (`5 0 R`) fails `as_dict()` on the
reference, so the function returned early; a page with no
`/Resources` of its own returned early too.

pdf-extract resolves resources the other way. `output_doc_inner`
(lib.rs:2418) uses `get_inherited(doc, page_dict, b"Resources")`,
which walks up the `/Parent` chain and dereferences indirect
references. So a Form reachable only through an indirect or an
inherited resource dict is decoded by the extractor but was never
measured by the pre-scan.

**Measured:** `indirect-resources-bomb.pdf` and
`inherited-resources-bomb.pdf` (a 20 MB Form) took the old module
~48 s / ~46 s and ~65 MB peak, refused only with `TextCap` after
the full 20 MB text was extracted. The repaired module refuses
both with `Bomb` in 178 ms / 569 ms at ~22 MB peak.

### 1.2 Compressed ToUnicode CMaps were never measured

The old pre-scan never looked at fonts. pdf-extract decodes a
font's `/ToUnicode` CMap stream on first use, via `get_contents`
(lib.rs:1255) → lopdf `Stream::decompressed_content()` —
uncapped — and `PdfCIDFont::new` does the same for a Type0 font's
`/Encoding` stream. Its font cache is cleared per page
(lib.rs:2407), so a CMap is re-decoded once per page the font is
used on.

**Measured:** `tounicode-compressed-bomb.pdf` (a 1.7M-entry CMap,
~20 MB decoded, ~40 KB raw) was **imported** by the old module
after **67,154 ms and 314,084 KB peak** — no refusal at all, just
a 67-second, 314 MB parse of 1.7M bfchar entries. The repaired
module refuses it with `Bomb` in 1,282 ms at 22,656 KB peak.

### 1.3 A Form invoked N times was charged once

The old pre-scan kept a `visited` set of Form object ids and
decoded each once. But lopdf's `decompressed_content`
(object.rs:794) does **not** cache — it decodes fresh on every
call — and pdf-extract's `Do` handler (lib.rs:1860) calls
`get_contents(xf)` on every `Do`. A Form invoked 20 times is
decoded 20 times. The same holds for any XObject `Do` invokes:
an image XObject is decompressed too (then parsed as a content
stream), which is why the scanned-image fixture exercises this
path. The old code also refused a `Do` to a non-Form XObject
outright, which was wrong — the extractor decompresses it.

**Measured:** `repeated-form-bomb.pdf` (one ~1 MB Form invoked
20 times: one decode fits the 16 MiB budget, 20 cost ~20 MB)
took the old module 45,715 ms / 45,424 KB peak, refused with
`TextCap`. The repaired module refuses it with `Bomb` in 28 ms
at 7,096 KB peak — the Form is decoded once and the other 19
decodes are charged arithmetically, so the peak stays at one
decode plus baseline.

### 1.4 The structural scan and the loader disagreed on stream boundaries

The old `locate_stream_data` ignored `/Length` and scanned for
`endstream`. The loader trusts `/Length`: lopdf's stream parser
(parser/mod.rs:355) takes exactly `length` bytes, then expects
`endstream`. If the compressed data itself contains the bytes
`endstream`, the scan truncates early and under-measures — or the
truncated Flate stream fails to decode at all.

**Measured:** `stream-boundary-ok.pdf` (a valid PDF 1.5 whose
`/ObjStm` is a two-block Flate stream: a stored block carrying
the literal bytes `endstream`, then a zlib block) was **falsely
refused** by the old module with
`Parse("decode error: incomplete deflate stream")` in 0 ms. The
repaired scan uses `/Length` and the document imports (7 ms,
6,704 KB peak).

### 1.5 The text limit was enforced after the page, not while writing

The old code called `output_doc_page` into a full `String` and
checked the per-page/total caps afterwards. A page with 1M chars
of text grew a 1M-char `String` before being refused.

**Measured** (synthetic one-page, 1,000,000 chars, decode under
budget, `/tmp/big-page.pdf`, not a committed fixture): old module
`TextCap` in 2,350 ms; repaired module `TextCap` in 92 ms — the
capped writer aborts extraction mid-page, after ~8,000 chars.

## 2. The repair

### 2.1 Resources resolved as the extractor resolves them

`resolve_page_resources` walks from the page up through
`/Parent`, returning the first `/Resources` found, dereferenced
(mirroring `get_inherited`). A page-tree cycle is refused.
`resolve_form_resources` returns a Form's own `/Resources` when
present, else the invoking context's resources (the `Do`
handler's rule). `deref_dict` / `resolve_stream` resolve direct
and indirect objects uniformly.

### 2.2 Font CMaps charged per (page, font)

`prescan_fonts` decodes each font's `/ToUnicode` and `/Encoding`
stream (when a stream) through the capped decoder, once per font
on the page. No dedup across pages (the parser re-decodes per
page) and none within a page across fonts (two fonts sharing one
CMap stream are decoded once per font). A font in the resources
that is never used for text is charged anyway — that over-counts,
which is safe (it can only refuse a document, never let a bomb
through).

### 2.3 Form/XObject invocations counted, charged per invocation

`scan_form_invocations` scans decoded content bytes for `/Name
Do`, skipping literal strings, hex strings, and comments, decoding
`#XX` name escapes, and resolving each name to an XObject id in
the (resolved) resources. A `Do` with no preceding name, an
unresolvable name, or a direct (non-reference) XObject stream is
refused — the extractor would fail on it too, and an
unreferenced stream has no id to charge per invocation against.

`charge_xobject` decodes the XObject once, charges
`multiplicity` × one decode's cost (the extra decodes are
charged arithmetically from the measured one-decode cost, via
`DecodeBudget::remaining()` before/after — no re-decoding), then
scans the decoded bytes for nested `Do` invocations and recurses
with the multiplicity multiplied down. Any XObject subtype is
charged (Form, Image, …), because `Do` decompresses them all.
A repeat on the current recursion path is a cycle and is refused;
nesting is capped at depth 32.

### 2.4 Structural scan uses `/Length`

`resolve_length` returns the byte count the loader will take:
a direct integer, or an indirect reference resolved by finding
the integer object in the raw bytes (`find_object_by_id` +
`read_object_int`). An indirect `/Length` that cannot be
resolved, a non-integer `/Length`, or a negative one is refused
(fail closed). `stream_data_by_length` takes exactly those bytes
after the `stream` EOL. When `/Length` is absent, the loader
decodes an empty stream, so the scan falls back to measuring up
to `endstream` (over-counting, safe).

### 2.5 Capped writer enforces the text limit while writing

`CappedStringWriter` implements `pdf_extract::ConvertToFmt` and
`fmt::Write` over the page `String`. Every `write_str` (each
character and each framing space/newline `PlainTextOutput`
writes) is counted against the per-page cap and the total cap
(the running total is passed in); exceeding either returns
`fmt::Error`, which `PlainTextOutput` turns into
`OutputError::FormatError` — the only source of that variant
during extraction, so the caller maps it to `TextCap`. The
post-write checks stay as defence in depth.

## 3. New fixtures (9; 31 total, original 22 byte-identical)

Generator `tests/fixtures/pdf/generate-pdf-fixtures.py` extended;
`MANIFEST.json` regenerated, 0 hash mismatches on the original 22.

| Fixture | Size | Expected | What it proves |
|---|---|---|---|
| `indirect-resources-ok.pdf` | 990 B | imported | Indirect `/Resources` resolved; Form text extracted |
| `indirect-resources-bomb.pdf` | 20,379 B | Bomb | 20 MB Form via indirect `/Resources` |
| `inherited-resources-ok.pdf` | 950 B | imported | `/Pages`-level `/Resources` inherited |
| `inherited-resources-bomb.pdf` | 20,338 B | Bomb | 20 MB Form via inherited `/Resources` |
| `tounicode-compressed-ok.pdf` | 1,091 B | imported (α) | 256-entry compressed CMap charged, imported |
| `tounicode-compressed-bomb.pdf` | 40,725 B | Bomb | 1.7M-entry CMap, ~20 MB decoded |
| `repeated-form-ok.pdf` | 1,236 B | imported (2 pp) | Same Form once per page, charged per page |
| `repeated-form-bomb.pdf` | 1,873 B | Bomb | ~1 MB Form × 20 invocations ≈ 20 MB |
| `stream-boundary-ok.pdf` | 854 B | imported | Two-block Flate ObjStm, literal `endstream` in stored block |

Generator additions: `flate_two_block` / `flate_split_at_marker`
(a zlib-wrapped Flate stream whose first deflate block is stored
— FlateDecode is zlib, not raw deflate, so the stream carries the
`0x78 0x9C` header and an Adler32; a raw-deflate version was
rejected by lopdf/poppler as "Unknown compression method"), and
an `objstm_compress` hook on `Pdf15.build`. The
`stream-boundary-ok` fixture reserves object 2 for the `/Pages`
tree before any `add_normal` (the same id-collision the `Pdf`
builder's `reserve_page_tree` fixes — the first version collided
the content stream with the tree and pdftotext reported "Weird
page contents"). All five ok-fixtures cross-checked with
`pdftotext`.

## 4. Before / after (per-process, debug build, VmHWM)

Old = extractor tip `a14d6c96b` (worktree
`~/workspace/Anansi-ultimate-before`, measure runner
`crates/core/examples/pdf_measure.rs`, one fixture per process).
New = this repair branch. "Refused" is the error variant; the
old module's `TextCap` refusals came only after the full
uncapped extraction.

| Fixture | Old (a14d6c96) | New (repair) |
|---|---|---|
| `indirect-resources-bomb` | TextCap, 47,883 ms, 65,080 KB | **Bomb, 178 ms, 22,720 KB** |
| `inherited-resources-bomb` | TextCap, 45,823 ms, 64,960 KB | **Bomb, 569 ms, 22,576 KB** |
| `tounicode-compressed-bomb` | **imported**, 67,154 ms, 314,084 KB | **Bomb, 1,282 ms, 22,656 KB** |
| `repeated-form-bomb` | TextCap, 45,715 ms, 45,424 KB | **Bomb, 28 ms, 7,096 KB** |
| `stream-boundary-ok` | Parse (incomplete deflate), 0 ms, 5,700 KB | **imported, 7 ms, 6,704 KB** |
| `indirect-resources-ok` | OK, 5 ms, 6,736 KB | OK, 10 ms, 6,728 KB |
| `inherited-resources-ok` | OK, 5 ms, 6,720 KB | OK, 68 ms, 6,564 KB |
| `tounicode-compressed-ok` | OK, 11 ms, 6,972 KB | OK, 75 ms, 7,048 KB |
| `repeated-form-ok` | OK, 9 ms, 6,780 KB | OK, 11 ms, 6,788 KB |
| `big-page` (1M chars, /tmp only) | TextCap, 2,350 ms, 9,112 KB | **TextCap, 92 ms, 9,072 KB** |

The earlier six bombs are unchanged in outcome (all `Bomb` on
both), with the repaired module at 125–630 ms / ~22 MB peak
(decompression 512 ms, object-stream 630 ms, xref-stream 610 ms,
chained-filter 503 ms, lzw 308 ms, nested-form 591 ms).

The ok-fixture timings differ by a few tens of ms run to run
(debug build, background load); the outcomes and texts are
identical on both.

## 5. Tests and verification (Linux, this VM)

- Module tests: **24/24** (`pdf::`), 8 new
  (`indirect_resources`, `inherited_resources`,
  `compressed_tounicode`, `repeated_forms`, `stream_boundary`,
  `length_resolution_uses_length_not_endstream`,
  `indirect_length_resolves`,
  `unresolvable_indirect_length_is_refused`). Core lib **224**
  (was 216); full workspace lib **289/289** (api 4 + core 224 +
  desktop 1 + providers 60), 0 failed.
- tsc: 0 errors. Contracts: exit 0, 0 drift.
- Docs validator: PASS (294 files, + this handoff).
- The `scanned-image` fixture caught a repair bug during
  development: the first version refused a `Do` to a non-Form
  XObject, but `Do` decompresses any XObject (image data included).
  The pre-scan now charges every invoked XObject, whatever its
  subtype, and scans its decoded bytes for nested `Do`s.
- The design doc's §10 / §3.3 "one trailing newline per page"
  wording is corrected in this branch to "two leading newlines
  per page, no trailing newline" (the correction the extractor
  handoff §9 flagged; the module trims both ends).
- The throwaway measurement runner
  `crates/core/examples/pdf_measure.rs` and the old-code worktree
  `~/workspace/Anansi-ultimate-before` are removed before
  delivery (the runner is not part of the module; the worktree is
  a local verification aid).

## 6. Known limitations (accepted, not gaps)

- The per-page text cap is enforced on the characters
  `PlainTextOutput` writes (text + framing). The stored page text
  after `trim()` is at most the cap; the cap is a bound on the
  extraction, not an exact stored-text count.
- `prescan_fonts` charges a font's CMaps once per page whose
  resources include the font, even if the font is never used for
  text on that page. That over-counts (safe); a document with a
  huge CMap shared across very many pages could be refused where
  the parser would decode it only on the pages that use the font.
- A direct (non-reference) XObject stream is refused explicitly
  (`Unsupported`): it has no object id to charge per invocation
  against. Indirect references are the norm; a direct XObject
  stream is vanishingly rare.
- The structural scan resolves an indirect `/Length` by scanning
  the raw bytes for the integer object, where the loader resolves
  it through the xref. A deliberately lying xref could make the
  two disagree; the scan's measurement is a good-faith bound for
  any well-formed document, and an unresolvable `/Length` is
  refused rather than guessed.
- The discard-sink refinement from the extractor handoff §6
  (stream the content pre-scan into a discard buffer to lower the
  bomb peak toward baseline) is still open; the peak remains
  budget + baseline (~22 MB) by construction.

## 7. Proposed global status updates (for Antigravity; not applied)

None to AN-025 or SPEC_COVERAGE §12 — this is a standalone
module repair, not an integration. The design doc §10 correction
above is applied here (a handoff doc, not a status document).

## 8. Integration and Windows state

- **TXT/Markdown repair: PASSED 9/9 on Windows** (owner report,
  2026-11-06). The two Windows findings from the Slice 1 review
  are closed by that rerun. Antigravity is preparing the merged
  baseline (the accepted integration commit); it has **not** been
  sent yet, so no integration work is started here.
- When the merged commit and bundle arrive, integrate the
  extractor on that baseline (rebase/cherry-pick this repair
  branch): per-kind raw limit in `snapshots::limited` (PDF
  1 MiB), `extract_pdf_text` replacing the `String::from_utf8`
  step for `kind == "pdf"`, `DocumentProvenance.pages` + contract
  regen (optional/defaulted field), `media_type_for("pdf")` →
  `application/pdf`, Documents-view plain-text page navigation,
  Ask `Page N (line M)` via the existing citation fields,
  backup/restore unchanged. Reuse the ingestion job, cancellation,
  restart, and restore paths; verify page provenance through
  search and Ask; keep OCR as a follow-up. That integration is a
  separate slice, delivered with a checksummed bundle for
  independent Windows review.
