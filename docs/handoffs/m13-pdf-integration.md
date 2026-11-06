# M13 PDF Integration — handoff

## 1. Scope

Integrate the bounded PDF text extractor (standalone module + hardening
repair) into the M13 document-ingestion workflow on Antigravity's merged
baseline, so a PDF with a text layer imports through the same upload, job,
cancellation, restart and restore paths as TXT/Markdown.

- Baseline: `3bf51af856e65adf2000f631dcea8390dedc3b5b`
  (`codex/windows-pilot-candidate`, merge of the Windows readiness closure
  `5bb5a7cbf678f62818becfe1bd9de273be084b03` and the TXT/Markdown repair
  `dd0423ced7761386db1ee2cc3f0bc19339f3b19d`; the merged baseline passed
  all nine Windows ingestion tests).
- Extractor + hardening brought in from `muse/m13-pdf-extractor-repair`
  tip `b340f916aaf1b2bb45ca03d5958826c04c14b478`, merged with zero
  conflicts (merge commit `d40b90b5f`).
- Branch: `muse/m13-pdf-integration`. This commit is the integration
  on top of `d40b90b5f`.
- OCR stays a follow-up slice. DOCX stays refused. No production
  execution, no paid inference, no model downloads.

## 2. What was wired in

### Per-kind limits

- `crates/core/src/document.rs` gains `max_bytes_for_kind(kind)` and
  `max_chars_for_kind(kind)`: pdf → 1 MiB raw / 32,000 chars, else the
  existing 65,536 B / 8,000 chars. `kind_for_label` accepts `"pdf"`;
  the error message now lists `.txt`/`.md`/`.pdf`.
- `media_type_for` maps `"pdf"` → `"application/pdf"`.
- `DocumentProvenance` gains `pages:
  Option<Vec<PdfPageProvenance>>` (`serde(default,
  skip_serializing_if = "Option::is_none")`). Contract regenerated
  (`DocumentProvenance.pages?: PdfPageProvenance[] | null`).

### Extraction branch

- `crates/core/src/ingestion.rs` `execute_ingestion`: after the
  staged-file length + SHA-256 gate, kind `"pdf"` calls
  `crate::pdf::extract_pdf_text(&bytes)`. On error the job fails with
  the extractor's user-facing message (`ExtractError::message()`,
  new in `crates/core/src/pdf.rs`); on success the body is the
  joined page text, `bom = false`, and the published provenance
  carries the page list. All decode and text limits (16 MiB cumulative
  decode budget, 100 pages, 8,000 chars/page, 32,000 total) are
  enforced inside the extractor module, unchanged.
- `enqueue_ingestion` and the ingest-part finish read the staged file
  with the per-kind limit (`snapshots::limited(&path,
  max_bytes_for_kind(&kind))`).

### A real gap found and fixed in this slice

- `crates/core/src/jobs.rs` `ExportSelection::Ingestion::validate`
  checked `expected_bytes` against the 64 KiB `MAX_DOCUMENT_BYTES`.
  That validation runs inside `enqueue_ingestion`, so a PDF over
  64 KiB would have been refused at `ingest-begin` even though the
  per-kind limit allows 1 MiB. Fixed to
  `crate::document::max_bytes_for_kind(format)`. New test
  `pdf_begin_uses_the_pdf_raw_limit` proves a 100,000-byte PDF is
  accepted at begin and a 1,048,577-byte PDF is refused.

### Page-aware Ask chunking

- `crates/core/src/context.rs` `chunk_markdown_sections`: if the body
  contains `\x0c`, it is chunked per page. A line exactly `"\x0c"` is
  a page break; each page's chunks get
  `section_heading = Some("Page {page}")` and a line number counted
  within the page from 1. `#` lines are plain text in paginated
  bodies. Non-paginated bodies keep the existing Markdown-heading
  logic unchanged. No contract or UI change was needed: the Ask UI
  already renders `{kind} · {sectionHeading} (line {lineNumber})`,
  so a citation reads `document · Page 2 (line 1)`.

