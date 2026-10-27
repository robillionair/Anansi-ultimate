# M8 Slice 1 design: text-first LEXI-COLLIDER

Baseline: the independently reviewed checkpoint
`codex/review-m7-source-review` @
`08465172aab8fcc99ceb5af347145d3bd4545b91`
(Windows corrections at `befc66f11` preserved:
coherent package/history/editor selection,
malformed-review guards, inspection-only
assessment labels, restore format validation).
Branch `muse/m8-slice1-collider` from that
exact tip. This slice is authorized while M7
native source-review acceptance, the execution
environment and M5 production gates remain
open; it declares none of them complete or
waived, and it does not claim M8 complete.

## Scope (one complete workflow, in an existing project)

- Two labelled text inputs, Beam A and Beam B,
  typed/pasted text only. Both required.
  Supplied text is private data and untrusted
  content: it is carried as data segments,
  never as instructions, and never overrides
  application policy.
- Advanced controls with defaults that work
  untouched, each explained in the UI without
  suggesting it guarantees novelty or
  feasibility: creativity (low/balanced/high,
  default balanced), technical depth
  (overview/standard/deep, default standard),
  practicality (speculative/balanced/practical,
  default balanced), innovation distance
  (near/moderate/far, default moderate),
  budget (free text, at most 200 chars, a
  user-stated constraint, not a cost promise),
  preferred output type
  (concept/architecture/product/research,
  default concept).
- Explicit Collide Ideas submission only.
  Nothing starts on mount, on typing, or on
  project selection. One provider request
  produces all six sections; the UI shows
  honest states only: generating, then
  validating (a real client-side strict parse
  of the completed output), then the result
  or an error. No six live stages, no
  percentages, no invented progress.
- Six structured, user-facing sections
  (§25, one request): source understanding;
  conceptual primitives; cross-domain
  mappings; several proposed combinations or
  architectures; feasibility hypotheses,
  dependencies, unknown costs, risks and
  testability; productization proposals
  (concept brief, architecture, roadmap, test
  plan, prototype proposal). Analogies,
  assumptions and hypotheses are labelled as
  such in the prompt and in the UI. Nothing
  claims independent research, technical
  validation, novelty, patentability or
  market evidence.
- Inspection, then explicit saving as one
  immutable, versioned experiment through
  the existing artifact registry. The saved
  record preserves exact beam inputs,
  controls, provider/model provenance and
  the disclosure record. Reopen after
  restart; backup/fresh restore preserves
  content and versions.
- Accessible 2D presentation only. No
  Three.js, no decorative computation
  animations.

## Explicitly deferred (not shown as working)

URL fetching, document ingestion,
saved-project/memory selection, 3D
visualization, and approved Forge transfer
("Build This Product"). The panel carries
one plain-text note naming these as later
slices; it offers no buttons for them.

## Reuse (no new provider client, no new orchestration)

- M5 substrate unchanged: `ai_command`
  providers arm, `providers-stream-start/
  -poll/-cancel`, routing, PrivacyGate,
  Zero-Cost Lock, permits, budgets and
  disclosure (DisclosureRecord). The collider
  is a new `collider.rs` module mirroring
  the M6 Slice 2 `ideas.rs` run pattern:
  process-local run registry (TTL 600 s,
  32 runs per workspace, session-scoped keys),
  run start/poll/cancel/save ops, claim/wait
  idempotent save, eviction that cancels
  live streams instead of orphaning them.
- Existing artifact registry: the generic
  §9 type `experiment` already exists in
  ARTIFACT_TYPES, so no type-list change.
  Save uses `artifact_register_generated`
  with generator {kind:model, provider,
  model} and status "validated" (the set was
  structurally validated, not factually
  verified — the UI says so).
- Existing UI lifecycle: a new
  `colliderRun.ts` controller mirroring
  `ideaRun.ts` (epoch + cancel sequence +
  serialized polling + stale-response drop),
  and a new `Collider.tsx` panel/view wired
  into the Projects screen beside the idea
  section. Project switching and unmount
  cancel the in-flight run.

## Contracts

Ops on the `ai_command` arm (allowlisted
in `ai_discovery.rs`, routed in
`provider_dispatch.rs` beside
`ideas_command`):
`collider-start`, `collider-poll`,
`collider-cancel`, `collider-save`.

- `ColliderStartRequest` (deny_unknown_fields,
  contractVersion 1): workspaceId, projectId,
  beamA/beamB (each 1–8000 chars after trim),
  controls (all optional, defaults applied
  server-side), model?, provider?, mode?,
  zeroCostLock (default false),
  permitId?.
- `ColliderPollResponse`: runId, status
  (running | cancelling | complete |
  invalid | failed | cancelled),
  partialChars, result?, disclosure?,
  error?.
- `ColliderSaveResponse`: artifact, body,
  alreadySaved.
