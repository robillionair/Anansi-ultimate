# ChatGPT sign-in lifecycle repair 3 — handoff

Branch: `muse/connect-ai-chatgpt-signin-lifecycle-repair-3`
Base: lifecycle repair 2 tip `1a4529f76057287c4673ef3d775dc48b70f7054e`
Tip: the handoff commit on this branch (exact SHA in the delivery
message); last code commit `19dbb6a2a`
Date: 2026-11-11 (Linux evidence only — **WINDOWS REVIEW OUTSTANDING**)
Design (committed before implementation):
`docs/handoffs/connect-ai-chatgpt-signin-lifecycle-repair-3-design.md`
(`8c64b5e3a`, AC-I1..I6)

## 1. What was repaired

One remaining lifecycle finding (D7) in the repair-2 tip:

- The refresh `InvalidGrant` arm deleted `record.reference()`
  without checking the captured epoch and without holding the
  backend lock. A newer sign-in can commit while that refresh is
  pending (its record lives under the same reference), and the
  stale failure then deletes the newer credential. The refresh's
  own tokens are dead; the newer session's are not — but the
  delete could not tell them apart, because it never looked at
  the epoch.

Commits on the branch (base `1a4529f76`):

- `8c64b5e3a` — design + acceptance criteria (AC-I1..I6) before code.
- `2897ac760` — reproduction: 1 new test fails on the repair-2
  tip; existing 29 still pass. No production change; harness
  reuse only (refresh-only token gate, byte-level mock read).
- `19dbb6a2a` — the fix. No DTO change (contracts 0 drift).

Files changed vs base (3 files): the design doc,
`crates/core/src/provider_dispatch.rs`,
`crates/core/tests/chatgpt_signin.rs`. No UI change; real account
sign-in stays dev-gated.

## 2. The fix (short form)

The destructive failure handling now follows the same epoch
discipline as a successful store: the version comparison and the
deletion happen under one backend-lock hold, serialized against
callback commits. In the `InvalidGrant` arm:

- **Version matches the captured epoch** — the refresh is
  genuinely current: delete the record (the docs' rule for an
  unusable refresh token is preserved) and return "The ChatGPT
  session ended. Sign in again."
