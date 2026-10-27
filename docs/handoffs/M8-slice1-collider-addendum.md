# M8 Slice 1 addendum: text-first LEXI-COLLIDER workflow

Baseline: the independently reviewed M7 source-review
checkpoint `codex/review-m7-source-review` @
`08465172aab8fcc99ceb5af347145d3bd4545b91`
(ZIP SHA-256
`d1e47d4653421710cad390df77de485f4ae9f95a0589b47abff9ec3569258d9e`,
bundle SHA-256
`db1202e8dff6ace26a3be3721929712249fe7aa1c4068460af6256e83a54307b`,
verified in a scratch repo before import:
bundle verifies, scratch fetch lands exactly on
`08465172a`, `befc66f11` and `7607dc7c` confirmed
ancestors). Branch `muse/m8-slice1-collider` from
that exact tip; the original Muse delivery was not
restarted from. The reviewer's corrections at
`befc66f11` (coherent package/history/editor
selection, malformed-review guards,
inspection-only assessment labels, restore
validation) are preserved untouched. No preview
branch was merged, rewritten or promoted.

This assignment authorizes scheduling this bounded
M8 slice while M7's native source-review
acceptance, execution environment and production
gates remain open. It does not declare M7
complete or waive those gates.

## What was built

A complete, text-first collision workflow inside
an existing project (design:
`docs/handoffs/M8-slice1-collider-design.md`,
committed before coding):

- Two clearly labelled inputs, **Beam A** and
  **Beam B**, bounded typed/pasted text
  (1–8000 characters each, both required).
  Supplied text is private data and untrusted
  content: it is sent as segments with privacy
  `"private"` and an explicit instruction segment
  framing the beams as data, and it never
  overrides application policy. The panel states
  that nothing starts until Collide Ideas is
  pressed, and the browser's required-field
  validation blocks submission while either beam
  is empty — verified in the browser spec with a
  zero provider-hit count.
- Sensible defaults for the §24 advanced
  controls (creativity balanced, technical depth
  standard, practicality balanced, innovation
  distance moderate, preferred output concept,
  budget empty, Zero-Cost Lock on), explained in
  the panel as steering emphasis only — they do
  not guarantee novelty, feasibility or
  correctness. Unknown enum values are refused
  server-side; defaults are applied server-side.
- Explicit **Collide Ideas** runs on the existing
  M5 provider/routing/privacy/cost substrate
  (ops `collider-start/-poll/-cancel/-save` on the
  `ai_command` providers arm, mirroring the M6
  Slice 2 ideas run pattern). No new provider
  client or orchestration framework was
  introduced. Zero-Cost Lock, permits, budgets,
  disclosure and cancellation are preserved.
- One provider request produces all six
  structured sections — source understanding,
  conceptual primitives, cross-domain mappings,
  several proposed combinations or architectures,
  feasibility hypotheses with dependencies,
  unknown costs, risks and testability, and
  productization (concept brief, architecture,
  roadmap, test plan, prototype proposal). The UI
  shows honest run states only: generating, then
  validating (a real strict client-side parse
  before render). No six invented live stages or
  percentages are shown.
- Output is untrusted and validated before any
  successful presentation: the backend
  `parse_collider_output` enforces the bounded
  structure (deny-unknown-fields, per-field
  character caps, list count ranges, total raw
  output cap 262144 characters, maxOutputTokens
  8192) and a malformed or invalid result becomes
  a terminal `invalid`/`failed` run that saves
  nothing — never a partially saved experiment.
- Honesty labels throughout: every section is
  presented as model-generated hypotheses,
  analogies and assumptions, not researched
  facts. Nothing claims independent research,
  technical validation, novelty, patentability or
  market evidence. The fixed limitations are
  explicit in the result view.
- Inspection followed by explicit saving as an
  immutable, versioned experiment through the
  existing artifact registry: the saved body
  format is `anansi-collider-experiment` v1 and
  reuses the existing generic artifact type
  `experiment` (already in ARTIFACT_TYPES from
  §9 — no type-list change). It preserves the
  exact beams, resolved controls, result,
  provider/model provenance and the full
  disclosure record. Save is idempotent via the
  run claim/wait pattern; repeated saves return
  the same artifact. Reopening after restart
  reads the saved body, never a newer run.
- Cancellation, provider refusal/down states,
  project switching, stale responses and
  repeated saves follow the established bounded
  lifecycle patterns: a process-local run
  registry (TTL 600s, cap 32 per workspace,
  session-scoped keys), serialized polling, an
  epoch guard so delayed responses never
  overwrite a newly selected project's state, and
  confirmed cancellation cleanup (the cancel
  path waits for an in-flight poll to settle and
  invalidates its response, so a late "running"
  can never resurrect a cancelled run).
