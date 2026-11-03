# M12 Product Lifecycle Slice 3 — Design: Integrated Lifecycle Guided Walk & Milestone 12 Closure

Status: design committed before code. Baseline: Windows-verified Slice 2 review tip
`30f1ee91dfa9f88bbc4ae1f5c8bebb3cfdc5b74c` (review branch
`codex/review-m12-product-lifecycle-slice2`, single docs-only commit over delivery tip
`77583b7c0`). Branch: `muse/m12-product-lifecycle-slice3`.

## 1. What this slice is

Slice 1 built the pipeline stage engine and navigator (§60, §62 foundational).
Slice 2 built Launch Studio (§61 bounded subset). Slice 3 closes Milestone 12 by
making the pipeline walkable end to end and proving, with a real browser run,
that the walk survives restart and backup/restore (§62, §85, §88).

Scope, exactly as assigned:

1. The complete §85 verification walk as six pipeline steps: Discover, Define,
   Simulate, Pressure-Test, Refine, Launch.
2. State recovery and persistence integrity (§62): pipeline state and stage
   statuses reconstruct deterministically across project restart and full
   database backup/restore; artifact associations stay bound to the active
   project context.
3. Milestone report and closure documentation (§88): the addendum is the §88
   report; `BUILD_STATUS.md` and `REQUIREMENTS_MATRIX.md` advance AN-012 to
   `verified` with "Milestone 12 complete" in its text.

## 2. Scoping notes (read before reviewing claims)

- **§85 has 17 steps; the walk has 6.** The assignment's walk maps the six
  pipeline stages, not all 17 §85 steps. Steps 7–10 and 13 and 16 (create a
  repository, generate application code, run tests in an approved sandbox,
  preview a working prototype, request a revised prototype, export source code
  and documentation) touch M7 source-package generation and the §32 execution
  sandbox, which are separately gated and are NOT built here. The walk's
  Define step covers concept + specification draft only — no source package,
  no code execution, no deployment. The addendum states this boundary
  explicitly; "Milestone 12 complete" means the M12 lifecycle scope, not the
  M7 production gates, which stay open and are preserved as remaining gates.
- **Assignment wording: "nightmare artifact".** No artifact type named
  `nightmare` exists in `ARTIFACT_TYPES` (21 entries). SOMNIA saves an
  evaluation of nightmare scenarios as a `somnia-evaluation` artifact. The
  walk's Pressure-Test step registers a `somnia-evaluation` artifact. The
  wording is recorded here, not silently corrected in code.
- **Assignment wording: "opportunity artifact".** Opportunity sets save via
  `saveProjectOpportunities` and register under the `idea` type, which is the
  pipeline's `discover` evidence (Slice 1 design: M6 ideas and M6
  opportunities both register as `idea`). The walk's Discover step registers
  an `idea`-typed opportunity set. No new artifact type is introduced.
- **AN-012 to `verified`.** The docs validator accepts only
  `provisional|planned|in progress|verified|blocked|deferred` — there is no
  `complete` status. Following the M10 (AN-015) and M11 (AN-016) precedent,
  AN-012 is advanced to `verified` with "Milestone 12 complete" in its text,
  while the addendum preserves the still-open gates (M7 native desktop
  acceptance + preview promotion, §32 sandbox, M5 production gates, Windows
  verification of M8 Slice 4) as remaining work. `verified` here must not be
  read as closing those gates.
- **Launch Complete still requires manual registration.** Slice 2's bundle is
  download-only, not registered. The walk teaches the honest final step:
  after downloading the bundle, the user registers a `launch-material`
  artifact through the existing manual form. Only then does the Launch stage
  (and the walk) read Complete. No silent auto-registration is added.

## 3. New product code: the Guided Walk panel

The six walk steps already work as tools (Opportunity panel, Collider +
ForgeSpecPanel, SwarmTank panel, SOMNIA panel, GHOST-PROMPT strip, Launch
Studio). What is missing is the guide itself: an ordered, plain-language
checklist that tells a new user what to do at each stage, in order, and jumps
to the right tool. That is what this slice builds.

