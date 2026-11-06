# SwarmTank v2 — Checkpoint 3: time, what-if branches, LARP

## Scope

Checkpoint 3 of the SwarmTank v2 design (`docs/handoffs/swarmtank-v2-design.md`
§4, AC-T1..T7). A time dimension for AI runs (future end date + step size
within an explicit run budget, with coarser steps when the horizon forces
them), scheduled events the user can add, a what-if branch from a saved
round with a side-by-side comparison, timeline playback (scrubber + persona
cards) in regular SwarmTank and in LARP scene 08, and recorded agent
interactions visually distinct from decorative wiring in the LARP matrix.

## What changed

### Rust: time dimension (`crates/core/src/swarmtank_ai.rs`)

- **Dates.** Howard Hinnant's civil calendar algorithms (no date crate):
  `civil_from_days` / `days_from_civil` / `parse_iso_date` (rejects
  impossible dates by round-tripping, e.g. 2026-02-30) / `format_iso_date`.
  Unit-tested including leap years.
- **Time plan.** `resolve_time_plan(end_date, step_days, max_rounds)`:
  the start date is today; the horizon is `end_date - start_date` in days;
  `step_count = ceil(horizon / step_days)`. When the horizon needs more
  steps than the round budget, the step is coarsened to
  `ceil(horizon / max_rounds)` and `step_coarsened` is set — the run stays
  within budget and the caller is told it happened. Step sizes are 1, 7,
  30 or 90 days. A past end date or an invalid step is refused.
- **Round dates.** Each `AiRound` gains an optional ISO `date`: round N
  lands N steps after the start date, clamped to the end date for the last
  round. The run doc records `startDate`, `endDate`, `stepDays` and
  `stepCoarsened`. The estimate and start responses carry `stepCount`,
  `effectiveStepDays` and `stepCoarsened`, so the UI shows the plan before
  Start. The actual rounds run = `step_count` when an end date is given,
  else the requested rounds.
- **Scheduled events in the panel.** The estimate and start requests accept
  `scheduledEvents` (the estimate request gained the field so the estimate
  accounts for them). Events are validated against the rounds that will
  run. Agents observe the events due in their round (they are in the
  prompt and in the bounded-memory snapshot); the API-level branch test
  proves round 2's agents observed `event-1` in
  `observedEventIds`.

### Rust: what-if branch (`swarmtank-ai-branch`)

- New op `swarmtank-ai-branch`. It reads the parent `swarmtank-ai-run`
  artifact (latest version), parses and validates it, and requires the
  branch round to be one of the parent's completed rounds. The branch
  input is the parent's input with the price changed when asked
  (`priceAmount` + the pricing text rewritten as `$N per month`). New
  events must fall due after the branch point and within the run. The
  branch reuses the parent's time plan, agents, seed and market.
- The runner resumes from the parent's rounds: `resume_rounds` are the
  parent's rounds up to the branch point (copied verbatim into the branch
  doc), the agents' memory is seeded from them, and new rounds run from
  `parentRound + 1` to the total. The branch doc records `parentRunId`,
  `parentArtifactId`, `parentRound` and `branchChanges` (price + new event
  ids). The saved artifact title names the branch
  (`… (complete, branch of <run> from round N): …`) so the saved list
  distinguishes it. The branch resolves its model and Zero-Cost Lock the
  same way a fresh start does.