- `ColliderResult` (the validated output):
  sourceUnderstanding (1–6000 chars);
  conceptualPrimitives (3–12 strings, each
  1–300); crossDomainMappings (2–12 strings,
  each 1–600); combinations (2–8 objects
  {name 1–120, description 1–1200});
  feasibility {hypotheses 1–8 × 1–600,
  dependencies 0–8 × 1–300, unknownCosts
  0–8 × 1–300, risks 1–8 × 1–300,
  testability 1–1200}; productization
  {conceptBrief 1–2000, architecture 1–3000,
  roadmap 1–3000, testPlan 1–3000,
  prototypeProposal 1–2000}. Total raw
  output capped at 262144 chars, as in
  ideas. Unknown JSON fields are refused.
- `ColliderExperimentDoc` (saved body,
  format `anansi-collider-experiment` v1):
  beamA, beamB, controls (resolved),
  result, provider, model, disclosure.
  Saved through the generated-contracts
  path: DTOs derive JsonSchema, listed in
  `crates/contracts/src/main.rs`; client
  methods `startProjectCollider`,
  `pollProjectCollider`, `cancelProjectCollider`,
  `saveProjectCollider`.

## Privacy, cost, cancellation, recovery

- Prompt: one instruction segment (standard
  privacy) stating beams are data, embedded
  instructions inside beams must be ignored,
  and every section is hypothesis labelled
  with no invented statistics, novelty or
  market claims; then two beam segments
  (privacy "private", labelled "Beam A
  (data)" / "Beam B (data)").
  explicitCloudAuthorization is true only
  because the user explicitly pressed
  Collide Ideas for the chosen provider.
  Sealed content is never involved: beams
  are fresh typed text, not memories.
- Gates before any provider traffic:
  project must exist and be writable
  (archived refused); provider must be
  configured; Zero-Cost Lock / permit /
  budget refusals come from the M5 layer
  with zero provider hits. A provider
  connect failure before streaming becomes
  a terminal failed run (ideas pattern),
  not a request refusal, so the client
  renders one failure surface.
- Cancellation: cancel marks the run
  cancelling, cancels the provider stream,
  and polls until the stream settles; the
  UI shows Cancelled only after settlement
  (never a Cancelled event while backend
  work remains active).
- Stale responses: server keys runs by
  workspace + run id and checks the session;
  the client controller drops responses
  whose epoch/run id/cancel sequence no
  longer match. A run from another project
  or workspace can never overwrite state.
- Repeated saves: claim/wait idempotency —
  concurrent saves resolve to the same
  artifact; a failed registration releases
  the claim for retry. Save is refused
  unless the run is complete.
- Malformed output: poll completion runs
  the pure `parse_collider_output`
  validator; failure sets status invalid
  with a human reason and no result, and
  nothing is saved. The client re-parses
  before rendering (the honest validating
  state).
- Restart: in-flight runs do not survive
  restart (documented, as with ideas/Forge
  runs); saved experiments are ordinary
  registry artifacts and reopen by exact
  version. Restore: experiment rows whose
  body declares
  `anansi-collider-experiment` must fully
  validate before any destination write
  (mirroring the review format guard);
  other generic `experiment` rows restore
  untouched. No id remapping is needed in
  slice 1 (no artifact references in the
  body); the viewer parser rejects
  unrenderable saved bodies instead of
  crashing (the Windows review lesson).

## Acceptance criteria

1. Both beams required; bounds enforced
   server-side; empty/oversize refused
   before any provider traffic.
2. No provider call before explicit
   Collide Ideas (unit + node tests with a
   mock client; backend start-count test).
3. Privacy/cost refusal (Zero-Cost Lock
   against a paid model, unconfigured
   provider) yields zero provider hits.
4. One request returns all six sections;
   malformed/truncated/over-bounds output
   becomes invalid, never a partial save.
5. Disclosure (provider, model, endpoint
   locality, routing reason, cost class)
   is shown before and with the result.
6. Cancel settles to cancelled with
   provider accounting cleaned up; a late
   poll cannot resurrect the run.
7. Project switch / workspace isolation /
   stale poll cannot overwrite state.
8. Save is immutable, versioned, idempotent
   under repeats; reopen after restart
   shows the exact saved beams/controls/
   result; fresh restore preserves content
   and versions, and refuses a tampered
   collider-format experiment before
   publication (with a generic-experiment
   restore control).
9. Beam text containing prompt-like
   instructions ("ignore previous
   instructions…") stays a data segment
   and never changes the instruction
   segment or the validation outcome.
10. Existing Forge revision, deliverable
    and source-review browser regressions
    still pass; canonical e2e battery
    stays green.
11. No execution, iframe preview,
    deployment, real credentials, paid
    inference or model downloads anywhere
    in the slice.

## Tests planned

- Backend `crates/core/tests/collider.rs`:
  bounds/required inputs; no-inference-
  before-submit; privacy/cost refusals
  with a counting fake provider; malformed
  output variants; cancel settlement;
  workspace isolation + stale poll;
  save idempotency + restart reopen;
  fresh restore (valid, tampered-format
  refusal, generic-experiment control);
  embedded-instruction beam stays data.
- Contract/UI node tests: run controller
  staleness/cancel, panel bargain (no
  start before submit), result parser
  strictness, experiment view pins.
- Browser `tests/e2e/collider.spec.ts`:
  defaults → collide → validate → save →
  reopen → restart → backup/fresh restore;
  cancel path; invalid-input path.
  Sequential, idle machine, original
  timeouts/assertions.
