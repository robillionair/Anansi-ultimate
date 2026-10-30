# M8 document input — bounded design and acceptance criteria (before code)

Baseline: reviewer tip `c50d6b03` (incoming runtime `06a3f573a`,
Windows-verified lineage repair). No lineage rework, no benchmark
campaign. Muse owns implementation and Linux verification;
ChatGPT independently reviews Windows; Antigravity's release
tooling checkout is untouched.

## Scope

MASTER_SPEC §24 lists "Choose document" for each beam. This slice
implements exactly that for UTF-8 `.txt`/`.md` files, for either
beam, through the existing M5 dispatch machinery and the existing
collider start/poll/cancel/save operations. Nothing else from
§23–§27 is claimed: URL fetching, PDF/DOCX, OCR, transcription,
HTML execution, archives, external-resource loading, 3D and
native promotion remain unfinished.

## Decoding policy (fixed before implementation)

- File bytes: at most 65,536 bytes. Decoded text: 1–8,000
  characters, matching the existing beam bounds, and
  non-whitespace after trimming (the backend's existing beam
  rule; the UI pre-check mirrors it, the backend is authority).
- UTF-8 only, decoded strictly: any invalid sequence refuses
  visibly. No replacement characters, ever.
- BOM: one leading UTF-8 BOM (EF BB BF), if present, is
  stripped and recorded (`bom: true`). No other normalization:
  indentation, tabs, CRLF/LF line endings and terminal newlines
  are preserved exactly as decoded.
