# M11 GHOST-PROMPT Slice 3 — Design (before code)

## Baseline — including a SHA discrepancy, reported not hidden

Branch `muse/m11-ghost-prompt-slice3` from the
Windows review head for Slice 2. The assignment
and the review MANIFEST both name parent
`3698833634e062630a845cfeb368fa918b958807`,
but that full SHA does not exist: the review
bundle's only head is
`3698833646bd92353791a5948d823e05e73c1144`
(shared prefix `36988336`, different tail), whose
commit message ("verify M11 GHOST-PROMPT Slice 2
on Windows"), parent (`5818a45c3`), and diff
(docs-only: BUILD_STATUS, REQUIREMENTS_MATRIX,
review report) are exactly the review the
assignment describes — and exactly the
assignment's authorized alternative ("clean tip
`5818a45c3` — the diff is review documentation
only"). The branch was therefore created from
that verifiable bundle head. The discrepancy is
recorded here, in the addendum, and in the
MANIFEST so the reviewer can confirm the tail;
nothing was built on an unverified commit. This
is the second consecutive review package with a
wrong MANIFEST tail (Slice 2 had the same
pattern); the bundle head, not the MANIFEST
tail, is treated as authoritative in both cases.

Slice 2 Windows verdict carried forward (review
branch `codex/review-m11-ghost-prompt-slice2`):
tsc 0, contracts 0 drift, docs 234, UI 186/186,
Rust ghost 11/11, workspace 603 + 4
offline-Ollama skips, Vite 2.31s, Playwright
20/20 (ghost Slice 2 2/2, regressions 18/18).
AN-016 stays `in progress` until this slice
closes it. Two reviewer wording notes are NOT
adopted as claims: the Slice 2 report says
"cubic Bézier spline threads" (the implementation
uses straight LineSegments) and "Tab / Enter node
selection" (the implementation is per-node
buttons plus canvas arrow-key cycling).

## Scope: §57–59, §86.5–7, §88 (milestone closure)

Draft analysis ("Review My Draft"), explicit
fragment selection, dual-panel comparison, and
the §86 Definition of Done verification plus the
§88 milestone report. §59 (Innovation Studio
send-to destinations) is satisfied in its
consent-gated form: the comparison's "Combine
into an updated prompt" puts the user-reviewed
combined text into the Ask prompt input for the
user to edit or submit — no deleted text is sent
anywhere without approval, and no send-to
Idea Generator / Collider / Forge path is built
(the combined prompt is ordinary user input from
that point on). Non-goals, unchanged: automatic
prompt rewriting without user approval,
background persistent storage of unapproved
drafts, automatic or paid provider inference
without explicit user dispatch.

## A. Deterministic analysis engine (Rust + TS mirror)

New functions in `crates/core/src/ghost_prompt.rs`
and `packages/ui/src/ghostPrompt.ts`, with
identical keyword lists, tokenizer, and rules.
The engine is pure: it reads fragments, events,
and the final text, and returns findings. It
never infers intent, never touches disk or
network, and never sees a password field.

- **Tokenizer (identical both languages).**
  Lowercase, split on whitespace, trim
  `,.;:!?()[]"'` from both ends, drop empties.
  Rust: `split_whitespace` + `trim_matches`;
  TS: the same steps.
- **Price signals.** A token is a price signal
  if it starts with `$` and the remainder
  (commas removed) parses as a number, or if it
  is a qualifier word: `cheap`, `free`, `budget`,
  `expensive`. Per event in seq order, signals
  are collected from the deleted side then the
  inserted side. If the sequence contains ≥2
  distinct signals → finding `price-changed`:
  "You changed the requested price several
  times." Evidence: the signals in order, e.g.
  `$50 → $20`. One signal, or none → no price
  finding.
- **Privacy keywords (exactly):** `privacy`,
  `confidential`, `local`, `untracked`, `sealed`.
  If any keyword appears in a deleted fragment
  or a deleted event side and is absent from the
  tokenized final text → finding
  `privacy-removed`: "You removed a requirement
  about privacy." Evidence: the keywords found.
  A keyword that returns in the final text is
  not a removal.
