# MCP Slice 3 Design — MCP Server Core (Knowledge Exposure & Controlled Tools)

Milestone 13, MASTER_SPEC §68. Baseline: Windows-certified Slice 2 review tip
`a3c517267d58761a63e9d50795a406f0d02b5810` on `codex/review-mcp-slice2`
(Slice 2 CLOSED on Windows; AN-017 stays `in progress`).
Branch: `muse/mcp-slice3`.

## 1. Scope: exactly three read-only tools

§68 lists possible server tools (`memory.search`, `memory.get`, `notes.search`,
`projects.search`, `ideas.create`, `products.get`, `experiments.list`). This slice
exposes exactly the three the assignment names:

- `memory.search` — title search over workspace memories.
- `memory.get` — one non-sealed memory by ID, metadata + body.
- `projects.search` — active project titles and summaries.

Deferred, explicitly, not silently: `notes.search` (a kind-filtered subset of
`memory.search`, trivial later), `products.get` and `experiments.list` (need their
own backend mappings), and `ideas.create` — a WRITE tool, which needs its own
approval story and is never smuggled into a read-only slice.

## 2. Architecture

```
external MCP client --(NDJSON stdio)--> anansi-api --mcp-server
    --> crates/core/src/mcp_server.rs serve() loop
    --> McpServerBackend trait
    --> CoreMcpServerBackend (Core::memory_command, the same validated
        path the HTTP API uses)          [production]
    --> FakeBackend                      [unit tests]
```

- `mcp_server.rs` is transport-agnostic: `serve(reader: impl BufRead,
  writer: impl Write, backend, config)`. Unit tests drive it over in-memory
  buffers; the binary wires it to locked stdin/stdout. No new HTTP route, no
  new server process: the runner is a mode of the existing `anansi-api` binary
  (see §5), so there is exactly one artifact to spawn and audit.
- The engine keeps no per-client state beyond authentication state and the
  negotiated protocol version. No sessions, no polling, no persistent
  connections: one stdio process serves one client for its lifetime.
- Line cap 1 MiB, mirroring the Slice 1 client channel. An overlong line gets
  a `-32700` parse error with null id and the loop continues.

## 3. Protocol

JSON-RPC 2.0, one message per line, NDJSON over stdio. Methods:

| Method | Behaviour |
|---|---|
| `initialize` | Auth + version negotiation (see §4). Responds with the negotiated `protocolVersion`, `capabilities: { tools: {} }`, `serverInfo: { name: "ANANSI", version: <crate version> }`. |
| `notifications/initialized` and any other notification (no `id`) | Ignored, no response. |
| `tools/list` | Returns exactly the three tool definitions (name, description, inputSchema). No pagination: three tools need no cursor. |
| `tools/call` | Dispatches to the allowlist; anything else is refused before any backend call. |
| `ping` | Returns `{}`. |
| anything else | `-32601` method not found. |

Error codes: `-32700` parse error (malformed JSON, null id), `-32601` unknown
method, `-32602` invalid params (bad shape, unknown tool name, bad id, bad
limit, unsupported protocol version), `-32001` unauthorized (implementation-
defined server-error range; message carries no token material), `-32603`
internal error with a generic message (storage failures never leak internals).

Version negotiation: the client must send `protocolVersion` `2025-11-25` or
`2026-07-28` (the two revisions the Slice 1 client negotiates). Anything else,
including a missing version, is `-32602`. The server echoes the accepted
version; it does not upgrade or downgrade the client silently. Behaviour is
identical across both versions in this slice (both use `tools/list` +
`tools/call`; stateless `_meta` handling is accepted but not required).

`tools/call` params: `{ name, arguments }`. `arguments` must be a JSON object.
Per-tool validation, enforced before dispatch:

- `memory.search`: `query` required, string, 1..=500 chars (trimmed; empty
  after trim is invalid). `limit` optional, integer, 1..=50, default 10.
- `memory.get`: `id` required, string, must pass the core id validation
  (the same `validate_id` the memory commands use). Unknown extra fields are
  ignored, not rejected — tool arguments are caller-shaped, and strict
  `deny_unknown_fields` would break forward-compatible clients.
