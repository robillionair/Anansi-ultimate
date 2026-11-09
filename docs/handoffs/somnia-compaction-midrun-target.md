# SOMNIA compaction mid-run target — handoff

Branch: `muse/somnia-compaction-midrun-target`, from the authorization
repair tip `2d86ccb3394572d9b23df27b9673ded34280f2cf`.
Design + AC committed first as `96c4ac247`; gated reproduction as
`2fbf36adc`; implementation as `07fd7e467`. This handoff is committed
after them; the ZIP MANIFEST is authoritative for the delivered tip.

Scope: the time-of-check/time-of-use gap the repair slice left open —
the approval hash is checked once at `start`, but each arm resolves
its runtime from the live configuration at dispatch time, so a
configuration change between the two arms could silently redirect the
second arm. Nothing else.

## §1 What changed

### Per-dispatch atomic validation (AC-M1, AC-M3)

- `ProviderDispatchRequest` gains optional `expectedTarget:
  {provider, model, endpoint, locality, cost}` (`CostClass`,
  camelCase, absent → no check). The request's doc comment now states
  the exception: the expected target is a binding the server verifies
  against backend state, never trusted configuration.
- Phase 1 of `providers-dispatch` verifies the expected target
  against the live configuration in the SAME locked section that
  acquires the runtime, before any traffic: the `chosen` provider/
  model must match the expected ones; the live endpoint (configured
  `baseUrl` trimmed, catalog default when blank), catalog-derived
  locality, and configured cost class must match exactly. A mismatch
  refuses (400). The network call stays in phase 2, unlocked — the
  backend lock is never held across network I/O. Callers that do not
  send `expectedTarget` are unaffected (all 18 existing
  provider-dispatch integration tests pass unchanged).
- The live derivation mirrors `target_for` exactly. The locality
  derivation is now one shared helper in `provider_dispatch.rs`
  (`catalog_locality`); the run module's private copy is removed. The
  endpoint/cost lookup is factored into `target_in_config` (shared by
  `configured_model_target` and the check) so the check reads
  `state.config` directly inside the locked section — it never calls
  `configured_model_target`, which would re-take the backend lock.
- `RunConfig` carries the approved endpoint/locality/cost from
  `start_op`'s resolved target; every compaction arm sends them as
  its expected target. Privacy per arm, Zero-Cost Lock, the free-only
  gate, permit refusals, and sealed exclusion are untouched — the
  dispatch still goes through `providers-dispatch` with the same
  request (privacy segment, `zeroCostLock: true`, no permit).

### First arm preserved, run ends (AC-M2, AC-M4, AC-M5)

- After a failed arm, the run re-derives the live target
  (`approved_target_matches`, same derivation as start; a removed
  target fails closed). If it no longer matches the approved target,
  the pair is recorded with the original reply/score/privacy
  preserved and the compacted arm failed, and the run ends with
  terminal status `"target-changed"` — no further pair is attempted.
  A failure with the target still matching keeps the existing
  behaviour (pair recorded as failed, run continues). The status is a
  free string; the UI renders unknown terminal statuses opaquely and
  only offers Save when status is `complete`, so a target-changed run
  cannot be saved as an experiment.

### Gated test pause (test-only)

- After the original arm of the first pair, if
  `ANANSI_SOMNIA_COMPACTION_TEST_PAUSE_AFTER_FIRST_ARM` names a
  release-file path, the run waits for that file (poll every 10 ms,
  bounded 120 s, aborts early on cancel; a timeout cancels the run
  and the existing cancel path records the pair). Inert unless set.

### Why the test is a Rust integration test

The e2e API session cannot call `providers-configure` (403,
owner-only; an API session can never grant itself — verified
2026-11-09), so the mid-run reconfiguration cannot be driven through
the browser/e2e harness. The gated test
(`crates/core/tests/somnia_compaction_midrun_target.rs`) drives
`Core::ai_command` in-process with a host scope, two hit-counting
loopback Ollama fakes, and the same prepare → resolve → start → poll
flow. It needed `sha2` as a dev-dependency for the approval-hash
mirror (the e2e TS mirror pattern, applied in Rust; the mirror uses
ordered structs because a `json!` map would sort keys alphabetically
and hash a different string).

## §2 Acceptance criteria — status

- **AC-M1** PASS. Each arm carries the approved target; phase 1
  verifies it and acquires the runtime atomically; a mismatch
  refuses (400) before traffic.
