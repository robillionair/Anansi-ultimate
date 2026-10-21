# M5 addendum 3 — application authorization boundary repair

Agent: Muse (M5 implementation agent) · Date: 2026-10-20
Branch: `muse/m5-provider-integration`, continuing from `96188b7`
(r3, independently verified on Windows: 105 provider tests, Clippy,
formatting, docs). This round repairs the authorization boundary of
the r3 application integration before any UI is built on it. The
verified tip and evidence for this round are recorded here (the
integration owner's `docs/BUILD_STATUS.md` summary is theirs to
update on merge; this branch does not edit it).

## What was wrong at r3

- Dispatch requests carried the entire provider configuration —
  locality, model cost classes, credential references — plus a
  caller-minted permit object. A caller could declare a cloud
  endpoint "Local/Free" and authorize its own paid inference.
- Stream run ids were sequential (`provider-run-N`) with no
  capability and no caller binding: any caller could poll or
  cancel any run.
- The runtime was rebuilt per configuration fingerprint, so the
  single-use permit set and the usage ledger reset on every
  configuration change (A → B → A replayed permits).
- Runtime dispatch used `Budget::default()` — configured budgets
  were never wired in — and budget checks were check-then-act:
  concurrent dispatches could collectively overrun a budget, and
  streamed tokens were never recorded.

## The repair, item by item

1. **Caller, workspace, session binding + run capabilities.**
   Every provider operation runs under a `CallerScope`: the
   workspace is derived by `Core` from its own storage root; the
   session is stamped by the API layer from the authenticated
   bearer token (a fingerprint — the token never leaves the API
   process), **overwriting** any client-supplied `callerContext`,
   which the core consumes as transport metadata. Stream start
   returns a 128-bit random capability (OS randomness) beside the
   run id; poll/cancel require both, compared in constant time.
   Wrong capability, wrong session or wrong workspace are all
   answered `NotFound` — run existence is not an oracle.
2. **Permit/usage persistence across configuration changes.** The
   used-permit set and the quota ledger live in per-workspace
   backend state, created once, and are shared into every runtime
   (`ProviderRuntime::with_shared_accounting`). The A → B → A
   replay test proves a consumed permit stays consumed and usage
   totals survive reconfiguration.
3. **Policy from trusted backend state only.** Configuration is
   installed by `providers-configure`; paid authorization is
   issued by `providers-authorize` (single-use, session-bound,
   expiring). Dispatch/stream requests use
   `deny_unknown_fields` DTOs with no configuration, locality,
   cost, credential or permit fields — the r3 inline shape fails
   closed. **Locality is derived** from the validated endpoint at
   configure time and a contradicting declaration is rejected
   (both masquerade directions tested). The configured model set
   is the dispatch model universe in every routing mode.
   Credential references exist only inside the trusted
   configuration; resolution still happens in the runtime via the
   keyring resolver.
4. **Configured budgets, reservations, streaming accounting.**
   `RuntimeConfig` carries a `Budget`, wired into every dispatch.
   `QuotaLedger` gained reservations: `reserve` checks and holds
   atomically under the ledger lock (budget checks count actuals +
   reserved), so concurrent dispatches cannot overrun. Unary
   dispatches settle/release per candidate; streams record the
   request at establishment and the reader thread settles the
   token hold with the provider-reported usage when the stream
   ends — completion, error and cancellation alike. The
   `Transport`/`SecretResolver`/`ProviderAdapter` traits gained
   `Send + Sync` supertraits so a dispatcher can be shared across
   threads; the concurrency test (4 in flight, budget 2) yields
   exactly 2 successes, 2 `QuotaExceeded`, 2 provider hits.
5. **Bounded streams and queues.** At most 32 retained runs per
   workspace; runs expire after 600 s (cancelled by the sweep that
   runs on every operation, then dropped); finished runs are
   removed by their terminal poll; poll drains at most 256 events
   per call. Event-queue memory is bounded by the transport's
   4 MiB response cap and the same 600 s lifetime. Expired permits
   are swept on the same pass.
6. **Generated, validated contracts.** The ten provider DTOs are
   in `crates/contracts` output: generated TypeScript types +
   standalone validators in `packages/contracts`, with
   `additionalProperties: false` and the numeric bounds published
   in the schema (permit caps 1..10_000_000, TTL 30..3600). Strict
   bounds are also enforced in Rust before any state change or
   traffic (32 providers, 128 models/provider, 256 messages, 64
   segments/message, 32 768 chars/segment, 262 144 chars/request,
   1 000 000 max output tokens). One generator fix was needed:
   `scripts/contracts.mjs` de-duplication now recognizes type
   aliases followed by doc comments, so identical aliases reachable
   from several schema roots are emitted once (conflicts still
   rejected).

## Adversarial evidence

- `crates/core/tests/provider_dispatch.rs` — 9 scenarios through
  `Core::ai_command`: inline-config dispatch rejected; cloud-as-
  Local and loopback-as-Cloud configurations rejected *by reason*
  (message asserts); forged, borrowed and embedded permits refused
  with **zero provider traffic** (fake servers count hits);
  A → B → A replay refused; cross-session poll/cancel `NotFound`
  while the owner's stream survives; caller-supplied budget field
  rejected and configured budget enforced; bounds enforced before
  traffic.
- `crates/api/tests/provider_boundary.rs` — two authenticated
  sessions over real HTTP sharing one workspace: session B cannot
  poll/cancel session A's run (404), a forged `callerContext` is
  overwritten and buys nothing, unauthenticated requests get 401,
  and the owner's own access keeps working.
- `crates/providers/tests/accounting.rs` — concurrency and stream
  accounting as above; ledger reservation unit tests in
  `gates.rs`.

## Remaining before/with the UI

- The provider configuration / credential setup / disclosure UI
  ("AI Matrix / Integrations") is the next slice; everything it
  needs is now in place: catalog/configure/authorize/dispatch/
  stream/usage operations, generated types and validators in
  `packages/contracts`, and a boundary that does not trust it.
- OAuth completion, Bedrock/Vertex, real-provider verification —
  unchanged, still deferred; no real credentials or paid inference
  were used in this round.
