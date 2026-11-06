# M13 Document Ingestion Slice 1 — Design: TXT/Markdown foundation

Milestone 13, first ingestion slice. Baseline: coordination tip
`7e24859e6c042aaa135d632ad7009d20ab3c35e1` (branch
`codex/coordination-progress-2026-11-05`). Feature branch:
`muse/m13-document-ingestion`. This document is committed before
implementation code.

## 0. What this slice is — and what it is not

MCP completion is not completion of document intelligence. MASTER_SPEC §12
requires a common ingestion system for Markdown, TXT, PDF, DOCX, PPTX, XLSX,
HTML, web pages, EML, images, audio and video transcripts, with preserved
source identifiers, local OCR/transcription where possible, no automatic
cloud transmission of private documents, and asynchronous, resumable,
cancellable ingestion (§66). AN-025 is still `planned` and §12 coverage is
still `planned`; the coordination handoff's "Milestone 13 is 100% complete"
wording is not adopted — this slice implements exactly the TXT/Markdown
foundation and leaves every other format tracked as follow-up work.

This slice is one complete thin vertical workflow, shared by browser and
desktop through the established contracts: choose a file → preview
name/size/privacy and approve import → bounded upload → persistent job
progress → survive client/API restart and resume safely → read/search the
imported document → inspect source provenance and Ask's evidence/disclosure
→ back up → restore fresh → reopen with valid provenance.

Out of scope, tracked as follow-ups (not implemented here, not declared
complete): PDF, DOCX, PPTX, XLSX, HTML/web pages, EML, OCR, transcription,
multi-file batch import, a native desktop file picker (the desktop uses the
same browser file input in this slice; native dialog details stay pending
Windows review), automatic resume (recovery is explicit-only, per §66 and
the existing jobs semantics), and any M14/macOS work.

## 1. Inventory — what already exists (verified in the tree)

1. **Persistent jobs** (`crates/core/src/jobs.rs`): durable SQLite `jobs`
   table, states Queued/Running/Paused/Completed/Failed/Cancelled, lease
   fencing (`LEASE_SECONDS = 120`), `recover_jobs` flips stale Running jobs
   to Paused ("Worker interrupted; resume explicitly"), `control_job` is a
   closed transition table with `expectedVersion` fencing, cap 32 active
   jobs per workspace, one background worker per process claiming the
   oldest Queued job. A Completed job MUST carry `artifact_id`,
   `artifact_bytes` and `artifact_hash` (table CHECK) — ingestion fits
   this exactly (artifact = published document memory, bytes/hash = raw
   file bytes/SHA-256).
2. **Bounded chunked upload** (`crates/core/src/backup_upload.rs`):
   `upload-begin` declares bytes + SHA-256, `upload-part` appends ≤64 KiB
   parts with offset-must-equal-received, finish re-reads and hash-verifies
   before publishing; staged files live in the workspace export directory
   via `snapshots::new_file` (create-new, 0600, link rejection). The
   backup flow is backup-specific (its `expected_bytes` CHECK starts at
   1024 and its finish path verifies a snapshot archive), so ingestion
   reuses the mechanics, not the table.
3. **Strict document validation** (`crates/core/src/collider.rs`
   `validate_document_source`, UI `packages/ui/src/documentInput.ts`
   `decodeDocumentBytes`): kind `txt`/`md` only, label is a basename of
   1–255 chars whose extension matches the kind, raw file ≤ 65,536 bytes,
   decoded text 1–8,000 chars, strict UTF-8 (fatal decoder, never
   replacement characters), one leading BOM stripped and recorded, text
   hash and raw hash (BOM bytes included) recomputed against the submitted
   text. The collider path is beam-scoped, not reusable as-is; this slice
   factors the shared rules into a new `document.rs` module (see §3).
