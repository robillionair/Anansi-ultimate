# "Continue with ChatGPT" — feasibility + host-side sign-in prototype — handoff

Branch: `muse/connect-ai-chatgpt-signin-spike`
Base: Connect AI verification repair `54347381ffa2f8c8f9e4a869f0a120e095a3672c`
Tip: the handoff commit on this branch (exact SHA in the delivery
message); last code commit `79c4dd61889b5a4246a6ede58052360408c6c490`
Date: 2026-11-10 (Linux evidence only — **WINDOWS REVIEW OUTSTANDING**)
Design (committed before implementation): `docs/handoffs/connect-ai-chatgpt-signin-design.md`
(`ddc51873e`, AC-S1..S12)

## 1. What was delivered

An isolated feasibility + prototype slice for "Continue with ChatGPT"
(Sign in with ChatGPT, open-source token-sharing flow). No live OAuth,
no real credentials, no paid calls, no generated-code execution, no
pushes, no preview promotion. The frozen Windows pilot and the existing
API-key flow are untouched; the only new surface is four
`chatgpt-signin-*` ops plus a development-gated UI section.

Commits on the branch (base `54347381f`):

- `ddc51873e` — design + acceptance criteria (AC-S1..S12) before code.
- `715c8161e` — host-side prototype ops + contracts (AC-S2..S8 backend).
- `435bad3ee` — 21 integration tests vs synthetic issuer (AC-S2..S9).
- `79c4dd618` — dev-gated prototype UI + fmt (AC-S10). Tip.

Files changed vs base (17 files, +4231/−20): `crates/core/src/chatgpt_signin.rs`
(new), `crates/core/src/provider_dispatch.rs`, `crates/core/src/lib.rs`,
`crates/core/src/ai_discovery.rs`, `crates/api/src/lib.rs`,
`crates/contracts/src/main.rs`, `crates/core/Cargo.toml` (+ `ring =0.17.14`,
`base64 =0.22.1`, already in Cargo.lock transitively; lockfile +2 lines),
`crates/core/tests/chatgpt_signin.rs` (new), `packages/ui/src/Providers.tsx`,
`packages/ui/tests/components.test.tsx`, generated contracts
(`packages/contracts/src/*`), `docs/handoffs/connect-ai-chatgpt-signin-design.md`.

## 2. Evidence (Linux, this VM)

- Rust lib: **366 passed** (362 baseline + 4 new `chatgpt_signin` unit
  tests), 0 failed.
- Integration `chatgpt_signin`: **21/21 passed**.
- Integration `provider_dispatch` (regression): **39/39 passed**.
- TypeScript: `tsc --noEmit` 0 errors.
- Contracts: `node scripts/contracts.mjs --check` exit 0, 0 drift.
- Docs validation: **21/21 passed**.
- UI suite (`pnpm test:ui`): **342/342 passed** (340 baseline + 2 new).
- Node suite (`pnpm test`): **39/40** — the one failure is the
  pre-existing MSVC installer test
  ("MSVC installer default mode is read-only and explicitly scoped"),
  confirmed failing identically on the base commit `54347381f` in a
  pristine worktree. Not a regression.
- e2e regressions (lease-run, one spec per invocation):
  connect-ai **1/1**, DOCX **7/7**, PDF **5/5**, TXT/MD **9/9**, Ask **1/1**.
- Warnings: one, pre-existing — `crates/core/src/mcp.rs:715`
  (`preserved_env_entries` never used). A second warning my code
  introduced (`unreachable pattern` in `chatgpt_complete_callback`)
  was fixed at its root cause by removing the genuinely unreachable
  arm; no warning remains from this slice.
- `rustfmt` applied to the four touched Rust files only. Lesson
  recorded in `~/AGENTS.md`: `rustfmt` on a crate root formats the
  whole module tree — it reflowed eight untouched files here, all
  reverted before committing.

## 3. Acceptance criteria — how each was met

- **AC-S1 (routes verified):** the three official pages plus the two
  linked siblings (Accounts and sessions; Errors and recovery) were
  read in full; the operative sign-in page was spot-checked verbatim
  against the research digest. Design §1.3 records buildable-now /
  needs-registration / needs-real-verification. Design + AC committed
  as `ddc51873e` before any implementation.
