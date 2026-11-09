# Connect AI — API key setup (first slice) — design + acceptance criteria

Branch: `muse/connect-ai-api-key-setup`, from the SQLite repair tip
`5925c93e6a68e83c3f63e0206dcf5b66669eb668`. Linux implementation +
verification only. Antigravity owns Windows integration; nothing
here is merged, pushed, or promoted. No real credentials, cloud
calls, paid inference, model downloads, or preview promotion.
ChatGPT OAuth and Codex are a separate slice — nothing here builds
an OAuth flow, and no catalog entry claims OAuth for inference
(M5 verification record: API keys are the documented mechanism
for every surface in this slice).

## 1. What exists today (read 2026-11-09)

- Catalog (`crates/providers/src/catalog.rs`): 20 entries. The
  five in this slice — Ollama `http://127.0.0.1:11434` (Local,
  Implemented, no auth), OpenAI `https://api.openai.com/v1`
  (Cloud, Implemented, key), Anthropic
  `https://api.anthropic.com` (Cloud, Implemented, key), Gemini
  `https://generativelanguage.googleapis.com/v1beta` (Cloud,
  Implemented, key), OpenRouter `https://openrouter.ai/api/v1`
  (Cloud, CompatibleFamily, key). Endpoints are the verified
  M5 bases; no new provider is invented here.
- `providers-configure` (full replacement): validates bounds +
  derived locality, builds the runtime at configure time,
  persists BEFORE installing under the backend lock, installs
  with shared accounting (`used_permits` + `ledger` survive
  rebuilds). Unchanged by this slice.
- Credentials: configs carry `CredentialRef` only. The OS
  keyring service is `com.robillionair.anansi.providers`
  (`KeyringSecretResolver`: `store_api_key` / `exists` /
  `delete` / `resolve` at dispatch time; keyring mock in tests).
  `providers-credential-set` (owner-only, prompt runs AFTER the
  backend lock is released), `providers-credential-status`
  (owner-only, `present` bool), `providers-credential-remove`
  (owner-only). The native prompt is Win32 CredUI
  (`provider_credentials.rs`: cancel → `InvalidRequest`
  "Credential entry was cancelled."; empty → invalid;
  unavailable → `StorageUnavailable`); debug-only test hook
  `ANANSI_PROVIDER_TEST_CREDENTIAL` (same pattern as the vault).
- The UI (`Providers.tsx`) already has a catalog picker, a
  single-provider configure form that REPLACES the whole config,
  a credential section (native prompt + status), dispatch with
  Zero-Cost Lock default-on, usage, and the session-access
  approval queue. The configure form's model-cost default is
  `free` and its reference is typed by hand — both change in
  this slice.
- Restart restore: `ai_discovery.rs` lazily installs the
  persisted config file on dispatch-ish ops
  (`install_persisted_config`); the file lives at
  `<root>/provider-config/<workspace>.json` (atomic persist,
  fsync + rename). Config files hold references only, never
  secrets — so backups carry refs, not keys.
- `providers-usage` needs no owner check — the precedent for a
  browser-readable status op.

## 2. New backend operations

### `providers-configure-add` — atomic add/update (owner-only)

Request `{ workspaceId, provider: RuntimeProvider }` (one entry;
NO budget field — the budget is never touched by this op).
Response `{ contractVersion, providers: [ids…], added: <id>,
updated: <bool> }`.

Under the backend lock, in order:
1. Owner check (`require_owner_recording`, same as configure).
2. Lazy-load the persisted config when `state.config` is
   `None` (the restart case — `install_persisted_config`
   cannot be called from inside `with_backend`, so the load
   is inlined: `load_config_file` → parse → bounds +
   locality → build runtime with shared accounting).
   When no config exists at all, start from
   `RuntimeConfig::default()` (empty providers, default budget).
3. Replace the entry with the same `config.provider` in
   place (preserving order), or append it. Other providers,
   `config.budget`, and the shared accounting objects are
   carried over untouched.
4. Validate the merged config exactly as configure does
   (bounds, derived locality, runtime build with shared
   accounting). A duplicate model id inside the entry, an
   over-long base URL, or a locality mismatch fails here.
5. Persist BEFORE installing (same `persist_config_file`
   helper); on persist failure return without touching
   `state.config` or the runtime — memory and disk never
   diverge. Then install `state.config` + `state.runtime`
   and return.

A failed save or a cancelled prompt leaves existing
connections usable: the op never mutates state before the
persist succeeds, and the credential prompt is a separate
op that runs before the add (see §4).

### `providers-credential-reference` — host-generated reference (owner-only)

