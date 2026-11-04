# MCP Slice 4 Design — Client Resources & Prompts + Extended Server Tools

Milestone 13, Slice 4. Baseline: Windows-certified Slice 3 review tip
`1b5582097` (branch `codex/review-mcp-slice3`, also origin master at
verification time). Feature branch: `muse/mcp-slice4`. This document is
committed before implementation code, per the owner's design-before-code
rule. The Slice 3 repair addendum's decisions carry over unchanged
(strict sealed-AND-private boundary, pairing-token server auth,
`anansi-server` identity, both protocol versions).

## 1. Extended server tools (`crates/core/src/mcp_server.rs`)

`tools/list` grows from three tools to six, in this order:
`memory.search`, `memory.get`, `notes.search`, `projects.search`,
`experiments.list`, `products.get`. The allowlist is enforced before any
backend call, as in Slice 3; `ideas.create` and everything else remain
refused with no backend contact.

### `notes.search` — `{query, limit?}`

Memory search with kind fixed to `note`: same query validation (1..500
chars, trimmed, non-empty) and limit validation (integer 1..50, default
10) as `memory.search`, same title-search backend path with
`kind: "note"`, same shareable filter (standard/public only), same
no-bodies hit shape `{id, title, snippet, kind, status}`. No new backend
method: it reuses `search_memories` with `kind = Some("note")`.

### `experiments.list` — `{projectId, limit?}`

"LEXI-COLLIDER collision experiment summaries for a given project"
means, grounded in code: the collider's save path
(`client.saveProjectCollider` → `ai/collider-save`) registers
`artifact_type: "experiment"` (`crates/core/src/collider.rs`). So this
tool lists artifacts of type exactly `"experiment"` for the project.
SwarmTank `scenario` artifacts are NOT experiments and are excluded.

### `products.get` — `{projectId, limit?}`

There is no product entity in ANANSI (no product kind, no product
table), so this tool is honestly derived: it returns summary metadata
of the project's Product Forge output artifacts. The Forge set is
`specification`, `source-package`, `deliverable`, `source-review` —
the four types Product Forge registers for a project (M7). The design
judgment is stated here, not hidden: a narrower set
(`source-package` + `deliverable` only) would hide the product's
definition and review state, so all four Forge output types are
included and the set is documented in the tool description.

### Shared project gating (both new project tools)

1. `projectId` must be a 32-character lowercase hex id, else `-32602`.
2. The project is read through the existing `read_memory` backend path
   and must be kind `project`, status active (not superseded, not
   archived — expired projects are already excluded by the list path's
   defaults, and a directly-read expired project is treated as not
   found), and privacy standard/public. Any failure of these checks
   returns the same isError result `Project not found.` — no oracle
   distinguishing unknown, inactive, and private projects.
3. Artifacts come from a new backend trait method
   `project_artifacts(workspace_id, project_id)` returning
   `{artifactId, type, title, version, timestamp, privacy}` summaries
   parsed from the validated `artifacts/list` response (the existing
   `project_artifact_types` returns type strings only, which cannot
   render summaries). Sealed AND private artifacts are dropped
   server-side; only standard/public artifacts are summarized. Bodies
   are never read or returned.
4. Result shape: `structuredContent` is
   `{projectId, projectTitle, experiments: [...]}` or
   `{projectId, projectTitle, products: [...]}`, with text the JSON of
   the same value. An empty list is a valid (non-error) result.
   Limit validation matches search (integer 1..50, default 10).

## 2. Client resources & prompts (`crates/core/src/mcp.rs`)

Four new operations on the existing `/v1/mcp/{operation}` group,
mirroring the `tools-call` flow exactly: registry lookup (unknown id
→ NotFound) → SSE refused with the standard message (no POST HTTP
client in-tree; §68 default-deny + §71) → fresh `StdioChannel::spawn`
(Slice 1 Windows boot env, 30s budget, child teardown) → shared
`handshake` helper → method call with `stateless_meta` for stateless
servers → parse with the shared `parse_list` cap (100 + `truncated`
flag) for lists.

- `resources-list` → `resources/list` → `McpResourceList`
  (`{contractVersion, serverId, resources, truncated}`).
  `McpResource = {uri, name, description?, mimeType?}`; a resource
  without a non-empty uri or name is dropped by the parser.
- `resources-read` → `resources/read {uri}` → `McpResourceReadResult`
  (`{contractVersion, serverId, uri, contents}`). URI validated:
  1..2048 chars, no control characters, else InvalidRequest.
  `McpResourceContent = {uri, mimeType?, text?}`: text parts map to
  text; blob parts map to `text: null` (binary content is never
  decoded or displayed — the UI says so). No URI is ever fetched by
  ANANSI itself: the MCP server returns the content over stdio, so
  there is no HTTP/SSRF surface in this operation.
- `prompts-list` → `prompts/list` → `McpPromptList` (reuses the
  existing `McpPrompt` DTO and `parse_prompt`).
- `prompts-get` → fresh `prompts/list` first (mirroring tools-call's
  fresh `tools/list`: never send to an unoffered prompt), find by
  name, validate required arguments present and every value a string,
  then `prompts/get {name, arguments}` → `McpPromptGetResult`
  (`{contractVersion, serverId, name, description?, messages}`).
  `McpPromptMessage = {role, content: [{contentType, text?}]}`: text
  parts map to text; image/resource parts map to `text: null` with
  their kind preserved, and the UI renders a placeholder.

### Explicit opt-in and review-only handling (the assignment's core rule)

- Nothing in this slice fetches resources or prompt content
  automatically. The Slice 1 probe still discovers only names
  (tools + prompt definitions); content is fetched only when the user
  clicks List/Read/Get for a specific server, resource, or prompt.
