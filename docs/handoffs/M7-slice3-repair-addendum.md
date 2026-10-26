# M7 Slice 3 acceptance repair — addendum

Branch `muse/m7-slice3-repair`, from the reviewed tip
`bd72a4ffe89b3a73275200e499716077d0f45051` (review branch
`codex/review-m7-slice3`, one commit over delivered Slice 3
`cbb95a6960f0bb1379741bdc6a6a34e900611e18`). The review's
production code was unchanged; this repair changes production
code only in the three bounded areas below. No new feature, no
preview promotion, no generated-code execution, no push, master
merge or remote CI. The prior preview ref `codex/local-preview-m7`
at `3335248b` is preserved as a separate ref, not merged or
promoted. The reviewed Slice 2 keyed-panel fix is preserved
(the deliverable prepare panel was already keyed; this repair
applies the same discipline to the saved-document view).

## What was wrong (reproduced on Linux before any fix)

- `forge_deliverable review_`: 0 passed / 2 failed. Export
  accepted `["status", "specification", "duplicate-document",
  "unknown-format"]`; restore returned success for an
  all-zeroes pinned package hash.
- Browser `forge-deliverable-review.spec.ts`: 1 failed, only
  the soft version-switch assertion — selecting deliverable v1
  loaded the v2 architecture draft. Restart and restore
  assertions completed after it, so the overall result was
  still failed, exactly as the Windows review reported.

## Repair

Design: `docs/handoffs/M7-slice3-repair-design.md`.

1. **Stored-record validation at export** (`crates/core/src/forge.rs`,
   `validate_deliverable_record`, called by `export_deliverable`
   before any manifest or tar bytes are built). A registry hash
   proves body bytes are unchanged, not that the body is a valid
   domain record, so export now validates the full saved record
   against backend-resolved provenance: supported format/version,
   package/spec identity against the pinned package's own
   specification, concept provenance, pinned hash equality,
   all-five status flags false, exactly the five fixed documents
   plus verified facts each once, permitted origins, and the
   save text bounds for editable documents. Invalid records are
   refused, never silently repaired or relabelled. The generic
   artifact editor is not hidden; the consumption boundary is
   where the refusal belongs.
2. **Original-binding validation before restore remapping**
   (`crates/core/src/restore.rs`,
   `validate_original_deliverable_bindings`, run over the source
   catalog before any id or hash changes and before any
   destination state is written). For every deliverable row it
   checks the row's own hash, the pinned package and
   specification rows' existence, types, workspace/project
   ownership and row hashes, then reuses the shared record
   validator against the original hashes. Only valid originals
   are remapped, and their pinned hashes are then recomputed
   to the restored rows' hashes as before. `fix_deliverable_hashes`
   and approval transfer are unchanged for valid records.
3. **Saved-document editor identity** (`packages/ui/src/Projects.tsx`,
   `packages/ui/src/Forge.tsx`). `DeliverableView` is now keyed
   by `deliverable:{workspace}:{project}:{artifact}:{version}`,
   so a version or record switch remounts it and discards the
   previous buffer and editing state. Inside the component, an
   effect resets the buffer (and leaves editing) on identity
   change, and Cancel editing resets the buffer to the viewed
   document — a cancelled or superseded draft never carries
   into another version or record.

## Tests

- Both review tests pass unchanged in substance; the restore
  test gained no-mutation controls (no destination workspace
  named for the refused restore, a second attempt with its own
  landing workspace also refuses with no restored workspace,
  source versions still exactly 2, source export still refuses).
- New `export_rejects_other_inconsistent_records`: each defect
  isolated from the original v1 body — `executed`, `built`,
  `tested`, `published` true; specification version mismatch;
  missing document; extra document; invalid origin `system`;
  `verified` origin on an editable document; empty document.
  All refuse export.
- Browser: the review spec is deleted; its version-switch probe
  is consolidated into the canonical `forge-deliverable.spec.ts`
  as a hard assertion (the review's soft assertion was
  diagnostic only). Restart and fresh-restore coverage is
  unchanged.

## Linux evidence (sequential, idle machine)

- Full core: **244/248** — only the 4 known environmental
  live-Ollama `semantic_workflow` failures (same four as Slice 3).
  Deliverable suite: **9/9** (6 original + 2 review + 1 new).
- Providers + API: **144/0**. Node UI/contracts: **60/60**.
  TypeScript clean; contracts drift check exit 0; docs PASS
  (116 Markdown files after this addendum).
- Strict Clippy `--all-targets -D warnings` (core/api/providers/
  contracts) clean; `cargo fmt --check` clean.
- API binary + web bundle builds exit 0.
- E2E canonical **12/12** (1.7 min), including the consolidated
  deliverable workflow (7.8 s; the failing review probe had
  taken 39 s on its 30 s soft-assertion timeout) and the
  reviewed revision version-switch regression.
- Logs: `~/workspace/verify-m7-slice3-repair-2026-10-26/`
  (kept outside the repo, as with prior slices).

## Still open (unchanged by this repair)

Windows verification of this repair checkpoint (independent
rerun required); native desktop acceptance and preview
promotion of Slices 2–3; Windows build/launch verification of
the R17 preview checkpoint; M5 production gates (Windows
native credential acceptance incl. real CredUI, live-provider
verification, G1 audit, remote CI); §32 execution sandbox as a
later, separately approved slice. Linux results above do not
close any of these.