Request `{ workspaceId, provider }` → response
`{ contractVersion, provider, reference }` where `reference`
is `{provider}-{16 lowercase hex}` from `getrandom`, checked
against `resolver.exists()` with bounded retries (8) on
collision. The reference is a label, not a secret; it is
generated on the host so the browser never invents keyring
account names. Validation: provider 1–64 chars, reference
through the existing `validate_reference`.

### `providers-status` — browser-readable setup status (NO owner check)

Request `{ workspaceId }` → response `{ contractVersion,
providers: [{ provider, displayName, baseUrlHost, locality,
enabled, models: [{ id, cost }], credentialReference,
credentialStored, connectionVerified }] , budget }`.
- `credentialStored`: `resolver.exists(reference)` (false
  when the entry has no credential, e.g. Ollama).
- `connectionVerified`: true only when the in-process usage
  ledger records at least one completed request for that
  provider since the app started. It is a session fact, not
  a persisted claim, and the UI labels it "verified in this
  app session". A fresh setup is always unverified — setup
  performs ZERO inference calls, and no "test connection"
  button is added (verification happens on the first
  authorized dispatch, outside setup).
- `budget`: the configured budget, so a browser can see the
  surviving budget without owner rights. No secret, no
  reference value beyond the label, no usage detail beyond
  the verified flag.

### Contracts + routing

- New DTOs are `JsonSchema` structs in `provider_dispatch.rs`;
  `node scripts/contracts.mjs` regenerates the schemas and
  the hand-maintained `packages/contracts/src/client.ts` gains
  the three client methods + validators.
- `crates/api/src/lib.rs` allowlist gains
  `/v1/ai/providers-configure-add`,
  `/v1/ai/providers-credential-reference`,
  `/v1/ai/providers-status` (the route itself is the generic
  `/v1/ai/{operation}` → `ai_command`).
- `ai_discovery.rs` op list gains the three names; the
  configure-add lazy-load is inside the op (see §2), so no
  change to the dispatch-time lazy install.

## 3. Guided Connect AI flow (AI Providers UI)

The configure form becomes a guided flow; the dispatch,
usage, credential and session-access sections stay, and
`providers-configure` (full replacement) stays available as
an operation — the UI no longer offers it as the only save
path.

- Step 1 — Choose provider: the five catalog entries as
  cards (display name, locality, status, docs link) plus the
  rest of the catalog below. Ollama is marked keyless.
- Step 2 — Endpoint: prefilled from the catalog entry.
  Advanced settings (collapsed): custom endpoint toggle +
  custom base URL; the same endpoint policy applies
  (https non-loopback, no userinfo/query/fragment, loopback
  http only, insecure custom needs explicit opt-in and no
  credential — all enforced backend-side).
- Step 3 — Credential (skipped for Ollama): the host
  generates the reference (`providers-credential-reference`);
  "Set credential (native prompt)" opens the host prompt;
  status shows stored / not stored. A browser refusal
  (`Forbidden`) shows "Set this credential in the desktop
  app — browsers cannot enter API keys" and the flow can
  still save the provider entry without a credential (the
  connection then reports credential missing, not broken).
  A cancelled prompt shows "Credential entry was cancelled.
  Nothing was changed — try again." and the flow stays on
  the credential step with the reference intact.
- Step 4 — Model: model id input + cost class select
  defaulting to **`unknown`** (M5-D5: unknown is treated as
  paid; the UI never defaults a model to `free`). The flow
  warns that an unknown-cost model is blocked by Zero-Cost
  Lock until it is verified free or a paid permit is issued.
  Multiple models can be added; the first is the dispatch
  default.
