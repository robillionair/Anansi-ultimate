# MCP Client Integration — Slice 2 Design: Tool Invocation & Explicit Approval

Branch `muse/mcp-slice2` from Windows-certified Slice 1 repair
review tip `84dc00b95e28e5c6b9e694c16c07eeae5a89fd81`
(`codex/review-mcp-slice1-repair`). Baseline verified from origin
by hash: origin head equals the assignment tail exactly, parent
is exactly the Slice 1 repair delivery tip `6ddef0995`, diff is
docs-only (BUILD_STATUS + RECOVERY + REQUIREMENTS_MATRIX +
60-line review report), report confirms the repair delivery
hashes (ZIP `cf4c1dca…`, bundle `4321d205…`) and certifies
Slice 1 CLOSED on Windows (decisive `mcp.spec.ts` test 2 passed
in 7.8s against a genuine Node MCP server; battery 50/50).
Honest note (same pattern as before): only the review ZIP's
`.sha256` sidecar was uploaded (expected hash `23f9eed7…`); the
ZIP itself lives on the owner's Windows path and could not be
re-hashed from here. The content-addressed commit on origin is
what this slice branches from.

## Scope (MASTER_SPEC §68, Milestone 13)

Slice 1 built registry + listing. Slice 2 adds exactly one new
capability: invoking a discovered tool on a registered local
stdio server, only after explicit per-call user approval.
Still out of scope: resources, the MCP server side, remote
(SSE) invocation, persistent connections, polling, auto-
invocation of any kind, paid inference.

## Backend: new `tools-call` operation

- API group `/v1/mcp/tools-call` (new `command_path` entry
  alongside the Slice 1 operations), core operation
  `tools-call` in `crates/core/src/mcp.rs`, request DTO
  `McpToolCallRequest { serverId, toolName, arguments }`.
