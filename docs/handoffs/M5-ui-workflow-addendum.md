# M5 addendum 7 — the provider configuration, credential setup and disclosure UI workflow

Agent: Muse (M5 implementation agent) · Date: 2026-10-21
Branch: `muse/m5-provider-integration`, continuing from `7301022`
(r7; the idle-cancel acceptance defect was closed at that
checkpoint by the owner's Windows verification).

## What was delivered

A complete desktop/browser workflow over the r4–r7 backend
boundary — no new trust assumptions, the UI is a client of the
same operations:

**Contracts client** (`packages/contracts`): ten typed methods
(catalog, configure, authorize, grant/revoke session, dispatch,
stream start/poll/cancel, usage) decoded with the generated
validators. The desktop (native) client gets them through the
existing generic `ai/*` invoke routing — in the desktop app the
user IS the host session, so owner operations just work there.
Failure mapping is provider-aware: owner-authorization refusals
are explained as such, and dispatcher detail (lock refusals,
permit refusals, quota, attempts) is surfaced verbatim because
disclosure is the point of the feature.

A real integration catch along the way: the HTTP client injects
`contractVersion: 1` into every request body and requires it on
responses — the provider DTOs initially did neither, so every
real UI call would have failed. The DTOs now follow the
codebase convention (requests accept/default the version and
refuse others as `UnsupportedContract`; responses carry it).

**The screen** (`packages/ui/src/Providers.tsx`, "AI Providers"
in the SYSTEM navigation group):

- **Catalog**: the §14 provider list with declared locality and
  status; selecting one pre-fills its endpoint.
- **Configure**: provider, base URL, models with cost classes,
  budget. Locality is shown as *derived from the endpoint* (the
  backend re-derives and rejects mismatches). Persistence: the
  configuration is saved per workspace (below) and survives
  restarts.
- **Approved credential setup**: the only credential input is a
  **reference** — the OS-keyring account name. There is no secret
  field anywhere in the screen; the copy instructs the owner to
  place the key in the OS keyring under that reference (§15).
- **Authorize**: issues the single-use paid permit (token cap,
  visible expiry). In a browser without a host grant this is
  refused and the refusal is a first-class state, not a silent
  failure.
- **Dispatch**: model, the eight routing modes verbatim (§17),
  the Zero-Cost Lock toggle (default on), streaming toggle,
  prompt. Streaming renders tokens live with a working Cancel;
  terminal states (done / cancelled / error with the provider
  error code) are distinct.
- **Disclosure** ("View what ANANSI sent", §19): provider, model,
  endpoint host, local?, cost class, routing mode + reason,
  message/segment counts, redactions, estimated tokens, actual
  usage, and every attempt with its outcome.
- **Usage**: session totals and per-provider accounting with
  live reservations.

## Configuration persistence (core)

The trusted configuration (endpoints, models, budget — credential
REFERENCES only, never secrets) persists per logical workspace
as one JSON file under `<storage root>/provider-config/`, with
the storage layer's link-rejection and private-permissions
discipline and atomic write+rename. It is loaded lazily on first
use after a restart and re-validated with the **full
configure-time rules** — bounds, derived locality, runtime build
— so a tampered file cannot smuggle a masquerade the configure
endpoint would reject. Permits, grants, stream runs and usage
totals deliberately do **not** persist: they are session-bound
security/accounting state and a restart correctly resets them.

Restart evidence uses real child processes (the backend registry
is process-global, so an in-process "restart" would prove
nothing): one process configures, a fresh process dispatches
from the persisted file alone, and a file with its locality
flipped is refused with zero provider traffic.

## Verification (Linux; synthetic throughout)

- Providers crate 119/119; core provider suite 13/13 including
  the cross-process restart tests; API suite green.
- UI + contract tests 36/36, including the Providers screen's
  workflow markup (no secret field exists), the locality mirror
  rule, and the disclosure panel. `tsc` clean.
- Regression baseline identical (only the four pre-existing
  live-Ollama failures).
- **Browser end-to-end** (`tests/e2e/providers.spec.ts`,
  Playwright, the repo's own harness): real web build + real API
  binary + a synthetic loopback Ollama server. It exercises
  rejected authorization (browser configure → owner refusal),
  owner configuration arriving as the persisted file across an
  API restart, unary dispatch with disclosure and usage,
  streaming with a mid-stream cancel, the Zero-Cost Lock refusal,
  the missing-permit refusal, owner-only authorize, and error
  recovery when the provider goes down and returns.
  Result on this Linux VM: **passed** (1 test, ~5 s) against
  the image-provided Chromium headless shell — the pinned
  browser's CDN download failed repeatedly in this environment,
  so the run used a temporary, uncommitted config pointing
  Playwright at the local binary; the committed spec and the
  repo's own `playwright.config.ts` are untouched, and the
  owner's environment runs the canonical harness. The e2e earned
  its keep: it caught two real bugs that the unit suites had
  not — the screen's stream state machine never landed on
  'cancelled' when the poll loop exited via the cancel button
  (the form hung in 'streaming'), and the client's Forbidden
  mapping conflated the owner boundary with the dispatcher's
  permit refusal (`PermissionRefused`), which is actionable
  from the screen and is now surfaced verbatim.
- Windows verification of this round is the owner's to run; the
  Linux results above are not a substitute for it.

## Synthetic vs live — stated plainly

Everything here is verified against **synthetic** providers
(loopback HTTP/HTTPS fakes) and fake credential references. This
is **workflow verification, not live-provider certification**:
no real provider has been called, no real credential exists,
and no paid inference has run. Live-provider verification
remains owner-gated, as do OAuth completion and the
Bedrock/Vertex adapters.
