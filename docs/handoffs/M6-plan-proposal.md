# M6 plan proposal — Innovation Studio Foundation (FOR OWNER APPROVAL)

> **Status update (2026-10-21, appended — the proposal text
> below is unchanged historical record):** the owner approved
> this plan slice by slice and all four slices were
> implemented, delivered as checksummed bundle-in-ZIP
> checkpoints, and Windows-accepted through the R17 preview
> checkpoint (`codex/local-testing-r17` @ `e915f63`): Slice 1
> projects + artifact registry, Slice 2 idea generation,
> Slice 3 rubric evaluation, Slice 4 Opportunity Discovery
> (supplied sources only — plan decision 2 settled by the
> owner), plus the R13/R15/R16/R17 repairs. Records:
> `M6-slice1-addendum.md` … `M6-slice4-addendum.md`,
> `M6-r17-repair-addendum.md`, `docs/BUILD_STATUS.md`.
> Still open/deferred, exactly as the matrix records:
> pre-project idea storage (plan decision 3), the broader
> Opportunity Engine, Collider (M8), and the standing M5
> production gates.

Status: **proposal only. No M6 product implementation has
started; none will until the owner approves this plan (or an
amended version).** Prepared 2026-10-21 on
`integration/m5-into-m2-m3`.

Authority: MASTER_SPEC Milestone 6 + §§9, 20–22, 66–67, 73,
79–80 (per `plans/PHASE-006-innovation-studio.md`), §62 (worked
example), §85 steps 1–3 (the M6 head of the product-creation
Definition of Done; steps 4+ belong to M7/M8/M9/M10 and are out
of scope here).

## Scope inventory (from the plan file's required scope)

1. **Opportunity Engine** — surfaces opportunities (problems,
   gaps, trends in the user's own material) as structured
   records the user can act on.
2. **Idea generator** — produces **5–10 ideas** per run,
   grounded in an opportunity or a free prompt.
3. **Idea scoring** — every idea carries an explicit,
   inspectable score breakdown (no opaque single number).
4. **Universal product projects** — an idea the user keeps
   becomes a product project: a durable workspace entity that
   later milestones (Forge M7, LEXI M8, SwarmTank M9) attach
   to.
5. **Versioned artifact registry** — artifacts (idea sets,
   scores, specs-to-come) are stored as immutable versions
   under a project, with provenance (what produced them,
   when, from which inputs — reusing the disclosure habit M5
   established).

Cross-cutting (§82 + AGENTS.md): local-first; no transmission
without the provider gates; contracts generated, never
hand-duplicated; tests with every behavior; no requirement
claimed complete without a verified workflow.

## Fit with what exists

- **Storage/domain**: projects + registry follow the existing
  workspace domain patterns (SQLite via core storage,
  contracts in `crates/contracts`, ops on the established
  command surfaces, React screens in `packages/ui`).
- **Generation**: idea generation dispatches through the **M5
  provider substrate** — same routing, Zero-Cost Lock,
  permits, and §19 disclosure ("View what ANANSI sent" for an
  idea run). M6 invents no second inference path.
- **Determinism first**: scoring is a deterministic rubric
  computed in core from recorded inputs (plus optional
  model-assisted rationale text) — scores must be reproducible
  and explainable without a model call.

## Proposed slices

### Slice 1 (first, independently testable): Projects + versioned artifact registry — NO AI
- Core: `projects` domain (create/list/open/rename/archive) in
  a workspace; artifact registry (register an artifact version
  with kind, content reference, provenance; list versions;
  fetch a version; versions immutable — a "change" is a new
  version).
- Contracts + client methods; a **Product Projects** screen:
  create a project, add a manual artifact (e.g. a note/spec
  draft typed by the user), see its version history, add a
  second version, archive the project.
- Acceptance: core integration tests (CRUD, version
  immutability, cross-workspace isolation, restart
  persistence); API tests; UI component tests; Playwright
  stage (create → artifact v1 → v2 → history shows both →
  restart → still there). Zero provider calls — fully
  verifiable offline, which is why it goes first.
- Definition of done: the §85 step-3 substrate exists ("choose
  a product concept" has somewhere durable to live) with
  recorded evidence; nothing about generation is claimed.

### Slice 2: Idea generation through the M5 substrate
- "Generate ideas" on a project or standalone: prompt →
  providers-dispatch (existing gates/disclosure) → parse 5–10
  structured ideas → store the run as a registry artifact
  (kind `idea-set`, provenance = disclosure record id).
- Synthetic-provider verification only until live verification
  is separately authorized (the M5 posture carries over).
- Acceptance: with a synthetic provider, a run yields a stored
  idea-set with 5–10 ideas and a disclosure the user can open;
  lock/permit refusals behave exactly as in M5.

### Slice 3: Scoring rubric v1
- A recorded rubric (dimensions + weights) versioned as a
  registry artifact; deterministic scoring in core; per-idea
  breakdown UI; re-score on rubric change creates new score
  artifacts, never edits history.
- Acceptance: same inputs → same scores (property-style
  test); breakdown sums correctly; rubric change re-scores
  into a new version.

### Slice 4: Opportunity Engine (minimal, honest)
- Opportunities derived from the user's workspace material
  (e.g. recurring themes/gaps in memories via existing search)
  presented as records that can seed Slice-2 generation.
- This slice's derivation method is the least specified part
  of §§20–22 — it needs the owner's steer (below) before
  design; scheduling it last is deliberate.

## Explicitly deferred within M6 (with rationale)

- LEXI-COLLIDER combination (§85 step 4) — that is **M8** by
  the roadmap; M6 projects merely must not preclude it
  (artifacts are project-scoped and linkable).
- Product specifications / Forge hand-off — **M7**; M6 stops
  at projects + registry + ideas + scores.
- Any live-model idea quality claims — synthetic verification
  only, as with M5, until separately authorized.

## Decisions needed from the owner

1. **Scoring rubric**: proposed default dimensions —
   problem severity, audience reach, build effort (inverse),
   differentiation, fit with the user's existing projects —
   each 0–5 with recorded weights. Approve, amend, or replace.
2. **Opportunity derivation**: acceptable sources for Slice 4
   (workspace memories only? projects history? explicit user
   seeds only for v1?).
3. **Where ideas live before a project exists**: standalone
   idea-sets in the registry (workspace-scoped, project
   optional) — proposed yes.
4. **Generation default**: local-first (Private/Auto-Free
   routing default for idea runs) — proposed yes, consistent
   with §16's posture.

## Verification posture

Each slice lands with the same evidence discipline as M5:
intermediate commits, synthetic-only inference, merged-tree
regressions, a browser workflow per slice, Windows
verification by the owner, and the pending external gates
(audit, remote CI, live providers) kept visible and unclaimed.
