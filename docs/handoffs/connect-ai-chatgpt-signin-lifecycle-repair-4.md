# ChatGPT sign-in lifecycle repair 4 — handoff

Branch: `muse/connect-ai-chatgpt-signin-lifecycle-repair-4`
Base: lifecycle repair 3 tip `1dfd312e0394b21d708cefeeae8140d0e652adb8`
Tip: the handoff commit on this branch (exact SHA in the delivery
message); last code commit `1b8438150`
Date: 2026-11-11 (Linux evidence only — **WINDOWS REVIEW OUTSTANDING**)
Design (committed before implementation):
`docs/handoffs/connect-ai-chatgpt-signin-lifecycle-repair-4-design.md`
(`4ebcc17db`, AC-J1..J8)

## 1. What was repaired

One remaining lifecycle finding (D8), the follow-up recorded in
the repair-3 handoff §6:

- Disconnect bumped the epoch under the backend lock, then
  revoked over the network (no lock), then deleted the record and
  cleared the registration stub **without** the lock and without
  re-checking the epoch. A sign-in started after the bump could
  commit during the revoke call — its store succeeds under the
  free lock — and the older disconnect's delete then removed the
  fresh record, after which the stub update cleared its
  `plan_usage`. The user signed in after clicking disconnect and
  ended up signed out with their fresh credential deleted.

Commits on the branch (base `1dfd312e0`):

- `4ebcc17db` — design + acceptance criteria (AC-J1..J8) before code.
- `6dcd6b4de` — reproduction: 2 new tests fail on the repair-3
  tip; existing 30 still pass. Test-only harness: revoke gate on
  the synthetic issuer + `wait_revoke_hits`.
- `1b8438150` — the fix + regenerated contracts.

Files changed vs base (5 files): the design doc,
`crates/core/src/provider_dispatch.rs`,
`crates/core/tests/chatgpt_signin.rs`, regenerated contracts
(`readiness.json`, `generated.ts`, `validators.mjs`). No UI
change; real account sign-in stays dev-gated.

## 2. The fix (short form)

The local destructive changes are now serialized against
callback commits using the epoch discipline, with the network
kept outside the lock:

1. Bump + invalidate (lock) — unchanged, but the closure now
   returns the bumped version.
2. Load (for the revoke decision) — unchanged.
3. Revoke (network, no lock) — unchanged, still targets the
   **old** session's refresh token only.
4. Delete decision under **one** lock hold: re-read the current
   epoch; `version != bumped && !ended` ⇒ a newer sign-in
   committed — delete nothing, touch no registration metadata;
   otherwise re-read the record in the same hold (another
   disconnect may have removed it) and delete / report `"absent"`
   / report `"failed"` as before, clearing the stub's
   `plan_usage` only when disconnected.

`localDeletion` gains a fourth value, `"superseded"`, with the
semantics defined in the design §3 and documented on the DTO:
`disconnected` is true only when the account has no live session
as a result of **this** call; `remoteRevocationConfirmed` keeps
describing the **old** session's revocation, so a superseded
disconnect can honestly report the old token revoked
(`remoteRevocationConfirmed: true`) alongside a live newer
session (`disconnected: false`).

## 3. Before / after evidence

Reproduction run on the repair-3 tip (production code untouched,
only the design commit + new tests):

```
test result: FAILED. 30 passed; 2 failed
- disconnect_during_revocation_does_not_delete_newer_signin:
  assertion `left == right` failed; left: Bool(true), right: false
  (the old disconnect claimed the newer session disconnected)
- second_disconnect_after_first_reports_absent_not_failed:
  assertion `left == right` failed; left: Bool(false), right: true
  (the second disconnect reported "failed" for an already-removed record)
```

After the fix, same suite:

```
test result: ok. 32 passed; 0 failed
```

## 4. Evidence (Linux, this VM)

- Rust lib: **367 passed**, 0 failed (unchanged).
- Integration `chatgpt_signin`: **32/32 passed** (30 baseline +
  2 new).
- Integration `provider_dispatch` (regression): **39/39 passed**.
- TypeScript: `tsc --noEmit` 0 errors.
- Contracts: regenerated for the new `localDeletion` value and
  its documented semantics; `node scripts/contracts.mjs --check`
  exit 0, 0 drift after regeneration.
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
  crate root).

## 5. Acceptance criteria — how each was met

- **AC-J1 (reproduce D8):**
  `disconnect_during_revocation_does_not_delete_newer_signin`
  failed on the tip (`disconnected: true` while the newer session
  was deleted). See §3.
