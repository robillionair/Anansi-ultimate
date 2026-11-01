# M10 SOMNIA Slice 1 — Design (before code)

## Baseline and authority

- Baseline: Windows coordination tip `68ee353291d2a5732ccdeaa340807590e502b572`
  (review branch `codex/review-m9-swarmtank-slice3`; reviewer delta over
  `bf79643eb` is docs-only: Slice 3 Windows verification report,
  BUILD_STATUS entry, AN-014 `in progress` → `verified`).
- Authority: MASTER_SPEC §42–51 (SOMNIA engine, safe environment, inputs,
  nightmare generation, evaluation, consolidation, lessons, scheduling,
  dashboard, product development), §72 (security boundary), §73 (synthetic
  provenance), §74 (privacy classes), §87 (Definition of Done — SOMNIA),
  §66 (shared background job system — reused, not duplicated), §67 (cost
  management — no paid inference in this slice, so no budget path is built);
  `plans/08-somnia.md` (planning stub — this slice is its first
  implementation); coordination HANDOFF.md 2026-11-01.
- Assignment boundaries preserved:
  - M9 is complete on Windows and AN-014 is `verified`, WITH the HANDOFF
    caveat kept verbatim in scope: provider-backed reassessment and
    headline/value-proposition/audience scenario controls remain deferred
    despite milestone-complete wording, and the "Full AI Simulation"
    button invokes a deterministic model-free evaluator. Nothing in this
    slice expands AN-014 or invents live-model evidence.
  - M7 native desktop acceptance + preview promotion, §32 sandbox, M5
    production gates, Windows verification of M8 Slice 4 remain open.
  - AN-015 (SOMNIA) moves `planned` → `in progress` for this slice only;
    it is not verified or completed by Linux evidence.
  - Antigravity owns `scripts/windows-release/`, `tests/windows-release/`,
    `docs/reviews/windows-release/` — untouched. No pushes, CI, preview
    promotion, paid/cloud inference, model downloads, or generated-product
    execution.

## Slice 1 scope (one complete workflow, nothing more)

Manual, isolated, data-only replay. No scheduling (§49 stays manual-only),
no consolidation (§47) and no lesson promotion (§48) — results are shown
as findings, never applied. No nightmare generation (§45): Slice 1 ships
four fixed curated cases plus arbitrary user-supplied input; seeded
variation generation is the recommended next slice.

Workflow (six steps, each explicitly gated):

1. Select an existing project (the panel lives in the project view, same
   mount pattern as SwarmTank) and explicitly supply a bounded synthetic
   or redacted replay input (JSON textarea, or "Load built-in cases").
2. Preview exactly what the evaluation will use — parsed client-side,
   no op call, no artifact — and approve it explicitly ("Run evaluation"
   is a separate click; preview alone does nothing persistent).
3. Run a deterministic, seeded reference evaluator (Rust, single source
   of truth; no TS mirror — there is no instant-local requirement here)
   against simulated tool events. No real tools, no network, no model.
4. Show scenarios, observed decisions, constraint violations, and
   results, with measured-vs-unmeasured metrics kept visibly apart.
5. Explicitly save an immutable evaluation artifact ("Save evaluation"
   is a separate click; the save op re-validates by full recomputation).
6. Restart, reopen, back up, and restore into a fresh workspace with
   provenance intact (existing jobs/backup machinery, existing restore
   gate extended to the new format).

## Data model

New Rust module `crates/core/src/somnia.rs` (tests in-module, SwarmTank
pattern). DTOs are schemars/serde structs, added to the contracts schema
map and the hand-maintained client.

- `SomniaReplayInput`: `{ scenarios: Vec<SomniaScenario>, seed: String }`.
- `SomniaScenario`: `{ id, title, category, instruction, evidence,
  retrieved, toolEvents, requiredEvidence, constraints }`.
  - `category`: one of `missing-evidence`, `conflicting-instructions`,
    `tool-timeout`, `malicious-instruction`, `contradictory-memory`,
    `sufficient-evidence` (the control case).
  - `evidence`: `[{ id, text, privacy }]`; `privacy` ∈
    `standard | private | public`. `sealed` is refused fail-closed.
  - `retrieved`: `[{ id, text, source, trusted: bool }]`. Retrieved text
    is untrusted data even when `trusted: true` is declared by the input
    author — the flag only records the scenario's claim; the evaluator
    never executes retrieved text either way.
  - `toolEvents`: `[{ tool, outcome }]` with
    `outcome` ∈ `ok | timeout | error`. All simulated; no tool runs.
  - `requiredEvidence`: `[evidenceId]` — the evidence the scenario
    declares necessary before any decision.
  - `constraints`: `[{ id, rule }]` with `rule` ∈
    `no-untrusted-execution | evidence-required | max-tool-calls:N`.
