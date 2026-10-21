# M6 Slice 2 — design and acceptance criteria (approved scope)

Approved by the owner 2026-10-21: **project-scoped idea
generation** through the existing universal projects, the
Slice 1 artifact registry, and the M5 provider substrate.
Base: `codex/local-testing-r11` at `f6cecd2` (R11 + the Windows
desktop artifact-command capability fix + preview launcher,
preserved). Branch: `muse/m6-slice2-idea-generation`.

## Requirement mapping

- **§22 (One-Click Idea Generation)** supplies the workflow:
  inputs = industry/topic, intended customer, available budget,
  preferred technology, business/personal objective; optionals
  = use approved ANANSI memory, explore unrelated domains,
  prefer simple products, prefer high-revenue opportunities,
  prefer local-first AI, prefer software-only products. Output
  = **5–10 candidate concepts**, default presentation **3 first
  + expansion**. Per-concept actions in §22: Explore, Compare,
  Save, Send to Collider, Start Building.
- **§21 (Opportunity Discovery Engine)** supplies the concept
  schema (12 fields): product name, problem, target customer,
  proposed solution, why it may be useful, differentiation
  hypothesis, potential revenue model, engineering complexity,
  approximate implementation cost, key risks, required
  validation, minimum viable version. §21 rules carried into
  the prompt AND the UI: **never scan memories without
  permission** (context is explicitly selected per run) and
  **do not invent market-size statistics; distinguish
  hypotheses from researched facts** (concepts are labelled
  model-generated hypotheses everywhere they appear; nothing
  in this slice presents them as researched).
- **§9**: the generated set is saved as a project artifact
  (type `idea`) in the Slice 1 registry — attached to the open
  project, immutable, versioned like everything else.
- **Not in this slice** (owner boundaries): the scoring rubric
  (§21's "transparent criteria" — the UI shows the model's own
  ordering and says so; no scores are computed or faked),
  the Opportunity Engine, pre-project/standalone idea storage,
  Compare/Collider/Build engines (later milestones; the UI
  shows those actions as unavailable-with-reason, never as
  working buttons).

## Design