4. **Canonical document identity** (`crates/core/src/memory.rs`,
   `entities.rs`): kind `document` is a supported kind; `memory_write`
   publishes a versioned memory with content-hash-addressed body files
   (`content::write`, 0600, hash verified on read), FTS indexing via
   `search::sync_index_tx`, privacy enforcement (`allow_plaintext`
   refuses Sealed), and restore replays `memory_revisions` rows verbatim
   (metadata JSON, body hash), so provenance stored in metadata survives
   backup/fresh restore. `MemoryDraft.body` ≤ 256 KiB.
5. **Search and Ask evidence** (`crates/core/src/search.rs`,
   `ask.rs`): `title_search`/`body_search` filter by kind and exclude
   Sealed in SQL; `locate_evidence` returns heading + line number, so
   Markdown headings in imported documents produce real evidence
   locations. Ask compiles context with `kind_filter: None`, so indexed
   document memories are citable with no Ask change; insufficient
   evidence still returns the honest `insufficientEvidence` state.
6. **API/contracts/client plumbing** (verified by a read-only inventory
   of the baseline): routes in `crates/api/src/lib.rs` under `/v1/`
   with a `command_path` allowlist and per-handler body caps (jobs
   handler 384 KiB); DTOs registered only in
   `crates/contracts/src/main.rs`, regenerated by
   `node scripts/contracts.mjs`; `packages/contracts/src/client.ts`
   passes `jobs/` through unchanged with a 120 s timeout, and
   `importBackup` is the template for a chunked upload; `nativeClient`
   maps `jobs/` to the `job_command` Tauri invoke. There is **no
   single-job read op** — fresh job state comes from `jobs/list` + find
   by id (the e2e poll pattern).
