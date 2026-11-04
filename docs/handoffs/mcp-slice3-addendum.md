# MCP Slice 3 Addendum — MCP Server Core

Owner-authorized Milestone 13 slice (MASTER_SPEC §68) on branch
`muse/mcp-slice3` from the Windows-certified Slice 2 review tip
`a3c517267d58761a63e9d50795a406f0d02b5810`.

## Baseline verification (from origin, by hash)

- Origin's `codex/review-mcp-slice2` head is exactly the assignment tail
  `a3c517267d58…`; its parent is exactly the Slice 2 delivery tip
  `22cef4967`; the diff is docs-only (BUILD_STATUS, RECOVERY,
  REQUIREMENTS_MATRIX, new 83-line
  `docs/reviews/windows-mcp-slice2-verified.md`).
- The report confirms the Slice 2 delivery hashes (ZIP `16efd992…`, bundle
  `edc46692…`, tip `22cef4967`) and certifies Slice 2 CLOSED on Windows
  (tsc 0, contracts 0, docs 272, UI 293/293, Vite 4.46s, Rust lib 227/227,
  `mcp.spec.ts` 6/6, battery 52/52).
- Two arithmetic notes, not discrepancies: docs 272 = this slice's 271 + the
  review report; Rust lib 227 on Windows = this slice's Linux 228 minus one
  platform-gated test (the Unix-only silent-child disconnect test does not
  compile on Windows). The report's "13/13 MCP core tests" reads as a source
  count; the Windows run total is consistent with the platform gate.
- The certified Windows package itself (`anansi-mcp-slice2-windows-verified-2026-11-04.zip`,
  SHA-256 `a9eeb6e5…`) sits on the owner's Windows path and was not
  re-hashed from here — same sidecar-only pattern as prior rounds; the
  content-addressed review commit on origin is what this slice branches from.

## Scope delivered

Exactly three read-only tools over NDJSON stdio: `memory.search`,
`memory.get`, `projects.search`. New module `crates/core/src/mcp_server.rs`
(transport-agnostic `serve` loop over `BufRead`/`Write`, backend trait,
production backend on `Core::memory_command`), plus a `--mcp-server` /
`--workspace-id` mode on the existing `anansi-api` binary. Design committed
before code (`docs/handoffs/mcp-slice3-design.md`).

Security posture, all enforced and tested:

- Disabled by default: no flag → HTTP server; flag without
  `ANANSI_MCP_SERVER_TOKEN` → the process refuses to start (covered by its
  own e2e test).
- Token gate: `initialize` must carry the token in
  `_meta.anansiMcpToken` (an ANANSI extension, documented as non-standard);
  comparison folds bytes without early exit; failed auth does not kill the
  process, so a client can retry. Token from environment only, never logged
  or echoed.
- Allowlist enforced in dispatch before any backend call: unknown tool names
  (including the write tool `ideas.create`) are refused with no backend
  contact — asserted by a unit test on the backend call log.
- Sealed boundary in three layers: storage SQL exclusion + `allow_plaintext`
  backstop (inherited), backend read errors mapped to not-found, and a
  server-layer drop/refuse tested with a fake backend that deliberately
  returns sealed rows. Missing and sealed `get` produce the identical
  `Memory not found.` result.
- Read-only by construction: the server's only backend operations are `list`
  and `read`, and it constructs `Core::with_storage_root` directly so the
  export worker and backup scheduler background writers never run in the
  server process.
- Deliberate splits, stated not hidden: search matches titles/snippets only
  (core title-search path) and never returns bodies; `get` is the only path
  returning a body, for non-sealed records only. "Active" projects = not
  superseded, not archived, not expired. One token = one fixed permission
  set; per-token tool grants are deferred.

No new HTTP routes were required (enablement is a process-launch decision,
not workspace state), so no client methods were added. Contract DTOs added
and regenerated: `McpServerInfo`, `McpServerToolDef`, `McpServerConfig`
(the config DTO never carries the token). Client type re-exports added for
the three DTOs.

## Verification (Linux)

- `npm run typecheck`: 0 errors. (One real tsc failure caught in-slice: the
  e2e's `delete env.ANANSI_MCP_SERVER_TOKEN` on a narrowly inferred object
  type; fixed by typing the env as `NodeJS.ProcessEnv`.)
- `node scripts/contracts.mjs --check`: 0 drift.
- `node scripts/validate-docs.mjs`: PASS (274 files).
- `npm run test:ui`: 293/293, 0 fail (unchanged; this slice adds no UI).
- Vite production build clean; API build clean (one pre-existing warning
  only: the Windows-only `preserved_env_entries` helper is unused on Linux
  builds — untouched, not introduced here).
- `cargo --workspace --lib`: 240 passed, 0 failed (228 + exactly the 12 new
  `mcp_server::tests`: both protocol versions, unsupported/missing version,
  auth refuse-then-accept, tools/list exactly three, all three tools,
  unknown-tool refusal without backend call, malformed/unknown/notification
  handling, ping, invalid-argument matrix, sealed never leaks + get matches
  missing, bad workspace/empty token rejected before reading, and a
  real-backend component round trip on a temp storage root).
- `tests/e2e/mcp-server.spec.ts`: 2/2 PASS (stdio handshake, tools, seeded
  data, auth incl. re-initialize under 2026-07-28; refuses to start without
  a token). The spec reuses the `startSession` harness with an additive
  `storageRoot` field; the harness-seeded `Harbor App` project covers
  `projects.search`.
- `tests/e2e/mcp.spec.ts`: 6/6 PASS (harness change regression-checked).
- Full regression battery: 54/54 PASS (sequential, real spec filenames, full
  summaries).
- Credentials sweep over the diff vs baseline: clean.

## Known limits (stated, not hidden)

- Sealed behaviour cannot be covered by the Playwright e2e: sealed memories
  cannot be created through any public API (`memories/create` validation
  refuses sealed drafts), so no harness path manufactures a sealed row.
  Sealed coverage is Rust-side (fake-backend layer-3 tests + inherited
  storage guarantees). No test hand-writes storage files to fake one.
- The server process serves one client for its lifetime; no sessions,
  polling, or persistent connections.
- Deferred: `notes.search`, `products.get`, `experiments.list`,
  `ideas.create` (write tool — needs its own approval story), resources /
  prompts, Streamable HTTP, OAuth, multi-token / per-tool grants.
- AN-017 stays `in progress`: the server side now exists and is tested, but
  the remaining tool surface and remote-server auth are still open.
