# ChatGPT sign-in lifecycle repair 4 — design

Branch: `muse/connect-ai-chatgpt-signin-lifecycle-repair-4`
Base: lifecycle repair 3 tip `1dfd312e0394b21d708cefeeae8140d0e652adb8`
Date: 2026-11-11 (Linux evidence only — **WINDOWS REVIEW OUTSTANDING**)

## 1. Defect (D8)

The disconnect op bumps the epoch under the backend lock, then
revokes over the network (no lock), then deletes the record and
clears the registration stub **without** the lock and without
re-checking the epoch:

```rust
with_backend(... bump epoch, invalidate attempts ...)?;
let load = load_chatgpt_record(...);
revoke_refresh_token(...); // network, no lock
let (local_deletion, disconnected) = match &load { ... delete ... };
if disconnected { reg.plan_usage = false; persist(...); }
```

A sign-in started after the bump can commit during the revoke
call — its store succeeds under the (free) lock — and the older
disconnect's delete then removes the fresh record, after which
the stub update clears its `plan_usage`. The user signed in
after clicking disconnect and ends up signed out with their
fresh credential deleted.

## 2. Fix design

Serialize the local destructive changes against callback commits
using the epoch discipline, keeping the network outside the lock:

1. Bump + invalidate (lock) — unchanged, but the closure now
   returns the bumped version.
2. Load (for the revoke decision) — unchanged.
3. Revoke (network, no lock) — unchanged, and still targets the
   **old** session's refresh token only.
4. Local destructive changes under **one** lock hold:
   - Re-read the current epoch.
   - `version != bumped && !ended` ⇒ a newer sign-in committed:
     delete nothing, touch no registration metadata.
   - Otherwise no newer sign-in is live (`ended = true` means the
     latest epoch event ended the session, so any record present
     is a dead session's leftover): re-read the record under the
     same hold — another disconnect may have removed it — then
     delete (`"deleted"` / `"failed"`) or report `"absent"`, and
     clear the stub's `plan_usage` only when disconnected.

The re-read matters for two concurrent disconnects: the second
one's delete must not report `"failed"` for a record the first
one already removed.

## 3. Response semantics (defined here, documented on the DTO)

`disconnected` is true only when the account has no live ChatGPT
session as a result of **this** call. `localDeletion` gains a
fourth value:

| `localDeletion` | `disconnected` | Meaning |
|---|---|---|
| `"deleted"` | true | This call deleted the old record. |
| `"absent"` | true | No record existed when the delete decision was made, and no newer sign-in committed. |
| `"failed"` | false | The delete was attempted and failed — the old session is still there; retry disconnect. |
| `"superseded"` | false | A newer sign-in committed before the delete ran. This call deleted nothing and changed no registration metadata. The newer session is live; disconnect again to end it. |

`remoteRevocationConfirmed` keeps its meaning and stays
independent: it describes the **old** session's refresh token
revocation, not the current session's state. A superseded
disconnect can report `remoteRevocationConfirmed: true` (the old
token was revoked) alongside `disconnected: false` (a newer
session is live) — that combination is the honest report, not a
contradiction.

No UI change is needed: the response is consumed by the
dev-gated UI only, and the contract is regenerated (see §5).

## 4. Audit of all credential and registration mutations

Every `store_chatgpt_record`, `persist_chatgpt_registrations`,
and ChatGPT `resolver.delete` call site was audited together:

| Site | Guard |
|---|---|
| Callback store: record + registration insert | Epoch re-check under the lock (repairs 1–2). |
| Refresh store: record + registration update | Epoch re-check under the lock (repair 2). |
| Refresh `InvalidGrant` delete | Epoch check under one lock hold (repair 3). |
| **Disconnect delete + stub update** | **Unguarded — D8, fixed here.** |
| `providers-credential-remove` delete | Different op, different reference namespace, guarded by the referenced-credential check under the lock (Connect AI repair). Out of scope. |
| Status / start loads | Reads only. |

Conclusion: the disconnect delete + stub update was the only
unguarded ChatGPT credential/registration mutation. The no-revoke
variant (identity-only session, no refresh token) shares the
fixed code path — the epoch check is uniform, so it needs no
separate gate; the gated-revoke test is the deterministic proof.

## 5. Acceptance criteria

- AC-J1 (reproduce D8): on the repair-3 tip, a gated disconnect
  whose revocation is pending while a newer sign-in completes
  deletes the newer credential and clears its `plan_usage`. A
  new synchronized test fails on the tip.
- AC-J2 (reproduce double-disconnect): on the tip, a second
  disconnect whose record the first already removed reports
  `"failed"` instead of `"absent"`. A new synchronized test fails
  on the tip.
- AC-J3 (guarded disconnect): after the fix, the superseded
  disconnect reports `disconnected: false`,
  `localDeletion: "superseded"`, `remoteRevocationConfirmed: true`
  (the old token was revoked), the newer credential remains
  byte-for-byte unchanged, its `plan_usage` stays true, and a
  follow-up refresh using the surviving token succeeds.
- AC-J4 (double-disconnect): after the fix, the second disconnect
  reports `"absent"` / `disconnected: true`, with the first
  reporting `"deleted"`.
- AC-J5 (preserved behavior): failed-delete reporting and retry
  unchanged (existing lifecycle test stays green); a genuinely
  current disconnect still deletes and reports `"deleted"`
  (existing tests); network (revoke) stays outside the lock.
- AC-J6 (audit): §4 — only the disconnect delete + stub update
  was unguarded; the no-revoke variant shares the fixed path.
- AC-J7 (scope): no live OAuth, no real credentials, no paid
  calls, no generated-code execution; real sign-in stays
  dev-gated (UI unchanged); contracts regenerated for the new
  `localDeletion` value, `--check` 0 drift; the existing 30 + 39
  tests stay green.
- AC-J8 (delivery): checksummed Git bundle ZIP with exact SHAs,
  before/after evidence for AC-J1/J2, a handoff, and Windows
  rerun commands.

## 6. Out of scope (unchanged)

- Any live OAuth, real credentials, paid calls, Responses turns,
  or model/route eligibility claims.
- The UI, the API-key flow, the frozen Windows pilot, any global
  status document.
- The `InvalidGrant` arm's swallowed delete (epoch-guarded since
  repair 3) and `chatgpt-signin-status` mapping (repair 2).
