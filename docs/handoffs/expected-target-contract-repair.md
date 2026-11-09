# Expected-target contract repair — handoff

Branch: `muse/expected-target-contract-repair`, from the mid-run
target tip `a453880304a987703eb9ecc74917949f0da84e26`.
Design + AC committed first as `9aa25fe70`; gated reproduction as
`69fa161d0`; implementation as `dcdb3ddb6`. This handoff is
committed after them; the ZIP MANIFEST is authoritative for the
delivered tip.

Scope: one shared dispatch contract repair — the two gaps the
mid-run slice left in `expectedTarget` handling. Nothing else.

## §1 What changed

### Gap 1 — unary: the binding was vacuous under automatic routing

`Router::plan` honors `chosen` only in `ChooseModel` mode; in
every other mode it ignores the field, admits every configured
model, and orders an automatic chain the dispatcher walks. The
mid-run slice's check only verified a present `chosen` matched the
expected target — with `chosen` absent (or with any non-chooseModel
mode) the dispatch could go to a different model while the check
passed.

- `enforce_expected_target` now takes the parsed `RoutingMode` and
  refuses (400, in the same locked phase-1 section, before
  `take_permit` / `runtime_for`, before any traffic) unless ALL
  hold: mode is `chooseModel`; `chosen` is present; `chosen`
  equals the expected provider/model; the live config still matches
  endpoint/locality/cost. The mode check runs first. The network
  call stays in unlocked phase 2.
- The request DTO is unchanged; its doc comment now states the
  per-op rules where the shape is defined.

### Gap 2 — stream-start: the field was parsed and silently ignored

`providers-stream-start` never called `enforce_expected_target`,
so a stream-start request could carry `expectedTarget` and the
stream would dispatch to whatever the router picked.

- `providers-stream-start` now refuses any request carrying
  `expectedTarget` (400, before traffic, before the locked phase-1
  section — a request-shape refusal needs no backend state). Even
  a *matching* expected target is refused: the op does not enforce
  the binding, so accepting it would be a lie. No existing caller
  sends the field (collider, forge, ideas, opportunities, and all
  TS callers — verified by repo-wide search), so nothing breaks.
  A future approval-bound stream is a new slice: enforce it the
  same way unary dispatch does, never accept it silently.

### Test support

- `FakeOllama` in `crates/core/tests/provider_dispatch.rs` now
  also records the model id each request names (parsed from the
  `"model":"..."` body field the Ollama adapter sends), so a test
  proves WHICH model a dispatch reached. No existing fake
  behaviour changed. Duplicate provider registrations are not an
  option for a two-server decoy: `ProviderRuntime::new` refuses
  `duplicate provider in configuration`, so the gap-1 test uses
  two models (`aaa-decoy`, `llama-x`) on one provider — auto mode
  orders `aaa-decoy` first.

## §2 Acceptance criteria — status

- **AC-C1** PASS. Unary + expectedTarget + mode auto (no chosen):
  refused 400, 0 hits, no model recorded. Before the fix the
  dispatch succeeded and the fake recorded `aaa-decoy`.
- **AC-C2** PASS. Unary + expectedTarget + mode auto + chosen
  present and matching: refused 400, 0 hits. Before the fix it
  also reached `aaa-decoy` — `chosen` is ignored in auto mode.
- **AC-C3** PASS. Unary + expectedTarget + chooseModel + chosen
  mismatched: refused 400, 0 hits.
