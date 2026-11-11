# ChatGPT sign-in token lifecycle repair — handoff

Branch: `muse/connect-ai-chatgpt-signin-lifecycle-repair`
Base: ChatGPT sign-in spike tip `5026f29f811d164431993e2355e1e0003b9fcbf4`
Tip: the handoff commit on this branch (exact SHA in the delivery
message); last code commit `a85e7ea80`
Date: 2026-11-11 (Linux evidence only — **WINDOWS REVIEW OUTSTANDING**)
Design (committed before implementation):
`docs/handoffs/connect-ai-chatgpt-signin-lifecycle-design.md`
(`5654b2786`, AC-L1..L8)

## 1. What was repaired

Four defects in the delivered spike tip, all in the token lifecycle
around `chatgpt-signin-disconnect`:

- **D1.** Keyring deletion failure was swallowed (`let _ =`) and
  disconnect returned `disconnected: true` anyway, while also
  clearing the registration stub's plan usage — so the caller was
  told the session was gone while the tokens stayed in the keyring.
- **D2.** A callback exchange in flight during disconnect stored its
  tokens afterwards, recreating the session.
- **D3.** A refresh in flight during disconnect stored its rotated
  tokens afterwards, recreating the session.
- **D4.** A callback delivered after disconnect still signed in —
  disconnect never touched the pending attempts.

Commits on the branch (base `5026f29f8`):

- `5654b2786` — design + acceptance criteria (AC-L1..L8) before code.
- `afbf0361b` — reproduction: 4 new tests fail on the delivered tip;
  existing 21 still pass. Test-only harness: a shared-mock keyring
  builder (one `MockCredential` per `(service, user)` plus a
  persistent `fail_delete` flag) and a token gate in the synthetic
  issuer.
- `a85e7ea80` — the fix + regenerated contracts (0 drift).

Files changed vs base (6 files, +696/−32): the design doc,
`crates/core/src/provider_dispatch.rs`, `crates/core/tests/chatgpt_signin.rs`,
regenerated contracts (`readiness.json`, `generated.ts`,
`validators.mjs`). No UI change; real account sign-in stays
dev-gated.

## 2. The fix (short form)

- **Per-account generation** (`chatgpt_generations` in
  `BackendState`, bumped only by disconnect, in-memory only).
  Disconnect bumps it and invalidates the account's
  `Pending`/`CallbackReceived` attempts (`consumed`, failed with
  reason `"disconnected"`) **before** any network call.
- **Callback and refresh** capture the generation before the
  exchange and store under a single backend-lock hold with the
  re-check. Mismatch ⇒ tokens discarded, attempt failed
  `"disconnected"`, refresh errors `"The ChatGPT session ended.
  Sign in again."`. The work under the lock is local-only
  (keyring + atomic file write), never network.
- **Disconnect response** gains `local_deletion` (`"deleted"` |
  `"absent"` | `"failed"`), kept separate from
  `remote_revocation_confirmed`. `disconnected` is true only when
  no local record remains. On `"failed"` the registration stub is
  left untouched, so status keeps showing the surviving session
  and a retry disconnect succeeds once the keyring fault clears.
- A late callback for an invalidated attempt now names the
  disconnect on its page instead of looking like a replay
  ("already used" is kept for genuine replays).

## 3. Before / after evidence

Reproduction run on the delivered tip (production code untouched,
only the design commit + new tests):

```
test result: FAILED. 21 passed; 4 failed
- disconnect_keyring_deletion_failure_...: left: Bool(true), right: false
  (disconnected reported true with the record still in the keyring)
- disconnect_during_callback_exchange_...: left: Null, right: "deleted"
  (no localDeletion field; the late store recreated the session)
- disconnect_during_refresh_...: left: Null, right: "deleted"
- late_callback_after_disconnect_is_refused:
  callback page: "Signed in with ChatGPT. ..." (after disconnect)
```

After the fix, same suite:

```
test result: ok. 25 passed; 0 failed
```

## 4. Evidence (Linux, this VM)

- Rust lib: **366 passed**, 0 failed (unchanged — no new lib tests).
- Integration `chatgpt_signin`: **25/25 passed** (21 baseline + 4 new).
- Integration `provider_dispatch` (regression): **39/39 passed**.
- TypeScript: `tsc --noEmit` 0 errors.
- Contracts: `node scripts/contracts.mjs --check` exit 0, 0 drift.
- Docs validation: **21/21 passed**.
- UI suite (`pnpm test:ui`): **342/342 passed**.
- Node suite (`pnpm test`): **39/40** — the one failure is the
  pre-existing MSVC installer test, confirmed identical on the
  spike base commit in the previous slice. Not a regression.
- e2e regressions (lease-run, one spec per invocation):
  connect-ai **1/1**, DOCX **7/7**, PDF **5/5**, TXT/MD **9/9**, Ask **1/1**.
- Warnings: one, pre-existing — `crates/core/src/mcp.rs:715`
  (`preserved_env_entries` never used). No warning from this slice.
