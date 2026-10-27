# M8 Slice 2 addendum: explicitly approved LEXI-COLLIDER → Product Forge transfer

Branch `muse/m8-slice2-forge-transfer`, from the
verified Slice 1 delivery @ `8718df8a` (ZIP
`deb3103d…`, bundle `67211ec0…`, `08465172a`
ancestor — verified in a scratch repo before
import: bundle verifies, scratch fetch lands
exactly on `8718df8a`).

Design:
[M8 Slice 2 design](M8-slice2-forge-transfer-design.md).
This slice delivers one workflow only: open an
exact saved collider experiment version, select
one proposed combination, preview the source
identity/version, selected result, proposed
brief, assumptions and unknowns, edit the brief,
and on explicit "Approve and send to Product
Forge" create a durable Forge-compatible concept
in the current project, then open the existing
Forge specification workflow with that concept.

## What was built

- New module `crates/core/src/transfer.rs`:
  direct model-free op `collider-transfer` on the
  `ai_command` providers arm. It resolves the
  experiment at the exact requested version,
  validates it with the Slice 1 validator,
  recomputes and pins its content hash, selects
  the combination from that pinned version, and
  registers a `concept` artifact (the existing
  registry type, previously unused) with body
  format `anansi-forge-concept` v1: experiment
  id/version/hash, selected combination index
  and verbatim name/description, approved brief
  exactly as submitted, verbatim assumptions
  (feasibility hypotheses) and unknowns (unknown
  costs + dependencies), verbatim
  productization brief, and provider/model
  provenance. Source references use the canonical
  `artifact://{id}@v{version}` form so restore
  remaps them.
- No provider traffic, no brief composition by
  the backend, no fabricated fields: the record
  labels model-derived material and the
  user-authored brief apart, and fields the
  experiment does not have are absent.
- Idempotency is structural: the same
  (experiment, version, combination, approved
  brief) returns the same concept artifact,
  whether the second submission is concurrent
  (process-wide transfer mutex) or a retry after
  restart (persisted scan of the project's
  concept artifacts). A different brief is a
  different transfer, never a silent merge.
- `forge-spec-start` admits transferred concepts
  at index 0 only, building the spec concept JSON
  from the transfer record alone. The idea-set
  path — including its ≥5 concept rule — is
  unchanged, and no filler concepts are
  fabricated. Spec approval, build approval, paid
  authorization and execution permission are
  untouched: transfer creates no specification
  approval and authorizes nothing.
- Restore mirrors the source-review pattern:
  `validate_original_transfer_bindings` runs in
  pre-validation (body hash, pinned experiment
  existence in the same workspace/project with
  matching row and pinned hashes, combination
  in range, brief bounds) and refuses before any
  destination publication; `remap_transfer_
  references` rewrites the experiment id through
  the restore identity map, failing closed when
  the experiment is not in the package. The
  experiment's own hash is unchanged by restore
  (its body carries no self-id), so the pinned
  source hash stays valid after remap.
- UI: `packages/ui/src/Transfer.tsx` —
  `ColliderTransferPanel` under the experiment
  view (combination radio list, preview, editable
  brief prefilled with name + description +
  concept brief, assumptions/unknowns, honesty
  copy; rendering, selecting or editing calls no
  client method and saves nothing) and
  `TransferConceptView` pinning the experiment
  id/version/hash with labeled origins. Projects
  gains a `concept` dispatch branch rendering the
  view plus the existing ForgeSpecPanel with a
  single concept, so spec generation remains its
  own explicit submit. The API route allowlist
  carries the new path (Slice 1 lesson).
- Contracts: `TransferRequest`,
  `TransferResponse`, `TransferConceptDoc`;
  client method `transferProjectCollider`.

## Verification (Linux, sequential on an idle machine)

- Backend transfer suite 8/8 (see
  `crates/core/tests/transfer.rs`): exact pins
  and provenance; forged/stale/cross-project
  refusals; selection and bounds refusals;
  duplicate + concurrent + restart-retry
  idempotency; newer-version non-inheritance;
  spec workflow entry at index 0 with no spec
  approval created; restart and fresh restore
  preservation; tampered-binding restore refusal
  without destination publication.
