# MCP Client Integration — Slice 1 Addendum (Milestone 13, §68)

Branch `muse/mcp-slice1` from Windows-certified Dashboard Slice 2
review tip `34f39bb7da54e70c3fc25ed9dbf0791cf4bab197`
(`codex/review-dashboard-slice2`). Baseline verified from origin by
hash: origin head equals the assignment tail exactly, parent is
exactly the Slice 2 delivery tip `330ed64e8`, diff is docs-only
(BUILD_STATUS + RECOVERY + 64-line review report), report confirms
the Slice 2 delivery hashes (ZIP `33e2a2c9…`, bundle `2e3a5014…`),
verdict PASS zero findings, docs 265 with report, UI 284/284, Rust
lib 215/215, dashboard spec 6/6, battery 46/46. HOME Command
Dashboard is therefore CLOSED on Windows, 100% feature-complete.
Honest note (same pattern as before): only the review ZIP's
`.sha256` sidecar was uploaded (expected hash `df3cb507…`); the ZIP
itself lives on the owner's Windows path and could not be re-hashed
from here. The content-addressed commit on origin is what this
slice branches from.

## Specification verification (done before implementation)

The MCP specification has two live revisions: 2025-11-25 (deployed
everywhere: `initialize` handshake + `ping`) and 2026-07-28
(current stable per the official changelog: stateless, handshake
and `ping` removed, `server/discover` mandatory). The probe
negotiates both: `initialize` first, `server/discover` fallback
only on method-not-found, and the health method actually used is
reported (`ping` vs `discover`) — a ping is never fabricated for a
stateless server.

## Scope (client side only)

- Configuration model + registry: name, transport (`stdio` |
  `sse`), command + args + environment (stdio), URL (SSE).
  Stored per workspace in `storage/mcp-servers/<safe-id>.json`,
  hardened like the provider-config pattern (link rejection,
  atomic persist, private permissions). Env VALUES never leave the
  server: API responses carry `envKeys` only.
- Tool discovery + capability listing from a live one-shot probe:
  tools (name, description, input schema, annotation badges) and
  prompts (name, description, arguments). Lists cap at 100 each
  with a truncation flag. Tool descriptions/schemas are untrusted
  text: validated, capped, rendered as text, never executed, never
  forwarded to a provider.
- Navigation: MCP promoted from planned tools to a live SYSTEM
  view (`MCP servers` heading, nav button count 16 → 17).
- OUT OF SCOPE, explicitly: `tools/call` (no tool is ever run),
  resources, the MCP server side, remote-server authentication,
  persistent connections, background polling.

## Transports: stdio yes, SSE refused (documented, not silent)

- stdio: fully implemented. No shell is ever involved; the child
  environment is CLEARED and rebuilt from the configured env vars
  only (the API process environment, which may hold proxy
  credentials, is never inherited); stderr is discarded; 30s total
  probe budget; the child is killed on timeout or error.
- SSE / Streamable HTTP: registrable and listable, but a probe is
  EXPLICITLY REFUSED ("Remote MCP servers cannot be probed in this
  slice: external network access is disabled by default (§68)").
  The workspace has no POST HTTP client (the in-tree fetcher is
  GET-only), and §71 (SSRF, DNS rebinding) requires remote access
  to arrive behind explicit per-server authorization in a later
  slice.
- No auto-connect, no polling, no probe on view load: a probe runs
  only when the user clicks Connect/Refresh for one server.

## Backend shape

- New `crates/core/src/mcp.rs` (`impl Core::mcp_command`), new API
  group `/v1/mcp/{operation}` (`servers-list`, `servers-register`,
  `servers-remove`, `servers-probe`) with `command_path` entries.
- Contracts: 13 new DTOs registered in
  `crates/contracts/src/main.rs`, generated TS + validators via
  `contracts.mjs`; TS client methods `listMcpServers`,
  `registerMcpServer`, `removeMcpServer`, `probeMcpServer`.
- Two in-slice fixes, both disclosed: (1) the TS `httpClient`
  path map routed unknown groups to `workspaces/` — `mcp/` was
  missing from the direct-path list, so browser calls 404'd; fixed
  by adding `mcp/` to the direct-path list (the timeout list
  already had it). The first failing e2e run's "Local core
  unavailable" message was this mis-routing surfacing through a
  stale binary/dist combination; after the fix plus a rebuild,
  registration and probing pass end to end. (2) The client
  `decode` requires every response to carry `contractVersion`,
  so `McpServer` and `McpRemoveResult` gained a
  `contractVersion: 1` field.
- Duplicate identical interfaces in `generated.ts` (e.g. two
  `McpServer` blocks) are the generator's normal behavior (Job
  appears 4×); TypeScript merges them.

## Tests

- Rust unit tests (5, in `mcp.rs`): legacy probe transcript
  (initialize → tools → prompts → ping), stateless fallback
  transcript (initialize -32601 → discover → lists, no ping),
  non-not-found initialize error as the verdict, malformed
  response handling, draft validation refusals. The probe is
  generic over a channel trait, so transcripts run without
  spawning a process.
- `packages/ui/tests/mcp.test.tsx` (5 tests): draft validation
  mirroring, env/args line parsing, tool schema parser (valid /
  missing / malformed), annotation labels, static-markup renders
  (empty state, server row with env keys, tool row with schema
  fields + badge, SSE refusal text).
- `tests/e2e/mcp.spec.ts` (4 tests) + `mcp-fake-server.mjs`
  (plain-JS NDJSON stdio server): navigation + empty state,
  register + probe a REAL local stdio server (genuine tools and
  prompts listed, annotation badges shown, then remove), SSE
  probe refusal, form validation. In-slice e2e fixes: the
  Workspace h1 and the section h2 share the text "MCP servers"
  (asserted the unique register heading instead), and
  `getByText` substring matching collided on 'search'/'summarize'
  (used `exact: true`) and on the refusal sentence (scoped to the
  probe-only sentence).

## Verification (Linux, assignment battery)

- `tsc --noEmit`: 0 errors.
- `node scripts/contracts.mjs --check`: 0 drift.
- `node scripts/validate-docs.mjs`: PASS (268 files).
- `npm run test:ui`: 289/289 (284 + 5 new).
- Production web build clean.
- `node scripts/cargo.mjs build --locked -p anansi-api`: clean.
- `cargo --lib` (per the assignment's battery list): 220 passed, 0 failed
  (215 + 5 new MCP unit tests).
- `mcp.spec.ts`: 4/4 PASS.
- Full regression battery: 50/50 (run sequentially on an idle machine,
  with REAL spec filenames and FULL summaries per the Slice 3 lesson).
- Credentials sweep over the diff vs baseline: clean.

## Known limitations

- SSE servers can be registered but not probed (see above).
- Probe results are live, not persisted: switching views drops
  the listed tools until the next probe (the registry persists;
  discovery does not).
- Tool/prompt lists cap at 100 each with a truncation flag.
- AN-017 stays `in progress`: Slice 1 covers client registry and
  listing only. Tool invocation, resources, the MCP server side,
  and remote-server authentication remain for later slices.

## Next

- Windows rerun of this slice (checklist in the delivery MANIFEST).

Last verified commit: tip recorded in the delivery MANIFEST.
