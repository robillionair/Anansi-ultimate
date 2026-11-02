# M10 SOMNIA Slice 3 — Delivery Addendum & Milestone 10 Report (§88)

## What shipped

Slice 3 (branch `muse/m10-somnia-slice3`, from owner tip
`b9f9edfc`) closes Milestone 10 per MASTER_SPEC §50,
§51, §87, with the §88 report below:

1. **SOMNIA dashboard (§50).** A dashboard section in
   the SOMNIA panel, loaded only on an explicit user
   action, aggregating the project's saved evaluations:
   recent failure rate (Refuse + Abort over all saved
   scenarios, one decimal), context efficiency (latest
   calls per scenario vs earliest-run baseline, as a
   ratio that is never presented as an improvement when
   it rises), duplicate evidence-id and retrieved-source
   counts, pending inert proposal count for the latest
   run, evaluation coverage over the seven §45 classes,
   latest run (record timestamp + runId), total
   scenarios executed, per-category failure totals,
   improvements proposed, and regressions between
   consecutive saved runs (failure count strictly
   increasing). Aggregates are pure derivations over
   saved local artifacts (Rust `compute_dashboard`
   mirrored in TS, pinned by fixed-doc tests in both
   suites). No telemetry, no analytics beacons, zero
   cloud calls. Coverage honesty: the
   privacy-boundary class cannot appear in a saved
   evaluation, because sealed inputs are refused before
   evaluation; the dashboard states this instead of
   silently scoring 6/7 as a failure.
2. **Product Forge QA bridge (§51).** A Forge QA
   section listing the project's registered Forge
   concepts and deliverables (types `concept` and
   `deliverable`, loaded on explicit action) and, per
   item, generating seven deterministic QA scenarios
   from the concept title — API failure, incorrect
   input, timeout, unexpected dependency, tool failure,
   recovery, memory/context failure — evaluated
   through the unchanged Slice 1 evaluator via the
   existing evaluate/save ops, saved as an ordinary
   SOMNIA evaluation titled with the Forge concept
   title, and exportable as JSON and Markdown. The
   section displays the §51 disclosure verbatim:
   "SOMNIA augments QA; it does not replace ordinary
   automated software and security testing." Nothing
   here runs real code, calls a model, or mutates the
   Forge concept; no new artifact type, op, or schema
   change.
3. **Proposal review state (§87.7).** Slice 2's
   local-only "Mark as reviewed" toggle gains a
   sibling local-only "Dismiss" toggle. Both are React
   state, never sent, never persisted, never mutating
   any memory or policy; there is still no apply path
   and no mutation op. The dashboard pending count is
   the latest run's proposals minus reviewed ∪
   dismissed.
4. **Dashboard action triggers (§50).** "Run Nightmare
   Test", "Review Improvement Proposals", and "View
   Results / Compare" move focus to the nightmare
   generator, proposals (viewing the latest run), and
   comparison sections respectively, asserted via
   `activeElement` in the spec.

## Slice 2 Windows verdict (recorded, not re-run on Linux)

The Slice 2 delivery was independently reviewed and
certified on Windows NTFS (reviewer's evidence, per
the Slice 3 assignment, review tip `4ca4a146`): tsc
exit 0, contracts `--check` exit 0 (zero drift), docs
validation PASS (226 Markdown files), UI 165/165 in
5.37s, Rust somnia 18/18 in 0.01s, full workspace
589 passed (0 failures, 4 pre-existing offline-Ollama
skips only), Vite production build clean in 2.09s,
Playwright battery under shared lease 14/14 (SOMNIA
Slice 2 2/2, Slice 1 2/2, SwarmTank scenario 3/3,
regression specs 7/7). The review tip is not present
locally and no review ZIP arrived, so Slice 3 builds
on owner tip `b9f9edfc` per the assignment's stated
alternative, and this addendum carries the verdict.
AN-015 therefore advances `verified` → `complete`
for Milestone 10, with the Slice 3 scope itself still
awaiting its own Windows rerun — the matrix row says
so explicitly, and Linux evidence is never presented
as Windows acceptance.

## Linux verification (sequential)

- Rust workspace: **592 passed + 4 known live-Ollama
  environment failures** (589 + 3 new Slice 3 tests;
  the 4 failures are the same `semantic_workflow`
  family as prior slices, `StorageUnavailable` with
  the embedding backend absent on this VM).
- Rust SOMNIA lib suite: **21/21** on the final tree
  (18 + 3 new: dashboard fixture, empty history,
  Forge QA intended decisions).
- Strict Clippy clean (four real findings fixed
  during the run: a collapsible `if let` chain and
  three manual checked divisions in the dashboard
  code), `cargo fmt --check` clean.
- Node `test:ui`: **168/168** (165 + 3 new
  Slice 3 tests: dashboard fixture parity with Rust,
  Forge QA determinism/shape, panel dashboard + Forge
  QA sections with the verbatim disclosure).
- `tsc --noEmit` exit 0, `contracts --check` exit 0
  (zero drift; the `uint` format lines are
  pre-existing validator noise on untouched schemas),
  docs validation PASS at 226 files (225 + this
  addendum), web build clean.
