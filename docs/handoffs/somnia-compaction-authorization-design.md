# SOMNIA compaction authorization — design + acceptance criteria

## §0 Baseline, branch, scope

- **Baseline (verified):** the published Windows preview,
  `codex/windows-preview-integration` at
  `cf69e37e667a7767f6ce1b7c0f5dc81901e0a187`. The commit was not
  local at the start of this slice; origin's branch tip was
  verified by `git ls-remote` to be exactly `cf69e37e`, then
  fetched read-only into `preview/windows-preview-integration`.
  It descends from `4e4e970a` and contains the Checkpoint 2 tip
  `726aa8a07` (merged by `ad29bb5ae` "Merge SOMNIA compaction into
  the Windows acceptance preview").
- **Branch:** `muse/somnia-compaction-authorization`, created from
  `cf69e37e`.
- **This slice:** fix SOMNIA automatic-compaction model selection
  and authorization without weakening the provider gates. Three
  defects, found by reading the code on this baseline (all
  citations are to the baseline, before any fix):
  1. Blank Model ID is refused although the panel promises it
     selects the first configured model (§1 F1).
  2. The cloud control offers a comparison the server can never
     run, and a supplied permit could never authorize it (§1 F2).
  3. Approval binds only the candidate hash server-side; the
     selected target and the dispatch budget are not bound (§1 F3).
- **Windows work adopted as-is:** `0aef92a54` "fix(somnia): pin
  compaction dispatches to the selected model" (dispatch `mode`
  changed `auto` → `chooseModel`; the e2e fake echoes the
  requested model; the workflow test configures a second model,
  `aaa-decoy`). This slice keeps that pinning and tests it; it
  does not re-do it. `4c59ee043` (backup workflow locator fix)
  is likewise adopted, not re-done.
- **Out of scope:** changing the compactor library semantics
  (Checkpoint 1 rules are frozen), changing the frozen benchmark
  fixtures or the scorer, changing the offline SOMNIA evaluator,
  a new `providers-configured-models` read op, a run-scoped permit
  (design note only, §5), OCR or further document formats, any
  global-status change (AN-015 stays `verified`; no new
  requirement row until Windows review).

## §1 Findings (evidence, baseline code)

### F1 — Blank Model ID is refused

- UI: `packages/ui/src/SomniaCompaction.tsx:626` —
  `Model ID (leave blank for the first configured model)`.
  The start payload sends `model: undefined` when blank
  (`SomniaCompaction.tsx:482`).
- Server: `crates/core/src/somnia_compaction_run.rs:464`
  `resolve_model` matches `(Some(provider), None)` into the
  catch-all arm and refuses with "Provider and model must be
  given together, or neither." The default panel state is
  provider `ollama` + blank model, so the panel's own default
  is refused (400) with zero traffic. The existing workflow e2e
  test never exercises this path — it fills `llama-x`
  (`tests/e2e/somnia-compaction.spec.ts:74`).
- The UI has no read op for the installed configuration
  (`packages/contracts/src/client.ts` has `providersConfigure`
  but no configured-models read; `Providers.tsx` only knows the
  config it installed itself), so the panel cannot resolve the
  blank default client-side. Resolution belongs on the server.

### F2 — The cloud control can never run, and a permit could never cover it

- UI: `SomniaCompaction.tsx:627` — "Authorize cloud
  transmission for this comparison" sets `cloud: true`. The
  start payload carries no `permitId` (the UI never sends one).
- Server: `somnia_compaction_run.rs:680` — any cloud locality
  without a permit is refused before any call. So the offered
  cloud comparison is refused at start, always.
- Even with a permit it could not work. The permit contract
  (`crates/providers/src/dispatcher.rs:9-10`, `:163-176`):
  "A permit authorizes one candidate dispatch chain; consume it
  exactly once here (retries reuse this authorization)" — the
  first dispatch inserts the permit id into `used_permits`; a
  second `providers-dispatch` with the same id is refused with
  "paid-inference permit '…' was already consumed". A comparison
  needs up to 100 dispatches (50 pairs × 2 arms), so one permit
  cannot authorize it, and issuing up to 100 owner permits (each
  `providers-authorize` call, TTL 30–3600 s, per-call token
  ceiling) is not a safe wiring.
- The provider layer itself does NOT require a permit for a free
  model: `crates/providers/src/gates.rs:179` `authorize_cost`
  returns `Ok(())` for `CostClass::Free` without a permit, and
  the privacy gate (`gates.rs:63`) only blocks *private*
  segments to cloud — compaction segments are `standard`
  (`somnia_compaction_run.rs:858`), so they pass. The only
  blocker for a free cloud model is the compaction module's own
  locality-based gate. Verdict: **the existing API can safely
  authorize a bounded, free-only cloud comparison — without a
  permit** — via the user's explicit review + approval, the
  server's cost-class check from the trusted configuration, and
  the run's bounds. It cannot authorize a billable comparison,
  so billable targets must be refused up front (§2 D3).

