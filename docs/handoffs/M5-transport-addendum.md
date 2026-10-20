# M5 addendum — production HTTPS transport + billable-safe retries

Agent: Muse (M5 implementation agent) · Date: 2026-10-20
Branch: `muse/m5-provider-integration`, continuing from `7e18f68`.
This addendum supersedes **step 1 (TLS transport)** of
[M5-to-integrator.md](M5-to-integrator.md) and the fail-closed half of
decision M5-D3 in
[M5-requirements-and-acceptance.md](M5-requirements-and-acceptance.md).
Everything else in those documents stands.

## What changed

Three commits on top of `7e18f68`:

1. `e45653c` feat(m5): production HTTPS transport via rustls behind the Transport trait
2. `47a5c1e` feat(m5): billable-safe retry and fallback semantics
3. `b6e27bd` test(m5): synthetic HTTPS suite and billable-retry safety scenarios

Plus a separate root `Cargo.lock` commit (dependency/lockfile only —
see "Dependencies" below).

### The transport

`crates/providers/src/http.rs` now has two `Transport`
implementations sharing one bounded HTTP/1.1 engine (header/body
caps, chunked decoding, connect/first-byte/idle/total deadlines,
cancellation via raw-socket shutdown, **no redirect following**):

- `StdHttpTransport` — unchanged plaintext transport; HTTPS still
  fails closed with `TlsUnavailable`. Suitable for loopback-only
  wiring.
- `RustlsTransport` — **the production transport.** Plaintext where
  endpoint policy allows it; HTTPS via rustls with validation below.

`crates/providers/src/tls.rs` (`TlsConfig`) is the whole TLS
configuration surface:

- rustls **0.23.46**, ring crypto provider, Mozilla root set via
  `webpki-roots` **1.0.10**.
- Certificate-chain building, expiry, and SAN/hostname verification
  are rustls/WebPKI defaults and run on every connection. The
  handshake is driven explicitly *before* any request bytes are
  written, so validation failures surface as the new
  `ProviderErrorCode::TlsError`, marked pre-send.
- **There is no insecure mode.** No danger/verifier-bypass API is
  used or exposed (verified by source scan: no `dangerous`,
  `accept_invalid`, custom `ServerCertVerifier`, or verify-off
  flags anywhere in `src/`), and no environment variable changes
  validation. The only trust customization is *adding* roots
  (`TlsConfig::add_root_pem` / `add_root_der`) for private-CA
  deployments — which extends trust to operator-chosen CAs and can
  never disable validation.
- Credentials stay bound to the single request the adapter built for
  the validated origin; since redirects are never followed (3xx is an
  `EndpointRejected`), a credential cannot be forwarded to a
  different origin. TLS errors contain the endpoint authority and the
  rustls failure detail (certificate facts), never credential
  material; adapter-level redaction is unchanged.

### Retry safety (the semantic change)

`ProviderError` now carries `request_sent` (default `true` — the
conservative assumption; transports explicitly mark DNS, connect and
TLS-handshake failures `pre_send()`). The dispatcher's
`may_retry` / `may_fallback` implement:

| Failure | Free model | Paid / Unknown-cost model |
|---|---|---|
| Pre-send (DNS, connect, handshake) | retry + fallback | retry + fallback |
| TLS validation (`TlsError`) | never retried | never retried |
| HTTP 429 (explicit non-execution) | retry (bounded, Retry-After) + fallback | retry (bounded, Retry-After) + fallback |
| Post-send 5xx / timeout / drop / malformed | bounded retry + fallback | **terminal: no retry, no fallback** |

Rationale: after the request was fully sent, a 5xx/timeout/drop is
ambiguous — the provider may have executed and billed it. Retrying,
or "falling back" to a second provider, can duplicate paid
inference, so billable dispatches surface the error instead and let
the caller decide. Unknown cost is treated as paid (M5-D5). Permits
are still consumed once per dispatch, never per attempt.

## Evidence (local, 2026-10-20, Rust 1.99.1)

`cargo test` in the workspace for `anansi-providers`: **93 tests, all
passing** — 48 unit (incl. the retry-policy matrix and TLS config
tests), 26 pre-existing synthetic scenarios (unchanged behaviour),
10 retry-safety scenarios, 9 HTTPS scenarios. Clippy
`--all-targets -- -D warnings` clean; `cargo fmt --check` clean.

The HTTPS suite (`tests/https_scenarios.rs`) exercises the **real**
`RustlsTransport` against a rustls-terminating synthetic server with
an in-process rcgen test CA (certificates are generated per run,
never written to disk):

- valid certificate → dispatch succeeds; credential observed inside
  the TLS session (OpenAI Bearer and Anthropic `x-api-key` variants);
- **wrong-host** certificate (trusted CA, wrong SAN), **expired**
  certificate, and **untrusted self-signed** certificate are each
  rejected with `TlsError` and **zero application hits** — the
  handshake fails before any request exists;
- SSE streaming, mid-stream cancellation (socket shutdown through
  the TLS session), redirect refusal, and bounded timeout all hold
  over HTTPS;
- paid-model 5xx over HTTPS is a single attempt (retry-safety × TLS).

The retry-safety suite (`tests/retry_safety.rs`) asserts exact
synthetic-server hit counts for every row of the policy table,
including: paid drop-after-send = 1 hit, paid timeout = 1 hit, paid
429 → retried and succeeds, paid connection-refused → falls back
(pre-send), free 5xx → 3 hits then fallback succeeds, paid stream
establishment 5xx → no fallback.

Baseline regression: the existing core/api/contracts suites were
re-run after this change; results match the pre-change baseline
(the only failures remain the 4 live-Ollama `semantic_workflow`
tests, which need a real Ollama on `127.0.0.1:11434` that this VM
does not have — environmental, unrelated).

## Dependencies

Crate (pinned exactly, per repo convention): `rustls =0.23.46`
(default-features off; `ring`, `std`, `tls12`), `rustls-pki-types
=1.15.1`, `webpki-roots =1.0.10`. Dev-only: `rcgen =0.13.2`,
`rustls-pemfile =2.2.0`, `time =0.3.47`. Root `Cargo.lock` gains the
rustls tree in its own commit; no other crate's manifest changed,
and nothing outside `crates/providers` + the lockfile was touched.

## Integration order (updated)

1. ~~TLS transport~~ **Done** (this addendum). The integrator's
   remaining choice is operational: dispatch with `RustlsTransport`
   (Mozilla roots). If the product later needs OS-native root stores
   instead of/alongside `webpki-roots`, that is a `TlsConfig`
   addition, not a redesign — flag it as an owner decision; the
   webpki snapshot updates ride crate releases.
2. Secret resolver over the keyring boundary (unchanged from the
   main handover).
3. `Core::ai_command` dispatch surface + contracts (unchanged).
4. UI + disclosure rendering (unchanged).
5. OAuth completion and Bedrock/Vertex (unchanged; still deferred
   for the reasons recorded).

## Limitations stated plainly

- HTTP/1.1 only. No HTTP/2, no ALPN protocol advertisement beyond
  rustls defaults, no WebSocket. Provider REST APIs used by the
  adapters all accept HTTP/1.1.
- No client certificates (mTLS) and no session-resumption tuning;
  neither is needed by the catalogued providers' API-key surfaces.
- TLS 1.2+ only (rustls has no older protocols — a feature).
- Real-provider verification is still pending: these tests prove
  the transport against synthetic servers with test certificates,
  not against live provider endpoints (no credentials, no paid
  inference — per the standing constraints).
