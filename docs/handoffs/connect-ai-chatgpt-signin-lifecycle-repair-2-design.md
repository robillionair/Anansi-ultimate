# ChatGPT sign-in lifecycle repair 2 — design

Branch: `muse/connect-ai-chatgpt-signin-lifecycle-repair-2`
Base: lifecycle repair tip `e8290ac08fc5c3bc41b1f21400cb0eb385a6876d`
Date: 2026-11-11 (Linux evidence only — **WINDOWS REVIEW OUTSTANDING**)

## 1. Defects (both in the repair tip, both reproduced before the fix)

### D5. A missing entry, a read error, and a malformed record all look absent

`load_chatgpt_record` returns `Option<Record>` and swallows every
failure (`.ok()?` on the resolve, `.ok()` on the JSON parse). Three
different keyring states collapse to `None`:

- no entry (absence — removal needs no work),
- the store failed to answer (read error — existence unknown),
- bytes exist but are not a valid record (malformed — the entry
  exists and can be deleted, but its refresh token cannot be read).

Disconnect treats all three as `"absent"` and reports
`disconnected: true` without attempting a delete — so on a read
error or malformed bytes it claims the session is gone while the
(entry possibly live) material stays in the keyring. The same
collapse makes refresh claim "No ChatGPT account is signed in."
when the store merely failed to answer.

### D6. An in-flight refresh overwrites a newer sign-in

The lifecycle repair bumps the generation only on disconnect (its
design §4 records this as a known limit). A refresh that started
under generation G and a re-sign-in that commits under the same G
race at the store: whichever stores last wins. If the refresh
stores after the sign-in, the newer session's tokens — including
its fresh refresh token — are replaced by the older rotation, and
the newer session is silently lost.

## 2. Fix design

### 2.1 Typed record load (D5)

New primitive on `KeyringSecretResolver` (providers crate, the
layer that owns the keyring error):

```rust
/// Ok(None) = no entry exists; Err = the store failed to answer
/// (as opposed to answering "absent").
pub fn read_bytes(&self, reference: &str) -> Result<Option<Vec<u8>>, ProviderError>
```

It matches `keyring::Error::NoEntry` explicitly; every other
store error becomes the `Err`.

Core loader returns what the read established:

```rust
enum ChatGptRecordLoad {
    Present(ChatGptCredentialRecord),
    Absent,      // only this establishes there is nothing stored
    Malformed,   // bytes exist but are not a valid record
    Unreadable,  // the store failed to answer at all
}
```

Disconnect (the operative requirement — "report local removal only
when deletion or absence is established"):

| Load | Revoke | Delete | Report |
|---|---|---|---|
| `Absent` | no (nothing to revoke) | not attempted | `localDeletion: "absent"`, `disconnected: true` |
| `Present` | yes, as today | attempted | `"deleted"` / `"failed"` |
| `Malformed` | no (no token readable) | attempted — a delete establishes removal even though the bytes were unreadable | `"deleted"` / `"failed"` |
| `Unreadable` | no (no token readable) | attempted — a succeeding delete establishes removal despite the failed read | `"deleted"` / `"failed"` |

`disconnected` stays true only when removal is established
(`"deleted"` or `"absent"`). No new `localDeletion` value is
needed: the delete attempt is what establishes (or fails to
establish) removal in every non-absent case.

Refresh uses the same load with truthful errors: `Absent` →
"No ChatGPT account is signed in."; `Malformed` → "The stored
ChatGPT session is damaged. Sign in again." (the next sign-in
overwrites the corrupt bytes); `Unreadable` → storage-unavailable.

Status and start keep mapping non-`Present` to "no record"
(their existing behaviour, documented, not changed): status never
claims a session it cannot read, and start merely skips the
`id_token_hint` when the record cannot be read.

### 2.2 Session epoch (D6)

Replace `chatgpt_generations: HashMap<String, u64>` with epochs
that carry the reason for the latest bump:

```rust
struct ChatGptEpoch { version: u64, ended: bool }
chatgpt_epochs: HashMap<String, ChatGptEpoch>
```

