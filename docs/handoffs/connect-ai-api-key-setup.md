# Connect AI — API key setup (first slice) — handoff

Branch: `muse/connect-ai-api-key-setup`, from the SQLite repair tip
`5925c93e6a68e83c3f63e0206dcf5b66669eb668`. Linux implementation +
verification only. Antigravity owns Windows integration; nothing
here is merged, pushed, or promoted. No real credentials, cloud
calls, paid inference, model downloads, or preview promotion.
ChatGPT OAuth and Codex are a separate slice — nothing here builds
an OAuth flow, and no catalog entry claims OAuth for inference
(M5 verification record: API keys are the documented mechanism
for every surface in this slice).

## Commits

| Commit | Contents |
|---|---|
| `2ab8d4d59` | Design + acceptance criteria (AC-C1..C10) before implementation |
| `cfb479de9` | Backend: three new ops + contracts + client methods |
| `3a40248b2` | Guided Connect AI flow in AI Providers |
| `93bc17bbb` | Workflow tests (AC-C3..C8) |
| `5f20e63e6` | rustfmt on touched files |
| `1270c418b` | E2E for the guided flow + restart; providers spec step 1 follows the new flow |

## What was built

### Three new backend operations (`crates/core/src/provider_dispatch.rs`)

- **`providers-configure-add`** (owner-only): adds or updates
  ONE provider entry atomically. Under the backend lock it
  lazy-loads the persisted config (restart case), replaces the
  entry with the same provider id in place or appends it,
  validates the merged config exactly as `providers-configure`
  does (bounds, derived locality, runtime build with shared
  accounting), persists BEFORE installing, then installs. The
  request carries no budget field — the budget can never move
  through this op. Other providers and usage accounting are
  carried over untouched. `providers-configure` (full
  replacement) is unchanged and still works.
- **`providers-credential-reference`** (owner-only): returns
  `{provider}-{16 lowercase hex}` from OS randomness, checked
  against the keyring (`exists`) with bounded retries on
  collision. The reference is a keyring account label, never a
  secret; it is generated on the host so a browser never invents
  account names.
- **`providers-status`** (NO owner check, like `providers-usage`):
  per configured provider — display name, endpoint host,
  locality, enabled, models with cost classes, credential
  reference + `credentialStored` (keyring presence), and
  `connectionVerified` (true only when the in-process usage
  ledger records a completed request for that provider since
  the app started) — plus the configured budget. No secret, no
  usage detail beyond the verified flag.

### Credential prompt

- Reuses the existing Win32 CredUI prompt
  (`provider_credentials.rs`) and the OS keyring service
  `com.robillionair.anansi.providers`. Keys move prompt →
  keyring only; they never appear in browser forms, request
  DTOs, config files, backups, or logs (config files hold
  references only — asserted in the restart test).