7. **UI precedents**: `Jobs.tsx` polls `listJobs` every 4 s and renders
   Pause/Resume/Cancel/Retry from the closed transition table;
   `Backups.tsx` uses `<input type="file">` (browser-mediated picking,
   no native path access); `Collider.tsx` + `documentInput.ts` already
   do client-side pre-refusal (size before reading, strict decode,
   generation-gated reads); `Memory.tsx` renders bodies as plain text
   (`<pre>`, "Literal Markdown text · maximum 256 KiB. Images, links
   and HTML are not executed."); `KnowledgeRecords.tsx` and `Search.tsx`
   already list/filter kind `document`, but no UI creates or renders a
   document record as a document. Navigation has 17 views with an
   exact-count test.

## 2. Assignment-wording corrections

1. **There is no `documents` command group to add, and none is added.**
   The whole slice runs through the existing `jobs/` group
   (`ingest-begin`, `ingest-part`), the existing `memories/` group
   (read/list/search), and the existing jobs table. A new group would
   need route + allowlist + client mapping + native mapping for no
   benefit; the jobs group already owns upload, progress, control and
   recovery.
2. **`ExportSelection` is the durable job-spec enum, not export-only.**
   It already carries a `Backup` variant; this slice adds an `Ingestion`
   variant to it (see §3). The name is kept to avoid churning every
   jobs consumer; the design records the meaning here.
3. **No schema migration.** Staging identity lives in the job row
   itself (selection JSON + checkpoint text + progress percent) and the
   staged file on disk; no new table is created and `SCHEMA_VERSION`
   stays 11. The jobs table is §66's durable record — a second staging
   table would duplicate what it already records.

## 3. Design

### 3.1 New module `crates/core/src/document.rs`

Shared, format-independent document rules factored out of the collider
path so ingestion and the collider validate the same contract:

- Constants: `MAX_DOCUMENT_BYTES = 65_536`, `MAX_DOCUMENT_CHARS = 8_000`,
  `MAX_LABEL_CHARS = 255`, `MAX_UPLOAD_PART_BYTES = 65_536`.
- `DocumentSource { label, kind ("txt"|"md"), bom, raw_bytes, raw_hash,
  text_hash, text_chars }` and `validate_document_source(&source, text)`,
  moved verbatim from `collider.rs` (same checks, same messages).
  `collider.rs` keeps working through a re-export
  (`pub type ColliderDocumentSource = crate::document::DocumentSource;`
  plus a re-exported validator), so collider tests do not change.
- `DocumentProvenance { file_name, media_type ("text/plain" |
  "text/markdown"), raw_hash, text_hash, raw_bytes, text_chars, bom }`
  — the exact-source provenance stored on the published memory (see
  §3.4). `media_type` is derived from the kind, never trusted from the
  client.

### 3.2 New module `crates/core/src/ingestion.rs`

Owns the upload operations and the worker. `job_command` routes
`ingest-begin` / `ingest-part` to it, exactly as `upload-begin` /
`upload-part` route to `upload_operation`.

**`ingest-begin`** (`IngestionBegin { workspace_id, file_name, bytes,
sha256, privacy, project_id? }` → `IngestionStatus { job, upload_id,
received }`):

- Validates workspace id; filename via the shared label rules
  (basename, 1–255 chars, extension `.txt`/`.md` case-insensitive →
  kind); `bytes` 1–65,536 (documents may be tiny — unlike backup
  uploads, whose floor is 1024); `sha256` 64 lowercase hex;
  `privacy` must not be Sealed (refused with an explicit message —
  Sealed import is refused until an established encrypted path
  exists); optional `project_id` must name an existing project memory
  in this workspace (early refusal, same EXISTS check `memory_write`
  uses at publish).
- Caps: at most 8 active ingestion uploads per workspace (mirror of
  the backup-uploads cap), inside the existing 32-active-jobs cap.
- Sweeps stale staging first (bounded, job-owned only): deletes
  `{jobId}.upload` files in the workspace export directory whose job
  is Completed/Cancelled, or which have no matching job row. Never
  deletes owner files.
- Creates the staged file (`snapshots::new_file`,
  `{jobId}.upload` in `export::directory`), inserts the job in state
  **Paused** with selection `Ingestion { upload_id (= job id),
  file_name, kind, expected_bytes, expected_hash, privacy,
  project_id }`, checkpoint `uploading:0`, progress 0, log
  "Upload in progress". The upload id IS the job id — one durable
  identity for upload, job and staging.
- Returns the job. No worker wake-up: a Paused job is never claimed.

**Why Paused, not Queued, during upload.** The worker claims any
Queued job. An upload-in-flight job must never be claimed, and there
is no "Uploading" state in §66. Paused already means "needs an
explicit action before it runs", which is exactly the upload state.
Finish moves the job to Queued; interruption leaves it Paused with
staging intact. `control_job` Resume is guarded: resuming an
ingestion job whose checkpoint is not `staged` is refused
("Upload is incomplete; resume the upload instead"), so the Resume
button can never queue an unuploaded file.

**`ingest-part`** (`IngestionPart { workspace_id, id (job id), offset,
bytes, finish }` → `IngestionStatus`):

- Validates ids; part ≤ 64 KiB; job must be Paused with an Ingestion
  selection and checkpoint `uploading:<received>` (a `staged` job
  refuses further parts with a conflict).
- `offset` must equal the staged file's length on disk (ground truth,
  link-rejected path). Writes are positional (`seek(received)` +
  write + sync), so a crash between the file write and the checkpoint
  commit is safe: the next part re-writes from the committed offset.
  `offset + len` must not exceed `expected_bytes`.
- Inside one Immediate transaction, fenced on the job's
  `checkpoint_version` (a concurrent part loses with a conflict):
  update checkpoint to `uploading:<new_received>` and progress to
  `received * 100 / expected` (percent — `progress` is CHECK-bound to
  0..10,000, so it cannot hold byte counts).
- If `finish` is true: `received` must equal `expected_bytes`; the
  staged file is re-read with `snapshots::limited` and its SHA-256
  compared to the declared hash. Mismatch → job Failed, staging
  deleted, log "Upload bytes do not match the declared hash; import
  the file again." (Retry then has nothing to retry from; the worker
  reports that honestly — see §3.3.) Match → checkpoint `staged`,
  progress 100, state Queued, log "Upload complete; queued for
  import", and the export worker is woken.

