# M5 AI Matrix — requirement map, design decisions and acceptance criteria

Agent: Muse (M5 implementation agent) · Date: 2026-10-20
Branch: `muse/m5-provider-integration`
**Baseline: `5b7a87b3433e1023daa9f1aa9caedf4d9181f8a0`** (published on
`origin/codex/m2-m3-integration`; one docs-only commit `fc2540f` follows it on
that branch and changes no code or contracts). Published `master` is
`8c9406b`; the baseline is 122 files / +22,821 lines ahead of it (M2–M4
integration: vault, Ask streaming, semantic retrieval). M5 is **not** built on
master, because the interfaces it must fit (Ask, discovery, Context Compiler,
vault credential store) exist only on the integration checkpoint.

Status of the milestone before this package: `plans/04-ai-matrix.md` is an
11-line planning stub ("detailed implementation design and acceptance approval
required before coding"); `docs/evidence/M5-READINESS.md` records "M5 readiness
is currently **not established**" and names the first dependency-safe M5 task
as "detailed provider/authentication/Zero-Cost-Lock contract inventory and
design". This document is that inventory plus the acceptance contract for the
implementation package in `crates/providers`. It does **not** claim M5
complete: application workflows (AI Matrix UI, `Core::ai_command` dispatch,
contract generation) are integrator work, and the readiness gates (manual
Win32 credential check, G1-External-Audit, remote CI/publication) are
untouched by this package.

## 1. Requirement traceability

Sources: MASTER_SPEC §§14–17, 19, 67, 70–71, 74, 82; `plans/04-ai-matrix.md`;
REQUIREMENTS_MATRIX rows AN-007 / AN-008 / AN-026; DECISIONS ADR-0002, 0005,
0022/0023/0025, 0027–0030; AGENTS.md execution policy.

| ID | Requirement (source) | Package slice | Acceptance evidence in this package |
|---|---|---|---|
| M5-R1 | Provider-independent abstraction, no tightly coupled provider code (§§3, 6, 14) | `ProviderAdapter` trait; all provider specifics behind adapters/catalog | One dispatcher drives every adapter in tests through the same trait object |
| M5-R2 | Support local models, OAuth-connected APIs, free tiers, user API keys, enterprise endpoints, custom OpenAI-compatible endpoints (§14) | Catalog with 20 spec-named providers + local runtimes, each classified by adapter kind and auth | Catalog unit tests assert every §14 name is present with an explicit status — none silently missing, none fabricated as "live" |
| M5-R3 | Verify authentication against current provider documentation; check OAuth eligibility at implementation time; never use retired/undocumented consumer auth (§14, §15) | `docs/handoffs/M5-provider-verification.md` (official links, verified 2026-10-20) + catalog `auth` fields | Verification record per implemented adapter; unsupported auth methods are represented as unsupported, never emulated |
| M5-R4 | OAuth-first where officially supported: system browser, Authorization Code + PKCE S256, state validation, issuer/audience and scope checks, secure storage, refresh/revocation (§15) | `secrets`/OAuth contract types: PKCE verifier/challenge, authorization-URL builder, state validation | Unit tests: PKCE S256 challenge matches RFC 7636 Appendix B vector; state mismatch is refused. Token exchange/refresh needs the TLS transport + browser flow → integrator (see §4) |
| M5-R5 | Where OAuth is unavailable: official API-key page, user pastes key, store in OS credential vault, test with no personal data (§15, AN-008) | `CredentialRef` + `SecretResolver` trait aligned to the existing keyring boundary (`crates/core/src/vault/credential_store.rs`); config stores references only | Tests prove configs/D Debug output/logs never contain a secret value; resolver is only consulted at dispatch time |
| M5-R6 | Never extract cookies, capture passwords, scrape sessions, use undocumented endpoints, or circumvent quotas (§15) | No such code paths exist; endpoint policy rejects non-official/custom hosts unless explicitly configured as custom endpoints | Endpoint-policy tests; design statement in handoff |
| M5-R7 | Zero-Cost Lock: default ON in Max Free mode; block paid models, unknown-cost models, free-to-paid fallback, automatic credit purchases, unapproved billable retries (§16, AN-026) | `gates::ZeroCostLock` consulted by router, fallback and retry policy | Tests: paid and unknown-cost candidates are denied; fallback chain stops rather than crossing to paid; paid retry without permit is refused — all with synthetic-server hit count = 0 for denied traffic |
| M5-R8 | Show real provider usage where available; where quota info is unavailable, say so; never invent free-token allowances (§16) | `Usage` parsed from provider responses; `QuotaReport::Unavailable(reason)` is a first-class result | Tests assert unavailable is reported as unavailable for providers without a verified quota API; no allowance constants exist in the crate |
| M5-R9 | Routing modes Auto, Auto Free, Private, Fast, Deep, Coding, Vision, Choose Model; router evaluates privacy, cost, availability, capability, context window, preferences; fallback chains; never silently transmit local-only data to cloud (§17) | `routing::Router` with the eight modes and ordered fallback chains | Router unit tests per mode; Private mode yields loopback-local candidates only; a local-only request never produces a cloud candidate |
| M5-R10 | Per-project budgets (Free only / Low cost / Balanced / Maximum quality / Custom); estimates displayed before expensive operations; disclose local vs cloud vs multi-model vs paid (§67) | `BudgetProfile` + `QuotaLedger` (request/token ceilings supplied by the caller — no invented money values) + pre-dispatch `CostEstimate` in the disclosure record | Gate tests: over-budget dispatch refused before any network request; estimate present in every disclosure record |
| M5-R11 | Pre-transmission pipeline: authorized context, workspace permissions, remove Sealed from cloud requests, redact restricted content, preserve citations, fit budget, apply cost policy, select approved model, record disclosure manifest (§19) | `gates::PrivacyGate` + `dispatcher` ordering + `DisclosureRecord` ("View What ANANSI Sent" at adapter level) | Tests: Sealed content refused for **every** provider kind (incl. local) before network; unredacted restricted content refused for cloud; disclosure record lists provider, model, redactions, estimated tokens, routing reason |
| M5-R12 | Privacy classes enforced in native code (§74); deterministic security outside the LLM; SSRF / DNS-rebinding / credential-leakage / excessive-spending protections (§71) | Endpoint policy: HTTPS required for non-loopback, no URL credentials, no userinfo, redirect refusal, loopback detection incl. `localhost`/`[::1]`; dispatcher-side enforcement (not model-side) | Endpoint validation + redirect tests against synthetic servers |
| M5-R13 | Credential storage via OS vault (Windows Credential Manager / macOS Keychain) (§70); secrets never in backups or Git (§§4, 76) | References-only design (M5-R5); no secret persistence in this crate at all | `git grep` over the package shows no credential literals; tests use obviously fake credentials against loopback only |
| M5-R14 | Streaming, cancellation, bounded timeouts (Ask substrate: ADR-0028/0029; §66 job limits; assignment scope) | SSE/NDJSON incremental parsers, `CancellationToken` with socket shutdown (mirrors `ask_stream.rs`), bounded connect/first-byte/idle/total timeouts | Synthetic-server tests: byte-boundary streaming, mid-stream cancellation observed as socket closure, timeout and connection-loss mapping |
| M5-R15 | Sanitized errors; secret-free logs; auth failure / rate-limit / retry behaviour (§78 evaluation areas; assignment scope) | `ProviderError` taxonomy + `redact` module; bounded retry (429/5xx, Retry-After honoured, capped) | Tests: 401/403 → auth error without retry; 429 → bounded retry then success / exhaustion; server echoing the fake Authorization value is scrubbed from errors |
| M5-R16 | Custom/enterprise endpoint policy (§14; AN-021 enterprise is `deferred`) | Custom OpenAI-compatible endpoints are first-class config with the same policy checks; Azure adapter via deployment URL + `api-key`; AWS Bedrock / Vertex AI recorded as IAM-signing-deferred in the catalog, not faked | Custom-endpoint tests against synthetic servers; catalog marks Bedrock/Vertex `Deferred` with the reason |
| M5-R17 | No real paid inference, downloads, or spending without explicit owner authorization (assignment; AGENTS.md) | `PaidInferencePermit` required before any paid dispatch; this package's tests never leave loopback | Cost/permission refusal tests assert zero network requests without a permit |

## 2. Design decisions recorded by this package

- **M5-D1 Standalone crate.** New code lives in `crates/providers`
  (`anansi-providers`) with its own `[workspace]` marker so it builds and
  tests without touching shared files. Registering it in the root workspace
  is a separate, final integration commit (see handoff).
- **M5-D2 Dependency posture mirrors `anansi-core`.** serde/serde_json only;
  hand-rolled bounded HTTP/1.1 over `std::net` (chunked decoding, size caps),
  no HTTP-client crate, no redirect following — the same pattern as
  `ask.rs` / `vector.rs`.
- **M5-D3 TLS is an explicit integration dependency.** `std` has no TLS, and
  this package does not smuggle in an unvetted TLS stack. The
  `http::Transport` trait is the seam: `StdHttpTransport` serves loopback and
  synthetic endpoints; non-loopback provider URLs must be `https://` at
  config time and dispatch over the std transport **fails closed** with
  `TransportError::TlsUnavailable` until the integrator supplies a vetted TLS
  transport behind the same trait. Credentials are never sent over plaintext
  non-loopback HTTP. This matches the baseline, where all implemented
  inference is loopback-only.
- **M5-D4 "Restricted content" definition.** MASTER_SPEC §19 uses the term
  without defining it. This package defines it operationally: a content
  segment the caller has classified `Restricted` (caller = Context Compiler /
  workspace policy). Restricted segments must arrive redacted
  (`redacted: true` with replacement text) or dispatch is refused; the
  dispatcher never guesses or auto-redacts. Recorded here so the owner can
  override with a repo-level definition.
- **M5-D5 Unknown cost = paid.** A model with no verified price entry is
  treated as paid: blocked under Zero-Cost Lock, and a paid permit must
  explicitly acknowledge unknown cost. No prices are hardcoded from memory;
  the catalog ships **no** price table — prices, when supplied, come from
  caller configuration recorded with its source/date.
- **M5-D6 Timeout/retry defaults are package policy, not spec values** (the
  spec sets none; ADR-0029's 30 s applies to the local Ask path and current
  `ask.rs` code already diverges to 180 s — flagged for the integrator).
  Package defaults: connect 5 s, first-byte 30 s, idle 30 s, total 180 s,
  all configurable and clamped (total ≤ 600 s). Retries: max 2, only for
  429/502/503/504 or pre-first-byte connection failure, Retry-After honoured
  up to a 30 s cap, exponential backoff from 250 ms.
- **M5-D7 "Max Free mode"** (§16, undefined elsewhere in the spec) is
  represented as `BudgetProfile::FreeOnly` with Zero-Cost Lock engaged.
- **M5-D8 OAuth scope.** PKCE/state/URL contracts and tests are implemented
  (M5-R4). Completing a live OAuth flow additionally needs the system-browser
  hand-off, loopback redirect listener and TLS token exchange — application
  integration, deferred to the integrator with the exact dependency noted in
  the handoff. Providers whose official docs offer no user OAuth for
  inference are catalogued as API-key only; this package does not fabricate
  OAuth for them (see verification record).

## 3. Acceptance criteria for this package (Definition of Done)

1. `cargo test` (in `crates/providers`), `cargo clippy --all-targets -- -D warnings`,
   and `cargo fmt --check` all pass locally on Rust 1.99.1.
2. Every scenario in the assignment's validation list has at least one
   executable synthetic-server test (see checklist in the handoff).
3. No test performs non-loopback network access; no real credential,
   paid inference, model download or CI run is used.
4. Commits are small and ordered: docs → core contracts → adapters →
   routing/gates → tests → separate integration commit. Nothing is merged
   to `master` or pushed.
5. The handoff (`docs/handoffs/M5-to-integrator.md`) states plainly what is
   locally tested, what is synthetic-only, and what remains pending
   real-provider verification.

## 4. Explicitly deferred (with the dependency that unblocks each)

- Live OAuth token exchange/refresh/revocation — needs TLS transport (M5-D3)
  + browser/loopback-redirect application flow.
- AWS Bedrock (SigV4) and Vertex AI (Google ADC/OAuth2 service account)
  adapters — need cloud IAM signing, which is a separate verified design;
  catalogued `Deferred`, not stubbed as working.
- AI Matrix UI, `Core::ai_command` operations, generated contracts —
  shared-file application integration (integrator; see handoff order).
- Real-provider certification — needs owner-supplied credentials and
  explicit paid-inference authorization; recorded as pending, never claimed.
