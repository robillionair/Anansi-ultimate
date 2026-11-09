# SOMNIA compaction authorization repair — design + acceptance criteria

## §0 Baseline, branch, scope

- **Baseline (verified):** the delivered authorization slice,
  `muse/somnia-compaction-authorization` at
  `4577ba131ee898408c6cf3f4a48304a0110f97ac`. Verified locally
  (`git rev-parse`); the working tree was clean.
- **Branch:** `muse/somnia-compaction-authorization-repair`,
  created from `4577ba131`.
- **This slice:** repair the two defects the authorization slice
  left open (its handoff §4, known limit D6, and the privacy
  audit the commission orders):
  1. A blank Model ID resolves at run start, AFTER the user
     approved — a configuration change between approval and start
     silently redirects both arms to a different model (§1 R1).
  2. The outgoing prompt segment is labelled `standard` even
     when the imported history contains Private records, so the
     provider privacy gate never sees the real class and the
     disclosure misreports what was sent (§1 R2).
- **Out of scope:** the compactor library semantics (Checkpoint 1
  rules frozen), the frozen benchmark fixtures and the scorer,
  a run-scoped permit for billable comparisons (still a design
  note only, authorization design §5), any change to the provider
  gates themselves (they are used as-is), any global-status
  change (AN-015 stays `verified`). Keep this repair separate
  from Antigravity's work: no edits to Antigravity-owned files,
  no preview promotion, no pushes, no remote CI.

## §1 Findings (evidence, baseline code)

### R1 — The approved target is a reference, not a resolution

- The approval hash binds the RAW provider/model (blank allowed)
  (`somnia_compaction_run.rs:512` `compute_approval_hash`;
  the UI hashes the trimmed raw values,
  `SomniaCompaction.tsx: approve()`). `resolve_model`
  (`:579`) runs at start, AFTER the hash check (`:837`).
- A blank model is therefore a reference to "first configured
  at start time" (authorization design §2 D6, handoff §4): if
  the configuration changes between approval and start — the
  first model is removed, replaced, or reordered — the run
  uses the then-first model and only reports it afterwards.
  The user reviewed one target and the run used another, with
  no refusal. The same holds for the effective endpoint
  (configured `baseUrl`), locality, and cost class: none is
  bound to the approval at all.
- The UI has no read op for the installed configuration, so it
  cannot resolve the blank default itself; resolution must be
  a server op the UI calls before approval.

### R2 — The prompt segment is labelled Standard whatever it contains

- `dispatch_pair_arm` (`somnia_compaction_run.rs:1017`)
  sends one segment per arm:
  `{ "text": prompt, "privacy": "standard" }` — hardcoded.
- Imported records carry privacy `standard | private | public`
  (`somnia_compaction.rs:93`; `sealed` is refused at parse,
  `:224`). The prompt is built from those records
  (`render_history` / `build_prompt`), so a history containing
  Private records is transmitted as a `standard` segment.
- Consequence 1: the provider privacy gate
  (`crates/providers/src/gates.rs`) refuses Private content to
  a cloud target without explicit cloud authorization — but it
  never sees the class, because the segment says `standard`.
  Consequence 2: the disclosure misreports what was sent:
  `DisclosureRecord` carries no privacy field, and the
  compaction pair results carry none either, so nothing in the
  run record shows that Private content was transmitted.
- The two arms can legitimately differ: the original arm renders
  ALL case records, the compacted arm only the retained ones.
  A Private record excluded from the compacted history (e.g.
  superseded) must not raise the compacted arm's class.

## §2 Design decisions