- E2E `somnia-slice3.spec.ts` **2/2**: §87 walk
  (baseline + nightmare saves, dashboard aggregates
  with independently computed duplicate counts and
  latest-run identity from the API, focus triggers,
  reviewed + dismissed with no new artifact,
  comparison, Forge QA run/save/exports, zero
  provider traffic) and backup/restore survival with
  identical dashboard aggregates.
- Regressions green: Slice 1 2/2, Slice 2 2/2, Slice
  3 2/2, evaluations, integrated workflow,
  opportunities, SwarmTank slice 1 / arena / scenario.
- Browser battery: **48/49 specs, 66/67 tests**. The
  single failure is `search.spec.ts`'s real-semantic
  test, which needs live Ollama embeddings absent on
  this VM and previously failed identically on
  pristine baselines.

## §88 milestone report (Milestone 10)

- **Features built.** Slice 1: isolated, manual,
  data-only replay with a deterministic seeded
  reference evaluator and immutable evaluation
  artifacts. Slice 2: seeded nightmare generation,
  inert consolidation proposals, before/after
  comparison, SwarmTank wording repair. Slice 3:
  dashboard aggregates, Forge QA bridge, proposal
  review state, action triggers — closing §50, §51,
  and §87.
- **Files added.** Slice 3: `docs/handoffs/M10-somnia-slice3-design.md`,
  `docs/handoffs/M10-somnia-slice3-addendum.md`
  (this file), `tests/e2e/somnia-slice3.spec.ts`.
- **Files modified.** Slice 3: `crates/core/src/somnia.rs`,
  `packages/ui/src/Somnia.tsx`,
  `packages/ui/tests/somnia.test.tsx`,
  `docs/BUILD_STATUS.md`, `docs/RECOVERY.md`,
  `docs/REQUIREMENTS_MATRIX.md`, `plans/08-somnia.md`.
- **Architecture decisions.** View-time derivation
  over saved artifacts for everything aggregate
  (dashboard, proposals, comparison); existing ops
  for everything evaluated or saved (nightmares,
  Forge QA); no new op, artifact type, or schema
  change in any slice; seed affects only runId and
  display order; wall time never enters a hashed
  doc; sealed input is refused before evaluation,
  never silently dropped.
- **Database migrations.** None in Slice 3 (schema
  v11 unchanged; the M8 lineage migration 10→11
  predates M10).
- **Tests passed.** Rust 592 + 4 environmental
  failures; Rust somnia 21/21; node 168/168; e2e
  Slice 3 2/2; battery 66/67.
- **Tests failed.** One: `search.spec.ts`
  real-semantic test (known pre-existing, live
  Ollama embeddings absent on this VM).
- **Tests not run.** Native Tauri desktop acceptance
  (out of scope for the browser-focused slices);
  live-provider verification (none by design — zero
  provider traffic is the invariant, and the fake
  provider assertions in every SOMNIA spec enforce
  it).
- **Security issues.** None found. Sealed fail-closed
  preserved at preview, evaluate, and save; no
  automatic memory scanning; no production accounts,
  filesystem capabilities, credentials, or network
  activity from replay instructions; user text and
  imported traces remain untrusted data.
- **Live-provider verification.** None — by design.
  Nothing in SOMNIA calls a model or the network
  outside localhost.
- **Known limitations.** The evaluator is a rule
  engine, not proof a real agent improved; the
  generator's variation is template slots over fixed
  word lists; coverage tops out at 6 of 7 from saved
  evaluations because sealed inputs never produce
  one; "restore the previous version" means the
  saved-evaluation history survives backup/restore,
  not single-artifact version rollback; the Forge QA
  summary is saved as a SOMNIA evaluation titled
  with the concept title, not as a new Forge
  artifact version; proposal thresholds are Slice 2
  conventions, not measured optima.
- **Last verified commit.** See the delivery
  MANIFEST (tip recorded there after closure docs).
- **Backup status.** Backup/restore survival
  verified in the Slice 3 spec (both saved
  evaluations present, dashboard aggregates
  identical after restore into a fresh workspace).
- **Current runnable command.** From the repo root:
  `node scripts/with-application-test-lease.mjs
  node_modules/@playwright/test/cli.js test --config
  playwright.config.ts tests/e2e/somnia-slice3.spec.ts`
  (after `node scripts/cargo.mjs build --locked -p
  anansi-api` and the `apps/web` Vite build).
- **Next milestone.** None commissioned. Milestone
  10 is closed by this slice; §48 lesson promotion,
  §49 scheduling, historical ingestion, and
  provider-backed evaluation remain out until
  separately approved and must not be treated as
  implied follow-ups.

## Requirement coverage

- AN-015: `complete` for Milestone 10 (§§42–51,
  §87). Slice 3's own Windows rerun remains the
  reviewer's step and is not claimed here.
- §87 items: 1–10 all walk end to end in the Slice
  3 spec, with 7 as review acknowledgement only
  (reviewed/dismissed in UI state; live memory
  unmutated) and 10 as history survival across
  backup/restore.
