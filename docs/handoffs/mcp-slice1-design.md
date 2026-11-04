# MCP Client Integration — Slice 1 Design (Milestone 13, MASTER_SPEC §68)

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

## Specification verification (§68: "after verifying its specification")

The MCP specification has two live revisions that matter here:

- **2025-11-25** (deployed everywhere): `initialize` +
  `notifications/initialized` handshake, `tools/list`,
  `prompts/list`, `ping`, `MCP-Protocol-Version` header on HTTP.
- **2026-07-28** (current stable per the official changelog):
  stateless — the `initialize` handshake and `ping` are REMOVED,
  every request carries its protocol version and client
  capabilities in `_meta`, servers MUST implement
  `server/discover`, and list results carry cache hints.

Slice 1 negotiates both, honestly: the probe sends `initialize`
(`2025-11-25`) first, since that is what deployed servers speak.
On a JSON-RPC method-not-found error it falls back to
`server/discover` (`2026-07-28`) and then lists with `_meta`.
Health is reported with the method actually used (`ping` for
legacy servers, `discover` for stateless ones) — a ping is never
fabricated for a server whose revision removed it. The server's
returned protocol version is recorded verbatim.

## Scope (client side only)

- Configuration model + registry UI: name, transport (`stdio` |
  `sse`), command + args + environment (stdio), URL (SSE),
  capabilities (as reported by the server, never invented).
- Tool discovery + capability listing: tools (name, description,
  input schema, annotations) and prompts (name, description,
  arguments) from a live probe.
- Navigation: MCP promoted from planned tools to a live SYSTEM
  view.
