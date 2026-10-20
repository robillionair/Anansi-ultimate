# M5 addendum 8 — credentials, session approval, cancellation honesty, persistence hardening

Agent: Muse (M5 implementation agent) · Date: 2026-10-21
Branch: `muse/m5-provider-integration`, continuing from `c813857` (r8).

## 1. Approved credential setup/removal (was: reference only)

`providers-credential-set` / `-remove` / `-status` are
host-only operations (a stamped browser session is refused
before anything else happens, and the refusal stores nothing).

Set works the way the vault's secrets do (§15's "official
credential location + OS vault", the vault architecture
mirrored, not replaced): a **trusted native prompt**
(Windows `CredUIPromptForCredentialsW`, `DO_NOT_PERSIST` — the
prompt itself saves nothing) collects the key on the host; the
key moves from the prompt buffer (zeroized) straight into the
OS credential store under the providers keyring service via
the resolver's write half. The secret never crosses an HTTP or
IPC DTO, never appears in a file, log, or configuration —
configuration continues to hold only the reference. Removal
deletes the store entry; status reports presence without
reading the secret out.

Core and every runtime now share one process-wide
`KeyringSecretResolver` (`Arc<KeyringSecretResolver>` implements
`SecretResolver`), so the entry cache — and under the keyring
mock builder, the store — is shared rather than per-instance.

Debug builds mirror the vault's test-provider pattern
(`ANANSI_PROVIDER_TEST_CREDENTIAL`) so the flow is testable
without a human at a prompt; the env hook is compiled out of
release builds. On non-Windows hosts the prompt reports
itself unavailable in the desktop app — stated plainly in the
UI copy and the error.

**Proof** (core integration, mock keyring + test prompt):
browser set → Forbidden, nothing stored; host set → stored;
status flips; an OpenAI-shaped config referencing the key
dispatches and the synthetic server records exactly
`Authorization: Bearer <the fake key from the prompt>`; host
remove → status absent, dispatch fails closed (Unauthorized)
with zero additional traffic.

## 2. Host-side browser-session approval

Refused owner attempts by browser sessions are now recorded in
a bounded, swept per-workspace **approval queue** (session
label, first/last seen, attempt count; capped at 16, 24 h TTL).
`providers-session-access` (host-only) returns the queue plus
live grants; granting clears the queue entry.

The Providers screen has a **Browser session access** card:
pending requests with Approve (30 min) and live grants with
Revoke, naming the exact session fingerprint and workspace.
In a browser the card explains the session cannot approve
itself — the workflow is host-side by construction.

**Proof**: core test (queueing, attempt aggregation, host-only
queue, grant clears + opens, revoke closes) and an API test
running the **actual granted-browser workflow over real HTTP**:
browser configure → 403 → host reads the queue and learns the
session identity from it → browser self-approval attempts →
403 → host grant → browser configure → 200 → host revoke →
403. This replaces r8's out-of-band file injection for the
grant path (the injected file remains how r8 tested
persistence itself).

## 3. Cancellation honesty in the UI

The stream state machine was rebuilt to the owner's rule:
**'cancelling' until the backend confirms**.

- Cancel enters `cancelling`; the watch loop keeps polling and
  the terminal state (`cancelled` / `done` / `error`) comes
  only from the run's terminal event plus `done`. No local
  declaration of cancellation — the r8 shortcut is gone.
- If the cancel call itself fails, the failure is surfaced,
  the state returns to `streaming` (still watched), the
  capability is retained, and Cancel can be retried.
- Transient poll failures retry with a visible notice; if
  watching is ultimately lost, the error says the run may
  still be active and offers **Resume watching** with the
  retained capability — a run is never abandoned silently.
- Unmount (navigation / workspace switch) cancels the active
  run best-effort; capabilities are never carried across views.

Browser proof: the e2e's stream-cancel stage still passes
against the new machine, and a new stage abandons a 12-second
drip stream by navigating away and asserts — **timed** — that
the usage reservation clears by cancellation, not by waiting
the stream out.

## 4. Configuration persistence hardening

- **Unique create-new temporaries**: unpredictable names
  (getrandom), `create_new` — a pre-planted file at a guessed
  name is never followed or truncated; leftovers are removed
  on failure (a test asserts none remain).
- **Link/reparse checks before writes**: destination and
  directory are `reject_links`-checked; a symlinked destination
  is refused, not replaced (failure-injection test: target
  sentinel untouched, previous configuration stays live).
- **Fsync before rename, directory sync after**.
- **Transactional configure**: persistence happens BEFORE the
  in-memory install, inside the backend lock — on failure the
  operation fails and memory is exactly as before
  (failure-injection test: sabotaged config path → configure
  errors, dispatch reports "not configured", zero traffic),
  and concurrent configures serialize whole (race test: the
  file is always exactly one whole configuration).

## Verification (Linux; synthetic throughout)

Providers 120/120 · core provider suite 18/18 · API 21/21
(boundary + granted-browser workflow) · UI/contracts 36/36 ·
tsc clean · providers clippy `-D warnings` clean (core carries
only the pre-existing vault dead-code warning on Linux) ·
browser e2e passed (real web build + API binary + synthetic
loopback provider, image Chromium headless shell as in r8) ·
regression baseline identical · docs validation clean.
Windows verification is the owner's; Linux results are not
presented as a substitute.

Synthetic vs live, unchanged and explicit: the credential
lifecycle is proven against the keyring **mock** and a test
prompt; the Windows CredUI path is compiled and mirrors the
vault's shipped prompt, but no real credential, real OS store,
or live provider has been touched. Live-provider
certification remains owner-gated.
