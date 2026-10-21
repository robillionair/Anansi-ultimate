# M5 addendum 6 — acceptance defect: cancellation of a blocked read on a silent provider

Agent: Muse (M5 implementation agent) · Date: 2026-10-21
Branch: `muse/m5-provider-integration`, continuing from `54c27f9`
(r6). Status: **CLOSED at checkpoint `7301022`** — the owner's
independent Windows verification of r7 (2026-10-21) passed all
119 provider tests, all 12 core integration tests including both
previously failing cancellation tests, the API two-session
isolation test, formatting and documentation checks.

## What the r6 Windows run showed

The r6 storage-latency fix worked: both previously failing tests
got past authorization and reached cancellation. They then timed
out waiting for terminal completion —
`cross_session_stream_access_is_denied` (provider_dispatch.rs:735)
and `slow_dispatch_does_not_block_poll_cancel_or_other_workspaces`
(:843). The gated test server holds its stream open and sends no
further bytes; existing stream tests all keep dribbling data,
which masked the case.

## The trace (cancel flag → terminal event → settlement)

1. Core cancel sets the `CancellationToken`.
2. The token shuts down its registered socket clones.
3. **The blocked read:** `SocketBody::read` checked the flag once,
   then blocked in a single `recv` for up to
   `min(idle, total remaining)` — 30 s at production defaults.
   Waking that `recv` depended entirely on step 2's `shutdown()`
   interrupting it. On Linux it does; **on Windows `shutdown()`
   from another thread does not reliably interrupt a blocked
   `recv`** — so the worker stayed parked.
4. Worker exit, the terminal `Cancelled` event, and the
   reservation settlement all live *after* that read in the
   reader thread — so none of them happened within the tests'
   deadlines. Nothing was faked at the UI layer; the work was
   genuinely still active, which is why this was an acceptance
   defect and not a test bug.

## The fix

Correctness no longer depends on `shutdown()` semantics:

- **Sliced reads.** No socket read in `SocketBody` blocks longer
  than 100 ms. Between slices the loop re-checks the cancel flag
  and recomputes both deadlines — idle measured from the last
  bytes received (not from the read call), total from the
  execution start. A slice expiry is not an error; the loop head
  produces the real `Timeout`/`Cancelled` when its condition
  fires. The response-header wait in `run_over` is sliced
  identically. `shutdown()` remains as an accelerator.
- **Cancel normalization.** A read failure or EOF that coincides
  with cancellation is reported as `Cancelled`: the token's
  shutdown can otherwise surface as a TLS abrupt-close error
  (`UnexpectedEof`) and be misreported as `ConnectionLost` —
  the new transport regression caught exactly that race on its
  first run.

## Regression evidence (Linux)

`crates/providers/tests/idle_stream_cancel.rs` — a gated TLS
server (test-only CA) sends headers + one NDJSON chunk, then
holds the connection in silence. Timeouts are set to 120 s idle /
300 s total, so only cancellation can end the stream:

- Transport level: the blocked read surfaces `Interrupted`
  ~0.35 s after cancel (300 ms cancel timer + one slice).
- Dispatcher level: the first token arrives; the request is
  accounted and the token hold is live mid-stream; on cancel the
  reader worker emits the `Cancelled` event, the channel closes
  promptly, and the ledger settles — reservations back to (0, 0),
  the request accounted with zero tokens.

Full suites on Linux after the fix: providers 119/119 (incl. the
deadline suite — idle/total semantics unchanged), core provider
suite 12/12 with both acceptance tests 5/5 on repeat, API suite
green, core/contracts regression baseline-identical, Clippy
`-D warnings` and fmt clean.

## Closure

- **Windows rerun of r7 passed** (owner, 2026-10-21): 119
  provider tests, 12 core integration tests, API two-session
  isolation, formatting and documentation checks. This defect is
  closed at checkpoint `7301022`.
- The configuration / credential setup / disclosure UI slice
  proceeds independently; it does not depend on this fix beyond
  the shared transport.