- New debug-only test hook `ANANSI_PROVIDER_TEST_CANCEL=1`
  (mirrors the vault's `ANANSI_VAULT_TEST_CANCEL`): makes the
  prompt return the cancelled error, so cancellation is
  testable on Linux.

### Guided Connect AI flow (`packages/ui/src/Providers.tsx`)

- Step 1 — Choose a provider: the catalog (the five slice
  providers are marked "Connect AI": Ollama, OpenAI, OpenRouter,
  Anthropic, Gemini). Step 2 — Endpoint: prefilled from the
  catalog; custom endpoints live in collapsed advanced settings
  (custom toggle + plaintext opt-in, same backend policy).
  Step 3 — Credential (skipped for keyless Ollama/LM Studio):
  host-generated reference, native-prompt set, stored/not-stored
  status, remove/check; a browser refusal shows the desktop
  direction and the flow can continue without a credential.
  Step 4 — Model: id + cost class defaulting to **`unknown`**
  (M5-D5: unknown is treated as paid; the flow never defaults a
  model to `free`), with a Zero-Cost Lock warning. Step 5 —
  Review & save: provider, endpoint, credential state, models,
  and "Your existing budget and other providers are unchanged";
  save calls `providers-configure-add`; a failed save names the
  surviving connections and offers a retry with inputs intact.
- A "Connected providers" list (from `providers-status`) is the
  single source of truth: credential stored / not stored,
  connection verified in this session / not yet verified, plus
  the surviving budget. Zero-Cost Lock stays default-on in
  dispatch and the flow never changes it.
- The old single-provider full-replacement configure form is
  gone from the UI (the operation still exists backend-side).
  Dispatch, usage, and session-access sections are unchanged.

## Verification (Linux, this VM)

| Check | Result |
|---|---|
| Rust lib (`cargo test --locked --workspace --lib`) | **360/360** (baseline 359; +1 cancel unit test; core 293, api 4, desktop 1, providers 62) |
| Provider-dispatch integration | **28/28** (baseline 22; +6 new Connect AI tests) |
| tsc (`pnpm typecheck`) | 0 errors |
| Contracts (`node scripts/contracts.mjs --check`) | 0 drift |
| Docs (`node scripts/validate-docs.mjs`) | 339 PASS (was 338; +1 design doc) |
| UI (`pnpm test:ui`) | **339/339** (was 338; +1 `unknown`-default test) |
| Node suite (`pnpm test`) | 39/40 — only the known pre-existing `pwsh`-missing MSVC-installer environmental failure |
| E2E `connect-ai.spec.ts` (new) | **1/1** — prefill, browser refusal + desktop direction, dispatch verifies the connection, restart keeps both providers + budget, zero calls to the second fake provider |
| E2E `documents-docx.spec.ts` | 7/7 |
| E2E `documents-pdf.spec.ts` | 5/5 |
| E2E `documents-ingestion.spec.ts` | 9/9 |
| E2E `ask_anansi.spec.ts` | 1/1 |

New Rust tests (`crates/core/tests/provider_dispatch.rs`):
- `configure_add_preserves_providers_budget_and_accounting` —
  A with budget, one dispatch, host reference + prompt store,
  add B, assert A + B + budget + usage, status separation
  (A verified / B stored-but-unverified), zero setup traffic,
  unknown-cost B refused under Zero-Cost Lock with zero traffic,
  in-place update of B.
- `configure_add_survives_restart` — fresh `Core` over the same
  root, lazy install via dispatch, both providers + budget +
  credential presence; config file holds the reference, never a
  secret.
- `configure_add_is_atomic_on_persist_failure` — persistence
  broken by planting a file at `provider-config`: the add fails
  `StorageUnavailable`, A stays usable, clean retry succeeds.
- `browser_refusal_leaves_state_unchanged` — add / reference /
  set as an ungranted browser session → `Forbidden` × 3, state
  unchanged, refusal queued for host approval.
- `cancelled_prompt_stores_nothing_and_leaves_config_unchanged`
  — cancel hook → `InvalidRequest` "cancelled", no keyring entry,
  config unchanged, retry succeeds.
- `status_is_readable_by_a_browser_and_shows_no_secrets`.

Pre-existing, unchanged: `providers.spec.ts` still times out at
240 s — it now fails at step 4b waiting for the 'Workspace' nav
button, which Dashboard Slice 1 removed (same root cause as the
`backups.spec.ts` failure). Steps 1–4 pass, including the new
Connect AI step 1. `mcp.rs:715` dead-code warning; `larp-somnia`
instruments test; 4 `semantic_workflow` live-Ollama failures.
None is a regression.

## Windows rerun checklist (for Antigravity)

```powershell
git fetch <bundle> ; git checkout muse/connect-ai-api-key-setup
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

Expected: lib 360/360, provider-dispatch 28/28, tsc 0,
contracts 0 drift, docs 340 PASS (339 + this handoff),
UI 339/339, connect-ai e2e 1/1, DOCX 7/7, PDF 5/5, TXT/MD 9/9,
Ask 1/1. Then the manual Windows checks this slice cannot
prove on Linux: the real CredUI prompt (enter a key, cancel,
empty), the key actually landing in Windows Credential Manager
under `com.robillionair.anansi.providers`, and a real
OpenAI/OpenRouter/Anthropic/Gemini dispatch after setup (owner
authorization required; no live verification is claimed here).

## Proposed status update (for Antigravity's documents — not edited here)

- No requirement-matrix change is proposed: this slice adds
  the Connect AI setup flow and its backend operations; it
  closes no M5 requirement on its own. If Antigravity tracks
  the M5 application integration, record the guided setup flow
  as implemented on this branch, pending Windows verification
  (including the manual CredUI + live-provider checks above).

## Not done / follow-ups

- ChatGPT OAuth and Codex are a separate slice (not started).
- No live-provider verification (no credentials, no paid calls).
- No price table and no model-list fetching from providers
  (model ids are user-entered; cost defaults to unknown).
- The `providers-status` verified flag is per app session
  (in-memory ledger); after a restart a previously verified
  connection shows "not yet verified" until its next dispatch.
  That is the honest reading of the flag, labelled as such in
  the UI — persisting verification state was not in scope.
- `providers.spec.ts` step 4b still waits on the removed
  'Workspace' button (pre-existing; same fix as `backups.spec.ts`
  would need, out of scope here).