- **Audience terms:** `freelancers`, `enterprise`,
  `lawyers`, `startups`, `agencies`, `developers`.
  For each replace event (deleted and inserted
  both non-empty): a term on the deleted side
  and a different term on the inserted side →
  finding `audience-shift`: "You revised the
  target audience." Evidence `freelancers →
  enterprise`. A pure deletion of a term that is
  absent from the final text gives the same
  finding with evidence `freelancers →
  (removed)`. At most one finding per kind;
  evidence collects up to 5 triggers.
- **Technical terms:** `api`, `embedding`,
  `vector`, `kubernetes`, `oauth`, `websocket`,
  `sql`, `llm`, `neural`, `algorithm`. For each
  replace event: a technical term on the deleted
  side and no technical term on the inserted
  side → finding `technical-term-substituted`:
  "You replaced a technical term with a broader
  description." Evidence `API → simple`. Pure
  deletion without replacement is a removal, not
  a substitution, and yields no finding here.
- **Finding order is fixed:** price, privacy,
  audience, technical. Finding texts are the
  four §57 sentences verbatim — no other
  sentence is ever produced. A unit test asserts
  no finding text contains intent words
  (`subconscious`, `intent`, `feel`, `want`,
  `think`, `believe`, `mind`). The UI carries
  the disclaimer: "These are objective
  observations about text changes, not
  interpretations of intent."

## B. Explicit selection and approved context

- The strip's Review drafts section gains a
  "Review My Draft" subsection: the findings
  (or "No draft findings yet."), the disclaimer,
  and the verbatim question: "Would you like
  ANANSI to consider any of these earlier
  alternatives?" Below it, one checkbox per
  deleted fragment (`Include fragment: <text>`),
  held in strip state as a set of fragment seqs.
  Unchecking removes the fragment; clearing or
  stopping the session clears the selection.
- `compileApprovedContext(finalText,
  selectedFragments)` (Rust + TS mirror):
  fragments in seq order, one line each:
  `- <text> (deleted at offset <n>)`, under the
  header `--- Approved draft context (earlier
  alternatives you selected) ---`, appended to
  the final text after a blank line. Empty
  selection → the final text unchanged (no
  header). Panel B is built ONLY from selected
  fragments; unselected text is never included,
  never displayed in Panel B, and never sent.
- "Compare responses" is disabled while the
  selection is empty, with the note "Select at
  least one fragment to compare."

## C. Dual-panel comparison (`GhostPromptCompare.tsx`)

Props: `finalText`, `selectedFragments`,
`onCombinePrompt`, and an optional `dispatch`
(the Ask dispatch helper; AskAnansi passes the
real one, tests pass a stub).

- **Panel A "Final Prompt Only"**: the final
  text, exactly. **Panel B "Final Prompt +
  Approved Draft Context"**: the compiled
  approved context, exactly, with the included
  fragments listed verbatim under "Exactly what
  extra context was included."
- **Counts (honest units):** character counts
  are Unicode scalar counts; estimated tokens
  are `ceil(chars / 4)`, always labeled
  "estimated". Simulated cost delta is
  `(tokensB - tokensA) × $2.00 per 1M tokens`,
  labeled "Simulated — assumes $2.00 per 1M
  tokens", formatted to 6 decimals. No real
  provider price is claimed.
- **Responses.** By default both panels show a
  deterministic offline reference text:
  "Reference response (offline, not a model
  answer) to: <panel prompt>", labeled as a
  comparison, not evidence of anything. The
  button "Send both to Ask ANANSI (uses your
  configured endpoint and model)" dispatches
  both prompts through the existing
  `startAsk`/`pollAsk` path, sequentially, with
  the user's current Ask settings; results
  replace the reference texts and are labeled
  with the model and endpoint. This is the only
  provider path, and it runs only on that
  explicit click — never on mount, never on
  selection, never with unselected fragments.
  The e2e walk uses the offline reference path
  (no live endpoint exists in the test env, and
  no fake-provider claim is made); the dispatch
  helper is unit-tested with a fake client.
- **Feedback.** "Which answer was more useful?"
  with three buttons (Final Prompt Only / Final
  Prompt + Draft Context / Neither); the choice
  renders as "You found X more useful." and is
  local state only, never sent anywhere.
  "Combine into an updated prompt" calls
  `onCombinePrompt` with the full Panel B text;
  AskAnansi sets the prompt input from it. The
  user reviews and edits before any submit —
  nothing is auto-submitted or auto-rewritten.
- **Import qualification.** Every new or edited
  ghost-module import uses an explicit extension
  (`.ts` / `.tsx`), per the Slice 1/2 NTFS
  lesson. Zero `innerHTML` in the new files.

## D. Tests

- Rust (`cargo test -p anansi-core --lib
  ghost_prompt`): price sequence ($50 → $20),
  qualifier-only sequence, single-signal
  negative, privacy removal + privacy-returned
  negative, audience shift + audience removal,
  technical substitution + technical-kept
  negative, finding order, dedupe, empty input,
  compile format (header, lines, offsets) and
  empty selection.
- UI (`packages/ui/tests/ghost-prompt-analysis.test.tsx`,
  added to the `test:ui` list): the same rules
  in the TS mirror (including byte-identical
  finding texts), the intent-word ban, compile
  formatting, comparison static renders (Panel B
  contains exactly the selected fragment and
  NOT an unselected one; counts and simulated
  cost delta math; feedback choice renders),
  and the dispatch helper with a fake client
  (calls startAsk with exactly the two prompts,
  polls to completion, returns both answers).
- E2E (`tests/e2e/ghost-prompt-slice3.spec.ts`,
  reusing `startSession` + the Slice 1/2 enable
  flow, with whole-text `fill()` revisions for
  deterministic diffs):
  - Walk (§86 1–9 in one test): enable → type
    and revise through all four finding triggers
    → recording indicator → inspect fragments →
    review interpretations (all four sentences)
    → select fragments → compare responses
    (both panels, exact context, counts, cost
    delta, feedback choice, combine fills the
    Ask input) → stop & discard → re-enable and
    verify the review shows no fragments and no
    findings.
  - Guardrail test: select one of two fragments
    and assert the other never appears in
    Panel B or its context list.
  - Zero provider traffic on the offline path
    (fakeHits unchanged); the provider button is
    not clicked in e2e (no live endpoint).
- Full verification: Rust workspace suite
  (expect 603 + 4 known live-Ollama failures),
  strict Clippy/fmt, node `test:ui`, tsc,
  contracts `--check` (zero drift; no new DTOs
  — analysis is client-side, no server op),
  docs validation, web build, regression specs
  (ghost Slices 1–2, SOMNIA ×3, evaluations,
  integrated workflow, opportunities, SwarmTank
  ×3).

## E. Acceptance criteria

- AC-1 All four finding kinds fire on their
  triggers and stay silent on their negatives,
  byte-identically in Rust and TS.
- AC-2 No finding text claims intent; the
  disclaimer renders with the findings.
- AC-3 Panel B contains exactly the selected
  fragments; unselected text appears nowhere in
  Panel B, its context list, or any dispatch.
- AC-4 Counts are labeled units (chars,
  estimated tokens, simulated cost with its
  assumption); no real price is claimed.
- AC-5 Reference responses are labeled offline
  and non-model; provider dispatch happens only
  on the explicit button click, with the user's
  own Ask settings, and only for the two panel
  prompts.
- AC-6 Feedback choice and combine are local
  until the user submits; combine never
  auto-submits.
- AC-7 The §86 walk passes end to end, and
  step 9 proves the wipe (no fragments, no
  findings after re-enable).
- AC-8 AN-016 advances to `verified` with
  "Milestone 11 complete" in its text (the
  validator's status set has no `complete`
  value — the SOMNIA Slice 3 precedent); the
  addendum is the §88 milestone report with all
  §88 fields.
- AC-9 Checksummed ZIP delivery with
  `review.bundle`, MANIFEST, and sidecars.
