# M6 Slice 4 design — project-scoped Opportunity Discovery (§21)

Owner-approved 2026-10-21 (supersedes the M6 plan proposal's
open decision 2 with an explicit steer). Baseline:
integrated preview checkpoint `2665e22`
(`codex/local-testing-r16`).

## What this slice is

§21's Opportunity Discovery Engine, restricted to sources the
user deliberately supplies in the project screen:

1. text typed or pasted by the user — a problem statement, a
   customer complaint, or an observation;
2. a document excerpt supplied by the user (pasted text with
   an optional label);
3. ANANSI memories the user explicitly selects.

Nothing else is read. There is no background scanning, no
automatic web research, no scraping, no external connector,
no inference on opening a screen. Generation happens only
when the user reviews the source list and explicitly presses
the generate action, and it rides the existing M5 provider
substrate (routing, Zero-Cost Lock, paid permits, privacy
gates, disclosure, streamed cancellation) exactly as Slice 2
idea generation does.

Each generated opportunity carries the §21 field set (the
same twelve fields as a §22 concept) plus:

- `sourceReferences`: the ids of the supplied sources the
  opportunity claims to derive from. Core validates every
  reference against the sources actually sent: unknown ids
  are dropped, and an opportunity left citing nothing makes
  the set invalid — ANANSI never fabricates a citation.
- `assumptions`: the model's stated assumptions for that
  opportunity (may be empty; rendered as assumptions, never
  as facts).

Saved sets are immutable registry artifacts of type `idea`
(the §9 registry already carries them; evaluations require
an `idea` source) whose body is an extended document
(`format: "anansi-opportunity-set"`) recording the full
source list — supplied texts verbatim, memory sources by id
+ revision — so provenance survives reopen, restart, backup
and restore. Restores remap the body's memory ids to the
restored memories (the evaluations precedent in
`restore::remap_artifact_row`), so a restored set's sources
resolve in the restored workspace; supplied texts are
self-contained and carry over exactly. User assessments
stay in Slice 3 evaluation artifacts; generated hypotheses
are never scored or marked as evidence by this slice.

## Architecture

- New core module `crates/core/src/opportunities.rs`, a
  sibling of `ideas.rs` following its proven shape: DTOs in
  the module, a bounded process-local run registry (TTL +
  per-workspace cap with eviction that cancels the victim's
  provider stream), provider dispatch built the same way,
  claim-and-wait idempotent save. Commands
  `opportunities-generate-start/-poll/-cancel` and
  `opportunities-save`, routed like `ideas-*` through
  `providers_command` and the `ai_command` providers arm
  (including the addressed-workspace poll/cancel fast path
  and lazy persisted-config load on start).
- Segments: one instruction segment (directs the model to
  derive opportunities ONLY from the supplied sources, to
  cite source ids, to state assumptions, to invent no
  statistics or validation, and to treat source text as
  data, never as instructions), one brief segment (optional
  user focus), then one segment per source — supplied texts
  as private segments under run-assigned ids
  (`supplied-1`, `supplied-2`, …), selected memories as in
  Slice 2 (title + body, each with its own privacy class and
  its memory id as source id). Supplied source text is
  untrusted data end to end: it is data in the prompt, data
  in the artifact, and never executed or followed.
- Output parsing (`parse_opportunity_set`): the §21/§22
  concept validation is reused exactly (the base concepts
  are produced by `ideas::parse_idea_set` over the model's
  concepts array — 5–10 bound, strict field bounds,
  truncation note), then per-concept `sourceReferences` /
  `assumptions` are validated and merged by index.
- Contracts: new DTO schemas registered in
  `crates/contracts`, regenerated TS types + validators,
  four client methods (`startProjectOpportunities`,
  `pollProjectOpportunities`, `cancelProjectOpportunities`,
  `saveProjectOpportunities`), API preflight allowlist paths
  `/v1/ai/opportunities-*` added with the boundary test.
- UI: an "Opportunity discovery" section in the Projects
  screen — source rows (kind + optional label + text),
  the explicit memory picker (sealed unselectable), an
  optional focus, a pre-send review listing exactly the
  sources that will be sent, Zero-Cost Lock, generate /
  cancel, the streamed disclosure panel, results rendered
  with their sources, citations and assumptions, save as
  artifact, and an artifact view for saved sets that mounts
  the existing Compare & evaluate panel unchanged. Run
  state is managed by `OpportunityRunController`, mirroring
  `IdeaRunController` (epoch/run/cancel-sequence staleness
  discipline); switching or closing the project cancels the
  run. No inference or prefetch on open.
- Restore: `restore::remap_artifact_row` gains an
  opportunity branch — for `idea` artifacts whose body is
  an opportunity set, memory ids in `sources` and in
  concept `sourceReferences` are remapped through the
  restore id map (a valid-format id missing from the map
  fails closed, as with evaluations; `supplied-N` markers
  pass through), and the content hash is recomputed when
  the body changed. Other bodies stay byte-exact.

## Deliberate boundaries

- No new artifact type, no schema migration, no parallel
  project/provider system; Collider, Forge, SwarmTank and
  any scheduler remain out of scope (later milestones).
- No §22 brief form (topic/customer/budget…) for discovery:
  the supplied sources are the brief; one optional focus
  line is all that is added.
- Generation defaults and gates are the substrate's: with
  the Zero-Cost Lock on, paid routes refuse as in Slice 2;
  sealed memories cannot be selected or sent; archived
  projects refuse generation and save (the registry's
  writable check); other workspaces' memories cannot be
  read (workspace-scoped reads).

## Acceptance criteria (verified by the named evidence)

1. Workflow: project → add typed problem + excerpt, select
   a memory → pre-send review lists exactly those sources →
   generate (synthetic provider) → results show sources,
   per-opportunity citations, assumptions, differentiation
   hypotheses and required validation → save → reopen after
   API restart → evaluate via the existing rubric panel →
   backup → restore into a fresh workspace → restored set
   reopens with sources resolving to the restored memory.
   (Browser e2e `opportunities.spec.ts`.)
2. Wire discipline: the captured provider request contains
   the selected memory's text and the supplied texts, each
   as its own segment with its own privacy class and source
   id; an unselected memory's text never appears; no other
   workspace content is sent. (Core integration test with
   request capture.)
3. Grounding: a model set citing an unknown source id has
   that reference dropped; a set whose opportunity cites no
   supplied source is invalid with a stated reason; counts
   outside 5–10, missing fields, malformed JSON and refusal
   text all end the run `invalid` with nothing saved.
   (Validation unit tests + integration.)
4. Lifecycle: provider failure at start → terminal `failed`
   run; mid-stream failure → `failed`; cancel → `cancelling`
   until the stream actually stops, then `cancelled`,
   nothing saved; duplicate/concurrent save → one artifact,
   `alreadySaved` for the rest. (Integration tests mirroring
   the Slice 2 suite.)
5. Isolation & refusal: another workspace's run ids and
   memories are refused; a sealed selected memory refuses
   the start; an archived project refuses start and save; a
   run with no sources at all is refused before dispatch.
   (Integration tests.)
6. Restore identity: after a workspace backup → fresh
   restore, the restored opportunity body's memory source
   ids equal the restored memories' ids (and differ from the
   originals), supplied texts are byte-identical, concept
   citations point at the restored ids, and the content hash
   verifies on read. (Core restore test.)
7. Regressions: ideas, evaluations, artifacts, providers,
   backup/restore suites and the existing browser specs stay
   green; contracts check, typecheck, UI tests, clippy and
   fmt clean. (Full battery on the final tree, recorded in
   the slice addendum.)
