# M12 Product Lifecycle Slice 3 — Addendum: Integrated Lifecycle Guided Walk & Milestone 12 Closure (§88 milestone report)

Branch `muse/m12-product-lifecycle-slice3`, from Windows-verified Slice 2
review tip `30f1ee91dfa9f88bbc4ae1f5c8bebb3cfdc5b74c` (review branch
`codex/review-m12-product-lifecycle-slice2`, single docs-only commit over
delivery tip `77583b7c0`).

## Baseline hold, resolved the same way as Slice 2

The Slice 3 assignment arrived asserting Slice 2 Windows certification at
`30f1ee91…`, but the first upload contained only the review ZIP's
`.sha256` sidecar (expected hash `10f84ebe…` for
`anansi-m12-product-lifecycle-slice2-windows-verified-2026-11-03.zip`) —
no ZIP, and the tip did not exist in the repo. Work was HELD and reported
with two paths (upload the package, or authorize building from clean tip
`77583b7c0`). The user then uploaded the actual review package. It was
verified before branching: ZIP hash equals the sidecar (`10f84ebe…`),
bundle hash equals the MANIFEST (`622e8acc…`), the bundle head is exactly
`30f1ee91d`, ancestry holds (`77583b7c0` is its parent), and the diff
versus `77583b7c0` is docs-only (BUILD_STATUS + REQUIREMENTS_MATRIX +
the review report). No code changes, no reviewer fix this round — unlike
Slice 1, where the hold preserved a real NTFS import fix.

Reviewer wording NOT adopted as claims (same discipline as Slices 1–2):
the Slice 2 report names a "Generate AI Polish" button (implementation:
"Approve and send to Ask ANANSI") and bundle files
`concept.json` / `specification.json` / `swarmtank-simulation.json` /
`somnia-evaluation.json` / `MARKETING-COPY.md` (implementation:
`marketing/show-hn.md`, `marketing/product-hunt.md`,
`marketing/x-thread.md`, `marketing/release-notes.md`,
`swarmtank-summary.md`, `somnia-pressure-test-notes.md`,
`artifacts/<type>-<id>-v<version>.json`, optional
`marketing/provider-draft.md`, `LAUNCH-MANIFEST.json`). The verification
NUMBERS match the delivery and are adopted; the misdescriptions are not.

## §88 report — Features built

- **Guided Walk panel** (`packages/ui/src/GuidedWalk.tsx` +
  `packages/ui/src/guidedWalk.ts`): an ordered, plain-language checklist
  over the pipeline state, mounted in the project view immediately after
  the PipelineNavigator. Six steps — Discover, Define, Simulate,
  Pressure-Test, Refine, Launch — each with its status, one-line
  instruction, and a button jumping to the tool that performs it
  (anchored sections for in-view tools; the existing Ask and Launch
  callbacks for Refine and Launch). A "Continue walk" control jumps to
  the first non-Complete step among the five completable steps; when all
  five are Complete it is replaced by the completion note.
- **Define spans Invent + Build.** A concept alone is not a defined
  product: the step completes only when a concept AND a specification
  are both registered. Its target follows the state: the collider while
  no concept exists, the artifacts section once one does. The
  spec-without-concept edge (manual registration) reads Awaiting with
  the build requirement sentence, honestly incomplete.
- **Refine never completes, by design.** GHOST-PROMPT sessions are
  in-memory only and are never registered, exactly like the pipeline's
  refine stage. Refine is deliberately excluded from Continue targeting:
  including it would pin Continue on Refine forever and block Launch.
  The completion note says so plainly: "Refine stays available as an
  ongoing practice; it is not a deliverable and never reads Complete."
- **The complete §85 walk as the assignment bounds it, executed end to
  end in the browser** (`tests/e2e/product-lifecycle-slice3.spec.ts`,
  test 1): opportunity set → collider concept → Forge specification
  draft → mid-walk client restart → 20-persona SwarmTank run → SOMNIA
  nightmare evaluation → GHOST-PROMPT session revised and wiped →
  Launch Studio four-channel copy → bundle download → manual
  `launch-material` registration → walk completion note.
- **State recovery and persistence integrity (§62), verified not
  asserted.** Test 1 restarts the app client mid-walk (full page reload
  + re-pair) and re-asserts the pipeline state. Test 2 backs up a walked
  project, restores it into a fresh workspace, and asserts identical
  walk statuses and the identical artifact type/title list, then opens a
  fresh project in the restored workspace and asserts an empty pipeline
  (restored artifacts do not leak into other projects). No new app code
  was needed for recovery: pipeline state is purely derived from the
  artifact registry (Slice 1 design), so restart/restore reconstruction
  is a property the tests prove, not a feature added.
- **Shared harness prompt routing** (`tests/e2e/collider-three-helpers.ts`):
  the fake provider now routes on the server-side instruction strings
  (verified in Rust source, all distinct): `opportunity discovery
  engine` → opportunity JSON, `product architect` → Forge spec JSON,
  otherwise the existing collider fixture. The collider instruction
  ("You are ANANSI's LEXI-COLLIDER engine.") contains neither marker,
  so existing collider specs are unaffected — proved by the regression
  battery below.

