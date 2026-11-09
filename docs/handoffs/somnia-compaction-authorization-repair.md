# SOMNIA compaction authorization repair — handoff

Branch: `muse/somnia-compaction-authorization-repair`, from the delivered
authorization tip `4577ba131ee898408c6cf3f4a48304a0110f97ac`.
Implementation commit: `34be314a5fca9a5bbc11a613db66b15182ea8b41` (design + AC
committed first as `08da3964d`). This handoff is committed after it; the
ZIP MANIFEST is authoritative for the delivered tip.

Scope: the remaining approval gap and the privacy audit from the repair
commission. Nothing else. The authorization slice's known limit D6 (blank
model binds "first at start time") is what this repair closes.

## §1 What changed

### Approval gap (AC-R1..R4, AC-R7)

- New server op `somnia-compaction-resolve-target` (contract v1, same
  workspace/project gating as prepare/start, bounded read returning only
  `{provider, model, locality, cost, endpoint}`; endpoint is the
  configured `baseUrl` trimmed, falling back to the catalog default
  when blank). A blank provider/model resolves to the first configured
  chat model; an explicit target that is not configured, or a blank
  provider with an explicit model, is refused (400).
- The panel resolves the default **before** approval: `approve()` calls
  the resolve op, hashes the resolved values, and the review shows the
  exact target (`ollama / llama-x · local · free · http://…`), never
  "first configured model". `start()` sends the approved exact
  provider/model; the live form values are no longer used at start.
- The approval hash now binds the resolved target: canonical JSON
  `{cases, provider, model, endpoint, locality, cost, cloud,
  maxTotalTokens}` in that key order (per case `caseId, candidateHash,
  questions`; per question `id, text, expectedAnswer`). The client
  hash, the e2e mirror, and the server hash build the same string.
- `start` refuses a blank provider/model with "Resolve the comparison
  target before approval…", resolves the exact target, re-derives
  endpoint/locality/cost from the **live** configuration, and recomputes
  the hash before any dispatch. A removed target fails resolution; a
  changed endpoint/locality/cost fails the hash. Both refuse before
  traffic (400, 0 hits). A changed default can never silently redirect
  either arm.
- The resolve op auto-installs the persisted provider config, as
  `start` does (see §4: this was a defect found during verification,
  not a test artifact).

### Privacy per arm (AC-R5, AC-R6)

- Each arm sends the strongest privacy class of the history it
  actually dispatches (`public < standard < private`; sealed/unknown
  fail closed; floor **standard**, because the prompt also carries the
  unclassified question text — a public-only history therefore labels
  `standard`). The two arms can differ: the original arm covers all
  case records, the compacted arm only the retained ones.
- The existing provider gates are used as-is: the segment's privacy
  now reaches the gate (previously it was hardcoded `standard`), and
  the pair fails if the response disclosure's `strongestPrivacy` does
  not match the class sent.
- `CompactionPairResult` gains `originalPrivacy`/`compactedPrivacy`
  (`Option<String>`, omitted when standard), so old saved experiments
  serialize identically and format stays v1. The poll discloses both
  classes; the panel annotates each arm's score with the sent privacy.
- Sealed records remain excluded (refused at parse/prepare and at
  start, 0 hits). Zero-Cost Lock, the free-only gate, and the
  paid/unknown/permit refusals are untouched.

### Unchanged

Free-only gate, Zero-Cost Lock, sealed exclusion, the frozen benchmark
and scorer (`crates/core/src/somnia_compaction.rs`, fixtures, scorer
untouched), the 40-case/50-question benchmark numbers, and the
authorization slice's provider gates. `rustfmt --edition 2024` was
applied to touched files only (workspace-wide fmt diff 922 lines vs
1082 at baseline — the baseline is not fmt-clean).

## §2 Acceptance criteria — status

- **AC-R1** PASS. Blank Model ID, two models (`llama-x` first,
  `aaa-decoy` second): the review shows `ollama / llama-x · local ·
  free · <endpoint>`; the run completes with `llama-x` on both arms;
  the poll echoes `llama-x`.
- **AC-R2** PASS. After approval, reordering so `aaa-decoy` is first:
  the old approval still runs `llama-x` on both arms (the resolved
  target is bound, not the position). Removing `llama-x`: old approval
  400 + 0 hits; fresh approve runs `aaa-decoy` on both arms.
- **AC-R3** PASS. After approval, changing the configured `baseUrl` to
  a second loopback server: old approval 400 + 0 hits on both
  endpoints; fresh approve hits the new endpoint twice.
- **AC-R4** PASS. Unconfigured explicit target refused by resolve (and
  by start); blank provider + explicit model refused; blank start
  refused with the resolve-before-approval message.
- **AC-R5** PASS. Private record superseded by a standard one: poll
  pair `originalPrivacy === 'private'`, `compactedPrivacy` absent
  (standard); both shown in the UI.
- **AC-R6** PASS. Private history approved for cloud completes via the
  cloud fake with `originalPrivacy 'private'`; the same history with
  the cloud flag flipped after approval is refused (400, 0 cloud
  hits). Sealed records refused at prepare and start, 0 hits.
- **AC-R7** PASS. All dispatch bodies in the two-model runs carry the
  exact resolved provider/model (fake records); the existing
  blank-model and benchmark flows run green under resolve-before-
  approval.