- **AC-J2 (reproduce double-disconnect):**
  `second_disconnect_after_first_reports_absent_not_failed`
  failed on the tip (`disconnected: false`, `"failed"`). See §3.
- **AC-J3 (guarded disconnect):** the superseded disconnect
  reports `disconnected: false`,
  `localDeletion: "superseded"`,
  `remoteRevocationConfirmed: true` (the old token
  `rt-test-refresh-0001` was revoked, not the new one), the newer
  credential remains byte-for-byte unchanged
  (`at-test-access-0003` / `rt-test-refresh-0003`), its
  `planUsage` stays true, and a follow-up refresh using the
  surviving token succeeds and rotates it
  (`rt-test-refresh-0002`). All asserted in the new test.
- **AC-J4 (double-disconnect):** the first disconnect reports
  `"deleted"`, the second `"absent"` / `disconnected: true`, with
  two revocations recorded and the account signed out.
- **AC-J5 (preserved behavior):** failed-delete reporting and
  retry unchanged — existing test
  `disconnect_keyring_deletion_failure_is_reported_and_retry_succeeds`
  stays green; genuinely current disconnects still report
  `"deleted"` (existing tests); revocation stays outside the lock
  (the test gates it while the sign-in completes).
- **AC-J6 (audit):** design §4 — every `store_chatgpt_record`,
  `persist_chatgpt_registrations`, and ChatGPT
  `resolver.delete` site audited together; only the disconnect
  delete + stub update was unguarded. The no-revoke variant
  shares the fixed code path (the epoch check is uniform).
- **AC-J7 (scope):** no live OAuth, real credentials, paid calls,
  or generated-code execution; UI untouched and real sign-in stays
  dev-gated; API-key flow and local dispatch unaffected (existing
  tests green, e2e regressions green); contracts regenerated,
  0 drift after regeneration.
- **AC-J8 (delivery):** this handoff + the checksummed bundle ZIP
  (see §7).

## 6. Known limits

- The `InvalidGrant` arm's delete is still swallowed (`let _ =`)
  when it fires — epoch-guarded since repair 3, so the swallow
  only fires when the delete was actually warranted.
- `chatgpt-signin-status` still maps an unreadable or malformed
  record to "not signed in" (fail-closed for use; the disconnect
  response is where removal truth lives — repair 2).
- If disconnect's keyring delete fails while a refresh is in
  flight, the refresh reports "session ended" (the disconnect's
  intent) even though the local record survives until a retry
  disconnect; status remains the source of truth for the local
  record (repair handoff §6).
- No change to `chatgpt-signin-status`, the UI, or any global
  status document.

## 7. Windows rerun checklist

1. Apply the bundle ZIP on Windows; confirm the tip stated in the
   delivery message (handoff commit; last code commit `1b8438150`)
   and a clean tree.
2. Run the Rust battery:
   `cargo test --locked --workspace --lib` (expect 367),
   `--test provider_dispatch` (39/39),
   `--test chatgpt_signin` (32/32).
3. Run `tsc --noEmit` (0), contracts `--check` (0 drift),
   docs validation (21/21), UI suite (342/342).
4. Run the e2e regressions: connect-ai 1/1, DOCX 7/7, PDF 5/5,
   TXT/MD 9/9, Ask 1/1.
5. Manual (dev-gated) check: start a disconnect, and while its
   revocation is in flight, complete a new sign-in for the same
   account; confirm the disconnect reports `localDeletion:
   "superseded"` with `disconnected: false`, the newer session
   stays signed in with its plan usage intact, and a second
   disconnect ends it normally.
6. **Do not** enable the sign-in button or attempt a live Responses
   turn: eligibility is still unverified (spike handoff §5) — this
   slice changes no eligibility fact.

## 8. Out of scope (unchanged)

- Any live OAuth, real credentials, paid calls, Responses turns, or
  model/route eligibility claims.
- Any change to the API-key flow, the frozen Windows pilot, or any
  provider op/contract except the `localDeletion` vocabulary
  (one new value + documented semantics).
- `providers.spec.ts` 240 s timeout and the other pre-existing
  failures (larp-somnia instruments test, backups.spec.ts
  'Workspace' button, 4 semantic_workflow live-Ollama failures,
  node pwsh failure, MSVC installer test — all pre-existing on
  this VM).
- No global status document was edited (Antigravity-owned files
  untouched); proposed updates, if any, belong in this handoff only.