## §88 report — Files added

- `packages/ui/src/guidedWalk.ts` (walk derivation)
- `packages/ui/src/GuidedWalk.tsx` (walk panel)
- `packages/ui/tests/guided-walk.test.tsx` (8 tests)
- `tests/e2e/product-lifecycle-slice3.spec.ts` (2 tests)
- `docs/handoffs/M12-product-lifecycle-slice3-design.md`
- `docs/handoffs/M12-product-lifecycle-slice3-addendum.md` (this file)

## §88 report — Files modified

- `packages/ui/src/Projects.tsx` (mount the walk panel after the
  navigator; explicit `.tsx` import)
- `packages/ui/src/lifecycle.ts` (export the engine's requirement
  sentence for the Define Awaiting case — see below)
- `package.json` (`test:ui` gains the guided-walk test file)
- `tests/e2e/collider-three-helpers.ts` (prompt-routed fake)
- `docs/BUILD_STATUS.md`, `docs/REQUIREMENTS_MATRIX.md` (AN-012 →
  `verified`, "Milestone 12 complete"), `docs/RECOVERY.md`,
  `plans/PHASE-012.md`

## §88 report — Architecture decisions

- The walk derives from `derivePipeline`, never from its own state.
  There is exactly one source of pipeline truth (the artifact registry),
  which is why restart/restore reconstruction holds without new
  persistence code.
- The walk panel reuses the navigator's navigation shapes (anchors for
  in-view tools, optional callbacks for Ask and Launch) and its
  defensive rendering (callback buttons render only when provided).
- The Define Awaiting sentence comes from the engine
  (`missingSentence('build')`, newly exported) rather than from the
  Build stage's state: when Build itself is Complete (spec present
  without concept), the stage's own `missing` is null, and the first
  unit-test run caught the row rendering with no requirement sentence.
  The fix keeps one source of truth for the sentence instead of
  duplicating the string.
- The e2e walk uses the harness's pre-created "Harbor App" project
  (empty at session start) rather than creating a project through the
  UI, because `pairAndOpen`/`openProject` address that project by name
  and the restart leg re-enters through them.
- The restart leg restarts the app client (full page reload + re-pair
  against the same running API), not the API process; the harness
  exposes no server-restart helper and none was added. Disk-level
  persistence is proved by the backup/restore leg instead. Both limits
  are stated here, not hidden.

## §88 report — Database migrations

None. No schema change, no new artifact type, no new server op, no Rust
change, no contracts change.

## §88 report — Tests passed

