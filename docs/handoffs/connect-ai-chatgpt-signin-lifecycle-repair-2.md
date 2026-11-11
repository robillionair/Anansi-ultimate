# ChatGPT sign-in lifecycle repair 2 — handoff

Branch: `muse/connect-ai-chatgpt-signin-lifecycle-repair-2`
Base: lifecycle repair tip `e8290ac08fc5c3bc41b1f21400cb0eb385a6876d`
Tip: the handoff commit on this branch (exact SHA in the delivery
message); last code commit `55f1735ce`
Date: 2026-11-11 (Linux evidence only — **WINDOWS REVIEW OUTSTANDING**)
Design (committed before implementation):
`docs/handoffs/connect-ai-chatgpt-signin-lifecycle-repair-2-design.md`
(`5887a0f15`, AC-R1..R8)

## 1. What was repaired

Two defects in the lifecycle repair tip, both reproduced before
the fix:

- **D5.** A missing entry, a read error, and a malformed stored
  record all collapsed to `None` in `load_chatgpt_record`, so
  disconnect reported `"absent"` and `disconnected: true` without
  attempting a delete — claiming the session was gone while the
  (possibly live) material stayed in the keyring. Refresh claimed
  "No ChatGPT account is signed in." when the store merely failed
  to answer.
- **D6.** An in-flight refresh overwrote a newer sign-in: the
  generation was bumped only by disconnect, so a refresh that
  started under the same generation as a re-sign-in stored its
  older rotation after the new session committed. A second
  concurrent sign-in likewise overwrote the first with no signal.

Commits on the branch (base `e8290ac08`):

- `5887a0f15` — design + acceptance criteria (AC-R1..R8) before code.
- `6c77f41f4` — reproduction: 4 new tests fail on the repair tip;
  existing 25 still pass. Test-only harness: `fail_read` flag on
  the shared mock, byte-level mock helpers (read/corrupt),
  selective token gate (grant/code) and per-code nonces.
- `55f1735ce` — the fix. No DTO change (contracts 0 drift).

Files changed vs base (4 files): the design doc,
`crates/core/src/provider_dispatch.rs`,
`crates/providers/src/keyring_resolver.rs`,
`crates/core/tests/chatgpt_signin.rs`. No UI change; real account
sign-in stays dev-gated.

## 2. The fix (short form)

- **Typed record load.** `KeyringSecretResolver::read_bytes`
  returns `Ok(None)` for a missing entry and `Err` for a store
  failure (matching `keyring::Error::NoEntry` explicitly). The
  core loader returns `Present` / `Absent` / `Malformed` /
  `Unreadable`. Disconnect: `Absent` ⇒ `"absent"` (no delete
  needed); anything else ⇒ attempt the delete, `"deleted"` on
  success, `"failed"` on failure — so a succeeding delete
  establishes removal even when the read could not. Revocation is
  attempted only when a token is readable. Refresh reports a
  damaged record ("The stored ChatGPT session is damaged. Sign in
  again.") and an unreadable record (storage-unavailable)
  truthfully instead of "no account". Status and start keep
  mapping non-`Present` to "no record" (documented, unchanged:
  status never claims a session it cannot read).
- **Session epoch.** `chatgpt_generations: HashMap<String, u64>`
  becomes `chatgpt_epochs: HashMap<String, ChatGptEpoch>` with
  `{version, ended}`. Disconnect bumps the version and sets
  `ended = true`. A callback commit bumps the version and sets
  `ended = false` in the same lock hold as the store. A refresh
  never bumps (it is a continuation, not a new session — bumping
  on refresh would discard a concurrent callback's fresher
  tokens). On a version mismatch the tokens are discarded; the
  reason comes from the current epoch: `ended` ⇒
  `"disconnected"`, otherwise `"superseded"`. Callback: attempt
  failed with that reason, page text names it. Refresh:
  disconnected ⇒ the existing "session ended" error; superseded
  ⇒ `refreshed: false` with the *current* record's expiry (the
  newer session is alive; nothing needed refreshing).

## 3. Before / after evidence

Reproduction run on the repair tip (production code untouched,
only the design commit + new tests):

```
test result: FAILED. 25 passed; 4 failed
- disconnect_read_failure_...: left: String("absent"), right: "deleted"
- disconnect_malformed_storage_...: left: String("absent"), right: "deleted"
- refresh_during_resignin_...: left: Bool(true), right: false
  (refresh reported refreshed:true after overwriting the newer session)
- second_concurrent_signin_...: callback page "Signed in with ChatGPT."
  (the parked second sign-in stored over the first with no signal)
```

After the fix, same suite:

```
test result: ok. 29 passed; 0 failed
```

## 4. Evidence (Linux, this VM)

- Rust lib: **367 passed**, 0 failed (366 + 1 new
  `read_bytes_distinguishes_missing_from_present` unit test in
  the providers crate).
- Integration `chatgpt_signin`: **29/29 passed** (25 baseline +
  4 new).