### F3 — Approval does not bind the target or the budget server-side

- The UI approval snapshot is `{caseId, candidate, questions,
  candidateHash}` (`SomniaCompaction.tsx:291-299`); edits to
  provider/model/cloud/maxTotalTokens clear it client-side
  (`editCompare`, `:379-384`), but that is UI state only.
- The server verifies only the candidate hash per case
  (`verify_approved_candidate`, `somnia_compaction_run.rs:438`).
  The questions, the selected target, and `maxTotalTokens` are
  start-request fields with no server-side binding to the review:
  a start that replays an approval with different questions, a
  different target, or a different budget is accepted as long as
  the candidate hash matches. The questions are not part of the
  candidate at all, so editing a question after approval changes
  both prompts with no server refusal.

## §2 Design decisions

- **D1 — Blank model resolves server-side.** `resolve_model`:
  `(Some(p), None)` or a blank model string → the first
  configured chat model for provider `p`, in configuration
  order (`configured_chat_models`, `provider_dispatch.rs:963`).
  `(None, None)` → the first configured model overall
  (unchanged). `(Some, Some)` → must be configured (unchanged).
  `(None, Some)` → still refused. Blank provider/model strings
  are treated as absent. The UI keeps sending raw values.
- **D2 — Approval hash binds everything reviewed.** The start
  request gains `approvalHash: string` (hex SHA-256). The UI
  computes it at approve time over canonical JSON of
  `{cases: [{caseId, candidateHash, questions}] (in request
  order), provider, model, cloud, maxTotalTokens}` where
  provider/model are the RAW approved values (blank allowed),
  `cloud` is the approved flag, and `maxTotalTokens` is the
  approved budget (the default when unset). Canonical JSON is
  `serde_json::to_string` field order on the server and the same
  key order via `JSON.stringify` on the client — the same
  discipline as the experiment `content_hash`, which already
  agrees across Rust/TS. The server verifies each candidate hash
  first, then recomputes the approval hash from the verified
  candidate hashes + the request's questions/provider/model/
  cloud/maxTotalTokens, and refuses on mismatch — before
  resolving the model and before any dispatch. The UI stores the
  hash + the raw values in the approval snapshot; any edit clears
  it (already the case for target/budget; questions/history edits
  already clear it).
