# M8 Slice 2 design: explicitly approved LEXI-COLLIDER → Product Forge transfer

Baseline: the Slice 1 delivery `muse/m8-slice1-collider` @
`8718df8a7f3def7f5322e3adbb44d3300cc24f34`
(ZIP SHA-256
`deb3103df01ab619a5079445019fd1145cc0182da996b640e76a40d21ff58c9f`,
bundle SHA-256
`67211ec01059e9fcd9f62aeea222331af2e622244d4677f16615a9a7b84786ee`,
verified in a scratch repo before import:
bundle verifies, scratch fetch lands exactly on
`8718df8a`, `08465172a` confirmed ancestor).
Branch `muse/m8-slice2-forge-transfer` from that
exact tip. All Windows review corrections
(`befc66f11`) are preserved. Slice 1 has Linux
evidence only; this slice permits independent
development and does not establish Windows
acceptance or authorize preview promotion.

Authority: MASTER_SPEC §9 (one universal project
model; approved context transfers internally, never
by manual copying), §§23–27 (Collider; §27: the
user selects Build This Product and that creates a
Product Forge project from approved outputs),
§§28–33 (Forge pipeline; §32 execution boundary
unchanged — nothing here executes, previews or
deploys generated code).

## Inventory (what already exists, reused as-is)

- Collider experiment: artifact type `experiment`,
  body format `anansi-collider-experiment` v1
  (beams, resolved controls, six-section result,
  provider/model, full disclosure). Validated by
  `validate_experiment_doc`; restored under the
  Slice 1 guard (only format-matching rows gated).
- Artifact registry: versioned artifacts with
  content hashes, `artifact_read` at an exact
  version, `artifact_list` per project,
  `artifact_register_generated` with a generator
  value and validation status. Type `concept`
  already exists in ARTIFACT_TYPES (19 types) but
  no code path produces or consumes it yet.
- Forge spec input: `forge-spec-start` takes a
  saved source artifact + exact version +
  `concept_index` + the §29 essential answers,
  and currently accepts ONLY type `idea`,
  parsing it with `parse_idea_set` — which
  requires ≥5 concepts. A single transferred
  concept therefore cannot ride the existing path
  unchanged, and fabricating four filler concepts
  is forbidden. The backward-compatible support
  below is the smallest change that admits one
  approved concept.
- Forge spec UI: `ForgeSpecPanel` takes a source
  artifact record + a `concepts: [{name}]` list
  and a concept-index selector; spec generation
  stays a separate explicit submit. Projects
  renders it under idea artifacts; the viewed-
  artifact dispatch already has an `experiment`
  branch (ColliderExperimentView) and a generic
  fallback for unrecognized bodies.
- Restore: pre-validation functions
  (`validate_original_*_bindings`) run before any
  destination write; per-row remapping
  (`remap_artifact_row` + `remap_*_references`
  helpers) rewrites pinned ids through the restore
  identity map and recomputes the row hash; the
  fix-up pass rewrites pinned hashes afterwards.

## Design

### New module and op

New module `crates/core/src/transfer.rs`, direct
model-free op `collider-transfer` on the
`ai_command` providers arm (the `collider-*`
prefix already routes to `collider_command`).
Required additions around it: the ai allowlist
entry, the API `command_path` entry (the Slice 1
lesson: a missing entry silently blocks the
browser), and contract DTOs + client method.
No provider traffic, no persisted-config load,
no new provider client or orchestration.

### Transfer request and backend binding

Request (deny unknown fields): `workspaceId`,
`projectId`, `experimentArtifactId`,
`experimentVersion`, `combinationIndex`,
`approvedBrief` (1–4000 characters, trimmed
non-empty). The backend, in order:

1. Requires the project writable in the scoped
   workspace (existing check).
2. Reads the experiment artifact at the EXACT
   requested version. Refuses when it is missing,
   belongs to another project, is not type
   `experiment`, or fails
   `validate_experiment_doc` (malformed or
   tampered shape — a generic revise that breaks
   the collider format is caught here).
3. Recomputes the body hash and records it as
   the binding. The transfer pins
   (experiment id, exact version, content hash
   as read) — never "latest".
4. Selects `combinations[combinationIndex]`
   from that pinned version; out-of-range is
   refused.
5. Idempotency: under a process-wide transfer
   mutex, scans the project's `concept` artifacts
   for an existing `anansi-forge-concept` body
   with the same experiment id, version,
   combination index AND approved brief, and
   returns it (`alreadyTransferred: true`)
   instead of creating. The mutex collapses
   concurrent identical submissions in-process;
   the persisted scan collapses retries after
   restart. A different approved brief is a
   different transfer (different user intent) and
   creates a new concept artifact; it is never
   silently merged.
6. Registers the concept artifact (below) and
   returns its record + body. Opening,
   selecting or editing in the UI calls no op
   and saves nothing; only this op creates.

### The transferred concept artifact

Type `concept`, format `anansi-forge-concept`
v1, strict validation (deny unknown fields):

- `sourceExperimentArtifactId`,
  `sourceExperimentVersion`,
  `sourceExperimentContentHash` (the binding).
- `selectedCombinationIndex` and
  `selectedCombination: {name ≤120, description
  ≤1200}` verbatim from the pinned version.
- `approvedBrief` (1–4000): user-authored text,
  recorded exactly as submitted. The UI prefills
  the editor with "{name}: {description}" plus
  the productization concept brief as a
  convenience, but whatever is submitted is the
  user's brief — the record never claims the
  brief is model text, and the backend composes
  no brief itself.
- `assumptions`: feasibility hypotheses verbatim
  (model-derived, labeled as such).
