# M11 GHOST-PROMPT Slice 3 — Addendum & §88 Milestone Completion Report (Linux delivery, Windows rerun pending)

## Baseline and the SHA discrepancy

Branch `muse/m11-ghost-prompt-slice3` from the
Windows review head for Slice 2. The assignment
and the review MANIFEST both name parent
`3698833634e062630a845cfeb368fa918b958807`,
but that full SHA does not exist: the review
bundle's only head is
`3698833646bd92353791a5948d823e05e73c1144`
(shared prefix `36988336`, different tail), whose
commit message, parent (`5818a45c3`), and
docs-only diff are exactly the review the
assignment describes — and exactly the
assignment's authorized alternative ("clean tip
`5818a45c3` — the diff is review documentation
only"). The branch was created from that
verifiable bundle head. The discrepancy is
recorded here, in the design doc, and in the
MANIFEST for reviewer confirmation. This is the
second consecutive review package with a wrong
MANIFEST tail; the bundle head, not the MANIFEST
tail, is treated as authoritative.

Slice 2 Windows verdict carried forward (review
branch `codex/review-m11-ghost-prompt-slice2`):
tsc 0, contracts 0 drift, docs 234, UI 186/186,
Rust ghost 11/11, workspace 603 + 4
offline-Ollama skips, Vite 2.31s, Playwright
20/20. AN-016 was `in progress` for the Slice 1
and 2 scopes; this slice advances it to
`verified` with "Milestone 11 complete" in its
text, because the docs validator's status set
has no `complete` value (the SOMNIA Slice 3
precedent for AN-015). Two Slice 2 reviewer
wording notes were not adopted as claims: the
report says "cubic Bézier spline threads" (the
implementation uses straight LineSegments) and
"Tab / Enter node selection" (buttons plus
canvas arrow-key cycling).

## §88 report — Features built