- **D1 — Resolve before approval (new op).** New server op
  `somnia-compaction-resolve-target` (contract v1, same
  workspace/project gating as the other compaction ops, no
  dispatch, no permit, no approval needed — it is a bounded
  read: it returns only the resolved target, never credentials
  or the full configuration). Input: raw provider/model (blank
  allowed). Output: the EXACT `{provider, model, locality,
  cost, endpoint}` where endpoint is the configured `baseUrl`
  (trimmed) for that provider, falling back to the catalog
  default when blank. Resolution uses the same rules as start:
  blank model → first configured chat model for the provider
  (configuration order); blank provider+model → first overall;
  explicit target must be configured; blank provider +
  explicit model refused. The UI calls it at approve time and
  shows the exact resolved provider/model/locality/cost/
  endpoint in the review; the approved line shows the resolved
  target, never "first configured model".
- **D2 — The hash binds the resolved target.** The approval
  hash input gains `endpoint`, `locality`, and `cost` (after
  model, before cloud): canonical JSON
  `{cases, provider, model, endpoint, locality, cost, cloud,
  maxTotalTokens}` over the RESOLVED values. The UI, the e2e
  mirror, and the server must agree byte-exact (same discipline
  as the authorization slice). Start now REQUIRES exact
  provider+model (blank refused with a clear message). The
  server re-resolves the requested exact target from LIVE
  config, recomputes endpoint/locality/cost, and recomputes
  the hash from those live values — before model resolution
  completes and before any dispatch. Target removed → resolve
  refuses. Endpoint, locality, or cost changed → hash mismatch
  → refuse. A changed default can never silently redirect
  either arm, because the run only starts when the live target
  is byte-identical to the approved one.
- **D3 — Strongest privacy per arm.** A pure function
  `strongest_privacy(records) -> Result<&'static str>`:
  `public` < `standard` < `private`; the strongest class among
  the records rendered in that arm's prompt. Original arm: all
  case records. Compacted arm: retained records only. `sealed`
  or an unknown value → Err (fail closed; sealed is already
  refused at parse, this is the second layer). `dispatch_pair_
  arm` takes the class and sends it as the segment's `privacy`.
  The provider gates are used as-is: with the real label, a
  Private segment to a cloud target without explicit cloud
  authorization is refused by the gate itself; with explicit
  authorization it is allowed; local dispatches are unaffected.
- **D4 — Disclosure per arm, format v1 preserved.** `Compaction
  PairResult` gains `original_privacy` and `compacted_privacy`
  as `Option<String>` with `#[serde(default,
  skip_serializing_if = "Option::is_none")]`: `Some(class)`
  when the arm's class is not `standard`, `None` when it is.
  Old saved experiments (no such fields) deserialize with
  `None`, serialize identically, and their content hashes
  still validate — the v1 format is preserved. The TS
  `canonicalPair` mirrors the skip rule (omit the key when
  absent). The UI results and saved-experiment views show the
  per-arm privacy from the poll (server-disclosed, never
  client-derived).
- **D5 — What does NOT change.** The free-only gate, Zero-Cost
  Lock, and the paid/unknown/permit refusals are untouched and
  still run at start before traffic. Sealed records stay
  excluded (parse refusal + D3 fail-closed). The frozen
  benchmark fixtures and the scorer are untouched. The saved
  experiment format stays v1. No new cloud calls for
  verification: all provider traffic in tests goes to loopback
  fakes, as before.

## §3 Acceptance criteria

- **AC-R1** Blank default resolves before approval. With two
  configured models (`llama-x` first, `aaa-decoy` second) and a
  blank Model ID, the review shows the exact resolved target
  (`ollama` / `llama-x` · local · free · endpoint); approval
  hashes the resolved values; the run completes with `llama-x`
  on both arms; the poll echoes `llama-x`.
- **AC-R2** Changed default never silently redirects. After
  approval, reconfiguring so `aaa-decoy` is first (or removing
  `llama-x`) makes the old approval refuse at start (400, 0
  hits); a fresh resolve + approve runs the new first model on
  both arms.
- **AC-R3** Endpoint / locality / cost change requires fresh
  review. After approval, changing the configured `baseUrl`
  (or the cost class) for the approved target makes the old
  approval refuse at start (400, 0 hits); a fresh resolve +
  approve runs.