- Flow, reusing the Slice 1 machinery exactly:
  1. Load the server from the workspace registry; unknown id
     → NotFound. SSE transport → the same refusal as probing,
     reworded for calls: remote servers cannot be called,
     external network access disabled by default (§68).
  2. `StdioChannel::spawn` — so the Windows boot environment
     restoration (SystemRoot/WINDIR/PATH) and the 30s whole-call
     budget from Slice 1 apply unchanged, and `Drop` kills the
     child on every exit path.
  3. Handshake: the probe's `initialize`-first /
     `server/discover`-on-`-32601` negotiation is factored out
     of `probe_channel` into a shared `handshake` helper with
     identical error strings, used by both probe and call.
     (Refactor, no behavior change to probing.)
  4. `tools/list` on the same channel; the tool must be
     offered by name, otherwise the call is refused ("Tool
     '<name>' was not offered by this server."). The schema
     used for validation is the one the server just offered —
     never one supplied by the client.
  5. Arguments validated against that schema (see below);
     validation failure refuses the call before anything is
     sent to the tool.
  6. `tools/call { name, arguments }`; the result is decoded
     into `McpToolCallResult` (see below). A JSON-RPC error
     from the tool is a transport error, not a result.
- Result decoding: `content` array kept as typed parts
  (`{ type, text? }`; non-text parts keep their type, text
  omitted — never rendered as anything but text), `text` =
  joined text parts, `isError` = the server's `isError` flag,
  `structuredContent` passed through verbatim as JSON for
  display only. A tool-level `isError: true` is a completed
  invocation with status Error, not a transport failure.
- Transport/validation failures return `CommandError` (the UI
  shows an error notice and records no history entry — nothing
  was invoked). Only completed dispatches enter history.

## Argument validation: conservative JSON Schema subset

Backend `validate_tool_arguments(schema, arguments)`, mirrored
in TS (`validateToolArguments`) so the form and the server
agree. Enforced, and only this:
- `arguments` must be a JSON object when the schema declares
  `type: 'object'` or carries `properties`/`required`.
- Every `required` property must be present (presence, not
  non-emptiness — exactly JSON Schema semantics).
- Present properties are type-checked: `string`, `integer`
  (no fraction), `number`, `boolean`, `array`, `object`.
- `enum` membership is checked when an `enum` array is present.
- Not enforced (documented): `additionalProperties`,
  `minLength`/`maxLength`, numeric bounds, nested schemas,
  `format`. A tool with no usable schema is called
  unvalidated — the UI says so ("No usable input schema")
  rather than pretending otherwise.

## UI: form, authorization dialog, result, history

In `packages/ui/src/Mcp.tsx` (presentational `McpView` +
container `Mcp`), with pure helpers in `mcp.ts`:
- Each discovered tool gains a "Call tool" button. The call
  form is built from the parsed schema fields: string → text
  input, number/integer → number input, boolean → checkbox,
  enum → select, anything else → textarea parsed as JSON.
  Empty optional fields are omitted from the arguments;
  invalid JSON or a failed client-side validation blocks the
  review step with a visible error.
- "Review and authorize" opens the mandatory authorization
  dialog: server name, tool name, annotation badges, the
  EXACT parameters as JSON, and a warning banner when
  `destructiveHint` is set. Two actions only: "Authorize and
  Call Tool" and "Cancel". No other path dispatches a call —
  zero auto-invocations, no call on form submit, no call on
  dialog open. Cancel dispatches nothing and records nothing.
- The result renders directly below the tool card: status
  `Success` or `Error` (tool-level `isError`), the text, and
  any structured content as formatted JSON. Server text is
  rendered as text, never executed, never forwarded.
- Execution history ("Recent tool calls"): timestamp, server
  name, tool name, status — and nothing else. Parameters,
  results, environment values, and keys are never logged.
  History is session-local to the MCP view (in-memory, capped
  at 50, newest first, via pure `recordToolCall`); it is not
  persisted and does not survive a page reload. That limit is
  stated in the UI copy and the addendum, not hidden.
- Copy updates: the Slice 1 standing note ("Tools are listed,
  never run… Calling a tool will require explicit approval in
  a later slice.") becomes present-tense approval copy.

## Contracts and client

New DTOs `McpToolCallRequest`, `McpToolCallContent`,
`McpToolCallResult` (all with `contractVersion: 1` on the
result, per the Slice 1 `decode` lesson), registered in
`crates/contracts/src/main.rs`, generated via
`contracts.mjs`, client method `callMcpTool`. The existing
`mcp/` direct-path routing and 120s fetch timeout already
cover the new operation; no client routing change.

## Tests

- Rust (in `mcp.rs`, via the canned channel): handshake
  reuse (legacy + stateless fallback through the shared
  helper), call success decoding (text join, `isError`,
  structured content passthrough), unknown-tool refusal,
  argument validation (required presence, each type, enum,
  non-object arguments), SSE refusal.
- TS (`mcp.test.tsx`): `validateToolArguments` mirroring,
  `recordToolCall` capping/ordering, static-markup renders
  (call form fields, authorization dialog with destructive
  banner, result block, history entry).
- Fake server (`mcp-fake-server.mjs`): `tools/call` for
  `search` (echoes `Search results for "<q>".`) and `write`
  (echoes `Wrote "<text>".`); unknown tool → JSON-RPC
  `-32602` error.
- E2E (`mcp.spec.ts`, 2 new tests, 6 total): formulate a
  search call, approve in the dialog, verify the returned
  text and a Success history entry; open a write call,
  confirm the destructive banner, Cancel, verify no result
  and no history entry. Existing 4 tests unchanged.

## Safety restated

No shell, child env cleared + Windows system paths only,
30s budget, child killed on every exit path. Tool results
are untrusted server text. No tool is ever invoked without
an explicit "Authorize and Call Tool" click for that exact
call. No parameters, results, env values, or keys enter the
audit history. Zero paid inference, zero remote SSE bypasses.

## Next

- Windows rerun of this slice (checklist in the delivery
  MANIFEST).
