# M7 Slice 3 addendum: a complete, reviewable export deliverable

Branch: `muse/m7-slice3-deliverable`, from the
Windows-reviewed Slice 2 checkpoint
`0f586f865a8c4d227729c53449ce171b03b18fe7`
(branch `codex/review-m7-slice2`), whose parent is the
Slice 2 delivery `9928bf5c`. The reviewed branch's
keyed revision panel and its browser regression are
preserved untouched (the canonical e2e subset below
includes the version-switch regression, passing).
Design (acceptance criteria written before coding):
`docs/handoffs/M7-slice3-deliverable-design.md`.

## What was built

The §33 deliverable workflow for an *unexecuted*
prototype — the honest subset of the §33 list, since
there is no build, test run, deployment or license to
report. The existing `forge-export` (package-only TAR)
is unchanged; this slice adds the saved deliverable
record and its archive:

- `forge-deliverable-draft-start` / `-poll` /
  `-cancel` (`crates/core/src/forge.rs`), riding the
  existing M5 substrate with unchanged Zero-Cost Lock,
  permits, budgets, cancellation and disclosure. The
  model receives the approved specification body and a
  package inventory (plan, file list with sizes,
  limitations, parent diff — not full file contents)
  and suggests five documents as strict JSON. The
  pinned specification version must hold backend
  approval **before any provider traffic**, as for
  build/revise; a draft saves nothing.
- `forge-deliverable-save` (a direct op, deliberately
  not a run-save, so user edits to a draft are
  saveable): validates the five fixed documents
  (`architecture.md`, `usage.md`, `limitations.md`,
  `security.md`, `changes.md`; origins `user`/`model`
  only, each 1–12000 characters), appends a sixth
  backend-computed document, `verified-facts.md`
  (origin `verified`: file inventory with per-file
  hashes, template, concept, parent diff paths,
  unexecuted status — and, by design, no artifact ids
  or body hashes, so it still matches a fresh
  recomputation after a restore remaps references),
  and registers a new `deliverable` artifact
  (ARTIFACT_TYPES 17→18, format
  `anansi-deliverable` v1) pinning the exact package
  version + recomputed package content hash and the
  pinned specification version + hash. The body,
  the view and the export manifest all carry
  `status: {executed:false, built:false, tested:false,
  published:false, productionReady:false}` and the
  banner "Unexecuted prototype — prepared for
  review." There is no license, deployment or build
  document anywhere in the slice.
- Documentation revision: save with
  `deliverableArtifactId` + `expectedVersion` revises
  via `artifact_revise_with_generator` (optimistic
  concurrency, as in Slice 2). A revision may never
  switch packages or package versions, and a saved
  deliverable never follows a newer package version.
- `forge-deliverable-export` (artifacts route family):
  re-reads and re-verifies the deliverable, the pinned
  package version (hash recomputed and compared), the
  pinned specification approval and the recomputed
  verified facts — any absence or mismatch fails
  closed — then builds a deterministic USTAR with
  the existing `build_tar` (fixed mtime 0, no
  timestamps in the manifest): source files
  byte-identical at their exact paths, documents
  under `deliverable-docs/`, and one
  `ANANSI-DELIVERABLE.json` manifest with ids,
  versions and per-file/per-document SHA-256.
  Exporting the same deliverable twice yields
  identical bytes.
- Collision handling, decided before code: documents
  live only under `deliverable-docs/`. A package that
  already uses that namespace (case-insensitive)
  refuses deliverable save and export; its bytes are
  never moved or rewritten. Document names are fixed,
  so documents cannot collide with each other.
- Restore: `remap_deliverable_references` rewrites
  the pinned package/specification ids fail-closed,
  and a new fix-up pass `fix_deliverable_hashes`
  (beside `fix_approval_hashes`) rewrites the pinned
  hashes to the restored rows' recomputed hashes, so
  the restored deliverable still exports and its
  manifest names the restored ids.