- Disconnect bumps `version` and sets `ended = true` (unchanged
  intent, richer state).
- A callback commit bumps `version` and sets `ended = false`, in
  the same lock hold as the store — "invalidate or version that
  work when the new session is committed".
- A refresh never bumps: it is a continuation of the same session,
  not a new one. (Bumping on refresh would discard a concurrent
  callback's fresher tokens — the wrong outcome.)

Store re-check (callback and refresh share it): on a version
mismatch the tokens are discarded; the reason comes from the
current epoch: `ended` ⇒ `"disconnected"`, otherwise
`"superseded"`. Callback: attempt failed with that reason, page
text names it (`"disconnected"` keeps the previous wording;
`"superseded"` says a newer sign-in already completed). Refresh:
`ended` ⇒ the existing "session ended" error; superseded ⇒
`refreshed: false` with the *current* record's expiry (the newer
session is alive; nothing needed refreshing). No DTO changes.

Why the reason is exact: a disconnect-then-sign-in sequence ends
with `ended = false` (a live session exists), so stale work is
"superseded", not "disconnected"; a bare disconnect leaves
`ended = true`.

### 2.3 Test harness (test-only, no production change)

- **Shared-mock builder:** gains a `fail_read` flag next to
  `fail_delete` (the wrapper's `get_secret` fails while set;
  deletes are unaffected), plus byte-level helpers
  (`read_mock_secret`, `read_mock_record`, `corrupt_mock_record`)
  that read the shared mock directly, bypassing the op's view, so
  a test can prove at the byte level that a record is gone or
  survived.
- **Synthetic issuer:** the token gate becomes selective —
  `token_gate_grant` (gate only that grant type) and
  `token_gate_code` (gate only that authorization code) — and the
  minted nonce becomes per-code (`nonce_by_code`, falling back to
  `expected_nonce`), so two concurrent sign-ins can each be
  validated while one exchange is parked.

## 3. Acceptance criteria

- AC-R1 (reproduce D5 read): on the repair tip, disconnect with a
  failing keyring read reports `"absent"` and leaves the record in
  place. A new test fails on the tip.
- AC-R2 (reproduce D5 malformed): on the repair tip, disconnect
  with corrupt stored bytes reports `"absent"` and leaves the bytes.
  A new test fails on the tip.
- AC-R3 (reproduce D6 refresh): on the repair tip, a refresh in
  flight during a re-sign-in overwrites the newer session (the
  surviving record holds the refresh's rotated tokens). A gated
  test fails on the tip.
- AC-R4 (reproduce D6 callback): on the repair tip, a second
  concurrent sign-in stores over the first with no signal (both
  attempts succeed). A new test fails on the tip.
- AC-R5 (truthful disconnect): after the fix, a read failure
  still attempts the delete — success ⇒ `"deleted"`, record gone,
  no revocation attempted; delete failure ⇒ `"failed"`, record
  survives, retry succeeds. Malformed bytes are removed by the
  delete (`"deleted"`), or reported `"failed"` with a successful
  retry.
- AC-R6 (newer session survives): after the fix, the gated refresh
  test ends with the newer sign-in's tokens byte-for-byte intact,
  refresh reporting `refreshed: false` with the newer expiry, and
  exactly the expected token grants. The concurrent sign-in test
  ends with one attempt succeeded and the other failed
  `"superseded"`.
- AC-R7 (scope): no live OAuth, no real credentials, no paid calls,
  no generated-code execution; real sign-in stays dev-gated (UI
  unchanged); no DTO change (contracts `--check` 0 drift); the
  existing 25 + 39 tests stay green.
- AC-R8 (delivery): checksummed Git bundle ZIP with exact SHAs,
  before/after evidence for AC-R1..R4, a handoff, and Windows rerun
  commands.

## 4. Out of scope (unchanged)

- `chatgpt-signin-status` still maps an unreadable/malformed
  record to "not signed in" (fail-closed for use; the disconnect
  response is where removal truth lives).
- The `InvalidGrant` refresh path's swallowed delete (noted in
  the repair handoff §6) is unchanged.
- No change to the UI, the API-key flow, or any global status
  document.
