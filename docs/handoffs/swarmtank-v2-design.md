# SwarmTank v2 — design + acceptance criteria

## 0. Baseline and branch

- Baseline: `7bf05e3a4dab185930a35eb68efac0a17c96bd7c`
  (`codex/swarmtank-matrix-handoff`, "Add interactive SwarmTank matrix
  to LARP scene 08"), itself on the pilot merged baseline `3bf51af8`.
  Verified before editing: bundle SHA
  `67254f9f0d6170aa88eb07d37f8d3119dbdab6b12a2ab608d9887597606d806d`
  == MANIFEST; tip == MANIFEST; baseline is an ancestor; the handoff
  is one commit on the pilot baseline. Source ZIP
  `anansi-swarmtank-matrix-muse.zip` SHA
  `8df1ea29a81649469e39a1d148cbc672629dfde0017b422eb5efef4f2ecdc881`.
- Branch: `muse/swarmtank-v2`.
- Preserved: the current seeded SwarmTank runs, the LARP scene 08
  matrix (cinematic card matrix, orbitable network, decorative
  cables), the browser regression, and the existing M9 behaviour.

## 1. What this build adds

SwarmTank v2 turns the seeded simulator into a regional, AI-driven,
time-aware scenario tool — without changing what the seeded simulator
already does, and without ever presenting a simulation as measured
real-world demand (§34, §40).

Three reviewable checkpoints:

1. **Market selection + profile provenance** (§2).
2. **A small multi-round AI run through the actual application with a
   synthetic provider test** (§3).
3. **Timeline playback + one what-if branch, in regular SwarmTank and
   in LARP scene 08** (§4).

## 2. Checkpoint 1 — Market selection + profile provenance

### 2.1 Territory data (CLDR, versioned)

- One canonical, versioned territory list, generated from Unicode
  CLDR territory names with the CLDR version pinned and recorded in
  the file. The same file is the source for both sides: the UI
  dropdown (TypeScript import) and the Rust validator
  (`include_str!`). A generator script pins the CLDR version and
  regenerates both copies together; the generated file is committed,
  so the build does not need network access.
- The dropdown is searchable and lists every CLDR territory. Every
  country is selectable. A territory with no researched profile
  opens in a **Custom assumptions** state: the user supplies the
  market assumptions explicitly, and the run records them as
  custom/unresearched (never as evidence).

### 2.2 Market profile format (data, not code)

- A versioned, data-driven profile format. Adding a region means
  adding a data file — no app-code change. The format lives in
  `packages/contracts/src/markets/` (shared) with the Rust side
  reading it via `include_str!` and the UI importing it.
- Each profile: `id` (CLDR territory code, or territory +
  subdivision, e.g. `AE-DXB`), `name`, `cldrVersion`,
  `profileVersion`, `subdivisions` (optional), `segments`, and two
  input lists:
  - `evidence`: every evidence-backed input carries `field`,
    `value`, `source` (name + URL or citation) and `date` (ISO).
  - `assumptions`: gaps are explicit, editable assumptions, each
    with `field`, `value`, `rationale`, `editable: true`.
- Personas are built from buyer role, industry, company size,
  buying authority, language, and budget evidence where available.
  **Geography is separate from personality**: the market profile
  supplies geographic/buyer-context evidence (language, procurement
  norms, size distribution, budget ranges); it never assigns a
  personality trait by nationality. A persona's skepticism,
  priorities and price sensitivity come from its buyer role and the
  scenario, not from its country.

### 2.3 Starter profiles

Three researched profiles ship first, each with a source and date
for every evidence-backed input and editable assumptions for gaps:

- **Estonia (EE)** — small, highly digital market; English widely
  used in business alongside Estonian; e-Residency and digital
  public services shape procurement expectations.
- **UAE → Dubai (AE-DXB)** — subdivision profile under the UAE
  territory; English as the business lingua franca, relationship
  and trust signals prominent in procurement, free-zone company
  structures common.
- **United States (US)** — large, heterogeneous market; English
  primary, procurement varies sharply by company size, formal
  vendor processes common at enterprise scale.

Evidence must be real and sourced (no invented market facts). Where
a claim cannot be sourced, it is an explicit assumption, not
evidence. The profiles are the starting set; the format is what
makes the next region addable without code.

### 2.4 Acceptance criteria (Checkpoint 1)

- AC-M1: The market selector lists every CLDR territory and is
  searchable by name and code.
- AC-M2: Each of EE, AE-DXB and US opens a profile showing its
  evidence (each with source + date) and its editable assumptions,
  clearly distinguished.
- AC-M3: Selecting a territory with no researched profile opens the
  Custom assumptions state and records the run as custom.
- AC-M4: A new region can be added by adding a data file only (a
  test adds a fourth profile from data and the selector shows it
  with no app-code change).
- AC-M5: Personas generated for a market use the profile's
  role/industry/size/authority/language/budget evidence; no persona
  field is derived from nationality.
- AC-M6: Old saved runs (no market) still load and render; their
  market is shown as "Unspecified".

## 3. Checkpoint 2 — AI simulation mode

### 3.1 Two modes, clearly separate (§39)

- **Quick simulation** is the existing seeded simulator
  (`evaluate_persona`, deterministic, seeded). Unchanged.
- **AI simulation** is new. The two modes are labelled distinctly
  in the UI and in the stored run; a price-slider update remains a
  synthetic sensitivity calculation, never a fresh AI evaluation.

### 3.2 Agents

- Each AI agent is a distinct synthetic agent with individual
  goals, constraints, and bounded memory. Bounded memory = a
  per-agent window of the last N relevant events plus selected
  prior interactions, capped by count and by characters; the cap is
  part of the run record. An agent never sees the full run history
  or another agent's private goals.
- All agents are labelled simulated, as in M9 (`synthetic: true`).

### 3.3 A round

- At each round, an agent observes the relevant scenario events
  (including scheduled events due that round) and selected prior
  interactions, then submits one validated action:
  `request_evidence`, `defer`, `recommend`, or `reject`.
- The model output is parsed as JSON and validated against a
  strict schema (action enum, required fields, any evidence
  reference must name an event or interaction that exists in the
  run). A malformed output is **not** retried into a different
  action: the call is recorded as failed in the run history with
  the raw output (truncated), and that agent's round is recorded
  as failed. The run continues with the remaining agents.
- A failed provider call (transport, auth, timeout, gate refusal)
  is likewise recorded in the run history with its error, and the
  agent's round is recorded as failed. The run does not silently
  drop the agent.

### 3.4 Provider layer, gates, budget

- Uses ANANSI's existing provider layer: `providers-dispatch`
  (synchronous) via `providers_command`, model resolved by
  `configured_chat_models` (first configured chat model in
  configuration order; configured local models are the default).
  `localOnly` is set when the user chooses local-only.
- Zero-Cost Lock applies: before Start the UI shows the mode,
  the chosen provider + model, its locality (local/cloud), and the
  estimated call budget (rounds × agents × max output tokens, plus
  the input estimate). A cloud/paid model requires the existing
  explicit authorization (permit flow); without it the run refuses
  before any call.
- Caps are enforced server-side: max rounds, max calls, max tokens
  per call, max total tokens. Budget exhaustion ends the run in a
  clear "budget exhausted" state (not an error, not a silent stop).
- Cancellation is checked between agent calls and between rounds;
  a cancelled run is saved in its cancelled state with the rounds
  completed so far.

### 3.5 Size of the first end-to-end run

- The default AI run is small enough to test on ordinary hardware:
  3 agents, 2 rounds, small token cap. The UI offers larger
  presets only after the small run is proven.
- The synthetic provider test uses a fake provider (as the existing
  e2e fake-provider servers do) so the full flow is verified
  without a real model.

### 3.6 Stored run (reopen without rerunning inference)

- A completed (or cancelled, or budget-exhausted) AI run is saved
  as a versioned artifact (`swarmtank-ai-run`, format v1) with a
  fail-closed content hash, containing: market + profile version,
  scenario input, scheduled events, per-round per-agent actions,
  evidence references, model identity (provider + model) per call,
  token usage where available, per-agent bounded-memory snapshots
  (or the window parameters), and the scenario assumptions. A saved
  run reopens from the artifact with no provider call.
- Old saved runs (the M9 `SwarmtankRunDoc` format) are unchanged
  and still load.

### 3.7 Acceptance criteria (Checkpoint 2)

- AC-A1: Quick simulation is unchanged (existing seeded tests pass;
  the mode label is distinct from AI simulation).
- AC-A2: A 3-agent, 2-round AI run completes end-to-end through the
  actual application with the synthetic provider; every agent action
  is one of the four valid actions or a recorded failure.
- AC-A3: The estimated call budget (calls + tokens) is shown before
  Start and matches the configured caps.
- AC-A4: A failed provider call appears in the run history with its
  error; the run continues with the remaining agents.
- AC-A5: A malformed model output is recorded as a failed action
  with the raw output; it is not retried into a different action.
- AC-A6: Budget exhaustion ends the run in the "budget exhausted"
  state with the completed rounds saved.
- AC-A7: Cancelling mid-run saves the completed rounds in the
  cancelled state.
- AC-A8: A saved AI run reopens with no provider call and shows the
  same rounds, actions, evidence references and model identity.
- AC-A9: Per-agent state (goals, constraints, bounded memory)
  persists correctly across rounds (a test asserts round 2's
  observation includes round 1's selected interaction and not an
  unbounded history).
- AC-A10: If an installed local model is available, a small smoke
  test runs and reports actual calls, latency and token usage where
  available. (On this Linux VM no local model is installed; the
  smoke test is Windows-pending unless a model appears.)

## 4. Checkpoint 3 — Time, what-if branches, LARP

### 4.1 Time and branches

- The user chooses a future end date and a step size within an
  explicit run budget. A longer horizon can use coarser steps; the
  UI shows the resulting step count and warns when the horizon
  forces a coarser step than the user asked for.
- Scheduled events: a price change or a competitor launch, each
  with a date/round and a payload. Events are part of the run
  record and are observed by agents in the round they fall due.
- Branch comparison: from a saved round, the user changes the price
  or adds an event and continues from that point. The branch is
  stored as a child of the original run (branch id + parent round)
  and the comparison shows the original timeline beside the branch.
- Outcomes are labelled **conditional simulation results** wherever
  they are shown. Longer horizons make assumptions more prominent:
  the assumption list is surfaced (not buried) and grows with the
  horizon in the report view.

### 4.2 LARP

- Keep the cinematic card matrix and the orbitable network, the
  decorative cables, reduced motion, and the accessible report view.
- Add a timeline scrubber, market and branch labels, and persona
  cards that show what each agent did in the selected round and
  what evidence or prior event it cited.
- Recorded agent interactions (from the run doc) are visually
  distinct from decorative wiring (cables). The distinction is
  labelled in the UI, not just in code.
- Market setup and playback are available in regular SwarmTank too,
  not only in LARP.

### 4.3 Acceptance criteria (Checkpoint 3)

- AC-T1: A run with a future end date and a chosen step size
  completes within its run budget; a longer horizon uses coarser
  steps and says so.
- AC-T2: A scheduled price change (or competitor launch) is
  observed by agents in the round it falls due.
- AC-T3: From a saved round, changing the price (or adding an
  event) and continuing produces a branch that compares against
  the original timeline; both timelines are labelled conditional
  simulation results.
- AC-T4: The timeline scrubber selects a round; the persona cards
  show that round's agent action and its cited evidence or prior
  event.
- AC-T5: Recorded agent interactions and decorative wiring are
  visually and textually distinct in the LARP view.
- AC-T6: Reduced motion and the accessible report view still work
  on the AI run and the branch.
- AC-T7: Market setup and playback work in regular SwarmTank (not
  only LARP).

## 5. Verification (final)

- Old saved runs load (AC-M6, AC-A1).
- Per-agent state across rounds (AC-A9).
- Budget exhaustion (AC-A6), cancellation (AC-A7), malformed model
  output (AC-A5), restart and reopen (AC-A8), backup and restore
  (the new artifact type survives a backup + fresh restore with
  its content hash intact), and the LARP interaction path
  (existing `larp-swarmtank.spec.ts` plus the new timeline tests).
- The existing SwarmTank and LARP regressions pass.
- Local-model smoke test (AC-A10) where a model is installed.

## 6. Calibration plan (separate status — not validation)

Real-world validation is a separate status from dropdown coverage
and researched profiles. The calibration plan (shipped as a doc) is:

1. Hold out a set of real pilot interviews (or real pilot
   behaviour) before running the simulation; never tune the agents
   on the held-out set.
2. Run the simulation on the same product + market + price, with
   the same scenario assumptions recorded.
3. Compare, per agent cohort, the simulated decision distribution
   (Convert/Confused/Reject) and the top cited objections against
   the held-out interviews' stated decisions and objections.
4. Report agreement per cohort and per objection, with the sample
   sizes, and state plainly where the simulation disagrees.
5. Any change to the agents or the decision logic that follows from
   a calibration round is recorded as a new profile/scenario
   version, with the old version kept for comparison. Never
   overwrite the version the calibration measured.
6. A simulation is never presented as validated until a calibration
   round has been run and reported; before that it is a conditional
   simulation result.

## 7. Honest boundaries (carried from M9 + §34/§40)

- A simulation is not measured real-world demand, conversion data,
  or a substitute for customer interviews, A/B tests, or
  statistically valid market research.
- Confidence scores are internal model indicators, not empirically
  calibrated probabilities unless validated (per §37).
- Do not claim N separately reasoned AI agents if one model created
  N persona records (§36). In AI mode the agents are distinct
  model calls with distinct prompts and memory; the run records how
  many calls were actually made.
- No paid calls, no pushes, no remote CI in this assignment.
- The Zero-Cost Lock and the existing privacy, disclosure,
  authorization and cost gates apply to every provider call.
- Worldwide dropdown coverage, researched regional profiles, and
  real-world validation are three separate statuses; completing one
  does not complete the others.