- `unknowns`: unknown costs + dependencies
  verbatim (model-derived, labeled as such).
- `productizationBrief`: concept brief verbatim.
- `provider`, `model`: experiment provenance.
- Title: "Concept: {combination name}".
  `sourceReferences`: [experiment artifact id].
  Generator: `{kind: "transfer", provider,
  model}` — the transfer is user-approved and
  model-free; the model-derived parts are the
  verbatim fields above, labeled in the view.
  Validation status: "validated" (structurally,
  not factually — same convention as the
  collider save).

Nothing in this record is fabricated: fields
the experiment does not have are absent, and
unknowns stay explicit. No model is asked to
manufacture missing fields.

### Forge spec-start: single-concept support

In the spec-start source loading, after reading
the pinned source artifact:

- Type `concept` with format
  `anansi-forge-concept`: validate strictly,
  require `concept_index == 0` (a transferred
  concept has exactly one concept), and build
  the concept JSON from the transfer record
  only: name, description, approved brief,
  assumptions, unknowns, productization brief,
  source experiment id/version, provider/model.
  No IdeaConcept fields are invented.
- Type `idea`: the existing IdeaSet path,
  unchanged (including the ≥5 validation).
- Anything else: the existing refusal, updated
  to name idea or transferred concept.

Spec save already pins the source artifact id
and version, so the chain spec → concept →
experiment survives intact. Spec approval,
build approval, paid-inference authorization and
execution permission are untouched: transfer
approval creates no specification approval,
authorizes no paid inference and permits no
code execution. The transfer stays in the
current project; no project is created by this
slice (additional projects remain an explicit
action through the existing universal project
operations, not built here).

### UI

- Experiment view gains a transfer section
  (new `packages/ui/src/Transfer.tsx`):
  combination radio list, preview showing the
  source experiment identity/version, the
  selected result verbatim, the editable brief,
  assumptions and unknowns lists, and the
  honesty copy (approval sends the concept to
  Forge; it does not approve a specification,
  authorize paid inference or run code).
  The explicit button reads "Approve and send
  to Product Forge". Rendering the panel,
  selecting or editing calls no client method.
- On success the panel shows the created
  concept and an "Open in Product Forge" action
  that opens the concept artifact in the
  Projects view.
- Viewed-artifact dispatch gains a `concept`
  branch: strict `parseTransferConceptBody`
  (unrecognized concept bodies fall through to
  the generic view), a TransferConceptView
  pinning the experiment id/version/hash,
  labeling model-derived vs user-authored
  material, and the existing ForgeSpecPanel
  with `concepts=[{name}]` — so spec generation
  remains its own explicit submit with the
  §29 answers.
- Client: `transferProjectCollider` +
  validators; the transfer panel surfaces a
  failed transfer as a visible error (Slice 1
  lesson: no silent failures).

### Restore

Mirror the review pattern in `restore.rs`:

- `validate_original_transfer_bindings` runs
  in pre-validation, before destination
  publication: for each `concept` row with
  format `anansi-forge-concept`, its body hash
  must recompute to the row hash; the pinned
  experiment row must exist in the backup in
  the same workspace and project, be type
  `experiment` with the collider format, and
  recompute to both its row hash and the pinned
  source hash; the selected index must be in
  range of that experiment's combinations; the
  brief must satisfy its bounds. Any failure
  refuses the restore and publishes nothing.
- `remap_transfer_references` (in transfer.rs)
  rewrites `sourceExperimentArtifactId` through
  the restore identity map, failing closed when
  the experiment is not in the package; the
  caller recomputes the row hash. The
  experiment's own content hash is unchanged by
  restore (its body carries no self-id), so the
  pinned source hash stays valid after remap.
- A transfer whose experiment is absent from
  the backup refuses the whole restore
  fail-closed, like a review with a missing
  package — documented, not worked around.

## Acceptance criteria

1. Opening an exact saved experiment version,
   selecting a combination and editing the
   brief create no artifact and contact no
   provider; only "Approve and send to Product
   Forge" creates the concept.
2. The concept pins the exact experiment
   version + recomputed hash, the selected
   combination verbatim, and the approved brief
   exactly as submitted; model-derived and
   user-authored material are labeled apart.
3. Forged/stale versions (nonexistent version,
   tampered experiment body), cross-project
   and cross-workspace sources, out-of-range
   selections and malformed experiments are
   refused with no artifact created.
4. Identical resubmission — including
   concurrent requests and retries after
   restart — returns the same concept artifact;
   the project gains no duplicate.
5. A newer experiment version neither changes
   an existing transfer nor inherits its
   approval; transferring it needs its own
   approval and yields its own concept.
6. The transferred concept opens the existing
   Forge spec workflow with one concept; spec
   generation stays a separate explicit action
   on the M5 substrate, and no specification
   approval, paid authorization or execution
   permission is created by the transfer.
7. Restart/reopen shows the concept and its
   provenance unchanged.
8. Valid backup/fresh restore remaps the
   experiment reference to the restored
   experiment, preserves pins and hashes, and
   reopens the concept; an invalid reference
   refuses before any destination publication
   and leaves no destination workspace.
9. Collider Slice 1 and Forge revision,
   deliverable and source-review regressions
   keep passing; no timeout, warning or
   assertion is loosened to obtain passes.

## Deferred (explicitly not in this slice)

- Transfer to a different/new project (requires
  an explicit project action through existing
  operations; not built here).
- Selecting the productization as a standalone
  transferable result (selection is per
  combination; productization context travels
  inside the concept record).
- Model-assisted brief composition or gap
  filling; multi-concept transfer bundles.
- Any execution, preview, deployment or paid
  inference.