- `rustfmt` applied to `provider_dispatch.rs` only (never to a
  crate root — see the `~/AGENTS.md` lesson from the spike slice).

## 5. Acceptance criteria — how each was met

- **AC-L1 (reproduce D1):** `disconnect_keyring_deletion_failure_is_reported_and_retry_succeeds`
  failed on the tip (`disconnected` true, stub plan usage false
  while the record survived). See §3.
- **AC-L2 (reproduce D2):** `disconnect_during_callback_exchange_does_not_recreate_session`
  failed on the tip (no `localDeletion` field; late store
  recreated the session). See §3.
- **AC-L3 (reproduce D3):** `disconnect_during_refresh_does_not_recreate_session`
  failed on the tip the same way. See §3.
- **AC-L4 (reproduce D4):** `late_callback_after_disconnect_is_refused`
  failed on the tip (callback page "Signed in with ChatGPT").
  See §3.
- **AC-L5 (truthful local status):** failed delete ⇒
  `disconnected: false`, `localDeletion: "failed"`,
  `remoteRevocationConfirmed: true` (revocation did succeed),
  status still signed in with plan usage, stub untouched; retry
  after clearing the fault ⇒ `"deleted"`, `disconnected: true`,
  status signed out; a further disconnect ⇒ `"absent"`,
  `remoteRevocationConfirmed: false`. All asserted in the retry
  test.
- **AC-L6 (pending work invalidated):** late callback (delivered
  after disconnect, or mid-exchange when disconnect lands) and
  late refresh store nothing: no keyring record remains, status
  signed out, attempt failed with reason `"disconnected"`, exactly
  one token exchange for the re-authorization / refresh (no retry,
  no second grant). Asserted in all three invalidation tests.
- **AC-L7 (scope):** no live OAuth, real credentials, paid calls,
  or generated-code execution; UI untouched and real sign-in stays
  dev-gated; API-key flow and local dispatch unaffected (existing
  tests green, e2e regressions green); contracts regenerated,
  0 drift.
- **AC-L8 (delivery):** this handoff + the checksummed bundle ZIP
  (see §7).

## 6. Known limits (recorded in the design, not fixed here)

- **Re-sign-in during a refresh.** Generation is bumped only by
  disconnect. If a user re-signs in as the same account while a
  refresh is in flight, the refresh's older rotated tokens can
  overwrite the fresher record. The tokens are still server-valid;
  no session is recreated and nothing is disconnected.
- **`InvalidGrant` refresh path.** Its `let _ = resolver.delete(...)`
  is unchanged: the op already errors ("session ended"), and a
  re-sign-in overwrites the stale record.
- **Refresh in flight + failed disconnect delete.** The refresh
  reports "session ended" (the disconnect's intent) even though the
  local record survives until a retry disconnect. Status remains
  the source of truth for the local record; no state gets stuck —
  the next refresh captures the current generation and works.
- No change to `chatgpt-signin-status`, the UI, or any global
  status document.

## 7. Windows rerun checklist

1. Apply the bundle ZIP on Windows; confirm the tip stated in the
   delivery message (handoff commit; last code commit `a85e7ea80`)
   and a clean tree.
2. Run the Rust battery: `cargo test --locked --workspace --lib`
   (expect 366), `--test provider_dispatch` (39/39),
   `--test chatgpt_signin` (25/25).
3. Run `tsc --noEmit` (0), contracts `--check` (0 drift),
   docs validation (21/21), UI suite (342/342).
4. Run the e2e regressions: connect-ai 1/1, DOCX 7/7, PDF 5/5,
   TXT/MD 9/9, Ask 1/1.
5. Manual (dev-gated) keyring check: after a real sign-in,
   disconnect once (record gone from Windows Credential Manager
   under the ChatGPT service, `localDeletion: "deleted"`), sign in
   again, then simulate a locked store if possible and confirm a
   failed delete reports `localDeletion: "failed"` with the record
   still present and a retry succeeding.
6. Manual invalidation check: start a re-authorization, disconnect
   mid-flow, and confirm the late callback is refused and no
   session is recreated.
7. **Do not** enable the sign-in button or attempt a live Responses
   turn: eligibility is still unverified (spike handoff §5) — this
   slice changes no eligibility fact.

## 8. Out of scope (unchanged)

- Any live OAuth, real credentials, paid calls, Responses turns, or
  model/route eligibility claims.
- Any change to the API-key flow, the frozen Windows pilot, or any
  provider op/contract except the `localDeletion` field on
  `ChatGptSigninDisconnectResponse`.
- `providers.spec.ts` 240 s timeout and the other pre-existing
  failures (larp-somnia instruments test, backups.spec.ts
  'Workspace' button, 4 semantic_workflow live-Ollama failures,
  node pwsh failure, MSVC installer test — all pre-existing on
  this VM).
- No global status document was edited (Antigravity-owned files
  untouched); proposed updates, if any, belong in this handoff only.
