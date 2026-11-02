# M10 SOMNIA Slice 2 — Design (before code)

## Baseline and authority

- Baseline: owner tip `8ba785aebb120e98eee9b7698a1d7eb11c5a3395`
  (branch `muse/m10-somnia-slice1`, Slice 1 delivery
  `anansi-m10-somnia-slice1-2026-11-02.zip`). The assignment also
  names Windows review commit `37970d3a…`, but that commit is not
  present locally and no review ZIP was uploaded with this
  assignment, so the assignment's stated fallback applies: the
  owner's tip with the doc updates incorporated. The Slice 2
  closure docs therefore record the Windows review verdict below
  instead of building on an unverified ref.
- Slice 1 Windows review verdict (reviewer's evidence, recorded
  here, not re-run on Linux): `tsc --noEmit` exit 0,
  `scripts/contracts.mjs --check` exit 0 (zero drift),
  `scripts/validate-docs.mjs` PASS (223 Markdown files — one more
  than the Linux 222, consistent with a docs-only reviewer delta),
  `npm run test:ui` 159/159, `cargo test -p anansi-core --lib
  somnia` 12/12, full workspace 583 passed (0 failures, 4
  pre-existing offline-Ollama skips), Vite production build clean
  in 2.23s, browser battery under shared lease 12/12 (SOMNIA Slice
  1 2/2, collision regressions 3/3 verifying the "Save SOMNIA
  evaluation" label fix, SwarmTank regressions 7/7). On that
  verdict, AN-015 advances `in progress` → `verified` for the
  Slice 1 scope only; Slice 2 work below is a new `in progress`
  scope and is not verified by this review.
- Authority: MASTER_SPEC §45 (nightmare generation), §47
  (consolidation), §87.6 (review proposals), §87.9 (before/after
  comparison), §39 (scenario controls — wording repair only, no
  new mode), §87.7 (approve/reject = review acknowledgement only
  in this slice, see below); Slice 1 design
  (`docs/handoffs/M10-somnia-slice1-design.md`) and its evaluator
  contract (rules R1–R5 + constraint check on every decision).
- Boundaries preserved: no scheduling (§49 stays manual-only), no
  lesson promotion (§48), no live memory or policy mutation (§47
  proposals are inert and review-only), no provider/cloud
  inference, no network traffic outside localhost, no
  generated-product execution, no pushes/CI/preview promotion.
  Antigravity owns `scripts/windows-release/`,
  `tests/windows-release/`, `docs/reviews/windows-release/` —
  untouched. M9 remains complete on Windows with its HANDOFF caveat
  (provider-backed reassessment and headline/audience controls
  deferred); nothing here expands AN-014.

## Slice 2 scope (one bounded slice, nothing more)

1. Seeded nightmare generator (§45) over the seven assigned
   failure classes, deterministic in Rust and in a TS preview
   mirror, evaluated by the unchanged Slice 1 evaluator.
2. Review-only inert consolidation proposals (§47, §87.6) derived
   from a saved nightmare evaluation — displayed, never applied.
3. Before/after comparison studio (§87.9) over two saved
   evaluations, with deterministic JSON/Markdown export.
4. One bounded copy repair (§39): the SwarmTank scenario
   re-run button wording, so the UI no longer claims a "Full AI
   Simulation" it does not perform.

Explicitly out: dashboard aggregates (§50), Forge integration
(§51), scheduling (§49), lesson promotion (§48), historical
trace ingestion, provider-backed reassessment, any new artifact
registry type, any new server op, any schema change.

## A. Seeded nightmare generator

### Determinism design (why not SHA-256 counter-mode)

Slice 1 uses SHA-256 for runId and scenario order, but the TS
side has no synchronous SHA-256 in the browser (WebCrypto is
async), and the assignment requires the Rust engine and the TS
preview mirror to yield identical scenario sequences. The
generator therefore uses exact integer arithmetic reproducible
in both languages with no crypto dependency:

- Seed string → u32 state by FNV-1a (offset 2166136261, prime
  16777619, wrapping multiply; JS uses `Math.imul`, Rust uses
  `wrapping_mul` — identical results by construction).
- Stream: xorshift32 (`x ^= x << 13; x ^= x >> 17; x ^= x << 5`,
  all wrapping u32; JS uses `Math.imul`-free 32-bit ops with `| 0`
  discipline and unsigned `>>> 0`).
- Per case, draws happen in a fixed order and fixed count, so a
  given (seed, count) always yields the same sequence. The
  generator is a pure function: no clock, no randomness source,
  no I/O. Wall-clock evaluation time stays out of the canonical
  doc hash exactly as in Slice 1.

### Classes, sequence, and rule mapping

`generate_nightmares(seed, count)` with count ∈ {5, 10, 20} in
the UI (function accepts 1–20, refuses 0 and >20). Case `i`
(0-based) uses class `CLASSES[i % 7]`, in assignment order:

1. `conflicting-instructions` → R2 Refuse. Scenario:
   `instructionConflict: true`, evidence present, no untrusted
   retrieved, no tool events. Category: existing
   `conflicting-instructions`.
2. `missing-evidence` → R3 Refuse. Scenario: required evidence
   id absent from evidence (the id itself is PRNG-picked from a
   fixed word list, so repeats across runs are detectable).
   Category: existing `missing-evidence`.
3. `simulated-tool-error` → R4 Abort. Events `ok, error, ok`:
   prior calls 1, violation `tool-error`. Category: new
   `tool-error` (added to the `CATEGORIES` allow-list).
4. `simulated-tool-timeout` → R5 Abort. Events `ok, timeout`:
   prior calls 1, violation `tool-timeout`. Category: existing
   `tool-timeout`.
5. `untrusted-retrieved-instruction` → R1 Refuse, 0 calls.
   One retrieved record with `trusted: false`, source
   PRNG-picked from a fixed source list. Category: new
   `untrusted-retrieved-instruction`.
6. `cost-limit-exhaustion` → Complete with a
   `constraint-violated` violation and NO decision change.
   Events `ok × 4`, constraint `max-tool-calls:2`. Category:
   new `cost-limit-exhaustion`.
7. `privacy-boundary-challenge` → fail-closed refusal of the
   whole input. One evidence record with `privacy: sealed`.
   Slice 1 `validate_input` already refuses any sealed record
   before any decision is computed, at preview, evaluate, and
   save. Category: new `privacy-boundary-challenge`.

The `CATEGORIES` allow-list therefore grows 6 → 10. Scenario
categories are plain strings in the DTOs, so no contract change
is expected; `contracts --check` must still pass and will be
re-verified. Scenario ids are `nightmare-<class>-<index>`;
titles, instruction text, evidence ids, sources, and tool names
are template slots filled from fixed word lists by PRNG draws.

### Sealed-case handling in the panel (user-initiated, explicit)

A generated set containing class 7 cannot run: preview shows
the sealed refusal and Run stays disabled, exactly as Slice 1
behaves for user-supplied sealed input. The panel offers one
explicit action, "Exclude sealed case", which removes that
scenario from the input (client-side only, before any op call)
and re-renders preview; the user then runs the remainder. The
generator never silently drops the sealed case, and the
exclusion is visible in the preview (scenario count drops by
one). E2E covers: generate 7 → sealed refusal shown → exclude
→ run 6 → per-class decisions verified.

### Rust/TS mirror and golden proof

- Rust: `pub fn generate_nightmares(seed: &str, count: usize)
  -> Result<Vec<SomniaScenario>, String>` in `somnia.rs`
  (reference implementation; no new op — the panel generates
  client-side for instant preview and evaluates through the
  existing `somnia-evaluate` / `somnia-save-evaluation` ops,
  the Slice 3 "engine mirrored in Rust and TS" precedent).
- TS: `generateNightmares(seed, count)` in `Somnia.tsx` next
  to `BUILTIN_SCENARIOS` (no new module, no `test:ui` list
  edit; the existing `somnia.test.tsx` covers it).
- Golden proof: one fixed seed (`somnia-nightmare-golden`)
  and count 7 produce an exact expected scenario JSON asserted
  byte-identically in a Rust test and a TS node test. If the
  mirrors ever drift, the golden fails in both suites. Rust
  tests also cover: determinism (same seed twice → identical
  JSON; different seed → different content, same per-class
  decisions), count bounds (0 and 21 refused), and per-class
  evaluation outcomes through `build_evaluation_doc` (each
  class lands on its intended decision/violation).

## B. Review-only inert consolidation proposals

- Derivation, not storage: proposals are a pure function over a
  saved evaluation doc, computed at view time in TS
  (`deriveConsolidationProposals(doc)`) with a Rust mirror for
  unit tests and golden agreement. No new artifact registry
  type, no restore-gate change, no schema change: a proposal
  set recomputes identically from a restored doc, so
  backup/restore survival is inherited from the evaluation
  artifact (SwarmTank-report precedent: derived, not stored).
- Detection rules (thresholds stated plainly; counts are over
  the evaluation's own scenarios only):
  - An evidence id absent from evidence in ≥ 2 scenarios →
    `flag-missing-evidence` (target: the id; evidence: the
    scenario ids).
  - An untrusted retrieved source appearing in ≥ 2 untrusted
    retrieved records → `archive-untrusted` (target: source).
  - A tool name with ≥ 2 timeout events →
    `procedural-rule` (target: tool; proposed rule text names
    a retry/backoff-or-replace convention as inert text).
  - Instruction-conflict in ≥ 2 scenarios →
    `supersede-conflict` (target: the scenario ids; proposed
    text names the superseding-record convention as inert
    text).
  - A `max-tool-calls` constraint violation in ≥ 1 scenario →
    `procedural-rule` (target: the constraint; proposed budget
    text as inert text).
- Proposal shape: `{ id, action, target, rationale,
  proposedChange, evidence: [scenarioIds] }`, sorted by action
  then target for stable display. `id` is
  `proposal-<action>-<index>` in that sorted order.
- Safety boundary (the load-bearing rule): the panel displays
  the exact proposed change, rationale, and a diff-style
  preview block, and offers NO apply button, NO mutation op,
  and NO API call. Originals remain untouched and recoverable
  because nothing in this slice can write to them. §87.7
  "approve or reject" is satisfied as review acknowledgement
  only: a local-only "Mark as reviewed" toggle (React state,
  never sent anywhere, never persisted) records that a human
  looked at the proposal. It changes no workspace state.
- Rust mirror: `derive_consolidation_proposals(doc)` over
  `SomniaEvaluationDoc`, unit-tested on a fixed doc with a
  golden proposal list matched by the TS test.

## C. Before/after comparison studio

- Panel section "SOMNIA comparison": two selects over the
  saved evaluations list (Baseline, Comparison) plus a Compare
  action. Both sides are loaded read-only via the existing
  `readProjectArtifact` path; nothing is written.
- Deltas shown side by side with signed values:
  Complete/Refuse/Abort counts, total simulated tool calls,
  total violation count, and a category breakdown matrix
  (rows = union of violation categories; columns = baseline
  count, comparison count, signed delta). Rows with baseline 0
  and comparison > 0 are labeled "newly exposed".
- Deterministic export: JSON (the comparison object, keys in
  construction order, no timestamps, no wall time) and
  Markdown (fixed template over the same object) via Blob
  download. E2E waits for the download event and asserts
  content. Export is client-side; no new op.
- Comparison inputs are saved evaluations only (the current
  unsaved run joins the list after Save). This keeps the
  comparison reproducible: the same two artifact ids always
  compare identically, including after backup/restore.

## D. SwarmTank scenario controls wording repair (§39)

- File: `packages/ui/src/SwarmtankScenarioControls.tsx` only.
- Button: `🤖 Re-run Full AI Simulation` →
  `Re-run Deterministic Scenario Simulation` (the first of the
  two options the assignment allows; recorded here so the
  choice is unambiguous).
- Help text (currently: "Re-run re-evaluates the scenario
  inputs through the same deterministic evaluate operation as
  the baseline run, with the same seed. It makes no model
  call.") → text that explicitly states: the button invokes
  the deterministic model-free PRNG evaluator using the
  specified seed, and does not perform live AI inference or
  contact a model provider. No live-AI mode is added: §39's
  "Re-run AI Simulation" describes a model-generated
  reassessment mode that does not exist in this product, and
  the repair makes the UI honest about what the button does
  instead of inventing that mode.
- Blast radius checked before this design: no UI test asserts
  the old label (only `rerunDisabled` props are passed in
  `swarmtank.test.tsx`), and no e2e spec clicks the button by
  name. The SwarmTank regression specs in the verification
  list still run unchanged.

## UI structure (Somnia panel additions)

New controls live inside the existing `SomniaPanel`, after the
saved-evaluations section, reusing its section/aria patterns
and the zero-`innerHTML` regime:

- "SOMNIA nightmare generator": seed input (default
  `somnia-nightmare-seed`), count select (5 / 10 / 20),
  "Generate nightmares" button → replaces the textarea input
  with the generated JSON and runs the normal preview path
  (so preview, sealed refusal, exclusion, Run, and Save all
  behave exactly as in Slice 1 — no second workflow).
- "SOMNIA consolidation proposals": rendered whenever a saved
  evaluation is viewed (or the current run exists), showing
  derived proposals with diff preview and the local-only
  reviewed toggle; an explicit "Review only — nothing is
  changed." notice.
- "SOMNIA comparison": baseline/comparison selects, Compare
  action, delta table, category matrix, "Export comparison
  JSON" and "Export comparison Markdown" download buttons.
- Button names are workflow-prefixed ("Generate SOMNIA
  nightmares", "Compare SOMNIA evaluations") to avoid a repeat
  of the Slice 1 "Save evaluation" collision.

## Tests

- Rust (`cargo test -p anansi-core --lib somnia`): generator
  determinism, golden JSON, count bounds, per-class decisions,
  proposal derivation + golden, sealed refusal of a generated
  set (via `validate_input`), comparison delta math as a pure
  function over two docs (shared by the panel's TS mirror
  through the same golden comparison object).
- UI (`packages/ui/tests/somnia.test.tsx`, extended): nightmare
  controls generate the golden set; proposals render inert
  (no apply button exists); comparison deltas render with
  signed values and newly-exposed labels; export builders
  produce deterministic JSON/Markdown; panel still makes no
  client call on mount; file still contains zero `innerHTML`.
- E2E (`tests/e2e/somnia-slice2.spec.ts`, 2 tests):
  1. Generate 7 nightmares → preview shows sealed refusal →
     exclude sealed case → run 6 → per-class decisions and
     metrics verified → save → view → proposals displayed
     (all four actions present where thresholds bite) with no
     apply control → compare saved baseline vs saved nightmare
     run → deltas and newly-exposed rows verified → export
     JSON and Markdown downloads asserted → zero provider
     traffic.
  2. Backup/fresh restore → both saved evaluations intact →
     comparison and proposals recompute identically from the
     restored docs → zero provider traffic.
- Regressions (must pass cleanly): Slice 1 spec, evaluations,
  integrated_workflow, opportunities, SwarmTank specs
  (slice1, arena, scenario), then the full battery.

## Verification and delivery

Sequential on an idle machine: Rust workspace suite
(`--no-fail-fast`, awk-summed `test result` lines), strict
Clippy, `cargo fmt --check`, node `test:ui`, `tsc --noEmit`,
`contracts --check`, docs validation, web build (before e2e),
new e2e spec, regression specs, full battery. Closure docs:
BUILD_STATUS prepend (including the Slice 1 Windows verdict
above), RECOVERY resume point, `plans/08-somnia.md`
checkpoint, addendum, REQUIREMENTS_MATRIX (AN-015 `verified`
for Slice 1 scope + Slice 2 scope noted; never verified from
Linux evidence). Packaging follows the standing rule: staging
dir with MANIFEST.json (baseline, tip, commits, Linux evidence
totals, Windows checklist, remaining gates) + full-history
bundle + evidence logs; ZIP + `.zip.sha256` in
`~/workspace/your_files/`; independent scratch verification
(ZIP hash = sidecar, bundle hash = MANIFEST, fetch = exact
tip, ancestry, intended-files-only diff, credentials sweep);
working tree clean at the end.

## Acceptance criteria

- AC-1: Same seed + count → byte-identical nightmare JSON in
  Rust and TS (golden test in both suites).
- AC-2: Each of the 7 classes evaluates to its intended
  decision/violation (Rust + e2e).
- AC-3: Sealed nightmare case refuses at preview and is only
  removed by explicit user exclusion; nothing is silently
  dropped.
- AC-4: Proposals derive deterministically from a saved doc,
  display exact change/rationale/diff preview, and have no
  apply path (no op, no API call, no button).
- AC-5: Comparison shows signed metric deltas, category
  matrix with newly-exposed labels, and deterministic JSON/MD
  exports.
- AC-6: SwarmTank button reads `Re-run Deterministic Scenario
  Simulation` with help text stating model-free PRNG
  evaluation, no live AI inference.
- AC-7: Nightmare + comparison + proposals survive
  backup/fresh restore identically (derived from restored
  docs).
- AC-8: Zero provider traffic in all new e2e tests; no new
  artifact type, no schema change, no release-tooling edits.
- AC-9: Full verification green except the known pre-existing
  live-Ollama environment failures (Rust `semantic_workflow`,
  search real-semantic e2e).
- AC-10: Checksummed ZIP delivery with scratch verification;
  Windows verification remains the reviewer's step.
