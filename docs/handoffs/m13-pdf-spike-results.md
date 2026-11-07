# M13 PDF Spike — Results

Branch: `muse/m13-pdf-spike`, from the design commit
`27b3acba0c8a4445dca1429bb896f44c98c73109`. Spike only — the harness
is an example binary (`crates/core/examples/pdf_spike.rs`), not
production code. Production integration still starts from
Antigravity's accepted commit once the Windows ingestion suite
passes; if the rerun finds a regression, that repair comes first.

## 1. Resolved versions and advisory coverage

| Crate | Resolved | Source |
|---|---|---|
| pdf-extract | **0.12.1** | `crates/core/Cargo.toml` (`=0.12.1`, exact pin) |
| lopdf | **0.42.0** (single version) | pdf-extract's dependency + direct dev-dep, both pinned `=0.42.0` |
| flate2 | **1.1.10** | lopdf's dependency; backend is **miniz_oxide** (pure Rust, no system zlib) |

- The direct lopdf dev-dep initially resolved to 0.47.0, which would
  have put **two** lopdf versions in the graph (0.42.0 under
  pdf-extract, 0.47.0 direct). That was corrected by pinning the
  direct dep to `=0.42.0`, so the pre-scan and the extractor use
  exactly one parser version. The production design adopts the same
  pinning.
- **RUSTSEC-2026-0187** (stack overflow in lopdf via deeply nested
  PDF objects, CVSS 7.5) is fixed in lopdf 0.42.0. Resolved lopdf is
  exactly 0.42.0, so the advisory is covered at the floor.
- `cargo audit` 0.22.2 (installed for this spike; the first install
  attempt filled the 512M /tmp tmpfs with its own artifacts — cleared,
  rebuilt with the target dir on the home disk) on the spike graph:
  **1 vulnerability, 7 warnings.** The vulnerability is
  **pre-existing, not introduced by this slice**: `ctutils` 0.4.3,
  RUSTSEC-2026-0385 (non-constant-time mask generation), already in
  the baseline lock via `argon2` and `chacha20poly1305`. It is out of
  this slice's scope and flagged for the owner. Of the 7 warnings,
  **1 is new**: `ttf-parser` 0.25.1 is unmaintained
  (RUSTSEC-2026-0192), pulled in by lopdf ← pdf-extract for font
  parsing. It is a warning, not a vulnerability; if a future advisory
  appears against it, the parser choice is revisited. The other 6
  warnings are pre-existing (zvariant, zvariant_derive, zbus_names,
  glib, proc-macro-error, rustls-pemfile).
- Dependency footprint: lock grows 551 → 580 crates (+29, all pure
  Rust). Direct PDF deps: pdf-extract, lopdf, flate2. No C library,
  no system dependency, no subprocess, no network at runtime.
- Regression check: the workspace `--lib` suite still passes
  **265/265** (anansi-api 4, anansi-core 200, desktop lib 1,
  anansi-providers 60), identical to the repair tip — the new
  dependencies introduce no regression.

## 2. Extraction paths measured

Four paths, all against the 12 committed fixtures
(`tests/fixtures/pdf/`, MANIFEST-verified):

- **A — pdf-extract alone** (`extract_text_from_mem_by_pages`).
- **B — bounded pre-scan + pdf-extract.** The pre-scan (lopdf) loads
  the document structure, checks `/Encrypt`, then decodes every page
  content stream (and Form XObject streams) through a capped
  FlateDecode reader with a 16 MiB total decoded budget, aborting
  mid-stream when the budget is exceeded. Only if it passes does
  pdf-extract run.
- **C — lopdf per-page incremental** (`extract_text(&[n])` one page
  at a time, accumulating chars, stopping early at the caps).
- **D — the production-flow prototype:** pre-scan first, then
  per-page extraction with pdf-extract's quality via the public
  `output_doc_page` + `PlainTextOutput` API (exactly what
  `extract_text_by_page` does internally), stopping early at the
  caps.

### 2.1 Outcome table (all 12 fixtures)