### 3.1 Derivation (`packages/ui/src/guidedWalk.ts`)

Pure functions over the artifact-type list, built on the Slice 1 engine
(`derivePipeline` from `packages/ui/src/lifecycle.ts`). No server op, no new
artifact type, no schema change, no Rust change.

Walk steps, in order:

| Step | Stages | Complete iff | Target |
|------|--------|--------------|--------|
| Discover | `discover` | `idea` evidence present | `#project-opportunities-heading` |
| Define | `invent` + `build` | concept AND specification present | concept missing → `#project-collider-heading`, else `#project-artifacts-heading` |
| Simulate | `simulate` | simulation/scenario present | `#project-swarmtank-heading` |
| Pressure-Test | `pressure_test` | `somnia-evaluation` present | `#project-somnia-heading` |
| Refine | `refine` | never (see §3.2) | Ask ANANSI via `onNavigateToAsk` |
| Launch | `launch` | `launch-material` present | Launch Studio via `onSelectLaunch` |

Step status rules:

- Single-stage steps reuse the pipeline stage status verbatim (Complete,
  Current / In Progress, Available, Awaiting Preceding Stage).
- Define: Complete iff `invent` Complete AND `build` Complete. Otherwise
  Current iff the pipeline current stage is `invent` or `build`; otherwise
  Available iff a concept is present (the `build` gate is met); otherwise
  Awaiting Preceding Stage with the build requirement sentence ("Needs at
  least one Forge concept artifact."). The spec-without-concept edge (manual
  registration) reads Awaiting, honestly: the Define step is incomplete
  without the concept.
- Refine: never Complete and never Current, exactly like the pipeline's
  `refine` stage (GHOST-PROMPT sessions are in-memory only and are never
  registered). It is always Available with its own button.

"Continue walk" target: the first step in walk order among the five
completable steps (Discover, Define, Simulate, Pressure-Test, Launch) that
is not Complete. Refine is deliberately excluded from Continue targeting:
it never completes, so including it would pin Continue on Refine forever
and block the Launch step. The Refine step still has its own button, and
the walk order still lists it between Pressure-Test and Launch. When all
five completable steps are Complete, Continue disappears and the panel
shows the completion note: "Walk complete — all five completable steps are
done. Refine stays available as an ongoing practice; it is not a
deliverable and never reads Complete."

### 3.2 Component (`packages/ui/src/GuidedWalk.tsx`)

- `<section aria-labelledby="project-walk-heading">` with an `<ol>` of steps.
  Each step row shows its number, name, status text, one-line instruction,
  and a button (anchor link for in-view tools, callback button for Ask and
  Launch — the same defensive shape as `PipelineNavigator`: the callbacks
  are optional props, and the buttons render only when the callback exists).
- Mounted in `Projects.tsx` immediately after `PipelineNavigator`, fed by
  the already-loaded `artifacts` state, with the same `onNavigateToAsk` and
  `onSelectLaunch` plumbing. All imports of new modules use explicit
  `.ts`/`.tsx` extensions (NTFS lesson).
- Instructions (plain language, one line each):
  - Discover: "Generate an opportunity set from your sources, then save it
    as an artifact."
  - Define: "Create a concept in LEXI-COLLIDER, then draft its
    specification and plan in Product Forge."
  - Simulate: "Run a SwarmTank simulation across 20 personas and save the
    run."
  - Pressure-Test: "Run a SOMNIA adversarial evaluation over nightmare
    scenarios and save it."
  - Refine: "Open Ask ANANSI, enable Draft Exploration, revise, review
    fragments, then wipe the session."
  - Launch: "Open Launch Studio, generate the four-channel copy, download
    the bundle, and register it as a launch-material artifact."

### 3.3 Unit tests (`packages/ui/tests/guided-walk.test.tsx`)

- Derivation across artifact sets: empty; `idea` only; `idea`+`concept`;
  `idea`+`concept`+`specification`; +`simulation`; +`somnia-evaluation`;
  +`launch-material`; and the spec-without-concept edge (Define Awaiting).
- Component rendering: statuses, Continue target selection (skips Refine,
  disappears at completion), completion note, Refine never Complete.