- Fetched content is rendered as text for review only. It is never
  executed, never forwarded to Ask ANANSI or any other tool, and no
  inclusion path into a prompt exists in this slice — a user who wants
  content in a prompt must copy it themselves. The UI carries a
  standing note to that effect. This is what "reviewed before
  inclusion in context" means here: display first, no automatic
  inclusion, ever.
- Resource and prompt content is untrusted server text, like tool
  text: rendered as text, never interpreted as instructions.

### Contracts and client methods

New DTOs registered in `crates/contracts/src/main.rs` and regenerated
(`node scripts/contracts.mjs`): `McpResource`, `McpResourceList`,
`McpResourceContent`, `McpResourceReadResult`, `McpPromptList`,
`McpPromptMessage`, `McpPromptMessageContent`, `McpPromptGetResult`,
plus request DTOs `McpResourcesListRequest`, `McpResourceReadRequest`,
`McpPromptsListRequest`, `McpPromptGetRequest`. New `client.ts`
methods: `listMcpResources`, `readMcpResource`, `listMcpPrompts`,
`getMcpPrompt`. New axum `command_path` entries for the four
operations. `McpPrompt` itself is reused unchanged.

## 3. UI (`packages/ui/src/Mcp.tsx`, `packages/ui/src/mcp.ts`)

- Per-server **Resources** section (inside the probe panel, like the
  tools section): "List resources" button → list → per-resource
  "Read" button → `ResourceViewer` renders uri, mimeType, and text
  (or "Binary content is not shown." when text is null). Errors render
  inline per server; failures also set the page notice.
- Per-server **Prompt templates** section: "List prompts" button →
  list → per-prompt argument form (from the prompt's declared
  arguments; required marked) → "Get prompt" button →
  `PromptViewer` renders description and messages (role + text, or
  "(non-text content not shown.)" for null text).
- `ResourceViewer` and `PromptViewer` are presentational components
  taking data only, so they are statically testable with canned data.
- `McpView` gains the new sections through OPTIONAL props (resource
  lists/reads/errors, prompt lists/results/errors, action callbacks,
  busy action key) so existing static tests compile unchanged.
- Pure helpers in `mcp.ts`, unit-tested: `validatePromptArguments`
  (missing required → error string; non-string value → error string;
  else null) and `assemblePromptArguments` (drops empty strings, keeps
  strings as-is — prompt arguments are strings, unlike tool arguments
  which parse typed fields).
- The container `Mcp` holds the new state and calls the new client
  methods. The standing review-only note is rendered above both
  sections. SSE servers keep their buttons; the backend refusal
  surfaces as an inline error, exactly like a refused tool call.

## 4. Fake server and E2E

`tests/e2e/mcp-fake-server.mjs` gains: `resources/list` (two text
resources), `resources/read` (content by uri, `-32602` for unknown
uri), `prompts/get` (message with the supplied argument substituted,
`-32602` for unknown prompt or missing required argument), and a
second prompt with an optional argument. `prompts/list` already exists
and gains the second prompt.

New `tests/e2e/mcp-slice4.spec.ts` (battery grows 54 → 56):

1. **Client flows**: register the fake server, list resources, read
   one and assert its text is shown for review; list prompts, get one
   with its required argument and assert the rendered message contains
   the argument; get with a missing required argument is refused.
2. **Server extended tools**: spawn the real `anansi-api --mcp-server`
   with the harness pairing root/token; seed a standard note, a
   private note, a standard project, a private project, an
   `experiment` artifact and a `deliverable` artifact (standard
   privacy) plus a private `experiment` artifact via
   `artifacts/register`; assert `notes.search` finds the standard note
   and not the private one, `experiments.list` returns the standard
   experiment only, `products.get` returns the deliverable only,
   invalid project id → `-32602`, unknown and private project →
   isError `Project not found.`, and `tools/list` now returns six
   tools in the documented order. Sealed rows cannot be seeded through
   any public API, so sealed exclusion stays Rust-covered (as in
   Slice 3); private exclusion is proven end to end here.

## 5. Test matrix

- Rust `mcp_server::tests`: new tools happy path; privacy filtering
  (sealed + private artifacts dropped, private project → not-found);
  invalid project id; unknown/inactive/wrong-kind project; limit
  validation; `tools/list` returns exactly six in order; unknown tool
  still refused without backend contact; real-backend component test
  (temp core: project + experiment + deliverable artifacts via
  `artifacts_command`, asserting summaries and privacy filtering).
- Rust `mcp::tests`: resources/prompts happy paths over canned
  transcripts; list truncation at 100; blob → null text; non-text
  prompt content → null text; SSE refusal for all four ops; invalid
  URI; unknown prompt; missing required prompt argument; non-string
  prompt argument.
- UI `mcp.test.tsx`: `validatePromptArguments` and
  `assemblePromptArguments` cases; `ResourceViewer` and `PromptViewer`
  static markup (text, binary placeholder, non-text placeholder);
  `McpView` renders the Resources and Prompts sections with canned
  data.
- E2E: `mcp-slice4.spec.ts` as above; `mcp.spec.ts` and
  `mcp-server.spec.ts` re-run as harness regression checks.
- Gates: typecheck 0, contracts 0 drift, docs PASS, `test:ui`,
  Vite build, API build, `cargo --workspace --lib`, full battery.

## 6. Out of scope (explicit)

- No Ask integration for resource/prompt content (no inclusion path).
- No resources/templates subscription, no `resources/subscribe`,
  no roots, no sampling.
- No SSE/Streamable HTTP for the new client ops (same refusal as
  tools-call).
- No `ideas.create` (a write tool — needs its own approval story).
- No change to Slice 3 tools, auth, or privacy posture.