- **AC-R8** PASS. Benchmark unchanged: synthetic 21/50 original, 21/50
  compacted, 0 failed pairs, 1600 tokens.
- **AC-R9** PASS (Linux) except the two pre-existing failures reported
  as-is below.
- **AC-R10** PASS. All provider traffic in tests goes to loopback
  fakes; no live cloud calls, no real credentials, no paid inference.

## §3 Verification (Linux, 2026-11-09)

Before evidence (unfixed code, 6 new tests): **5 failed, 1 passed**
(the Sealed test passed before the fix — parse already refused sealed;
a preserved property, not a reproduction). Saved at
`deliveries/somnia-compaction-authorization-repair/evidence/before-e2e-reproduction.md`.

After (implementation commit `34be314a5`, full evidence at
`deliveries/somnia-compaction-authorization-repair/evidence/after-checks.md`):

- New e2e regressions: 6/6; full `somnia-compaction.spec.ts`: **16/16**.
- Benchmark: 1/1, synthetic numbers identical to Checkpoint 2 and the
  authorization slice.
- `larp-somnia.spec.ts`: 1 passed, 1 failed — the instruments test, the
  known pre-existing failure (locator.click timeout on 'Load built-in
  cases', LARP overlay), reproduced identically on pristine `4e4e970a`
  in Checkpoint 2. Not a regression; not fixed in this slice.
- `cargo test --locked --workspace --lib`: **345/345** (api 4, core 278,
  desktop 1, providers 62; baseline 341 — +3 core, +1 providers).
- `node scripts/contracts.mjs --check`: exit 0 (pre-existing "unknown
  format uint" warnings only).
- `pnpm typecheck`: exit 0. `pnpm test:ui`: **337/337** (baseline 336 —
  +1 canonical-pair privacy test).
- `node scripts/validate-docs.mjs`: PASS, 328 files (baseline 327 — +1
  design doc).
- `providers.spec.ts` was not re-run in this slice; its single-workflow
  240 s timeout is the known pre-existing failure reproduced identically
  on pristine `cf69e37e` in the authorization slice. It is in the
  Windows rerun checklist below.

## §4 Defect found during verification

The six new tests initially still failed after the Rust/UI implementation:
`resolveTarget` returned 400 "No chat model is configured". Cause:
`ai_command` auto-installs the persisted provider config only for a
listed set of operations, and `somnia-compaction-resolve-target` was
not in it while `somnia-compaction-start` is — so resolve ran before
any config was installed. Fixed by adding the op to that list in
`crates/core/src/ai_discovery.rs`; all six tests then passed. The
manual curl probe that exposed this used a config file written after
API start with no workspace open, which is why it reproduced the same
400 outside e2e.

## §5 Known limits

- A blank Model ID binds the first configured model **at approve
  time**. A configuration change between approval and start is caught
  (fresh review required), but the approved run is still a snapshot
  of the configuration at approval — that is the intended semantics.
- The resolve op returns the configured `baseUrl` trimmed, or the
  catalog default when blank; it does not probe the endpoint.
- AC-I10 live rerun still BLOCKED on this VM (no local model, no
  provider credential); rerun path unchanged via
  `ANANSI_SOMNIA_LIVE_ENDPOINT` / `ANANSI_SOMNIA_LIVE_MODEL`.
- `providers.spec.ts` and `larp-somnia.spec.ts` instruments remain
  pre-existing failures (see §3); 4 `semantic_workflow` live-Ollama
  Rust failures and 4 strict-Clippy lints (`mcp.rs:715`, `mcp.rs:1225`,
  `mcp_server.rs:584`, `mcp_server.rs:987`) are likewise pre-existing.

## §6 Windows rerun checklist

1. Extract the ZIP, verify the bundle SHA against the MANIFEST, fetch
   from the bundle and confirm the tip matches the MANIFEST.
2. `node scripts/cargo.mjs test --locked --workspace --lib` — expect
   345/345.
3. `node scripts/contracts.mjs --check`; `pnpm typecheck`;
   `pnpm test:ui` — expect 337/337.
4. `node scripts/validate-docs.mjs` — expect PASS, 328 files.
5. Rebuild the API, rebuild the web app, then e2e (one spec per
   invocation, lease script):
   - `tests/e2e/somnia-compaction.spec.ts` — expect 16/16, including
     the 6 new tests (exact resolved target, stale approval after a
     default change, stale approval after an endpoint change,
     per-arm privacy labels, Private cloud only with approval, Sealed
     refused).
   - `tests/e2e/somnia-compaction-benchmark.spec.ts` — expect the
     synthetic 21/50, 21/50, 0 failed, 1600 tokens.
   - `tests/e2e/providers.spec.ts` — pre-existing 240 s timeout
     expected on Linux; report the Windows outcome as-is.
   - `tests/e2e/larp-somnia.spec.ts` — 1 pass expected; the instruments
     test is the known pre-existing failure; report the Windows
     outcome as-is.
6. If a local model is available, run the AC-I10 live rerun via
   `ANANSI_SOMNIA_LIVE_ENDPOINT` / `ANANSI_SOMNIA_LIVE_MODEL`.

## §7 Proposed global-status updates

NONE. AN-015 stays `verified`. M13 is not complete; AN-025 stays
planned. Antigravity owns the global status/release documents; no
files under that ownership were edited.
