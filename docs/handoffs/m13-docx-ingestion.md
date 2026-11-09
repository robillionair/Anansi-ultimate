# M13 DOCX Ingestion — handoff

Branch: `muse/m13-docx-ingestion`, from the expected-target
contract repair tip `af2d4b1646bb7c5a7712efcfd4ee2b95f2e4703d`.
Linux implementation + verification only. Antigravity owns Windows
integration; nothing here is merged, pushed, or promoted.

## Commits

| Commit | Contents |
|---|---|
| `1e0ffa955` | Design + acceptance criteria (AC-D1..D12) before code |
| `747803d59` | Parser spike (zip 9.5.0 + quick-xml 0.42.0, 23 fixtures, PASS) + design amendments §10 |
| `115757019` | Integration: bounded extractor (`docx.rs`), kinds/limits, sections provenance, UI, contracts |
| `409681ff1` | E2E `documents-docx` spec (7 tests); TXT/MD refusals case moves from docx to pptx |
| (this slice) | UI unit tests — DOCX accepted for server-side extraction, pptx refuses |

## Scope

Import an ordinary text-bearing DOCX through the existing
resumable ingestion job and publication guard, preserving heading
and paragraph provenance, searchable and citable through the
existing document and Ask workflows. Design:
`docs/handoffs/m13-docx-ingestion-design.md` (AC-D1..D12,
amendments §10). Spike results:
`docs/handoffs/m13-docx-spike-results.md`.

Out of scope: PPTX, XLSX, OCR, scanned/image-only documents,
headers/footers/footnotes/comments text, list-number expansion,
embedded-object text, preview promotion, pushes, remote CI.
No cloud calls, no paid inference, no model downloads.

## What changed

