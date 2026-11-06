# M13 Document Ingestion — Slice 1 closure addendum (TXT/Markdown)

Owner-tasked bounded slice from the 2026-11-05 coordination package.
Baseline: `7e24859e6c042aaa135d632ad7009d20ab3c35e1`
(`codex/coordination-progress-2026-11-05`). Branch:
`muse/m13-document-ingestion`, tip `5a48a0ed174620d7652128ac265e5f6440c75ded`.

Design committed first: `70ebe4665` (`docs/handoffs/m13-document-ingestion-design.md`,
AC-ING-01..18, findings F-ING-01..03, §8 next format: PDF text extraction).
Implementation: `5a48a0ed1` (this addendum is a separate docs-only commit
on top; the slice's evidence is recorded here, not in Antigravity-owned
global status documents).

**M13 is NOT declared complete.** This slice delivers one complete
TXT/Markdown workflow only. AN-025 stays `planned` in the matrix (a
proposed `in progress` scoped to the TXT/MD foundation is listed in §7
below for Antigravity). No macOS work was started. The LARP showcase
film was already delivered 2026-11-05 and was NOT restarted.

## 1. What was built

One complete workflow, reusing existing identities, persistent jobs,
storage protections, search, provenance and backup/restore — no new
command group, no schema migration (SCHEMA_VERSION stays 11), no new
dependencies, no parallel storage/job system, no Collider beam-text
loader duplication.

- Select file → review and approve import → bounded chunked upload
  (`ingest-begin` / `ingest-part`, 64 KiB parts, offset-exact,
  sequential) → persistent job progress → restart/resume → open and
  search imported content → Ask evidence/disclosure → backup/fresh
  restore/reopen.
- Strict rules (shared `crates/core/src/document.rs`): raw file
  1–65,536 bytes; decoded text 1–8,000 chars, non-whitespace; strict
  UTF-8 (invalid refuses, never replacement chars); one leading BOM
  stripped and recorded; label basename-only 1–255 chars with
  extension matching kind (`.txt` / `.md`); SHA-256 of raw bytes and
  of decoded text recomputed server-side, never trusted from the
  client.
- Refusals are explicit and honest: oversize, invalid UTF-8, Sealed
  (no encrypted path exists yet), unsupported formats (PDF/DOCX/image/
  audio — "tracked as follow-up work"), hash mismatch, wrong project,
  offset mismatch.
- Provenance: `MemoryMetadata.document` (`DocumentProvenance`:
  fileName, mediaType, rawHash, textHash, rawBytes, textChars, bom)
  bound by `document_import`; `memory_write` re-validates provenance
  against the body and refuses forged provenance (`InvalidMemory`).
  Provenance/document are refused on revise.
- Atomic publish guard (`JobPublishGuard`): the worker verifies the
  staged file (length + SHA-256) and publishes inside
  `memory_write`'s transaction with an in-transaction duplicate check
  (same title + text hash → returns the existing record, publishes
  nothing). Concurrent duplicate imports create exactly one record.
