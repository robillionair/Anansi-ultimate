# SOMNIA compaction mid-run target — design + acceptance criteria

Branch: `muse/somnia-compaction-midrun-target`, from the authorization
repair tip `2d86ccb3394572d9b23df27b9673ded34280f2cf`.
Design + AC committed BEFORE implementation.

## §1 Defect

The repair slice binds the approved target at `start`: `start_op`
resolves the exact target, re-derives endpoint/locality/cost from the
live configuration, and recomputes the approval hash before spawning
the run thread. But each arm then dispatches through
`providers-dispatch`, which resolves the runtime from the CURRENT
backend configuration at call time. If the configuration is replaced
between the two arms of a pair — same provider and model IDs, new
`baseUrl`, via owner `providers-configure` — the second arm dispatches
to the replacement endpoint. The disclosure check only compares
provider/model IDs, so the misdirection is silent: the pair completes
with a compacted reply from an endpoint the user never approved.

This is a time-of-check/time-of-use gap, not a hash gap: the hash is
checked once, the runtime is resolved per dispatch.

## §2 Decisions

- **D1 Per-dispatch atomic validation (chosen over a runtime
  snapshot).** A snapshot would bypass `providers-dispatch` and force
  the compaction module to reimplement its policy (model enforcement,
  permit handling, privacy gating, disclosure, Zero-Cost Lock). Instead
  the dispatch request carries the approved target, and phase 1 of
  `providers-dispatch` verifies it against the live configuration and
  acquires the runtime in the SAME locked section. The network call
  stays in phase 2, unlocked. Callers that do not send the expected
  target are unaffected.
- **D2 Expected-target shape.** `ProviderDispatchRequest` gains
  optional `expectedTarget: {provider, model, endpoint, locality,
  cost}` (camelCase, `deny_unknown_fields` preserved, absent → no
  check). Cost uses the existing `CostClass` (`"free"` / `"paid"` /
  `"unknown"`). When `chosen` is present it must equal the expected
  provider/model, so a caller cannot validate one target while
  dispatching another. The request's doc comment ("carries no
  configuration…") is updated: the expected target is a binding the
  server verifies against backend state, never trusted configuration.
- **D3 Live derivation mirrors `target_for` exactly.** Endpoint is the
  configured `baseUrl` trimmed, falling back to the catalog default
  when blank; locality is catalog-derived; cost is the configured
  model's cost class. The locality derivation becomes one shared
  helper in `provider_dispatch.rs` (the run module's private copy is
  removed) so the two can never drift. The check reads
  `state.config` directly inside the phase-1 closure — it must NOT
  call `configured_model_target`, which takes the backend lock itself.
- **D4 First arm preserved, run ends.** If either arm fails and the
  live target no longer matches the approved target, the run records
  the pair with the original reply/score/privacy preserved, the
  compacted arm failed, and ends with terminal status
  `"target-changed"` — no further pair is attempted. A failure with
  the target still matching keeps the existing behaviour (pair
  recorded as failed, run continues). The status is a free string in
  the poll response; the UI already renders unknown terminal statuses
  opaquely and only offers Save when status is `complete`, so a
  target-changed run cannot be saved as an experiment.
- **D5 Gated pause (test-only).** After the original arm of the FIRST
  pair, if `ANANSI_SOMNIA_COMPACTION_TEST_PAUSE_AFTER_FIRST_ARM`
  names a release-file path, the run waits for that file (poll every
  10 ms, bounded 120 s, aborts early if the run is cancelled). The
  hook is inert unless the variable is set; the timeout fails the
  pair cleanly instead of hanging the run thread.
- **D6 Test level.** The e2e API session cannot call
  `providers-configure` (403: owner-only; an API session can never
  grant itself, verified 2026-11-09), so the mid-run reconfiguration
  cannot be driven through the browser/e2e harness. The gated test is
  therefore a Rust integration test
  (`crates/core/tests/somnia_compaction_midrun_target.rs`) driving
  `Core::ai_command` in-process with a host scope, two hit-counting
  loopback Ollama fakes (adapted from
  `crates/core/tests/provider_dispatch.rs`), and the same
  prepare → resolve → start → poll flow. It needs `sha2` as a
  dev-dependency for the approval-hash mirror (the e2e TS mirror
  pattern, applied in Rust).
- **D7 Unchanged.** Privacy per arm, Zero-Cost Lock, the free-only
  gate, permit refusals, sealed exclusion, the frozen benchmark and
  scorer, and the start-time hash check. The dispatch still goes
  through `providers-dispatch` with the same request (privacy
  segment, `zeroCostLock: true`, no permit).

## §3 Acceptance criteria

- **AC-M1** Per-dispatch validation. Each compaction arm's dispatch
  carries the approved target; phase 1 verifies it against the live
  configuration and acquires the runtime atomically in the same
  locked section; a mismatch refuses (400) before any provider
  traffic.
- **AC-M2** Mid-run endpoint change. The gated test pauses after the
  first arm, reconfigures the endpoint (same provider/model IDs) via
  owner configure, releases; the replacement endpoint receives zero
  requests; the run ends `"target-changed"`; the first arm's result
  (reply, correct score) is preserved in the recorded pair.
- **AC-M3** Lock discipline. The backend lock is never held across
  network I/O (phase 2 unchanged); validation + runtime acquisition
  happen in one locked section.
- **AC-M4** First arm preserved + gates intact. The recorded pair
  keeps the original reply/score/privacy; privacy labels per arm
  unchanged; Zero-Cost Lock still enforced per dispatch; no permit
  used.
- **AC-M5** No silent continuation. After a target-mismatch failure
  the run stops; no further pair is attempted against the changed
  target.
- **AC-M6** Unchanged behaviour otherwise. Callers without
  `expectedTarget` are unaffected; the successful local workflow, the
  benchmark numbers, and the existing 16 compaction e2e tests still
  pass.
- **AC-M7** Focused checks green (Linux): Rust lib (including the new
  integration test), tsc, contracts `--check`, docs, full UI suite,
  compaction e2e, benchmark, LARP scene 09. The pre-existing
  `providers.spec.ts` timeout and `larp-somnia.spec.ts` instruments
  failure are reported as-is, not fixed.
- **AC-M8** No live cloud calls, no real credentials, no paid
  inference. All provider traffic in tests goes to loopback fakes.

## §4 Tests (planned)

- New integration test `midrun_endpoint_change_gets_zero_requests`:
  temp Core + workspace + project; fake A configured (host); prepare
  one case (r1 superseded by r2, standard privacy; question "What is
  the customs fee now?", expected answer "core reply"); resolve
  target; approval hash over the resolved values; set the pause env
  var to a release path in the temp dir; start; wait for fake A hits
  == 1; reconfigure (host) to fake B with the same provider/model
  IDs; create the release file; poll to terminal. Assert: fake B
  hits == 0, fake A hits == 1, status `"target-changed"`,
  one pair with `originalReply == "core reply"`,
  `originalScore == "correct"`, `compactedReply` absent,
  `compactedScore == "failed"`.
- Before-evidence: the same test on unfixed code must fail with fake
  B hits == 1 (the compacted arm dispatched to the replacement
  endpoint).
- Rust unit tests for the live-target derivation helper if factored
  out separately.
