# SOMNIA compaction integration — design + acceptance criteria (Checkpoint 2)

## §0 Baseline, branch, scope

- **Baseline (verified):** Antigravity's portable Windows preview,
  `codex/windows-preview-integration` at
  `4e4e970af8ad0c406cfae35fc3da55f99662b174`, supplied as
  `anansi-windows-preview-muse-base-2026-11-09.zip`
  (SHA-256 `5d1ebe49a2c438301e98279f40a5249c8ab69d948f0b4713ea893e983cbcf9ca`;
  bundle SHA-256
  `3e281b132f065c81afea0aef8d85363fabcdb3c558dbae17084f54e95de90c4d`).
  Fetched as `preview/windows-preview-integration`; tip verified,
  `bb1cc6e6d` (SwarmTank v2 tip, the Checkpoint 1 baseline) is an
  ancestor.
- **Branch:** `muse/somnia-compaction-integration`, created from
  `4e4e970a`. Merge commit `6b9cd2c10` brings in the five Checkpoint 1
  commits unchanged (design `194cedf0f`, library `c4590cb78`,
  benchmark `a3a908551`, clippy fix `81893ac37`, handoff `b0febf9e5`),
  so their hashes stay exactly as delivered.
- **Preview contents adopted as-is (not rebuilt):** the M13 PDF
  integration lineage (`79a332bb1`), LARP scene 09
  (`LarpSomnia.tsx` + `SomniaFilamentField.tsx`, dock/keyboard `9`),
  `SomniaModelCompare.tsx` (review-then-run two-prompt comparison),
  the OpenRouter reasoning fix (`disable_open_router_reasoning`),
  and the client-side cancel in model comparison. None of these is
  modified except where the integration explicitly extends them.
- **This slice (Checkpoint 2):** wire the actual SOMNIA compaction
  flow through the application — import → candidate → review →
  approve → compare → save — reusing the existing provider gates
  and LARP scene 09, prove it end to end with a synthetic provider,
  then rerun the frozen benchmark with an authorized live provider.
- **Out of scope:** changing the compactor library semantics
  (Checkpoint 1 rules are frozen), changing the offline SOMNIA
  evaluator, automatic prompt generation from consolidation
  proposals (§47 proposals stay inert and manual), OCR or further
  document formats, any global-status change (AN-015 stays
  `verified`; no new requirement row until Windows review).

## §1 The flow, end to end

1. **Import.** The user supplies compaction records in Format A
   (structured JSON, `anansi-somnia-compaction-input` v1) or Format B
   (text lines), or loads one frozen benchmark case. Import is
   synthetic or explicitly approved records only. The import op is
   pure: it parses, quarantines damaged records with reasons, and
   returns the parsed records plus quarantine — no provider call, no
   persistence.
2. **Prepare.** The server runs the Checkpoint 1 compactor
   (`compact_input`) and returns the complete candidate:
   retained sections (trusted / unverified / conflicts / deletions /
   uncertainties), excluded records with reasons, quarantined
   records with reasons, stats, `input_hash`, `content_hash`.
   Prepare is pure: no provider call, no persistence.
3. **Review.** The UI shows the candidate exactly as prepared:
   retained records by section, excluded records with their reason
   and detail, quarantined records with their reason. Nothing is
   editable in place.
4. **Approve.** Approval binds to the exact reviewed inputs: the
   approval carries `input_hash` + `content_hash`. Any edit to the
   imported records clears the approval in the UI (same pattern as
   `SomniaModelCompare.change()` clearing `reviewed`), and the
   server re-runs compaction at start time and refuses unless the
   recomputed candidate hash matches the approved one. An edit
   therefore always requires a fresh review.
5. **Compare.** A server-side background run dispatches both
   histories — original (all records) and compacted (retained
   records only) — to the same configured model, one pair per
   benchmark case (or one pair for a manual import). The run is
   pollable and cancellable. Every dispatch goes through the
   existing provider gates (Zero-Cost Lock, model-mismatch refusal,
   cloud requires explicit authorization). A failed or mismatched
   dispatch never produces a completed pair; it is recorded as a
   failed pair and the run continues with the remaining pairs.
6. **Save.** The user explicitly saves the complete experiment as a
   new artifact type (see §4). Saving revalidates the whole
   experiment fail-closed on the server, exactly like
   `somnia.rs::save_op` recomputes the evaluation doc. A saved
   experiment reopens with no provider call.

## §2 Deterministic prompt rendering and scoring

- **Rendering (identical for both arms except the record set).**
  For one benchmark case, the prompt is exactly:
  ```
  History:
  [<id>] <key> (<source>, rev <n>): <text>
  ...

  Question: <question text>

  Answer with the exact value from the history, or UNKNOWN if the history does not state it.
  ```
  The original arm renders all case records; the compacted arm
  renders only the records the candidate retained, in the same
  canonical order. The reviewed prompts shown in the UI are
  exactly the strings dispatched — no hidden additions.
