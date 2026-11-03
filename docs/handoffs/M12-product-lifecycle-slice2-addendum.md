# M12 Product Lifecycle Slice 2 — Addendum: Launch Studio Marketing Copy & Asset Bundler

## Baseline

Branch `muse/m12-product-lifecycle-slice2` from the independently
Windows-verified Slice 1 review tip `27c5754ac`. That tip carries the user's
README update `59ea5bfab`, the reviewer's NTFS import qualification
`3ff95281f`, and the Slice 1 review record. The Slice 2 assignment's baseline
was initially unavailable (the first uploaded file was byte-identical to the
Slice 1 delivery, and the review tip was absent from the repo); work was HELD
until the actual Windows-verified package arrived, was hash-verified, and
branched from. The hold mattered: the review branch contained a real fix (the
extensionless `PipelineNavigator` import) that building from the author's own
tip would have dropped.

## What was built (§61, §85, as bounded by the assignment)

- `packages/ui/src/launchCopy.ts`: deterministic rule-based generator for the
  four assigned channels (Show HN, Product Hunt, X thread, release notes)
  from parsed concept / specification / SwarmTank run / SOMNIA evaluation
  evidence. Generation requires a concept plus a simulation or an evaluation;
  otherwise the studio lists exactly what is missing. Numbers come only from
  the selected evidence and are labeled simulated where simulated. The
  local-first guarantee appears only when the specification text itself
  supports it; otherwise the architecture post describes the static prototype
  and states its status. Every release-notes generation carries the status
  line "Prototype for review; not executed, not deployed." No dates, clocks,
  or randomness: the same selection yields byte-identical channel texts.
- `packages/ui/src/launchBundle.ts`: dependency-free stored-ZIP builder
  (method 0, fixed 1980-01-01 DOS timestamps, UTF-8 filenames, CRC32 per
  entry, entries sorted by path) plus the `LAUNCH-MANIFEST.json` builder
  (format `anansi-launch-bundle` v1, generator label, channel list, included
  artifacts with id / version / content-hash prefix). The ZIP format was
  validated independently of the author's own parser: system `unzip -t`
  passes its CRC checks and Python's `zipfile` reads the archive.
- `packages/ui/src/LaunchStudio.tsx`: the studio component. Artifact picker
  (one checkbox per launch-relevant record, all checked by default), Generate
  button gated on the evidence rule, preview cards per channel with
  copy-to-clipboard buttons and Preview/Source tabs (Preview is
  paragraph-split plain text; the Source tab is authoritative — no full
  markdown renderer is built), a gated provider-draft section (exact prompt
  preview + estimated input tokens + explicit "Approve and send to Ask
  ANANSI" button; the draft never overwrites the deterministic copy and
  enters the bundle only when the user checks the opt-in box), and a
  "Download Launch Bundle (.zip)" action (fixed filename
  `anansi-launch-bundle.zip`).
- `packages/ui/src/Projects.tsx` + `packages/ui/src/PipelineNavigator.tsx`:
  the Launch row is now a working button ("Jump to Launch Studio" while
  Awaiting, "Open Launch Studio" otherwise) via a new `onSelectLaunch`
  callback, and the Launch Studio section (anchor
  `#project-launch-heading`) mounts as the last section of the project view
  when the stage is selected. The Slice 1 "Launch Studio arrives in Slice 2."
  note is removed.
- Bundle contents, and nothing else: the four channel markdown files under
  `marketing/`, `swarmtank-summary.md` (when a run is selected, with the full
  objection matrix), `somnia-pressure-test-notes.md` (when an evaluation is
  selected, with review-only proposals labeled as not applied),
  `artifacts/<type>-<artifactId>-v<version>.json` (verbatim bodies of the
  checked artifacts), an optional `marketing/provider-draft.md`, and
  `LAUNCH-MANIFEST.json`. Source-package files are NOT re-expanded (the Forge
  export already serves that); the deliverable body's pins reference them.

## Scope notes and stated limitations

- §61 is NOT complete: landing-page copy, onboarding, documentation, SEO
  plan, demo script, sales outline, deployment checklist, and post-launch
  experiments remain deferred. Slice 2 delivers the assignment's four
  channels only.
- No connected web research was performed (§61 permits it only if
  authorized); no competitor claims appear in the copy.
- No domain registration, payments, live deployment, or publishing is built
  or triggered.
- The bundle is download-only and is NOT registered as an artifact, so
  Launch Complete still requires manual `launch-material` registration,
  exactly as in Slice 1.