**Worker** (`Storage::execute_ingestion(&JobLease)`, dispatched from
`wake_export_worker` on `ExportSelection::Ingestion`):

1. Re-reads the job under an Immediate transaction: state must be
   Running with this lease's owner, checkpoint must be `staged`.
   Anything else → release quietly (a Cancelled job is already
   Cancelled; nothing was published).
2. Reads staged bytes (`limited`, 65,536 cap) and re-verifies length
   and SHA-256 against the selection. Mismatch → fail the job.
3. Strict decode: `String::from_utf8` — invalid UTF-8 fails the job,
   never replacement characters. Strips one leading BOM (recorded).
   Validates decoded text 1–8,000 chars, non-whitespace, and calls
   the shared `validate_document_source` against the decoded text.
   Any failure → fail the job with the specific refusal; no memory
   is created; staging is kept so Retry can re-run from the same
   bytes.
4. Publishes via `memory_write` with kind `document`, title =
   file name, body = decoded text, privacy = selection privacy,
   provenance `document_import`, document = provenance record (§3.4),
   and an internal job guard (see §3.5).
5. On success: deletes the staged file (best effort), marks the job
   Completed with `artifact_id` = memory id, `artifact_bytes` = raw
   byte count, `artifact_hash` = raw SHA-256, checkpoint `published`,
   log "Document imported". On a duplicate (guard found an existing
   document), completes with that memory id and log "Already imported".

**Cancellation.** `control_job` Cancel works on Queued/Running/Paused
ingestion jobs (closed table, unchanged). Cancelling deletes the
staged file best-effort in the same operation and sets checkpoint
`discarded` + log "Import cancelled; upload staging removed". A
Running job whose worker has already passed the publish guard
completes instead — Cancel then correctly refuses (Completed is not
in the Cancel row), so the UI never shows Cancelled while backend
work already published. This is the owner's hard line, enforced by
the guard in §3.5, not by lengthening any deadline.

**Interruption / restart.** The API process may die at any point.
`recover_jobs` (called by `list_jobs`, `control_job`,
`claim_export`) flips a stale Running ingestion job to Paused with
log "Worker interrupted; resume explicitly"; staging and the job row
survive, so explicit Resume → Queued → the worker re-runs from the
staged file, and the guard + dedup (§3.5) make the re-run publish
at most one record. A client restart resumes an upload from the
job's checkpoint (`uploading:<received>`): the Documents UI offers
"Resume upload" and continues parts from that offset. There is no
auto-resume anywhere.

