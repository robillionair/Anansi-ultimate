# M5 addendum 2 — overall deadline, keyring resolver, runtime, app integration

Agent: Muse (M5 implementation agent) · Date: 2026-10-20
Branch: `muse/m5-provider-integration`, continuing from `c24a261`
(the transport addendum). The integration baseline was re-checked
before this round: `origin/codex/m2-m3-integration` still tips at
`fc2540f` and master at `8c9406b`, so this branch still sits on the
latest baseline; nothing needed rebasing.

## 1. One overall request deadline (transport)

`TimeoutConfig::total_ms` is now a single clock started in
`Transport::execute` and shared by **every** phase of an execution:
DNS resolution, connect, TLS handshake, request write, response
read. Per-phase timeouts still bound inactivity within a phase; the
deadline bounds the whole. This matters because per-read timeouts
alone cannot stop a peer that dribbles bytes slowly enough to keep
every read "active" — the socket-idle reasoning in the original
task does not establish a total bound, and now the code does not
rely on it.

How each phase is bounded:

- **Connect** — `connect_timeout` with `min(connect_ms, remaining)`.
- **TLS handshake** — driven on a worker thread; the waiter enforces
  the deadline and cancellation by closing the socket. Validation
  still completes before any request byte exists.
- **Write** — socket write timeout clamped to the remaining budget,
  deadline re-checked after the write.
- **Read** — header loop and body reader check the shared clock; the
  body reader's `total` now starts at `execute()` entry rather than
  at the first body byte.

**DNS limitation, stated plainly:** `std`'s resolver
(`ToSocketAddrs`) is a blocking OS call with **no timeout and no
cancellation hook** — that API cannot be interrupted, full stop.
Mitigation implemented: resolution runs on a worker thread and only
the *wait* is bounded (by `min(connect_ms, remaining deadline)` and
by the cancellation token). If the wait expires, the dispatch fails
with a pre-send `Timeout` and the worker is abandoned; it owns only
the hostname string (no request or credential material) and exits
whenever the OS resolver returns. Operators who need stronger DNS
control should front providers with endpoints that resolve fast or
use IP-literal/internal names — the deadline guarantees the
*dispatch* is never held hostage either way.

Evidence (`tests/deadline.rs`): a proxy that dribbles a **genuine**
rustls handshake flight at 3 ms/byte (per-read timeouts set to
10 s — they never fire) is cut at a 1.2 s deadline with a pre-send
`Timeout` and zero application hits; a silent-server handshake
times out pre-send; a dribbling response body is bounded; the fast
path is unaffected. All pre-existing suites still pass.

## 2. Native keyring SecretResolver

`crates/providers/src/keyring_resolver.rs` implements the existing
`SecretResolver` boundary over the OS credential store via
`keyring` **3.6.3** — the version and per-target feature pattern
`anansi-core` already uses, including core's explicit
`linux-secret-service` opt-in feature (mirrored as a crate feature;
without it, Linux resolution fails closed, never mock).

- Provider keys live under service
  `com.robillionair.anansi.providers`, one entry per credential
  reference — beside the vault's store
  (`com.robillionair.anansi.vault`), never inside it; the resolver
  never sees vault material and the vault design is untouched.
- `store_api_key` / `delete` are the write half of the §15 setup
  and rotation flows. Secrets pass through memory only; every error
  is `AuthFailed` + pre-send and carries the reference plus a
  bounded store message — never secret bytes.
- Tests use the keyring crate's mock builder: entry handles are
  cached per reference (the mock scopes data to the entry
  instance). Roundtrip, missing-reference fail-closed, delete, and
  no-secret-in-errors are covered. **No test touches a real
  credential store, and no real credential exists anywhere in this
  work** — chat, logs and files included.

One naming note for the integrator: the vault builds entries with
explicit target control (`new_with_target`); the resolver uses the
portable `Entry::new` so the mock builder can intercept it in
tests. The namespaces are separate services, so entries cannot
collide; if byte-level target-name parity with vault entries is
ever required, it is a contained change inside this one module.

## 3. ProviderRuntime + the complete workflow

`crates/providers/src/runtime.rs` is the application-facing seam:

- `RuntimeConfig` (JSON; credential references only) is validated
  **eagerly** at construction — duplicate providers and endpoint
  violations fail immediately (new `InvalidConfiguration` code).
- `ProviderRuntime` owns the transport, resolver, quota ledger and
  the single-use permit set across calls (`Dispatcher` gained
  `with_used_permits` for exactly this): a permit cannot authorize
  two dispatches because the application called twice.
- `dispatch` / `dispatch_stream` return the dispatcher's outcomes,
  disclosure included.

`tests/full_workflow.rs` proves the whole chain against a synthetic
HTTPS provider (test CA) with the keyring mock: configuration JSON
→ key stored → permit-authorized dispatch (the server observes
exactly the stored key) → response + disclosure → permit reuse
refused with zero network traffic → Zero-Cost Lock refusal →
streamed dispatch → mid-stream cancellation (disclosure intact) →
ledger totals.

## 4. Application integration (anansi-core + anansi-api)

- `anansi-core` now depends on `anansi-providers` and exposes five
  `Core::ai_command` operations from the new
  `crates/core/src/provider_dispatch.rs`: `providers-catalog`,
  `providers-dispatch`, `providers-stream-start`,
  `providers-stream-poll`, `providers-stream-cancel`.
- The runtime is process-wide, cached by configuration fingerprint
  (permits and ledger persist across operations; a configuration
  change replaces the runtime).
- Provider errors map to `CommandError` codes (config →
  `InvalidRequest`, credential → `Unauthorized`/`Forbidden`,
  dispatch → `StorageUnavailable`) with the provider code name
  preserved at the front of the message. The mapping is documented
  in the module for the API owners.
- `anansi-api` allow-lists the five `/v1/ai/providers-*` routes.
- `crates/core/tests/provider_dispatch.rs` runs the workflow
  through `ai_command` against a fake loopback Ollama: catalog,
  unary dispatch with disclosure, stream start/poll token flow, and
  a slow stream cancelled via `providers-stream-cancel` surfacing a
  `Cancelled` event on poll.

## Remaining integration work (unchanged ownership)

- UI rendering of the catalog, configuration and disclosure
  ("View What ANANSI Sent") — the data is now available through the
  operations above.
- OAuth completion and Bedrock/Vertex — still deferred (see the
  main handover).
- **Real-provider verification is still pending and still requires
  explicit owner authorization**: everything above is synthetic —
  test certificates, fake keys, loopback servers. No real
  credential, paid inference, cloud provisioning or CI run was used
  in this round either.