| Fixture | A | B | C | D |
|---|---|---|---|---|
| normal-3page | ok 3 pp, 270 chars | ok, prescan 427 decoded B; 3 pp, 270 | ok 3 pp, 262 chars | ok 3 pp, 270 chars |
| unicode-cmap | ok 1 p, 5 chars | ok, prescan 43 B; 1 p, 5 | ok 1 p, 4 chars | ok 1 p, 5 chars |
| malformed-truncated | ERR `failed parsing cross reference table: invalid start value` | REFUSED load (same) | ERR load (same) | ERR load (same) |
| malformed-header | ERR `couldn't parse input: invalid file header` | REFUSED load (same) | ERR load (same) | ERR load (same) |
| malformed-xref | ok **0 pages, 0 chars, no error** | ok, prescan 0 B; 0 pp | ok 0 pp | ERR `parse error: document has no pages` |
| encrypted-user | ERR `decryption error: the supplied password is incorrect` | REFUSED `encrypted: /Encrypt present` | ok **0 pages, 0 chars** | ERR `encrypted: /Encrypt present` |
| oversized-raw | ok 1 p, 27 chars | ok, prescan 70 B; 1 p, 27 | ok 1 p, 25 chars | ok 1 p, 27 chars |
| oversized-text | ok 5 pp, 35,350 chars | ok, prescan 40,325 B; 5 pp, 35,350 | ERR `text cap: exceeded at page 5 (35005 chars)` | ERR `text cap: exceeded at page 5 (35350 chars)` |
| too-many-pages | ok 101 pp, 1,003 chars | ok, prescan 3,932 B; 101 pp | ERR `page cap: 101 pages` | ERR `page cap: 101 pages` |
| scanned-image | ok 1 p, 0 chars | ok, prescan 33 B; 1 p, 0 | ok 1 p, 0 chars | ok 1 p, 0 chars |
| empty-text | ok 1 p, 0 chars | ok, prescan 0 B; 1 p, 0 | ok 1 p, 0 chars | ok 1 p, 0 chars |
| decompression-bomb | ok 1 p, **20,000,002 chars** | REFUSED `decode budget exceeded: stream (4, 0) expands past 16777216 bytes` | ERR `page cap: page 1 has 20000001 chars` | ERR `decode budget exceeded` (same) |

### 2.2 Findings

1. **pdf-extract alone does NOT stop the bomb during decoding.**
   Path A decodes the full 20,000,002 chars (~45 s, ~64 MB peak — §3).
   The design's 16 MiB decoded-stream bound is therefore not
   enforced by pdf-extract; the bounded pre-scan is **mandatory**,
   not optional. Path D (pre-scan first) is the production flow.
2. **The per-page cap check does not bound decoding either.** Path
   C decodes the bomb's full page (20,000,001 chars, 5 s) before
   the page cap fires. The pre-scan is the only decode bound; the
   page/text caps bound the *stored* text and, via per-page
   extraction, the extraction work for multi-page docs (§2.3).
3. **lopdf's own extractor corrupts text.** Path C drops line breaks
   entirely (`Harbor LedgerThe bramblewick shipment…`, 262 vs 270
   chars on the normal fixture) and uses glyph names over ToUnicode:
   on the CMap fixture it returns capitals U+0391/U+0392 instead of
   the mapped lowercase U+03B1/U+03B2 (path A returns
   U+000A U+000A U+03B1 U+03B2 U+20AC — correct, and exactly the
   manifest text after trimming). lopdf is therefore used **only
   for structure** (page count, content-stream enumeration for the
   pre-scan); pdf-extract is the sole text extractor. This is why
   the production needs both crates, not lopdf alone.
4. **0 pages with no error is a trap.** The malformed-xref fixture
   (xref offsets zeroed + trailer `/Root` broken) and the encrypted
   fixture both yield 0 pages and NO error from lopdf/pdf-extract.
   A worker that treats "0 pages" as "empty" would misclassify both
   (a broken document as empty, an encrypted document as empty).
   The worker must check, in order: `/Encrypt` present → encrypted
   refusal; load error → parse error; **0 pages → parse error**
   (not empty); then per-page extraction + caps; empty total →
   empty / no-text-layer refusal. Path D implements this ordering
   and is the only path that refuses both correctly.
5. **The bad-header fixture stands.** lopdf refuses it
   (`invalid file header`) on all paths. Poppler's recovery of this
   input is poppler's leniency, not a fixture defect. Design spike
   item §2.3.5 is closed.
6. **Empty-password encrypted PDFs are unencrypted in practice.**
   A /tmp-only R=2 document with an empty user password extracts
   normally under path A (41 chars) and passes the pre-scan: lopdf
   strips `/Encrypt` after a successful empty-password decryption.
   The design is amended to say so (AC-PDF-03 covers user-password
   documents; empty-password documents import normally — they need
   no credential).
