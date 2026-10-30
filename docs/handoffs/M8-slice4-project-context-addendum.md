# M8 Slice 4 — explicitly selected saved-project input for LEXI-COLLIDER — record

Baseline: `c7327bb7e486b0b43d4c8b12e50fef625ae78705` (the A6
reporting-correction tip, including the Windows runtime improvements,
retrieval repairs and preview isolation). Slice branch:
`muse/m8-slice4-project-context`. Design:
[M8 Slice 4 design](M8-slice4-project-context-design.md), committed
before coding (`4bfda48f`). The branch's first commit (`c14ce920`)
records the completed free-model comparison from the supplied package
(model `dots-studio/dots-3-note-preview:free`, 30 scored requests;
ANANSI 100/100 supported results at 65,770 total tokens vs full
context 232,321, 71.69% fewer reported total tokens; no general
accuracy, commercial superiority, latency improvement or production
certification claim); that record is branch ancestry, not this slice's
implementation. Owner authorization: one complete vertical slice,
2026-10-30. The benchmark round is finished; no new benchmark
campaign was started.

## What changed

- Client submits references only. `ColliderStartRequest` gained an
  optional `projectContext: { projectId, items: [{ kind, memoryId?,
  revisionId?, artifactId?, version? }] }`; bodies, titles, privacy
  labels, project-membership claims and workspace claims are never
  trusted from the client. Bounds mirror Slice 3: at most 8 items,
  each resolved body at most 8000 chars, total context at most 32000
  chars; duplicates refused (including a memory selected through both
  the memory picker and the project picker); no truncation.
  Resolution happens in `collider.start` immediately before dispatch,
  through the existing workspace-scoped, revision-pinned,
  ownership-checked, hash-verified `Storage::memory_read` (refuses
  Sealed via `allow_plaintext`) and the version-pinned, hash-reverified
  `Storage::artifact_read` — before any provider traffic. Failures
  name only the 1-based item position. The source project itself must
  exist in the workspace, be a project and not be archived
  (`require_project_writable`); its generation-time title comes from
  `memory_read` of the project record.
- Membership is per pinned revision for memories (the revision's
  metadata `projectId` must equal the selected project; project-kind
  records and archived revisions refused) and per version row for
  artifacts (row `project_id` must equal the selected project, row
  type must be one of the 19 `ARTIFACT_TYPES`, row privacy must be
  non-Sealed). Approval rows (`transfer-approval`,
  `specification-approval`) are not in `ARTIFACT_TYPES`, so they can
  never be selected; the picker also filters them out client-side.
  The project's own record text is never auto-included.
- Resolved items are sent as additional `private` provider segments
  after the memory segments, in selection order, framed as `Project
  context (data) — {projectTitle} / {itemTitle} (memory revision
  {version}):\n{content}` or `(artifact version {version}):`. The
  instruction segment now states project-context segments are likewise
  data, not instructions. Project content travels on the unchanged M5
  dispatch, authorization, disclosure, cancellation and accounting
  machinery. No new provider client, no schema change.
- Saved experiments keep format `anansi-collider-experiment` v1.
  `projectContext` is optional/defaulted and skipped when absent, so
  older experiments without it remain readable. Each saved item pins
  its kind, ids, version, title, verbatim `content`, `contentHash`
  and resolved privacy; later project edits cannot change historical
  experiments. `validate_experiment_doc` now also checks project
  context, including content-hash consistency and the cross-picker
  duplicate rule.