- **D3 — Free-only gate, cost from trusted configuration.**
  After approval verification and model resolution, the server
  reads the resolved model's cost class from the trusted
  configuration (new helper over the backend state's installed
  config; never from the request):
  - `Free` → allowed, local or cloud.
  - `Paid` or `Unknown` → refused up front, naming the cost and
    the reason (one permit authorizes one dispatch; a comparison
    needs up to N dispatches). This applies regardless of
    locality — a local billable model has the same single-use
    problem.
  - A supplied `permit_id` → refused up front ("Compaction
    comparisons do not use single-use permits"), because
    accepting it would waste it on the first dispatch and fail
    the rest. The run never sends a permit.
  - `local_only` (cloud unchecked) + cloud locality → refused
    (unchanged).
  - Cloud + Free → allowed; the dispatch sets
    `explicitCloudAuthorization: true` from the approved cloud
    flag (not from a permit), recording the user's explicit
    consent to send this history to the cloud provider.
  - Zero-Cost Lock is NOT relaxed: the lock still runs per
    dispatch inside `authorize_cost`; free passes it, billable
    never reaches it (refused at start).
- **D4 — The resolved target is reported, not assumed.** The
  start and poll responses echo the resolved provider, model,
  locality, and cost class (poll already echoes the first three;
  cost is added). The UI shows the resolved target in the run
  results ("Ran against ollama / llama-x · local · free"). The
  saved experiment format stays v1 and is unchanged.
- **D5 — No new cloud calls for verification.** All cloud-path
  tests use a local HTTP fake. The fake is configured as
  provider `openrouter` (catalog locality Cloud,
  `catalog.rs:324` OpenAI-compatible adapter, POST
  `/chat/completions`, `choices[0].message.content`) with a
  loopback base URL declared `Local` — which passes
  `enforce_derived_locality` (`provider_dispatch.rs:727`,
  loopback → Local) — while the compaction module derives Cloud
  from the catalog id. The fake answers the verbatim-extractor
  way (expected answer iff the history contains it verbatim,
  else `UNKNOWN`) and counts hits per provider. No real cloud
  endpoint is contacted, ever.
- **D6 — Known limit (documented, not hidden).** Approval binds
  the selection as specified. A blank model is a reference to
  "first configured at start time": if the configuration changes
  between approval and start, the run uses the then-first model
  and reports it in the results. The run's recorded target is
  the authority for what ran. A future `providers-configured-
  models` read op would let the UI resolve at approve time and
  bind the concrete model; that is a follow-up, not this slice.

## §3 Acceptance criteria

- **AC-A1** Blank Model ID resolves. With provider `ollama` +
  blank model and two configured models (`llama-x` first,
  `aaa-decoy` second), the UI flow (model field left blank)
  starts and completes against `ollama` / `llama-x`; every
  dispatch body the fake receives carries model `llama-x`; the
  poll echoes provider `ollama`, model `llama-x`.
- **AC-A2** Refusals preserved. Blank provider + explicit model
  is refused; no configured model at all is refused with the
  existing message; an explicit unconfigured model is refused.
  All before traffic.
- **AC-A3** Free cloud runs. Provider `openrouter`, free model,
  cloud checked, against the OpenAI-compatible fake: the run
  completes, both arms dispatch to the fake with the exact
  selected provider/model, no permit is involved, and the poll
  reports locality `cloud`, cost `free`.
- **AC-A4** Billable targets refused before traffic. A configured
  model with cost `Paid` (local or cloud) → start refuses (400)
  with the free-only message; fake hit count 0. Same for cost
  `Unknown`. A supplied `permit_id` → refused before traffic,
  hit count 0.
- **AC-A5** Approval invalidation is server-side. A start whose
  `approvalHash` was computed over different questions, a
  different target, a different budget, or a different candidate
  than the request carries → 400, zero dispatches (one tamper
  per test: questions / target / budget / candidate).
- **AC-A6** Pinning holds. With two configured models, all
  dispatch bodies carry the exact selected provider/model
  (fake records them); a replayed approval with a changed target
  is refused by AC-A5.
- **AC-A7** Zero-traffic proofs. Missing authorization
  (billable cloud, no permit) → 0 hits; stale approval (AC-A5)
  → 0 hits; disallowed target (AC-A4) → 0 hits. Each asserted
  on the fake's hit counter.
- **AC-A8** Frozen benchmark and scorer preserved. The 40-case
  benchmark still passes through the app with the synthetic
  provider (same 21/50 synthetic result); no change to
  `crates/core/src/somnia_compaction.rs`, the fixtures, or the
  scorer.
- **AC-A9** Regressions green (Linux): Rust lib, tsc, contracts
  `--check`, docs, full UI suite, compaction e2e, provider e2e
  (`providers.spec.ts`), LARP scene 09, SOMNIA slices 1–3,
  model comparison, SwarmTank AI. The pre-existing
  `larp-somnia.spec.ts` instruments failure (proven on the
  pristine `4e4e970a` baseline) is reported as-is, not fixed.
- **AC-A10** No new cloud calls, no real credentials, no paid
  inference. All provider traffic in tests goes to loopback
  fakes; the free-cloud fake is loopback too.

## §4 Tests (planned)

- e2e `somnia-compaction.spec.ts`: blank-model UI run (AC-A1);
  free-cloud run through the OpenAI-compatible fake (AC-A3);
  billable/unknown refusal + supplied-permit refusal with hit
  counts (AC-A4, AC-A7); approval-tamper API tests, one field
  at a time (AC-A5); dispatch-body model recording in the fake
  for pinning (AC-A6).
- Rust unit tests in `somnia_compaction_run.rs`: blank-model
  resolution (first-for-provider, none-configured, explicit
  unconfigured), approval-hash recompute + tamper, free-only
  gate (free local/cloud allowed, paid/unknown refused, permit
  refused).
- UI tests in `somnia-compaction.test.tsx`: approval hash
  changes when any bound field changes; approval clears on
  target/budget/question edits; the resolved target is shown
  from the poll.
- Existing suites unchanged in shape: benchmark spec still
  asserts the same synthetic numbers (AC-A8).

## §5 The missing contract (design note, not implemented)

A billable multi-dispatch comparison needs a run-scoped permit
the current API does not have. Such a contract would need, at
minimum: scope bound to one run id + one provider/model; a call
cap (≤100) and a token cap (≤ the approved `maxTotalTokens`)
fixed at issue time; a TTL covering the run's lifetime with
expiry enforced server-side; consumption once per run (not per
dispatch) with the run id recorded in the used set; revocability
by the owner before the run starts; and disclosure of the permit
id, caps, and expiry in the review. Issuing it must stay an
owner-only explicit action, and the UI must show the caps before
approval. This slice does not implement it: the free-only gate
(§2 D3) is the safe subset the existing API already supports.

## §6 Verification plan

Rust workspace lib, tsc, contracts `--check`, docs validator,
full `test:ui`, the compaction e2e specs, `providers.spec.ts`,
`larp-somnia.spec.ts`, SOMNIA slices 1–3, model comparison,
SwarmTank AI, documents/PDF regressions. Package as a
checksummed bundle ZIP with MANIFEST + evidence + scratch
verification, delivered with the ZIP attached. Stop at this
slice.