- `projects.search`: same as `memory.search`.

Result shape: one text content part whose text is the JSON rendering of the
result, plus `structuredContent` carrying the same value (MCP 2025-11-25 and
later). Tool-level failures that are about the data (unknown id, sealed id)
are `isError: true` results, not JSON-RPC errors — the call succeeded, the
lookup did not. Transport/validation failures are JSON-RPC errors.

## 4. Authentication and per-client permissions

§68 requires authentication and per-client permissions. Stdio has no
transport auth standard, so this slice is explicit about what the gates are
and what they are not:

1. **Disabled by default.** The server runs only when the operator launches
   `anansi-api --mcp-server --workspace-id <id>` AND sets
   `ANANSI_MCP_SERVER_TOKEN`. No flag → the binary stays an HTTP server. Flag
   without a token → the process refuses to start (stderr message, non-zero
   exit, no token material printed). Enablement is a process-launch decision,
   not workspace state, so no management HTTP route can flip it remotely.
2. **Token gate.** The client must present the token in `initialize` params
   as `_meta.anansiMcpToken` (an ANANSI extension on MCP's sanctioned `_meta`
   extension point; it is not part of the MCP spec and is documented as such).
   Comparison folds bytes without early exit on first difference. Until a
   valid `initialize` succeeds, every request except `initialize` is refused
   with `-32001`; a failed `initialize` does not kill the process, so a
   client can retry with the right token. The token is never logged, never
   echoed in any response or error, and is accepted only from the environment
   — never from a CLI argument (which would expose it in process listings).
3. **Per-client permissions, this slice's granularity: one token, one fixed
   permission set** — exactly the three approved read-only tools. The
   allowlist is enforced in dispatch, before any backend call, so an unknown
   tool name can never reach storage. Finer grants (multiple tokens with
   per-tool permission sets) are deferred and stated as such; this slice does
   not pretend a single token is per-client granularity.

## 5. Runner (crates/api/src/main.rs)

New arguments, parsed alongside the existing `--port` / `--pairing-root`:

- `--mcp-server` — run the stdio MCP server instead of the HTTP server.
- `--workspace-id <id>` — required with `--mcp-server`; validated with the
  core id rules; unknown or empty workspaces simply yield empty results (the
  same as an empty workspace — no existence oracle, no data leak across
  workspace ids, since every query is scoped to the given id).

Storage root resolution mirrors `Core::for_host` (`ANANSI_STORAGE_ROOT` env,
else the host default), but the server constructs `Core::with_storage_root`
directly: `for_host` also starts the export worker and backup scheduler, and
a read-only server must not run background writers. The server process is
read-only by construction: its only backend operations are `list` and `read`.

## 6. Production backend (CoreMcpServerBackend)

Built on `Core::memory_command` — the same validated entry the HTTP API uses
— so the server inherits id validation, query validation, and the storage
layer's sealed handling instead of reimplementing them:

- `memory.search` → `memory_command("list", { workspaceId, query, limit })`.
  This is the core's **title** search path (`title_search`), not body search.
  The split is deliberate and documented: search discovers titles and
  snippets; `memory.get` retrieves the body. Search results never carry bodies.
  Returned per hit: `{ id, title, snippet, kind, status }`.
- `memory.get` → `memory_command("read", { workspaceId, id })`. Returned:
  `{ id, title, snippet, kind, status, body }`.
- `projects.search` → `memory_command("list", { workspaceId, kind: "project",
  query, limit })`, then the server drops any hit whose status is
  `Superseded` or `Archived` (the list defaults already exclude archived and
  expired rows; superseded is the additional server-side filter). "Active"
  therefore means: not superseded, not archived, not expired. Returned per
  hit: `{ id, title, snippet, status }`.

## 7. Sealed-memory boundary (three layers, all enforced)

1. **Storage layer (existing, inherited):** list SQL excludes
   `privacy = 'sealed'` rows, and `allow_plaintext` refuses sealed reads.