- `SomniaEvaluationDoc`: format `anansi-somnia-evaluation`, version 1:
  `{ format, version, input, seed, runId, scenarioOrder, results,
  metrics, contentHash }`. Artifact type `somnia-evaluation`
  (21st entry in `ARTIFACT_TYPES`, following the simulation/scenario
  precedent of one type per workflow). Self-contained: validation
  recomputes everything from the embedded input, so restore never trusts
  stored decisions. The doc body carries NO project id (SwarmTank-run
  precedent): ownership lives on the artifact record column set at
  registration, so backup/restore remapping never orphans or rewrites
  the doc.
- `runId`: `somnia-<first 16 hex of SHA-256(seed || contentHash-input)>`.
  Same seed + same input → byte-identical doc. Different seed → same
  decisions, different runId and different scenario display order (order
  is a stable sort by SHA-256(seed || scenarioId); the seed's only
  effects are runId and order — stated plainly, no fake randomness).

Bounds (fail-closed, exact errors; mirrors collider/swarmtank bounds):

- Scenarios ≤ 20; scenario id 1–64 chars, unique; title ≤ 200 chars;
  instruction ≤ 4000 chars.
- Evidence ≤ 20 items × ≤ 2000 chars; retrieved ≤ 20 × ≤ 2000 chars;
  tool events ≤ 50; requiredEvidence ≤ 20 ids; constraints ≤ 20.
- Seed 1–128 chars. Duplicate scenario ids, unknown constraint rules,
  and any `sealed` privacy value refuse the whole evaluation before
  any decision is computed. Required-evidence ids are deliberately
  NOT checked for existence: an absent required id is the R3
  missing-evidence outcome, and the preview shows required vs present
  side by side so a typo is visible before approval.

## Reference evaluator semantics (rule order is the contract)

For each scenario, in this exact order (first matching rule decides;
later rules never override an earlier refusal):

1. **R1 untrusted instruction**: any retrieved item with `trusted: false`
   → decision `Refuse`, reason `Refused untrusted retrieved
   instruction`, violation `{ category: untrusted-instruction }`.
   The evaluator stops for this scenario; no tool event is counted as
   authorized after this point.
2. **R2 conflicting instructions**: scenario declares
   `instructionConflict: true` (a boolean on the scenario; the
   conflicting texts are carried in `evidence`/`retrieved` for display)
   → decision `Refuse`, reason `Conflicting instructions`, violation
   `{ category: instruction-conflict }`.
3. **R3 missing evidence**: any `requiredEvidence` id absent from
   `evidence` → decision `Refuse`, reason `Missing evidence: <first
   missing id>`, violation `{ category: missing-evidence }`.
4. **R4 tool timeout**: any tool event with `outcome: timeout` →
   decision `Abort`, reason `Simulated tool timeout`, violation
   `{ category: tool-timeout }`. No automatic retry; recovery is
   reported as not recovered. An `error` outcome aborts the same way
   (`tool-error`); only all-`ok` events proceed.
5. **R5 proceed**: otherwise decision `Complete`, reason
   `All declared checks passed`, no violations; simulated tool calls =
   the count of declared `ok` events.

Constraint check (defense in depth, runs after R1–R5 for EVERY
decision, not only Complete):

- `no-untrusted-execution` violated if an untrusted retrieved item
  exists and the decision is not `Refuse` (a refusal already embodies
  it, so no second violation is added);
- `evidence-required` violated if required evidence is missing and the
  decision is not `Refuse` (same no-double-report rule);
- `max-tool-calls:N` violated if simulated tool calls > N.
- A violation from the constraint check appends to the scenario's
  violations; it never changes the decision (decisions come only from
  R1–R5, so a constraint failure is visible, not silently repaired).

Contradictory-memory category: modeled as R2 (conflicting instructions)
when the contradiction is between instruction-bearing records, or R3
when the contradiction removes a required evidence item — the curated
case documents which mapping it uses. The category label is display
metadata; rules operate on the declared facts, not on the label.

## Metrics: measured vs explicitly unavailable

Per evaluation (all computed locally, no model):

- Measured: scenario counts (total / complete / refuse / abort),
  violation counts by category, simulated tool-call totals, decisions
  per scenario, evaluator wall time in ms (local pure-function time,
  labeled as such — it is NOT model latency).
- Simulated (declared, not measured): per-event declared durations, if
  present, summed as "simulated elapsed ms (declared by the input)".
- Explicitly unavailable (shown as "Not measured", never zero-filled):
  model tokens, model accuracy, production resilience, real latency.
  The UI renders these rows with the reason "not measured in this
  reference replay".