7. **The pre-scan does not decode non-content streams.** On
   `oversized-raw.pdf` it decoded 70 bytes (the page's content
   stream); the 1.1 MB unreferenced stream was never touched. The
   raw cap, not the pre-scan, handles that case. Correct scope.
8. **pdf-extract's page text is exactly the manifest text after
   trimming.** Path A adds one trailing `\n` per page (270 vs 267
   manifest chars over 3 pages); after the worker's per-page trim,
   pages equal the manifest exactly. The oversized-text total is
   35,350 chars (manifest 35,360; the 10-char difference is the
   same trailing-newline accounting, 2 per page).

## 3. Bomb: peak memory and elapsed time

Per-process runs (one fixture per process; peak is Linux VmHWM).

| Case | Path | Result | Elapsed | Peak RSS |
|---|---|---|---|---|
| Fixture bomb (20 MB expansion) | A pdf-extract alone | decoded 20,000,002 chars | **45,463 ms** | **65,448 KB (~64 MB)** |
| Fixture bomb | B pre-scan + pdf-extract | refused at 16 MiB budget | **187 ms** | **7,168 KB (~7 MB)** |
| Fixture bomb | C lopdf per-page | decoded 20,000,001 chars, then page-cap error | 4,993 ms | 65,600 KB |
| Fixture bomb | D production flow | refused at 16 MiB budget | **169 ms** | **7,064 KB** |
| Worst bomb (1 GiB expansion, /tmp-only synthetic: 1,042,921-byte FlateDecode stream, under the 1 MiB raw cap) | B | refused at 16 MiB budget | **163 ms** | **11,656 KB** |
| Worst bomb (1 GiB) | D production flow | refused at 16 MiB budget | **182 ms** | **11,496 KB** |

- Path A was deliberately **not** run on the 1 GiB bomb (it would
  allocate ~1 GiB and take ~35 minutes by the fixture's rate).
  The fixture bomb already proves path A does not bound decoding.
- The pre-scan stops a 1 GiB expansion at the 16 MiB budget in
  ~160–180 ms with ~11 MB peak — the bound holds at the worst case the
  raw cap admits, not just at the fixture size.
- pdf-extract's per-character processing is slow on huge pages
  (~45 s for 20M chars). The pre-scan + caps keep the worker fast:
  every in-cap fixture extracts in ≤ 13 ms.

## 4. Page and text limits bounding work incrementally

- **Page cap precedes extraction.** `too-many-pages.pdf`
  (101 pages): path C and D refuse with `page cap: 101 pages`
  before extracting any text. The page count comes from the
  structure load, so no text work happens for over-cap documents.
- **Text cap stops multi-page extraction early.** A /tmp-only
  20-page document (7,000 chars/page, 141,400 total): path A
  processes all 20 pages (430 ms); path C and D stop at page 5
  with `text cap: exceeded at page 5 (35350 chars)` — path D in
  **115 ms**, ~4× faster, having never touched pages 6–20. The
  `oversized-text.pdf` fixture (5 pages) trips the same check at
  page 5; it saves no work there (the cap is crossed on the last
  page), but the mechanism is identical.
- **Per-page quality is preserved.** Path D's output equals path
  A's exactly on the normal (270 chars) and unicode (5 chars)
  fixtures, and its text-cap trip count (35,350) matches path A's —
  unlike path C, whose dropped newlines undercount by 345 chars.
  Early termination therefore costs no text quality.
- **Page provenance construction verified.** For the normal
  fixture, the design §3.3 body (pages joined by `\n\x0c\n`) is
  276 chars; the per-page ranges are contiguous, in order, and
  cover the body exactly (page 1 at 0/114 chars, page 2 at 117/92,
  page 3 at 212/64). (The spike's page hashes are a stand-in
  hasher for the range check; the production uses the existing
  `content::hash` SHA-256, already proven for text hashes.)

## 5. Design corrections (applied to the design doc)

The design doc (`docs/handoffs/m13-pdf-ingestion-design.md`) is
amended in §10; the corrections are:

1. **Storage cap vs Ask retrieval budget (§3.1, §3.4).** The design
   justified the 32,000-char document cap as "a max-size PDF can
   still be fully used as Ask evidence" because it equals
   `MAX_ALLOWED_BUDGET_CHARS`. That is wrong. Ask's retrieval
   budget is a **per-query** budget (default 4,000 chars, max
   32,000 only if the caller requests it), shared across **all**
   evidence, selecting top-ranked 1,000-char chunks. A 32,000-char
   document is not fully retrieved by default — only ~4,000 chars
   of its top-ranked chunks are used, fewer if other evidence
   competes. The 32,000-char storage cap is now justified on its
   own grounds (worker time, memory, FTS index size, UI rendering,
   4× the TXT cap for multi-page documents). The Ask side is
   stated separately: a document contributes at most a few
   1,000-char chunks per answer, and the page-level citation
   points at the chunk's page.
2. **Pre-scan is mandatory (§2.2, §2.3.4, §3.2).** pdf-extract
   alone does not bound the bomb; the bounded pre-scan must run
   before pdf-extract on every PDF. Path D is the production
   flow. lopdf becomes a production dependency (pinned `=0.42.0`,
   the version pdf-extract resolves, so the graph holds exactly
   one lopdf).
3. **Per-page pdf-extract is the extraction path (§3.2).** The
   public `output_doc_page` + `PlainTextOutput` API gives
   per-page extraction with identical output to by-pages, enabling
   early termination at the text cap with no quality loss.
4. **Worker check ordering (§3.2, AC-PDF-04).** `/Encrypt` →
   encrypted; load error → parse error; 0 pages → parse error
   (not empty); then caps; empty total → empty/no-text-layer.
   The malformed-xref fixture's expected outcome is corrected to
   "refused: parse error (0 pages — the parser reports no error;
   the worker maps 0 pages to a parse failure)".