- **Scoring (exact-match, per pair).** A reply counts as an
  abstention when, trimmed, it is empty, or equals `UNKNOWN`
  case-insensitively, or starts with `UNKNOWN` followed by a space
  or punctuation. Otherwise it is compared exactly (trimmed)
  against the expected answer.
  - Expected answer is a string: the reply (trimmed) equals it →
    correct. A reply of `UNKNOWN` (or empty) → abstention, counted
    separately, not correct. Any other text → incorrect.
  - Expected answer is `null` (missing-fact question): the correct
    behaviour is to abstain. `UNKNOWN`/empty → correct-abstention.
    Any concrete answer → incorrect.
  - Provider error, model mismatch, or an unparseable dispatch →
    failed pair: recorded with the error, never scored, run
    continues.
- **Honesty.** Library-level "answer accuracy" from Checkpoint 1
  was oracle retention coverage, not model accuracy. Model accuracy
  exists only after a live run, per case, per arm, per category,
  with every case in the denominator. A synthetic-provider run
  proves the plumbing only; its accuracy numbers are labelled
  synthetic and are never presented as model accuracy.

## §3 Server design

- **New module** `crates/core/src/somnia_compaction_run.rs`
  (the evaluator in `somnia.rs` is untouched). New ops in
  `somnia_command`, following the SwarmTank AI start/poll/cancel
  precedent:
  - `somnia-compaction-prepare` — input records → candidate
    (pure, ephemeral, no provider call).
  - `somnia-compaction-start` — validates the approved
    `input_hash`/`content_hash` by recomputing the candidate,
    resolves the model via `configured_chat_models` (local-first
    default), applies the up-front Zero-Cost Lock check (cloud
    model without an explicit permit refuses before any call;
    local-only with a cloud model refuses likewise), builds the
    pair list, spawns a background thread, returns `runId`.
  - `somnia-compaction-poll` — live progress: pairs done/total,
    calls made, tokens used, per-pair results so far, status.
  - `somnia-compaction-cancel` — sets a shared cancellation flag;
    the runner checks it before every dispatch and between pairs.
  - `somnia-compaction-save` — validates the complete experiment
    doc fail-closed and registers the artifact.
- **Process-local run registry** (mirroring `swarmtank_ai.rs`):
  600 s TTL, 32 runs per workspace, keyed by
  `(workspace, project, runId)`. Poll/cancel from a different
  project return not-found, never another project's run.
- **Caps (server-side, enforced before and during the run):**
  cases per run ≤ 40, pairs per run ≤ 50 (the frozen benchmark is
  40 cases with 50 questions), one pair = two dispatches, max
  calls 100, `maxOutputTokens` 384, `temperature` 0,
  `disable_open_router_reasoning` for OpenRouter, max total
  tokens 200,000. Budget is checked before every dispatch;
  exhaustion ends the run in terminal status
  `budget-exhausted`. A cancelled run ends `cancelled` with its
  completed pairs preserved in the poll response.
- **Restart behaviour (honest, documented):** the registry is
  process-local. After an API restart an in-flight run polls as
  not-found — it is never silently resumed and never
  double-dispatched. Only explicitly saved experiments survive a
  restart, as artifacts, and reopen with no provider call.
- **Routing:** the new ops go in all four lists, as the SwarmTank
  AI ops required: API path allowlist, `somnia_command`
  dispatcher, `ai_command` allowlist, and the persisted-provider-
  config lazy-load list in `ai_command`.

## §4 Saved experiment (new artifact type)

- **Type:** `somnia-compaction-experiment` — the 23rd entry in
  `ARTIFACT_TYPES` (after `swarmtank-ai-run`). Format
  `anansi-somnia-compaction-experiment` v1.
- **Contents:** the imported input records, the full candidate
  (retained / excluded / quarantined, hashes, stats), the provider
  and model identity, both prompts per pair, both replies, per-pair
  scores, abstentions, provider-reported input/output tokens per
  arm, failed pairs with errors, compaction time (µs per case and
  total), total latency, and the run's terminal status.
  `content_hash` = SHA-256 of the canonical serialization with the
  hash field empty (the `somnia.rs` doc pattern); `validate`
  recomputes it and refuses tampering fail-closed.
- **Privacy:** saved `private`, like SOMNIA evaluations and model
  comparisons. Opening a saved experiment never triggers a
  provider call. Existing saved runs (evaluations, model
  comparisons, SwarmTank runs) are untouched by this slice.

## §5 UI design

- **New component** `packages/ui/src/SomniaCompaction.tsx`,
  mounted as a section inside `SomniaPanel` (after the model
  comparison section). Because LARP scene 09 mounts the same
  `SomniaPanel`, the compaction flow appears in the chamber with
  no LARP-side code change; the chamber instruments keep showing
  replay/model activity as today.
- **Sections, in order:** import (textarea for JSON/text, or a
  frozen-case picker) → Prepare → review (retained by section,
  excluded with reason, quarantined with reason) → Approve
  (shows the bound hashes) → compare (provider/model inputs,
  cloud checkbox, Zero-Cost Lock line, estimated pair/dispatch
  budget) → Run / Cancel → results (per-pair table: case,
  original answer, compacted answer, score, tokens, failure) →
  Save → saved experiments list (reopen).