## Privacy, ownership, security (fail-closed list)

- No automatic memory scanning: the panel never reads project memories
  to build input. Project selection only supplies `projectId` for
  ownership; input text comes only from the textarea or the built-in
  cases button.
- No Sealed input: any `sealed` privacy value refuses the whole
  evaluation (evaluate op) and the whole save (save op). The preview
  parser flags Sealed input with an explicit error banner and disables
  Run — it never silently drops the record.
- No silent inclusion: the preview renders every record exactly
  (scenario, evidence, retrieved, constraints, tool events) with counts;
  what is previewed is byte-identical to what the evaluate op receives
  (the client sends the parsed input object, not re-typed text).
- Evaluation changes nothing: evaluate creates no artifact, no memory,
  no policy, no project, no permission change. Save registers exactly
  one new artifact; it never revises, merges, consolidates, promotes,
  or schedules anything. There is no revise path for evaluation
  artifacts in Slice 1.
- Ownership: evaluate requires the project to exist and be writable
  (same helper as SwarmTank/Forge — stricter than existence alone,
  deliberately, so the ownership boundary is identical for evaluate
  and save); save registers under the request project, and the
  artifact record column is the ownership authority. Saved evaluations
  are listed per project; opening one is a read-only `artifact_read`.
- User text and imported traces remain untrusted data: instruction and
  retrieved text are never interpreted as instructions by the app, never
  executed, never used to build shell commands, file paths, URLs, or
  provider requests. The UI renders them as text (no `innerHTML`).
- No executable agent code, generated software, real tool execution,
  production accounts, filesystem capabilities, credentials, or network
  activity from replay instructions — by construction: the evaluator is
  a pure function over declared facts with no I/O.

## Persistence, restore, backup

- Ops (allowlist in `crates/core/src/ai_discovery.rs`, paths in
  `crates/api/src/lib.rs`, prefix-routed like `swarmtank-`):
  - `somnia-evaluate`: `{ contractVersion, workspaceId, projectId,
    input }` → `{ evaluation }` (ephemeral; nothing persisted).
  - `somnia-save-evaluation`: `{ contractVersion, workspaceId,
    projectId, evaluation }` → `{ artifact }`. Server re-validates by
    full recomputation (format, version, bounds, privacy, decisions,
    hash) and registers type `somnia-evaluation`, title
    `SOMNIA evaluation: <first scenario title ≤160 chars>`, generator
    `{ kind: "deterministic", generator: "anansi-somnia-evaluator" }`,
    status `validated`. A tampered client doc fails closed.
- Restore gate: new `validate_original_somnia_bindings` in
  `crates/core/src/restore.rs`, mirroring
  `validate_original_swarmtank_bindings`: every
  `artifact_type = 'somnia-evaluation'` row with format
  `anansi-somnia-evaluation` v1 is hash-checked and fully revalidated
  before publication; failure blocks the restore (same fail-closed
  posture as SwarmTank runs).
- Backup/restore survival uses the existing jobs machinery — no new job
  framework, no parallel storage (§66 reused).

## Cancellation and concurrency (honest semantics)

The evaluator is a synchronous pure function; there is no long-running
job to cancel, and Slice 1 does not pretend otherwise:

- Abandonment: closing the panel, navigating away, or starting over
  before Save leaves no artifact and changes no state (tested at UI
  and op level: evaluate-then-abandon → artifact list unchanged).
- Independence: consecutive evaluations (different seeds or inputs)
  produce distinct artifacts with distinct runIds/hashes; the pure
  evaluator returns identical output for identical input+seed (tested).
- No partial save: the save op is atomic through the existing
  registration path; a failed validation registers nothing.

## No inference or artifact on open/preview (guarantee + tests)

- Opening the panel: renders static UI only. No op call, no artifact.
- Preview: client-side parse + render only. Invalid JSON, oversized
  input, duplicate ids, or Sealed records produce inline errors and
  disable Run; nothing is sent.
- Tests: node test renders the panel and asserts zero client calls on
  mount and after preview; e2e asserts the artifact list is unchanged
  after open + preview + abandon.

## Files

- NEW `crates/core/src/somnia.rs` (engine, DTOs, validation, hash,
  both ops, in-module tests).
- EDIT `crates/core/src/artifacts.rs` (type 21), `ai_discovery.rs`
  (allowlist), `restore.rs` (gate), `transfer.rs`? No — no transfer
  in Slice 1 (do not touch).
- EDIT `crates/api/src/lib.rs` (two command paths).
- EDIT `crates/contracts/src/main.rs` (schema map) + regenerated
  contracts + hand-maintained `packages/contracts/src/client.ts`
  (`evaluateSomnia`, `saveSomniaEvaluation`).
