# M6 Slice 2 addendum — project-scoped idea generation, as built

Date: 2026-10-21 · Branch `muse/m6-slice2-idea-generation`
over the owner-supplied baseline `codex/local-testing-r11`
at `f6cecd2` (R11 + the Windows desktop artifact-command
capability fix + preview launcher — preserved verbatim as
this branch's parent history). Design and acceptance criteria:
`M6-slice2-design.md`, committed before code.

## What shipped

- **Core** (`crates/core/src/ideas.rs`): four ops on the
  existing `ai_command` providers arm — identical CallerScope
  resolution and lazy persisted-config install as every M5 op.
  A run orchestrates the existing provider stream ops
  in-process; routing, privacy gates, Zero-Cost Lock, permits
  and disclosure are M5's, unchanged. No second inference path.
- **Explicit context only** (§21's rule): the request names
  memory ids; core reads exactly those, in-workspace, and each
  travels as a provider segment with its own privacy class and
  source id, so the PrivacyGate applies per segment. Selecting
  context is the run's explicit cloud authorization. Wire
  capture in the integration tests proves the selected memory
  was transmitted and an unselected one was not.
- **Untrusted output validation** (`parse_idea_set`, pure):
  §22 count 5–10 (fewer fails; more truncates to 10 with a
  recorded note); all twelve §21 fields with strict bounds;
  code fences tolerated; unknown fields dropped, never stored;
  prose refusals and malformed JSON are terminal `invalid`
  outcomes with the reason shown. Concepts are labelled
  model-generated hypotheses in the prompt, the UI, and the
  component tests; no market statistics are invented and no
  score is computed — presentation order is the model's order,
  stated as such.
- **Honest run states**: running / cancelling (until the
  stream's terminal event, the r9 rule) / complete / invalid /
  failed / cancelled. A provider failure at stream start (e.g.
  HTTP 500) becomes a terminal `failed` run rather than a
  request error, so the client renders one failure surface;
  request refusals (gates, archived project, no configured
  model) stay start errors. The default model resolves from
  the trusted backend configuration
  (`configured_chat_models`), never from caller policy.
- **Save**: registers the set as ONE immutable `idea`
  artifact — body is the canonical set JSON; generator records
  the actual dispatch's provider + model; validationStatus
  `validated` (structural validation passed, explicitly not
  factual verification); source references are the approved
  context memories. **Idempotent**: a second save returns the
  same artifact (`alreadySaved`), proven in tests.
- **Surfaces**: generated contracts (8 DTOs incl. IdeaConcept/
  IdeaSet), typed client methods, the four `/v1/ai/ideas-*`
  paths in the API preflight allowlist (with a boundary test —
  the Slice 1 lesson applied up front), verbatim client failure
  mapping for ideas refusals, and the Projects screen's Idea
  generation section: §22 brief form + preference toggles,
  context picker (sealed memories cannot be selected), poll/
  cancel with unmount cleanup, the §19 disclosure panel,
  first-3 + expansion presentation, per-concept Explore
  (works) and Compare / Send to Collider / Start Building
  (disabled, with their milestone reasons), and Save → the
  artifact appears in the project's list; opening a saved idea
  artifact renders its concepts.

## Evidence (Linux, this branch)

- Validation units 4/4 · ideas integration 5/5 (happy path +
  provenance + wire context discipline + idempotent save;
  cancellation; failure/malformed/too-few; isolation and
  project validation; restart persistence of the saved set).
- Full anansi-core: **506 passed, only the 4 pre-existing
  live-Ollama failures** (baseline-identical). API 23/23 (incl.
  the new preflight test). Providers 120/120. UI/contracts
  41/41. TypeScript, contracts drift, fmt and docs validation
  clean.
- Browser e2e: `ideas.spec.ts` (the owner's exact workflow,
  synthetic streaming provider) plus artifacts, app_checkpoint
  and providers specs — **4 passed**.

## Not in this slice (owner boundaries, unchanged)

The scoring rubric (model order is presented as model order),
the Opportunity Engine, pre-project idea storage, per-concept
Save, Compare/Collider/Build engines, idea editing. §22's
optional "use approved ANANSI memory" is realized as the
explicit per-run context selection. Live-provider verification
remains unauthorized and unclaimed; synthetic verification
establishes application behavior only. M5's pending checks —
Windows native/manual acceptance, live-provider verification,
G1 external audit, remote CI — remain open; Antigravity's
Windows findings on the R11 line are consumed at review
boundaries, not from this branch.
