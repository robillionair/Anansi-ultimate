# MCP Slice 3 Repair Addendum — Revised Spec Compliance

Owner-authorized revision of the Slice 3 delivery (branch `muse/mcp-slice3`,
tip `98ac536fc`) against the fuller Slice 3 assignment received after that
delivery. Branch: `muse/mcp-slice3-repair`, from `98ac536fc`.

## Why a repair, not a redo

The earlier Slice 3 package (ZIP `d7c71f32…`, bundle `2dea572b…`) already
delivered the server core, the three tools, and the stdio loop. The revised
assignment keeps that scope but changes five requirements. This repair applies
exactly those five deltas; everything else in the Slice 3 package is
unchanged. The revised package reuses the instructed filename
`anansi-mcp-slice3-2026-11-04.zip` and SUPERSEDES the earlier file of that
name — the old hash `d7c71f32…` no longer denotes the current Slice 3.

## The five deltas

1. **Server identity: `anansi-server`.** The revised spec fixes
   `serverInfo.name` to `"anansi-server"`. The first delivery reported
   `"ANANSI"` (the first assignment named no identity). Changed in
   `SERVER_NAME`, the contract info value, and the e2e assertion.
2. **`memory.search` kind filter.** New optional `kind` argument, validated
   against the core's supported kinds (`note`, `project`, `task`, `person`,
   `organization`, `decision`, `event`, `document`, `memory`) — anything
   else is `-32602`. The backend passes it to the validated list query
   (whose title search already honors `kind`), and the server layer
   enforces it again, so the filter holds even if a backend ignores it.
3. **`projects.search` stage metadata.** Each project hit now carries
   `stage` (current stage id or null), `stageLabel`, and `stages` (all seven
   stages with id, label, status), derived with the Rust `derive_pipeline`
   — the same function the Dashboard's TypeScript mirror implements, so
   server and UI cannot drift. Artifact types come from the validated
   `artifacts/list` query per project; sealed artifacts are excluded from
   the derivation, and stage lookup runs only for surviving (active,
   shareable) hits.
4. **Strict exposure boundary: sealed AND private never exposed.** The
   revised spec's words are "Sealed or private memories must never be
   exposed", read here in the strict sense: `Privacy::Private` — the
   default for created memories — is excluded alongside `Sealed`. Only
   memories explicitly marked `standard` or `public` leave the process.
   This is the fail-closed reading, and it is flagged as a judgment call:
   if the owner meant "private" loosely (non-public in general), the
   filter is one predicate change, but the safe default ships now. The
   consequence is stated plainly: a workspace of default-created memories
   is invisible to the server until individual records are marked
   standard or public. `get` on a missing, sealed, or private id returns
   the identical `Memory not found.` result.
