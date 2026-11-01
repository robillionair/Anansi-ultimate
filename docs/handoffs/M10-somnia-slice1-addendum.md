# M10 SOMNIA Slice 1 — Addendum (Linux delivery)

## Baseline

- Branch `muse/m10-somnia-slice1` from the Windows coordination tip
  `68ee353291d2a5732ccdeaa340807590e502b572` (review branch
  `codex/review-m9-swarmtank-slice3`; ZIP
  `anansi-m9-coordination-baseline-2026-11-01.zip` SHA-256
  `dd7596efd2ca1bbf075909210394282d044440eceb4c721aa436fe9eac55bdb2`,
  bundle SHA-256
  `55bb3ddf93bc2611223ad0315d09b7b10a35cf3c382ab49f116e898199063986`,
  ancestry verified to include `bf79643eb`). Reviewer delta over
  `bf79643eb` is docs-only: Slice 3 Windows verification report,
  BUILD_STATUS entry, AN-014 `in progress` → `verified`.
- M9 is complete on Windows; AN-014 is `verified`, WITH the HANDOFF
  caveat preserved: provider-backed reassessment and
  headline/value-proposition/audience scenario controls remain deferred
  despite milestone-complete wording, and the "Full AI Simulation"
  button invokes a deterministic model-free evaluator. Nothing in this
  slice expands AN-014 or invents live-model evidence.
- Design committed before code:
  `docs/handoffs/M10-somnia-slice1-design.md` (`e8050e894`).

## What was built (one complete workflow, nothing more)

Manual, isolated, data-only replay. No scheduling (§49 stays
manual-only), no consolidation (§47), no lesson promotion (§48), no
nightmare generation (§45), no historical trace ingestion (§44 real
events), no provider-backed evaluation. Results are findings, never
applied.

1. Select an existing project (panel mounted in the project view, same
   pattern as SwarmTank) and explicitly supply bounded synthetic or
   redacted input (JSON textarea, or "Load built-in cases").
