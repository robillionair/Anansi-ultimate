# ChatGPT sign-in token lifecycle repair — design

Branch: `muse/connect-ai-chatgpt-signin-lifecycle-repair`
Base: ChatGPT sign-in spike tip `5026f29f811d164431993e2355e1e0003b9fcbf4`
Date: 2026-11-11 (Linux evidence only — **WINDOWS REVIEW OUTSTANDING**)

## 1. Defects (all in the delivered tip, all reproduced before the fix)

### D1. Keyring deletion failure is swallowed; disconnect lies about it

`chatgpt-signin-disconnect` does `let _ = resolver.delete(...)` and
returns `disconnected: true` unconditionally. If the OS keyring refuses
the delete, the tokens stay in the keyring while the caller is told the
session is gone. Worse, the op also sets the registration stub's
`plan_usage = false`, so the status labels disagree with each other:
the accounts list says no plan usage while the surviving record still
carries the plan scopes and `signed_in` stays true.

### D2. A late callback store recreates a disconnected session

`chatgpt_complete_callback` validates the attempt under the backend
lock, exchanges the code with **no lock held**, then stores the record
unconditionally. If disconnect runs during the exchange, the store
writes fresh tokens for the just-disconnected account. The attempt is
already consumed (`CallbackReceived`), so nothing re-checks whether the
session still exists.

### D3. A late refresh store recreates a disconnected session

`chatgpt-signin-refresh` has the same shape: exchange with no lock
held (only the global `chatgpt_refresh_in_flight` flag, which
disconnect does not consult), then `store_chatgpt_record`
unconditionally. Disconnect during the refresh is undone by the
refresh's own completion.

### D4. A callback delivered after disconnect still signs in

Disconnect does not touch `chatgpt_attempts`. A pending attempt for
the disconnected account survives it, so a late callback (browser
delivering the redirect after the user disconnected) validates,
exchanges, and stores — a full sign-in for a session the user just
ended.

## 2. Fix design

### 2.1 Per-account generation (epoch) in `BackendState`

```rust
chatgpt_generations: HashMap<String, u64>,
```

- Absent means generation 0. Bumped **only** by disconnect, for the
  disconnected `client_id`. In-memory only: in-flight exchanges do
  not survive a restart, so no persistence is needed.
- Disconnect also invalidates pending work for that account: every
  attempt with `attempt.client_id == <disconnected id>` in state
  `Pending` or `CallbackReceived` is marked `consumed = true`,
  `Failed("disconnected")`. Attempts for a new registration carry
  `client_id == dynamic_agent_client` (no issued id yet) and are
  left alone — disconnect cannot target an account that does not
  exist yet, and their store is still generation-checked against
  the issued id (fresh id ⇒ generation 0 ⇒ store proceeds).

### 2.2 Callback store: capture generation, re-check under one lock

- At validation time (under the lock, where `issued_client_id` is
  already computed), capture `generation(issued_client_id)` into
  `Taken`.
- At store time, do the generation re-check **and** the store
  (keyring write + registration persist) inside a single
  `with_backend` closure. Mismatch ⇒ discard the tokens, mark the
  attempt `Failed("disconnected")`, and return a page telling the
  user the session was disconnected before sign-in finished.
- Holding the backend lock across the store is safe: the work under
  it is local-only (keyring write + atomic file write), never
  network. No helper called from the closure re-enters
  `with_backend`.

### 2.3 Refresh store: same rule

- Capture `generation(client_id)` in the same `with_backend` call
  that sets `chatgpt_refresh_in_flight`.
- After the exchange, store under one lock with the re-check.
  Mismatch ⇒ do not store; return the existing "session ended"
  error (`"The ChatGPT session ended. Sign in again."`).
- Disconnect never waits for a refresh and never touches the
  flag: disconnect always wins, the late store is discarded.

### 2.4 Disconnect reports the actual keyring outcome

`ChatGptSigninDisconnectResponse` gains one field; the other two
keep their meaning:

```rust
pub struct ChatGptSigninDisconnectResponse {
    pub contract_version: u8,
    /// True only when no local record remains after this call
    /// (deleted, or there was none).
    pub disconnected: bool,
    /// "deleted" | "absent" | "failed" — the keyring outcome.
    pub local_deletion: String,
    /// Whether the remote revocation was confirmed. Independent
    /// of `local_deletion`: either can succeed while the other
    /// fails, and both facts are reported.
    pub remote_revocation_confirmed: bool,
}
```

Rules:

- No record loaded ⇒ `local_deletion = "absent"`, `disconnected = true`
  (nothing local remains), no delete attempted.