- **Version differs, `ended = true`** — a disconnect landed
  during the exchange: delete nothing (disconnect owns the
  record's fate) and return the same "session ended" error.
- **Version differs, `ended = false`** — a newer sign-in
  committed: delete nothing, and return `refreshed: false` with
  the *current* record's expiry (read under the same lock hold),
  mirroring the success path's superseded outcome.

## 3. Before / after evidence

Reproduction run on the repair-2 tip (production code untouched,
only the design commit + new test):

```
test result: FAILED. 29 passed; 1 failed
- stale_invalid_grant_does_not_delete_newer_signin:
  called `Result::unwrap()` on an `Err` value: CommandError {
    code: InvalidRequest,
    message: "The ChatGPT session ended. Sign in again." }
  (the stale refresh errored — and deleted the newer record)
```

After the fix, same suite:

```
test result: ok. 30 passed; 0 failed
```

## 4. Evidence (Linux, this VM)

- Rust lib: **367 passed**, 0 failed (unchanged).
- Integration `chatgpt_signin`: **30/30 passed** (29 baseline +
  1 new).
- Integration `provider_dispatch` (regression): **39/39 passed**.
- TypeScript: `tsc --noEmit` 0 errors.
- Contracts: `node scripts/contracts.mjs --check` exit 0, 0 drift
  (no DTO change — the refresh response already covers
  `refreshed: false` + `expires_at_ms`).
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

- **AC-I1 (reproduce D7):**
  `stale_invalid_grant_does_not_delete_newer_signin` failed on
  the tip (stale refresh errored "session ended" and deleted the
  newer record). See §3.
- **AC-I2 (guarded failure):** the same sequence now returns
  `refreshed: false` with the newer session's expiry, the newer
  credential remains byte-for-byte unchanged
  (`at-test-access-0003` / `rt-test-refresh-0003`), and a
  follow-up refresh using the surviving refresh token succeeds
  and rotates it (`rt-test-refresh-0002`). All asserted in the new
  test.
- **AC-I3 (preserved deletion):** a genuinely current
  `InvalidGrant` still clears the record and returns "session
  ended" — existing test
  `signin_refresh_invalid_grant_clears_record` stays green.
- **AC-I4 (audit):** the other refresh error paths are audited in
  the design §3 — `Ok(tokens)` store (guarded since repair 2),
  other `ChatGptError` arms (delete nothing), the
  `!needs_refresh` early return (reads the record fresh), the
  no-refresh-token error (no deletion), flag clearing, and the
  callback failure arms (no deletion). The disconnect-delete race
  found while auditing is recorded in the design §4 as a
  follow-up, not fixed here.
- **AC-I5 (scope):** no live OAuth, real credentials, paid calls,
  or generated-code execution; UI untouched and real sign-in stays
  dev-gated; API-key flow and local dispatch unaffected (existing
  tests green, e2e regressions green); contracts 0 drift.
- **AC-I6 (delivery):** this handoff + the checksummed bundle ZIP
  (see §7).

## 6. Known limits and follow-up finding

- **Follow-up (not fixed here):** the disconnect op has the same
  shape of race in the other direction — it bumps the epoch under
  the lock, then revokes over the network (no lock), then deletes
  the record **without** the lock. A sign-in started after the
  bump can commit during the revoke call (its store succeeds
  under the free lock), and disconnect's delete then removes the
  fresh record, after which the stub update clears its
  `plan_usage`. Narrow window (the revoke round-trip), real
  consequence. It needs its own reproduction (gated revoke +
  concurrent sign-in) and its own slice.
- The `InvalidGrant` arm's delete is still swallowed (`let _ =`)
  when it fires — now only after the epoch check established the
  delete was warranted, so a failed delete of a genuinely current
  invalid token is reported as "session ended" either way and the
  record survives until a re-sign-in overwrites it.
- No change to `chatgpt-signin-status`, the UI, or any global
  status document.

## 7. Windows rerun checklist

1. Apply the bundle ZIP on Windows; confirm the tip stated in the
   delivery message (handoff commit; last code commit `19dbb6a2a`)
   and a clean tree.
2. Run the Rust battery:
   `cargo test --locked --workspace --lib` (expect 367),
   `--test provider_dispatch` (39/39),
   `--test chatgpt_signin` (30/30).
3. Run `tsc --noEmit` (0), contracts `--check` (0 drift),
   docs validation (21/21), UI suite (342/342).
4. Run the e2e regressions: connect-ai 1/1, DOCX 7/7, PDF 5/5,
   TXT/MD 9/9, Ask 1/1.
5. Manual (dev-gated) check: start a refresh against a
   short-lived session, re-sign in while it is in flight, revoke
   the old refresh token server-side (or let it expire), and
   confirm the newer session survives and the refresh reports
   `refreshed: false`.
6. **Do not** enable the sign-in button or attempt a live Responses
   turn: eligibility is still unverified (spike handoff §5) — this
   slice changes no eligibility fact.

## 8. Out of scope (unchanged)

- The disconnect-delete race (§6) — follow-up slice.
- Any live OAuth, real credentials, paid calls, Responses turns, or
  model/route eligibility claims.
- Any change to the API-key flow, the frozen Windows pilot, or any
  provider op/contract (no DTO change).
- `providers.spec.ts` 240 s timeout and the other pre-existing
  failures (larp-somnia instruments test, backups.spec.ts
  'Workspace' button, 4 semantic_workflow live-Ollama failures,
  node pwsh failure, MSVC installer test — all pre-existing on
  this VM).
- No global status document was edited (Antigravity-owned files
  untouched); proposed updates, if any, belong in this handoff only.