- Label: the picked file's name only (browsers already supply
  a basename). Labels are 1–255 characters, contain no `/`,
  `\` or NUL, and their extension (case-insensitive) must match
  the kind (`.txt` → `txt`, `.md` → `md`). A label is a
  user-supplied display string, never a path and never proof
  the backend accessed a file.
- No absolute filesystem paths are sent, persisted or rendered
  anywhere. Selecting or reading a file starts no inference
  and creates no durable record; no separate memory/document
  is registered.

## Backend contract (no schema change)

New optional fields on the start request and the saved
experiment document (serde defaults; old documents without
them parse and restore exactly as before, and new documents
without them serialize identically to before):

```text
ColliderDocumentSource {
  label: String,        // basename label, see policy above
  kind: "txt" | "md",
  bom: bool,            // a leading UTF-8 BOM was stripped
  rawBytes: u64,        // file byte length as received
  rawHash: String,      // SHA-256 hex of the raw file bytes
  textHash: String,     // SHA-256 hex of the submitted text bytes
  textChars: u64,       // submitted text char count
}
```

The backend validates every field against the submitted beam
text before any provider traffic, and the saved-document
validator re-checks the same contract at restore and transfer
time (both already call `validate_experiment_doc`):

- `textHash` equals the SHA-256 of the beam text bytes;
  `rawHash` equals the SHA-256 of (BOM bytes if `bom`, else
  nothing) followed by the beam text bytes; `rawBytes`
  equals the beam byte length plus 3 iff `bom`;
  `textChars` equals the beam char count and lies in 1–8,000;
  kind/extension and label rules as above.
- Any mismatch refuses with `InvalidRequest` before provider
  traffic. Forged, stale (edited-beam) or inconsistent
  metadata therefore cannot pass, even from a hostile client.
- The document text travels to the provider only as the
  beam's existing private data segment, verbatim. Labels and
  hashes are never sent to the provider; they live in the
  saved document and the UI review/saved views. Embedded
  instructions, URLs and Markdown in document text remain
  data under the existing instruction segment, which already
  frames beams as data.
- The run carries the sources from start to save, so the
  saved document pins exactly the submitted generation-time
  content. Later source-file changes cannot rewrite it, and
  backup/restore carries it verbatim like the beams.

## UI contract (shared browser/desktop file input)

- One ordinary `<input type="file" accept=".txt,.md,…">` per
  beam — the same element in browser and desktop webview.
  No native filesystem capability is introduced, so no new
  trust boundary is created for an untrusted browser.
- Pre-send review per beam: the textarea shows the exact
  decoded text; a source line shows label, kind, byte count,
  char count and the text hash (truncated display, full hash
  in the title attribute). Both lines render as inert text
  (React-escaped; Markdown is never rendered as HTML).
- Editing a document-loaded beam clears its attribution in
  the UI (the beam becomes manual text); the backend's hash
  check refuses any stale attribution regardless.
- Clearing a source clears the loaded beam text and its
  attribution. A failed read leaves the previous beam and
  attribution untouched and shows a visible error with retry
  (re-pick the file).
- Race safety, per beam, via a load session with a
  generation counter (`packages/ui/src/documentInput.ts`,
  unit-tested with deferred reads, not sleeps):
  a newer selection, an edit, a clear, or unmount
  invalidates any pending read, and only the current
  generation's result is applied. The file input's value is
  reset after every selection so reselecting the same
  filename with changed content fires again. Picker
  cancellation fires no change and changes nothing.
- The start request sends the beam texts plus the current
  source metadata (or nothing for manual beams). Nothing is
  sent before the explicit Collide Ideas press.
- Saved view: each beam's document provenance (label, kind,
  bytes, chars, hashes) renders as inert text beside the
  verbatim beam; experiments without document metadata render
  exactly as before.

## Tests (synthetic providers only)

Backend (`crates/core/tests/collider.rs` + a small unit
module in `collider.rs`):

- Valid document start: exact text (BOM + CRLF +
  indentation + terminal newline) reaches the correct beam
  on the wire; label/hashes never reach the provider;
  saved document pins exact beams and sources.
- Forged metadata (wrong text hash, raw hash, byte count,
  char count, kind/extension mismatch, path separator in
  label, flipped BOM flag) refuses before traffic
  (fake hits == 0), one case per rule.
- Edited beam with stale metadata refuses before traffic.
- Oversized/whitespace-only beams with metadata refuse.
- Save → restart → reopen preserves exact content and
  sources; backup → fresh restore preserves them.
- A document-backed experiment completes the approved
  Forge-transfer workflow.
- Old documents without document fields validate, restore
  and transfer unchanged (existing suites plus one explicit
  unit case).

UI (node, `packages/ui/tests/collider.test.tsx` +
`documentInput` unit tests):

- Decode: BOM strip + flag, CRLF/indent/terminal-newline
  preservation, invalid UTF-8 refusal, empty/whitespace/
  oversized refusal, extension/kind rules, hash recomputation.
- Load session: stale read dropped, edit invalidates,
  clear invalidates, second selection wins, failure leaves
  prior state untouched — all with deferred promises.
- Experiment-body parser: accepts valid document fields,
  rejects malformed ones; saved view renders provenance
  as inert text; documents without fields parse as before.

Browser (`tests/e2e/collider-document-input.spec.ts`,
canonical, fake provider with hit counting):

- Complete workflow: project → Collider → choose .md for
  Beam A → review exact text + label → choose .txt for
  Beam B → Collide Ideas → result + disclosure → save →
  reload/reopen → backup/restore → reopen with identical
  content and provenance; fake hits == 0 until Collide.
- At least one rejection (invalid UTF-8 file) with a
  visible error and unchanged beam.
- At least one source-switch race (select file A, then
  file B for the same beam) ending with B's content.
- Same filename reselected with changed content updates
  the beam; both beams hold independent sources.

## Regressions and verification order

Focused tests first (new backend + UI + browser spec),
then collider/transfer/forge_restore/backup_restore/
backup_artifacts/opportunities_restore/entities_roundtrip,
node UI/contracts, strict Clippy, fmt, contracts `--check`,
TypeScript, docs validation, API/web builds. Broaden once
(full core + providers/API) after focused acceptance passes;
keep the 4 known live-Ollama `semantic_workflow` environment
failures distinct. No repeated unchanged-suite runs.

## Delivery limits (retained as unfinished)

URL fetching, PDF/DOCX/OCR/transcription, HTML execution,
archives, external resources, 3D visualization, native
desktop acceptance, preview promotion, M5 production gates,
G1 audit, remote CI. Linux evidence does not certify Windows
native file-picker behavior; the delivery carries the exact
Windows checklist (dialog opens, .txt/.md filter, cancel,
unicode filename, same-name reselect, large-file refusal).