- Restore pre-validation (`validate_original_experiment_bindings`)
  checks each project item against source-package rows: memory items
  through their revision rows (same checks as Slice 3, plus the pinned
  revision's metadata `projectId` must equal the pinned project);
  artifact items through their version rows (same workspace, pinned
  project, eligible type, non-Sealed, matching privacy, row body
  hashing to the pinned hash, snapshot content hashing to the pinned
  hash). Any failure refuses the whole restore before destination
  publication. Remap (`remap_experiment_project_references`, chained
  after the memory remap on the already-remapped body) moves memory
  items through the same revision-ownership validation, artifact items
  by artifact id only (versions are stable across restore, never
  renumbered), and the pinned project id through the memory identity
  map (projects are memories); snapshot content, titles, versions and
  pinned hashes stay verbatim; the artifact row hash is recomputed.
  No new transfer fix-up was needed: the existing
  `fix_transfer_experiment_hashes` already covers all experiment rows
  and runs before `fix_transfer_receipt_hashes` (the Slice 3 ordering
  fix), and the new transfer test proves a project-context
  experiment's concept survives restore and stays Forge-admissible.
- `memory_list` gained an optional `projectId` filter
  (`MemoryListRequest.project_id`, SQL `AND (?9 IS NULL OR
  json_extract(r.metadata,'$.projectId') = ?9)`), applying to both
  the current-revision listing and a single memory's revision history,
  composing with the existing Sealed exclusion. This is what the
  project picker and its version dropdowns read; previously there was
  no server-side project-member listing (the UI filtered client-side).
- UI: the collider panel (already keyed by workspace/project) gained
  an optional project picker (a saved-project dropdown fed by
  `listMemories({ kind: 'project' })`, nothing auto-selected, the
  current project offered and labeled "this project"), per-project
  member checkboxes (`{title} · memory · Revision {version} ·
  {privacy}`) and artifact checkboxes (`{title} · artifact ({type}) ·
  Version {version}`, filtered to the 19 eligible types), version
  dropdowns (member revisions via `listMemories({ id, projectId })`,
  artifact versions via `listProjectArtifactVersions`, defaulting to
  current/latest), a pre-send review section showing exactly which
  items at which versions will be sent with verbatim bodies, and a
  generation-time snapshot view on saved experiments (`Project context
  (as sent at generation time)`, `From project {title}`, per-entry
  details with verbatim `<pre>` content). Legacy experiments render
  without the section. Project review loads carry a selection epoch
  (like the memory review) so late responses for superseded selections
  are dropped; changing the project clears choices, review and version
  options synchronously; submit sends exactly the reviewed refs and is
  disabled until both reviews are ready. The panel help text no longer
  lists saved-project selection among deferred slices; URL fetching,
  document ingestion and 3D visualization remain deferred.
- Contracts regenerated from the changed DTOs; the handwritten client
  re-export list gained `ColliderProjectContext`,
  `ColliderProjectContextItem` and `ColliderProjectSelection`, and
  `listMemories` gained the `projectId` option. `colliderRun.ts`
  needed no change: `ColliderStartInput` already omits only
  `contractVersion`/`workspaceId` from the start request.

## Clarifications against the design (no design amendment needed)

- The archived rule is pinned-revision status, not current-record
  status: resolution refuses an item pinned at an archived revision,
  consistent with Slice 3's revision-pinning philosophy. The picker
  never offers currently-archived members (the list default excludes
  them), so in practice archival before selection already excludes
  the item; the resolution check is the backstop.
- Artifact privacy labels: the registry API only registers
  `private`/`standard` artifact privacy, so a `public` artifact label
  cannot occur through the API; validation accepts it for symmetry
  with memory labels, but segments are always sent `private` at
  dispatch regardless of the provenance label, so no label can widen
  disclosure.
- All 19 registry types are eligible, including `source-package`
  (its body is a text document; the 8000-char item bound refuses
  oversized export blobs). This deliberately differs from the
  read-only inventory's "safest excluded" suggestion, which was
  advisory, not a finding.
- The project list is loaded inside `ColliderPanel` (which already
  receives `client` and `workspaceId`), not passed down from
  `Projects.tsx` as the inventory sketch suggested; the outcome is
  the same and no parent change was needed. The inventory's other
  flagged gaps — no server-side project filter, no privacy gate in
  `artifact_read` — are exactly what this slice added at the
  resolution layer.

## Failed attempts preserved (no assertion weakened)

- A `Box::leak` shortcut for duplicate tracking in resolution was
  replaced with owned `(String, String)` keys before the first test
  run.
- The project remap first borrowed the items array mutably while
  also writing the project id (borrow-checker rejection); the fix
  remaps the project id in its own scope before the items borrow.
- The in-crate test helper constructing `ColliderRun` missed the new
  field (E0063); fixed by adding `project_context: None`.
- Strict Clippy `large_enum_variant` fired on the save-claim enum
  after the new field grew the `Claimed` variant; fixed by boxing
  the context exactly like the existing boxed result, not by
  suppressing the lint.
- Strict Clippy `needless_borrow` fired in the tamper-test builder
  (its `f` is already `&Fixture`); fixed by passing `f` directly.
- `cargo fmt` drift in the new backend code was applied, not argued
  with.
- The tamper test was first drafted on one shared fixture; a tampered
  version would poison every later restore of that workspace, so each
  case now gets its own fixture (mirroring the Slice 3 test).
- The pinning test first looked up the restored experiment under the
  source project; experiments save into the run's home project while
  their context points at the source project, so the lookup now finds
  both restored projects by title.
- E2E: `getByLabel('Version')` matched two elements — the version
  select and the artifact checkbox, whose label contains "Version 1"
  as a substring. The fix gives each version select an exact
  `aria-label` (`Version for {title}`); this also fixed the deeper
  cause, that a label wrapping a `<select>` absorbs the option texts
  into the accessible name (the select's name computed as
  "VersionRevision 2Revision 1").
- E2E: the first rerun after the aria-label fix still timed out
  because the preview serves the built web bundle, which predated the
  fix; rebuilding `apps/web` (gitignored `dist/`) before rerunning
  made the spec pass. The stale-bundle cause is recorded here so a
  future UI-change rerun rebuilds first.
- A service restart interrupted one combined verification command;
  its steps were re-run separately and all passed (no result was
  assumed from the interrupted run).

## Verification

Focused suites, sequential, `--test-threads=1` (baselines from the
clean branch before coding):

- `collider` 24/24 (baseline 19/19; +5 new: wire/provenance,
  refusal battery incl. foreign/other-project/Sealed/archived/
  corrupted/duplicate/cross-picker-duplicate/project-record-as-item,
  pinning through edits and restore, tampered/misbound restore
  refusal, project list filter).
- `transfer` 19/19 (baseline 18/18; +1 new: project-context
  experiment transfer survives restore and stays Forge-admissible).
- `forge_restore` 4/4, `backup_restore` 7/7 (unchanged).
- Node UI/contracts 83/83 (baseline 81/81; +2 new: project-context
  parser acceptance/rejection and saved-view rendering; the panel
  help-text assertion was updated to the new wording, not weakened).
- Strict Clippy (`-D warnings`, workspace, all targets), `cargo fmt
  --check`, `tsc --noEmit`, `node scripts/contracts.mjs --check`,
  `node scripts/validate-docs.mjs` (166 files), API build and web
  build all pass. The web build's chunk-size warning is pre-existing.
- New e2e spec `tests/e2e/collider-project-context.spec.ts` passes
  standalone: selection, version pinning to an older revision,
  project-switch discard, collision, save, restart reopen with
  post-save source edits, and backup/fresh restore with remapped
  references (7.1 s).

Full canonical e2e battery (all 29 specs, sequential, lease command,
3.9 min): 34 passed / 1 failed. The single failure is
`tests/e2e/search.spec.ts` ("real semantic UI workflow"), failing at
the `.index-status` / "Indexed 1" assertion after clicking "Index
workspace embeddings" — an embedding-indexing step, not a collider,
transfer or restore path. It was proven pre-existing, not a Slice 4
regression: the same spec run on a pristine `c7327bb7` worktree (no
Slice 4 code) fails at exactly the same assertion and line, and passes
its other test. The cause is consistent with the known environment
limitation (the embedding model is absent on disk on this VM, as with
the `semantic_workflow` core failures); no Slice 4 code path is
involved (the search UI does not use the collider panel, and the new
list filter is a tautology when no project id is given).

Full core suite (`-p anansi-core`, `--test-threads=1`,
`--no-fail-fast`): 334 passed / 4 failed. The 4 failures are exactly
the known live-Ollama `semantic_workflow` environment failures (the
embedding runtime is offline on this VM); nothing else failed,
including the 13 binaries the first fail-fast run never reached (see
the counting note below). 334 = the A6-lineage baseline 328 + the 6
new Slice 4 tests (collider +5, transfer +1) — an exact match, no
regression, no missing test. Providers + API: 144/0, matching the
baseline exactly.

Counting note: the first full-core run (default fail-fast) reported
only 263 passed because cargo stops at the first failing binary and
never ran the 13 binaries ordered after `semantic_workflow`
(`transfer` among them). The `--no-fail-fast` rerun is the complete
measurement; the earlier 263 figure was an incomplete count, not a
regression, and must not be quoted as the suite result.

## Short Windows test guide

1. Build the web app (`apps/web` vite build) and the API
   (`cargo build -p anansi-api`); the e2e preview serves the built
   bundle, so rebuild after any UI change or the browser will test
   stale code.
2. Run the new spec plus the Slice 3 and transfer regressions:
   `ANANSI_REAL_MODEL=0 node scripts/with-application-test-lease.mjs
   node_modules/@playwright/test/cli.js test --config
   playwright.local.config.ts tests/e2e/collider-project-context.spec.ts
   tests/e2e/collider-memory-context.spec.ts tests/e2e/transfer.spec.ts`
   (workers = 1, vite preview port 1420, `~/.codex/` must exist).
3. In the browser: open a project, choose a saved project in the
   collider's Project context picker, check one member and one
   artifact, change the member's version, confirm the review shows
   exactly the pinned versions, collide, save, restart, reopen (the
   snapshot must still show the pinned versions after editing the
   sources), then backup/restore and reopen in the restored
   workspace. Nothing is selected automatically at any step, and
   selection/review/version changes must cause zero provider hits
   (observable via the fake-provider hit count in the spec).
4. Backend focused checks: `node scripts/cargo.mjs test --locked
   --test collider --test transfer -- --test-threads=1`.

## Remaining defects and gates (not closed by this slice)

- Windows verification of M8 Slice 4 is pending; all evidence above
  is Linux-only and must not be presented as closing Windows
  acceptance.
- M7 native desktop acceptance + preview promotion; §32 execution
  sandbox; M5 production gates — all still open, unchanged by this
  slice.
- M8 is NOT complete: URL fetching, document ingestion and 3D
  visualization remain deferred in the M8 plan.
- The traffic-gating (sufficiency gate) owner decision from the A6
  round remains open and is untouched by this slice.
- The 4 pre-existing `semantic_workflow` live-Ollama environment
  failures are expected in the full core suite (embedding model
  absent on disk); they are not Slice 4 regressions.