**Retry.** Retry ← Failed|Cancelled → Queued (closed table,
unchanged). A Failed job with staging re-runs validation + publish
from the same bytes. A Failed/Cancelled job whose staging was
discarded fails fast in the worker ("Staging was removed; import the
file again") — the UI then offers a fresh import, not another Retry.

### 3.3 `jobs.rs` changes (minimal, additive)

- `ExportSelection` gains `Ingestion { upload_id, file_name, kind,
  expected_bytes, expected_hash, privacy, project_id }` (serde
  tag `kind`, same as the other variants) with validation in
  `ExportSelection::validate` (filename/kind/hash/privacy/project
  rules — the same checks `ingest-begin` runs, so a forged job row
  cannot smuggle an invalid ingestion spec past the worker).
- `wake_export_worker` dispatches Ingestion selections to
  `execute_ingestion`.
- `control_job`: Resume refuses an ingestion job whose checkpoint is
  not `staged`; Cancel on an ingestion job deletes staging
  best-effort and records checkpoint `discarded`.
- Nothing else in the jobs table, states, caps, lease or recovery
  changes. No new table, no migration.

### 3.4 Provenance on the canonical record

`MemoryMetadata` gains an optional field, serde-defaulted and
skipped when absent (the established pattern of `section_heading` /
`line_number`):

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub document: Option<DocumentProvenance>,
```

`MemoryWriteRequest` gains two optional fields, also defaulted:

- `provenance: Option<String>` — only the value
  `document_import` is accepted, and only together with a
  `document` value and kind `document`. Any other value, or a
  `document_import` provenance without the document record, is
  refused. `memory_write` otherwise keeps provenance
  `user_authored`.
- `document: Option<DocumentProvenance>` — accepted only with kind
  `document`. `memory_write` re-validates it against the submitted
  body with the shared validator (kind/extension match, text hash
  == `hash(body)`, raw hash recomputed from BOM + body bytes,
  raw byte count and char count match). Provenance is therefore
  verified, never trusted: `memories/create` cannot forge a
  document record whose hashes do not match its body.

The document record travels in the metadata JSON, so backup/fresh
restore preserves it verbatim with no restore change (restore
replays `memory_revisions` rows). The UI reads it from
`MemoryDetails.memory.document` and renders: original file name,
media type, raw/text SHA-256, byte and char counts, BOM flag.
`source_references` stays empty — no invented references.

### 3.5 Atomic publish guard (lease + dedup inside one transaction)

`MemoryWriteRequest` gains an internal-only field
(`#[serde(skip)] pub(crate) job_guard: Option<JobPublishGuard>`,
unreachable from JSON — only Rust callers set it):

```rust
pub(crate) struct JobPublishGuard {
    pub job_id: String,
    pub owner: String,
    pub dedup_title: Option<String>,      // file name
    pub dedup_text_hash: Option<String>,   // SHA-256 of decoded text
}
```

Inside `memory_write`'s existing Immediate publish transaction,
when the guard is present, the worker first verifies the job is
still Running with this lease owner, then — when dedup fields are
set — looks up a current-revision `document` memory in this
workspace with the same title and `body_hash`. If one exists, the
call returns that existing record's details without creating a
revision; otherwise it publishes normally. The lookup and the
publish are therefore atomic with respect to both cancellation and
a concurrent same-file publish, even across two API processes
(the Immediate transaction serializes them). If the lease check
fails, the call returns `RevisionConflict` ("job cancelled"),
the worker releases quietly, and nothing is published. The guard
is used only by the ingestion worker; no other `memory_write`
caller changes behavior.

### 3.6 API / contracts / client

- `job_command` gains `ingest-begin` and `ingest-part`, routed to
  `ingestion_operation` (same position as `upload-begin` /
  `upload-part`). No new command group.
- `crates/api/src/lib.rs` `command_path` allowlist gains
  `jobs/ingest-begin` and `jobs/ingest-part`. The jobs handler's
  384 KiB body cap already covers a 64 KiB part plus JSON.
- Contracts: register `IngestionBegin`, `IngestionPart`,
  `IngestionStatus`, `DocumentProvenance` (and the extended
  `ExportSelection` / `Job` / `MemoryMetadata` / `MemoryWriteRequest`
  shapes) in `crates/contracts/src/main.rs`; regenerate with
  `node scripts/contracts.mjs` (drift check must be 0).
- Client (`packages/contracts/src/client.ts`): `ingestBegin` /
  `ingestPart` methods following the `importBackup` chunked-upload
  template (client-side SHA-256 via `crypto.subtle`, 64 KiB parts,
  sequential, offset-exact). No path-mapping change (`jobs/`
  already passes through with the 120 s timeout). `nativeClient`
  needs no change (`jobs/` already maps to `job_command`).

### 3.7 UI — one `Documents` view

New view `documents` in the KNOWLEDGE group (`Navigation.tsx`;
view count 17 → 18, exact-count test updated — disclosed test
maintenance). `Workspace.tsx` heading "Your documents" and mount.
New `packages/ui/src/Documents.tsx` + `packages/ui/src/documents.ts`
helpers, reusing `documentInput.ts`'s decoder (client-side
pre-refusal before any read: extension, 64 KiB, strict UTF-8,
empty, >8,000 chars — no network traffic for a refusal):