- NEW `packages/ui/src/Somnia.tsx` (panel + results view; add to the
  zero-`innerHTML` grep list), EDIT `packages/ui/src/Projects.tsx`
  (section mount, SwarmTank pattern).
- NEW `packages/ui/tests/somnia.test.tsx` (parsers, preview
  no-op-call, Sealed flag, metric display incl. "Not measured" rows);
  EDIT root `package.json` `test:ui` list.
- NEW `tests/e2e/somnia-slice1.spec.ts` (builtin cases → preview →
  approve → run → results → save → restart/reopen → backup/fresh
  restore → evaluation intact; abandonment leaves no artifact; zero
  provider traffic).
- Docs: this design doc, addendum, BUILD_STATUS entry, RECOVERY resume
  point, `plans/08-somnia.md` Slice 1 checkpoint, REQUIREMENTS_MATRIX
  AN-015 `planned` → `in progress` (scope + Linux evidence + Windows
  pending qualifier; never `verified` from Linux evidence).

## Tests (meaningful, by family)

- Unit (Rust, in-module): rule order R1–R5 incl. R1-beats-R3 and
  R3-beats-R4 precedence; constraint-check invariants; seed determinism
  (same input+seed byte-identical; different seed same decisions,
  different runId/order); bounds (20 scenarios, duplicate ids, unknown
  evidence ids, Sealed refusal); hash tamper fail-closed (mutated
  decision, mutated input, mutated hash all fail validation).
- Privacy: Sealed refused at evaluate and at save; preview flags Sealed
  and disables Run; no record is dropped silently (preview counts ==
  input counts).
- Ownership: nonexistent/foreign project refused at evaluate and save;
  saved evaluation lists only under its own project.
- Concurrency: pure-function determinism under repeated calls;
  sequential saves yield distinct artifacts/hashes.
- Cancellation/abandonment: evaluate-then-abandon leaves no artifact
  (node + e2e); failed save validation registers nothing.
- Tamper/refusal: tampered doc refused at save (recompute mismatch);
  oversized/malformed input refused with exact errors.
- Recovery: saved doc validates after simulated restore (validate fn);
  e2e backup/fresh restore shows the evaluation intact with provenance
  (seed, runId, content hash, synthetic labels).
- Browser: the full 6-step workflow above, zero provider traffic.

## Acceptance criteria

- AC-1: Project selection + explicit bounded input (typed or built-in)
  reaches the evaluator; no memory is scanned to build it.
- AC-2: Preview shows exactly the evaluation input (counts + full
  text); preview alone creates no artifact and makes no op call.
- AC-3: Explicit Run executes the seeded reference evaluator; results
  show per-scenario decisions, reasons, violations, and metrics, with
  model tokens/accuracy/production resilience shown as Not measured.
- AC-4: The four initial cases behave per R1–R5: missing evidence →
  Refuse; conflicting instructions → Refuse; simulated tool timeout →
  Abort with no retry; malicious retrieved instruction → Refuse with
  the instruction never executed (and no tool event counted after it).
- AC-5: Explicit Save registers one immutable `somnia-evaluation`
  artifact after server-side recomputation; tampered docs fail closed.
- AC-6: Restart + reopen shows the saved evaluation unchanged;
  backup/fresh restore preserves it with provenance (seed, runId, hash,
  synthetic labels) intact.
- AC-7: Sealed input is refused at preview (disabled Run), evaluate,
  and save; nothing is silently dropped or included.
- AC-8: Evaluation changes no memory, policy, project source, or
  permission; no consolidation, promotion, or scheduling occurs.
- AC-9: Abandoning before Save leaves no artifact; evaluations are
  independent across seeds/inputs.
- AC-10: Synthetic inputs are labeled synthetic everywhere (panel,
  preview, results, artifact title/body); historical events are never
  claimed — Slice 1 has no historical ingestion path at all.
- AC-11: Full Linux battery green except the known live-Ollama
  environment failures; reviewer probes (if any in baseline) pass
  unchanged.
- AC-12: Delivery is a checksummed ZIP with bundle + MANIFEST +
  evidence + Windows checklist; Windows verification remains the
  reviewer's step and is never claimed from Linux evidence.

## Out of scope (deferred, not claimed)

- §45 seeded nightmare generation/variation (recommended next slice,
  with §47 consolidation proposals as review-only inert proposals and
  §87.9 before/after comparison).
- §47 consolidation, §48 lesson promotion, §49 scheduling (manual-only
  stays), §50 dashboard aggregates, §51 Forge integration, historical
  trace ingestion (§44 real events), provider-backed evaluation.
- Native save dialog, desktop-shell confirmation (Windows checklist),
  per-agent visuals (not applicable here).
