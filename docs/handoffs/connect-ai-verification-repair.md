# Connect AI — verification binding + abandoned-credential lifecycle — handoff

Repair branch: `muse/connect-ai-verification-repair`, from the delivered
Connect AI tip `88737ef09a82b7c6913343d74e88cda861f57896`. Linux
implementation + verification only. Antigravity owns Windows
integration; nothing here is merged, pushed, or promoted. No OAuth,
no Codex, no real credentials, no cloud calls. Kept separate from
the pilot candidate: this branch contains only the two acceptance
findings below.

## Commits

| Commit | Contents |
|---|---|
| `0cafc8652` | Design + acceptance criteria (AC-V1..V12) before implementation |
| `6955b22ca` | Reproduction tests for both findings — 10 fail on the delivered tip |
| `bbacd39ca` | Backend fix: fingerprint-bound verification, guarded removal, orphan ledger |
| `3819f512b` | `orphanedCredentials` contract, Unused credentials + Discard draft UI, e2e invalidation + discard |
| `d7e4bce61` | Test-only imports moved into `mod tests` (no new warnings) |

## Finding 1 — "Connection verified" was bound to a provider id, not a connection

Delivered `providers-status` derived `connection_verified` from the
usage ledger: any provider with `requests > 0` since the app started
read verified. A changed endpoint or a replaced credential inherited
the previous connection's status, and a slow success from the old
connection landing after a change verified the new one.

### Fix

- A saved connection gets a fingerprint — provider, endpoint,
  credential reference, custom-endpoint and plaintext flags. The
  model list is deliberately excluded: changing offered models does
  not change the connection.
- `BackendState.verified: HashMap<provider, fingerprint>` records
  the connection whose dispatch last succeeded. Session-scoped,
  in-memory only; a restart clears it, as the UI states.
- `providers-status` reports verified only when the stored
  fingerprint equals the current saved connection's fingerprint.
  The usage ledger is never consulted for verification.
- Recording is bound at both ends: phase 1 snapshots every
  configured provider's fingerprint; on a successful unary
  dispatch the serving provider is `outcome.response.provider`
  (the candidate that actually served) and verification records
  only if the saved connection still matches the snapshot.
  Streams carry the same binding (snapshot + serving provider
  from `disclosure.provider` in the `StreamRun`; recorded when
  polling first observes the terminal `Completed` event — a
  cancelled or errored stream never verifies). Usage and budgets
  are untouched: the runtime settles the ledger independently, so
  a late call still counts as usage without verifying the new
  connection.
- `providers-configure` and `providers-configure-add` drop
  verification for providers whose fingerprint changed or that
  disappeared; unchanged providers keep theirs.
- Rotation under the same reference is invisible to the
  fingerprint, so `providers-credential-set` and
  `providers-credential-remove` invalidate verification for every
  saved provider using that reference after the store/delete.

## Finding 2 — abandoned setup credentials had no lifecycle

Delivered flow: the owner could store a key and then discard the
draft or leave setup, leaving the key in the OS keyring with no
saved connection and no path back. And `providers-credential-remove`
deleted unconditionally — including a key a saved connection still
referenced.