- **Approval binding in the UI:** any change to the import text
  or the loaded case clears `reviewed`/`approved` state
  immediately; the Run button is absent until a fresh review.
- **Strict parsing:** a `parseSomniaCompactionExperiment` parser
  for saved docs (exact keys, no silent coercion), mirroring the
  existing `parseSomniaEvaluationDoc` pattern. UI tests cover the
  parser and the review rendering.

## §6 Verification plan

- **Rust:** unit tests for the run module (approval-hash refusal,
  prompt rendering identical except record set, scoring incl.
  correct-abstention, budget exhaustion, cancellation flag,
  registry project isolation, save revalidation).
- **Contracts:** new DTOs registered; `contracts --check` 0 drift.
- **TypeScript, docs, UI tests** as usual; docs validator must
  pass with the new design + integration handoffs.
- **Synthetic-provider e2e** (`tests/e2e/somnia-compaction.spec.ts`):
  full workflow through the app with the harness fake provider —
  import → prepare → review (retained/excluded/quarantined
  visible) → approve → run (exactly 2 dispatches per pair, token
  counts shown) → save → artifact readback (private, hash
  validates) → reopen after API restart with zero further provider
  calls. Cancellation test (hold the dispatch route, cancel,
  assert no second dispatch and no saved artifact). Project-switch
  test (start a run in project A, switch to project B and back;
  state intact, no cross-project visibility).
- **Benchmark e2e** (`tests/e2e/somnia-compaction-benchmark.spec.ts`):
  all 40 frozen cases through the app workflow with the synthetic
  provider — plumbing proof only; accuracy labelled synthetic.
- **Regressions:** existing SOMNIA (slices 1–3), model comparison,
  LARP SOMNIA, documents/PDF, and SwarmTank suites stay green.
- **Live rerun (AC-I10):** the 40 frozen cases with an authorized
  live provider, through the same run op, results written to a raw
  JSON file with every case kept. Report: compaction time,
  answer accuracy (original vs compacted, per category),
  abstentions, provider-reported tokens, failed pairs, total
  latency. **Gate:** the rerun needs a provider that is authorized
  and reachable from the run environment. The preview's Windows
  evidence used OpenRouter Nemotron (free) and a warmed local
  Qwen2.5 3B; this Linux VM has no local model and holds no
  provider credential. If no authorized live provider is reachable
  here, the live rerun is reported blocked (not faked, not
  substituted with the synthetic provider), and the synthetic proof
  stands on its own.

## §7 Acceptance criteria

- **AC-I1** Design + acceptance criteria committed before code
  (this document).
- **AC-I2** Prepare is pure (no provider call, no persistence) and
  returns the full candidate with hashes; a tampered candidate
  hash is refused at start.
- **AC-I3** Review shows retained (trusted/unverified split),
  excluded with reasons, quarantined with reasons; approval binds
  to `input_hash` + `content_hash`; any edit clears approval in
  the UI and is refused by the server.
- **AC-I4** Compare dispatches both histories to the same
  configured model through the existing provider gates
  (Zero-Cost Lock, model-mismatch refusal, cloud requires explicit
  authorization); a failed/mismatched dispatch never produces a
  completed pair.
- **AC-I5** Cancellation stops the run between dispatches, saves
  nothing, dispatches nothing further; an in-flight run after an
  API restart polls not-found and never resumes.
- **AC-I6** Project scoping: runs are invisible from another
  project; switching projects preserves each project's state.
- **AC-I7** Save stores the complete experiment as
  `somnia-compaction-experiment` (private), fail-closed hash,
  reopenable with no provider call; existing saved runs untouched.
- **AC-I8** Synthetic-provider e2e proves the full workflow,
  including cancellation, project switch, and restart/reopen.
- **AC-I9** The frozen 40-case benchmark runs through the app
  workflow with a synthetic provider (plumbing proof; accuracy
  labelled synthetic).
- **AC-I10** Live rerun: 40 frozen cases with an authorized live
  provider; report compaction time, answer accuracy (original vs
  compacted, per category), abstentions, provider-reported tokens,
  failed pairs, total latency; every case in the results.
- **AC-I11** LARP scene 09 shows the compaction flow with the same
  controls; no provider traffic on scene entry; reduced motion
  honored.
- **AC-I12** No regressions in the existing SOMNIA, model
  comparison, LARP SOMNIA, documents/PDF, or SwarmTank suites; no
  global-status changes (AN-015 stays `verified`).

## §8 Boundaries (carried from Checkpoint 1)

- The compactor library's rules are frozen; this slice wires them
  in, it does not change them.
- No §47 consolidation proposal generates a prompt or changes live
  memory; the comparison prompts come only from the approved
  candidate and the imported input.
- Imported instructions remain data: never interpreted, never used
  to build commands, paths, URLs, or provider requests.
- Sealed input is refused fail-closed, as in the evaluator.
- No paid inference without explicit owner authorization; no
  credentials in chat, logs, or commits.
- Windows review findings outrank expanding scope.
