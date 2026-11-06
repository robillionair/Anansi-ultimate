# SwarmTank v2 — Checkpoint 2: AI simulation mode

## Scope

Checkpoint 2 of the SwarmTank v2 design (`docs/handoffs/swarmtank-v2-design.md`
§3, AC-A1..A10). A genuinely AI-driven simulation mode alongside the
preserved seeded Quick simulator. Distinct synthetic agents with individual
goals, constraints and bounded memory; round-by-round validated actions
through the existing provider layer; server-side caps and budget exhaustion;
cancellation between calls and between rounds; a versioned
`swarmtank-ai-run` artifact (format v1, fail-closed content hash) that
reopens with no provider call. No paid calls, no pushes, no remote CI.

## What changed

### Rust: new module `crates/core/src/swarmtank_ai.rs`

- **Agents.** Built from the Quick personas (market-shaped when a market is
  given, via `generate_personas_for_market` / `generate_personas`): each
  agent embodies one persona; its goal is derived from the persona's needs
  and priorities, its constraints from the purchase constraints, budget,
  price sensitivity, required trust signals and expected implementation
  effort. Geography shapes which buyers appear, never a personality trait.
- **A round.** Each agent observes the scenario events due that round plus
  a bounded memory window (the most recent prior actions, capped by count
  and by characters — `MEMORY_MAX_EVENTS = 5`, `MEMORY_MAX_CHARS = 4000`;
  the cap is part of the run record and the snapshot records exactly which
  event/interaction ids the agent observed). It then submits one validated
  action: `request_evidence`, `defer`, `recommend`, or `reject`.
- **Provider layer.** `providers-dispatch` (synchronous) via
  `providers_command`, model resolved by `configured_chat_models` (first
  configured chat model in configuration order; configured local models
  are the default). `localOnly` is set when the user chooses local-only or
  the model is local. Zero-Cost Lock is on (`zeroCostLock: true`).
- **Up-front Zero-Cost Lock check.** The model is resolved and its locality
  looked up in the provider catalog before any call. A cloud model without
  an explicit permit refuses the start ("This model requires explicit
  authorization…") before any call is made; local-only with a cloud model
  refuses likewise. An unknown provider is treated as cloud (fail-closed).
- **Malformed output / failed calls.** The model output is parsed as JSON
  and validated against a strict schema (action enum, rationale 1–2000
  chars, any evidence/prior-interaction reference must name an event or
  interaction that exists in the run, unknown fields refused). A malformed
  output is **never** retried into a different action: the call is recorded
  as failed with the raw output (truncated to 500 chars), that agent's
  round is recorded as failed, and the run continues. A failed provider
  call is recorded the same way, with its error.
- **Caps (server-side).** Agents 1–10 (default 3), rounds 1–10 (default 2),
  max calls = agents × rounds (hard cap 100), max output tokens 64–2048
  (default 512), max total tokens 1000–200000 (default 20000). Before every
  call the runner checks calls-made and tokens-used + the next call's
  estimate against the caps; exhaustion ends the run in the terminal state
  `"budget-exhausted"` (not an error, not a silent stop).
- **Cancellation.** An `Arc<AtomicBool>` is checked before every agent call
  and between rounds. `swarmtank-ai-cancel` sets it; the thread saves the
  run as `"cancelled"` with the rounds completed so far.
- **Architecture.** `swarmtank-ai-start` validates, resolves the model,
  builds the agents, and spawns a background thread that runs the
  simulation. The run lives in a process-local registry (mirroring
  ideas.rs: 600 s TTL, 32 runs per workspace) so `swarmtank-ai-poll`
  reports live progress (rounds, calls, tokens, per-agent actions) and
  `swarmtank-ai-cancel` can stop it. The thread saves the artifact when the
  run reaches a terminal state. `swarmtank-ai-estimate` returns the model,
  locality and estimated calls/tokens with no run and no provider call
  (the input estimate is measured from a real first-round prompt, not a
  guessed constant).
- **Stored run.** Artifact type `"swarmtank-ai-run"` (the 22nd entry in
  `ARTIFACT_TYPES`), format `anansi-swarmtank-ai-run` v1, fail-closed
  content hash (SHA-256 of the canonical serialization with the hash field
  empty). Contains: market + profile version, scenario input, scheduled
  events (price_change / competitor_launch / scenario, each bound to a
  round), per-round per-agent actions, evidence references, model identity
  per call, token usage where available, per-agent bounded-memory
  snapshots, and the scenario assumptions (including the verbatim
  "conditional simulation results" and "not a calibrated probability"
  lines).

### Routing (three allowlists again, plus a fourth)

The new ops (`swarmtank-ai-estimate/start/poll/cancel`) had to be listed in:
the API path allowlist (`crates/api/src/lib.rs`), the `swarmtank_command`
dispatcher, and the `ai_command` allowlist (`ai_discovery.rs`). A fourth
list was found by a live probe: the persisted-provider-config lazy-load in
`ai_command` only runs for named ops, so `swarmtank-ai-estimate` and
`swarmtank-ai-start` were added there — without it a restart left "No chat
model is configured" even though the config file existed. (The market ops
needed no workspace; the AI ops do, so they are not in the
catalog-style exemption.)