- `crates/core/src/docx.rs` (new): `extract_docx_text` — bounded
  ZIP walk (`by_index_data` metadata-only walk; encrypted entries
  refused; `word/vbaProject.bin` refused; `w:altChunk` refused;
  `<!DOCTYPE` refused) + streaming quick-xml parse of
  `word/document.xml` only. Caps: raw 1 MiB (caller-enforced),
  entries ≤ 128, single entry ≤ 8 MiB (declared-size pre-check;
  lying sizes refused by the zip crate's read-time size check),
  cumulative inflation ≤ 16 MiB (capped reader, enforced during
  inflation), text ≤ 32,000 chars (mid-paragraph), paragraphs ≤
  10,000. Headings (`Heading1`–3, `Title`, `outlineLvl` 0–2
  fallback) become `#`/`##`/`###` lines; `w:instrText` skipped,
  field results included; `w:del` skipped, `w:ins` included;
  tables as ordered cell paragraphs; drawings/objects skipped;
  external relationships never fetched or recorded; no `\x0c`
  in any body. `DocxSectionProvenance` (plain heading text,
  level, startChar, chars, SHA-256) with contiguous ranges
  covering the body exactly. Empty/whitespace-only documents
  refused (`Empty`), consistent with the PDF no-empty rule.
- `crates/core/src/document.rs`: kind `docx`, raw 1 MiB / text
  32,000 chars, media type
  `application/vnd.openxmlformats-officedocument.wordprocessingml.document`,
  `kind_for_label` accepts `docx`, shape-only raw-hash check
  (value verified at ingestion against the staged file, as for
  PDF), `DocumentProvenance.sections`.
- `crates/core/src/ingestion.rs`: `kind == "docx"` branch in
  `execute_ingestion` (extract after the staged-file length+hash
  gate; refusal fails the job with the extractor's message; no
  partial publish). `jobs.rs` needed no change (per-kind
  `max_bytes_for_kind` + `kind_for_label` already).
- `crates/core/src/memory.rs`: media-type mapping for `docx`.
- `crates/core/src/lib.rs`: `pub mod docx;`.
- UI: `documents.ts` accepts `.docx` for server-side extraction
  (1 MiB cap, raw hash only); `Documents.tsx` accepts the file,
  shows "DOCX — text extracted on the server during import",
  renders the body as Markdown (headings render), shows the
  section heading of the first search match, and lists sections
  in provenance. `contracts/src/client.ts` `ingestDocument`
  size limit is per-kind (`.pdf`/`.docx` → 1 MiB).
- Contracts regenerated: `DocumentProvenance.sections` +
  `DocxSectionProvenance` (doc-comment text only besides the
  field). `context.rs` unchanged — the `#` lines in the body
  give Ask citations `document · # <Heading> (line N)` through
  the existing Markdown-heading chunking, as for Markdown
  documents.
- Fixtures: `tests/fixtures/docx/` (23 deterministic fixtures +
  generator + MANIFEST), including a LibreOffice round-trip
  cross-check (spike output byte-identical to the python-docx
  original).
- Tests updated for the behavior change (DOCX is now supported):
  `document.rs` `wrong_kind_refuses` (docx → pptx),
  `ingestion.rs` `begin_refusals_are_explicit` (docx → pptx),
  `documents-ingestion.spec.ts` refusals case (docx → pptx),
  `documents.test.tsx` refusal case (docx → pptx) + new DOCX
  acceptance test.

## Verification (Linux, this VM)

| Check | Result |
|---|---|
| Spike (23 fixtures, MANIFEST-exact) | 23/23, 0 failures; bomb capped 0 ms / ~4–5 MB vs uncapped 901 ms / ~30 MB |
| Rust lib (`cargo test --locked --workspace --lib`) | 358/358 (was 345; +13: 9 docx module, 1 document, 3 ingestion) |
| Provider-dispatch integration | 22/22 |
| tsc (`pnpm typecheck`) | 0 errors |
| Contracts (`node scripts/contracts.mjs` re-run) | 0 drift |
| Docs (`node scripts/validate-docs.mjs`) | 335 PASS (was 333; +2 new handoff docs) |
| UI (`pnpm test:ui`) | 338/338 (was 337; +1 DOCX acceptance) |
| E2E `documents-docx.spec.ts` | 7/7 (UI import + provenance + Markdown rendering; search names the section; Ask cites `## Manifest (line 5)`; backup/fresh restore preserves sections; 6 refusals publish nothing; cancel; kill/restart/resume once) |
| E2E `documents-ingestion.spec.ts` (TXT/MD) | 9/9 |
| E2E `documents-pdf.spec.ts` | 5/5 |
| E2E `ask_anansi.spec.ts` | 1/1 |

Ask-citation note (measured, not assumed): a heading-only
question term does not rank its chunk — the e2e Ask test asks
with terms from the Manifest section's *content* ("How many
crates of copper fittings are there?"), which cites
`## Manifest (line 5)`. The heading text itself is citable
once its chunk is retrieved; the citation form is
`document · # <Heading> (line N)`, the existing Markdown
behaviour.

## Audit and pre-existing findings (disclosed, not fixed here)

- `cargo audit` on the spike graph: 2 vulnerabilities, 7
  warnings — all pre-existing. `ctutils` 0.4.3 /
  RUSTSEC-2026-0385 (same as the PDF spike). `libsqlite3-sys`
  0.38.2 / RUSTSEC-2026-0459 (C SQLite CVE-2026-119375,
  medium 6.8, fix ≥0.39.0) surfaced in this audit run but is
  pre-existing via `rusqlite` 0.40.2, not introduced by this
  slice (neither zip nor quick-xml depends on it); the fix is
  a dependency upgrade, out of scope, flagged for the owner.
  All 7 warnings identical to the PDF spike's, none new.
  Yanked status unverified (578 registry-timeout errors).
- Node suite (`pnpm test`): 39/40 — the single failure is the
  Windows-only MSVC installer test (`tests/native-setup.test.mjs`),
  which spawns `pwsh`/`powershell`; neither exists on this
  Linux VM. `pnpm test:backup` likewise needs `pwsh`.
  Environmental, pre-existing, unrelated to this slice.
- Known pre-existing failures, unchanged: `providers.spec.ts`
  240 s timeout; `larp-somnia.spec.ts` instruments test;
  `backups.spec.ts` waits for the removed 'Workspace' button;
  4 `semantic_workflow` live-Ollama Rust failures;
  `mcp.rs:715` dead-code warning. None was re-run in this
  slice except where noted; none is a regression.

## Windows rerun checklist (for Antigravity)

```powershell
git fetch <bundle> ; git checkout muse/m13-docx-ingestion
node scripts/cargo.mjs test --locked --workspace --lib
node scripts/cargo.mjs test --locked -p anansi-core --test provider_dispatch
pnpm typecheck
node scripts/contracts.mjs ; git status --short packages/contracts/  # expect clean
node scripts/validate-docs.mjs
pnpm test:ui
node scripts/cargo.mjs build --locked -p anansi-api
cd apps/web ; node node_modules/vite/bin/vite.js build ; cd ../..
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/documents-docx.spec.ts
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/documents-ingestion.spec.ts
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/documents-pdf.spec.ts
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/ask_anansi.spec.ts
```

Expected: lib 358/358, provider-dispatch 22/22, tsc 0,
contracts 0 drift, docs 335 PASS, UI 338/338, DOCX e2e 7/7,
TXT/MD e2e 9/9, PDF e2e 5/5, Ask e2e 1/1. AC-I10 live
rerun stays blocked (no local model/credential on the Linux
VM); the e2e Ask assertions use the offline citation path,
as for PDF.

## Proposed status update (for Antigravity's documents — not edited here)

- `docs/REQUIREMENTS_MATRIX.md` AN-025: stays `in progress`
  (M13 is not complete; OCR remains a separate slice). This
  slice adds DOCX text-layer import to the AN-025 scope
  alongside TXT/Markdown (Windows-closed) and PDF (delivered,
  Windows review outstanding).

## Not done / follow-ups

- No data-descriptor bomb fixture (the capped reader's
  untrusted-size path is proven by its unit check only).
- No committed fixtures for `w:sdt` / `w:smartTag` / `w:tab` /
  `xml:space="preserve"` / drawing skipping (code paths
  present; the spike-results doc §8 lists them).
- `libsqlite3-sys` RUSTSEC-2026-0459 upgrade (out of scope).
