# HOME Command Dashboard — Slice 2 Design

Branch `muse/dashboard-slice2` from Windows-certified Dashboard Slice 1
review tip `c5de4fad9334b06f88b6114f69e6c19c32dcd34e`
(`codex/review-dashboard-slice1`). Baseline verified from origin by
hash (LARP Slice 3 precedent): origin head equals the assignment
tail exactly, parent is exactly the Slice 1 delivery tip `f57889edb`,
diff is docs-only (BUILD_STATUS + RECOVERY + 67-line review report),
report confirms the Slice 1 delivery hashes (ZIP `d590735b…`, bundle
`dfdc34a0…`), verdict PASS zero findings, docs 262 with report, UI
277/277, Rust lib 215/215, dashboard spec 4/4, battery 44/44.
Dashboard Slice 1 is therefore CLOSED on Windows. One honest note:
only the review ZIP's `.sha256` sidecar was uploaded (expected hash
`04b996cc…`); the ZIP itself lives on the owner's Windows path and
could not be re-hashed from here. The content-addressed commit on
origin is what this slice branches from, which is sufficient.
Reviewer wording NOT adopted: the report claims paths
`packages/ui/src/dashboard/Dashboard.tsx` and
`packages/ui/src/components/...` (actual paths have no such
subdirectories) and telemetry including "Ask runs, tokens, cost"
(Slice 1 shows memory nodes, active notes, relationships — no
Ask-run/token/cost aggregates exist on the Dashboard).

## Scope (§8, §50, §66 dashboard system & queue health)

- System & Storage Health section in `Dashboard.tsx` (new region,
  `aria-labelledby="dashboard-health-heading"`).
- Projects hub recency ordering + 5-project cap.
- `Navigation.tsx`: remove `Dashboard` from `plannedGroups[0].items`
  (HOME planned tools) — it is live now.
- No new server op, no new schema, no paid inference, no Rust change,
  no contracts change. All new/touched imports carry explicit
  extensions.

## Health derivations (all genuine local reads)

- Backup: `client.readBackupSchedule(workspaceId)` →
  `BackupSchedule { enabled, intervalSeconds, ... }`.
  `formatBackupStatus(schedule)` = enabled
  ? `Auto-backup every ${formatBackupInterval(intervalSeconds)}`
  : `Manual backups only`. `formatBackupInterval`: divisible by
  86400 → `${d}d`; else by 3600 → `${h}h`; else by 60 → `${m}m`;
  else `${s}s`. Secondary action → `'backups'` view.
- Queue: `client.listJobs(workspaceId)` paged by `nextOffset` (same
  pattern as the memory paging in Slice 1).
  CORRECTION to the assignment's wording: the contract's job states
  are capitalized — `JobState = "Queued" | "Running" | "Paused" |
  "Completed" | "Failed" | "Cancelled"`. There is no `'running'` /
  `'pending'` status. Active = state `'Queued'` or `'Running'`
  (the same set `Jobs.tsx` treats as pausable work; `'Paused'` is
  not active work).
  `deriveQueueStatus(jobs)`: no active jobs →
  `{ idle: true, text: 'Background queue idle' }`; otherwise
  `{ idle: false, activeCount, latestType }` where latest = the
  active job with the greatest `createdAt` (ties → first in list
  order). "Latest job type" has no dedicated field on `Job`, so the
  label reuses `Jobs.tsx`'s exact selection mapping:
  `backup` → `Workspace backup` / `Application data backup`
  (by `selection.application`), `project` → `Project export`,
  `records` → `Selected records export`, otherwise
  `Workspace export`. Display: `N active job(s) · latest: <label>`.
  Direct link → `'jobs'` view.
- Failure handling (unchanged from Slice 1): the health reads join
  the container's single load; a failed local read surfaces the
  dashboard error + Retry state rather than a fabricated partial
  health panel.

## Recency sorting + cap

- New pure helper `sortProjectsByRecency(projects)` where each entry
  carries `updatedAt` (project record) and `latestArtifactAt`
  (max `ArtifactRecord.timestamp` across that project's artifacts,
  unparseable → ignored). Key = max(parsed latest artifact time,
  parsed `updatedAt`); unparseable both → 0 (oldest). Sort
  descending, stable (ties keep list order).
- Container: per project it already loads artifacts for the stage
  badge, so the latest artifact time comes from the same list at no
  extra call. `DashboardView` receives the sorted, capped list plus
  `totalProjects`; when `totalProjects > 5` it renders a
  "View all in Projects" link → `'projects'` view.
- "Most recent" for the LEXI-COLLIDER quick action now means first
  in the recency-sorted order (previously first in list order);
  the button's help text is unchanged in meaning.

## Tests

- `dashboard.test.tsx` additions: `formatBackupInterval` (days /
  hours / minutes / seconds / zero), `formatBackupStatus` (scheduled
  vs manual), `deriveQueueStatus` (idle on empty / completed-only /
  paused-only; active count; latest by createdAt; selection label
  mapping incl. backup application variant), `sortProjectsByRecency`
  (artifact time beats updatedAt; unparseable → oldest; ties keep
  order), and a static-markup render asserting the health region,
  the 5-cap (6 projects in → 5 cards + "View all in Projects"),
  and no "View all" link at exactly 5.
- `dashboard.spec.ts` additions (2 tests): (1) backup schedule
  display + navigation to Backups — fresh workspace shows
  `Manual backups only`, click the Backups action → Backups view
  visible; (2) background queue display — fresh workspace shows
  `Background queue idle` + Jobs link → Jobs view visible. Active-job
  e2e is not added (it would need enqueueing a real job; the idle
  path plus unit-covered active derivation is the honest scope).

## Verification (assignment battery)

tsc, contracts, docs, `test:ui`, vite build, api build,
`cargo --lib` (per the assignment's battery list — the reviewer's
Slice 1 report likewise ran `--lib`, 215 tests), slice spec
standalone, full battery with real filenames + full summaries, sweep
over the diff vs `c5de4fad`.

## Known limitations

- The health reads join the single dashboard load: if the backup
  schedule read fails, the whole dashboard shows its retry state
  rather than a partial panel.
- Queue "latest" is by `createdAt`, not by last state change; a
  long-running job started earlier is not "latest" if a newer job
  queued after it.