- Step 5 — Review & save: shows provider, endpoint host,
  locality, credential reference + stored state, models with
  cost classes, and the preserved budget note ("Your existing
  budget and other providers are unchanged"). Save calls
  `providers-configure-add`; on success the provider appears
  in the connected list with credential + verification status
  and the flow offers "Connect another provider" or "Done".
  On a failed save the error is shown, the existing
  connections are listed as still usable, and retry returns
  to the review step with all inputs intact.
- The connected list (from `providers-status`) is the
  single source of truth for what is configured: provider,
  models, credential stored / not stored, connection
  verified in this session / not yet verified. It is visible
  to paired browsers; credential entry is desktop-only.
- Zero-Cost Lock stays default-on in dispatch and the flow
  never changes it.

## 4. Ordering rules (the failure matrix)

- Credential-set before configure-add: the prompt runs first
  (outside any lock), then the add. A cancelled prompt means
  no add is attempted.
- Add before credential-set is allowed too (save the entry
  without a credential; the status shows it missing). Either
  order leaves the other providers and the budget intact.
- A failed persist leaves `state.config`, the runtime, the
  file, and every other provider exactly as before.
- A browser calling any owner op gets `Forbidden` and is
  recorded in the approval queue (existing behavior); the UI
  turns that into the desktop direction, not a dead end.

## 5. Acceptance criteria

- AC-C1: Guided flow offers the five catalog providers with
  prefilled endpoints; custom endpoints live in advanced
  settings only; a new model defaults to cost `unknown`
  (never `free`).
- AC-C2: The credential reference is generated on the host
  (`providers-credential-reference`); keys never appear in
  browser forms, request DTOs, files, backups, or logs
  (config files hold references only; the keyring is the
  only secret store).
- AC-C3: `providers-configure-add` is atomic: configure A
  with a budget, add B, and A + the budget + usage
  accounting are unchanged; `providers-configure` still
  works as full replacement.
- AC-C4: After a restart (fresh `Core` over the same storage
  root), both providers and the original budget are present
  (config file reload); credential presence survives
  (keyring); usage accounting within a process survives
  reconfiguration (existing shared-accounting guarantee).
- AC-C5: Browser refusal: a non-owner session calling an
  owner op gets `Forbidden`, state is unchanged, and the UI
  directs the user to the desktop. Cancellation: a
  cancelled prompt leaves no keyring entry and no config
  change, and offers a clean retry. Failed persistence:
  the add fails and the previous config is intact.
- AC-C6: Zero inference calls during setup: the full setup
  sequence (catalog → reference → credential-set → add →
  status) makes zero requests to the synthetic provider.
- AC-C7: Zero-Cost Lock stays on by default; an
  unknown-cost model is refused under the lock without a
  permit (existing gate behavior, asserted in the new
  workflow test).
- AC-C8: Status separates credential-stored from verified
  connection: fresh setup shows stored (after the prompt)
  and not-yet-verified; after a successful dispatch the same
  provider shows verified in this session. No status claims
  a connection is verified before a dispatch succeeds.
- AC-C9: Locked Rust checks clean (lib count recorded vs
  the 359 baseline), provider-dispatch 22/22 + new tests,
  tsc 0, contracts 0 drift, docs PASS, UI suite green,
  focused e2e green. No new failures.
- AC-C10: Delivery is a small checksummed Git bundle ZIP
  with exact SHAs, a Windows rerun checklist, and this
  handoff. No ChatGPT OAuth / Codex work; no real
  credentials, cloud calls, or paid inference.

## 6. Verification plan

1. Rust (new tests in `crates/core/tests/provider_dispatch.rs`
   + a credential-flow test):
   - `configure_add_preserves_providers_budget_and_accounting`:
     fake Ollama A with budget, dispatch once (usage > 0),
     add B (second fake provider id on the same fake
     server), assert A + B + budget in status, usage intact,
     fake hits counted (setup itself adds 0).
   - `configure_add_survives_restart`: fresh `Core` over the
     same root, lazy install via dispatch, assert A + B +
     budget; credential presence via the keyring mock.
   - `configure_add_is_atomic_on_persist_failure`: plant a
     file named `provider-config` so persistence fails;
     assert the add fails and the previous config is intact.
   - `browser_refusal_leaves_state_unchanged`: non-host
     session without a grant → `Forbidden` on add/set/
     reference; config unchanged.
   - `cancelled_prompt_stores_nothing`: test-hook cancel
     env → credential-set fails, `exists` false, config
     unchanged; retry with the hook off succeeds.
   - `unknown_cost_model_blocked_under_zero_cost_lock`:
     add B with an `unknown` model, dispatch under the lock
     without a permit → refused, fake hits 0.
2. Test-hook addition (mirroring the vault's
   `ANANSI_VAULT_TEST_CANCEL`): `ANANSI_PROVIDER_TEST_CANCEL=1`
   makes `prompt_for_api_key` return the cancelled error in
   debug builds. Small, test-only, same pattern as the vault.
3. UI tests: guided-flow step rendering, `unknown` default,
   browser-refusal direction, retry-after-cancel copy (mock
   client).
4. E2E (API-level Node check + one Playwright spec with
   `ANANSI_PROVIDER_TEST_CREDENTIAL` in the API env):
   configure A with budget → add B → restart API → verify
   both + budget; credential status present; Zero-Cost Lock
   on; zero fake-server hits during setup.
5. Full battery: lib, provider-dispatch, tsc, contracts,
   docs, UI, e2e documents-docx/pdf/ingestion/ask (regression
   only — no provider e2e spec is added to the documents set).

## 7. Out of scope

ChatGPT OAuth, Codex, live-provider verification, price
tables (M5-D5: no invented prices), model-list fetching from
providers (model ids are user-entered), Azure/Bedrock/Vertex
(unchanged catalog statuses), preview promotion, pushes,
remote CI. Global status documents are Antigravity's —
proposed updates go in the handoff only.
