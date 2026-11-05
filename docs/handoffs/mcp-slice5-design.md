# MCP Slice 5 Design — Server Mutation Gate (`ideas.create`) & Milestone 13 Closure

Milestone 13, Slice 5. Baseline: Windows-certified Slice 4 review tip
`b6c6f39ea` (branch `codex/review-mcp-slice4`, also origin master at
verification time). Feature branch: `muse/mcp-slice5`. This document is
committed before implementation code. Slice 3–4 decisions carry over
unchanged (strict sealed-AND-private boundary, pairing-token server
auth, `anansi-server` identity, both protocol versions).

## 1. What `ideas.create` is — and two assignment-wording corrections

`ideas.create` is the only mutating tool in §68's list, and the
assignment frames this slice as its "mutation gate". Two wordings in
the assignment do not match the tree, and the implementation follows
the tree, as in earlier slices (Slice 2's JobState correction):

1. **`artifacts_command("create", ...)` does not exist.** The
   `artifacts_command` operations are `register`, `revise`, `list`,
   `versions`, `read`, and the evaluations ops. The validated creation
   path is **`register`**, and that is what `ideas.create` calls.
2. **The ideas module is not a manual creation path.** `crates/core/
   src/ideas.rs` is provider-based generation (`ideas-generate-start/
   poll/cancel/save`): `ideas-save` registers a model-generated idea
   SET as a type-`idea` artifact, tied to a generation run and session.
   It cannot create a single caller-supplied idea. Manual creation
   goes through `artifacts_command("register", {type: "idea"})`.

## 2. Tool definition (7th tool, `tools/list` order)

Order: `memory.search`, `memory.get`, `notes.search`,
`projects.search`, `experiments.list`, `products.get`,
`ideas.create`. Input schema:

- `projectId` (string, required): 32-character lowercase hex.
- `title` (string, required): trimmed, 1–120 characters. (The
  register path allows 1–200; the tool enforces the tighter 1–120
  from the assignment.)
- `description` (string, required): trimmed, non-empty. Upper bound
  inherited from register validation (256 KiB body cap).
- `tags` (optional array of strings): at most 32 entries, each
  trimmed 1–64 characters. Any empty, non-string, overlong, or
  over-count tags value is a parameter error.

Tool description (what clients see) states the binding and the tags
rule in full, so no client has to guess.

## 3. The mutation gate — five layers, no new credential

There is no per-tool permission grant store in ANANSI, and this slice
does not invent one. The gate is the composition of existing,
independently enforced layers:

1. **Disabled by default.** The server runs only when the operator
   launches `anansi-api --mcp-server` with a workspace id and a
   pairing root. No flag, no server, no writes.
2. **Live pairing authentication.** `serve()` refuses to start with
   no live pairing session, and every `initialize` re-reads
   `<pairing-root>/session-*/pairing.json` tokens. A write
   operation therefore requires the same live session credential as
   every read: the pairing token is the user's own session, so the
   "client" holding it is the authenticated user, not a stranger.
   No separate server token exists, so no second credential
   inventory is created.
3. **Project gating (server-side, before any write).** Identical to
   the Slice 4 project tools: the project is read through
   `read_memory` and must be kind `project`, status active (not
   superseded, not archived), privacy standard/public, and
   non-expired (unparseable expiry fails closed). Any failure
   returns the same isError result `Project not found.` — no oracle
   distinguishing unknown, inactive, private, expired, and
   wrong-kind projects. The register path's own
   `require_project_writable` (archived rejection) is a second,
   deeper layer, not the gate.
4. **Schema validation before dispatch.** All argument validation in
   §2 runs before any backend call; malformed or missing arguments
   return standard JSON-RPC `-32602`.
5. **Strict project binding.** The artifact is registered with
   `projectId` equal to the validated project's id and
   `type: "idea"` — never any other type, never another project.
   The created artifact's privacy mirrors the validated project's
   privacy (standard or public): the artifact is visible exactly
   where its project is visible, and never private-by-default here
   (register's `default_privacy` is `private`, so privacy is passed
   explicitly).

## 4. Tags — the documented judgment call

The artifact schema has no tags column (neither do memories), and
adding one would be a storage migration — out of scope for this
slice. The alternatives were weighed and rejected:

- **Silently dropping validated tags**: dishonest; rejected.
- **Mapping tags to `source_references`**: wrong semantics, and
  restore re-validates references — a real hazard; rejected.
- **A schema migration for a tags column**: disproportionate and
  unrequested; rejected.

Decision: tags are validated strictly (§2) and persisted as the final
line of the artifact body, `Tags: tag1, tag2`, with the description
stored verbatim before that line. The body therefore always starts
with the exact description, tags are visible and recoverable, and the
tool description states the format. The response echoes the tags as
given, so the caller sees what was stored.

## 5. Backend trait and error mapping

New backend trait method `create_idea_artifact(workspace_id,
project_id, title, body, privacy) -> Result<String /* artifact_id */,
CommandError>`. Production implementation calls
`artifacts_command("register", {contractVersion: 1, workspaceId,
projectId, type: "idea", title, body, privacy})` and parses
`response["artifact"]["artifactId"]`; a missing id is an internal
error.

Error mapping for `ideas.create`:

- Malformed/missing arguments (including bad project id format) →
  `-32602`.
- Project gate failure → isError result `Project not found.`
  (never an error code — no oracle).
- Register validation failure (`InvalidRequest`, e.g. body over the
  cap) → `-32602` with the register message (validation, not
  execution failure).
- Any other backend failure → `-32603` "Tool execution failed."

Response shape: `structuredContent` is `{projectId, projectTitle,
artifactId, type: "idea", title, tags}`, text is its JSON. The body
is not echoed (the caller already holds the description); the e2e
verifies persistence through `artifacts/list`, not the response.

## 6. Test plan

- Rust `mcp_server::tests`: validation cases (missing/empty/overlong
  title, missing/empty description, bad tags shapes, bad project id);
  project gating (unknown, private, superseded, wrong-kind,
  expired) all → identical not-found; happy path asserts the fake
  backend received exactly one creation with the validated project
  id, type `idea`, mirrored privacy, and the tags trailer in the
  body; register-validation error → `-32602`.
- Rust component test (real backend, temp core): create a standard
  project, call `ideas.create`, then `artifacts/list` shows the idea
  artifact with the right title, body (description + tags line), and
  privacy; a private project's call is not-found and creates nothing.
- E2E `mcp-slice5.spec.ts`: spawn the real server with the harness
  pairing root/token; call `ideas.create` for a seeded standard
  project; assert the result carries an artifact id; then
  `artifacts/list` via the harness API shows the idea artifact in
  that project's registry with the right title and body; rejection
  cases (private project → not-found, malformed id → `-32602`,
  missing title → `-32602`) create nothing.
- Gates: typecheck 0, contracts 0 drift (no new contract DTOs — the
  tool definition lives in `mcp_server.rs` value data, as in Slice
  4; no HTTP route is added), docs PASS, `test:ui` (no UI in this
  slice), Vite build, API build, `cargo --workspace --lib`, full
  battery.
- The existing `mcp-server.spec.ts` tools/list assertion (six tools)
  is updated to seven.

## 7. Milestone 13 closure

- `docs/REQUIREMENTS_MATRIX.md`: AN-017 `in progress` → `verified`,
  text gains "Milestone 13 complete" (the docs validator accepts
  `verified`; precedent: AN-016/AN-012 closed the same way).
- `docs/SPEC_COVERAGE.md`: §68 row `planned` → `verified`.
- `README.md`: M13 row `In Progress` → `Verified`, "6 knowledge
  tools" → "7 tools (6 read-only + ideas.create mutation gate)",
  battery count updated to the new total.
- `docs/DECISIONS.md`: deferred future protocols with rationale
  (see §8). `docs/BUILD_STATUS.md` + `docs/RECOVERY.md` closure
  entries. The M13 plan file (`plans/PHASE-013-integrations.md`) is
  a tracked planning task, not an implementation status doc, and is
  left unchanged (the validator only requires its existence).
- Closure honesty note: Slice 5's own Windows rerun remains
  outstanding, as with every slice before it; the milestone closes
  per the owner's instruction on Linux evidence plus Slices 1–4
  closed on Windows. The rerun becomes the next round's baseline.

## 8. Deferred, with rationale (for `docs/DECISIONS.md`)

- **SSE / Streamable HTTP remote server hosting**: no POST HTTP
  client exists in-tree, and §68 default-deny + §71 SSRF require
  remote access behind explicit per-server authorization. Needs its
  own slice with transport + auth design; refused, not half-built.
- **Subscription pushes** (`resources/subscribe`, prompt/resource
  change notifications): the server is a one-shot stdio process with
  no push channel and no persistent-connection lifecycle. Needs a
  connection-lifecycle design; out of scope.
- **Per-tool / per-client permission grants**: the pairing token is
  session-scoped (the user's own session). Finer grants need a grant
  store, a grant UI, and a revocation story — a separate slice, not
  smuggled into the mutation gate.
- **UI for calling the ANANSI server**: the server is consumed by
  external clients (Claude Desktop, Cursor, CLI agents), not by
  ANANSI's own UI; ANANSI's client UI already calls any tool an
  external server offers via `tools/call`.

## 9. Out of scope (explicit)

- No change to Slices 3–4 tools, auth, or privacy posture.
- No UI changes (no new components, no test:ui additions).
- No new HTTP routes, no new contract DTOs.
- No schema migration, no tags column.
- No automatic or scheduled idea creation; every write follows an
  explicit, authenticated client call.