- Record present and `delete` succeeds ⇒ `"deleted"`,
  `disconnected = true`, stub `plan_usage = false` as today.
- Record present and `delete` fails ⇒ `"failed"`,
  `disconnected = false`, **stub left untouched** (the session is
  still there — status must keep showing it signed in, with plan
  usage from the surviving record), revocation result still
  reported separately. The caller retries disconnect; the retry
  succeeds once the keyring fault clears.
- Order is unchanged: revoke (best-effort) first, then local delete.
  A failed revocation never blocks the local delete, and a failed
  local delete never rewrites the revocation fact.

`chatgpt-signin-status` needs no change: it already derives
`signed_in` / `plan_usage` from the keyring record, so after a
failed delete it truthfully still shows the account signed in —
provided the stub is left alone (D1 fix), the accounts list agrees.

### 2.5 Test harness (test-only, no production change)

- **Shared-mock keyring builder.** The `keyring` mock creates a fresh
  `MockCredential` per `Entry::new`, and its one-shot `set_error` is
  consumed by the disconnect's own `load` before the `delete` — so
  neither can inject a delete failure. The test file instead installs
  a custom builder (same `test_env_once` slot) whose credentials share
  one `MockCredential` per `(service, user)` and carry a persistent
  per-entry `fail_delete` flag: while set, `delete_credential` fails
  and leaves the secret in place; gets/sets are unaffected. Default
  behaviour is identical to the plain mock, so the existing 21 tests
  are unaffected.
- **Token gate in the synthetic issuer.** `IssuerMode` gains an
  optional shared gate; the token handler records the request and
  hit count, then waits on the gate (bounded, so a stuck test fails
  rather than hanging the suite). Tests drive the callback / refresh
  on a spawned thread, wait for the token hit (exchange mid-flight),
  disconnect on the main thread, then release the gate.

## 3. Acceptance criteria

- AC-L1 (reproduce D1): on the delivered tip, disconnect with a
  failing keyring delete returns `disconnected: true`, and status
  disagrees with itself (stub plan usage false while the record
  survives). A new test fails on the tip for exactly this.
- AC-L2 (reproduce D2): on the delivered tip, disconnect during a
  gated reauthorization exchange ends with the session signed in
  again (the late store recreates it). A new test fails on the tip.
- AC-L3 (reproduce D3): on the delivered tip, disconnect during a
  gated refresh ends with the session signed in again (rotated
  tokens stored). A new test fails on the tip.
- AC-L4 (reproduce D4): on the delivered tip, a callback delivered
  after disconnect completes the sign-in (no token exchange should
  have happened after disconnect). A new test fails on the tip.
- AC-L5 (truthful local status): after the fix, a failed keyring
  delete yields `disconnected: false`, `local_deletion: "failed"`,
  remote revocation reported separately, status still signed in with
  plan usage, stub untouched; a retry after clearing the fault
  yields `local_deletion: "deleted"`, `disconnected: true`, status
  signed out; a further disconnect yields `local_deletion:
  "absent"`.
- AC-L6 (pending work invalidated): after the fix, a late callback
  (delivered after disconnect, or mid-exchange when disconnect
  lands) and a late refresh store nothing: no keyring record
  remains, status reports signed out, the attempt is failed with
  reason `"disconnected"`, and exactly one token exchange happened
  (no retry, no second grant).
- AC-L7 (scope): no live OAuth, no real credentials, no paid calls,
  no generated-code execution; real account sign-in stays dev-gated
  (UI unchanged); the API-key flow, local dispatch, and the existing
  21 + 39 tests stay green; contracts regenerated with 0 drift.
- AC-L8 (delivery): checksummed Git bundle ZIP with exact SHAs,
  before/after evidence for AC-L1..L4, a handoff, and a Windows
  rerun checklist.

## 4. Out of scope (known limits, recorded not fixed)

- **Re-sign-in during a refresh.** Generation is bumped only by
  disconnect. If a user re-signs in as the same account while a
  refresh is in flight, the refresh's older rotated tokens can
  overwrite the fresher record. The tokens are still server-valid,
  so the next refresh rotates from them; no session is recreated
  and nothing is disconnected. Fixing it means bumping the
  generation on sign-in too — a separate slice.
- **`InvalidGrant` refresh path.** Its `let _ = resolver.delete(...)`
  is left as-is: the op already errors ("session ended"), and a
  re-sign-in overwrites the stale record. Same follow-up as above.
- No change to `chatgpt-signin-status`, the UI, or any global
  status document.