- **AC-C4** PASS. Unary + expectedTarget + chooseModel + chosen
  matching + live config matching: succeeds; the fake records
  `llama-x`; disclosure names ollama/llama-x. (Positive control;
  this is exactly SOMNIA's dispatch shape.)
- **AC-C5** PASS. Stream-start + expectedTarget (mismatched
  endpoint, then matching endpoint): both refused 400, 0 hits on
  both fakes. Before the fix the mismatched stream started and the
  configured fake got the traffic. Stream-start without the field
  still starts (positive control).
- **AC-C6** PASS. Callers that omit `expectedTarget` are
  unaffected: all 18 pre-existing provider-dispatch tests pass.
- **AC-C7** PASS. SOMNIA's atomic check preserved: gated mid-run
  test 1/1 (first arm retained, replacement endpoint 0 requests);
  compaction e2e 16/16; benchmark synthetic identical.
- **AC-C8** PASS (Linux) except the two pre-existing failures
  reported as-is below.

## §3 Verification (Linux, 2026-11-09)

Before evidence (reproduction tests only, no fix): 1 passed,
3 failed — see `deliveries/expected-target-contract-repair/evidence/before-test-reproduction.md`.

After (implementation commit `dcdb3ddb6`, full evidence at
`deliveries/expected-target-contract-repair/evidence/after-checks.md`):

- `cargo test --locked -p anansi-core --test provider_dispatch`:
  **22/22** (18 pre-existing + 4 new).
- Gated mid-run test
  `midrun_endpoint_change_gets_zero_requests`: **1/1**.
- `cargo test --locked --workspace --lib`: **345/345** (api 4, core
  278, desktop 1, providers 62).
- `node scripts/contracts.mjs --check`: exit 0 (pre-existing
  "unknown format uint" warnings only). Regeneration changed only
  the `expectedTarget` description text; no shape change.
- `pnpm typecheck`: exit 0. `pnpm test:ui`: **337/337**.
- `node scripts/validate-docs.mjs`: PASS, 332 files (333 once this
  handoff was added).
- Compaction e2e `somnia-compaction.spec.ts`: **16/16**.
- Benchmark: 1/1, synthetic numbers identical (21/50, 21/50, 0
  failed, 1600 tokens).
- `larp-somnia.spec.ts`: 1 passed, 1 failed — the instruments test,
  the known pre-existing failure (locator.click timeout on 'Load
  built-in cases', LARP overlay), reproduced identically on pristine
  `4e4e970a` in Checkpoint 2. Not a regression; not fixed in this
  slice.
- `providers.spec.ts`: 1 failed — the single provider workflow
  test, the known pre-existing 240 s timeout, reproduced
  identically on pristine `cf69e37e`. Not a regression; not fixed
  in this slice.
- `rustfmt --edition 2024` applied to touched files only.

## §4 Known limits

- A future caller that needs an approval-bound stream must
  implement enforcement on stream-start (D2); the field is
  refused there today by design.
- AC-I10 live rerun still BLOCKED on this VM (no local model, no
  provider credential); rerun path unchanged via
  `ANANSI_SOMNIA_LIVE_ENDPOINT` / `ANANSI_SOMNIA_LIVE_MODEL`.

## §5 Windows rerun commands

From the repo root, after extracting the bundle and checking out the
branch:

```sh
node scripts/cargo.mjs test --locked --workspace --lib
# expect 345/345 (api 4, core 278, desktop 1, providers 62)

node scripts/cargo.mjs test --locked -p anansi-core --test provider_dispatch
# expect 22/22 (18 pre-existing + 4 new expected-target tests)

node scripts/cargo.mjs test --locked -p anansi-core --test somnia_compaction_midrun_target
# expect 1/1: midrun_endpoint_change_gets_zero_requests

node scripts/contracts.mjs --check
# expect exit 0

pnpm typecheck
# expect exit 0

pnpm test:ui
# expect 337/337

node scripts/validate-docs.mjs
# expect PASS, 332 files
```

Then rebuild the API and the web app, and run e2e one spec per
invocation with the lease script:

```sh
node scripts/cargo.mjs build --locked -p anansi-api
cd apps/web && node node_modules/vite/bin/vite.js build
cd ~/workspace/Anansi-ultimate

node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/somnia-compaction.spec.ts
# expect 16/16

node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/somnia-compaction-benchmark.spec.ts
# expect 1/1, synthetic 21/50, 21/50, 0 failed, 1600 tokens

node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/providers.spec.ts
# pre-existing 240 s timeout expected on Linux; report the Windows outcome as-is

node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/larp-somnia.spec.ts
# expect 1 pass; the instruments test is the known pre-existing failure; report as-is
```

If a local model is available, run the AC-I10 live rerun via
`ANANSI_SOMNIA_LIVE_ENDPOINT` / `ANANSI_SOMNIA_LIVE_MODEL`.

## §6 Proposed global-status updates

NONE. AN-015 stays `verified`. M13 is not complete; AN-025 stays
planned. Antigravity owns the global status/release documents; no
files under that ownership were edited.