1. **Choose** — `<input type="file" accept=".txt,.md,text/plain,
   text/markdown">` (browser-mediated picking; no native path is
   read, sent or stored — only the basename). The picked file is
   decoded locally; a refusal is shown immediately with the reason.
2. **Preview & approve** — file name, size, kind, char count, BOM
   note, privacy select (Private default; Standard/Public allowed;
   Sealed disabled, as in `Memory.tsx`), optional project binding,
   and the exact bytes/hash that will be declared. "Import document"
   sends `ingest-begin` with the client-computed SHA-256.
3. **Upload & progress** — sequential 64 KiB parts via `ingestPart`;
   progress from the job (percent + `uploading:<received>`); Cancel
   via `controlJob` (stops sending parts and cancels the job);
   "Resume upload" after a client restart reads the job checkpoint
   and continues from that offset. Job states rendered verbatim
   (Queued/Running/Paused/Completed/Failed/Cancelled) with the
   server log; no invented states.
4. **Open & read** — imported documents list from
   `listMemories(kind: "document")`; open via `memories/read`;
   renders Markdown with a new minimal safe subset renderer
   (`packages/ui/src/markdown.ts`: headings, bold/italic, inline
   code, fenced code, unordered lists, paragraphs — React elements
   only, no `dangerouslySetInnerHTML`, no anchors, no HTML
   execution) plus a "View source" toggle showing the raw text in
   `<pre>`. Provenance panel shows the §3.4 record. Private stays
   private: the list/read path is the existing workspace-scoped
   memory path; Sealed is never listed.
5. **Search & Ask** — a "Search documents" entry runs the existing
   body search scoped to kind `document`; Ask evidence is inspected
   in the existing Ask view (Data Disclosure box) — no Ask change.
   The workflow links to Jobs for the full job list rather than
   duplicating it.

`Jobs.tsx` gains an ingestion label arm in its inline job-type
rendering ("Document import: <file name>") and keeps its 4 s poll.
No other existing view changes behavior.

### 3.8 Limits (conservative, documented)

| Bound | Value | Enforced |
|---|---|---|
| Raw file | 1–65,536 bytes | client pre-refusal + `ingest-begin` + worker re-read |
| Decoded text | 1–8,000 chars, non-whitespace | client decoder + worker validation |
| File name | 1–255 chars, basename, `.txt`/`.md` | client decoder + begin + worker |
| Upload part | ≤ 64 KiB, offset-exact, sequential | `ingest-part` |
| Active ingestion uploads / workspace | ≤ 8 | `ingest-begin` |
| Active jobs / workspace | ≤ 32 (existing) | enqueue path |
| Staging | one `{jobId}.upload` per active upload | sweep at begin; delete on Completed/Cancelled/hash-mismatch |
| Memory body | ≤ 256 KiB (existing) | `memory_write` (8,000 chars fits easily) |

No shell, no external URL fetching, no remote scripts, no
macro/HTML execution, no automatic provider/OCR/model calls. No
credentials. No paid/cloud inference. No model downloads.

## 4. Requirement mapping

- **§12 DOCUMENT INTELLIGENCE** — TXT + Markdown only in this slice;
  async/resumable/cancellable via §66 jobs; source identifiers
  preserved (file name, media type, both hashes, counts, BOM);
  no automatic cloud transmission (no provider call anywhere in
  the slice). PDF/DOCX/PPTX/XLSX/HTML/EML/images/audio/transcripts
  and OCR remain tracked follow-ups.
- **§66 SHARED BACKGROUND JOB SYSTEM** — persistent jobs with
  progress (percent + checkpoint), cancellation, retry, resource
  limits (table above), logs, recovery after restart (explicit
  Resume only). No heavy computation on the UI thread (worker
  thread, chunked upload).