### Contracts + client

- The AI DTOs are registered in `crates/contracts/src/main.rs`; `contracts
  --check` is clean. Client methods `estimateSwarmtankAi`,
  `startSwarmtankAi`, `pollSwarmtankAi`, `cancelSwarmtankAi` in
  `packages/contracts/src/client.ts`.
- `SwarmtankAiPollResponse` carries the agents (so the UI can name
  per-agent state) as well as the rounds, calls, tokens, status and the
  saved artifact id/version.

### UI

- New `packages/ui/src/SwarmtankAi.tsx`: `SwarmtankAiPanel` (estimate,
  agent/round/token-cap inputs, Start/Cancel, live progress with per-agent
  actions and visibly failed calls, saved AI runs list, reopen) and
  `SwarmtankAiRunView` (the saved run: status, model/locality, calls,
  tokens, per-round actions with cited evidence and token usage, failed
  actions with failure + raw output, assumptions, "conditional simulation
  result" label). `parseSwarmtankAiRunBody` is a strict client-side parser
  (format/version/status/action enum/content-hash shape; the hash itself is
  verified server-side on restore).
- `packages/ui/src/Swarmtank.tsx`: a Simulation mode radio (Quick /
  AI) above the action buttons. Quick is unchanged (seeded, deterministic,
  no provider call). AI shows the AI panel. The mode is labelled distinctly
  in the UI and in the stored run; a price-slider update remains a
  synthetic sensitivity calculation, never a fresh AI evaluation.

## Acceptance criteria — results (AC-A1..A10)

- **AC-A1** — Quick simulation unchanged: the Quick generate/evaluate flow
  and its e2e (market spec) and regressions pass unmodified; the mode
  toggle defaults to Quick.
- **AC-A2** — a small multi-round AI run completes through the actual
  application with a synthetic provider: e2e `swarmtank-ai.spec.ts` test 1
  (3 agents × 2 rounds, fake Ollama, 6 calls, all actions valid,
  run saved). Proven end to end, not just at the API level.
- **AC-A3** — the estimated call budget is shown before Start: the estimate
  line reads `Estimated budget before Start: ollama/llama-x (local),
  6 provider calls, N tokens estimated.` and the start notice repeats the
  model/locality/calls/tokens. The estimate is measured from a real
  first-round prompt.
- **AC-A4** — a failed provider call is visible in the run history: the
  runner records dispatch errors as failed actions with the error message
  (proven at the API level by the two early dispatch-shape failures, and
  by the `dispatch_agent_call` error path; the e2e malformed test proves
  the same visible-failure surface for malformed output).
- **AC-A5** — malformed output is recorded, not retried, and the run
  continues: e2e test 2 (fake returns non-JSON on call 2; that agent's
  round shows `Failed.` + `Malformed model output:` + `Raw output: this
  is not json`; the other five actions are valid and the run completes).
- **AC-A6** — budget exhaustion ends the run in a clear state: e2e test 3
  (max total tokens 4000; round 1 completes, round 2 cannot start; status
  `Budget exhausted`, the message names the token budget, the run is saved
  with the completed round).
- **AC-A7** — cancellation saves the completed rounds: e2e test 4 (fake
  delayed 3 s/call; Cancel clicked mid-run; status `Cancelled`, the run is
  saved and reopens as Cancelled).
- **AC-A8** — per-agent state across rounds: round-2 actions observe
  round-1 interactions (the memory snapshots show
  `observedInteractionIds: ["round-1-agent-1", …]`; the e2e run view shows
  each agent's round-2 action). An agent never sees the full history or
  another agent's private goals (the window is capped at 5 events /
  4000 chars; unit-tested).
- **AC-A9** — reopen with no provider call, restart, backup/restore: the
  e2e reopen asserts the fake hit count does not increase; the API-level
  backup script proves restart (artifact reads back with the same hash
  after an API restart) and backup + fresh restore (the
  `swarmtank-ai-run` artifact survives with its content hash intact,
  status complete, 2 rounds, 6 calls).
- **AC-A10** — local-model smoke test if available: **no local model is
  installed on this Linux VM** (no `ollama` binary, nothing on
  `127.0.0.1:11434`). The synthetic-provider e2e is the verification
  here; a real local-model smoke test is **Windows-pending** (run the AI
  e2e's flow against an installed Ollama/LM Studio model and record the
  actual calls, latency and token usage).

## Verification (Linux, this branch)

- Rust workspace lib: **275 passed, 0 failed** (api 4 + core 210 + desktop
  1 + providers 60). Core +5 (the AI module tests).
- TypeScript: **0 errors**. Contracts `--check`: **0 drift**. Docs
  validator: **310 PASS**.
- UI tests: **326 passed, 0 failed** (incl. 3 new AI parser tests).
- E2E `tests/e2e/swarmtank-ai.spec.ts`: **4/4** (complete run, malformed,
  budget exhaustion, cancellation).
- E2E `tests/e2e/swarmtank-market.spec.ts`: **3/3** (Quick + market
  unchanged). Regressions `larp-swarmtank` + `swarmtank-arena`: **4/4**.
- API build: clean except the pre-existing `mcp.rs:715` warning.
- Direct API smoke (`evidence/ai-api-smoke.log`): estimate 6 calls /
  ~5172 tokens, start, poll complete with 6 calls / 870 tokens, artifact
  reads back (format `anansi-swarmtank-ai-run`, status complete, 2 rounds,
  hash present). The fake Ollama was hit exactly 6 times.
- Backup/restore + restart (`evidence/ai-backup-restore.log`): restart
  preserves the artifact (hash identical); backup + fresh restore preserves
  the `swarmtank-ai-run` artifact with its content hash intact.

## Defects found and fixed in this checkpoint

1. **Dispatch segment shape.** The dispatch JSON used
   `{kind: "text", text}` for message segments; the provider layer expects
   `{text, privacy}` (`Segment {text, privacy, …}`). Rejected as "Use a
   supported provider dispatch request." until fixed.
2. **Routing mode.** The dispatch `mode` was set to `"swarmtank-ai"`; the
   router only accepts `auto | autoFree | private | fast | deep | coding |
   vision | chooseModel`. The mode is a routing mode, not a feature label —
   set to `"auto"` (the exact model is already pinned by `chosen`).
3. **Persisted config lazy-load.** The `ai_command` lazy-load list did not
   include the AI ops, so after a restart the estimate/start refused with
   "No chat model is configured" even though the config file existed.
   Added `swarmtank-ai-estimate` and `swarmtank-ai-start` to that list.
4. **Three allowlists, again.** As in Checkpoint 1, the new ops needed the
   API allowlist, the `swarmtank_command` dispatcher, and the `ai_command`
   allowlist. The live curl probe caught the missing third list.
5. **Type mismatches in the new module** (found by the compiler, fixed
   before any run): `generate_personas` takes a `u32` preset;
   `SwarmtankInput.segment/industry/valueProposition` are plain `String`s,
   not `Option`s; `approximate_budget` is a plain `f64`; the catalog's
   `TargetLocality` enum is private (use the Debug form, as
   `providers-catalog` does); `ProviderDispatchResponse` has no
   `Deserialize` (read the `Value` fields instead); `ArtifactDetails`
   nests the record under `.artifact` (`.artifact_id` / `.version`).

## Known limitations

- **AC-A10 (local-model smoke test) is Windows-pending** — no local model
  on this VM. The synthetic-provider e2e is the Linux verification.
- The AI run's per-call token usage comes from the provider's reported
  usage; when a provider reports none, the runner falls back to the
  pre-call estimate for accounting (conservative: the budget check uses
  the estimate, so a run never overspends its cap because of missing
  usage data).
- The estimate is a planning figure (measured from the first agent's
  first-round prompt); later rounds' prompts differ slightly in length.
  The server-side cap is the enforcement, not the estimate.
- The poll interval is 2 s; a very fast run may complete between polls
  and the UI will jump straight to its terminal state (correct, just not
  animated).
- A run whose thread is still in flight when the API process exits is not
  persisted (the registry is in-memory); only terminal states save the
  artifact. A run started and then abandoned by closing the browser keeps
  running in the API until it finishes or is cancelled from another
  session — the registry TTL (600 s) bounds retention.

## Proposed global-status updates (for Antigravity; not applied here)

None to AN-014 — SwarmTank v2 is new scope on top of the verified M9
milestone, and this checkpoint adds no new requirement row.

## Windows acceptance checklist (Checkpoint 2)

1. Verify the branch tip and bundle hashes (see the delivery MANIFEST).
2. `cargo test --locked --workspace --lib` — expect 275 passed.
3. `npm run typecheck` (0), `node scripts/contracts.mjs --check` (0 drift),
   `node scripts/validate-docs.mjs` (310 PASS), `npm run test:ui` (326).
4. Build the API and the web app; run `tests/e2e/swarmtank-ai.spec.ts`
   (4/4), `tests/e2e/swarmtank-market.spec.ts` (3/3), and
   `tests/e2e/larp-swarmtank.spec.ts` + `tests/e2e/swarmtank-arena.spec.ts`
   (4/4) through the shared application-test lease.
5. In the browser with a configured local model (Ollama or LM Studio):
   open SwarmTank in a project, switch to AI simulation, confirm the
   estimate shows the provider/model/locality and the estimated calls and
   tokens, start a 3-agent / 2-round run, confirm it completes with valid
   per-agent actions, reopen the saved AI run and confirm no further
   provider call is made. Record the actual calls, latency and token
   usage (AC-A10).
6. With the synthetic provider (or a slow local model), confirm a
   malformed output is recorded as failed and the run continues, a small
   token budget ends the run as budget-exhausted, and Cancel saves the
   completed rounds.
7. Confirm the Quick simulation is unchanged (default mode, same
   generate/evaluate flow, no provider call).