- No new file list entry is needed beyond the test itself: the root
  `package.json` `test:ui` list must gain the new test file (house rule).

## 4. Shared harness: prompt-routed fake (`tests/e2e/collider-three-helpers.ts`)

The shared fake currently returns collider JSON for every prompt. The walk
drives two more provider-backed panels through the same fake, so the fake
routes on the server-side instruction strings (verified in Rust source,
all distinct):

- Body contains `opportunity discovery engine` (from
  `crates/core/src/opportunities.rs` INSTRUCTION) → opportunity JSON: six
  concepts in the opportunities.spec.ts fixture shape, each citing
  `supplied-1` (the single problem source the walk supplies; no memory
  source, so no memory id is cited).
- Body contains `product architect` (from `crates/core/src/forge.rs`
  SPEC_INSTRUCTION) → spec JSON in the forge.spec.ts fixture shape, titled
  after the walk's concept: `Mycelial order router`, with a routing-layer
  summary, features, pages, non-goals, dataAndState, acceptance criteria
  and open questions.
- Otherwise → the existing collider JSON, unchanged.

Safety: the collider instruction is "You are ANANSI's LEXI-COLLIDER
engine." — it contains neither marker. No existing spec that uses
`startSession` calls the provider with anything but collider prompts
(ghost, SOMNIA and SwarmTank panels are deterministic; the slice1/slice2
specs' only provider traffic is the collider step, and slice2's
no-additional-hits assertion covers the studio path, which is local).
The regression battery is the proof: if routing misclassified a prompt,
a collider, ghost, SOMNIA or SwarmTank spec would fail.

No other harness change. In particular, no server-restart helper is added:
the restart leg (§5, test 1) restarts the app client (full page reload +
re-pair against the same running API), which proves the client rebuilds
pipeline state from the server's persisted registry. Server-process
restart is not separately exercised; disk-level persistence is proved by
the backup/restore leg (test 2), which reads restored state from the
backup artifact in a fresh workspace. Both limits are stated in the
addendum, not hidden.

## 5. End-to-end walk (`tests/e2e/product-lifecycle-slice3.spec.ts`)

One spec, two tests, `test.setTimeout(300000)` each,
`page.setDefaultTimeout(8000)`, via `startSession(page, false)`.

### Test 1: the complete walk, with a mid-walk restart

Fresh project "Harbor Walk" (created through the UI like the
opportunities spec). At each step, assert the walk panel status before
acting and after the artifact lands.

1. **Discover.** Walk shows Discover Current, the rest Awaiting; Continue
   jumps to the opportunities section. Fill `Source text` (problem:
   dock offices lose supplier quotes in shared inboxes), `Discover
   opportunities`, wait for the set, `Save opportunity set as artifact`,
   assert the notice and walk Discover Complete.
2. **Define.** `collideAndSave` (kitchen/orbital beams) → open the
   experiment → Build This Product, combination `Mycelial order router` →
   notice "Built: Forge concept created from the approved outputs." →
   reopen the project → open the concept artifact → ForgeSpecPanel: fill
   purpose / intended users / must-have → `Generate product
   specification` → heading "Product specification: Mycelial order
   router" → `Save specification draft` → notice → walk Define Complete.
3. **Restart checkpoint.** `page.reload()` + `pairAndOpen(workspaceId)` +
   reopen the project. Assert walk Discover and Define still Complete
   and Simulate Current — pipeline state rebuilt from the persisted
   registry, not from client memory.
4. **Simulate.** SwarmTank: `Import from saved experiment` (the collider
   experiment) → pricing `$49 per month`, amount `49`, segment `small
   agencies`, industry `services`, value proposition, population `20`,
   seed → `Generate personas` → `Evaluate decisions` → notice
   "Evaluated and saved the SwarmTank run." → walk Simulate Complete.
5. **Pressure-Test.** SOMNIA: seed + count → `Generate SOMNIA
   nightmares` → `Run evaluation` → `Save SOMNIA evaluation` → walk
   Pressure-Test Complete.