2. **Backend layer:** a sealed `read` errors in core; the backend maps ANY
   read failure for a well-formed id to the same outcome (see layer 3) rather
   than distinguishing error kinds to the caller.
3. **Server layer (new, defense in depth):** the server drops any search hit
   whose `privacy` is `sealed`, and refuses any `get` result whose `privacy`
   is `sealed`, even if a backend ever returned one. This layer is unit-tested
   with a fake backend that deliberately returns sealed rows.

`memory.get` on a missing id and on a sealed id produce the **identical**
`isError: true` result with text `Memory not found.` — the server must not
become an existence oracle for sealed records. Search can never reveal a
sealed record's title, snippet, or existence, because sealed rows never reach
the result list at any layer.

Note on testability, stated not hidden: sealed memories cannot be created
through any public API — `memories/create` validation calls
`allow_plaintext` and refuses sealed drafts — so the Playwright e2e cannot
manufacture a sealed row through the harness. Sealed behaviour is therefore
covered in Rust (fake-backend layer-3 tests + the inherited storage-layer
guarantees), and the e2e covers the real binary with plaintext seeded data.
No test fakes a sealed row by hand-writing storage files.

## 8. Contracts

No new HTTP routes are required (enablement is a launch decision, §4.1), so
no client methods are added. New contract DTOs, registered in
`crates/contracts/src/main.rs` and regenerated via `node scripts/contracts.mjs`:

- `McpServerInfo` — `{ contractVersion, name, version, protocolVersions,
  tools: string[] }` (the capability surface a client can expect).
- `McpServerToolDef` — `{ name, description, inputSchema }`.
- `McpServerConfig` — `{ contractVersion, enabled, workspaceId, tools }`.
  It never carries the token.

`contracts.mjs --check` must report 0 drift.

## 9. Test strategy

- **Rust unit tests** (`mcp_server.rs`, fake backend): handshake for both
  protocol versions; unsupported/missing version rejected; auth failure then
  success (retry works); requests before auth refused; `tools/list` returns
  exactly the three tools; each tool's happy path; unknown tool refused with
  no backend call; malformed JSON → `-32700`; missing/invalid params →
  `-32602`; overlong query rejected; notifications produce no output; `ping`
  returns `{}`; sealed search hits dropped; sealed get → `Memory not found.`
  (identical to missing); unknown method → `-32601`.
- **Rust component test** (real `CoreMcpServerBackend`, `Core::with_storage_root`
  on a temp dir): create plaintext memories and a project through
  `Core::memory_command`, then drive the server loop over buffers and assert
  search/get/projects results end to end. This test owns a temp storage root
  directly — no environment manipulation, so it cannot disturb parallel tests.
- **Playwright e2e** (`tests/e2e/mcp-server.spec.ts`): reuse the
  `startSession` harness (additive `storageRoot` field on `Session`) to get a
  real workspace + seeded `Harbor App` project, create note memories through
  `session.call`, then spawn the real `target/debug/anansi-api --mcp-server`
  binary over stdio with the harness storage root and a test token, and assert:
  initialize with token (both protocol versions, in separate runs), tools/list
  exactly three tools, seeded memory.search, seeded memory.get (body
  returned), projects.search finds `Harbor App`, auth failure then success,
  malformed line → `-32700`, unknown tool → `-32602`, unknown id → isError
  `Memory not found.`. The spec kills the server child on cleanup. Sealed
  behaviour is Rust-covered per §7's note.
- **Static gates:** `npm run typecheck` (tsc, 0 errors),
  `node scripts/contracts.mjs --check` (0 drift),
  `node scripts/validate-docs.mjs` (PASS), `npm run test:ui`, Vite build, API
  build, `cargo --workspace --lib`, `mcp.spec.ts` + `mcp-server.spec.ts`,
  full regression battery under the shared lease.

## 10. Out of scope (stated, not deferred silently)

`notes.search`, `products.get`, `experiments.list`, `ideas.create` (write tool
— needs its own approval story), MCP resources/prompts, Streamable HTTP
transport, OAuth, multi-token/per-tool grants, persistent connections,
polling, auto-start, paid inference, any HTTP management route for server
enablement.
