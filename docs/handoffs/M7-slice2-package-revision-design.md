# M7 Slice 2 design: revising an existing source package

Status: design, written before coding (2026-10-26). Baseline:
`518418b6a955feaadcc6212eab0a3f3fd1e1bcb9` (M7 Slice 1 repair,
Windows-verified on the owner's side for the supplied suites).
The owner's Windows preview commit `3335248` on
`codex/local-preview-m7` is not present in this machine's
repository, so this slice branches from `518418b6` directly;
the preview-integration requirement is carried into the
handoff instead of being silently dropped (see §8).

## 1. Selection: why this slice

`plans/05-product-forge.md` lists, as required scope, "specs,
repository, generation, tests/build, preview, revision, export".
Slice 1 implemented concept → specification → plan → source
package → inert inspection → versioned save → export-for-review.
The §30 pipeline's **Revision** stage has no implementation, and
no later non-executing Forge requirement is smaller or more
coherent: revision reuses every Slice 1 protection (M5 provider
machinery, exact-version pinning, specification approval, the
shared portable-path validator, immutable artifact versions,
backup/restore remapping) while adding exactly one new workflow.
The §32 execution sandbox, additional templates, build/test
stages and deployment remain explicitly out of scope: they are
later, separately approved slices, and nothing here executes,
previews, builds or tests generated code.

## 2. Workflow

1. The user opens a saved `source-package` artifact at an exact
   version (the parent version) in a project.
2. The user writes bounded revision instructions (1–2000
   characters) and starts `forge-revise-start`. The backend
   checks, **before any provider traffic**, that the package's
   pinned specification version still holds a backend
   `specification-approval` record bound to its exact content
   hash. A package whose pinned specification was never
   approved (for example a manually registered package) is
   refused.
3. The model receives: the revision instruction (data framed as
   the revision task), the pinned specification body, and the
   parent package's plan and files — all as data, never as
   instructions to follow. Generated output is parsed with the
   existing `parse_source_package` and validated with the
   shared `validate_package_files` (counts, sizes, portable
   paths, reserved names, prefix conflicts, root
   `index.html` + `README.md`).
4. Polling returns the proposed package **and** a server-side
   diff against the parent: `added`, `removed` and `changed`
   paths (same path with different content counts as changed).
   The UI shows the diff and the full proposed files as inert
   text, labelled unexecuted and unverified, with the pipeline
   stages shown as generated — not saved.
5. **Accept** saves a new version of the *same* artifact via
   the registry's optimistic revise: `expectedVersion` is the
   parent version, so a second revision that landed first makes
   the save fail with a revision conflict and the user retries
   against the new version. The new version's body keeps the
   exact specification references of the parent (no spec switch)
   and adds `parentPackageArtifactId`,
   `parentPackageVersion` and `revisionInstructions` (verbatim).
   Accepting does not create, transfer or renew any
   specification approval: approval belongs to specification
   versions, and the revised package pins the same one.
6. **Reject** calls no save operation: nothing is persisted,
   the parent version is untouched, and the UI says so
   explicitly. Cancellation follows the existing run discipline
   (stream cancel, no UI Cancelled event while backend work
   remains active); a cancelled run cannot be saved.
7. Restart: runs are in-memory (as in Slice 1), so a restart
   discards in-flight revisions but every saved version
   persists in the registry and reopens exactly.
8. Backup/restore: the revised version travels with the
   package; restore remaps `parentPackageArtifactId` through
   the restore identity map (fail closed if the parent is not
   in the package; the common case — parent is the package
   itself — maps to the restored package id) and remaps the
   specification reference as in Slice 1. Approval records
   travel as in the Slice 1 repair, so the pinned
   specification version stays approved in the restored
   workspace without re-approval.

Linear history is a stated limitation of this slice: a revision
is requested against a version and saved only while that
version is still the latest. Branching from an old version is
not supported; the save conflict tells the user to retry
against the current version.

## 3. Operations and DTOs

New `ai_command` operations, mirroring the build ops:

- `forge-revise-start` (`ForgeReviseStartRequest`:
  workspace, project, `packageArtifactId`, `packageVersion`,
  `instructions`, model/provider/mode, zero-cost lock,
  permit) → `ForgeStartResponse` (run id + disclosure).
- `forge-revise-poll` / `forge-revise-cancel` /
  `forge-revise-save` (`ForgeRunRequest`) → `ForgePollResponse`
  (now carrying optional `revisionDiff: RevisionDiff` and the
  proposed package in `sourcePackage`) / `ForgeSaveResponse`.
- API routes `/v1/ai/forge-revise-start|poll|cancel|save`;
  generated contracts and client methods
  `startForgeRevise` / `pollForgeRevise` / `cancelForgeRevise` /
  `saveForgeRevise`; `forgeRun.ts` gains a `revise` stage.

`SourcePackageDoc` gains three optional fields
(`parentPackageArtifactId`, `parentPackageVersion`,
`revisionInstructions`), defaulting to absent so Slice 1
packages parse unchanged. `RevisionDiff` is
`{ added: string[], removed: string[], changed: string[] }`,
each list sorted by path.

## 4. Requirement mapping

| Requirement | How this slice honours it |
|---|---|
| MASTER_SPEC §28 (Forge converts concepts to projects) | Revision stays inside the same project and template; no new product types. |
| §29 (essential questions only) | One bounded instructions field; no new forms. |
| §30 (pipeline, statuses, never "completed" without a build) | Revision stage added; generated output is "generated — not yet saved"; build/test/preview/deployment stay Unavailable with the §32 reason; nothing is marked built or completed. |
| §31 (agent roles) | Revision uses the product-engineer role on the existing single-model orchestration; no new paid models. |
| §32 (no unrestricted execution) | No execution, iframe preview, shell, install, build or test of generated code. Export-for-review remains the deliverable. |
| §33 (deliverables) | Revised packages keep plan, files, limitations and README; no invented license or research/build/test claims. |
| AN-012 (Product Forge, in progress) | Next vertical step; AN-012 stays in progress. |
| AN-024 (universal projects/artifacts/versioning) | Same artifact, new immutable version; optimistic concurrency; history readable per version. |
| AN-019 (backups/restoration) | Parent/spec references remapped fail-closed on restore. |
| AN-026 (Zero-Cost Lock, budgets, disclosure) | Unchanged M5 gates on the revise stream; disclosure returned on start and poll. |
| AN-018/031 (privacy, vault) | Package/spec privacy classes carried into provider segments; sealed inputs refused by existing gates; no credential material. |

## 5. Status-copy fix (owner-observed)

The pipeline display previously listed "Product specification,
Development plan, Source generation" as saved stages even for
an unsaved draft: `SourcePackageView` rendered
`PipelineStages` with a fixed saved list. Fix: `PipelineStages`
takes both saved and generated stage lists; the build panel
passes the generated stages until the save succeeds and then
the saved stages (and the saved artifact, which also enables
export from the panel). A saved package viewed from the
Artifacts list already passes its artifact, so it keeps
showing saved stages. Wording: "Generated — not yet saved as
a project artifact." until persistence succeeds.

## 6. Acceptance criteria

1. Open package v1 → revise with instructions → diff shows
   added/removed/changed paths correctly → reject → artifact
   still has exactly one version and v1 bytes unchanged →
   revise again → accept → artifact version 2, body pins the
   same specification id/version, `parentPackageVersion` = 1,
   `revisionInstructions` verbatim.
2. Revise-start is refused before provider traffic when the
   pinned specification version holds no approval record.
3. Revising a package after its specification was revised to
   a new version still pins the package's original
   specification version; no approval is created by the
   revision (artifact count of `specification-approval`
   artifacts is unchanged by revise-save).
4. Malformed, oversize, traversal, reserved-name and
   prefix-conflict model output is refused as an invalid run
   and cannot be saved; nothing is persisted.
5. Cancellation during streaming ends with no saved version
   and no Cancelled display while backend work is active.
6. Two revise runs from the same parent: first save wins;
   second save fails with a revision conflict.
7. Restart: saved v2 reopens with revised content and intact
   parent/spec references.
8. Backup → fresh restore: restored package v2 resolves to
   the restored package (parent) and restored specification,
   with recomputed hashes; approval status for the pinned
   specification version is approved in the restored
   workspace without re-approval.
9. Export of a revised version validates with the shared
   validator before any tar is built and produces the same
   deterministic format as Slice 1.
10. Full browser workflow (synthetic loopback provider): open
    → revise → inspect diff → reject → retry → accept →
    restart/reopen → backup/fresh restore, all in the UI/API
    path, with generated code shown only as inert text.
11. No regressions: existing Forge, provider, artifact and
    recovery suites; contracts, TypeScript, strict Clippy,
    formatting and docs validation pass; only the known
    environmental live-Ollama core failures remain.

## 7. Tests

- New Rust integration suite `forge_revision.rs`: workflow,
  approval gate (unapproved package, spec-revised case,
  approval-count invariant), rejection (no save, bytes
  unchanged), cancellation, invalid outputs (traversal,
  reserved name, prefix conflict, oversize), stale-version
  conflict, restart persistence, backup/fresh restore
  (parent + spec remap, approval transfer).
- `forge_restore.rs`: one added test for a manually built
  v1→v2 revised package (parent reference remapped on
  restore).
- Forge units: `package_diff` cases (added, removed,
  changed, unchanged, ordering) and revised-doc parsing
  (optional fields absent = Slice 1 package).
- Node: `forge-run.test.ts` revise stage dispatch;
  `components.test.tsx` revise-panel diff/accept/reject
  rendering.
- E2E: new `forge-revision.spec.ts` (synthetic loopback
  provider with a distinct revision response) covering the
  §6.10 browser workflow including restart/reopen and
  backup/fresh restore through the jobs API, then reopening
  the restored workspace in the browser.

## 8. Integration and boundaries

- Branch: `muse/m7-slice2-package-revision` from `518418b6`.
  The owner's Windows preview branch `codex/local-preview-m7`
  @ `3335248` (preview materials + Windows evidence only,
  application code unchanged) is **not** in this bundle's
  history, because that commit is unavailable on this
  machine. The owner must integrate before promoting: either
  fast-forward/rebase the preview branch onto this slice's
  tip and re-run the preview checkpoint, or cherry-pick this
  slice onto `3335248`. This slice must not be presented as
  Windows-verified or preview-promoted.
- Boundaries unchanged from Slice 1 + repair: no generated-
  code execution, no iframe preview, no dependency installs,
  builds or tests of generated products, no real credentials,
  paid/cloud inference, model downloads, deployment, pushes,
  master merges or remote CI. Building and testing ANANSI
  itself remains allowed. Antigravity review files are not
  edited and advisory reviews are not execution approval.
- Open and unwaived: §32 execution sandbox (later slice),
  M5 production gates (real CredUI/keyring acceptance,
  live-provider verification, G1 audit, remote CI), Windows
  verification of this slice.