- Routing: the op is in the API allowlist, the `swarmtank_command`
  dispatcher, the `ai_command` allowlist, and the persisted-config
  lazy-load list (the same four lists as the other AI ops — the dispatcher
  entry was initially missed and caught by a live probe returning "Use a
  supported SwarmTank operation.").

### UI: timeline, branch, comparison (regular SwarmTank)

- **AI panel** (`SwarmtankAi.tsx`): end-date input, step-size select
  (Daily/Weekly/Monthly/Quarterly), the time-plan line ("Time plan: N
  steps of M days." + the coarser-step warning), and a scheduled-events
  editor (kind + description + round, add/remove, listed in the run
  record). The estimate and start calls carry `endDate`, `stepDays` and
  `scheduledEvents`.
- **AI run view**: a timeline scrubber (range input + per-round buttons
  with dates) that selects the round shown in detail; the selected round's
  persona cards show each agent's action, rationale, cited evidence and
  prior interaction, model and tokens. A "Branch from this run" form
  (branch round, new price, new event kind/description/round) starts a
  branch; starting a branch now tracks the new run's progress in the panel
  (the earlier version left the progress section showing the parent).
  A "Compare with the original timeline" section (branch runs only) loads
  the parent artifact and renders `SwarmtankAiComparison`: the original
  timeline beside the branch, rounds marked shared (identical) or diverged,
  the branch price vs the original, both labelled conditional simulation
  results. Longer horizons (≥ 90 days) surface a "Long horizon —
  assumptions matter more" note above the assumption list.
- **Parser**: `parseSwarmtankAiRunBody` accepts the new fields
  (`startDate`, `endDate`, `stepDays`, `stepCoarsened`, `parentRunId`,
  `parentArtifactId`, `parentRound`, `branchChanges`) and requires the
  time fields; `AiRound` accepts the optional `date`.

### UI: LARP scene 08 (`SwarmtankAiMatrix.tsx`)

- New `SwarmtankAiMatrix`: the cinematic card matrix for an AI run,
  rendered in the LARP scene when an AI run is open (the panel passes
  `cinematic` from `arenaHost`). Header carries the market and branch
  labels plus "Conditional simulation result." A timeline scrubber selects
  the round; each persona card shows the agent's action in the selected
  round and what it cited (`Cited: …` / `Prior: …` / `No citation`).
  Selecting a card shows the persona detail (role, round action,
  rationale, cited evidence, prior interaction, model, failure when the
  action failed).
- **Recorded vs decorative (AC-T5).** The decorative grid wires are drawn
  exactly as in the Quick matrix (and the footer still says "WIRING IS
  VISUAL ONLY"). Recorded interactions — an agent citing another agent's
  prior interaction in the selected round, resolved from the run doc's
  `priorInteractionRef` — are drawn as solid gold lines (`.wire-recorded`,
  `#f5c542`) between the citing and cited cards, and the footer labels
  them "RECORDED INTERACTIONS (GOLD LINES) ARE FROM THE SAVED RUN".
  The two are visually and textually distinct, in the UI, not just in code.
- **Reduced motion (AC-T6).** The matrix takes `staticView` from the same
  synchronous reduced-motion probe the arena uses, with its own toggle in
  the AI panel when cinematic; the `.wire-signal` animation is off in the
  static view (and under `prefers-reduced-motion`). The accessible report
  view is the AI run list below the matrix (the run view is a plain list,
  not canvas).

## Acceptance criteria — results (AC-T1..T7)

- **AC-T1** — end date + step size within the run budget; a longer horizon
  uses coarser steps and says so: e2e timeline test 1 (90 days at 30-day
  steps = 3 steps, no coarsening; 365 days at 7-day steps with a 2-round
  budget coarsens and the warning shows; the run completes). API-level:
  a 365-day horizon at 7-day steps with a 2-round budget coarsens to
  183-day steps, `stepCoarsened: true`.
- **AC-T2** — a scheduled event is observed by agents in the round it falls
  due: e2e timeline test 2 (a competitor launch added in round 2 via the
  panel; the run view shows `Events due: event-1` in round 2). API-level:
  round 2's agents have `event-1` in `observedEventIds`.
- **AC-T3** — a branch from a saved round compares against the original:
  e2e timeline test 3 (branch from round 1 with price $79; the branch
  completes with 3 new calls; the comparison shows the branch price
  `$79 (original: $49)`, `Round 1 (shared — identical to the original)`,
  `Round 2 (diverged)`). API-level: the branch doc records the parent,
  parent round, price 79, new event `event-2` due in round 3; round 1's
  action is copied verbatim; round 3 has `event-2` due.
- **AC-T4** — the scrubber selects a round; persona cards show that round's
  action and cited evidence/prior event: e2e timeline test 2 (scrubber to
  round 2 shows `Prior: round-1-agent-1`; back to round 1 shows the call-1
  action) and the LARP AI test (matrix scrubber to round 2, cards show the
  cited prior interaction; selecting a card shows the detail).
- **AC-T5** — recorded interactions and decorative wiring are visually and
  textually distinct in LARP: the LARP AI test asserts both
  `.wire-filament` (decorative) and `.wire-recorded` (gold) are attached,
  and both footer labels are visible; the screenshot shows the gold lines
  and the legend.
- **AC-T6** — reduced motion and the accessible report view still work on
  the AI run and the branch: the LARP AI test asserts the matrix has
  `data-static="true"` and the `.wire-signal` animation is `none`, and the
  run list (accessible view) is present; no renderer errors. The branch
  comparison is a plain list (no canvas).
- **AC-T7** — market setup and playback work in regular SwarmTank: e2e
  timeline test 4 (Estonia researched profile selected, AI run started,
  the run view shows `Market: Estonia`, the scrubber moves between rounds).

## Verification (Linux, this branch)

- Rust workspace lib: **277 passed, 0 failed** (api 4 + core 212 + desktop
  1 + providers 60). Core +2 (the time-plan tests).
- TypeScript: **0 errors**. Contracts `--check`: **0 drift**. Docs
  validator: **311 PASS**.
- UI tests: **326 passed, 0 failed**.
- E2E `tests/e2e/swarmtank-ai-timeline.spec.ts`: **4/4** (end date +
  coarsening, scheduled event + scrubber, branch + comparison, market +
  playback).
- E2E `tests/e2e/larp-swarmtank-ai.spec.ts`: **1/1** (LARP matrix,
  scrubber, recorded interactions, reduced motion, no renderer errors).
- E2E `tests/e2e/swarmtank-ai.spec.ts`: **4/4** (Checkpoint 2, re-run;
  the main test was updated to select round 1 via the scrubber, since the
  run view now defaults to the last round).
- Regressions: `swarmtank-market` 3/3, `larp-swarmtank` 1/1,
  `swarmtank-arena` 3/3.
- API build: clean except the pre-existing `mcp.rs:715` warning.
- Direct API branch test (`evidence/ai-branch-api.log`): parent 3 rounds
  with dates (2026-12-06, 2027-01-05, 2027-02-04), round 2 observes
  `event-1`; coarsening to 183-day steps; branch completes (6 new calls,
  3 rounds), parent/price/event recorded, round 1 copied, round 3 has
  `event-2` due.

## Defects found and fixed in this checkpoint

1. **Branch op not routed.** `swarmtank-ai-branch` returned "Use a
   supported SwarmTank operation." until it was added to the
   `swarmtank_command` dispatcher (the fourth routing list).
2. **`plan` moved before use.** The `TimePlan` was moved into the thread
   config and then read again for the response; the response fields are
   now captured into locals before the move (two sites).
3. **Branch progress not tracked.** Starting a branch left the progress
   section showing the parent run, because `startBranch` never set the
   panel's run id / polling state. It now receives the branch run id and
   starts polling it.
4. **Duplicate aria-label.** The outer "Branch comparison" section and the
   inner comparison table shared one label (a strict-mode violation in a
   screenshot helper); the inner table is now "Branch comparison table".
5. **Test expectation vs correct coarsening.** The first timeline test
   expected 3 steps of 30 days with the default 2-round budget; the
   coarsening to 45-day steps was correct behaviour, and the test now
   raises the round budget to 3 first.
6. **Run view defaults to the last round.** The Checkpoint 2 main e2e
   asserted a round-1 action after opening a run; the scrubber now
   defaults to the last round, so the test selects round 1 first.

## Known limitations

- The branch's new rounds reuse the parent's agents and market; a branch
  cannot change the market or the agent set (only the price and the
  events). Changing the market is a new run, not a branch.
- The branch runs the remaining rounds with the parent's token budget
  defaults unless the caller overrides them; the branch's own
  `callsMade`/`tokensUsed` count only its new calls (the parent's calls
  are in the parent doc).
- The recorded-interaction links in the LARP matrix only connect agents
  on the same matrix page (12 per page); a citation to an agent on another
  page is shown as text on the card, not as a line.
- Round dates are computed from the run's start date (its creation date);
  a branch keeps the parent's start date and step size.
- AC-A10 (local-model smoke test) remains Windows-pending from Checkpoint 2.

## Proposed global-status updates (for Antigravity; not applied here)

None to AN-014 — SwarmTank v2 is new scope on top of the verified M9
milestone, and this checkpoint adds no new requirement row.

## Windows acceptance checklist (Checkpoint 3)

1. Verify the branch tip and bundle hashes (see the delivery MANIFEST).
2. `cargo test --locked --workspace --lib` — expect 277 passed.
3. `npm run typecheck` (0), `node scripts/contracts.mjs --check` (0 drift),
   `node scripts/validate-docs.mjs` (311 PASS), `npm run test:ui` (326).
4. Build the API and the web app; run `tests/e2e/swarmtank-ai-timeline.spec.ts`
   (4/4), `tests/e2e/larp-swarmtank-ai.spec.ts` (1/1),
   `tests/e2e/swarmtank-ai.spec.ts` (4/4), `tests/e2e/swarmtank-market.spec.ts`
   (3/3), and `tests/e2e/larp-swarmtank.spec.ts` +
   `tests/e2e/swarmtank-arena.spec.ts` (4/4) through the shared
   application-test lease.
5. In the browser: open SwarmTank, switch to AI simulation, set an end date
   90 days out with monthly steps and confirm the time plan reads 3 steps;
   set a 365-day horizon with weekly steps and a 2-round budget and confirm
   the coarser-step warning; add a competitor-launch event in round 2 and
   confirm the run view shows it due in round 2; open the run, move the
   timeline scrubber between rounds, and confirm the persona cards show the
   selected round's action and citation; branch from round 1 with a price
   rise, confirm the branch completes, and compare it with the original
   (shared round 1, diverged round 2, branch price shown).
6. In LARP scene 08: run an AI simulation, open the saved AI run, confirm
   the AI matrix appears with the scrubber and the persona cards, confirm
   the gold recorded-interaction lines and both footer labels are present,
   and confirm reduced motion renders the static matrix.
7. With a configured local model, run AC-A10 (Checkpoint 2): record the
   actual calls, latency and token usage.
