# Connect AI — verification binding + abandoned-credential lifecycle — design

Repair branch: `muse/connect-ai-verification-repair`, from the delivered
Connect AI tip `88737ef09a82b7c6913343d74e88cda861f57896`. Two acceptance
findings against that slice. Linux implementation + verification only;
Antigravity owns Windows integration. Nothing is merged, pushed, or
promoted. No OAuth, no Codex, no real credentials, no cloud calls.

## Finding 1 — "Connection verified" is bound to a provider id, not a connection

Delivered `providers-status` derives `connection_verified` from the
usage ledger: any provider with `requests > 0` since the app started
reads verified. So a changed endpoint or a replaced credential inherits
the previous connection's status, and a slow success from the old
connection landing after a change verifies the new one. Usage history
is the wrong witness: it records consumption, not the identity of the
connection that succeeded.

### Fix

A saved connection gets a fingerprint, computed from exactly the
fields that define the connection:

```
connection_fingerprint(p) = fingerprint(
  provider | baseUrl | credential reference (or none)
  | customEndpoint | allowInsecureCustom)
```

The model list is deliberately excluded: changing which models are
offered does not change the connection. The endpoint, the credential
reference, and the endpoint-security flags are included.

- `BackendState` gains `verified: HashMap<String, String>` —
  provider id → fingerprint of the connection whose dispatch last
  succeeded. Session-scoped, in-memory only (like the ledger);
  a restart clears it, and the UI already labels verification as
  per-session.
- `providers-status` reports `connection_verified` only when the
  stored fingerprint for that provider equals the fingerprint of
  the connection currently saved. The ledger is no longer read
  for verification at all.
- Recording is bound at both ends of a dispatch. Phase 1 (locked)
  snapshots every configured provider's fingerprint. On a
  successful unary dispatch the serving provider is
  `outcome.response.provider` (the dispatcher sets it from the
  candidate that actually served — `CompletionResponse.provider`).
  The backend re-locks, compares the saved connection's current
  fingerprint with the snapshot, and records verification only
  when they are equal. A delayed success from the old connection
  therefore finds a changed fingerprint and records nothing.
  Usage and budgets are untouched: the runtime settles the ledger
  independently, so the late call still counts as usage — it
  simply does not verify the new connection.
- Streams get the same binding: stream-start snapshots the
  fingerprints and records the serving provider from
  `outcome.disclosure.provider` (built from the established
  candidate) in the `StreamRun`; when polling observes a terminal
  `Completed` event the same re-check runs. A cancelled or
  errored stream never verifies. A stream success under an
  unchanged connection verifies exactly like a unary success.
- Invalidation on change, in one helper
  (`invalidate_changed_verifications`): when `providers-configure`
  or `providers-configure-add` installs a new configuration, every
  verified entry whose fingerprint changed — or whose provider
  disappeared — is dropped. Unchanged providers keep their
  verification: editing provider B must not clear provider A.
- Key rotation under the same reference: the fingerprint cannot
  see a key replaced under an unchanged reference, so
  `providers-credential-set` and `providers-credential-remove`
  invalidate verification for every saved provider whose
  credential reference matches, after the store/delete succeeds
  (config lazy-loaded when the state has none, as the add op
  already does).

## Finding 2 — abandoned setup credentials have no lifecycle

Delivered flow: the owner can store a key under a host-generated
reference and then discard the draft or leave setup; the key then
sits in the OS keyring with no saved connection referencing it and
no path back to it. Worse, `providers-credential-remove` deletes
unconditionally — removing a key a saved connection still
references, e.g. while editing that provider, or while a second
setup overlaps the first.