- Rust workspace **616 passed + 4 known live-Ollama failures** (no Rust
  files touched; the 4 are the pre-existing `semantic_workflow`
  environment failures, identical to the Slice 1/2 baselines), strict
  Clippy clean, fmt clean. Full-suite accounting, stated exactly: the
  concurrent full run (e2e battery running alongside it on this
  2-CPU machine) showed 615 passed + 5 failed — the 4 known failures
  plus `slow_but_valid_fragmented_record_succeeds` (`url_fetch`, "Fetching
  the URL timed out"). That test is timing-sensitive and unrelated to
  this slice (no Rust touched, no URL code touched); re-run in
  isolation on an idle machine it passed 21/21, restoring the 616 + 4
  baseline. The concurrent run was a harness mistake, not a product
  regression, and is recorded here rather than smoothed over.
- Node **228/228** (220 + 8 new guided-walk tests).
- tsc 0, contracts `--check` exit 0 (zero drift), docs validation PASS
  (245 files at design commit; closure count in the delivery MANIFEST),
  web build clean.
- E2E `product-lifecycle-slice3.spec.ts` **2/2**: the complete walk
  (15.2s) and backup/restore integrity (10.8s).
- Regression battery **28/28**: lifecycle slices 1–3 (2/2 each),
  GHOST-PROMPT slices 1–3 (2/2 each), SOMNIA slices 1–3 (2/2 each),
  evaluations 1/1, integrated workflow 1/1, opportunities 1/1,
  SwarmTank scenario 3/3, arena 3/3, slice1 1/1.

## §88 report — Tests failed

- The 4 pre-existing live-Ollama `semantic_workflow` environment
  failures (embedding model absent on this VM), identical to baseline.
  Plus one load-induced failure in the concurrent full run
  (`url_fetch` timeout, see "Tests passed") that passed 21/21 in
  isolation — recorded, not hidden.
  No other failures.
- In-slice e2e failures, all fixed and disclosed (not hidden):
  1. Both walk tests first stalled on a disabled "Generate personas":
     the spec had dropped the "Import experiment" click after selecting
     the experiment (the slice2 pattern) — selecting alone does not
     fill the description. Fixed by restoring the click.
  2. The restart checkpoint first read a stale empty artifact list:
     the Artifacts heading renders before the list reloads, so the walk
     briefly shows a fresh project. Fixed with an explicit wait for a
     known artifact row, and the remaining post-save walk assertions
     were converted to polling expectations (panel notices fire before
     the parent's artifact reload lands — the same race class, now
     handled uniformly).
  3. The SOMNIA leg first stalled on a disabled "Run evaluation": the
     generated nightmare set always contains a sealed-record scenario,
     and the validator refuses sealed records as replay input. The
     spec now mirrors the somnia spec's flow — Preview, Exclude sealed
     case, Preview again — which is the privacy boundary working as
     designed, not a workaround.
  4. A strict-mode violation on the SOMNIA save assertion: the notice
     text and the renamed Save button share the string. Fixed by
     asserting the notice's exact text (with its trailing period).
  5. One unit-test expectation was wrong, not the code: the
     spec-without-concept case continues at Discover (the first
     incomplete completable step), not at Define. Fixed in the test.

## §88 report — Tests not run

- No live-provider verification (see below); no Windows rerun yet
  (checklist in the delivery MANIFEST); no native Tauri desktop run
  (M7 gate, still open); no production certification.

## §88 report — Security issues

None. Credentials sweep clean over the diff (no matches). No secrets,
no provider credentials, no sealed-memory transmission anywhere in the
slice: the walk's opportunity step sends only the user-supplied problem
source to the local fake provider; the SOMNIA leg's sealed-case
exclusion is the privacy boundary operating, and the sealed scenario is
never replayed or saved.

## §88 report — Live-provider verification

None, and none claimed. All provider traffic in the walk e2e is the
local fake server (opportunity, collider, spec generations). Zero live
model calls, zero paid inference. The Launch step itself adds no fake
hits (asserted as a delta, the slice2 pattern).

## §88 report — Known limitations (preserved, not waived)

- **§85 is 17 steps; the walk covers 6.** Repository creation,
  application-code generation, sandbox test runs, prototype preview,
  revised-prototype requests, and source/documentation export (§85
  steps 7–10, 13, 16) touch M7 source-package generation and the §32
  execution sandbox and are NOT built here. The walk's Define step
  covers concept + specification draft only — no source package, no
  code execution, no deployment.
- **§61 remainder deferred** (landing page, onboarding, SEO, demo
  script, sales outline, deployment checklist, experiments) — unchanged
  from Slice 2.
- **Launch Complete still requires manual `launch-material`
  registration** (Slice 2's bundle is download-only). The walk teaches
  that step; no silent auto-registration was added. Whether the bundle
  should one day self-register remains an open owner decision.
- **Assignment wording notes** (recorded, not silently corrected):
  step 4's "nightmare artifact" is a `somnia-evaluation` artifact (no
  `nightmare` type exists); step 1's "opportunity artifact" registers
  as `idea` (Slice 1 design). The Slice 1 MATRIX sentence calling the
  engine "6-stage" is reviewer wording from the Slice 1 review edit and
  was left untouched; the implementation is 7 stages.
- **First-run stall disclosure (Slice 1, still open):** the lifecycle
  spec's first two runs once stalled on a stale pre-simulation state
  with no established root cause. This slice's polling-expectation
  change hardens the assertions against the reload race class but is
  not claimed as a fix for that stall.
- **Preserved open gates** (none closed by this slice): M7 native
  desktop acceptance + preview promotion; §32 execution sandbox; M5
  production gates (incl. live-provider verification); Windows
  verification of M8 Slice 4; Windows rerun of this slice.

## §88 report — Last verified commit

Branch `muse/m12-product-lifecycle-slice3`, tip recorded in the delivery
MANIFEST (this addendum's closure commit). Baseline: Windows-verified
Slice 2 review head `30f1ee91dfa9f88bbc4ae1f5c8bebb3cfdc5b74c`.

## §88 report — Backup status

Backup/restore behavior itself is unchanged by this slice. The walk's
test 2 exercises the existing backup/restore path end to end (backup →
chunked download → landing workspace → upload → preview → restore)
and proves pipeline state and project binding survive it. GHOST-PROMPT
sessions remain in-memory only and are never backed up, by design.

## §88 report — Current runnable command

Web app: the repo's standard Vite preview (`apps/web` build + preview
on 127.0.0.1:1420 under the application-test lease in e2e, as in prior
slices). API: `target/debug/anansi-api` built via `node
scripts/cargo.mjs build --locked -p anansi-api`. UI tests: `npm run
test:ui`. Rust: `node scripts/cargo.mjs test --locked --workspace
--no-fail-fast`.

## §88 report — Next milestone

None commissioned. **Milestone 12 is complete** in substance on Linux
(Slices 1–3 delivered; Slice 3 awaits its own independent Windows
rerun, checklist in the delivery MANIFEST). AN-012 is `verified` with
"Milestone 12 complete" in its text, per the M10/M11 precedent (the
validator has no `complete` status); the preserved gates above stay
open and are not closed by that status.