- **§68 MCP INTEGRATION** — untouched by this slice (no MCP change).
- **§71 SECURITY MODEL** — path traversal / symlink escapes refused
  (basename-only labels, link rejection on every staged/content
  path, `create_new` staging); malicious documents refused by
  strict UTF-8 + size + hash validation before publication;
  Markdown rendered inert (no HTML execution); browser requests
  carry bounded bytes and upload ids, never native filesystem
  authority; no shell, no URL fetching.
- **§74 DATA PRIVACY** — Private by default; Standard/Public by
  explicit choice; Sealed import refused (no encrypted path yet);
  no cross-workspace reads (workspace-scoped jobs, staging,
  memories, search); no privacy downgrade (imports never change
  an existing record's privacy; dedup returns the existing
  record unchanged).
- **AN-025** — partial: TXT/MD foundation only. Stays `planned`
  in the matrix (Antigravity-owned; proposed update in the
  handoff, not edited here). M13 is **not** declared complete.

## 5. Acceptance criteria (named)

- **AC-ING-01 (happy path)** — A selected `.txt` file imports
  end-to-end in the browser UI over the real API: preview → approve
  → upload → job reaches Completed → the document opens with its
  text, file name and provenance (both hashes, counts, BOM flag).
  Same for `.md`.
- **AC-ING-02 (encoding refusal)** — A file with invalid UTF-8 is
  refused client-side before any upload and, when forced through
  the API, fails the job server-side; no memory is created and no
  text is searchable.
- **AC-ING-03 (size refusal)** — A file over 65,536 bytes is refused
  before reading/upload; a file over 8,000 chars (within the byte
  cap) fails the job; an empty/whitespace-only file is refused.
- **AC-ING-04 (hash refusal)** — Declaring one SHA-256 and sending
  different bytes fails the upload at finish; the job is Failed,
  staging is deleted, no memory is created. Modifying the source
  between selection and upload is caught the same way.
- **AC-ING-05 (format refusal)** — `.pdf`, `.docx`, image and audio
  files are explicitly refused with a helpful message naming the
  format and the follow-up status; none is treated as imported.
- **AC-ING-06 (privacy)** — Import defaults to Private; Standard
  and Public import when chosen; Sealed is refused at begin with
  an explicit message. A Private document is invisible to another
  workspace (list/search/read all refuse or omit it).
- **AC-ING-07 (workspace isolation)** — An ingestion job, its
  staging file and its published memory are bound to one workspace;
  a second workspace cannot list, read, search or control them.
- **AC-ING-08 (cancellation)** — Cancelling during upload stops
  the upload, cancels the job and removes staging; cancelling a
  Queued/Running job publishes nothing and leaves no searchable
  document. A job that already Completed cannot be cancelled into
  a false Cancelled state.
- **AC-ING-09 (interruption recovery)** — Killing the API process
  mid-publish (or mid-upload) leaves no half-published or
  searchable document; after restart the job is Paused with its
  staging intact, explicit Resume re-runs it, and exactly one
  document exists.
- **AC-ING-10 (retry/idempotency)** — Retrying a Failed job with
  staging re-runs it without a new upload and publishes at most
  one record. Two concurrent imports of the same file (same
  workspace, name and text hash) complete with the same memory
  id and create exactly one record.
- **AC-ING-11 (upload resume)** — After a client restart, an
  interrupted upload resumes from the job's checkpoint offset
  without re-sending completed parts and without duplicating bytes.
- **AC-ING-12 (search)** — Imported text is findable through the
  existing body search scoped to kind `document` (including a
  Markdown heading term), and only after the job Completes — never
  while Queued/Running.
- **AC-ING-13 (Ask evidence)** — Asking about imported content
  returns a citation naming the document (title + heading/line
  from `locate_evidence`) with the Data Disclosure manifest
  listing it as a source memory; with no matching import, Ask
  honestly reports insufficient evidence.