Two facts shape the fix. The keyring resolver has no enumeration
API (`exists` / `store` / `delete` / `resolve` only; the portable
`keyring` crate cannot list a service's entries), so the backend
cannot discover orphans by scanning the store. And a reference is
only ever a label the host minted: the backend can remember which
references it minted.

### Fix

- A persisted minted-reference ledger:
  `<root>/provider-config/<safe-workspace>.minted.json`, a JSON
  object `{reference: provider}` — labels only, never secrets —
  written atomically with the same hardened pattern as the config
  file (create-new tmp + fsync + rename). Loaded lazily into
  `BackendState.minted_references`. `providers-credential-reference`
  records each minted reference and persists the ledger; a persist
  failure fails the mint (nothing was stored yet, so a retry is
  safe). Before minting, entries that are neither stored nor
  referenced by a saved connection are pruned, so the ledger does
  not grow without bound. Deleting a reference drops it from the
  ledger.
- `providers-status` gains `orphanedCredentials`:
  minted references that are stored in the keyring and are not
  referenced by any saved connection — each with the provider it
  was minted for. This is the recovery path for a key left behind
  by leaving setup: it is listed, with its reference, until the
  owner removes it or saves a connection under it. The list is
  computed at read time from (minted, stored, referenced), so a
  reference that becomes referenced stops being an orphan without
  any extra write.
- `providers-credential-remove` is guarded: under the backend lock
  (config lazy-loaded), if any saved connection references the
  reference, the op refuses with `InvalidRequest` ("in use by a
  saved connection") and deletes nothing. Otherwise it deletes
  from the keyring, drops the ledger entry, and invalidates
  verification for that reference. Owner-only as before; a paired
  browser is still refused with `Forbidden` and recorded.
- UI: a "Discard draft" action in the flow (steps 2–5, and on a
  failed save next to the retry) resets the draft to step 1. When
  the draft stored a key under an unreferenced reference, discard
  removes it through the guarded op and reports the removal; if
  the key became referenced meanwhile (an overlapping setup saved
  it first), the backend refuses and discard reports that the key
  is now in use and was kept. Leaving setup without discarding
  keeps the key stored — it then appears in the new "Unused
  credentials" section of Connected providers, with a Remove
  button per entry. A failed save keeps its inputs for retry
  (existing behaviour) and now also offers discard.
- Overlapping setups are safe by construction: each draft mints
  its own reference, and every removal re-checks referenced-ness
  at removal time, so discarding one draft can never remove
  another draft's key or a saved connection's key.

## Acceptance criteria

- AC-V1 (finding 1, endpoint change): dispatch succeeds for
  provider A; change A's endpoint (configure-add, same credential
  reference); status shows A "not yet verified"; usage still
  counts the earlier request; budget unchanged.
- AC-V2 (finding 1, credential change): dispatch succeeds for A;
  rotate A's key under a new reference and save; status shows A
  "not yet verified" until a dispatch succeeds on the new
  connection.
- AC-V3 (finding 1, rotation under the same reference): dispatch
  succeeds for A; store a new key under A's existing reference
  (rotation); status shows A "not yet verified" until a dispatch
  succeeds afterwards.
- AC-V4 (finding 1, delayed success): a slow unary dispatch for A
  is in flight; while it is in flight the endpoint is changed;
  the old dispatch then succeeds. The new connection is never
  verified; the late request still counts in usage; budget
  unchanged. The same bound holds for a stream that completes
  after a change.
- AC-V5 (finding 1, unchanged providers keep verification):
  dispatch succeeds for A and B; change B only; A stays verified,
  B is unverified.
- AC-V6 (finding 2, discard cleans up): mint a reference, store a
  key, discard the draft without saving; the key is gone from the
  keyring and the reference leaves the orphan list.
- AC-V7 (finding 2, leaving setup is recoverable): mint a
  reference, store a key, leave setup without saving or
  discarding; status lists the reference as an orphaned
  credential (provider + reference, no secret); removing it
  through the guarded op deletes it; the orphan survives a
  restart (persisted ledger) and is still listed.
- AC-V8 (finding 2, referenced keys are protected): a saved
  connection's reference cannot be removed — the op refuses and
  the key stays stored. Removing it while a second setup draft
  holds the same reference unreferenced is allowed; discarding a
  draft whose key another setup saved meanwhile keeps the key.
- AC-V9 (finding 2, failed save): a save that fails persistence
  leaves existing connections usable, keeps the draft inputs for
  retry, and offers discard; discarding after the failed save
  removes the draft's unreferenced key.
- AC-V10 (proof, preservation): adding a second provider
  preserves the first provider and the budget across restart
  (mock keyring, synthetic providers).
- AC-V11 (proof, zero setup traffic): setup (reference, prompt
  store, status, save) makes zero inference calls — fake hit
  counts stay 0 until a real dispatch.
- AC-V12 (proof, browser cannot manage credentials): a paired
  browser session (no grant) cannot set or remove credentials —
  both refused `Forbidden`, state unchanged, refusal recorded.

## Out of scope (unchanged)

- ChatGPT OAuth and Codex (separate slice).
- Verification is still session-scoped: a restart clears it, as
  the UI already states. Persisting verification was not asked
  for.
- The minted ledger covers references this backend minted. A key
  stored by any other means under an unknown label cannot be
  listed — the keyring API cannot enumerate — and the handoff
  says so.
- `providers.spec.ts` 240 s timeout (pre-existing, removed
  'Workspace' button) and the other pre-existing failures are
  not in scope.
