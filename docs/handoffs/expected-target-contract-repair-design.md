# Expected-target contract repair — design + acceptance criteria

Branch: `muse/expected-target-contract-repair`, from the mid-run
target tip `a453880304a987703eb9ecc74917949f0da84e26`.
This document is committed BEFORE implementation.

## §1 The two gaps (both verified in code before writing this)

### Gap 1 — unary: `expectedTarget` does not bind automatic routing

`Router::plan` (`crates/providers/src/routing.rs`) honors
`chosen` ONLY when `mode == ChooseModel`. In every other mode
(Auto, AutoFree, Private, Fast, Deep, Coding, Vision) `chosen` is
ignored: the router admits every configured model, orders the chain
(by mode tier, preference, provider id, model id), and the dispatcher
walks it, binding the wire request to the FIRST candidate's model.
`enforce_expected_target` (mid-run slice) only checks that a
present `chosen` equals the expected provider/model — with `chosen`
absent it verifies the expected target is configured, then the
dispatch can still go to a different model. The binding is vacuous
whenever the caller does not pin `chooseModel` + `chosen`.

Concrete shape: one ollama provider, models `aaa-decoy` and
`llama-x`, mode `auto`, `expectedTarget` = ollama/llama-x at the
configured endpoint, no `chosen`. Auto mode orders `aaa-decoy`
first; the dispatch goes to the decoy while the expected-target
check passes. Duplicate provider registrations are NOT an escape
hatch: `ProviderRuntime::new` refuses `duplicate provider in
configuration`, so the decoy must be a second model on the same
provider, distinguished by the model the fake server records.

### Gap 2 — stream-start: the field is parsed and silently ignored

`providers-dispatch` and `providers-stream-start` share one request
shape (`ProviderDispatchRequest`), so a stream-start request can
carry `expectedTarget` — but the stream-start arm never calls
`enforce_expected_target`. The field is accepted, never validated,
and the stream dispatches to whatever the router picks. A caller
that believes it bound a stream to an approved target is wrong.

No existing caller requires streaming support for the field
(verified by repo-wide search — `expectedTarget` appears only in
`provider_dispatch.rs`, `somnia_compaction_run.rs`, and the generated
contracts): the stream-start callers are collider, forge, ideas, and
opportunities, none of which sends the field, and no TS caller does
either. So the repair refuses the field on stream-start rather than
enforcing it there.

## §2 Decisions

- **D1 — Unary rule.** When `expected_target` is present on
  `providers-dispatch`, phase 1 refuses (400, before any traffic)
  unless ALL hold: routing mode is `chooseModel`; `chosen` is
  present; `chosen` equals the expected provider/model (existing
  check, kept); the live config still matches endpoint/locality/cost
  (existing check, kept). The checks run in the same locked phase-1
  section as today, before `take_permit` / `runtime_for`; the
  network call stays in unlocked phase 2. `enforce_expected_target`
  gains the parsed `RoutingMode` (taken from the already-parsed
  `routing_request`, not a second parse); the mode check runs first.
- **D2 — Stream-start rule.** `providers-stream-start` refuses any
  request carrying `expectedTarget` (400, before traffic, before the
  locked phase-1 section — a request-shape refusal needs no backend
  state). Even a *matching* expected target is refused: the op does
  not enforce the binding, so accepting it would be a lie. If a
  future caller needs an approval-bound stream, that is a new slice:
  enforce it the same way (chooseModel + chosen + live check),
  never accept it silently.
- **D3 — No DTO/schema change.** The request shape is unchanged
  (`expectedTarget` is already in the schema); the change is per-op
  validation semantics. The DTO doc comment is updated to state
  both per-op rules where the shape is defined. Contracts
  regenerate with 0 drift expected.
- **D4 — Reproduction level.** Rust integration tests in
  `crates/core/tests/provider_dispatch.rs` (same harness as the
  mid-run test): host scope, hit-counting fakes. The e2e API
  session cannot configure providers (403, owner-only), and the
  gaps are server-side validation rules — no browser test can drive
  them. E2E regressions still run for the SOMNIA workflow, whose
  dispatch path is unchanged.
- **D5 — Gap-1 test mechanics.** One ollama provider, two models
  (`aaa-decoy`, `llama-x`), one fake. The fake records the
  requested model per hit (additive field on `FakeOllama`, parsed
  from the `"model":"..."` body field the Ollama adapter sends;
  no existing fake behaviour changes). Unfixed, the dispatch
  succeeds and the fake records `aaa-decoy`; fixed, it is refused
  with 0 hits and no model recorded.
- **D6 — Gap-2 test mechanics.** Two fakes: fake B is the
  configured provider (llama-x); fake A is a second server whose
  URL is used as the (mismatched) expected endpoint. Unfixed, the
  stream starts and fake B gets hits; fixed, refused with 0 hits
  on both. A matching expected target is also refused in the same
  test, and a stream without the field still starts (positive
  control).
- **D7 — Preservation.** SOMNIA's dispatch (chooseModel + chosen +
  expectedTarget) is unchanged and still passes; the gated mid-run
  endpoint-swap regression is re-run and must still pass with the
  first arm retained and 0 requests to the replacement endpoint.
  Callers that omit `expectedTarget` are unaffected, as are callers
  that send `chosen` without it (ChooseModel path unchanged) and
  callers that send mode auto without it.

## §3 Acceptance criteria

- **AC-C1** PASS. Unary + expectedTarget + mode auto (no chosen):
  refused 400 before traffic; 0 hits on all fakes; no model
  recorded. (On unfixed code the dispatch succeeds to `aaa-decoy`.)
- **AC-C2** PASS. Unary + expectedTarget + mode auto + chosen
  present and matching: refused 400 before traffic; 0 hits.
  (`chosen` is ignored in Auto mode, so its presence cannot rescue
  automatic routing.)
- **AC-C3** PASS. Unary + expectedTarget + chooseModel + chosen
  present but mismatched: refused 400 before traffic; 0 hits.
- **AC-C4** PASS. Unary + expectedTarget + chooseModel + chosen
  matching + live config matching: succeeds; the fake records
  model `llama-x`; the disclosure names ollama/llama-x. (Positive
  control; this is exactly SOMNIA's dispatch shape.)
- **AC-C5** PASS. Stream-start + expectedTarget (mismatched
  endpoint, then matching endpoint): both refused 400 before
  traffic; 0 hits on both fakes. Stream-start without the field
  still starts and the fake gets hits.
- **AC-C6** PASS. Callers that omit `expectedTarget` are
  unaffected: all pre-existing provider-dispatch integration tests
  pass unchanged.
- **AC-C7** PASS. SOMNIA's atomic check preserved: the gated
  mid-run endpoint-swap regression passes (first arm retained,
  replacement endpoint 0 requests); compaction e2e 16/16;
  benchmark synthetic identical.
- **AC-C8** PASS. Contracts regenerate with 0 drift; provider,
  SOMNIA, and browser regressions green on Linux, with the two
  known pre-existing failures (providers.spec.ts 240 s timeout,
  larp-somnia instruments test) reported as-is, never fixed in
  this slice.

## §4 Out of scope

- No new features, no new dispatch modes, no streaming enforcement
  of expectedTarget (D2), no changes to the router's ordering or
  the dispatcher's fallback walk, no UI/TS changes, no live cloud
  calls, no paid inference, no pushes, no promotion.
