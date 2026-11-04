# MCP Client Integration — Slice 2 Addendum (Milestone 13, §68)

Branch `muse/mcp-slice2` from Windows-certified Slice 1 repair
review tip `84dc00b95e28e5c6b9e694c16c07eeae5a89fd81`
(`codex/review-mcp-slice1-repair`). Baseline verified from
origin by hash: origin head equals the assignment tail exactly,
parent is exactly the Slice 1 repair delivery tip `6ddef0995`,
diff is docs-only (BUILD_STATUS + RECOVERY +
REQUIREMENTS_MATRIX + 60-line review report), report confirms
the repair delivery hashes (ZIP `cf4c1dca…`, bundle
`4321d205…`) and certifies Slice 1 CLOSED on Windows
(decisive `mcp.spec.ts` test 2 passed in 7.8s against a genuine
Node MCP server; battery 50/50). Honest note (same pattern as
before): only the review ZIP's `.sha256` sidecar was uploaded
(expected hash `23f9eed7…`); the ZIP itself lives on the
owner's Windows path and could not be re-hashed from here. The
content-addressed commit on origin is what this slice branches
from. Design committed before code
(`docs/handoffs/mcp-slice2-design.md`).

## Scope: one new capability, explicitly approved per call

Slice 1 built registry + listing. Slice 2 adds invoking a
discovered tool on a registered local stdio server — only
after the user approves that exact call. Still out of scope:
resources, the MCP server side, remote (SSE) invocation,
persistent connections, polling, auto-invocation of any kind,
paid inference.

## Backend: `tools-call`

- New API operation `/v1/mcp/tools-call` (new `command_path`
  entry), core operation `tools-call`, DTOs
  `McpToolCallRequest`, `McpToolCallContent`,
  `McpToolCallResult` (result carries `contractVersion: 1`,
  per the Slice 1 `decode` lesson), client method
  `callMcpTool`. The existing `mcp/` direct-path routing and
  120s fetch timeout already covered the operation.
- Flow: registry lookup (unknown id → NotFound) → SSE
  refused with the Slice 1 wording adapted for calls →
  `StdioChannel::spawn` (Windows boot environment
  restoration, 30s whole-call budget, `Drop` kills the child
  on every exit path) → shared `handshake` helper (the probe's
  `initialize`-first / `server/discover`-on-`-32601`
  negotiation, factored out of `probe_channel` with identical
  error strings; probe behavior unchanged, proven by its
  existing tests) → fresh `tools/list` on the same channel →
  the tool must be offered by name, otherwise refused before
  anything is sent → arguments validated against the schema
  the server just offered (never a client-supplied schema) →
  `tools/call`.
- Argument validation is a conservative JSON Schema subset,
  mirrored in TS: arguments must be an object; every
  `required` property present (presence, exactly JSON Schema
  semantics); present properties type-checked (string,
  integer, number, boolean, array, object); `enum`
  membership checked. Not enforced, by design:
  `additionalProperties`, string/number bounds, nested
  schemas, `format`. A tool with no usable schema is called
  unvalidated, and the UI says so.
- Result decoding: content parts kept typed (`type`, optional
  `text`; non-text parts keep their type and are never
  rendered as anything but text), `text` = joined text parts,
  `isError` = the server's flag, `structuredContent` passed
  through verbatim as JSON for display only. A tool-level
  `isError: true` is a completed invocation with status
  Error; transport/validation failures return `CommandError`
  (the error enum has no execution-failure code, so they
  surface as InvalidRequest carrying the real message) and
  record no history entry — nothing was invoked.

## UI: form, authorization dialog, result, history

- Each discovered tool has a "Call tool" button. The form is
  built from the parsed schema fields: string → text input,
  number/integer → number input, boolean → checkbox,
  enum → select, anything else → textarea parsed as JSON.
  Empty fields are omitted, so a required field left empty
  fails validation as missing; invalid JSON or a failed
  validation blocks the review step with a visible error.
- "Review and authorize" opens the mandatory dialog: server
  name, tool name, annotation badges, the EXACT parameters
  as JSON, and a warning banner when `destructiveHint` is
  set. Two actions only: "Authorize and Call Tool" and
  "Cancel". No other path dispatches a call — not form
  submit, not dialog open, not view load. Cancel dispatches
  nothing and records nothing.
- The result renders directly below the tool card: status
  `Success` or `Error`, the text, non-text part types if
  any, and structured content as formatted JSON. Server text
  is rendered as text, never executed, never forwarded.
- Execution history ("Recent tool calls"): timestamp, server
  name, tool name, status — and nothing else. Parameters,
  results, environment values, and keys are never logged.
  History is session-local (in-memory, capped at 50, newest
  first, via pure `recordToolCall`); it is not persisted and
  does not survive a page reload. That limit is stated in
  the UI copy, not hidden.
- Copy updates: the Slice 1 standing note ("Tools are
  listed, never run…") is now present-tense approval copy,
  and the register section no longer says SSE servers
  "cannot be probed in this slice" — they cannot be probed
  or called.

## Tests

- Rust (5 new, canned channel): call success decoding (text
  join, structured content passthrough, method sequence),
  stateless fallback through the shared handshake helper,
  unknown-tool refusal before dispatch (no `tools/call`
  sent), tool-level `isError` as a completed result,
  argument validation (required presence, each type, enum,
  non-object arguments, no-schema passthrough).
- TS (4 new tests): `validateToolArguments` mirroring,
  `assembleToolArguments` (omission, typed parsing, error
  count), `recordToolCall` (ordering, cap, no parameters
  carried), static markup (Call tool button, result block,
  history entry, history privacy note). The dialog flow
  itself is state-gated and is covered by e2e, not static
  markup — stated, not hidden.
- Fake server: `tools/call` for `search` (echoes `Search
  results for "<q>".`) and `write` (echoes `Wrote
  "<text>".`); unknown tool → JSON-RPC `-32602` error.
- E2E (2 new, 6 total): formulate a search call, approve
  in the dialog (exact JSON verified), verify the returned
  text and a Success history entry; open a write call,
  confirm the destructive banner, Cancel, verify no result
  and no history entry. Existing 4 tests unchanged except
  the navigation test's intro-copy assertion, updated to
  the new copy (disclosed test maintenance).
- In-slice edit incident, disclosed: two test-file edits
  swallowed the following test's header line (the same
  failure mode seen once in the Rust tests during Slice 2
  authoring); both headers were restored and the suites
  re-run green. No production code was affected.

## Verification (Linux)

- `tsc --noEmit`: 0 errors.
- `node scripts/contracts.mjs --check`: 0 drift.
- `node scripts/validate-docs.mjs`: PASS (271 files).
- `npm run test:ui`: 293/293, 0 fail (289 + 4 new).
- Production web build clean; API build clean.
- `cargo test --package anansi-core --lib mcp::tests`: 13/13
  PASS (8 + 5 new).
- `cargo test --workspace --lib`: 228 passed, 0 failed
  (223 + 5 new).
- `mcp.spec.ts`: 6/6 PASS.
- Full regression battery: 52/52 PASS (sequential, real spec
  filenames, full summaries).
- Credentials sweep over the diff vs baseline: clean.

## Next

- Windows rerun of this slice (checklist in the delivery
  MANIFEST). AN-017 stays `in progress`: invocation is done,
  but resources, the MCP server side, and remote-server
  authentication remain for later slices.

Last verified commit: tip recorded in the delivery MANIFEST.
