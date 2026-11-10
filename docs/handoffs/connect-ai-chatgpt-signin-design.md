# "Continue with ChatGPT" — feasibility + host-side sign-in prototype — design

Spike branch: `muse/connect-ai-chatgpt-signin-spike`, from the delivered
Connect AI repair tip `54347381ffa2f8c8f9e4a869f0a120e095a3672c`. Linux
implementation + verification only. Antigravity owns Windows
integration. Nothing here is merged, pushed, promoted, or
preview-promoted. No live OAuth, no real credentials, no paid calls,
no generated-code execution. The frozen Windows pilot and the
existing API-key flow are independent of this work and untouched by
it: no API-key op, contract, or UI behaviour changes except the
addition of a development-gated prototype section.

## 1. What the official routes say (verified 2026-11-10)

Read in full via text fetch, and spot-checked by the parent on the
operative page:

- https://developers.openai.com/siwc/quickstart
- https://developers.openai.com/siwc/token-sharing-open-source/sign-in
- https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server

Plus two directly-linked siblings the research followed:
Accounts and sessions (refresh/revocation/multi-account) and Errors
and recovery (plan-not-enabled, error codes, disconnection). Pages
NOT read and therefore NOT relied on: Preview limitations, Models
and inference, Token lifetimes, UI/UX Guidelines, "A client vs. an
agent host", "Managing ChatGPT accounts", "On your website" (the
JWKS signature-validation example).

### 1.1 The four things, kept distinct

- **ChatGPT identity** — who the user is. Scopes `openid profile
  email`. The validated ID token yields a stable `sub`, name, email,
  picture. "Identity scopes don't grant access to ChatGPT
  conversations or OpenAI API resources." Identity alone never
  authorizes inference.
- **ChatGPT plan usage** — whose quota a call consumes. Scopes
  `offline_access resource.invoke chatgpt.tokens.use.direct`,
  `resource=https://api.openai.com/v1`, access token as
  `Authorization: Bearer` on `POST /v1/responses`. The call is
  billed to the user's ChatGPT Plus/Pro plan, never to an API key.
  Plus users share one five-hour usage limit across *all* apps;
  Pro users have no five-hour limit. Errors never silently fall
  back to another billing path.