### UI

- `packages/ui/src/documents.ts`: `prepareDocument` accepts `.pdf`
  (kind `pdf`, no client-side decode — the binary goes to the server;
  `text: ''`, `textChars: 0`, `rawHash` = SHA-256 of the raw bytes).
  Refuses 0 or > 1 MiB. `uploadDocument`/`resumeUpload` unchanged
  (they use `bytes` + `rawHash` only), so upload resume works for PDF.
- `packages/ui/src/Documents.tsx`: file input accepts
  `.txt,.md,.pdf`; review shows "PDF — text extracted on the server
  during import" for a PDF pick. Search computes the page of the
  first match client-side and shows "· Page N". New `PdfReader`
  component splits the body on `'\x0c'` and shows one page at a time
  ("Page N of M", Previous/Next). PDF documents never go through
  `renderMarkdown` and have no "View source" toggle.
- `packages/contracts/src/client.ts`: `ingestDocument` size limit is
  per-kind (1 MiB for `.pdf`, 64 KiB otherwise).

## 3. The PDF raw-hash verification note

For TXT/Markdown, `validate_document_source` recomputes the raw hash
from the decoded text, so the stored `raw_hash` is fully verified.
For PDF that is impossible: the binary PDF is not recoverable from
the extracted text, so `validate_document_source` checks the PDF raw
hash for shape only (64 lowercase hex). The value itself is verified
at ingestion: `execute_ingestion` re-reads the staged file (the
ground truth), checks its length and SHA-256 against the job's
declared values, and only then extracts and publishes, with the
staged file's hash stored as `rawHash`. A forged provenance with a
wrong raw hash therefore cannot reach the store through the
ingestion path; `validate_import` in `memory.rs` additionally
requires `media_type` `application/pdf` to map to kind `pdf`.

## 4. Verification (Linux only — no Windows claim)

- Rust workspace lib: **297 passed, 0 failed**
  (api 4 + core 232 + desktop 1 + providers 60). Core grew from 224
  (repair tip) by 8: document +2, context +3, ingestion +3
  (happy-path provenance, refusals, begin-limit).
- `cargo check -p anansi-core`: clean except the pre-existing
  `mcp.rs:715` `preserved_env_entries` warning. No new warnings.
- TypeScript: **0 errors**. Contracts `--check`: **0 drift**
  (only the benign "unknown format uint" warnings). Docs validator:
  **312 PASS** (311 before this handoff).
- UI tests: **321/321** (was 320; +1 PDF `prepareDocument` test).
- API build: exit 0. Web (Vite) build: exit 0.
- New PDF e2e `tests/e2e/documents-pdf.spec.ts`: **5/5** (23 s).
  (1) UI import of `normal-3page.pdf` — page navigation "Page 1 of
  3" → 2 → 3 with the right page texts, Previous works, Next disabled
  on the last page, no "View source" button, provenance shows
  `application/pdf` + "3 pages" + per-page char counts.
  (2) Search names the page — "aurora" → Page 2, "lighthouse" →
  Page 3, "bramblewick" → Page 1.
  (3) Ask (closed-port offline path) — citation title
  `normal-3page.pdf`, citation meta contains "Page 2 (line 1)".
  (4) Backup + fresh restore via API — restored workspace has the
  document with mediaType `application/pdf`, rawHash == fixture SHA,
  `pages` length 3 with page numbers [1,2,3], body contains all
  three pages, search for "aurora" finds it.
  (5) Refused PDFs via API — encrypted / scanned / bomb / malformed
  jobs all Failed with the expected log substrings, documents list
  length 0.
- Existing TXT/Markdown e2e `tests/e2e/documents-ingestion.spec.ts`:
  **9/9**. Two tests were updated for the new format, assertions
  preserved: the markdown test now uses a 4-byte `%PDF` pick (accepted,
  then "Import failed: PDF could not be parsed", list still length 1);
  the refusals test now uses `report.docx` for the unsupported-format
  case (PDF is supported).