- **AC-S2 (start):** `chatgpt-signin-start` returns the documented
  authorization URL — first time `client_id=dynamic_agent_client` +
  `agent_name_hint=ANANSI` + persisted `ext_agent_host_id`;
  returning account reuses the saved issued `client_id` without
  `agent_name_hint`; `redirect_uri` is
  `http://127.0.0.1:<bound-port>/auth/callback` with the listener
  bound before the URL is built; PKCE S256; fresh `state` + `nonce`
  per attempt; six documented scopes;
  `resource=https://api.openai.com/v1`. Proven by
  `signin_start_returns_documented_authorization_url` and
  `signin_reauthorization_uses_saved_client_id_and_hint`.
- **AC-S3 (callback validation):** wrong state → failed, no exchange,
  nothing stored; replayed callback → rejected (listener stays alive
  until the attempt TTL so the replay reaches the handler instead of
  a refused connection); `error=access_denied` with valid state →
  failed, no exchange; different `client_id` → rejected; new
  registration without an issued ID → incomplete. Proven by
  `signin_wrong_state_is_refused_without_exchange`,
  `signin_replayed_callback_is_rejected`,
  `signin_denied_consent_stops_before_exchange`,
  `signin_callback_client_id_mismatch_is_rejected`,
  `signin_new_registration_without_client_id_is_incomplete`.
- **AC-S4 (token + scope validation):** code exchange against the
  synthetic token endpoint; ID token validated (issuer, audience =
  issued client ID, expiry, nonce, `sub`, RS256 signature vs the
  synthetic JWKS, via `ring` `RsaPublicKeyComponents::verify`); wrong
  nonce → refused; expired ID token → refused; tampered signature →
  refused; missing `chatgpt.tokens.use.direct` → identity-only
  sign-in retained, plan usage disabled, no inference path. Proven by
  `signin_full_flow_stores_tokens_in_keyring_only`,
  `signin_wrong_nonce_is_refused`, `signin_expired_id_token_is_refused`,
  `signin_tampered_signature_is_refused`,
  `signin_missing_plan_scope_is_identity_only`.
- **AC-S5 (storage):** tokens live only in the OS keyring under the
  distinct service `com.robillionair.anansi.chatgpt`, reference
  `chatgpt-<client_id>`; registrations under
  `<root>/provider-config/<safe-wid>.chatgpt.json` hold labels only;
  host id in `<root>/provider-config/chatgpt-host.json`; status
  returns labels only; the keyring entry is removed on disconnect.
  Proven by the full-flow test (keyring-mock assertions) and
  `browser_status_shows_labels_only`.
- **AC-S6 (expiry + refresh):** expired access token refreshed via
  the `refresh_token` grant; rotating refresh token replaced together
  with the access token; concurrent refreshes serialized by the
  in-flight flag (one exchange, one stored record);
  `invalid_grant` on refresh clears the record and status reports
  needs-sign-in. Proven by `signin_refresh_rotates_tokens`,
  `signin_refresh_is_serialized`, `signin_refresh_invalid_grant_clears_record`,
  `signin_refresh_nothing_to_do_when_token_is_fresh`. (A transient
  network failure keeps the record — the transport error path is the
  same `StorageUnavailable` mapping used by the other provider ops;
  no dedicated synthetic test was added for it.)
- **AC-S7 (disconnect):** revocation attempted at the issuer's
  `revocation_endpoint` from `.well-known/openid-configuration`;
  local tokens always cleared; unconfirmed revocation is reported
  (`remote_revocation_confirmed: false`) and the user is directed to
  ChatGPT Settings; the registration stub (client ID + email)
  survives for re-sign-in. Proven by
  `signin_disconnect_revokes_and_clears` and
  `signin_disconnect_unconfirmed_revocation_is_reported`.
- **AC-S8 (browser boundaries):** a paired browser session cannot
  start, refresh, or disconnect (each refused `Forbidden` and
  recorded in the approval queue); `chatgpt-signin-status` from a
  browser returns labels only. Proven by
  `browser_cannot_start_refresh_or_disconnect` and
  `browser_status_shows_labels_only`.
