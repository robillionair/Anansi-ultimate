# HOME Command Dashboard — Slice 2 Addendum

Branch `muse/dashboard-slice2` from Windows-certified Dashboard Slice 1
review tip `c5de4fad9334b06f88b6114f69e6c19c32dcd34e`
(`codex/review-dashboard-slice1`). Baseline verified from origin by
hash: origin head equals the assignment tail exactly, parent is
exactly the Slice 1 delivery tip `f57889edb`, diff is docs-only
(BUILD_STATUS + RECOVERY + 67-line review report), report confirms
the Slice 1 delivery hashes (ZIP `d590735b…`, bundle `dfdc34a0…`),
verdict PASS zero findings, docs 262 with report, UI 277/277, Rust
lib 215/215, dashboard spec 4/4, battery 44/44. Dashboard Slice 1 is
therefore CLOSED on Windows. Honest note: only the review ZIP's
`.sha256` sidecar was uploaded (expected hash `04b996cc…`); the ZIP
itself lives on the owner's Windows path and could not be re-hashed
from here. The content-addressed commit on origin is what this slice
branches from (LARP Slice 3 precedent). Reviewer wording NOT adopted:
the report claims paths `packages/ui/src/dashboard/Dashboard.tsx`
and `packages/ui/src/components/...` (actual paths have no such
subdirectories) and telemetry including "Ask runs, tokens, cost"
(Slice 1 shows memory nodes, active notes, relationships).

## Scope (§8, §50, §66 dashboard system & queue health)

- System & Storage Health section in `Dashboard.tsx`: backup status
  from genuine `readBackupSchedule` (scheduled frequency or
  `Manual backups only`) with a Backups action, and background queue
  status from genuine paged `listJobs` (active count + latest job
  type, or calm `Background queue idle`) with a Jobs action.
- Projects hub: recency ordering (most recent artifact timestamp or
  project update) + 5-project cap with a "View all in Projects" link
  beyond 5.
- `Navigation.tsx`: `Dashboard` removed from
  `plannedGroups[0].items` (HOME planned tools) — it is live now.
- No new server op, no new schema, no paid inference, no Rust change,
  no contracts change.

## Contract correction (disclosed, not silently followed)

The assignment described queue activity as status `'running'` or
`'pending'`. The contract's `JobState` has no such values: it is
`"Queued" | "Running" | "Paused" | "Completed" | "Failed" |
"Cancelled"`. Slice 2 implements active = `'Queued'` or `'Running'`
(the same set `Jobs.tsx` treats as pausable work; `'Paused'` is not
active work). Likewise, `Job` carries no dedicated type field, so the
"latest job type" label reuses `Jobs.tsx`'s exact selection mapping
(backup → Workspace/Application data backup, project → Project
export, records → Selected records export, otherwise Workspace
export).

## Tests

- `dashboard.test.tsx` (+7 tests, 14 total): `formatBackupInterval`
  (days/hours/minutes/seconds/zero), `formatBackupStatus` (scheduled
  vs manual), `deriveQueueStatus` (idle on empty/completed-only/
  paused-only; active count; latest by `createdAt`; selection label
  mapping incl. the backup application variant),
  `latestArtifactTimestamp` (newest parseable, junk ignored),
  `sortProjectsByRecency` (artifact time beats update time,
  unparseable → oldest, ties keep order), and a static-markup render
  asserting the health region, the 5-cap (6 projects in → 5 cards +
  "View all in Projects"), no "View all" link at exactly 5, and the
  active-queue text.
- `dashboard.spec.ts` (+2 tests, 6 total): health section shows
  `Manual backups only` on a fresh workspace and the Backups action
  lands on the Backups view; health section shows
  `Background queue idle` and the Jobs action lands on the Jobs view.
  Active-job e2e is deliberately not added (it would need enqueueing
  a real job; the idle path plus unit-covered active derivation is
  the honest scope).
- Slice 1 test data updated for the extended `DashboardData` shape
  (backup status, queue, total projects) — same render assertions,
  no behavior change.

## Verification (Linux, assignment battery)

- `tsc --noEmit`: 0 errors.
- `node scripts/contracts.mjs --check`: 0 drift.
- `node scripts/validate-docs.mjs`: PASS (264 files).
- `npm run test:ui`: 284/284 (277 + 7 new dashboard tests).
- Production web build clean.
- `cargo --lib` (per the assignment's battery list): 215 passed, 0 failed
  (no Rust files touched).
- `dashboard.spec.ts`: 6/6 PASS.
- Full regression battery: 46/46 (run sequentially on an idle machine,
  with REAL spec filenames and FULL summaries per the Slice 3 lesson).
- Credentials sweep over the diff vs baseline: clean.

## Known limitations

- The health reads join the single dashboard load: if the backup
  schedule read fails, the whole dashboard shows its retry state
  rather than a partial panel.
- Queue "latest" is by `createdAt`, not by last state change.
- The HOME planned-tools group now renders with an empty item list;
  the group itself was kept (minimal diff per the assignment's exact
  instruction).

## Next

- Windows rerun of this slice (checklist in the delivery MANIFEST).

Last verified commit: tip recorded in the delivery MANIFEST.