- Cancellation/interruption: cancel during upload deletes staging
  best-effort and publishes nothing; a killed worker's lease expires
  via `recover_jobs` (Running → Paused, "Worker interrupted; resume
  explicitly"); Resume is refused while the upload is incomplete
  ("resume the upload instead of the job") and allowed once the
  upload is staged; Failed jobs keep staging for Retry.
- Privacy: Private by default, Sealed refused; workspace/project
  binding explicit; staged files are job-owned (`.upload` in the
  workspace export directory) and the upload-begin sweep deletes only
  stale job staging, never owner files.
- Search/Ask: imported content is found through the existing
  full-body FTS path (`fullBody: true`); the Documents view search
  uses it. A plain list query searches titles only — that is the
  pre-existing `memories/list` contract, not an ingestion defect.
  Ask cites the imported document; a missing topic answers with
  honest insufficient-evidence (offline/disclosure tests only — no
  model download, no cloud benchmark).
- UI: new `Documents` view in the KNOWLEDGE group (nav 17 → 18);
  inert Markdown renderer (React elements only, no
  `dangerouslySetInnerHTML`, no anchors — links render as plain text
  "text (url)", raw HTML renders as literal text); provenance
  `<details>`; Ask/Jobs navigation from the reader.

## 2. Design deviations and corrections (all disclosed)

1. **Worker ground truth is the staged file, not the checkpoint.**
   Design §3.2 step 1 said the worker checks the checkpoint is
   `staged`. `claim_export` resets the checkpoint to `snapshot` on
   claim, so that check was wrong (caught by the first publish-path
   test failure). The worker now re-verifies the staged file
   (length + SHA-256) and `claim_export` preserves the ingestion
   checkpoint and progress (CASE on `json_extract(selection,
   '$.kind')='ingestion'`), so an interrupted publish keeps
   checkpoint `staged` and can Resume. The design doc is superseded
   on this point by this addendum.
2. **The `ColliderDocumentSource` struct did NOT move to
   `document.rs`.** Its name is part of the generated contracts, so
   it stays in `collider.rs`; `collider::validate_document_source`
   became a delegation wrapper over `document::validate_document_source`
   and the collider's local `MAX_DOCUMENT_BYTES`/`MAX_LABEL_CHARS`
   constants were removed (`MAX_BEAM_CHARS` kept). Behavior identical;
   collider tests untouched.
3. **Forged-provenance refusal is `InvalidMemory`, not
   `InvalidRequest`.** `memory.rs`'s `invalid()` helper returns
   `InvalidMemory`; the test initially asserted `InvalidRequest` and
   was corrected to the code's actual behavior.
4. **Documents search uses the full-body FTS path.** The design
   assumed the Documents search would find imported content through
   the normal list query; the normal list query is title-only
   (`title_search` indexes titles in an in-memory FTS table). The
   UI search and the e2e helper pass `fullBody: true` (body_search
   over `memory_body_fts`, kind-filtered). No server change.

## 3. In-slice incidents (all repaired and re-verified)

1. **Edit-tool line drops (3×).** Inserting code swallowed a
   following line in `memory.rs` `metadata()` (first body line),
   `content.rs` `read()` (first body line), and the client
   `enqueueBackup` line, and once swallowed the
   `ingestion_operation` signature lines. Each was restored exactly
   and confirmed via `git diff`; the final diff was reviewed hunk by
   hunk before commit.
2. **Background-worker race in Rust tests.** `ingest-part` finish
   wakes the real export worker thread, which raced manual claims in
   tests. Tests now poll for terminal state (`wait_for_terminal`);
   tests that must own the claim fabricate the staged/Queued state
   via `stage_for_test` (SQL + staged file) instead of racing the
   worker.
3. **Stale-closure bug in `Documents.tsx`.** The 4s poll interval
   captured the first render's `refreshJobs`, where `activeJob` was
   `null`, so the "Document imported" notice never fired. The
   in-flight job id now lives in a ref (`activeJobIdRef`), set on
   import/resume and cleared on terminal states. Found by the UI e2e
   tests, fixed, re-run green.
4. **Harness cleanup hang (restart test).** The test timed out at
   300s with the body complete. Two latent harness defects, both in
   `tests/e2e/collider-three-helpers.ts`:
   - `cleanup` awaited `child.once('exit')` when
     `child.exitCode === null`; a signal-killed child keeps
     `exitCode === null` after it exits, so cleanup waited on an
     event that had already fired. Now checks
     `exitCode === null && signalCode === null` and races the wait
     with a 10s SIGKILL fallback.
   - `fake.close()` waits for idle keep-alive connections from the
     browser; `closeAllConnections()` is now called first.
   Also exported the `Session` type (the spec imports it; `tsc`
   failed until then).
5. **Test-only corrections.** The restore test read the workspace id
   from `restored.body.workspace?.id`; the real shape is
   `RestoreResult.workspaces[0].id`. The Markdown-inertness
   assertion `page.locator('script')` counts the app's own script
   tags; it is now scoped to `.documents` plus a literal-text
   assertion. The UI search assertion was scoped to the search
   section (the main list used to false-pass it).
6. **Concurrent-duplicate begin flake (once).** One of two
   concurrent `ingest-begin` calls returned non-200 in the first
   full-spec run; the same test passed alone and in the re-run
   battery. No code change was needed and no cause was established;
   recorded here as an unreproduced flake, not claimed fixed.

## 4. Verification evidence (Linux only — Windows rerun outstanding)

- TypeScript: `npm run typecheck` 0 errors.
- Contracts: `node scripts/contracts.mjs --check` exit 0 (generated
  TS + schemas regenerated for `IngestionBegin`, `IngestionPart`,
  `IngestionStatus`, `DocumentSource`, `DocumentProvenance`).
- Docs: `node scripts/validate-docs.mjs` 288 files PASS (287
  baseline + design doc; this addendum makes 289 — re-verify at
  packaging).
- UI suite: `npm run test:ui` 320/320 PASS (314 baseline + 6 new
  documents tests).
- Rust unit: ingestion tests 11/11 (`happy_path_publishes_document_
  with_provenance`, `duplicate_import_completes_with_the_same_record`,
  `finish_hash_mismatch_fails_and_discards_staging`,
  `part_offset_mismatch_conflicts`,
  `invalid_utf8_fails_without_publishing`,
  `cancel_during_upload_removes_staging`,
  `resume_refused_until_upload_finishes`,
  `begin_refusals_are_explicit`,
  `forged_provenance_refused_by_memory_write`,
  `stale_lease_recovers_to_paused_then_resumes_once`,
  `cancelled_job_publishes_nothing`); document.rs 4/4.
- Rust workspace `--lib`: 262/262 PASS (247 at MCP Slice 5 + 15 new).
- Full Rust workspace suite: 663 passed, 4 failed — the 4 are the
  pre-existing `semantic_workflow` live-Ollama environmental
  failures (embedding model absent; Ollama stopped), identical to
  the standing list; nothing new failed.
- API build: clean, 1 pre-existing Windows-only warning
  (`preserved_env_entries` unused on Linux, `crates/core/src/mcp.rs:715`).
- Web build: Vite clean.
- E2E `tests/e2e/documents-ingestion.spec.ts`: 9/9 PASS in 29.2s
  (UI import/provenance/search; Markdown inert + PDF refusal;
  invalid UTF-8; oversize/Sealed/hash-mismatch/unsupported refusals;
  cancel; concurrent duplicates → 1 record; kill/restart/resume →
  exactly 1 record; backup/fresh restore preserves the import with
  search; Ask citation + honest insufficient-evidence).
- Full regression battery: **67 passed, 1 failed, across 25 specs**
  (the MCP Slice 5 battery of 24 + `documents-ingestion.spec.ts`;
  7.9m under the shared lease). Per-spec: product-lifecycle-slice1
  2/2, slice2 2/2, slice3 2/2, larp-mode-slice1 2/2, slice2 3/3,
  slice3 3/3, slice4 4/4, ghost-prompt-slice1 2/2, slice2 2/2,
  slice3 2/2, somnia-slice1 2/2, slice2 2/2, slice3 2/2,
  swarmtank-scenario 3/3, swarmtank-arena 3/3, swarmtank-slice1 1/1,
  evaluations 1/1, integrated_workflow 1/1, opportunities 1/1,
  dashboard 5/6, mcp 6/6, mcp-server 2/2, mcp-slice4 3/3,
  mcp-slice5 2/2, documents-ingestion 9/9. The single failure is
  **pre-existing, not a regression**: `dashboard.spec.ts` "quick
  actions jump to Ask, LARP, and project creation" fails identically
  (5 passed, 1 failed, same test) on the PRISTINE baseline
  `7e24859e` in a scratch worktree — the LARP toggle mounts the
  `LarpHud` overlay, which covers the Dashboard's Quick actions
  region, so the `+ New Project` click times out waiting for an
  actionable element. This slice touches neither `Dashboard.tsx`
  nor `larp-mode/` (verified: no diff in either). The MCP Slice 5
  battery was 59/59 because it ran before the cinematic LARP
  upgrade merge; the merge introduced this failure. Fixing it is
  out of scope for this slice (it needs a Dashboard/LARP owner
  decision: dismiss the HUD before the next quick action, or
  navigate back to the Dashboard first); it is recorded here and
  in §5 so the Windows reviewer can confirm the same behavior.

### Pre-existing strict-check failures (NOT introduced by this slice)

- `cargo clippy --all-targets -- -D warnings` fails on 4
  pre-existing lints this slice did not touch: `mcp.rs:715`
  (`preserved_env_entries` dead on Linux, Windows-only),
  `mcp.rs:1225`, `mcp_server.rs:584`, `mcp_server.rs:987`
  (collapsible `if`s). This slice's own 5 clippy errors
  (ingestion.rs:171, jobs.rs:358-359, memory.rs:734,
  collider.rs:529, ingestion.rs:430) were fixed; plain `cargo
  clippy` (warnings tolerated) is unaffected. The fixes were kept
  out of `mcp.rs`/`mcp_server.rs` deliberately — no unrelated
  reflow in Windows-verified code.
- `cargo fmt --check` fails on the PRISTINE baseline under this
  toolchain (57 diffs, verified in a scratch worktree at
  `7e24859e`): the local rustfmt 1.99 applies style-edition 2024
  defaults to these edition-2024 crates and disagrees with the
  baseline formatting; there is no `rustfmt.toml`. This slice's
  files were formatted with the local rustfmt where that did not
  create gratuitous reflow; a bulk `cargo fmt` was NOT run, and a
  stray rustfmt pass that reflowed `mcp.rs` (177 lines) and
  `mcp_server.rs` (248 lines) was reverted — `mcp.rs` is untouched
  in the final diff and `mcp_server.rs` carries exactly 1 line
  (`document: None` in the FakeBackend test helper).

## 5. Limitations (stated, not hidden)

- TXT/Markdown only. PDF, DOCX, PPTX, XLSX, HTML, EML, images,
  audio/video transcripts remain follow-ups (§8 next slice: PDF
  text extraction, same job/upload/provenance shape, page-aware
  evidence; DOCX after PDF).
- The 64 KiB raw cap means every file in this slice is a single
  upload part; multi-part resume is proven from offset 0 and from a
  staged checkpoint, not from a mid-file offset (a larger cap in a
  later slice exercises that path).
- Multi-process same-instant duplicate import is mitigated by the
  in-transaction guard; the cheap pre-check can still race, but the
  authoritative in-tx check guarantees at most one record.
- Native desktop file picker is unchanged — the Documents view uses
  the browser file input, as the browser app does; no arbitrary
  native filesystem access is granted to the browser.
- **Pre-existing dashboard battery failure (not a regression).**
  `dashboard.spec.ts` "quick actions jump to Ask, LARP, and project
  creation" fails on the pristine baseline `7e24859e` exactly as on
  this branch (5/6, same test): toggling LARP on mounts the
  `LarpHud` overlay over the Dashboard Quick actions, so `+ New
  Project` is never actionable. Out of scope for this slice.
- The Documents view does not yet surface per-part upload progress
  beyond received bytes; the job log/progress covers it.
- `claim_export`'s ingestion checkpoint preservation (see §2.1) is
  specific to ingestion jobs; export/backup jobs keep the
  `snapshot`/0 reset.

## 6. Files changed (tip 5a48a0ed1 over 7e24859e6)

- New: `crates/core/src/document.rs`, `crates/core/src/ingestion.rs`,
  `packages/ui/src/Documents.tsx`, `packages/ui/src/documents.ts`,
  `packages/ui/src/markdown.ts`, `packages/ui/tests/documents.test.tsx`,
  `tests/e2e/documents-ingestion.spec.ts`.
- Modified: `crates/core/src/{collider,content,export,jobs,lib,
  mcp_server,memory}.rs`, `crates/api/src/lib.rs`,
  `crates/contracts/src/main.rs`, `packages/contracts/{src/client.ts,
  src/generated.ts, src/validators.d.mts, src/validators.mjs,
  schemas/readiness.json}`, `packages/ui/src/{Jobs,Navigation,
  Workspace}.tsx`, `packages/ui/tests/components.test.tsx`,
  `package.json`, `tests/e2e/collider-three-helpers.ts`.
- Docs: `docs/handoffs/m13-document-ingestion-design.md` (70ebe4665),
  this addendum.
- NOT touched: `docs/BUILD_STATUS.md`, `docs/ROADMAP.md`,
  `docs/REQUIREMENTS_MATRIX.md`, `docs/RECOVERY.md`,
  `docs/SPEC_COVERAGE.md`, README status rows, Antigravity-owned
  directories/files, reviewer files, the owner's Desktop demo
  runtime (`548f3b36e`), master (no merge, no push, no CI).

## 7. Proposed global status updates (for Antigravity — NOT applied)

- `docs/REQUIREMENTS_MATRIX.md`: AN-025 `planned` → `in progress`
  with text "TXT/Markdown ingestion foundation delivered on Linux
  (M13 Slice 1); PDF/DOCX and other formats, OCR/transcription and
  async connectors remain planned." Do NOT mark `verified`.
- `docs/SPEC_COVERAGE.md`: §12 stays `planned` with a note
  "TXT/Markdown slice delivered on Linux 2026-11-05 (branch
  muse/m13-document-ingestion); full §12 still planned." §66 stays
  as-is (ingestion jobs use the existing job system; no §66 change
  claimed).