- **AC-S9 (regression):** after a failed sign-in, a local Ollama
  dispatch still succeeds and an API-key provider can still be
  configured and dispatched (explicit `chooseModel` selection —
  auto mode scores all models and picks the free local one, which is
  the router's existing behaviour, not a sign-in effect); the full
  provider-dispatch suite stays green (39/39). Proven by
  `failed_signin_leaves_local_and_apikey_flows_working`.
- **AC-S10 (dev gate):** the prototype UI section is absent by
  default — the full Providers render contains no "Continue with
  ChatGPT" text; with `window.__ANANSI_CHATGPT_SIGNIN_DEV__ === true`
  it renders status labels with the sign-in button disabled and the
  "live sign-in is not yet available" note. No production code path
  sets the flag. Proven by two new component tests (UI suite 342).
- **AC-S11 (decision):** §5 below (carried from the design, verdict
  unchanged) + the Codex note in §6.
- **AC-S12 (delivery):** this handoff + the checksummed bundle ZIP
  (see §7). No live OAuth, real credentials, paid calls,
  generated-code execution, pushes, or preview promotion.

## 4. Design notes the reviewer should know

- **Debug-only issuer override:** `ANANSI_CHATGPT_TEST_ISSUER` is
  accepted only for `http://127.0.0.1:<port>` and refused otherwise;
  it is compiled out in release builds. The prototype is executable
  in tests without contacting OpenAI.
- **Listener lifetime:** the loopback listener thread stays alive
  until the attempt TTL (600 s) rather than exiting after the first
  callback, so a replayed callback gets the "already used" page
  instead of a refused connection.
- **Synthetic issuer:** the test file embeds a committed TEST RSA key
  (PKCS#8 DER, kid `test-key-1`) and a loopback issuer serving
  discovery / JWKS / token / revocation. It is test-only and never
  reaches production paths.
- **Error mapping:** `ChatGptError::Network` →
  `ErrorCode::StorageUnavailable`; everything else →
  `InvalidRequest` with the message. Owner-only ops
  (`start`/`refresh`/`disconnect`) go through
  `require_owner_recording`; `status` is browser-readable.
- **Two test-side fixes during the run (both in the test, not the
  product):** the dispatch response field is `response`, not `body`;
  and a second immediate refresh rotates again when the record is
  inside the 300 s refresh window — the chaining assertion now
  covers rotation, with a separate test for the fresh-token no-op.

## 5. Decision memo (unchanged from the design)

**Verdict: feasible as specified; not yet eligible to ship.**

The OSS dynamic-registration flow is fully specified and buildable
without prior registration, and this spike proves the host-side
mechanics against a synthetic issuer. What is NOT established on
this VM — and cannot be established by reading docs — is whether
ANANSI counts as an eligible open-source partner, whether a real
Plus/Pro account receives the plan scopes, which models/routes that
account can call, token lifetimes, and preview limitations.

**Exact owner action needed:** on a Windows machine with a ChatGPT
Plus or Pro account, attempt one real dynamic registration through
the (still gated) prototype flow, or obtain written confirmation
from OpenAI that ANANSI qualifies as an open-source partner. Do not
enable the sign-in button before that attempt succeeds and the plan
scopes are granted.

**Smallest path to one authorized, synthetic-data Responses turn
after eligibility is confirmed:**

1. Eligibility attempt succeeds (owner action above); the issued
   `client_id` + plan scopes are on record.
2. Add a `chatgpt-plan` provider entry to the existing dispatch
   path: `Authorization: Bearer <OAuth access token>`,
   `resource=https://api.openai.com/v1`, `POST /v1/responses`,
   scope-gated (refuse when `chatgpt.tokens.use.direct` is
   absent), plan-usage errors surfaced verbatim (429
   `subscription_sharing_usage_limit_exceeded` → pause + link to
   ChatGPT Settings → Usage; 403
   `subscription_sharing_user_not_eligible` → stop, do not loop
   through OAuth; never fall back to API-key billing). Cost class
   for plan calls is a decision for that slice: plan usage consumes
   the user's shared five-hour Plus limit, so it is not "free" —
   treat as `unknown` (blocked by Zero-Cost Lock without a permit)
   until that slice decides otherwise.
3. Synthetic-data Responses turn in tests: the synthetic issuer
   stands in for `api.openai.com`; one turn with synthetic user
   text, asserting the bearer token, the resource, and the
   plan-usage error mapping.
4. Windows manual verification: real browser handoff, the token
   record in Windows Credential Manager under the ChatGPT service,
   one real Responses turn on synthetic data, and the disconnect +
   re-sign-in path.

## 6. Codex app-server integration (documented separately, not built)

Per the app-server page: after exchanging the code, set
`ACCESS_TOKEN` in the app-server child process's environment to the
OAuth access token and launch `codex app-server --listen stdio://`
with `model_provider="openai_chatgpt_plan"`,
`base_url="https://api.openai.com/v1"`, `env_key="ACCESS_TOKEN"`,
`wire_api="responses"`, `requires_openai_auth=false`,
`supports_websockets=false`. Drive it over newline-delimited JSON
on stdin/stdout: `initialize` (`clientInfo.name` stable across
installations, matching `agent_name_hint`), `initialized`,
`thread/start`, `turn/start`, display `item/agentMessage/delta`
events; only `turn.status == completed` is success. On token
renewal, restart app-server with the updated `ACCESS_TOKEN` and
resume with `thread/resume` + the saved thread ID.

**ANANSI's execution boundary (the decision this spike records, not
a docs claim):** the app-server page does not describe Codex's tool,
shell, or file-execution behaviour. ANANSI's standing rule is no
generated-code execution and no automatic tool execution. Therefore
a Codex integration is a separate slice requiring its own
authorization design: what Codex may execute, on whose machine,
under which sandbox, with which owner approval per action. This
spike builds no Codex process, ships no Codex binary, and obtains
no Codex binary. How the app obtains a Codex binary (download,
version pinning, verification) is unverified — the page assumes
`codex` is installed.

## 7. Windows rerun checklist

1. Apply the bundle ZIP on Windows; confirm the tip stated in the
   delivery message (handoff commit; last code commit
   `79c4dd61889b5a4246a6ede58052360408c6c490`) and a clean tree.
2. Run the Rust battery: `cargo test --locked --workspace --lib`
   (expect 366), `--test provider_dispatch` (39/39),
   `--test chatgpt_signin` (21/21).
3. Run `tsc --noEmit` (0), contracts `--check` (0 drift),
   docs validation (21/21), UI suite (342/342).
4. Run the e2e regressions: connect-ai 1/1, DOCX 7/7, PDF 5/5,
   TXT/MD 9/9, Ask 1/1.
5. Manual (dev-gated) browser handoff: set the dev flag, start a
   sign-in from the desktop host, confirm the system browser opens
   `auth.openai.com` with the loopback callback, and that a paired
   browser cannot start/refresh/disconnect (Forbidden + approval
   queue entry).
6. Manual Credential Manager check: after a real sign-in, the token
   record exists under the ChatGPT service
   (`com.robillionair.anansi.chatgpt`); no token appears in logs,
   exports, or backups; disconnect removes the record and the
   registration stub survives for re-sign-in.
7. Manual orphan/registration check: a second ChatGPT account keeps
   its own issued client ID and credentials separate; signing out
   and back does not create a new client.
8. **Do not** enable the sign-in button, attempt a live Responses
   turn, or start the Codex app-server slice until the §5 owner
   action (eligibility) is confirmed.

## 8. Out of scope (unchanged)

- Any live OAuth, real credentials, paid calls, Responses turns, or
  model/route eligibility claims.
- Any change to the API-key flow, the frozen Windows pilot, or any
  provider op/contract except the four new `chatgpt-signin-*` ops.
- `providers.spec.ts` 240 s timeout (pre-existing, removed
  'Workspace' button) and the other pre-existing failures
  (larp-somnia instruments test, backups.spec.ts 'Workspace'
  button, 4 semantic_workflow live-Ollama failures, node pwsh
  failure, MSVC installer test — all pre-existing on this VM).
- Token lifetimes / Preview limitations / Models and inference /
  UI/UX Guidelines pages (unread; listed in the design §1.3 as
  needing reading before a live slice).
- No global status document was edited (Antigravity-owned files
  untouched); proposed updates, if any, belong in this handoff only.
