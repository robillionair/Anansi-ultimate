# MCP Client Integration — Slice 1 Repair Addendum (Milestone 13, §68)

Branch `muse/mcp-slice1-repair` from the Slice 1 delivery tip
`4b0b42e0077802490ee468b2c7304e28fd0f9de7`, as instructed.
Note: no `codex/review-mcp-slice1` branch exists on origin at the
time of this repair — the Windows finding arrived as a review
receipt in chat, not as a review-branch commit. The repair
therefore branches from the instructed delivery tip exactly, and
touches only `crates/core/src/mcp.rs` plus this repo's docs.
Nothing else changed: no contracts, no UI, no DTOs.

## The Windows finding

Windows review of Slice 1: static gates, UI 289/289, Vite build,
and Rust workspace unit tests 220/220 all GREEN; Playwright
tests 1, 3, and 4 passed. Test 2 (stdio registration probes a
real local server) failed on Windows only.

Root cause, as diagnosed by the reviewer and confirmed in code:
`StdioChannel::spawn` called `.env_clear()` and then applied
only the server's configured env. On Windows the CRT and Node's
crypto provider need `SystemRoot`/`WINDIR` to locate
`C:\Windows\System32` and initialize the system CSPRNG; without
it a Node.js MCP server aborts in ~0.11s with exit code 134
(`Assertion failed: ncrypto::CSPRNG(nullptr, 0)`) before running
any script. Linux never reproduced this: nothing in the Linux
runtime boot path needs those variables.

Secondary finding, also confirmed in code: when the child dies on
startup, the stdout reader thread breaks on EOF and drops its
sender, so `recv_timeout` returns `RecvTimeoutError::Disconnected`
— which the probe mapped to the generic "Probe timed out waiting
for the server.", masking a startup crash as slowness.

## The repair (two changes, both in `crates/core/src/mcp.rs`)

1. **Windows system-env preservation.** `spawn` still clears the
   child environment. On Windows only (`#[cfg(windows)]`), it then
   restores `SystemRoot`, `WINDIR`, and `PATH` from the parent
   process — but only variables the server configuration does not
   already supply (comparison case-insensitive, because Windows
   env names are), and the configured env is applied after the
   restored entries, so explicit configuration always wins.
   Security posture preserved: the parent environment is still
   never inherited wholesale — only three system paths cross the
   boundary, and none of them can carry credentials. The
   selection logic is a pure function
   (`StdioChannel::preserved_env_entries`) so it is unit-tested
   on any platform even though the call site is Windows-only.
   On Linux and other platforms the child keeps exactly the
   configured environment, as before.

2. **Disconnect reported distinctly.** A new `channel_recv_error`
   maps `RecvTimeoutError::Timeout` to the existing timeout text
   and `RecvTimeoutError::Disconnected` to
   "Server exited or closed connection before replying." The
   probe's `request` path uses it for every line read.

## Tests added (3, all in `mcp.rs`)

- `preserved_env_keeps_system_vars_and_config_wins`: a synthetic
  parent env with `SystemRoot`/`WINDIR`/`PATH` plus a
  credential-like variable yields exactly the two system vars;
  a configured `Path` (different case) suppresses the parent's
  PATH, and the credential-like variable never crosses.
- `channel_recv_error_distinguishes_disconnect_from_timeout`:
  both arms of the mapping, verbatim.
- `stdio_channel_reports_disconnect_when_child_exits_silently`
  (Unix-only; it relies on `sh`, which Windows test runs do not
  have): a real child that reads one stdin line and exits
  without replying produces the disconnect message through the
  full `StdioChannel::request` path, not a timeout.

## Verification (Linux)

- `cargo test --package anansi-core --lib mcp::tests`: 8/8 PASS
  (5 existing + 3 new).
- `cargo test --workspace --lib`: 223 passed, 0 failed (220 + 3
  new). The reviewer's "expect 220/220" predates the 3 repair
  tests; the total grew by exactly those 3, with zero failures.
- `tsc --noEmit`: 0 errors. `contracts.mjs --check`: 0 drift.
  `validate-docs.mjs`: PASS (268 files).
- `npm run test:ui`: 289/289, 0 fail.
- Production web build clean; API build clean.
- `mcp.spec.ts`: 4/4 PASS.
- Full regression battery: 50/50 PASS (sequential, real spec
  filenames, full summaries).

## Honest limit

Linux verification cannot prove the Windows fix: the bug is a
Windows CRT/runtime boot dependency, and no Linux child needs
`SystemRoot`. What Linux proves is that the repair is behavior-
preserving off Windows (env still exactly configured, battery
green) and that the new code paths are correct as far as they
can be exercised here. The decisive check is the Windows rerun
of `mcp.spec.ts` test 2 against a real Node MCP server — that
is the reviewer's rerun, and this repair must not be presented
as Windows-verified until it passes there.

## Next

- Windows rerun of this repair (checklist in the delivery
  MANIFEST).

Last verified commit: tip recorded in the delivery MANIFEST.
