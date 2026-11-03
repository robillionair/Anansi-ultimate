# HOME Command Dashboard — Slice 1 Addendum

Branch `muse/dashboard-slice1` from Windows-certified LARP Slice 4
review tip `ac2f05aba7647c48646ef2ae013a3be5a1084f72`
(`codex/review-larp-mode-slice4`). No baseline discrepancy: the
assignment tail, the bundle head, the review MANIFEST tip, and the
origin head all agreed.

## Scope (§8 only)

- `packages/ui/src/Dashboard.tsx` (new): calm landing screen —
  workspace overview (name, ID, readiness uptime badge, telemetry),
  recent & active projects hub (title, snippet excerpt, pipeline
  stage badge, "Continue Pipeline"), and exactly four quick actions
  (+ New Project, Ask ANANSI, LEXI-COLLIDER, LARP Telemetry Engine).
- `Navigation.tsx`: HOME is now exactly Dashboard + Ask ANANSI, per
  §8. The old "Workspace" sidebar button is removed.
- `Workspace.tsx`: Dashboard is the default active view upon
  connection (transition-guarded: only the not-ready → ready
  transition while on the `'workspace'` view navigates; a deliberate
  later trip to the workspace view does not bounce back). Onboarding
  `onFinish` now lands on the Dashboard.
- `Projects.tsx` (additive only): optional `focusProjectId` /
  `focusSection` / `onFocusConsumed` props — the Dashboard's jumps
  open a project and scroll its Guided Walk or collider section into
  view. Absent props preserve today's behavior exactly.
- `tokens.css`: one `.dashboard-stats` flex rule. The global
  `:focus-visible` ring already in the file covers Dashboard controls,
  so no new focus rule was needed.
- No server op, no new artifact type, no schema change, no Rust
  change, no contracts change.

## Honest derivations (stated in the UI and here)

- Telemetry counts come from paged `listMemories` (limit 100 per page
  via `nextOffset`) + one `listRelationships` call: memory nodes =
  kind `'memory'`; active notes = kind `'note'` with status neither
  `'archived'` nor `'superseded'`; relationships = recorded count.
- The uptime badge reads the genuine `Readiness.uptimeMs` from the
  existing local `client.readiness()` call (App already calls it on
  connect; it is a status read, not inference). Formatted as `2h 3m` /
  `45m` / `52s`; if the call fails the badge degrades to plain
  `Ready`, never an error state.
- Project excerpts are the list entries' `snippet` (null → "No
  description yet."). No per-project `readMemory` call.
- Stage badges come from the existing `derivePipeline` mirror over
  each project's artifact types: `Current stage: <label>`, or
  `Pipeline complete` when `current` is null (launch holds evidence).
  The Dashboard does not duplicate stage routing — "Continue
  Pipeline" jumps to the Guided Walk, whose own Continue logic targets
  the first non-Complete completable step.
- The `'workspace'` view is NOT deleted: it still renders the
  pre-connection create/open flow and the workspace management card +
  Vault, reachable from the Dashboard's "Manage workspace" action.
  Settings and Connection remain in the nav.
- The LEXI-COLLIDER quick action opens the first project in list
  order at its collider section; with no projects it navigates to the
  Projects view (creation first). The button's help text says so.
- The New Project action navigates to the Projects view, which
  remounts with no project selected, so the creation form shows.

## Tests

- `packages/ui/tests/dashboard.test.tsx` (7 tests): `formatUptimeMs`
  (seconds/minutes/hours/days/negative), `projectExcerpt` (null,
  blank, trim, 160-char truncation), `countTelemetry` (kind split +
  archived/superseded exclusion), `stageBadge` (current label vs
  `Pipeline complete`), and static-markup renders of `DashboardView`
  (overview + telemetry + quick actions, empty-state invitation,
  project card with excerpt and badge).
- `tests/e2e/dashboard.spec.ts` (4 tests): (1) connection lands on
  the Dashboard (name, Ready badge, all four quick actions,
  `aria-current="page"` on the nav Dashboard button); (2) quick
  actions — Ask jump, back to Dashboard, LARP toggle shows the HUD,
  New Project lands on the creation form; (3) seeded project card +
  Continue Pipeline → Projects view with Guided Walk heading and
  "Edit project:" title; (4) fresh workspace with no projects shows
  the empty-state invitation and zero Continue buttons. Navigation
  buttons and quick actions share labels (Ask ANANSI), so every click
  is scoped to its region.
- Shared-harness change (additive): `collider-three-helpers.ts` gains
  an `openWorkspace` step, extracted verbatim from `pairAndOpen`,
  which now calls it before the Projects clicks. Existing specs are
  unaffected (identical steps).

## In-slice fixes (disclosed)

- The pre-existing `components.test.tsx` navigation test rendered
  `Navigation` with `view="workspace"` and expected an
  `aria-current="page"` button; that button was the removed Workspace
  entry. Fixed as test maintenance: the render now uses
  `view="dashboard"` (the new landing view), plus explicit assertions
  that Dashboard carries `aria-current` and no button is labeled
  exactly `Workspace`. Button count stays 16.
- The empty-state e2e test first failed honestly: a fresh
  API-created workspace is not onboarded, so Onboarding shows instead
  of the Dashboard. Fixed by driving the real onboarding flow in the
  test (review checkbox → Continue to AI setup → Skip AI setup and
  enter ANANSI) before asserting the Dashboard — which also proves
  the onboarding → Dashboard landing.

## Verification (Linux)

- `tsc --noEmit`: 0 errors.
- `node scripts/contracts.mjs --check`: 0 drift.
- `node scripts/validate-docs.mjs`: PASS (261 files).
- `npm run test:ui`: 277/277 (270 + 7 new dashboard tests).
- Production web build clean (`.dashboard-stats` verified in bundled
  CSS).
- `dashboard.spec.ts`: 4/4 PASS.
- Full regression battery: 44/44 (run sequentially on an idle machine,
  with REAL spec filenames and FULL summaries per the Slice 3 lesson).
- Backend Rust tests: 616 passed + 4 known live-Ollama env failures (no
  Rust files touched; identical to baselines).
- Credentials sweep over the diff vs baseline: clean (only the
  `tokens.css` filename itself matched the token pattern).

## Known limitations

- "Most recent project" for the collider jump means first in the
  server's `listMemories` order, not a separately sorted recency.
- Telemetry paging costs one list round-trip per 100 memories on very
  large workspaces.
- The Dashboard does not surface tasks, people, or jobs — §8 calm
  scope; all remain one nav click away.

## Next

- Windows rerun of this slice (checklist in the delivery MANIFEST).
- No MATRIX row exists for the Dashboard track in this slice's scope
  (none invented); §8 itself is a spec section, not a requirement ID.

Last verified commit: tip recorded in the delivery MANIFEST.