2. Preview exactly what the evaluation will use — parsed client-side,
   no op call, no artifact — and approve it explicitly ("Run
   evaluation" is a separate click).
3. Run a deterministic, seeded reference evaluator (Rust, single
   source of truth; no TS mirror — there is no instant-local
   requirement here) against simulated tool events. No real tools, no
   network, no model.
4. Results show scenarios, observed decisions, constraint violations,
   and metrics, with measured-vs-unmeasured kept visibly apart.
5. Explicit "Save SOMNIA evaluation" registers one immutable
   `somnia-evaluation` artifact after server-side recomputation.
6. Restart, reopen, back up, and restore into a fresh workspace with
   provenance intact (existing jobs machinery; restore gate extended).

### Reference evaluator semantics (rules R1–R5, in order)

- R1 untrusted retrieved instruction → Refuse, violation
  `untrusted-instruction`, zero simulated calls (never executed).
- R2 declared instruction conflict → Refuse, `instruction-conflict`.
- R3 required evidence absent → Refuse, `missing-evidence`, reason
  naming the first missing id. Required-evidence ids are deliberately
  NOT checked for existence at input validation: an absent required id
  IS the R3 outcome, and the preview shows required vs present side by
  side so a typo is visible before approval.
- R4 first non-`ok` tool event aborts with no retry (`timeout` →
  `tool-timeout`, `error` → `tool-error`); `ok` events before it count.
- R5 all `ok` → Complete, calls = all `ok` events.
- Constraint check runs after R1–R5 for EVERY decision (a defect found
  by the e2e run: it originally ran only on Complete, so the timeout
  case's `max-tool-calls:0` violation was invisible; fixed in
  `1f440259c` with a regression test). It appends violations but never
  changes a decision, and never double-reports a refusal the decision
  already embodies (`no-untrusted-execution` / `evidence-required`
  fire only on non-refusal decisions).
- Seed effects are exactly runId and scenario display order (stable
  sort by SHA-256(seed || scenarioId)); decisions are seed-independent.
- Metrics genuinely measured: scenario counts, violation counts by
  category, simulated tool calls, local pure-function wall time. Wall
  time is reported ONLY in the evaluate response, never stored in the
  hashed doc — otherwise save-time recomputation could never match.
  Model tokens, model accuracy, production resilience, and real
  latency are shown as "Not measured", never zero-filled.
- Doc body carries NO project id (SwarmTank-run precedent): ownership
  lives on the artifact record column, so backup/restore remapping
  never orphans or rewrites the doc. Save requires the project to
  exist and be writable (same helper as SwarmTank/Forge); evaluate
  uses the same helper, deliberately stricter than existence alone so
  the ownership boundary is identical for both ops.

### Built-in cases (5 scenarios, seed `somnia-builtin-seed`)

- missing-receipt → Refuse (`missing-evidence`).
- conflicting-approval → Refuse (`instruction-conflict`).
- fetch-timeout → Abort (`tool-timeout` + `constraint-violated` via
  `max-tool-calls:0`, 1 prior call counted).
- malicious-note → Refuse (`untrusted-instruction`, 0 calls, retrieved
  text never executed, constraint satisfied by the refusal).
- clean-approval → Complete (2 calls, no violations).
- Totals: Complete 1, Refuse 3, Abort 1, simulated calls 3.

## Files (19 changed vs baseline, +5336/−7)

- NEW `crates/core/src/somnia.rs` (engine, DTOs, validation, hash,
  both ops, 12 in-module tests).
- EDIT `crates/core/src/artifacts.rs` (type 21 `somnia-evaluation`),
  `lib.rs` (module), `ai_discovery.rs` (allowlist),
  `provider_dispatch.rs` (`somnia-` prefix routing), `restore.rs`
  (`validate_original_somnia_bindings` gate, fail-closed).
- EDIT `crates/api/src/lib.rs` (two command paths),
  `crates/contracts/src/main.rs` (schema map), regenerated contracts
  (`generated.ts`, `validators.*`, `schemas/readiness.json`),
  hand-maintained `packages/contracts/src/client.ts`
  (`evaluateSomnia`, `saveSomniaEvaluation`).
- NEW `packages/ui/src/Somnia.tsx` (panel, preview, results view,
  strict parsers, built-in cases; zero `innerHTML`), EDIT
  `packages/ui/src/Projects.tsx` (section mount, SwarmTank pattern).
- NEW `packages/ui/tests/somnia.test.tsx` (7 tests: parsers, sealed
  refusal, fail-closed defaults, doc-shape strictness, Not-measured
  rows, mount makes zero client calls, zero-`innerHTML` grep); root
  `package.json` `test:ui` list extended.
- NEW `tests/e2e/somnia-slice1.spec.ts` (2 tests, see below).
- Docs: design doc (+2 corrections recorded here: R3 existence rule,
  constraint-check scope, doc-has-no-project-id), this addendum,
  BUILD_STATUS entry, RECOVERY resume point, `plans/08-somnia.md`
  Slice 1 checkpoint, REQUIREMENTS_MATRIX AN-015 `planned` →
  `in progress`.

## Tests

- Rust SOMNIA suite 12/12: rule order (R1 beats R3), all four named
  cases + control, timeout/error abort with prior-call counting,
  max-tool-calls violation without decision change, constraint-on-abort
  + no-double-report, seed determinism (same input+seed byte-identical;
  different seed same per-scenario decisions, different runId, order a
  permutation), bounds (empty, 21 scenarios, duplicate ids, Sealed,
  bad constraint rule), tamper (mutated decision/input/hash/runId all
  fail validation).
- Node 159/159 (152 baseline + 7 new), tsc 0, contracts `--check`
  clean, docs validation clean, web build clean.
- E2E `tests/e2e/somnia-slice1.spec.ts` 2/2: (1) load built-in →
  preview (all 5 titles, required-evidence line) → preview creates no
  artifact → run (5/1/3/1, calls 3, all five violation categories,
  Not-measured rows, runId captured) → zero provider traffic → save
  (registry hash equals SHA-256 of the exact stored body, format v1,
  seed matches) → reload/reopen (same runId, same decisions) →
  backup/fresh restore (same runId, same metrics); (2) preview + run
  then Start over leaves no artifact; sealed input refused at preview
  with Run disabled; malformed JSON refused the same way; zero
  provider traffic throughout.
- Privacy: Sealed refused at preview (disabled Run), evaluate, and
  save; nothing silently dropped (preview counts == input counts).
- Ownership: foreign/archived/nonexistent projects refused via the
  shared writable-project helper (evaluate and save alike).
- Cancellation/abandonment: evaluate-then-abandon leaves no artifact
  (node mount test + e2e); failed save validation registers nothing
  (recompute gate).
- Recovery: restore gate revalidates every `somnia-evaluation` body
  (hash + full recomputation) before publication; e2e backup/restore
  leg proves survival with provenance (seed, runId, hash, synthetic
  labels) intact.

## Linux evidence (sequential)

- Rust workspace: **583 passed + 4 known live-Ollama environment
  failures** (571 baseline + 12 new SOMNIA tests; the 4 failures are
  all in `crates/core/tests/semantic_workflow.rs` failing with
  `StorageUnavailable` — the live embedding backend is absent on this
  VM, the same environmental family as prior slices, and no other
  suite failed). Rust SOMNIA suite 12/12.
- Strict Clippy (`--locked --workspace --all-targets -D warnings`)
  clean, `cargo fmt --check` clean.
- Node `test:ui` 159/159, tsc 0, contracts `--check` clean, docs
  validation clean (222 files at delivery), web build clean.
- Browser battery: **46/47 specs, 62/63 tests**. The single failure
  is `search.spec.ts`'s real-semantic test, which needs live Ollama
  embeddings absent on this VM and previously failed identically on
  the pristine baseline. The battery initially showed 3 additional
  failures (`evaluations`, `integrated_workflow`, `opportunities`),
  all strict-mode violations from a genuine Slice 1 regression: the
  panel's "Save evaluation" button duplicated the M6 evaluations
  panel's button on the same project page. Fixed by renaming the
  SOMNIA button to "Save SOMNIA evaluation" (button + saved state);
  all 3 specs pass on rerun, as does the SOMNIA spec (2/2).

## Known limitations (disclosed, not hidden)

- The evaluator is a rule engine over declared facts, not a model:
  it cannot detect conflicts or malice the input does not declare
  (e.g. an untrusted record the author marks `trusted: true` is
  treated as trusted — the flag is the author's claim, and the UI
  says so). This is a reference replay runner, not a detector.
- Evaluator wall time is local pure-function time, not model latency,
  and is not stored in saved evaluations (shown as "Not recorded"
  when viewing a saved doc).
- No revise path for evaluation artifacts: re-saving the same
  evaluation creates a new artifact (immutability by design); the
  panel disables Save after a successful save per run to avoid
  accidental duplicates.
- One-frame and ordering notes from SwarmTank do not apply here (no
  3D view in this slice).

## Still open (not closed by this slice)

- Independent Windows rerun of this slice by ChatGPT (checklist in
  MANIFEST, incl. desktop-shell confirmation).
- M7 native desktop acceptance + preview promotion; §32 sandbox; M5
  production gates; Windows verification of M8 Slice 4.
- M9 caveat from HANDOFF: provider-backed reassessment and
  headline/value-proposition/audience scenario controls remain
  deferred despite AN-014 `verified`.
- Next slice (recommended): §45 seeded nightmare generation
  (deterministic variations over the Slice 1 case shapes, still
  manual and data-only) + §47 consolidation proposals as review-only
  inert proposals + §87.9 before/after comparison. Scheduling (§49),
  lesson promotion (§48), and historical ingestion stay out until
  separately approved.

## Requirement coverage

- AN-015 (SOMNIA): `planned` → `in progress` for this slice only.
  MASTER_SPEC §87 items covered by Slice 1: select a project (item 1,
  project-scoped panel), start an isolated evaluation (item 2,
  manual-only), see scenarios generated (item 3 — curated + supplied,
  not generated variations), review test results (item 4). Items 5–10
  (recurring weaknesses aggregates, proposals, approve/reject changes,
  re-run, before/after comparison, restore previous version) remain
  open; item 10's restore leg is partially covered (backup/restore of
  the evaluation artifact itself survives, but no version rollback of
  workspace state is claimed).
- AC-1–AC-10 from the design doc: met on Linux (see tests above).
  AC-11 (battery green except known Ollama env failures) met:
  46/47 specs, 62/63 tests, single failure the known pre-existing
  search real-semantic test. AC-12 (checksummed ZIP delivery)
  completes at packaging; Windows verification remains the
  reviewer's step.