- Backup/fresh restore: the restore
  pre-validation `validate_original_experiment_bindings`
  gates ONLY bodies whose format is
  `anansi-collider-experiment` — generic manual
  `experiment` rows (including non-JSON bodies)
  restore untouched, a distinction the test
  suite caught and the implementation was fixed
  for (see below). Collider records validate
  their original bindings before any remapping,
  remap contained identities, preserve
  historical versions, and refuse invalid
  references before destination publication.
- Accessible 2D presentation only. No Three.js,
  no decorative computation animations.
- Explicitly deferred to later slices and shown
  only as a plain-text note, never as working
  controls: URL fetching, document ingestion,
  saved-project/memory selection, 3D
  visualization and approved Forge transfer.

## Substrate rules learned during verification

- The M5 stream substrate reports a routing
  refusal (Zero-Cost Lock against a paid model)
  as a terminal **failed run**, not as a start
  error — the same shape as a connect failure.
  The refusal happens before network: the test
  asserts the failed status names the cost
  constraint, the result is null, the save of
  that run is refused, and the provider hit
  count stays zero. This is the substrate's
  established lifecycle, not a weakened gate.
- The restore guard initially refused generic
  manual experiment rows whose bodies are not
  JSON. Fixed: only format-matching rows are
  gated; everything else restores untouched.
  The tampered-restore test carries the
  generic-manual-experiment control proving it.

## Verification (Linux, sequential, idle machine)

- Backend: `crates/core/tests/collider.rs` — 12
  tests (save/reopen/restart, required inputs
  and bounds before provider traffic,
  unconfigured/archived refusals, privacy and
  cost refusals with zero provider hits,
  malformed outputs saving nothing, provider-down
  as failure, cancellation cleanup, workspace
  isolation and stale polls, idempotent save,
  embedded instructions staying data, restart
  and fresh restore, tampered-restore refusal).
- Node: full `test:ui` suite including 9 new
  collider tests (run-controller staleness/cancel
  discipline mirroring idea-run; strict result
  and experiment-body parsers; panel bargain
  starting nothing on render; six-section view
  honesty labels; experiment view pins).
- Browser: `tests/e2e/collider.spec.ts` — no
  inference before submit, cancel saves nothing,
  collide renders all six sections, save,
  restart/reopen, backup/fresh restore under
  remapped identities. Forge revision,
  deliverable and source-review regressions
  preserved in the canonical battery.
- Full core 268/272 (only the 4 known
  environmental live-Ollama `semantic_workflow`
  failures; 258 at the Slice 4 era + 2 reviewer
  restore tests + 12 collider = 272); providers
  120/0 + API 24/0 = 144/0; node `test:ui` 73/73
  (64 prior + 9 collider); tsc clean; contracts
  `--check` clean; docs validation PASS (126
  Markdown files); strict Clippy (`-D warnings`)
  and `fmt --check` clean; API+web builds pass;
  browser canonical battery 13/13 (see
  BUILD_STATUS for the file list).
- Two failures found and fixed during
  verification, neither by weakening an
  assertion: (1) the API route allowlist in
  `crates/api/src/lib.rs` lacked the four
  collider paths, so the browser preflight
  blocked every collision while backend tests
  passed — fixed by adding the paths, and the
  panel now surfaces a failed start as a visible
  run error instead of staying silent; (2) the
  browser spec's result assertion matched the
  generic editor's raw JSON copy as well as the
  rendered section (strict-mode violation) —
  scoped to the combinations section. An
  initial 12-file canonical run omitted
  `app_checkpoint` and was superseded by the
  complete 13-file run above; only the complete
  run is cited as evidence.

## Limitations (unchanged by this slice)

- In-flight collider runs do not survive a
  restart, as with ideas and Forge runs: only
  saved experiments persist.
- Synthetic loopback providers and fake
  credentials only. This slice proves
  application behavior, not real-provider
  behavior; no live-provider verification is
  claimed.
- Linux evidence only. Windows verification of
  this slice is pending, as are M7 native
  desktop acceptance and preview promotion,
  the §32 execution sandbox, and the M5
  production gates. Nothing here claims M8 or
  ANANSI complete.

## Owner test guide

1. Open a project, scroll to LEXI-COLLIDER.
2. Press Collide Ideas with a beam empty: nothing
   happens and no provider is contacted.
3. Fill both beams, press Collide Ideas, watch
   the generating then validating states, then
   the six sections with the honesty banner.
4. Press Save collision as experiment, then open
   the experiment from the artifact list: exact
   beams, controls, provider and model are
   pinned and the record is immutable.
5. Restart the API and reopen the experiment;
   back up the workspace and restore into a
   fresh workspace — the experiment reopens
   under its new identity with content intact.