6. **Refine.** Walk Refine button → Ask ANANSI view → `Enable Draft
   Exploration` → consent dialog → type a prompt in `ask-prompt`,
   revise it → `Review drafts` → fragments visible → `Stop & discard
   session` → "Draft history purged." → back to Projects → walk Refine
   still Available (never Complete — asserted explicitly, not assumed).
7. **Launch.** Walk Launch button → studio → `Generate launch copy` →
   four channel articles visible → `Download Launch Bundle (.zip)` →
   ZIP signature + `LAUNCH-MANIFEST.json` + `marketing/show-hn.md`
   entries → manual form: type `launch-material`, title "Launch
   materials", body, `Register artifact` → notice → walk Launch Complete
   → completion note visible.

Traffic: the walk legitimately hits the fake provider for the opportunity,
collider and spec generations. No zero-traffic assertion is made for the
walk as a whole; the Launch step itself adds no hits (proven in slice2
and re-proved here by comparing the count before and after the studio
section opens, same pattern as slice2 test 2).

### Test 2: backup / restore integrity

Fresh project → collide + Build concept → SwarmTank run (the slice2
test-2 path, shortened: no opportunity, no spec, no SOMNIA) → record the
pipeline stage statuses and the artifact type/title list → backup via
`call('jobs', …)` → download the backup artifact in chunks → landing
workspace → upload → preview → restore → open the restored project →
assert the identical stage statuses and artifact list → open a fresh
project in the restored workspace → assert Discover Current and an empty
artifact list (restored artifacts do not leak into other projects).
Pattern: the inline backup/restore flow in `swarmtank-slice1.spec.ts`
and the SOMNIA specs.

## 6. Closure documentation (§88)

- `docs/handoffs/M12-product-lifecycle-slice3-addendum.md` is the §88
  milestone report: features built, files added/modified, architecture
  decisions, database migrations (none), tests passed / failed / not run,
  security issues (none; credentials sweep clean), live-provider
  verification (none — all provider traffic in e2e is the local fake;
  zero live-model evidence is claimed), known limitations (the §2
  boundaries, the §61 remainder, the restart-leg scope, the
  first-run-stall disclosure from slice1 if it recurs), last verified
  commit, backup status, current runnable command, next milestone.
- `docs/BUILD_STATUS.md`: new top entry (Slice 3 delivered, Linux
  evidence, Windows rerun pending until the reviewer reports).
- `docs/REQUIREMENTS_MATRIX.md`: AN-012 row advanced to `verified` with
  "Milestone 12 complete" in its text and the preserved-gates sentence.
- `docs/RECOVERY.md` and `plans/PHASE-012.md`: resume point updated to
  the Slice 3 tip; the stale "Slice 2 rerun pending" tail already
  corrected in slice2 is not reintroduced.

## 7. Verification battery (sequential, on an idle machine)

1. `tsc --noEmit` — 0 errors.
2. `node scripts/contracts.mjs --check` — 0 drift (no contracts change
   expected).
3. `node scripts/validate-docs.mjs` — PASS.
4. `node scripts/cargo.mjs fmt -- --check` and strict Clippy — clean
   (no Rust files touched; run to confirm).
5. Rust workspace suite — 616 passed + 4 known live-Ollama failures
   expected (no Rust change; the 4 are environmental).
6. `npm run test:ui` — node total grows by the guided-walk tests.
7. Web build (`apps/web` Vite) — clean, then rebuilt before e2e.
8. New spec `product-lifecycle-slice3.spec.ts` — 2/2.
9. Full regression battery (slice1, slice2, slice3, ghost 1–3, somnia
   1–3, evaluations, integrated_workflow, opportunities,
   swarmtank-scenario, swarmtank-arena, swarmtank-slice1) — 28/28
   expected (26 + the 2 new walk tests).

## 8. Non-goals

- No source-package generation, code execution, sandbox, preview,
  deployment, publishing, payments, or domain work.
- No new server op, artifact type, schema change, or Rust change.
- No provider dispatch from the walk panel itself; no live provider
  traffic anywhere in this slice.
- No change to the Slice 2 bundle format, the navigator, or reviewer
  files. No merge to master, no force-push, no remote CI.
