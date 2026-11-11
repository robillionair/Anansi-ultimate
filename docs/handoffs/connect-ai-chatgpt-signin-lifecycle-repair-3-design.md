# ChatGPT sign-in lifecycle repair 3 — design

Branch: `muse/connect-ai-chatgpt-signin-lifecycle-repair-3`
Base: lifecycle repair 2 tip `1a4529f76057287c4673ef3d775dc48b70f7054e`
Date: 2026-11-11 (Linux evidence only — **WINDOWS REVIEW OUTSTANDING**)

## 1. Defect (D7)

The refresh `InvalidGrant` arm deletes `record.reference()`
without checking the captured epoch and without holding the
backend lock:

```rust
Err(crate::chatgpt_signin::ChatGptError::InvalidGrant) => {
    let _ = resolver.delete(&record.reference());
    Err(invalid("The ChatGPT session ended. Sign in again."))
}
```

A newer sign-in can commit while that refresh is pending (its
record lives under the same reference), and the stale failure
then deletes the newer credential. The refresh's own tokens are
dead; the newer session's are not — but the delete cannot tell
them apart, because it never looks at the epoch.

## 2. Fix design

Guard the destructive failure handling with the same epoch
discipline as a successful store: the version comparison and the
deletion are serialized against callback commits by doing both
under one backend-lock hold.

In the `InvalidGrant` arm, under one `with_backend` hold:

- Read the current epoch for the account.
- **Version matches the captured `epoch_version`** — the refresh
  is genuinely current: delete the record (the docs' rule for an
  unusable refresh token is preserved) and return the existing
  "The ChatGPT session ended. Sign in again." error.
- **Version differs, `ended = true`** — a disconnect landed
  during the exchange: delete nothing (disconnect owns the
  record's fate) and return the same "session ended" error.
- **Version differs, `ended = false`** — a newer sign-in
  committed: delete nothing, and return `refreshed: false` with
  the *current* record's expiry (read under the same lock hold),
  mirroring the success path's superseded outcome. The session is
  alive; the failure belonged to the old tokens.

No DTO change: the refresh response already carries
`refreshed: false` + `expires_at_ms`.

## 3. Audit of the other refresh error paths

- `Ok(tokens)` store — epoch re-check under the lock (repair 2).
  Guarded.
- `Err(e)` for any other `ChatGptError` (network, invalid token,
  …) — returns the mapped error, deletes nothing, stores nothing.
  A stale failure here cannot touch the record; a retry captures
  the current epoch. No change.
- `!needs_refresh` early return — reads the record fresh at call
  time, returns its expiry. No staleness. No change.
- No-refresh-token early error — no deletion. No change.
- In-flight flag clearing — unconditional, correct. No change.
- Callback exchange-failure arm and store-error arm — mark the
  attempt failed, delete nothing. No change.

Conclusion: `InvalidGrant` is the only refresh error path with a
destructive action, and the only one that changes.

## 4. Follow-up finding (recorded, NOT fixed in this slice)

The disconnect op has the same shape of race in the other
direction: it bumps the epoch under the lock, then revokes over
the network (no lock), then deletes the record **without** the
lock. A sign-in started after the bump can commit during the
revoke call — its store succeeds under the (free) lock — and
disconnect's delete then removes the fresh record, after which
the stub update clears its `plan_usage`. Narrow window (the
revoke round-trip), real consequence (a session created after
disconnect is deleted). It needs its own reproduction (gated
revoke + concurrent sign-in) and its own slice; this repair
stays bounded to the refresh `InvalidGrant` path.

## 5. Acceptance criteria

- AC-I1 (reproduce D7): on the repair-2 tip, a gated old refresh
  released with `InvalidGrant` after a newer sign-in commits
  returns "session ended" and deletes the newer record. A new
  synchronized test fails on the tip.
- AC-I2 (guarded failure): after the fix, the same sequence
  returns `refreshed: false` with the newer session's expiry, the
  newer credential remains byte-for-byte unchanged, and a
  follow-up refresh using the surviving refresh token succeeds
  (proving it usable).
- AC-I3 (preserved deletion): a genuinely current `InvalidGrant`
  still clears the record and returns "session ended" (existing
  test `signin_refresh_invalid_grant_clears_record` stays green).
- AC-I4 (audit): the other refresh error paths are audited in §3;
  the disconnect race is recorded in §4 as a follow-up, not
  fixed here.
- AC-I5 (scope): no live OAuth, no real credentials, no paid
  calls, no generated-code execution; real sign-in stays
  dev-gated (UI unchanged); no DTO change (contracts `--check`
  0 drift); the existing 29 + 39 tests stay green.
- AC-I6 (delivery): checksummed Git bundle ZIP with exact SHAs,
  before/after evidence for AC-I1, a handoff, and Windows rerun
  commands.

## 6. Out of scope (unchanged)

- The disconnect-delete race (§4) — follow-up slice.
- `chatgpt-signin-status` mapping, the `InvalidGrant` path's
  swallowed-delete note (it is now epoch-guarded, so the swallow
  only fires when the delete was actually warranted), the UI, the
  API-key flow, any global status document.