- Deterministic draft analysis ("Review My
  Draft", §57, §86.5): a rule-based engine
  mirrored in Rust (`crates/core/src/ghost_prompt.rs`)
  and TypeScript (`packages/ui/src/ghostPrompt.ts`)
  with identical tokenizers, keyword lists, and
  rules. Four finding kinds, fixed order, one
  finding per kind, evidence capped at 5
  triggers: price changes (≥2 distinct price
  signals — `$` numbers or qualifier words
  `cheap`/`free`/`budget`/`expensive` — across
  the event sequence), privacy requirement
  removals (keyword in deleted text, absent
  from the final text; keywords exactly
  `privacy`, `confidential`, `local`,
  `untracked`, `sealed`), audience shifts
  (term swap on a replace event, or pure
  deletion absent from the final text; terms
  `freelancers`, `enterprise`, `lawyers`,
  `startups`, `agencies`, `developers`),
  technical term substitutions (technical term
  on a replace event's deleted side, none on
  the inserted side; terms `api`, `embedding`,
  `vector`, `kubernetes`, `oauth`, `websocket`,
  `sql`, `llm`, `neural`, `algorithm`). Finding
  texts are the four §57 sentences verbatim;
  no other sentence is produced, and a unit
  test bans intent words from finding texts.
  The UI carries the disclaimer: "These are
  objective observations about text changes, not
  interpretations of intent."
- Explicit fragment selection (§57, §86.6): the
  verbatim question "Would you like ANANSI to
  consider any of these earlier alternatives?"
  with one checkbox per deleted fragment.
  Selection is strip state, cleared on Clear
  and on Stop. Panel B is built only from
  selected fragments; unselected text appears
  nowhere in Panel B, its context list, or any
  dispatch — proven by a dedicated e2e test.
- Dual-panel comparison (§58, §86.7):
  `GhostPromptCompare.tsx` with Panel A "Final
  Prompt Only" (final text exactly) and Panel B
  "Final Prompt + Approved Draft Context"
  (final text plus the compiled approved
  context, with the included fragments listed
  verbatim). Counts are labeled units:
  Unicode scalar characters, estimated tokens
  (`ceil(chars / 4)`), simulated cost delta at
  a stated assumed rate ($2.00 per 1M tokens,
  6 decimals) — no real provider price claimed.
  Responses are deterministic offline reference
  texts by default, labeled as comparisons, not
  evidence of subconscious insight. The "Send
  both to Ask ANANSI" button dispatches exactly
  the two panel prompts through the existing
  `startAsk`/`pollAsk` path with the user's
  current Ask settings, sequentially, only on
  that explicit click. Feedback toggle ("Which
  answer was more useful?") is local state.
  "Combine into an updated prompt" fills the
  Ask prompt input with the Panel B text for
  user review and edit — never auto-submitted,
  never auto-rewritten.
- §59 in its consent-gated form: the combined
  prompt is ordinary user input from that point
  on; no send-to Idea Generator / Collider /
  Forge path is built, and no deleted text is
  transmitted without approval.
- §86 Definition of Done verified end to end in
  `tests/e2e/ghost-prompt-slice3.spec.ts` (all
  9 items in one walk, plus the unselected-
  fragment guardrail test).

## §88 report — Files added

- `docs/handoffs/M11-ghost-prompt-slice3-design.md`
- `docs/handoffs/M11-ghost-prompt-slice3-addendum.md` (this file)
- `packages/ui/src/GhostPromptCompare.tsx`
- `packages/ui/tests/ghost-prompt-analysis.test.tsx`
- `tests/e2e/ghost-prompt-slice3.spec.ts`

## §88 report — Files modified

- `crates/core/src/ghost_prompt.rs` (analysis
  engine + approved-context compiler + 6 new
  tests; ghost suite 11 → 17)
- `packages/ui/src/ghostPrompt.ts` (TS mirror
  of the same engine + compiler)
- `packages/ui/src/GhostPrompt.tsx` (Review My
  Draft subsection, selection state, Compare
  toggle; selection cleared on Clear/Stop)
- `packages/ui/src/AskAnansi.tsx` (combine
  wiring via `setPrompt`; real two-prompt
  dispatch via `startAsk`/`pollAsk`)
- `package.json` (`test:ui` list gains the
  analysis test file)
- `docs/BUILD_STATUS.md`, `docs/RECOVERY.md`,
  `docs/REQUIREMENTS_MATRIX.md` (AN-016 →
  `verified`, Milestone 11 complete),
  `plans/09-ghost-prompt.md`

## §88 report — Architecture decisions

- Analysis lives in the existing engine modules
  (Rust reference + TS mirror) rather than a
  new service: the same discipline as Slices 1–2,
  and the same byte-identical finding sentences
  in both suites.
- The price rule counts distinct signals, not
  edits: a `$50` → `$20` change and a `cheap` →
  `$50` change are both price changes, while a
  single signal is not. Bare numbers are not
  price signals (no currency context), which is
  why the e2e price edit uses a selection
  replacement that keeps the `$` inside the diff
  span — the full-text diff would otherwise
  leave the `$` in the shared prefix.
- Comparison responses default to offline
  reference texts: the walk is fully verifiable
  without a live endpoint, and the provider path
  is an explicit, separately labeled action.
- No server op, no new artifact type, no schema
  change: analysis and comparison are
  client-side over the in-memory session.
  Contracts regen confirms zero drift.

## §88 report — Database migrations

None. No schema change in this slice (schema
remains v11).

## §88 report — Tests passed

- Rust workspace **609 passed + 4 known
  live-Ollama `semantic_workflow` environment
  failures** (603 + 6 new ghost analysis
  tests), strict Clippy clean, fmt clean.
- Node **192/192** (186 + 6 new analysis
  tests).
- tsc 0, contracts `--check` exit 0 (zero
  drift), docs validation PASS, web build
  clean.
- E2E `ghost-prompt-slice3.spec.ts` **2/2**
  (§86 walk; unselected-fragment guardrail),
  zero provider traffic on the offline path.
- Regression battery **22/22**: ghost Slices
  1–3 (2/2 each), SOMNIA Slices 1–3 (2/2 each),
  evaluations 1/1, integrated workflow 1/1,
  opportunities 1/1, SwarmTank scenario 3/3,
  arena 3/3, slice1 1/1.

## §88 report — Tests failed

- The 4 pre-existing live-Ollama
  `semantic_workflow` environment failures
  (embedding model absent on this VM), identical
  to baseline. No other failures.

## §88 report — Tests not run

- The full e2e directory run (including the
  known-failing `search.spec.ts` real-semantic
  test, a pre-existing live-Ollama environment
  failure) was not re-run in this slice; the
  12-spec regression battery above covers the
  affected areas. The provider-dispatch button
  was not clicked in e2e (no live endpoint in
  the test environment); the dispatch runner is
  unit-tested with a recording fake client.

## §88 report — Security issues

None found. Credentials sweep over the delivery
diff clean. No deleted text is persisted,
transmitted, or dispatched without explicit
user selection (and, for providers, an explicit
dispatch click). No password scope, no global
keylogging (editor-scoped input only, as in
Slice 1). The comparison's reference responses
contain only the user's own prompt text.

## §88 report — Live-provider verification

None in this slice. The comparison's default
path is offline reference text; the provider
path is implemented (Ask `startAsk`/`pollAsk`
with the user's settings) but was not exercised
against a live endpoint, and no live-model
claim is made. This is stated, not hidden:
live-provider verification remains an open
global gate (see below).

## §88 report — Known limitations

- The keyword lists are small and English-only;
  a price, privacy, audience, or technical
  change phrased outside those lists produces no
  finding (a deliberate precision-over-recall
  choice for a deterministic engine).
- Bare numbers without a `$` are not price
  signals; only `$`-prefixed numbers and the
  four qualifier words count.
- The provider-dispatch path has no cancel
  button inside the comparison panel; closing
  the panel does not cancel in-flight asks (the
  asks complete server-side and their results
  are dropped with the component).
- §59's send-to destinations (Idea Generator,
  Collider, Forge) are not built; the combined
  prompt is the handoff, as ordinary user input.

## §88 report — Last verified commit

Branch `muse/m11-ghost-prompt-slice3`, tip
recorded in the delivery MANIFEST (this
addendum's closure commit). Baseline: Windows
review head `3698833646bd92353791a5948d823e05e73c1144`
(see the discrepancy note at the top).

## §88 report — Backup status

No backup/restore behavior changed in this
slice. GHOST-PROMPT sessions are in-memory
only and are never backed up; saved artifacts
(SwarmTank runs, SOMNIA evaluations, Forge
concepts) are untouched by this slice, and the
regression battery covering their specs passed
22/22.

## §88 report — Current runnable command

Web app: the repo's standard Vite preview
(`apps/web` build + preview on 127.0.0.1:1420
under the application-test lease in e2e, as in
prior slices). API: `target/debug/anansi-api`
built via `node scripts/cargo.mjs build
--locked -p anansi-api`. UI tests: `npm run
test:ui`. Rust: `node scripts/cargo.mjs test
--locked --workspace --no-fail-fast`.

## §88 report — Next milestone

None commissioned. **Milestone 11 is complete**
in substance on Linux (Slices 1–3 delivered;
Slice 3 awaits its own independent Windows
rerun, checklist in the delivery MANIFEST).
Remaining global gates, unchanged: M7 native
desktop acceptance + preview promotion, §32
execution sandbox, M5 production gates
(including live-provider verification), Windows
verification of M8 Slice 4.