- The approval panel cannot display the live configured provider budget
  because no client method exists to read it; adding one would be a new
  server op, out of scope. The panel shows the exact prompt and the estimated
  input tokens, and the existing Ask dispatch path enforces the configured
  budget server-side. The assignment's "§74" citation for budget approval
  points at MASTER_SPEC §74 (Data Privacy) in the actual document; the
  requirement's intent is implemented as described, and §74 itself is
  respected (prompts are built only from artifacts the user selected).
- The Forge export (M7 Slice 3) is a server-side deterministic USTAR archive;
  the launch bundle mirrors its pattern (deterministic bytes, version/hash
  manifest, portable relative paths, zero unstated files) in a different
  container — a client-side stored ZIP — because every input is already
  readable by the client and no new server op is wanted. The formats differ;
  the guarantees rhyme.

## Genuine issues found during the slice

1. E2E assertion corrected, not a product defect: the first run of the Slice
   2 spec failed only on `fakeHits() === 0`, because the collider step
   legitimately hits the fake provider by design. The spec now captures the
   fake-hit count before opening the studio and asserts no additional hits
   from the studio path through download — which is the claim that matters.
2. Staleness fix (commit `7e7817eb3`): unchecking an artifact after
   generating left the previous copy and bundle available for download even
   though the selection had changed. Selection changes now invalidate the
   generated copy and provider draft, forcing a regenerate.
3. Contract-shape corrections during implementation: the SwarmTank input and
   decision contract types carry optional fields (`pricing`, `segment`,
   `industry`, `valueProposition`, `features`, `priceAmount`, optional
   `keyObjection`) that the first draft of the generator's evidence types
   declared as required. The evidence types were loosened to match, and the
   objection filter was fixed to drop `undefined` as well as `null` (the
   first draft's `!== null` filter would have admitted `undefined` as an
   objection string).

## Tests and verification (Linux)

- Rust workspace: 616 passed + 4 known live-Ollama env failures
  (`semantic_workflow`, Ollama stopped on this VM — identical to the Slice 1
  baseline; no Rust files touched by this slice).
- Strict Clippy (`-D warnings`) clean; `cargo fmt --check` clean.
- Node UI suite: 220/220 (198 baseline + 20 new launch-studio tests + 2 new
  pipeline-navigator tests).
- `tsc --noEmit`: 0 errors. `node scripts/contracts.mjs --check`: 0 drift.
  `node scripts/validate-docs.mjs`: PASS (242 files at feature time).
- Web build: clean (Vite, `dist/assets/index-D7o7tYUt.js`).
- E2E `product-lifecycle-slice2.spec.ts`: 2/2 (fresh-project jump to an
  empty studio; genuine collide → Build concept → SwarmTank evaluate →
  studio generate → bundle download with PK signature and manifest /
  release-notes entry names; zero additional fake provider hits from the
  studio path).
- Regression battery: 26/26 in a single sequential run (lifecycle slice1
  2/2, lifecycle slice2 2/2, ghost-prompt slice1–3 2/2 each, somnia slice1–3
  2/2 each, evaluations 1/1, integrated_workflow 1/1, opportunities 1/1,
  swarmtank-scenario 3/3, swarmtank-arena 3/3, swarmtank-slice1 1/1).
- Zero provider traffic on the deterministic path (see the e2e measurement
  above). The provider-draft path is not exercised end to end (it would
  require a live provider); it is covered by the dispatch seam unit test
  with a recording fake.

## Files added / modified

Added: `packages/ui/src/launchCopy.ts`, `packages/ui/src/launchBundle.ts`,
`packages/ui/src/LaunchStudio.tsx`, `packages/ui/tests/launch-studio.test.tsx`,
`tests/e2e/product-lifecycle-slice2.spec.ts`,
`docs/handoffs/M12-product-lifecycle-slice2-design.md`, this addendum.
Modified: `packages/ui/src/Projects.tsx`,
`packages/ui/src/PipelineNavigator.tsx`,
`packages/ui/tests/pipeline-navigator.test.tsx`,
`tests/e2e/product-lifecycle-slice1.spec.ts` (Launch row now works),
`package.json` (`test:ui` list), `docs/BUILD_STATUS.md`,
`docs/REQUIREMENTS_MATRIX.md` (AN-012 stays `in progress`),
`docs/RECOVERY.md`, `plans/PHASE-012-product-lifecycle.md`.
No server op, no new artifact type, no schema change, no Rust change.

## Next

- Independent Windows rerun of this slice (checklist in the delivery
  MANIFEST), including desktop-shell confirmation of the studio section.
- Slice 3 (§85 guided walk) not commissioned.
- AN-012 stays `in progress` until the Launch Studio scope (and the M7
  production gates) are closed.