- `docs/BUILD_STATUS.md`: append an entry recording this slice
  (branch, tip, Linux evidence counts above, Windows rerun
  outstanding), and note the F-ING-01 finding (unresolved merge
  markers at lines 21/34, duplicated LARP sections, stray BOM) and
  the F-ING-02 finding ("Milestone 13 is 100% complete" contradicts
  AN-025/§12 `planned`) remain for Antigravity to resolve.
- No README status change proposed.

## 8. Windows acceptance checklist (outstanding — Linux evidence closes nothing here)

1. NTFS run of `tests/e2e/documents-ingestion.spec.ts` (all 9) on
   Windows, with the same lease wrapper.
2. Full regression battery (25 specs) on Windows.
3. Native desktop smoke of the Documents view (Tauri file input,
   not just the browser app).
4. Pairing/Origin behavior on Windows (the API requires
   `Origin: http://127.0.0.1:1420`).
5. Backslash-path handling in any Windows review ZIP extraction
   (Python zipfile, not plain `unzip`).
6. Confirm no Linux-only claim in this addendum is read as Windows
   acceptance.

## 9. Proposed next format slice (per PROMPT-MUSE.md)

**PDF text extraction** (design §8): reuse the same
`ingest-begin`/`ingest-part`/job/provenance shape; add a local
PDF text extractor (no cloud, no OCR in this slice — text-based
PDFs only, scanned/image PDFs refused honestly); page-aware
evidence (page numbers in citations); per-page size accounting
against the same 8,000-char text cap (or a documented per-file
cap revision, decided in that slice's design). DOCX after PDF.