- Contracts/API/client: new
  `ForgeDeliverableDraftStartRequest`,
  `ForgeDeliverableSaveRequest`, `DeliverableDoc`,
  `DeliverableDocument`, `DeliverableStatus` schemas;
  routes `/v1/ai/forge-deliverable-draft-start|poll|
  cancel|save` and
  `/v1/artifacts/forge-deliverable-export`; client
  methods `startForgeDeliverableDraft` /
  `pollForgeDeliverableDraft` /
  `cancelForgeDeliverableDraft` /
  `saveForgeDeliverable` / `exportForgeDeliverable`;
  a `deliverable` stage in `ForgeRunController` with
  the same staleness discipline.
- UI: `DeliverablePanel` on a saved source package
  (keyed by workspace + project + package id +
  version, the reviewed Slice 2 keying lesson applied
  pre-emptively): banner, per-section textarea +
  origin badge, model suggestion with Zero-Cost Lock
  and cancel, Save, then Export after save.
  `DeliverableView` for a saved deliverable: inert
  document text with origins, verified facts, pinned
  versions, banner, Export, and "Revise
  documentation" (edits save as a new version).

## Verification (sequential, this branch)

- Forge units 11/11 (was 10: + draft parse/facts/
  collision/remap cases).
- `forge_deliverable` 6/6: manual save + deterministic
  export (manifest hashes match the tar entries;
  source bytes identical; no license/deployment
  names), model draft + edit + save (origins tracked;
  malformed draft invalid, nothing saved), approval
  gate (draft and save refused before provider
  traffic, wire capture unchanged), collision +
  missing/stale/verified-forgery/wrong-origin +
  revision-conflict + package-switch refusals,
  pinned version does not follow a newer package
  version, restart + fresh restore with remapped
  references, rewritten pinned hashes and continued
  exportability (manifest names restored ids).
- `forge_revision` 5/5, `forge_generation` 5/5,
  `forge_restore` 4/4 (unchanged).
- Full core 241/245 (only the 4 known environmental
  live-Ollama `semantic_workflow` failures; total +7
  over Slice 2's 238 = 6 deliverable tests + 1 unit
  test).
- Providers + API 144/0, node contracts/UI 60/60
  (was 57: + deliverable controller dispatch test, +
  2 panel/view rendering tests), `tsc` clean,
  contracts check PASS, docs PASS, Clippy
  `--all-targets -D warnings` (core, api, providers,
  contracts) + fmt clean.
- E2E canonical subset 12/12 (was 11: + new
  `forge-deliverable.spec.ts` — synthetic loopback
  provider, suggest → edit → save → view origins and
  verified facts → export via the API with archive
  assertions → revise documentation to v2 →
  restart/reopen → backup/fresh restore → reopen
  restored deliverable and export again), including
  the reviewed version-switch regression in
  `forge-revision.spec.ts`. One e2e test-only defect
  found and fixed in the spec: unscoped `getByText`
  assertions matched both the deliverable view's
  `<pre>` and the artifact revise textarea holding the
  raw body; assertions are now scoped to the view.
  No production code changed for it.

## Boundaries and open items (unchanged)

- No generated-code execution, iframe preview, shell
  commands, dependency installs, builds/tests of
  generated products, deployment or external fetching.
  No real credentials, paid/cloud inference, model
  downloads, system/security changes, pushes, master
  merges or remote CI. Antigravity review files
  untouched; advisory reviews are not execution
  approval.
- **Still open:** Windows verification of this slice;
  native desktop acceptance and preview promotion of
  Slice 2 and Slice 3 (the owner's preview branch is
  preserved, not promoted or replaced by this
  delivery); the §32 execution sandbox as a later
  separately approved slice; M5 production gates
  (real CredUI/keyring acceptance, live-provider
  verification, G1 audit, remote CI).

## Integration note for the owner

This branch's history is: reviewed Slice 2
`0f586f8` → Slice 3 work. The owner's preview branch
`codex/local-preview-m7` @ `3335248b` is included in
this delivery's bundle as a separate ref for
preservation, exactly as the reviewed bundle shipped
it; it is not merged, rewritten or promoted here.
Before promoting any preview, rebase or cherry-pick
as in the Slice 2 integration note. This delivery
must not be presented as Windows-verified or
preview-promoted.