- **AC-M2** PASS. Gated test: pause after the first arm, reconfigure
  to a new endpoint with the same provider/model IDs, release —
  replacement endpoint 0 requests, approved endpoint exactly 1, run
  ends `"target-changed"`, the pair keeps `originalReply "core
  reply"`, `originalScore "correct"`, `compactedReply` absent,
  `compactedScore "failed"`.
- **AC-M3** PASS. Lock discipline unchanged (phase 2 unlocked);
  validation + acquisition share one locked section.
- **AC-M4** PASS. First arm preserved; privacy labels and Zero-Cost
  Lock unchanged (existing 16 e2e tests + benchmark green).
- **AC-M5** PASS. The run stops after the target-mismatch failure;
  no further pair is attempted (single-pair test; the loop breaks on
  `target-changed`).
- **AC-M6** PASS. Callers without `expectedTarget` unaffected
  (provider-dispatch 18/18); compaction e2e 16/16; benchmark
  identical.
- **AC-M7** PASS (Linux) except the two pre-existing failures
  reported as-is below.
- **AC-M8** PASS. All provider traffic in tests goes to loopback
  fakes; no live cloud calls, no real credentials, no paid inference.

## §3 Verification (Linux, 2026-11-09)

Before evidence (pause hook only, no validation fix): the gated test
FAILED with `replacement endpoint received requests; left: 1,
right: 0` — the compacted arm dispatched to the replacement endpoint.
Saved at `deliveries/somnia-compaction-midrun-target/evidence/before-test-reproduction.md`.

After (implementation commit `07fd7e467`, full evidence at
`deliveries/somnia-compaction-midrun-target/evidence/after-checks.md`):

- Gated test `midrun_endpoint_change_gets_zero_requests`: **1/1**.
- `cargo test --locked --workspace --lib`: **345/345** (api 4, core
  278, desktop 1, providers 62).
- `cargo test --locked -p anansi-core --test provider_dispatch`:
  **18/18**.
- `node scripts/contracts.mjs --check`: exit 0 (pre-existing
  "unknown format uint" warnings only).
- `pnpm typecheck`: exit 0. `pnpm test:ui`: **337/337**.
- `node scripts/validate-docs.mjs`: PASS, 330 files (329 before the
  handoff was added).
- Compaction e2e `somnia-compaction.spec.ts`: **16/16**.
- Benchmark: 1/1, synthetic numbers identical (21/50, 21/50, 0
  failed, 1600 tokens).
- `larp-somnia.spec.ts`: 1 passed, 1 failed — the instruments test,
  the known pre-existing failure (locator.click timeout on 'Load
  built-in cases', LARP overlay), reproduced identically on pristine
  `4e4e970a` in Checkpoint 2. Not a regression; not fixed in this
  slice.
- `providers.spec.ts` was not re-run in this slice; its
  single-workflow 240 s timeout is the known pre-existing failure
  reproduced identically on pristine `cf69e37e`. It is in the Windows
  rerun checklist below.
- `rustfmt --edition 2024` applied to touched files only.

## §4 Known limits

- The pause hook is test-only and env-gated; a timeout cancels the
  run (the pair is recorded by the existing cancel path).
- The expected-target check covers unary `providers-dispatch` only;
  no compaction flow uses stream-start.
- A target change between approval and start is still caught by the
  start-time hash check (repair slice); this slice closes the
  change-between-arms gap.
- AC-I10 live rerun still BLOCKED on this VM (no local model, no
  provider credential); rerun path unchanged via
  `ANANSI_SOMNIA_LIVE_ENDPOINT` / `ANANSI_SOMNIA_LIVE_MODEL`.

## §5 Windows rerun commands

From the repo root, after extracting the bundle and checking out the
branch:

```sh
node scripts/cargo.mjs test --locked --workspace --lib
# expect 345/345 (api 4, core 278, desktop 1, providers 62)

node scripts/cargo.mjs test --locked -p anansi-core --test somnia_compaction_midrun_target
# expect 1/1: midrun_endpoint_change_gets_zero_requests

node scripts/cargo.mjs test --locked -p anansi-core --test provider_dispatch
# expect 18/18

node scripts/contracts.mjs --check
# expect exit 0

pnpm typecheck
# expect exit 0

pnpm test:ui
# expect 337/337

node scripts/validate-docs.mjs
# expect PASS, 330 files
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