- **AC-ING-14 (backup/restore)** — A workspace backup containing
  an imported document restores into a fresh catalog; the document
  reopens with identical text, file name, media type and both
  hashes, and remains searchable.
- **AC-ING-15 (Markdown rendering)** — An imported `.md` renders
  headings/bold/code/lists inertly (no executed HTML, no anchors,
  no scripts); "View source" shows the exact raw text.
- **AC-ING-16 (no native authority)** — No browser request carries
  a filesystem path or native capability; the file input is the
  only picker; staged paths are server-derived from the job id.
- **AC-ING-17 (honest states)** — Queued, Running, Paused,
  Completed, Failed and Cancelled are all reachable and rendered
  verbatim with the server log; an interrupted job says so and
  waits for an explicit Resume.
- **AC-ING-18 (regression)** — Focused + full regression suites
  pass on Linux (Rust `--lib`, UI, typecheck, contracts drift 0,
  docs validator, web build, API build, Playwright battery
  including the new spec); no existing test is weakened or skipped.

## 6. Files to change (planned)

- NEW `crates/core/src/document.rs` — shared constants, source
  validation, provenance type.
- NEW `crates/core/src/ingestion.rs` — `ingest-begin` /
  `ingest-part` operations, `execute_ingestion` worker.
- `crates/core/src/collider.rs` — re-export the shared validator
  (no behavior change; collider tests untouched).
- `crates/core/src/jobs.rs` — `ExportSelection::Ingestion`,
  worker dispatch, Resume guard, Cancel staging cleanup.
- `crates/core/src/memory.rs` — `MemoryMetadata.document`,
  `MemoryWriteRequest.provenance` / `.document` /
  `.job_guard`, publish-transaction guard + dedup.
- `crates/core/src/lib.rs` — module declarations.
- `crates/api/src/lib.rs` — `command_path` allowlist entries.
- `crates/contracts/src/main.rs` — register new DTOs; regenerate.
- `packages/contracts/src/client.ts` — `ingestBegin` /
  `ingestPart`.
- NEW `packages/ui/src/Documents.tsx`, `packages/ui/src/documents.ts`,
  `packages/ui/src/markdown.ts` (+ tests).
- `packages/ui/src/Navigation.tsx`, `packages/ui/src/Workspace.tsx`
  — `documents` view (count 17 → 18, test updated).
- `packages/ui/src/Jobs.tsx` — ingestion job label arm.
- NEW `tests/e2e/documents-ingestion.spec.ts` (+ harness help if
  needed).
- NEW `docs/handoffs/m13-document-ingestion-design.md` (this file),
  `docs/handoffs/m13-document-ingestion-addendum.md` at closure
  (own files; proposed global-status updates go in the handoff,
  Antigravity's files are not edited).

## 7. Baseline findings carried into the handoff (not fixed here)

- **F-ING-01** — `docs/BUILD_STATUS.md` in the coordination tip
  contains unresolved merge-conflict markers (`=======` at line 21,
  `>>>>>>> codex/larp-entity-chat` at line 34, duplicated LARP
  sections and two versions of the Slice 4 entry). The file is
  Antigravity-owned; this slice does not edit it. Note the docs
  validator still passes 287 files despite the markers.
- **F-ING-02** — The coordination baseline's BUILD_STATUS and README
  state "Milestone 13 is 100% complete" while AN-025 is `planned`
  and §12 is `planned`. This slice does not declare M13 complete;
  the proposed correction is in the handoff.
- **F-ING-03** — The LARP 30-second film is already finished and
  delivered (exact 30.000 s, all seven views, MP4 attached
  2026-11-05 04:42 CET); its baseline remains valid and it is not
  restarted by this slice.

## 8. Proposed next format slice

PDF text extraction (local, no cloud): same job/upload/provenance
shape, a `pdf` kind + `application/pdf` media type, page-aware
evidence locations, and the same refusal/idempotency/cancel/
restart matrix. DOCX follows the same pattern after PDF.