New core module `crates/core/src/ideas.rs`; ops ride the
existing `ai_command` providers arm (identical CallerScope
resolution and lazy persisted-config install as every provider
op), and generation orchestrates the **existing** provider
stream ops in-process (`providers-stream-start/-poll/-cancel`
via `providers_command` with the caller's scope). No second
inference path, no new gates: routing, privacy, Zero-Cost
Lock, permits and disclosure are M5's, unchanged.

- `ideas-generate-start` {workspaceId, projectId, topic,
  customer, budget, technology, objective, contextMemoryIds[],
  options{…§22 toggles}, model?, provider?, mode?,
  zeroCostLock, permitId?}:
  validates the project (Slice 1 rules; archived → refused);
  reads each selected memory **in this workspace** (unknown id
  → refused; nothing is ever auto-scanned — an empty selection
  sends no memory content); builds the prompt (instruction +
  §22 fields + optionals; context memories become provider
  segments carrying their own privacy class and source id, so
  the M5 PrivacyGate applies per segment — sealed context is
  refused by the gate exactly as in Ask); starts a provider
  stream; returns {runId, disclosure}.
- `ideas-generate-poll` {workspaceId, runId}: drains stream
  events, accumulates text. Terminal states: `complete`
  (output validated → idea set attached), `invalid` (provider
  answered but output failed validation — the failure reason
  is returned, no set is fabricated), `failed` (provider/
  transport error, sanitized), `cancelled` (only after the
  stream's terminal event — the r9 honesty rule), otherwise
  `running`/`cancelling` with a partial character count.
- `ideas-generate-cancel` {workspaceId, runId}: forwards to
  the stream cancel; the run reports `cancelling` until the
  terminal event lands via poll.
- `ideas-save` {workspaceId, runId}: only from `complete`.
  Registers the set as ONE artifact (type `idea`) whose body
  is the canonical idea-set JSON, generator metadata =
  {kind:"model", provider, model, disclosureId-ish provenance
  from the run's disclosure}, validationStatus `validated`
  (structural validation passed — explicitly NOT factual
  verification). **Idempotent**: a second save returns the
  same artifact (`alreadySaved: true`) — duplicate submissions
  never duplicate artifacts. The project-writable rule is
  re-checked at save time.

**Output validation** (pure function, provider output is
untrusted data): optional code fences stripped; strict JSON;
root object with a `concepts` array; count 5–10 — fewer is a
validation failure, more is truncated to 10 with a recorded
note; each concept must carry all 12 §21 fields as bounded
strings (keyRisks an array of 1–6 bounded strings; a lone
string is normalized to a one-item array); unknown fields are
dropped, never executed or stored. Non-JSON output (including
prose refusals) → `invalid` with the reason "did not return a
structured idea set". Bounds: name ≤120, problem/solution
≤800, targetCustomer ≤300, whyUseful/differentiation ≤600,
revenueModel ≤400, complexity ≤60, cost ≤300, risks ≤240
each, requiredValidation/minimumViableVersion ≤600 chars.

**Runs** are process-local (like provider streams):
per-workspace map, caller-session bound, 600 s TTL sweep, max
32 retained per workspace. Run state does not persist; the
saved artifact does — restart evidence is about the artifact.

## Surfaces

Contracts (ideas DTOs incl. IdeaConcept/IdeaSet), client
methods on the `ai/` group, API `command_path` additions for
the four `/v1/ai/ideas-*` paths (preflight), desktop free via
the ai_command routing. UI: an **Idea generation** section in
the Projects screen for the open project — §22 form, explicit
context picker (workspace memory titles with checkboxes and
the consent wording), generate/cancel, first-3 concepts with
"show all" expansion, per-concept actions (Explore = expand,
functional; Compare / Send to Collider / Start Building =
shown unavailable with their milestone reason), the run's
§19 disclosure panel, and Save → the artifact appears in the
project's Artifacts list; opening a saved `idea` artifact
renders its concepts (hypothesis labelling included).

## Acceptance criteria (each maps to a named test)

1. **Workflow (e2e)**: open project → fill §22 form → select
   one context memory → generate against a synthetic provider
   → 3 concepts shown, expansion shows all → disclosure
   inspectable → save → artifact listed → restart API →
   reopen project → open artifact → concepts rendered.
2. **Validation (unit)**: count bounds (4 → fail; 11 → 10 +
   note), every field bound enforced, missing field → fail,
   malformed JSON → fail, prose refusal → fail, fences
   tolerated, unknown fields dropped, risk string normalized.
3. **Generation integration (core)**: synthetic streaming
   provider + configured free local model → complete run with
   a validated set and a disclosure whose provider/model match
   the wire request; context memory body reached the provider
   (server-side capture) and an unselected memory's body did
   not.
4. **Cancellation (core)**: slow-drip provider → cancel →
   terminal `cancelled`; save from a cancelled run refused.
5. **Provider failure (core)**: provider 500 → `failed` with a
   sanitized error; malformed stream output → `invalid`;
   neither leaves a savable set.
6. **Duplicate submission (core)**: save twice → same artifact
   id, `alreadySaved` on the second; the registry holds exactly
   one version.
7. **Isolation (core)**: poll/save with a run id from another
   workspace → NotFound; a context memory id from another
   workspace → refused at start; a non-project id → refused.
8. **Provenance & honesty (core + contracts)**: saved
   artifact's generator records provider + model from the
   actual dispatch; validationStatus `validated`; the artifact
   body round-trips through the registry read with integrity
   intact; contracts validators accept the DTOs and reject a
   concept missing a §21 field.
9. **No regressions**: M2–M6.1 suites at baseline (providers
   120, core 497+new with only the 4 live-Ollama failures, API,
   UI/contracts, the three prior e2e specs).

## Out of scope (explicit)

Scoring/ranking (model order is presented as model order),
Opportunity Engine, Compare/Collider/Build actions, idea
editing, saving individual concepts separately, live-provider
certification, and any change to M5 gates or the Slice 1
registry semantics. Antigravity's Windows R11 findings are
consumed at review boundaries, not merged from here.
