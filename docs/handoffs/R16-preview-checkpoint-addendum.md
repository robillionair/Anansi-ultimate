# R16 integrated preview checkpoint — addendum (2026-10-21)

Delivered under the owner's three-part assignment of
2026-10-21 13:41 CEST (preview checkpoint; vault ladder
reconciliation; integrated application verification), on top
of the independently Windows-verified R16 baseline.

## Composition

- Baseline: R16 `a1b0c80724eb30497ecf0ccb69149a0499dc6ee7`
  (branch `muse/m6-r15-repair`).
- `muse/vault-ladder-reconciliation` (`76f5322`, `82f4eb3`,
  `0e0ab01`): the schema-ladder reconciliation — see
  `vault-ladder-reconciliation-addendum.md`. A production
  fix (vault snapshot verification/staging no longer
  migrates the live catalog it opens) plus test
  reconciliation (genuine legacy schema-8 crash matrix with
  its original expectations; explicit schema-10 matrix with
  artifact-survival assertions; canonical-test expectations
  updated only where Slice 1 legitimately changed the
  semantics, with the reasoning recorded in the tests).
- `codex/local-testing-r16` (from the R16 tip directly;
  R11's `f6cecd2` is an ancestor — nothing re-cherry-picked;
  the Tauri `artifact_command` capability fix is preserved):
  the vault branch fast-forwarded in, then
  - `e82a5a1` — PREVIEW.md rewritten for this checkpoint
    (separate data root `%LOCALAPPDATA%\ANANSI-Preview\r16`,
    Windows build/launch instructions, owner test guide) and
    the launcher retargeted to the r16 root/title;
  - `ed6b4e5` — new end-to-end spec
    `tests/e2e/integrated_workflow.spec.ts` (below);
  - `a7a6037` — production UI fix for a rubric-load race
    found by that verification (below);
  - `504033c` — evaluations e2e determinism fix (below).

## Integrated workflow evidence (new spec)

`tests/e2e/integrated_workflow.spec.ts` runs one browser
session against the real API with a synthetic loopback
provider: create project and context memory → generate
ideas with the context selected → save the idea set as an
artifact → evaluate two concepts, one dimension scored →
create a workspace backup in the Backups screen → download
it from Jobs & export → import it on the Backups screen →
restore it as a NEW workspace → restart the API → assert
over the API that the restored evaluation cites the RESTORED
idea-set and rubric artifact ids (and its `artifact://`
source references point at them), then reopen the restored
workspace in the UI and see the evaluation rendered against
the restored source. Passed on Linux (5.1s); the original
workspace's evaluation still resolves to the originals.

## Defects found and fixed during integration verification

1. **Rubric edits lost during rubric load (production UI).**
   `EvaluationPanel.chooseRubric` swaps the selection
   immediately but replaces the edited document only when the
   artifact read resolves; an edit made in that window (pick
   a saved rubric, immediately change a weight) landed on the
   previous document and was silently overwritten by the
   arriving read, so "Save rubric as new version" stored the
   un-edited weights under a success notice — and an
   evaluation saved in the window cited the new rubric
   version with the old snapshot/summary. Observed directly:
   one run saved an evaluation citing rubric v2 whose
   embedded snapshot and weighted summary (4.5) were the v1
   values; another run of the same flow produced the correct
   v2 snapshot (4.75). Fix (`a7a6037`): the rubric form and
   both save buttons are disabled while a rubric read is in
   flight (with a loading note), and a sequence guard ensures
   a superseded read never applies. No backend change; the
   core's snapshot semantics were verified correct throughout
   (it embeds exactly the version it is cited).
2. **Evaluations e2e assumed artifact list order (test).**
   `artifact_list` orders by artifact id, so the two
   same-named evaluation buttons swap positions between
   runs; the spec's final block opened `.first()` and
   asserted the v1 snapshot, failing whenever the v2
   evaluation sorted first (its header also contains
   "(version 1)" — the source version — which masked the
   real mismatch for one assertion). Fix (`504033c`): open
   both evaluations, await each cited rubric version, and
   check each against its own snapshot; the cited set must
   be exactly {1, 2}. After fix 1+2 the spec passes
   repeatedly (6/6 consecutive runs, plus the full subset).

## Linux verification on the final tree (`504033c`)

- anansi-core full suite: **196/200**. The only failures
  are the 4 `semantic_workflow` tests that require a live
  Ollama server (absent on this machine; environmental, the
  same baseline failures recorded since M5). The 2
  `vault_increment2` failures are closed (suite 9/9).
- Focused suites: `backup_artifacts` 5/5, `backup_restore`
  7/7, core lib 34/34; vault suites: certification 11/11,
  runtime 2/2, credential store 1/1, search boundary 3/3,
  search integration 1/1, search invalidation 2/2.
- anansi-providers + anansi-api (all test binaries):
  **144 passed, 0 failed**.
- Node: contracts + UI tests **47/47**; `tsc --noEmit`
  clean; contracts check clean; web build succeeds.
- Browser e2e (`playwright.local.config.ts`): **8/8** —
  `integrated_workflow`, `app_checkpoint`, `artifacts`,
  `ideas`, `evaluations`, `providers`, `vault`, `backups`.
- Clippy `-D warnings` clean (core lib, `--all-targets`);
  `fmt --check` clean; docs validation passes.
- Environment note: `pnpm` is not on this machine's PATH in
  this session, so the node-side checks were run through
  their underlying `node`/`node_modules/.bin` commands (the
  same commands the package scripts wrap).

## Status boundaries

- **Windows verification of THIS checkpoint is pending.**
  The owner's Windows pass covers R16 (`a1b0c80`) and,
  earlier, R15's suites; the vault reconciliation, the
  preview packaging, the rubric-load fix and the integrated
  spec have Linux evidence only. The Windows build/launch
  steps in PREVIEW.md were not executed on the preparing
  machine (Linux) and are not claimed as executed.
- **Production gates unchanged and open:** M5 pending
  checks (Windows native acceptance incl. real CredUI and
  OS keyring behaviour, live-provider verification, G1
  external audit, remote CI) — none are closed by this
  checkpoint. No pushes, no master merge, no remote CI.
- Slice 4 (Opportunity Engine) remains owner-gated; nothing
  here starts it.
