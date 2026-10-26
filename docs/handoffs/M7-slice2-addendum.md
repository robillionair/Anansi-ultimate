# M7 Slice 2 addendum: revising an existing source package

Branch: `muse/m7-slice2-package-revision`, from the
Windows-verified M7 Slice 1 repair tip
`518418b6a955feaadcc6212eab0a3f3fd1e1bcb9`.
Design (acceptance criteria written before coding):
`docs/handoffs/M7-slice2-package-revision-design.md`.

## What was built

The §30 pipeline's Revision stage, as the next coherent
non-executing Forge workflow (the §32 execution sandbox,
additional templates, build/test stages and deployment
remain later, separately approved slices):

- `forge-revise-start` / `-poll` / `-cancel` / `-save`
  (`crates/core/src/forge.rs`), riding the existing M5
  provider machinery with unchanged Zero-Cost Lock,
  permits, budgets, cancellation and disclosure. The model
  receives the user's bounded revision instructions
  (1–2000 characters), the pinned specification body and
  the parent package's plan and files — all framed as data,
  never instructions.
- The pinned specification version must hold a backend
  `specification-approval` record bound to its exact
  content hash **before any provider traffic**, exactly as
  for a build. A manually registered package whose pinned
  specification was never approved is refused, with the
  provider wire capture unchanged. A revision creates no
  approval, transfers none, and switches no specification:
  the revised body pins the parent's exact specification
  id and version even after that specification was revised
  to a new version.
- Polling returns the proposed package plus a backend-
  computed diff (`RevisionDiff`: sorted `added`,
  `removed`, `changed` paths; same path with different
  content counts as changed). The UI shows the diff and
  the full proposed files as inert text, with pipeline
  stages shown as generated — not saved — until the save
  succeeds (see the status-copy fix below).
- **Accept** saves a new version of the *same* artifact
  via a new `Storage::artifact_revise_with_generator`
  (the generic revise keeps the latest version's generator;
  the Forge path records the revising run). Optimistic
  concurrency: `expectedVersion` is the parent version,
  so a second revision that landed first fails with a
  revision conflict. The new version's body adds
  `parentPackageArtifactId`, `parentPackageVersion` and
  `revisionInstructions` (verbatim) and keeps the parent's
  title, so the artifact list is stable across versions.
  **Reject** calls no save operation: nothing is
  persisted and the parent version is untouched by
  construction. A cancelled run cannot be saved.
- Restore remaps `parentPackageArtifactId` through the
  restore identity map, fail-closed when the parent is not
  in the package (the common case — parent is the package
  itself — maps to the restored package id); specification
  references remap as in Slice 1 and approval records
  travel as in the repair, so the pinned specification
  version stays approved in the restored workspace without
  re-approval.
- `SourcePackageDoc` gained three optional fields, absent
  for Slice 1 packages, so existing packages parse
  unchanged. Export of a revised version reuses the shared
  validator before any tar is built.
- Contracts/API/client: new `ForgeReviseStartRequest` and
  `RevisionDiff` schemas, routes
  `/v1/ai/forge-revise-start|poll|cancel|save`, client
  methods `startForgeRevise` / `pollForgeRevise` /
  `cancelForgeRevise` / `saveForgeRevise`, and a `revise`
  stage in `ForgeRunController` with the same staleness
  discipline as spec/build.
- UI: `ForgeRevisePanel` on the Artifacts view of a
  source package (open package → instructions → proposed
  diff → accept/reject), with the parent, pinned
  specification and no-execution wording stated on the
  panel. After an accepted revision the panel reloads the
  artifact to its new version.

## Status-copy fix (owner-observed)

The pipeline display previously listed "Product
specification, Development plan, Source generation" as
saved stages even for an unsaved draft. `PipelineStages`
now takes both saved and generated stage lists;
`SourcePackageView` shows generated stages ("Generated —
not yet saved as a project artifact.") when no artifact is
attached and saved stages when one is. The build panel
passes its saved artifact after a successful save (which
also enables export from the panel). A saved package
viewed from the Artifacts list already passed its
artifact, so it keeps showing saved stages.

## Verification (sequential, this branch)

- Forge units 10/10 (was 8: + `package_diff` cases,
  + revised-doc parse/restore-remap cases).
- `forge_revision` 5/5: accept saves v2 with diff and
  parent/spec refs, reject saves nothing (byte-identical
  v1), approval gate (unapproved package refused before
  provider traffic, spec-revised case, approval-count
  invariant, empty/over-long instructions, wrong-stage
  save), invalid outputs (traversal, reserved name,
  prefix conflict, oversize file, malformed, refusal) and
  cancellation save nothing, stale-parent save conflicts,
  restart persistence, backup/fresh restore with remapped
  parent/spec and transferred approval.
- `forge_generation` 5/5, `forge_restore` 4/4 (was 3:
  + manually built v1→v2 revised package restores with
  remapped parent reference and still exports).
- Full core 234/238 (only the 4 known environmental
  live-Ollama `semantic_workflow` failures; total +8 over
  the repair's 230 = 5 revision tests + 1 restore test +
  2 unit tests).
- Providers + API 144/0, node contracts/UI 57/57 (was 55:
  + revise controller dispatch test, + revise panel /
  pipeline-stage test), `tsc` clean, contracts check
  PASS, docs PASS, Clippy `--all-targets -D warnings`
  (core, api, providers, contracts) + fmt clean.
- E2E canonical subset 11/11 (was 10: + new
  `forge-revision.spec.ts` — synthetic loopback provider,
  open → revise → inspect diff → reject → retry → accept
  → restart/reopen → backup/fresh restore through the job
  routes → reopen the restored workspace in the browser),
  including the existing `forge.spec.ts` unchanged in
  outcome. Two e2e test-only defects found and fixed in
  the spec: the upload-begin response carries no artifact
  id (the finished upload-part's job does), and preview/
  restore address the import job id, not the upload id.
  No production code changed for either.

## Boundaries and open items (unchanged)

- No generated-code execution, iframe preview, shell
  commands, dependency installs, builds or tests of
  generated products. Build/test/preview/deployment stay
  Unavailable with the §32 reason; nothing is marked
  built or completed. Export-for-review remains the
  deliverable.
- Linear history is a stated limitation: a revision is
  requested against a version and saved only while that
  version is still the latest; branching from an old
  version is refused with a revision conflict.
- No real credentials, paid/cloud inference, model
  downloads, deployment, pushes, master merges or remote
  CI. Antigravity review files untouched; advisory reviews
  are not execution approval.
- **Still open:** Windows verification of this slice;
  Windows preview promotion (see the integration note
  below); the §32 execution sandbox as a later separately
  approved slice; M5 production gates (real CredUI/keyring
  acceptance, live-provider verification, G1 audit, remote
  CI).

## Integration note for the owner

The owner's Windows preview branch `codex/local-preview-m7`
@ `3335248` (preview materials + Windows evidence only,
application code unchanged) is **not** in this bundle's
history: that commit is unavailable on this machine, so
this slice branches from the repair tip `518418b6`
directly, as the assignment's fallback allowed. Before
promoting, either fast-forward/rebase the preview branch
onto this slice's tip and re-run the preview checkpoint,
or cherry-pick this slice onto `3335248`. This delivery
must not be presented as Windows-verified or
preview-promoted.