- **OpenAI project-key usage** — the separate, existing ANANSI
  path: a project API key the user enters, billed to their OpenAI
  project. The docs position it as the fallback billing path when
  plan usage is disabled or the user is ineligible ("configure
  another supported billing path, such as the user's own API
  key"). Nothing in SIWC describes project-key management; it is
  outside SIWC and unchanged here.
- **Codex execution** — a separate layer on top of plan usage.
  The Codex binary runs as a child process on the user's machine,
  driven over stdio JSON-RPC, and sends inference to
  `/v1/responses` with the user's OAuth token. ANANSI's job in
  that integration would be token supply + process lifecycle +
  conversation driving; Codex itself performs the agentic work.
  The app-server page does **not** describe Codex's tool, shell,
  or file-execution behaviour or sandbox. See §6: the execution
  boundary is a separate decision and is NOT settled by these
  docs — and not built in this slice.

### 1.2 The open-source sign-in flow (the operative route for ANANSI)

Dynamic client registration — no prior registration, no client
secret, no partner API key:

1. First time: authorize with `client_id=dynamic_agent_client`,
   `ext_agent_host_id` (a stable per-host identifier, persisted
   before first sign-in and reused for the same host),
   `agent_name_hint` = the app's actual name, used consistently.
   After the user names and authorizes the agent, the callback
   returns the issued `client_id` (e.g. `oaiapp_...`). Save it
   with the registration and reuse it for later sign-ins.
   `dynamic_agent_client` is the first-time entrypoint, never
   the ID to save or use for token exchange.
2. Authorization request — system browser at
   `https://auth.openai.com/api/accounts/authorize`, all values
   URL-encoded: `response_type=code`,
   `redirect_uri=http://127.0.0.1:<port>/auth/callback` (listener
   started before opening the browser; only the port may vary;
   scheme/host/path fixed; never `localhost`),
   `scope=openid profile email offline_access resource.invoke
   chatgpt.tokens.use.direct`, `resource=https://api.openai.com/v1`,
   fresh random `state` and OIDC `nonce` per attempt, PKCE
   `S256` (`code_challenge` = base64url SHA-256 of the verifier,
   no padding). Returning accounts use the saved issued
   `client_id` (+ optional retained `id_token_hint`, which may
   be expired, and/or `login_hint` email); `agent_name_hint`
   only on initial registration.
3. Callback: new registration returns `code`, `state`, issued
   `client_id` (and possibly `scope`). On reauthorization the
   callback may omit `client_id` — retain the pending request's
   ID; a *different* supplied ID is rejected. A new-registration
   callback without an issued ID is incomplete. `error=
   access_denied` (after validating `state`) stops the attempt
   with no code exchange.
4. Code exchange: form-encoded `authorization_code` grant to
   `https://auth.openai.com/api/accounts/oauth/token` with the
   issued `client_id`, `code`, `code_verifier`, the same
   `redirect_uri`, and the same `resource`. No client secret.
   `invalid_grant` → discard the code, start a fresh
   authorization with the issued client ID retained.
5. ID-token validation: verify the signature against OpenAI's
   published JWKS; check issuer, audience against the issued
   client ID, expiration, and the nonce saved for this attempt.
   Use the validated `sub` as the account identity. For a
   returning account, confirm the new identity matches the
   selected account before replacing credentials. Check the
   token response's granted scopes for
   `chatgpt.tokens.use.direct` before proceeding to inference —
   **a valid ID token alone does not authorize ChatGPT plan
   usage.**
6. Credential record — one per issued client ID + verified
   identity: validated identity, tokens, granted scopes,
   expiry; retain the ID token for later `id_token_hint`; keep
   the stable host ID with its runtime. Store under an
   app-defined path, atomically, owner-only permissions, never
   committed or logged. Replace access token, expiry, granted
   scopes, and the rotating refresh token together after a
   successful refresh.
7. Refresh: standard OAuth refresh near expiry — form-encoded
   `grant_type=refresh_token`, the issued `client_id` saved
   with the token set (not `dynamic_agent_client`), the saved
   `refresh_token`, `resource=https://api.openai.com/v1`, to
   the same token endpoint; omit `scope` to retain the grant.
   Serialize refreshes for the same session so two processes
   do not race a rotating token.
8. Revocation (sign-out): POST the refresh token to the
   `revocation_endpoint` from
   `https://auth.openai.com/.well-known/openid-configuration`
   (`token=<REFRESH_TOKEN>`, `token_type_hint=refresh_token`,
   issued `client_id`). Empty HTTP 200 is success, including
   for an already-invalid token. On network failure/5xx, retry
   with backoff while the refresh token is available; if
   sign-out completes locally without confirmed revocation,
   clear the local tokens and tell the user remote revocation
   was not confirmed (they can disconnect in ChatGPT
   Settings). Revoking the session does not delete the
   registered client.
9. Disconnection: OpenAI does not notify the tool when a user
   disconnects in ChatGPT Settings. When a request or refresh
   confirms the disconnection, stop using the invalid token
   set and ask the user to sign in again. Do not erase
   credentials solely because of a temporary network or
   infrastructure failure.
10. Token hygiene: tokens live in protected local runtime
    storage only — never in browser storage, source control,
    logs, analytics, or support transcripts. Never put access
    or refresh tokens in URLs. Send an ID token as
    `id_token_hint` only to the OpenAI authorization endpoint,
    and redact authorization URLs containing that hint from
    logs and diagnostics.

### 1.3 What ANANSI can build now / what needs registration / what needs real verification

| Item | Status per the docs |
|---|---|
| OIDC + PKCE sign-in via system browser, loopback `127.0.0.1` `/auth/callback` | Buildable now — endpoints and parameters fully specified |
| First-time dynamic registration (`dynamic_agent_client` + `ext_agent_host_id` + `agent_name_hint`) | Buildable now — no prior registration step exists in the docs for the OSS flow; the issued `client_id` comes back in the callback |
| Code exchange, no client secret (public client) | Buildable now |
| ID-token validation against JWKS (issuer, audience = issued client ID, expiry, nonce) | Buildable now (this spike implements it with `ring` RS256 + the issuer's published JWKS; see §3) |
| Refresh (rotating tokens, serialized) and revocation via `.well-known/openid-configuration` | Buildable now |
| Responses API calls with the OAuth access token, plan usage gated on `chatgpt.tokens.use.direct` | Buildable now as specified; model/route eligibility is enforced at request time (error codes) — the Responses turn itself is a *next* step, not this slice (§5) |
| Identity-only sign-in (`openid profile email`) | Buildable now; works without plan scopes |
| Codex app-server integration (child process, `ACCESS_TOKEN` env, stdio JSON-RPC) | Specified — but requires a Codex binary the app must obtain (docs do not say how) and a separate execution-boundary decision (§6); NOT built here |
| "Continue with ChatGPT" button, approved branding | Buildable in code, but this slice forbids shipping any enabled sign-in button before live eligibility + authorization are verified; the UI/UX Guidelines page was not read |
| Commercial-partner SIWC (website sign-in, plan usage with "available credits") | Needs registration/approval — "currently available to selected commercial partners through a limited trial" |
| `force_reconsent=true` | Needs OpenAI confirmation — "only after OpenAI confirms deployment for your integration" |
| ANANSI qualifying as an "open-source partner" | **Needs real verification** — the docs never define "open-source partner"; ANANSI's repo is public on GitHub, but whether that satisfies OpenAI is a docs claim, not a verified fact |
| Which models/routes are actually eligible for a given Plus/Pro user | **Needs real verification** — enforced at inference time; `model/list` is "a catalog, not an entitlement check" |
| ChatGPT plan usage for Free-tier users | Not available per the docs — only "Eligible ChatGPT Plus and Pro users"; a Free user gets identity only, and the app must offer another billing path (API key) |
| Windows desktop specifically | **Not addressed on any page** — no Windows exclusion or inclusion, no desktop-packaging guidance; the flow is loopback-based and platform-agnostic; storage examples are Unix (`~/.config`, `0600`), so the Windows equivalent is the app's own choice (ANANSI already uses the OS keyring) |
| Token lifetimes / renewal limits | Needs reading — lives on the unread "Token lifetimes" page |
| Preview limitations | Unread — may contain eligibility or capability constraints that change the feasibility verdict |

**Account/plan claims needing real verification (owner action, §5):**
(1) that ANANSI counts as an eligible open-source partner for the
dynamic-registration flow; (2) that a real Plus/Pro account can
complete registration and receive the plan scopes; (3) which
models/routes a real account can actually call (enforced at
request time); (4) token lifetimes and renewal limits; (5) any
preview limitations. None of these is established by reading the
docs, and none is claimed here.

## 2. Prototype scope (this slice)

A host-side sign-in prototype against a **synthetic local OAuth
server** (a fake issuer in tests, speaking the documented
endpoints: `/api/accounts/authorize` is simulated by the test
driving the callback directly, `/api/accounts/oauth/token`,
`/.well-known/openid-configuration`, `/jwks`). The prototype is
executable in tests without contacting OpenAI. It covers:

- browser handoff (the host builds the authorization URL and
  opens the system browser; the test simulates the browser by
  issuing the callback GET to the loopback listener),
- loopback callback (`http://127.0.0.1:<ephemeral>/auth/callback`,
  listener bound before the URL is returned),
- PKCE S256, fresh `state` + `nonce` per attempt, constant-time
  state comparison, single-use attempts (replay rejected),
- token + scope validation (token response shape, ID-token
  issuer/audience/expiry/nonce/sub, RS256 signature against the
  issuer JWKS, granted-scope gate for plan usage),
- expiry, refresh (serialized, rotating tokens replaced
  together), disconnect (revocation attempt + local clear +
  unconfirmed-revocation reporting),
- storage boundaries (tokens only in the OS keyring under a
  distinct service; callback codes never logged; status returns
  labels only; nothing in exports or backups),
- browser boundaries (a paired browser cannot read credentials
  or initiate host-only login),
- regression (local models and API-key setup still work after a
  failed sign-in).

NOT in this slice: any Responses API call (the next-step path is
documented in §5, not built), any Codex binary or app-server
process (§6), any enabled sign-in button (§4), any live OAuth.

## 3. Design

### 3.1 Module: `crates/core/src/chatgpt_signin.rs`

Pure sign-in logic, no `Core` dependency beyond the caller:

- `ChatGptEndpoints { issuer_base }` — production
  `https://auth.openai.com`; a test-only override via env
  `ANANSI_CHATGPT_TEST_ISSUER`, accepted **only** when it is an
  `http://127.0.0.1:<port>` URL (loopback-locked; any other
  value is refused). Same debug-only test-hook pattern as
  `ANANSI_PROVIDER_TEST_CREDENTIAL` / `ANANSI_PROVIDER_TEST_CANCEL`.
- `SigninAttempt` — attempt id, `state`, `nonce`, PKCE
  `verifier` + `challenge` (S256, base64url SHA-256, no
  padding), `redirect_uri`, `client_id` used (dynamic entrypoint
  or issued ID), `ext_agent_host_id`, created-at, status
  (`pending` / `callback_received` / `succeeded` /
  `failed` + non-secret reason), single-use flag. Attempts live
  in `BackendState.chatgpt_attempts` (per workspace), swept
  with the existing `sweep()` (TTL 10 minutes; expiry cancels
  nothing network-side — the listener thread simply stops
  accepting and the attempt is dropped).
- `ChatGptCredentialRecord` — the per-registration record:
  `email`, `subject` (validated `sub`), `issuer`,
  `client_id`, `ext_agent_host_id`, `id_token`,
  `access_token`, `refresh_token`, `token_type: Bearer`,
  `expires_in`, `scopes` (from the returned space-separated
  `scope`), `saved_at` (UTC ms). Serialized as JSON only into
  the keyring — never to a file, log, or response body.
- `validate_id_token(...)` — structural JWT decode (base64url,
  no padding), header `alg` must be `RS256`, `kid` must match
  a JWKS key; claims: `iss == https://auth.openai.com` (or the
  test issuer), `aud == issued client_id`, `exp` in the
  future (30 s clock skew allowed), `nonce == saved nonce`
  (initial sign-in only; refresh responses validate
  iss/aud/exp/sub-match, no nonce), `sub` present and, for a
  returning account, equal to the selected account's `sub`.
  Signature verified with `ring`
  (`RsaParameters::RSA_PKCS1_2048_8192_SHA256` +
  `RsaPublicKeyComponents { n, e }` from the JWKS) over
  `header.payload`. A token that fails any check is refused
  with a non-secret reason; the token bytes are never logged.
- `exchange_code(...)` / `refresh_tokens(...)` — form-encoded
  POSTs through the existing bounded provider transport
  (`anansi_providers::http`: `StdHttpTransport` for loopback,
  `RustlsTransport` for HTTPS), `ValidatedEndpoint` built from
  the issuer base (the endpoint policy already allows https,
  or http on loopback), bodies capped, no redirects followed.
  Token responses are parsed strictly (`access_token`,
  `token_type == Bearer`, `expires_in` 60–86400, optional
  `refresh_token`, optional `id_token`, `scope`).
- `build_authorization_url(...)` — URL-encodes every parameter
  exactly as §1.2 step 2 specifies. The URL is returned to
  the host caller; the host opens the system browser. The URL
  is never logged (it may carry `id_token_hint` on
  reauthorization; the status path never includes it).

### 3.2 Storage boundaries (reuse, don't reinvent)

- **Tokens:** OS keyring via `KeyringSecretResolver::with_service`
  (`com.robillionair.anansi.chatgpt` — a distinct service from
  the provider keys, so the two stores cannot collide and the
  minted-ledger/orphan logic for API keys never sees ChatGPT
  material). Reference = `chatgpt-<issued-client-id>` (a label,
  not a secret). One keyring entry per registration; a refresh
  replaces the whole JSON record in one `set_secret` (atomic
  replacement by construction).
- **Host id:** `ext_agent_host_id` (`urn:uuid:<uuid4>` from
  `getrandom`) persisted once per storage root at
  `<root>/provider-config/chatgpt-host.json`, atomically via the
  existing `persist_text_file_atomic` helper, created lazily on
  first sign-in start. Labels only.
- **Registrations (labels only):** `<root>/provider-config/
  <safe-workspace>.chatgpt.json` — `{client_id: {email,
  subject, plan_usage, last_verified_ms}}`, written atomically.
  This is what lets a later launch offer "choose a saved
  ChatGPT account". It holds no tokens.
- **Attempts:** in-memory only (`BackendState`), never
  persisted — a restart drops pending attempts, which is safe
  (the user simply signs in again).
- **Never:** tokens or callback codes in `providers-status`,
  config files, backups, exports, logs, or any browser-visible
  response. The `redact` module's rules apply to any new log
  line in this code (no token-shaped values; authorization
  URLs are redacted when they carry a hint).

### 3.3 Operations (in `provider_dispatch.rs`, routed via `ai_discovery.rs` + the API allowlist)

| Op | Auth | Behaviour |
|---|---|---|
| `chatgpt-signin-start` | owner-only | Creates an attempt (fresh state/nonce/PKCE), binds the loopback listener on `127.0.0.1:0`, returns `{attemptId, authorizationUrl, redirectUri, expiresInSeconds}`. The host opens the URL in the system browser. No network call. |
| `chatgpt-signin-status` | browser-readable (labels only) | `{signedIn, email?, planUsage, planUsageReason?, expiresAtMs?, accounts: [{clientId, email, planUsage}], pendingAttempt?}` — never tokens, never the authorization URL. |
| `chatgpt-signin-refresh` | owner-only | Refreshes the active record when expired or within 5 minutes of expiry (or on demand); serialized per workspace; rotating refresh token replaced together; failed refresh with `invalid_grant` clears the record (needs-sign-in); transient failure keeps the record and surfaces the error. |
| `chatgpt-signin-disconnect` | owner-only | Attempts revocation at the issuer's `revocation_endpoint`; clears local tokens regardless; reports `remoteRevocationConfirmed: bool`; retains the registration stub (client ID + email) for re-sign-in. |
| (no `complete` op) | — | The loopback listener thread receives the callback, validates `state` (constant-time) and single-use, stores the callback in the attempt, and runs the exchange on a host worker thread, updating the attempt to `succeeded` / `failed`. The UI polls `chatgpt-signin-status`. The callback `code` never crosses into browser state. |

The listener: `std::net::TcpListener` on `127.0.0.1:0`,
single-shot accept loop with a bounded wait (attempt TTL),
parses only `GET /auth/callback?...`, rejects any other path
or method, responds with a minimal HTML page (success: "You
can close this tab and return to ANANSI"; failure: the
non-secret reason), then closes. It binds the exact port that
goes into `redirect_uri` — the port in the URL is the bound
port, never a guessed one.

### 3.4 Contracts + UI

- New DTOs in `provider_dispatch.rs` (`JsonSchema`), regenerated
  via `node scripts/contracts.mjs`; the API allowlist gains the
  four `/v1/ai/chatgpt-signin-*` routes; `ai_discovery.rs` op
  list gains the four names.
- UI: one prototype section in `Providers.tsx`, rendered **only**
  when `window.__ANANSI_CHATGPT_SIGNIN_DEV__ === true` (set by
  no production code path; the dev harness/test sets it). The
  section shows sign-in status labels and a *disabled* note
  that live sign-in is not yet available — **no enabled
  "Continue with ChatGPT" button ships in this slice**, in any
  build. A component test proves the section is absent by
  default and present (disabled) with the flag.

## 4. Acceptance criteria

- AC-S1 (routes verified): the three official pages (plus the
  two linked siblings) are read and their load-bearing claims
  spot-checked; the design records what is buildable now, what
  needs registration/approval, and which account/plan claims
  need real verification (§1.3, §5). Design + AC committed
  before implementation.
- AC-S2 (start): `chatgpt-signin-start` returns a well-formed
  authorization URL — first time `client_id=
  dynamic_agent_client` + `agent_name_hint=ANANSI` +
  persisted `ext_agent_host_id`; returning account uses the
  saved issued `client_id` without `agent_name_hint`;
  `redirect_uri` is `http://127.0.0.1:<bound-port>/auth/callback`
  with the listener already bound; PKCE S256 challenge matches
  the verifier; fresh `state` + `nonce` per attempt; scopes
  are the six documented scopes; `resource=
  https://api.openai.com/v1`.
- AC-S3 (callback validation): wrong `state` → attempt failed,
  no exchange, nothing stored; replayed callback (same code +
  state delivered twice) → rejected; `error=access_denied`
  with valid state → attempt failed, no exchange; callback
  with a different `client_id` than the pending attempt's →
  rejected; new-registration callback without an issued ID →
  incomplete, nothing stored.
- AC-S4 (token + scope validation): code exchange against the
  synthetic token endpoint succeeds and the ID token is
  validated (issuer, audience = issued client ID, expiry,
  nonce, `sub`, RS256 signature vs the synthetic JWKS); wrong
  nonce → refused; expired ID token → refused; tampered
  signature → refused; missing `chatgpt.tokens.use.direct`
  in granted scopes → identity-only sign-in retained, plan
  usage disabled, no inference path taken.
- AC-S5 (storage): tokens exist only in the OS keyring under
  the ChatGPT service; no token or callback code appears in
  any file, log, status response, export, or backup; status
  returns labels only; the keyring entry is removed on
  disconnect.
- AC-S6 (expiry + refresh): an expired access token is
  refreshed via the `refresh_token` grant; the rotating
  refresh token is replaced together with the access token;
  concurrent refreshes are serialized (one exchange, one
  stored record); `invalid_grant` on refresh clears the
  record and status reports needs-sign-in; a transient
  network failure keeps the record and surfaces an error.
- AC-S7 (disconnect): revocation is attempted at the issuer's
  `revocation_endpoint`; local tokens are cleared; when
  revocation cannot be confirmed the status reports it and
  the user is directed to ChatGPT Settings; the registration
  stub (client ID + email) survives for re-sign-in.
- AC-S8 (browser boundaries): a paired browser session cannot
  start, refresh, or disconnect (each refused `Forbidden` and
  recorded in the approval queue); `chatgpt-signin-status`
  from a browser returns labels only and no tokens.
- AC-S9 (regression): after a failed sign-in (wrong state and
  denied consent), a local Ollama dispatch still succeeds and
  an API-key provider can still be configured and dispatched;
  the full provider-dispatch suite stays green.
- AC-S10 (dev gate): the prototype UI section is absent by
  default (no "Continue with ChatGPT" button in any build);
  with the dev flag it renders status labels with sign-in
  disabled and a note that live sign-in is not yet available.
- AC-S11 (decision): a short decision memo (§5) + the Codex
  app-server note (§6) are written; the smallest path to one
  authorized synthetic-data Responses turn after eligibility
  is confirmed is documented; the exact owner action for
  eligibility is stated.
- AC-S12 (delivery): checksummed Git bundle ZIP with exact
  SHAs, a Windows rerun checklist, and a handoff; no live
  OAuth, real credentials, paid calls, generated-code
  execution, pushes, or preview promotion.

## 5. Decision memo (the next step, not built here)

**Verdict: feasible as specified; not yet eligible to ship.**

The OSS dynamic-registration flow is fully specified and
buildable without prior registration, and this spike proves
the host-side mechanics against a synthetic issuer. What is
NOT established on this VM — and cannot be established by
reading docs — is whether ANANSI counts as an eligible
open-source partner, whether a real Plus/Pro account receives
the plan scopes, which models/routes that account can call,
token lifetimes, and preview limitations.

**Exact owner action needed:** on a Windows machine with a
ChatGPT Plus or Pro account, attempt one real dynamic
registration through the (still gated) prototype flow, or
obtain written confirmation from OpenAI that ANANSI qualifies
as an open-source partner. Do not enable the sign-in button
before that attempt succeeds and the plan scopes are granted.

**Smallest path to one authorized, synthetic-data Responses
turn after eligibility is confirmed:**
1. Eligibility attempt succeeds (owner action above); the
   issued `client_id` + plan scopes are on record.
2. Add a `chatgpt-plan` provider entry to the existing
   dispatch path: `Authorization: Bearer <OAuth access
   token>`, `resource=https://api.openai.com/v1`,
   `POST /v1/responses`, scope-gated (refuse when
   `chatgpt.tokens.use.direct` is absent), plan-usage errors
   surfaced verbatim (429 `subscription_sharing_usage_limit_
   exceeded` → pause + link to ChatGPT Settings → Usage; 403
   `subscription_sharing_user_not_eligible` → stop, do not
   loop through OAuth; never fall back to API-key billing).
   Cost class for plan calls is a decision for that slice:
   plan usage consumes the user's shared five-hour Plus
   limit, so it is not "free" — treat as `unknown` (blocked
   by Zero-Cost Lock without a permit) until that slice
   decides otherwise.
3. Synthetic-data Responses turn in tests: the synthetic
   issuer stands in for `api.openai.com`; one turn with
   synthetic user text, asserting the bearer token, the
   resource, and the plan-usage error mapping.
4. Windows manual verification: real browser handoff, the
   token record in Windows Credential Manager under the
   ChatGPT service, one real Responses turn on synthetic
   data, and the disconnect + re-sign-in path.

## 6. Codex app-server integration (documented separately, not built)

Per the app-server page: after exchanging the code, set
`ACCESS_TOKEN` in the app-server child process's environment
to the OAuth access token and launch
`codex app-server --listen stdio://` with
`model_provider="openai_chatgpt_plan"`,
`base_url="https://api.openai.com/v1"`,
`env_key="ACCESS_TOKEN"`, `wire_api="responses"`,
`requires_openai_auth=false`, `supports_websockets=false`.
Drive it over newline-delimited JSON on stdin/stdout:
`initialize` (`clientInfo.name` stable across installations,
matching `agent_name_hint`), `initialized`,
`thread/start`, `turn/start`, display `item/agentMessage/delta`
events; only `turn.status == completed` is success. On token
renewal, restart app-server with the updated `ACCESS_TOKEN`
and resume with `thread/resume` + the saved thread ID.

**ANANSI's execution boundary (the decision this spike
records, not a docs claim):** the app-server page does not
describe Codex's tool, shell, or file-execution behaviour.
ANANSI's standing rule is no generated-code execution and no
automatic tool execution. Therefore a Codex integration is a
separate slice requiring its own authorization design: what
Codex may execute, on whose machine, under which sandbox,
with which owner approval per action. This spike builds no
Codex process, ships no Codex binary, and obtains no Codex
binary. How the app obtains a Codex binary (download,
version pinning, verification) is unverified — the page
assumes `codex` is installed.

## 7. Out of scope (unchanged)

- Any live OAuth, real credentials, paid calls, Responses
  turns, or model/route eligibility claims.
- Any change to the API-key flow, the frozen Windows pilot,
  or any provider op/contract except the four new
  `chatgpt-signin-*` ops.
- `providers.spec.ts` 240 s timeout (pre-existing, removed
  'Workspace' button) and the other pre-existing failures.
- Token lifetimes / Preview limitations / Models and
  inference / UI/UX Guidelines pages (unread; listed in
  §1.3 as needing reading before a live slice).
