# M5 AI Matrix — handover to the integration owner

Agent: Muse (M5 implementation agent) · Date: 2026-10-20
Branch: `muse/m5-provider-integration` · **Baseline: `5b7a87b`** (published on
`origin/codex/m2-m3-integration`; branch tip `fc2540f` is one docs-only commit
ahead and changes no code). Nothing here is merged to `master`, and nothing
was pushed. M5 is **not** claimed complete: this package is the
provider-integration layer plus its evidence. Application workflows and the
readiness gates listed at the end remain open.

Requirement map and acceptance criteria:
[M5-requirements-and-acceptance.md](M5-requirements-and-acceptance.md).
Provider documentation verification:
[M5-provider-verification.md](M5-provider-verification.md).

## What was built

A new crate, **`crates/providers` (`anansi-providers`)**, dependency-light by
design (serde/serde_json only, hand-rolled bounded HTTP over `std::net`,
socket-shutdown cancellation — the exact posture of `anansi-core`):

| Module | Contents |
|---|---|
| `types` | Segment-level privacy classes (Public/Standard/Private/Sealed/Restricted), completion + stream types, `ModelInfo` with caller-supplied prices only |
| `secrets` | `CredentialRef` (references only), `SecretResolver` trait, zero-on-drop `SecretValue` with redacted `Debug`, PKCE S256 + OAuth state validation (RFC 7636 vector tested) |
| `config` | Endpoint policy (HTTPS/loopback rules, no URL credentials, no query secrets, insecure-custom opt-in that can never carry a credential), bounded timeouts, bounded retry policy |
| `http` | `Transport` trait + std HTTP/1.1 transport: chunked decoding, 64 KiB header / 4 MiB body caps, no redirect following, connect/first-byte/idle/total deadlines, cancellation shuts the socket down |
| `stream` | Incremental SSE and NDJSON parsers, safe at arbitrary byte boundaries |
| `providers/` | `OpenAiCompatibleAdapter`, `AnthropicAdapter`, `GeminiAdapter`, `OllamaAdapter`, `AzureOpenAiAdapter`; shared sanitized error mapping (401/403/429/5xx, Retry-After honoured) |
| `catalog` | All 20 MASTER_SPEC §14 providers + Ollama/LM Studio/custom: OpenAI-compatible family served by one adapter; **AWS Bedrock and Vertex AI `Deferred`** (cloud IAM signing — not faked); Cloudflare `CatalogOnly` (account-scoped endpoint); no provider claims user OAuth for inference |
| `gates` | `PrivacyGate` (Sealed refused everywhere; Private needs explicit cloud authorization; unredacted Restricted refused for cloud), `ZeroCostLock`, single-use `PaidInferencePermit`, `Budget`/`QuotaLedger`, `QuotaReport::Unavailable` as a first-class answer |
| `routing` | The eight §17 modes, privacy/cost/capability-filtered fallback chains, user preference ordering |
| `dispatcher` | The single dispatch path: per-candidate gate order privacy → cost → budget → credential resolution → bounded retry → availability-only fallback; streaming with pre-first-byte fallback only; `DisclosureRecord` ("View What ANANSI Sent" at adapter level) on every dispatch |

## Verification actually performed (all local, 2026-10-20, Rust 1.99.1)