- **AC-R4** Exact target still enforced. An explicit target
  that is not configured is refused by resolve (and by start);
  blank provider + explicit model refused; a start with a
  blank provider/model is refused with a clear message.
- **AC-R5** Privacy per arm. A Private record in the history
  makes the original arm's segment `private`; when that
  Private record is excluded from the compacted history, the
  compacted arm's segment is `standard`. Both classes are
  disclosed in the poll pair results and shown in the UI.
- **AC-R6** Private cloud authorization. A Private history
  approved for cloud dispatches segments labelled `private`
  and completes; the same history with the cloud flag flipped
  after approval is refused (400, 0 cloud hits). Sealed records
  are refused at prepare/start with 0 hits.
- **AC-R7** Both arms pinned to the exact resolved target.
  With two configured models, all dispatch bodies carry the
  exact resolved provider/model (fake records); the successful
  local workflow still works (the existing blank-model and
  benchmark flows green under the new resolve-before-approval
  flow).
- **AC-R8** Frozen benchmark and scorer preserved. The 40-case
  benchmark still passes with the same synthetic numbers
  (21/50 original, 21/50 compacted, 0 failed pairs, 1600
  tokens); no change to `crates/core/src/somnia_compaction.rs`,
  the fixtures, or the scorer.
- **AC-R9** Focused checks green (Linux): Rust lib (including
  new unit tests), tsc, contracts `--check`, docs, full UI
  suite, compaction e2e, LARP scene 09. The pre-existing
  `providers.spec.ts` timeout and `larp-somnia.spec.ts`
  instruments failure are reported as-is, not fixed.
- **AC-R10** No live cloud calls, no real credentials, no paid
  inference. All provider traffic in tests goes to loopback
  fakes. Zero-Cost Lock and the free-only gate intact
  (billable/permit refusals still 400 with 0 hits).

## §4 Tests (planned)

- Rust unit tests in `somnia_compaction_run.rs`: resolve-target
  (blank → first with endpoint/locality/cost; explicit target;
  removed target refused); approval hash binds endpoint/cost
  (changed endpoint → different hash); strongest privacy per
  arm (private present → private; excluded → standard;
  all-public → public; sealed/unknown → error).
- UI tests in `somnia-compaction.test.tsx`: approval hash
  changes when endpoint/locality/cost change; resolved target
  shown from the resolve response; per-arm privacy shown from
  the poll; canonical pair omits absent privacy keys (hash
  agreement with the server).
- e2e `somnia-compaction.spec.ts` (new tests, hit-counting
  fakes, two configured models): blank-model UI run asserting
  the review shows the resolved target (AC-R1); config-change
  after approval — reorder to `aaa-decoy` first, old approval
  400 + 0 hits, fresh approve runs `aaa-decoy` (AC-R2);
  endpoint change after approval — old approval 400 + 0 hits
  (AC-R3); Private history — segment labels per arm on the
  fake's recorded dispatch bodies + poll disclosure (AC-R5);
  Private cloud authorized (labels `private`, completes) vs
  cloud-flag tamper (400, 0 cloud hits) + sealed refusal
  (AC-R6); dispatch-body model recording for pinning (AC-R7).
- Existing suites unchanged in shape: benchmark spec still
  asserts the same synthetic numbers (AC-R8); the older
  workflow tests are updated to resolve first (explicit
  resolved values + the new hash inputs).

## §5 Verification plan

Reproduce the defects first (new tests fail on the unfixed
code), then fix, then re-run: Rust workspace lib, tsc,
contracts `--check`, docs validator, full `test:ui`, the
compaction e2e specs, `larp-somnia.spec.ts`. Package as a
checksummed bundle ZIP with MANIFEST + evidence + a Windows
rerun checklist, delivered with the ZIP attached. Stop at this
slice.
