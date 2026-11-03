# HOME Command Dashboard — Slice 1 Design

Branch `muse/dashboard-slice1` from Windows-certified LARP Slice 4
review tip `ac2f05aba7647c48646ef2ae013a3be5a1084f72`
(`codex/review-larp-mode-slice4`, verified in the previous turn:
ZIP `734a219e…`, bundle `e9a3cbff…`, docs-only diff over delivery tip
`8b4e37d1f`, PASS zero findings). No baseline discrepancy this time:
assignment tail, bundle head, MANIFEST tip, and origin head all agree.

## Spec anchor

MASTER_SPEC §8 (MAIN USER INTERFACE): HOME contains Dashboard and
Ask ANANSI; "Do not display a massive dashboard full of experimental
tools. Show beginners a small number of meaningful actions." This slice
implements exactly that: one overview, one project hub, four quick
actions. No new server op, no new artifact type, no schema change, no
Rust change, no contracts change.

## Navigation (Navigation.tsx)

- `View` gains `'dashboard'`.
- HOME group becomes exactly Dashboard + Ask ANANSI, per §8. The old
  "Workspace" nav button is removed from the sidebar.
- The `'workspace'` view itself is NOT deleted: it still renders the
  pre-connection create/open flow (when no workspace is selected) and
  the workspace management card + Vault (when one is open). It is
  reachable from the Dashboard's workspace card ("Manage workspace"),
  so nothing is stranded; Settings and Connection stay in the nav as
  the assignment states.
- Default landing: when a workspace transitions from not-ready to
  ready while the view is `'workspace'`, Workspace navigates to
  `'dashboard'` (transition-guarded with a ref, so a deliberate trip
  to the workspace view afterwards does NOT bounce back). Onboarding
  `onFinish` now navigates to `'dashboard'` instead of `'workspace'`.
  Existing specs are safe: `pairAndOpen` clicks Projects + Harbor App
  explicitly after opening, so the landing change cannot strand them.

## Dashboard (packages/ui/src/Dashboard.tsx)

Container + presentational split, so unit tests can render the
presentational `DashboardView` with static markup (effects do not run
there; click behavior is covered by e2e, same pattern as LARP tests).

Container loads, all local list reads, zero inference, zero fake
traffic:
- `listMemories({ kind: 'project' })` → project cards.
- Per project, `listProjectArtifacts(workspaceId, projectId)` →
  `derivePipeline` (existing `lifecycle.ts` mirror) → current stage.
- `listMemories` paged by `nextOffset` (limit 100) → telemetry
  counts. `listRelationships` (single call, no paging in the type).
- `client.readiness()` → `uptimeMs` for the badge. Readiness is a
  local status read, not inference; App already calls it on connect.

Definitions (stated in the UI, not hidden):
- Memory nodes: records with kind `'memory'`.
- Active notes: kind `'note'` with status neither `'archived'` nor
  `'superseded'`.
- Relationships: `listRelationships` count.
- Uptime badge: `Ready · core up <humanized uptimeMs>` (e.g. `2h 4m`,
  `45m`, `52s`). If the readiness call fails, the badge degrades to
  `Ready` with no uptime — never an error state for a ready workspace.
- Project excerpt: the list entry's `snippet` (null → "No description
  yet."). No per-project `readMemory` call is needed.
- Stage badge: `Current stage: <label>` from
  `derivePipeline(...).current`; when `current` is null (launch holds
  evidence) the badge reads `Pipeline complete`. The card's
  "Continue Pipeline" button jumps to the project's Guided Walk; the
  walk's own Continue logic already targets the first non-Complete
  completable step, so the Dashboard does not duplicate stage routing.

Sections (each `role="region"` + `aria-labelledby`):
1. Workspace overview: name, ID, readiness uptime badge, telemetry
   numbers, "Manage workspace" button → `'workspace'` view.
2. Recent & active projects hub: one card per project (title, excerpt,
   stage badge, "Continue Pipeline"). Empty state: "No projects yet —
   create your first project." + the New Project action.
3. Quick Action Launcher, exactly four buttons:
   - `[ + New Project ]` → `'projects'` view (remounts with no project
     selected, so the creation form shows).
   - `[ Ask ANANSI ]` → `'ask'` view.
   - `[ LEXI-COLLIDER ]` → most recent project (first in list order)
     opened at its collider section; with no projects, → `'projects'`
     view (creation first). Documented on the button's help text.
   - `[ LARP Telemetry Engine ]` → `toggleLarpMode()` from the LARP
     store; label carries the live On/Off state via
     `useLarpModeStore()`. Toggling does not navigate; the HUD appears
     over the current view.

## Projects focus jump (Projects.tsx, additive only)

New optional props `focusProjectId?: string` and
`focusSection?: 'walk' | 'collider'`. On mount with a focus request,
the component opens that project (existing `open()`), then scrolls
the `project-walk-heading` or `project-collider-heading` element into
view once the project is current, and clears the request. No change to
the default path (props absent → today's behavior exactly).
Workspace holds the focus request in state and passes it to Projects;
navigating to Projects by any other route passes nothing.

## Styling

Reuse `.card`, `.actions`, `.badge` from `tokens.css`. One small
addition: a `.dashboard-stats` flex row for the telemetry numbers.
The global `:focus-visible` ring already in `tokens.css` covers
Dashboard buttons, inputs, and links — no new focus rule needed; the
slice relies on it and the e2e spec tabs through the quick actions.

## Tests

- `packages/ui/tests/dashboard.test.tsx`: `formatUptimeMs` (0, seconds,
  minutes, hours, day+), `projectExcerpt` (snippet / null fallback),
  telemetry counting (`countTelemetry`: kinds + archived/superseded
  exclusion), stage badge text (`stageBadge`: current label vs
  `Pipeline complete`), and static-markup renders of `DashboardView`
  (overview + empty state + project card + quick actions present,
  region labelling).
- `tests/e2e/dashboard.spec.ts` (4 tests): (1) landing — fresh
  session, open workspace → Dashboard visible with workspace name +
  uptime badge + quick actions; (2) quick actions — Ask ANANSI jump,
  back to Dashboard, LARP toggle shows HUD, New Project lands on the
  creation form; (3) project jump — seeded project via
  `session.call('memories','create', { kind: 'project', ... })`,
  Dashboard card + Continue Pipeline → Projects view with Guided Walk
  heading visible; (4) empty state — fresh workspace with no projects
  shows the invitation text and no project cards. Per-test timeouts
  300s, `page.setDefaultTimeout(8000)` in beforeEach, `startSession`
  + `cleanup` in finally.

## Verification (same battery discipline as LARP)

tsc, contracts, docs, `test:ui`, vite build, api build, slice spec
standalone, full battery with real filenames + full summaries, Rust
full workspace, credentials sweep over the diff vs `ac2f05ab`.

## Known limitations

- Project list order is the server's `listMemories` order; "most
  recent" for the collider jump means first in that order, not a
  separately sorted recency.
- Telemetry counts page through all memories (limit 100 per page);
  very large workspaces pay one extra list round-trip per page.
- The Dashboard does not surface tasks, people, or jobs — §8 calm
  scope; those remain one nav click away.