- `cargo test` in the crate: **40 unit + 26 synthetic-server integration
  tests, all passing**. The integration suite (`tests/synthetic_scenarios.rs`
  + `tests/common/`) runs scripted loopback servers and covers: success and
  malformed responses for all five adapter families; streaming over
  3-byte-dribbled chunks; mid-stream cancellation reported as `Cancelled`;
  401 without retry; 429 retry-then-success and bounded exhaustion;
  Retry-After honoured; timeout; connection drop and refusal; redirect
  refused; server echoing the fake `Authorization` value scrubbed from
  errors; **cost/permission/privacy/budget refusals with synthetic-server
  hit count exactly 0**; consumed-permit reuse refused; fallback after 503;
  free→paid fallback blocked under Zero-Cost Lock; streaming fallback
  before first byte.
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`:
  clean.
- Root workspace after the integration commit:
  `cargo test -p anansi-providers --locked` passes. Baseline
  regression (`cargo test -p anansi-core -p anansi-api
  -p anansi-contracts --locked`): **111 tests passed**; the only
  failures are the 4 `semantic_workflow` tests that require a live
  Ollama with `all-minilm:22m` on `127.0.0.1:11434` — this VM has no
  Ollama (connection refused), and those tests are untouched by and
  independent of this package. Docs validation passes (75 Markdown
  files, local links checked).
- **Synthetic tests prove adapter behaviour, not real-provider
  certification.** No real provider was called: no credentials exist on
  this machine, no paid inference was authorized, and HTTPS dispatch
  fails closed by design (below). Real-provider verification is
  **pending** for every cloud provider.

## Commits (in order, on `muse/m5-provider-integration`)

1. `cc9520d` feat(m5): provider crate core contracts, endpoint policy and bounded transport
2. `bbeec0a` feat(m5): provider adapters and catalog for the full spec provider set
3. `527fae8` feat(m5): privacy/cost gates, routing and dispatcher
4. `bdf6008` test(m5): synthetic-server integration suite for provider dispatch
5. `4dad873` integrate(m5): register anansi-providers in the root workspace — **the only commit touching shared files** (`Cargo.toml` members, `Cargo.lock` +8 lines, crate manifest). Cherry-pick or drop independently.
6. docs commit(s): this handover, the requirement map, and the provider verification record.

Reproduce locally:

```bash
git checkout muse/m5-provider-integration
node scripts/cargo.mjs test -p anansi-providers --locked
node scripts/cargo.mjs clippy -p anansi-providers --all-targets --locked -- -D warnings
node scripts/cargo.mjs fmt --all -- --check
node scripts/validate-docs.mjs
```

## Integration order for the owner

1. **TLS transport — DONE after this handover was written.** See
   [M5-transport-addendum.md](M5-transport-addendum.md): production
   HTTPS (`RustlsTransport`, rustls + Mozilla roots, validation
   always on, no insecure mode) and billable-safe retry semantics
   landed on this branch with synthetic-HTTPS evidence. The
   integrator's remaining step is to dispatch with `RustlsTransport`;
   OS-native root stores, if ever wanted, are an owner decision
   noted in the addendum.
2. **Secret resolver.** Implement `SecretResolver` over the existing
   keyring boundary (`crates/core/src/vault/credential_store.rs`, service
   naming for provider credentials is an owner decision). Until then,
   credentialed dispatch refuses before network via
   `UnavailableResolver`. Provider setup UI follows §15: official
   API-key page, user pastes key into the OS vault, only the
   `CredentialRef` is stored in settings.
3. **Dispatch surface.** Add `Core::ai_command` operations (e.g.
   `providers-list`, `providers-dispatch`, `providers-stream-*`)
   mirroring the existing ask operations allow-list in
   `crates/api/src/lib.rs`, with generated contracts in
   `packages/contracts`. Carry the request's privacy classification
   from the Context Compiler output — the dispatcher consumes
   segment classes, it does not classify.
4. **UI.** The Navigation placeholders ("AI Matrix / Integrations")
   already exist. Render `DisclosureRecord` for "View What ANANSI
   Sent"; surface `QuotaReport::Unavailable` honestly instead of
   inventing allowances (§16).
5. **OAuth completion (only where officially offered).** PKCE/state
   contracts are in `secrets.rs`. A live flow additionally needs the
   system-browser hand-off, a loopback redirect listener, and the TLS
   token exchange. At verification time, none of the implemented API
   surfaces offered user OAuth for inference — do not relabel API keys
   as OAuth (§14/§15; verification record).
6. **Bedrock / Vertex**, if wanted: separate verified designs for AWS
   SigV4 and Google ADC; they are catalogued `Deferred`, with reasons.

## Shared-file changes (complete list)

- Commit `4dad873` only: root `Cargo.toml` (one member added;
  `default-members` unchanged), root `Cargo.lock` (+8 lines),
  `crates/providers/Cargo.toml` (standalone `[workspace]` marker
  removed). No existing crate depends on `anansi-providers` yet; no
  API route, contract, UI, settings schema, or migration was touched.
- Everything else is new files under `crates/providers/` and
  `docs/handoffs/`.

## Limitations stated plainly

- Cloud dispatch cannot complete until step 1 (TLS) lands; loopback
  local dispatch (Ollama/LM Studio) works end-to-end today and is
  covered by synthetic tests shaped exactly like those runtimes.
- Timeout/retry numbers are package policy (M5-D6), not spec values;
  ADR-0029's 30 s local-Ask figure and `ask.rs`'s current 180 s already
  diverge — reconcile when wiring Ask onto this dispatcher.
- "Restricted content" had no repo definition; M5-D4 records the
  operational definition used (caller-classified, must arrive
  redacted). Owner override welcome.
- The crate does not persist provider configuration or usage; storage
  belongs to the app's settings/ledger layers at integration.
- Known remaining M5 gates outside this package: manual Win32
  credential-dialog acceptance, `G1-External-Audit`, remote
  CI/protected publication (deferred by owner policy). None is claimed
  passed.

## Requirement-to-evidence checklist

| Requirement (map ID) | Evidence |
|---|---|
| M5-R1 provider abstraction | One dispatcher + trait drives all adapters; adapter unit tests + 26 scenario tests |
| M5-R2 provider set | `catalog::tests::every_spec_named_provider_is_present` (all §14 names) |
| M5-R3 auth verification | `M5-provider-verification.md` (official links, dated 2026-10-20); catalog auth fields; `no_provider_claims_oauth_without_basis` test |
| M5-R4 OAuth contract | PKCE RFC 7636 vector test, state-validation tests, authorization-URL test in `secrets::tests` |
| M5-R5 credential references | `CredentialRef`/`SecretResolver`; `UnavailableResolver` fail-closed; redaction tests |
| M5-R6 prohibited mechanisms | No such code paths; endpoint-policy tests in `config::tests` |
| M5-R7 Zero-Cost Lock | `gates` tests + scenarios: lock blocks paid (0 hits), free→paid fallback blocked, billable retry only under permit |
| M5-R8 honest quota reporting | `QuotaReport::Unavailable`; usage recorded only from provider responses (`openai_success_records_usage_and_disclosure`) |
| M5-R9 routing modes | `routing::tests` (Private/local-only/lock/ChooseModel) + fallback scenarios |
| M5-R10 budgets + estimates | Budget gate scenario (0 hits); estimate + disclosure in every outcome |
| M5-R11 privacy pipeline | Sealed/Private scenarios (0 hits); `DisclosureRecord` asserted in success scenario |
| M5-R12 endpoint security | Redirect scenario; HTTPS fail-closed scenario; config policy tests |
| M5-R13 secret storage boundary | References-only design; no secret literals in the crate (only obviously fake test keys against loopback) |
| M5-R14 streaming/cancellation | Dribbled-chunk SSE/NDJSON scenarios; cancellation scenario; parser byte-boundary unit tests |
| M5-R15 sanitized errors/retries | 401/429/5xx scenarios incl. server-echo scrubbing |
| M5-R16 custom/enterprise | Azure scenario (deployment path, `api-key` header); Bedrock/Vertex `Deferred` catalog test |
| M5-R17 no unauthorized spend | Permit scenarios: no permit → refused (0 hits); permit is single-use |
