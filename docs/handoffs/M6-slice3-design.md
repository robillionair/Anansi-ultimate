# M6 Slice 3 design — transparent concept evaluation & comparison

Status: design + acceptance criteria, written before code
(2026-10-21), per repo AGENTS.md. Continues from the R13
repair checkpoint `7b39c6b` on a new branch
`muse/m6-slice3-evaluation`.

## Where this sits

The approved M6 plan (M6-plan-proposal.md) defines Slice 3
as **"Scoring rubric v1"**: a recorded rubric (dimensions +
weights) versioned as a registry artifact; deterministic
scoring in core; per-idea breakdown UI; re-score on rubric
change creates new score artifacts, never edits history.
Requirement AN-027 (Innovation/Opportunity Studio and idea
scoring). The plan left the rubric itself as owner decision
#1; the owner's 2026-10-21 instruction settles it:

- Dimensions (initial): **problem clarity, customer
  specificity, MVP feasibility, fit to the user's stated
  budget/technology, validation readiness**.
- **Equal initial weights**, labelled an owner-approved
  *starting* rubric — not an objectively validated model.
- Scores are **user-entered 1–5 assessments with a
  rationale**; the only computation is a deterministic
  weighted summary in core.
- Missing assessments stay **"not assessed"**; never
  silently substituted with zero.
- No AI score is presented as market evidence, probability
  of success, verified demand or factual validation — and
  this slice makes **no model calls at all**.

Identity stays singular (§9): concepts live in the saved
`idea` artifacts of Slice 2; rubrics and evaluations are
registry artifacts in the same project. No parallel system,
no schema migration: the registry (Slice 1) already provides
immutable versions, provenance, integrity and isolation.
`ARTIFACT_TYPES` gains `rubric` and `evaluation` (the
manual-registration list; the generated path Slice 2 uses
does not consult it).

## Design

### Rubric document (artifact type `rubric`, body JSON)

`{ name, dimensions: [{ key, label, description, weight }] }`
— parsed and bounded by a pure core parser (`parse_rubric`):
1–8 dimensions; keys are unique slugs; label ≤ 80 chars;
description ≤ 300; weight finite, 0–1000, at least one > 0.
The standard starting rubric is defined ONCE in core
(`standard_rubric()`, the five owner dimensions, weight 1)
and served read-only via the artifacts op
`evaluations-standard-rubric`, so UI, tests and saves share
one definition. Editing a rubric = register (v1) or revise
(new version) through the existing registry ops — history
is never edited, per the plan.

### Evaluation save (artifacts op `evaluations-save`)

Request: project, source idea artifact id + **exact
version**, rubric artifact id + **exact version**, 1–3
concept indices, and assessments
`[{ conceptIndex, dimension, score: 1–5 | null, rationale }]`.

Core, in order: workspace-scoped reads of both artifacts at
the named versions (the source must be an `idea` artifact
whose body re-parses as an idea set; the rubric must parse);
selection bounds (1–3, unique, in range); assessment
validation (known concept + dimension, score range,
rationale ≤ 600 chars, no duplicates); **deterministic
summary in core** — per concept, Σ(score×weight) /
Σ(weights of assessed dimensions), rounded to 2 dp, `null`
when nothing was assessed (missing is never zero). The
stored body is a self-contained document: format marker,
source + rubric ids and versions, the **rubric snapshot**,
the selected concepts (full §21 fields as of the source
version), the assessments, the computed summary, and a
standing note that these are user assessments, not market
evidence. Registration goes through
`artifact_register_generated` with generator
`{kind: manual}` (the assessments are the user's) and
validation status `validated` — structural validation only,
the codebase's established meaning. Source references link
`artifact://<id>@v<n>` for both inputs. Archived projects
refuse (registry rule); sealed sources cannot be read for
this (registry read rules apply unchanged).

**Idempotency:** the canonical body is compared by content
hash against the project's existing `evaluation` artifacts;
an identical save returns the existing artifact with
`alreadySaved: true` instead of registering a duplicate
(the Slice 2 save lesson, applied via the registry's own
recorded hashes).

### UI (Projects screen)

- Opening an `idea` artifact shows a **Compare & evaluate**
  panel: concept checkboxes (max 3 enforced), a rubric
  picker (the standard rubric, or any project `rubric`
  artifact at its latest version), a rubric editor
  (name/labels/descriptions/weights, add/remove dimension;
  save as artifact or as a new version of the selected
  rubric), an assessment grid (score select + rationale per
  concept × dimension), a live summary preview using the
  same formula (labelled a preview; core recomputes at
  save), and Save.
- Opening an `evaluation` artifact renders the saved
  comparison read-only from its own body: provenance
  (source + rubric ids/versions), the rubric snapshot
  *as used* (with the note that later rubric versions do
  not change it), side-by-side concept fields, scores +
  rationales or "not assessed", and the summary with
  assessed counts.

## Acceptance criteria (each mapped to a test)

1. Rubric parser bounds + the standard rubric's exact five
   dimensions with equal weights (unit).
2. Summary math: weighted mean over assessed dimensions
   only; partially assessed concept excludes missing
   dimensions (never zero); fully unassessed → null;
   assessed counts reported (unit).
3. Save flow: evaluation artifact registered with the
   embedded snapshot, concept snapshots, core-computed
   summary, generator `manual`, links to the exact source
   and rubric versions (integration).
4. Idempotent save: a second identical save returns the
   same artifact, `alreadySaved`, and no duplicate exists.
5. Rubric versioning: revising the rubric afterwards does
   not change the saved evaluation (body hash identical);
   a new evaluation embeds the new version's snapshot.
6. Refusals: >3 concepts, unknown concept index, score
   outside 1–5, unknown dimension, nonexistent versions,
   non-idea source, cross-workspace source/rubric ids,
   archived project (integration).
7. Restart persistence: a fresh Core reopens the evaluation
   and its body verifies (registry integrity) unchanged.
8. Surfaces: generated contracts validate the new DTOs;
   API preflight allowlist includes the new artifacts paths
   (boundary test); the evaluation view renders snapshot +
   "not assessed" + summary (component test); the full
   browser workflow passes (e2e): register idea set →
   select concepts → assess → save → restart → reopen →
   rubric new version → second evaluation.

## Honest boundaries

Scores are one user's structured judgement under a rubric
they can inspect and change. Nothing in this slice
researches, verifies or predicts anything; the summary is
arithmetic over the user's own inputs, displayed with its
assessed/missing state. Opportunity Engine (Slice 4) is
untouched and still gated on the owner's remaining plan
decisions; Compare per-concept buttons from Slice 2 now
point at this panel for saved sets.
