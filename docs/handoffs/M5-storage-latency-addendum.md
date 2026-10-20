# M5 addendum 5 — Windows storage latency in the provider command path

Agent: Muse (M5 implementation agent) · Date: 2026-10-21
Branch: `muse/m5-provider-integration`, continuing from `0ca7c22`
(r5). Trigger: the owner's Windows verification of r5 passed all
117 provider tests and the API two-session test, but two core
integration tests failed — `cross_session_stream_access_is_denied`
(owner cancel returned `NotFound`) and
`slow_dispatch_does_not_block_poll_cancel_or_other_workspaces`
(poll took 1.58 s against an 800 ms bound). Both passed on Linux.

## Root cause, measured separately from lock contention

The r5 backend-lock fix was real (dispatch no longer holds the
mutex across network calls) but it was not the whole path. Every
provider operation ALSO validated its workspace through
`Storage::list()` in the `ai_command` arm. On Windows that call
is expensive by construction:

- `Storage::connection()` calls `root_ready()`, which calls
  `private_permissions(root, verify)` — on Windows this spawns
  `powershell.exe` (storage.rs, `protect.ps1`).
- `connection()` then opens the registry database, calling
  `private_permissions(db_file, verify)` — a second spawn.

So one `Storage::list()` costs **two PowerShell process spawns**
on Windows (commonly ~0.5–1.5 s combined on a loaded machine —
matching the observed 1.58 s poll), while on Unix
`private_permissions` is a local metadata check and the same code
costs microseconds. This is a per-call platform asymmetry in the
storage layer that r5's validate-on-every-operation design
multiplied into the hottest loop in the system: stream polling.

Failure mechanics, both tests:

1. **Concurrency test:** the poll paid its own ~1.5 s storage
   validation on top of everything else; the backend lock was
   already free. Lock contention was not the (remaining) cause.
2. **Cross-session test:** the intruder poll/cancel sequence and
   the owner's first poll each paid seconds of validation, so the
   2.4 s dripping test stream finished (and its run was harvested
   by the terminal poll) before the owner's cancel arrived —
   hence `NotFound` for a legitimate cancel.

## The fix (protections preserved)

- **Poll/cancel never touch storage.** They address backend
  state by key and authorize purely through the run record:
  existence in that workspace's backend + session binding +
  unguessable capability. The lookup is now non-creating
  (`with_backend_existing`), so an invented workspace id finds no
  run and gets the same `NotFound` a wrong capability gets. Cost
  per poll: one mutex + channel drains, on every platform.
- **State operations keep registry validation, cached.** Configure,
  authorize, grant/revoke, dispatch, stream-start and usage still
  validate the workspace id through `Storage::list()` — but a
  positive result is cached per (root, workspace) for the
  process. That is sound here because workspace ids are created
  through the domain operations and **never deleted** in this
  codebase; misses are deliberately not cached, so a workspace
  created later becomes visible on its very next operation. A
  counting unit test pins both halves (second validation =
  zero registry reads; unknown id = refused and re-checked next
  time).

## Test synchronization (no thresholds were raised)

The latency bound (800 ms) is unchanged; what changed is that
the tests no longer depend on scheduler timing:

- The fake server grew a **gate**: a "gated" stream sends three
  tokens, then holds the stream open until the test releases it.
  The cross-session test now proves the stream is active (gate
  closed, tokens observed, `done == false`) at the moment the
  owner cancels — on any platform.
- The fake also flags **slow-unary in flight**; the concurrency
  test waits for the server to actually be processing the slow
  request before measuring poll/cancel/other-workspace latency,
  instead of sleeping a guessed 300 ms.

## Verification (Linux, this machine)

- Both named tests: **10/10 consecutive passes** each.
- Full core provider suite: 12/12. Providers crate: 117/117.
  API suite: all binaries green, including the two-session test.
  Core/contracts regression baseline-identical (only the four
  pre-existing live-Ollama failures).
- Clippy `-D warnings` (providers), fmt, docs validation: clean.

**Not claimed:** Windows verification of this round. The fix
removes the per-operation PowerShell cost from poll/cancel
entirely and from repeated validations for the rest, by
construction; the owner's Windows rerun is the confirming
evidence, and this addendum will be corrected if it disagrees.
