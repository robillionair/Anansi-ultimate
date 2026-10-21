# M6 Slice 3 addendum — transparent concept evaluation & comparison, as built

Date: 2026-10-21 · Branch `muse/m6-slice3-evaluation` over
the R13 repair checkpoint `7b39c6b`. Design + acceptance:
`M6-slice3-design.md` (written before code). This is the M6
plan's Slice 3 ("Scoring rubric v1"); the owner's
2026-10-21 instruction settled plan decision 1 — the rubric
dimensions and the user-entered scoring model below.

## What shipped

- **Rubric as a versioned registry artifact** (type
  `rubric`): `{ name, dimensions: [{key, label, description,
  weight}] }`, parsed and bounded in core (`parse_rubric`:
  1–8 dimensions, unique slug keys, weights 0–1000 with at
  least one positive). The **standard rubric** — problem
  clarity, customer specificity, MVP feasibility, budget &
  technology fit, validation readiness, all weight 1 — is
  defined once in core and served via
  `artifacts/evaluations-standard-rubric`; it is labelled an
  owner-approved starting point, not a validated model.
  Editing = register/revise through the existing registry;
  history is never edited.
- **Deterministic summary, in core** (`summarize`): per
  concept, Σ(score × weight) / Σ(weights of assessed
  dimensions), rounded to 2 dp. Missing assessments are
  excluded and reported as "not assessed" with assessed/total
  counts — never zero-substituted. Property-tested for
  determinism; there are no model calls anywhere in the
  slice.
- **`evaluations-save`** (on `artifacts_command`): reads the
  exact source idea-artifact version and rubric version
  (workspace-scoped, same-project enforced), validates the
  selection (1–3 concepts) and every assessment (known
  concept/dimension, score 1–5, rationale ≤ 600, no
  duplicates), then stores ONE immutable `evaluation`
  artifact: rubric snapshot embedded, concept snapshots as
  of the source version, the assessments, the core-computed
  summary, and the standing note that these are user
  judgements, not market evidence. Generator is
  `{kind: manual}` — the assessments are the user's —
  validation status `validated` (structural, as established).
  Source references cite `artifact://<id>@v<n>` for both
  inputs. **Idempotent** by content hash against the
  project's existing evaluations. Archived projects refuse
  (registry rule); cross-workspace ids resolve to NotFound.
- **UI**: saved idea sets gain a Compare & evaluate panel
  (selection ≤ 3, rubric picker + editor with save-as-artifact
  / save-as-new-version, per-dimension score + rationale,
  live preview using the same formula — labelled a preview,
  core recomputes at save; saving with the standard rubric
  registers it as an artifact first so the evaluation can
  cite an exact version). Saved evaluations render read-only
  from their own document: provenance, the rubric *as used*,
  side-by-side concepts, scores or "not assessed", summary.
  One defect found by the full browser suite and fixed: the
  panel's concept-picker labels duplicated the concept cards'
  accessible names; the picker now uses bare concept names.
- **Surfaces**: generated contracts for 7 DTOs, typed client
  methods, the two `/v1/artifacts/evaluations-*` paths in
  the API preflight allowlist with a boundary test, the
  register form's type list extended with `rubric` /
  `evaluation`.

## Evidence (Linux, this branch)

- Core: rubric/summary units 4/4; evaluations integration
  3/3 (snapshot+summary+provenance; idempotency + rubric-v2
  history stability, incl. the recomputed 4.43 weighted
  case; refusals/bounds/isolation/archived/restart).
- Suites: providers 120/120 · API 24/24 · UI/contracts 47/47
  · clippy `-D warnings` clean · fmt/tsc/contracts/docs
  clean · focused core (ideas+registry+dispatch+evaluations)
  38/38.
- Browser e2e 5/5: checkpoint, artifacts, ideas,
  **evaluations** (the owner's exact workflow, incl. restart
  and the rubric-v2/history check), providers. Individual
  specs also each passed standalone; during the VM's
  post-restart contention window, suite runs intermittently
  flaked one spec at a time on visibility timeouts — the
  final calm-window run passed all five in 17.9s.
- Full anansi-core: 188 passed / 6 failed of 194 — the 4
  known live-Ollama tests + the 2 load-sensitive
  `vault_increment2` tests (identical failure reproduced on
  the pre-repair tip; see the R13 addendum).

## Remaining / not claimed

- Windows verification of R13 + Slice 3 (owner).
- M6 Slice 4 (Opportunity Engine) is NOT started: it awaits
  the owner's remaining plan decisions (derivation sources).
  Plan decision 3 (pre-project idea storage) remains
  superseded by the owner's Slice 2 boundary (project-scoped
  only); decision 4 (generation defaults) remains as built
  in Slice 2 (Zero-Cost Lock default on).
- M5's pending checks are unchanged: Windows native/manual
  acceptance, live-provider verification, G1 audit, remote
  CI. The next integration step per the owner is updating
  the isolated user preview from a reviewed checkpoint.