- Ask e2e `tests/e2e/ask_anansi.spec.ts`: **1/1**.

### Pre-existing failure, not a regression

- `tests/e2e/backups.spec.ts` **fails on this branch** (20 min
  timeout waiting for a button named exactly "Workspace" in
  `scripts/backup-workflow.mjs` `restoreBackup`). Root cause
  established by inspection, not by this slice: HOME Command
  Dashboard Slice 1 (commit `1523016e9`, 2026-11-03) removed the
  "Workspace" nav button, replacing it with "Dashboard". No button
  with the accessible name "Workspace" exists in the baseline
  `3bf51af8` UI either (`git grep ">Workspace<"` on the baseline
  returns nothing; this slice does not touch `Navigation.tsx` or
  `Workspace.tsx`). The restore itself completed in the run
  ("Restore complete:" status appeared); only the test's navigation
  step cannot match. Out of scope for this slice; fixing the shared
  helper is a separate repair. The backup/restore requirement for
  PDF is covered by the passing PDF e2e test (4) above.

### Environment note

During verification the VM's `/tmp` tmpfs was cleared mid-run
(2026-11-06 ~03:47 CET), killing a background e2e process and
deleting the `/tmp` logs. A stale application-test lease was left
behind; the owner pid was verified dead and the lease removed before
re-running. Evidence logs for this delivery are kept under
`~/workspace/deliveries/anansi-m13-pdf-integration/evidence/`
(not `/tmp`) so they survive.

## 5. Known limitations (carried from the extractor + repair)

- Font ToUnicode/Encoding CMaps are charged even when the font is
  unused for text on a page (safe over-count — a huge shared CMap
  across many pages could be falsely refused).
- A direct (non-reference) XObject stream is refused (no id to
  charge per invocation). Form reference cycles are refused.
- The structural scan resolves indirect `/Length` by raw-byte scan,
  not via the xref.
- The discard-sink refinement for the content pre-scan is still open
  (an optimisation, not a correctness gap): bomb fixtures peak at
  ~22 MB RSS in debug builds because the pre-scan retains decoded
  bytes it must parse.
- A PDF whose text layer is empty (scanned / image-only) is refused
  honestly with "no readable text … OCR is a separate slice".

## 6. Proposed global-status updates (for Antigravity — NOT applied)

- `docs/REQUIREMENTS_MATRIX.md`: AN-025 → `in progress`, scoped to
  the TXT/Markdown foundation plus the PDF text layer. Do NOT mark
  `verified`; M13 is not complete (OCR and further formats remain).
- `docs/SPEC_COVERAGE.md`: §12 stays `planned`, with a note that the
  TXT/Markdown + PDF text-layer foundation is implemented and
  Linux-verified, pending Windows review.
- `docs/BUILD_STATUS.md`: record the PDF integration branch, tip and
  Linux evidence; note the pre-existing `backups.spec.ts` failure
  (§4) so the Windows reviewer does not mistake it for a PDF
  regression.

## 7. Windows acceptance checklist

- [ ] `tests/e2e/documents-ingestion.spec.ts` — 9/9 (TXT/Markdown,
      incl. restart/resume and backup/restore).
- [ ] `tests/e2e/documents-pdf.spec.ts` — 5/5 (import + page
      navigation, search page, Ask "Page N (line M)" citation,
      backup/fresh restore, refusals publish nothing).
- [ ] `tests/e2e/ask_anansi.spec.ts` — 1/1.
- [ ] Rust workspace lib, tsc, contracts, docs validator, UI tests
      as §4.
- [ ] Confirm a PDF over 64 KiB and up to 1 MiB begins and imports
      (the `jobs.rs` per-kind fix, §2).
- [ ] Confirm the pre-existing `backups.spec.ts` failure (§4) is
      unchanged from the baseline and is not treated as a PDF
      regression.