- Integration `provider_dispatch` (regression): **39/39 passed**.
- TypeScript: `tsc --noEmit` 0 errors.
- Contracts: `node scripts/contracts.mjs --check` exit 0, 0 drift
  (no DTO change — `localDeletion` vocabulary and the refresh
  response already cover the new outcomes).
- Docs validation: **21/21 passed**.
- UI suite (`pnpm test:ui`): **342/342 passed**.
- Node suite (`pnpm test`): **39/40** — the one failure is the
  pre-existing MSVC installer test, confirmed identical on the
  spike base commit in the previous slice. Not a regression.
- e2e regressions (lease-run, one spec per invocation):
  connect-ai **1/1**, DOCX **7/7**, PDF **5/5**, TXT/MD **9/9**, Ask **1/1**.
- Warnings: one, pre-existing — `crates/core/src/mcp.rs:715`
  (`preserved_env_entries` never used). The one new warning the
  fix introduced (unused `SecretResolver` import — the loader no
  longer calls `resolve`) was removed at root cause; the build is
  back to the single pre-existing warning.
- `rustfmt` applied to `provider_dispatch.rs` and
  `keyring_resolver.rs` only (never to a crate root).

## 5. Acceptance criteria — how each was met

- **AC-R1 (reproduce D5 read):**
  `disconnect_read_failure_does_not_claim_removal_and_retry_succeeds`
  failed on the tip (`"absent"` while the record survived). See §3.
- **AC-R2 (reproduce D5 malformed):**
  `disconnect_malformed_storage_is_removed_and_retry_succeeds`
  failed on the tip (`"absent"` while the corrupt bytes survived).
  See §3.
- **AC-R3 (reproduce D6 refresh):**
  `refresh_during_resignin_does_not_overwrite_newer_session`
  failed on the tip (`refreshed: true`, newer tokens overwritten).
  See §3.
- **AC-R4 (reproduce D6 callback):**
  `second_concurrent_signin_is_superseded_not_overwritten`
  failed on the tip (second sign-in's page "Signed in with
  ChatGPT"). See §3.
- **AC-R5 (truthful disconnect):** read failure ⇒ delete
  attempted: success ⇒ `"deleted"`, record gone at the byte
  level, no revocation attempted (`revoke_hits` 0); delete
  failure ⇒ `"failed"`, record survives, retry succeeds. Malformed
  bytes ⇒ `"deleted"` with the bytes gone, or `"failed"` with a
  successful retry. All asserted in the two disconnect tests.
- **AC-R6 (newer session survives):** gated refresh test ends
  with the newer sign-in's tokens byte-for-byte intact
  (`at-test-access-0003` / `rt-test-refresh-0003`), refresh
  reporting `refreshed: false` with the newer session's expiry,
  exactly 3 token grants. Concurrent sign-in test ends with the
  parked sign-in's page naming the newer sign-in and the third
  sign-in's tokens surviving.
- **AC-R7 (scope):** no live OAuth, real credentials, paid calls,
  or generated-code execution; UI untouched and real sign-in stays
  dev-gated; API-key flow and local dispatch unaffected (existing
  tests green, e2e regressions green); contracts 0 drift.
- **AC-R8 (delivery):** this handoff + the checksummed bundle ZIP
  (see §7).

## 6. Known limits (recorded in the design, not fixed here)

- `chatgpt-signin-status` still maps an unreadable or malformed
  record to "not signed in" (fail-closed for use; the disconnect
  response is where removal truth lives).
- The `InvalidGrant` refresh path's swallowed delete is unchanged
  (noted in the repair handoff §6).
- If disconnect's keyring delete fails while a refresh is in
  flight, the refresh reports "session ended" (the disconnect's
  intent) even though the local record survives until a retry
  disconnect; status remains the source of truth for the local
  record (noted in the repair handoff §6).
- No change to `chatgpt-signin-status`, the UI, or any global
  status document.

## 7. Windows rerun checklist

1. Apply the bundle ZIP on Windows; confirm the tip stated in the
   delivery message (handoff commit; last code commit `55f1735ce`)
   and a clean tree.
2. Run the Rust battery:
   `cargo test --locked --workspace --lib` (expect 367),
   `--test provider_dispatch` (39/39),
   `--test chatgpt_signin` (29/29).
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
6. Manual supersede check: start a refresh against a short-lived
   session, re-sign in while it is in flight, and confirm the
   newer session survives and the refresh reports
   `refreshed: false`.
7. **Do not** enable the sign-in button or attempt a live Responses
   turn: eligibility is still unverified (spike handoff §5) — this
   slice changes no eligibility fact.

## 8. Out of scope (unchanged)

- Any live OAuth, real credentials, paid calls, Responses turns, or
  model/route eligibility claims.
- Any change to the API-key flow, the frozen Windows pilot, or any
  provider op/contract except the internal loader and epoch state
  (no DTO change).
- `providers.spec.ts` 240 s timeout and the other pre-existing
  failures (larp-somnia instruments test, backups.spec.ts
  'Workspace' button, 4 semantic_workflow live-Ollama failures,
  node pwsh failure, MSVC installer test — all pre-existing on
  this VM).
- No global status document was edited (Antigravity-owned files
  untouched); proposed updates, if any, belong in this handoff only.