5. **Empty-password documents (§3.2, AC-PDF-03).** Documents that
   open with an empty password are treated as unencrypted and
   import normally.
6. **lopdf is structure-only (§2.2, §3.2).** Its own extractor
   drops line breaks and ignores ToUnicode (glyph names win), so
   it must never supply text; pdf-extract is the sole text
   extractor.
7. **Pre-scan scope and gaps (§3.2).** Decodes page content
   streams + Form XObject streams (one level; nested forms are a
   documented integration gap). Filter chains are refused in the
   prototype (gap). Non-Flate expanding filters (LZW) are refused
   as unsupported in the prototype (gap — integration adds a
   capped LZW path via `weezl`, already in the graph, or refuses
   LZW content streams; LZW in text content streams is rare).
   Non-content streams are never decoded (raw cap's job). Object
   streams (PDF 1.5+ compressed objects) are decoded by lopdf at
   load, outside the pre-scan's budget — residual risk, same class
   as RUSTSEC-2026-0187, mitigated by the version floor + raw cap;
   a dedicated object-stream bomb fixture is follow-up work.
8. **Audit findings (§2.2).** ctutils/RUSTSEC-2026-0385 is
   pre-existing (flagged for the owner, out of scope);
   ttf-parser unmaintained is new (warning only, revisit if an
   advisory appears).

## 6. Spike-gate outcomes (design §2.3)

1. by-pages returns the manifest text exactly (modulo one trailing
   newline per page, removed by the worker's trim) — **PASS**.
2. Error (not empty, not panic) for truncated / xref /
   encrypted — **PARTIAL**: truncated and encrypted error ✓;
   xref yields 0 pages with no error — corrected by the worker's
   0-pages→parse-error mapping (correction 4).
3. Empty/whitespace for scanned + empty — **PASS** (0 chars).
4. Bomb bounded — **FAIL for pdf-extract alone; PASS with the
   pre-scan** (fixture ~190 ms/~7 MB; worst case ~160–180 ms/~11 MB).
   Pre-scan is now mandatory (correction 2).
5. Malformed-header refused by lopdf — **PASS** (fixture stands).

## 7. What the spike does not do

- No production code: the harness is an example binary on this
  branch. The `Cargo.toml`/`Cargo.lock` changes on this branch are
  spike evidence (the exact resolved versions); integration
  re-applies the dependency additions on Antigravity's accepted
  commit.
- No Windows run: the spike ran on this Linux VM only. The
  pure-Rust dependency graph (no system libs, no subprocess) is
  the Windows-compatibility evidence; the Windows rerun of the
  ingestion suite is still the gate for integration.
- No object-stream bomb fixture, no nested-Form-XObject walk, no
  LZW capped decode, no filter-chain handling — recorded as gaps
  in §5.7 for integration.
- The worst-case bomb (1 GiB) and the 20-page early-termination
  document and the empty-password document were /tmp-only
  synthetics, not committed fixtures. The committed 12 fixtures
  are unchanged.