- Node `test:ui` 76/76 (73 Slice 1 + 3 transfer:
  strict concept parser, view pins/labels, panel
  renders selection + editable brief + honesty
  copy and calls nothing on render).
- tsc clean; contracts `--check` clean; strict
  Clippy (`-D warnings`) and `fmt --check` clean;
  API + web builds pass.
- Full core 276/280 (only the 4 known
  environmental live-Ollama `semantic_workflow`
  failures; 272 at the Slice 1 era + 8 transfer
  = 280); providers 120/0 + API 24/0 = 144/0;
  docs validation PASS (128 Markdown files).
- Browser canonical battery 14/14 across
  app_checkpoint, artifacts, backups, collider,
  evaluations, forge, forge-revision,
  forge-deliverable, forge-review, ideas,
  opportunities, providers, vault and the new
  transfer spec — including the Forge
  revision/deliverable/source-review regressions
  and the Slice 1 collider spec.

## Failures found during verification

- The first transfer test run failed 4/8 on
  test-side errors only: the artifacts `revise`
  DTO carries no `privacy` field (removed from
  the revise calls) and the spec fixture needed
  3 features (the spec validator requires 3–8).
  No implementation code changed for these, and
  no assertion was weakened.
- The first canonical battery run failed 2/14:
  (a) the transfer spec's brief assertion matched
  the generic editor's raw JSON copy as well as
  the rendered view (strict-mode violation) —
  root-caused and fixed by scoping the assertions
  to `<p>` elements, the same pattern as the
  Slice 1 collider spec; (b) the pre-existing
  forge-review spec failed at the findings
  "Prepare revision instructions" step (the
  instructions textarea never appeared within
  30s) after 34.4s. That failure did not recur:
  the spec passed alone in 10.6s and passed in
  the rerun battery (14/14). No code touched by
  this slice renders that flow (the review
  branch, findings selection and revision
  composition are pre-existing and unmodified),
  and no root cause was established — it is
  recorded as an intermittent battery failure in
  the same class as the Slice 1 deliverable
  approval-screen delay, not as a proven defect
  or a fix. Only the complete rerun battery is
  cited as evidence.

## Limitations and open items

- The transfer stays in the current project. A
  transfer to another or a new project is not
  built; it remains an explicit action through
  the existing universal project operations.
- Selection is per combination; the
  productization travels inside the concept
  record, not as a standalone transferable
  result. No model-assisted brief composition.
- A transfer whose experiment is absent from a
  backup refuses the whole restore fail-closed,
  like a review with a missing package — by
  design, not worked around.
- Windows verification of Slices 1–2, M7 native
  desktop acceptance, preview promotion, the §32
  sandbox and the M5 production gates remain
  open. This slice does not declare M8 or ANANSI
  complete. Synthetic only: no generated-code
  execution, credentials, paid inference,
  downloads, pushes or remote CI.

## Owner test guide

1. Open a project, open a saved collider
   experiment (or collide and save one).
2. In "Send a combination to Product Forge",
   select a combination and edit the brief.
   Confirm no concept artifact appears and no
   provider is contacted before approval.
3. Press "Approve and send to Product Forge":
   one concept artifact appears. Submit the same
   transfer again (or approve the same brief
   twice): the same artifact returns, no
   duplicate.
4. Press "Open in Product Forge": the concept
   view shows the pinned experiment id/version,
   the verbatim combination, the brief exactly
   as edited, and assumptions/unknowns labeled
   model-derived. The Forge panel offers exactly
   one concept.
5. Fill purpose, users and must-have, then
   "Generate product specification": a spec
   draft appears and saves, pinning the concept.
   Note that saving the spec does not approve
   it — approval is still a separate step.
6. Restart the API and reopen the concept: pins
   and brief unchanged. Back up and restore into
   a fresh workspace: the concept reopens with
   its experiment reference remapped to the
   restored experiment.
