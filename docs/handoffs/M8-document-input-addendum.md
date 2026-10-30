# M8 document input — delivery addendum (Linux verification)

Baseline: reviewer tip `c50d6b03` (incoming runtime `06a3f573a`,
focused Windows acceptance of the lineage repair). Design and
acceptance criteria were committed before code in
`docs/handoffs/M8-document-input-design.md`. No lineage rework,
no benchmark campaign, no schema change, no new provider
integration, no native filesystem capability.

## What was built

Explicit `.txt`/`.md` document input for either Collider beam,
through the existing M5 dispatch machinery:

- Backend (`crates/core/src/collider.rs`): new optional
  `ColliderDocumentSource` on the start request and the saved
  experiment document (serde defaults; old documents parse
  and restore unchanged, and new documents without sources
  serialize identically to before). `validate_document_source`
  recomputes every field against the submitted beam text
  before any provider traffic — text hash over the beam
  bytes, raw hash over (BOM bytes if flagged) + beam bytes,
  byte/char counts, kind/extension match, basename-only
  label — and `validate_experiment_doc` re-checks the same
  contract, so restore and transfer validation refuse
  forged or stale records too. The run carries the sources
  from start to save, pinning generation-time provenance.
- UI (`packages/ui/src/documentInput.ts`,
  `packages/ui/src/Collider.tsx`): one ordinary file input
  per beam, shared by browser and desktop webview; strict
  UTF-8 decode (no replacement characters), one leading BOM
  stripped and recorded, no other normalization; pre-send
  review shows the exact decoded text plus label, kind,
  byte/char counts and text hash as inert text; editing a
  loaded beam clears its attribution; clearing a source
  clears the loaded text; failed reads leave the previous
  beam and attribution untouched with a visible error and
  retry; per-beam load sessions with generation counters
  drop stale reads (new selection, edit, clear, unmount);
  the file input value resets after every selection so a
  same-name reselect with changed content fires again.
  Labels and hashes are never sent to the provider — the
  document text travels only as the beam's existing private
  data segment.
- Limits (fixed before implementation): file ≤ 65,536 bytes;
  decoded text 1–8,000 characters and non-whitespace;
  label 1–255 characters, basename only, extension matching
  kind (case-insensitive).

## Regression found and fixed during this slice

The first browser battery failed 6 of the first 7 affected
specs: the new file-input labels ("Document for Beam A …")
contained "Beam A" as a substring, so existing specs'
`getByLabel('Beam A')` resolved to two elements. Fixed by
renaming the file labels to first/second beam wording
(commit `fix(ui): document file labels must not collide
with beam labels`); no existing spec was touched. The full
affected battery then passed 24/24.

## Verification (sequential, Linux)

- New backend tests: 4 integration (exact wire text and
  provenance, 7 forgery variants + edited beam refusing
  before traffic with zero provider hits, oversized/blank
  refusal, transfer + backup/restore survival) + 2 lib unit
  tests (BOM source acceptance, every forgery refusal,
  old documents validating, documented documents pinning).
- Focused Rust: collider 45/45 (+1 ignored real-v10
  fixture test), transfer 19/19, forge_restore 4/4,
  backup_restore 7/7, backup_artifacts 5/5,
  opportunities_restore 3/3, entities_roundtrip 7/7,
  lib 76/76.
- Node UI/contracts: 94/94 (90 + 4 new document tests:
  decode, refusal cases, load-session races with deferred
  promises, provenance parser + saved view).
- Browser: 24/24 affected specs, including the new
  `collider-document-input` canonical spec (review → race
  → reselect → two rejections → collide → save → restart
  → backup/restore, with zero provider hits until the
  explicit press and byte-exact saved/restored bodies).
- Static/build: strict Clippy clean, fmt clean (one
  formatting pass applied after the first check),
  contracts `--check` clean, TypeScript clean, docs
  validation PASS (182 Markdown files), API/web builds pass
  (web bundle-size warning only).
- Broadened once: full core `--no-fail-fast
  -- --test-threads=1` 369 passed / 4 failed — reconciliation
  363 (lineage repair) + 4 integration + 2 unit = 369,
  failures only the known live-Ollama `semantic_workflow`
  environment failures; providers+API 144/0.

## Still unfinished (explicitly retained)

URL fetching, PDF/DOCX, OCR, transcription, HTML execution,
archives, external-resource loading, 3D visualization, M7
native desktop acceptance + preview promotion, M5 production
gates, G1 audit, remote CI. Linux evidence does not certify
Windows native file-picker behavior — see the Windows
checklist in the delivery MANIFEST (dialog opens, .txt/.md
filter, cancel, unicode filename, same-name reselect,
large-file refusal, plus the focused Rust/UI/browser reruns).
No generated-code execution, pushes, master merges, CI or
preview promotion. Owner data/previews and Antigravity's
release tooling are untouched.