5. **Pairing-token authentication, replacing the bespoke env token.** The
   first delivery invented `ANANSI_MCP_SERVER_TOKEN`; the revised spec
   requires a valid workspace session pairing/bearer token, so that
   mechanism is REMOVED and replaced. The server takes `--pairing-root`
   (defaulting to the API's host pairing root) and, on every `initialize`,
   re-reads `<pairing-root>/session-*/pairing.json` tokens — the same
   files the HTTP API's session pairing writes. A presented token is valid
   only while its session is live: pairing sessions delete their files on
   shutdown, so revocation is automatic, and a session started after the
   server also works. The server refuses to start when the pairing root
   holds no live session. No second credential inventory exists, and the
   token is never logged or echoed. The contract DTO gains
   `authMethod: "pairing-token"` (schema regenerated, 0 drift) so the
   mechanism is visible in contracts without ever carrying a token value.
   Liveness caveat, learned from this machine: "live" means "a pairing
   file exists". Clean shutdown deletes the file, but a killed API
   process leaves its pairing file behind — this VM's host pairing root
   held two orphaned October session files, which is exactly how the bug
   below stayed hidden. A crashed session's token therefore stays valid
   until its file is removed; operators should treat the pairing root as
   credential material and clean orphaned session directories.

Unchanged from the first delivery: exactly three read-only tools, the
allowlist before any backend call (including refusing `ideas.create` with
no backend contact), both protocol versions, NDJSON stdio with stdout
reserved for protocol traffic (diagnostics to stderr only), read-only
construction (no export worker or backup scheduler), and the documented
splits (search = titles/snippets only, `get` = the only body path).

## In-repair incident: the duplicate `--pairing-root` arm (found by the e2e, fixed)

The first repair build added a second `--pairing-root` match arm to the API
argument parser, but the parser already had one storing into `root`. Rust
compiled the duplicate as an unreachable-pattern warning, which went
unread in the build log, and the flag value was silently discarded: the
server always validated against the host default pairing root. The
rewritten e2e caught it exactly: `initialize` with the harness session's
token was refused (the server was checking unrelated orphaned sessions),
and the no-live-session test hung on stdin for the full 300s because the
server had started against those orphans. Fix: the duplicate arm is
removed; the mcp branch uses the single shared `root` (flag value or host
default), with a comment naming the trap. The two orphaned October
session files in the host pairing root were removed after confirming no
API process owned them. Lesson carried forward: read build warnings on
API builds, and never add a second arm for a flag the parser already
handles.

## Verification (Linux)

- `npm run typecheck`: 0 errors.
- `node scripts/contracts.mjs --check`: 0 drift.
- `node scripts/validate-docs.mjs`: 275 PASS.
- `npm run test:ui`: 293/293 PASS (no UI in this repair).
- Vite production build: clean (chunk-size advisory only); API build
  clean (one pre-existing warning only: Windows-only
  `preserved_env_entries` unused on Linux).
- `mcp_server::tests`: 12/12 PASS (rewritten for the revised spec:
  revised name, pairing gate, kind filter incl. server-side enforcement
  against a kind-ignoring fake backend, stage metadata with artifact-call
  scoping, sealed + private never leaking with `get` identical to missing,
  startup refusal without a live session, real-backend component round
  trip with standard/private seeds).
- `cargo --workspace --lib`: 240/240 PASS, 0 failed.
- `tests/e2e/mcp-server.spec.ts`: 2/2 PASS in 5.0s (rewritten: live
  pairing token from the harness — new additive `pairingRoot`/
  `pairingToken` Session fields — standard/private seeds where the
  private title deliberately contains the query token so only the filter
  can exclude it, kind filter, stage `discover` on a fresh project,
  Harbor App exclusion proven by a title-matching `Harbor` query
  returning empty, both protocol versions, refusal to start against an
  empty pairing root).
- `tests/e2e/mcp.spec.ts`: 6/6 PASS (harness change regression check).
- Full regression battery: 54/54 PASS (22 specs, sequential, real
  filenames; per-spec pass lines captured with failure markers included
  in the filter, so a failure could not hide behind a tail).
- Credentials sweep over the diff vs the repair baseline: clean — the
  only credential-pattern matches are removals of the old env-token
  mechanism, and the only hex constant is the synthetic repeating-pattern
  test token.

## Known limits (unchanged, restated)

- Sealed/private behaviour is Rust-covered plus e2e-covered for private
  (private rows CAN be manufactured through the API, unlike sealed rows,
  so the e2e proves private exclusion end to end; sealed exclusion in the
  e2e remains unprovable because sealed rows cannot be created through any
  public API, and no test hand-writes storage files).
- One pairing token = the fixed three-tool permission set; per-token tool
  grants remain deferred.
- The server serves one client for its lifetime; tokens are re-read per
  `initialize`, not per request.
- Deferred: `notes.search`, `products.get`, `experiments.list`,
  `ideas.create`, resources/prompts, Streamable HTTP, OAuth.
- AN-017 stays `in progress`.