Two facts shaped the fix. The keyring resolver has no enumeration
API (`exists` / `store` / `delete` / `resolve` only; the portable
`keyring` crate cannot list a service's entries), so the backend
cannot discover orphans by scanning the store. And a reference is
only ever a label the host minted: the backend can remember which
references it minted.

### Fix

- Persisted minted-reference ledger
  `<root>/provider-config/<safe-workspace>.minted.json` — a JSON
  object `{reference: provider}`, labels only, never secrets —
  written atomically with the same hardened pattern as the config
  file (the atomic writer is now shared by both files).
  `providers-credential-reference` records each mint (a persist
  failure fails the mint — nothing was stored yet, so a retry is
  safe) and prunes entries that are neither stored nor referenced.
- `providers-status` gains `orphanedCredentials`: minted
  references that are stored in the keyring and not referenced by
  any saved connection. Computed at read time, so a reference
  that becomes referenced stops being an orphan without a write.
- `providers-credential-remove` is guarded: under the backend
  lock (config lazy-loaded, so a restart cannot bypass it), a
  reference used by a saved connection is refused with
  `InvalidRequest` ("in use by a saved connection") and nothing
  is deleted. Otherwise it deletes from the keyring, drops the
  ledger entry, and invalidates verification for that reference.
  Owner-only as before; a paired browser is still refused
  `Forbidden` and recorded.
- UI: "Discard draft" in the flow (steps 2–5, and on a failed
  save next to the retry) resets the draft; when the draft stored
  an unreferenced key it removes it through the guarded op and
  reports the removal; if the key became referenced meanwhile the
  backend refuses and discard reports it was kept; in a paired
  browser the key stays stored and listed under the new "Unused
  credentials" section of Connected providers, with a Remove
  button per entry. Leaving setup without discarding keeps the
  key stored — it appears there too. The credential step's Remove
  button is disabled with an explanation when the draft's
  reference is in use by a saved connection. A failed save keeps
  its inputs for retry (existing) and now also offers discard.

## Verification (Linux, this VM)

Before/after: the 11 new reproduction tests ran against the
delivered tip first — **10 failed** (the 11th, browser refusal,
already held) — and all pass after the fix. The two failure modes
observed before the fix were exactly the findings: an endpoint
change left `connectionVerified: true`, and removing a referenced
credential returned `{"removed": true}`.

| Check | Result |
|---|---|
| Rust lib (`cargo test --locked --workspace --lib`) | **362/362** (was 360; +2 unit tests: ledger round-trip + fingerprint) |
| Provider-dispatch integration | **39/39** (was 28; +11 new) |
| tsc (`pnpm typecheck`) | 0 errors |
| Contracts (`node scripts/contracts.mjs --check`) | 0 drift |
| Docs (`node scripts/validate-docs.mjs`) | 341 PASS (was 340; +1 design doc) |
| UI (`pnpm test:ui`) | **340/340** (was 339; +1 orphan-list render test) |
| Node suite (`pnpm test`) | 39/40 — only the known pre-existing `pwsh`-missing MSVC-installer environmental failure |
| E2E `connect-ai.spec.ts` (extended) | **1/1** — now also proves endpoint change clears verification across restart (no call to the new endpoint) and the Discard-draft reset after a failed save |
| E2E `documents-docx.spec.ts` | 7/7 |
| E2E `documents-pdf.spec.ts` | 5/5 |
| E2E `documents-ingestion.spec.ts` | 9/9 |
| E2E `ask_anansi.spec.ts` | 1/1 |

New Rust tests (`crates/core/tests/provider_dispatch.rs`):
- `verified_does_not_survive_endpoint_change` (AC-V1),
  `verified_does_not_survive_new_reference_rotation` (AC-V2),
  `verified_does_not_survive_rotation_under_same_reference`
  (AC-V3, incl. re-verification on the next success),
  `delayed_success_never_verifies_new_connection` (AC-V4 unary,
  slow dispatch in flight across an endpoint change; usage still
  counts the late request, budget unchanged, new endpoint never
  called), `delayed_stream_success_never_verifies_new_connection`
  (AC-V4 stream), `unchanged_provider_keeps_verification_when_other_changes`
  (AC-V5), `orphaned_credential_listed_and_survives_restart`
  (AC-V7, incl. the ledger file on disk holding the reference and
  no secret), `orphaned_credential_removed_through_guarded_op`
  (AC-V6/V7, AC-V11 zero setup traffic),
  `referenced_credential_cannot_be_removed` (AC-V8),
  `failed_save_leaves_key_orphaned_and_removable` (AC-V9 backend
  half: persist failure → orphan listed → removable → clean retry
  succeeds), `browser_cannot_set_or_remove_credentials` (AC-V12,
  both writes refused `Forbidden`, nothing stored or deleted,
  refusal recorded).
- AC-V10 (second provider preserves the first + budget across
  restart) is the pre-existing `configure_add_survives_restart`
  test, rerun green; the e2e re-proves it at the browser level.
- Pre-existing test updated to the new rule:
  `credential_lifecycle_prompt_keyring_resolver_wire` now removes
  the provider from the saved config before removing its
  credential, then re-adds the provider under the same reference
  and proves dispatch fails closed (`Unauthorized`, zero
  additional traffic) without the key.

Pre-existing, unchanged: `providers.spec.ts` 240 s timeout
(removed 'Workspace' button); `larp-somnia` instruments test; 4
`semantic_workflow` live-Ollama failures; `mcp.rs:715` dead-code
warning (the only build warning). None is a regression.

## Windows rerun checklist (for Antigravity)

```powershell
git fetch <bundle> ; git checkout muse/connect-ai-verification-repair
node scripts/cargo.mjs test --locked --workspace --lib
node scripts/cargo.mjs test --locked -p anansi-core --test provider_dispatch
pnpm typecheck
node scripts/contracts.mjs --check
node scripts/validate-docs.mjs
pnpm test:ui
node scripts/cargo.mjs build --locked -p anansi-api
cd apps/web ; node node_modules/vite/bin/vite.js build ; cd ../..
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/connect-ai.spec.ts
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/documents-docx.spec.ts
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/documents-pdf.spec.ts
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/documents-ingestion.spec.ts
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/ask_anansi.spec.ts
```

Expected: lib 362/362, provider-dispatch 39/39, tsc 0,
contracts 0 drift, docs 342 PASS (341 + this handoff),
UI 340/340, connect-ai e2e 1/1, DOCX 7/7, PDF 5/5, TXT/MD 9/9,
Ask 1/1. Then the manual Windows checks Linux cannot prove:
the real CredUI prompt (enter, cancel, empty), the key landing
in Windows Credential Manager, a real dispatch after setup, and
the orphan lifecycle end to end on the desktop — store a key,
discard the draft, confirm the key is gone from Credential
Manager; store a key, leave setup, confirm it appears under
Unused credentials and can be removed there; try removing a key
a saved connection uses and confirm the refusal.

## Proposed status update (for Antigravity's documents — not edited here)

- No requirement-matrix change is proposed: this repair closes
  the two acceptance findings against the Connect AI first slice
  on this branch, pending Windows verification (including the
  manual CredUI + live-provider + orphan-lifecycle checks above).

## Not done / known limits

- Verification is still session-scoped: a restart clears it, as
  the UI states. Persisting verification was not asked for.
- The minted ledger covers references this backend minted. A key
  stored by any other means under an unknown label cannot be
  listed — the keyring API cannot enumerate — and the UI copy
  does not claim otherwise.
- `providers.spec.ts` step 4b still waits on the removed
  'Workspace' button (pre-existing; out of scope).
- ChatGPT OAuth and Codex remain a separate slice (not started).