- OUT OF SCOPE, explicitly: `tools/call` (no tool is ever run by
  this slice), resources, the MCP server side (§68's "expose
  approved ANANSI knowledge tools"), authentication/OAuth for
  remote servers, persistent connections, background polling.

## Transports: stdio yes, SSE refused (documented, not silent)

- **stdio**: fully implemented. The probe spawns exactly the
  configured command with exactly the configured args. No shell
  is ever involved (`Command::new(command).args(args)` — shell
  metacharacters are inert). The child environment is CLEARED
  and rebuilt from the configured env vars only: the API server
  process environment (which may hold proxy credentials) is never
  inherited. Stderr is discarded (never read) so a chatty server
  cannot block the probe. Total probe timeout 30s; the child is
  killed on timeout or error.
- **SSE / Streamable HTTP**: present in the configuration model
  and registry, but a probe is EXPLICITLY REFUSED with the reason
  "Remote MCP servers cannot be probed in this slice: external
  network access is disabled by default (§68)." Rationale: the
  workspace has no POST HTTP client (the in-tree fetcher is
  GET-only text retrieval), and §71 (SSRF, DNS rebinding) plus
  §68 (default external access disabled) require remote access to
  arrive behind explicit per-server authorization in a later
  slice — not smuggled in as an unreviewed transport. This is the
  assignment's "explicit refusal for unauthorized external network
  access", applied to all remote servers in Slice 1.
- No auto-connect, no polling, no probe on view load. A probe
  runs only when the user clicks Connect/Refresh for one server.

## Backend shape

- New Rust module `crates/core/src/mcp.rs`, `impl Core` command
  `mcp_command`, new API group `/v1/mcp/{operation}` with
  `command_path` entries for `servers-list`, `servers-register`,
  `servers-remove`, `servers-probe`.
- Config storage mirrors the provider-config pattern:
  `storage/mcp-servers/<safe-workspace-id>.json` (`safe` = the
  same alphanumeric/`-`/`_` sanitization), link rejection on dir
  and file, atomic persist (create-new temp + fsync + rename +
  dir sync), private permissions. File shape `{version: 1,
  servers: [...]}`. Env VALUES are never returned to clients:
  list/register responses carry `envKeys` only.
- Validation (Rust, mirrored in TS): name 1..100 chars, no
  control characters; transport enum; stdio requires a non-empty
  command (no NUL/control chars); args are strings (cap 64);
  env is string→string (cap 50 entries, sane key charset);
  SSE requires an `http(s)` URL (parsed, other schemes refused).
  Server ids are `mcp-` + 16 hex chars from time/pid/counter,
  collision-checked against the file.
- Probe protocol (transport abstracted over `BufRead + Write` so
  unit tests run canned transcripts, no process needed):
  sequential JSON-RPC 2.0 requests with incrementing integer
  ids, one NDJSON line per response, 1 MiB line cap, per-request
  timeout inside the 30s total. Lists are capped at 100 tools /
  100 prompts with a `truncated` flag. Tool descriptions and
  schemas are treated as untrusted text: rendered as text, never
  executed, never forwarded to any provider.
- Contracts: new DTOs (`McpServer`, `McpServerDraft`,
  `McpServerList`, `McpTool`, `McpToolAnnotations`, `McpPrompt`,
  `McpPromptArgument`, `McpProbeResult`, `McpRemoveResult`)
  registered in `crates/contracts/src/main.rs`, generated TS +
  validators via `contracts.mjs`. TS client methods
  `listMcpServers`, `registerMcpServer`, `removeMcpServer`,
  `probeMcpServer`; `mcp/` joins the 120s client timeout list
  (probes spawn processes; the backend caps at 30s).

## UI shape (flat files — no new subdirectories)

- `packages/ui/src/mcp.ts`: pure helpers — `validateMcpDraft`
  (error strings), `parseToolSchema` (JSON Schema object →
  properties with name/type/description/required, tolerating
  missing/invalid schemas), `annotationLabels` (readOnly /
  destructive / idempotent / open-world badges).
- `packages/ui/src/Mcp.tsx`: registry (register form: name,
  transport select, command, args lines, env `KEY=value` lines,
  URL; server rows with Connect/Refresh + Remove), probe result
  panel (health, protocol version, health method, server name,
  tools table, prompts list), and honest empty states ("No MCP
  servers configured yet.", "No tools discovered."). Standing
  note, verbatim in spirit: tools are listed, never run, in this
  slice; connecting a server runs only its configured local
  command; remote servers are refused.
- `Navigation.tsx`: `View` gains `'mcp'`; SYSTEM gains a live
  "MCP" button; `'MCP'` leaves `plannedGroups` SYSTEM items.
- `Workspace.tsx`: heading `'MCP servers'` + render branch.
- Authorization display per tool: annotation badges when the
  server reports them, plus the standing note that calling any
  tool will require explicit approval (future slice) — no
  per-tool permission is invented.

## Tests

- Rust unit tests in `mcp.rs`: config validation (each refusal),
  file round-trip in a temp dir, probe transcripts (legacy
  initialize flow, stateless discover flow, error flow, malformed
  line flow) over in-memory reader/writer pairs.
- `packages/ui/tests/mcp.test.tsx`: validation cases, schema
  parser cases (valid / missing / malformed / annotations),
  static-markup renders (empty state, server row with env keys,
  tool row with schema fields + annotation badge).
- `tests/e2e/mcp.spec.ts` + `tests/e2e/mcp-fake-server.mjs`
  (plain-JS NDJSON stdio server: initialize, initialized,
  tools/list, prompts/list, ping): navigation to MCP, empty
  state, register + probe a real local stdio server (genuine
  tools/prompts listed), SSE probe refusal, validation error,
  remove.
- Battery: tsc, contracts, docs, `test:ui`, vite build, api
  build, cargo `--lib`, slice spec, full battery (real filenames,
  full summaries), credentials sweep over the diff.

## Known limitations

- SSE servers can be registered but not probed (see above).
- Probe results are live, not persisted: switching views drops
  the listed tools until the next probe (the registry persists;
  discovery does not).
- Tool/prompt lists cap at 100 each with a truncation flag.
