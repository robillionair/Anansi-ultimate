# M10 SOMNIA Slice 3 — Design (before code)

## Baseline

Branch `muse/m10-somnia-slice3` from owner tip `b9f9edfc`
(Slice 2 delivery). The assignment also names Windows
review tip `4ca4a146`, which is not present locally and
arrived with no review ZIP; the assignment authorizes
`b9f9edfc`, so Slice 3 builds on it. The reviewer's
Slice 2 verdict is recorded in the Slice 3 closure docs:
tsc 0, contracts 0 drift, docs 226, UI 165/165, Rust
somnia 18/18, workspace 589 + 4 offline-Ollama skips,
Vite 2.09s, browser battery 14/14 under shared lease.
(Slice 2's own tree had 224 docs; the review tip adds
two reviewer docs. Slice 3's docs count is measured
against its own tree.)

## Scope: §50 dashboard, §51 Forge QA bridge, §87 DoD, §88 report

Slice 3 closes Milestone 10. It adds no server op, no
artifact type, no schema change, no release-tooling edit,
no scheduling, no lesson promotion, no live memory
mutation, no provider call. Everything new is either a
pure derivation over saved evaluation docs (dashboard,
proposals, comparison — the Slice 2 precedent) or
client-side scenario generation evaluated through the
existing `somnia-evaluate` / `somnia-save-evaluation`
ops (the nightmare-generator precedent).

## A. Dashboard (§50)

A new `SOMNIA dashboard` section in `Somnia.tsx`,
rendered from panel state. It loads only on an explicit
user action ("Load SOMNIA dashboard"): for each saved
evaluation record (type `somnia-evaluation`, from the
panel's `artifacts` prop) it reads the doc via
`readProjectArtifact` and pairs it with its record, so
the §9 record timestamp is available alongside the doc.
No client calls on mount are added; re-clicking reloads.
The aggregates are computed by a pure function,
`computeSomniaDashboard(pairs)`, mirrored in Rust
(`compute_dashboard`) and pinned by fixed-doc tests in
both suites.

Definitions (binding; the e2e asserts the computed
values for its fixture, not hand-waved text):

- **Ordering.** Pairs are sorted by record timestamp
  ascending (oldest first); ties keep list order. The
  earliest pair is the baseline; the latest pair is the
  latest run. All "consecutive" comparisons use this
  order.
- **Recent failure rate.** (Refuse + Abort scenarios
  across ALL saved runs) / (total scenarios across ALL
  saved runs), percent with one decimal. No time-window
  filter is applied: the project's saved history is the
  history, and the dashboard says so. Empty history
  renders "No saved evaluations yet" and no numbers.
- **Context efficiency.** Calls per scenario for the
  latest run divided by calls per scenario for the
  earliest (baseline) run, shown as "N.NN× baseline
  calls per scenario" plus both raw rates. A ratio above
  1 means the latest run made MORE calls per scenario
  than baseline; the dashboard never calls this an
  improvement. Baseline rate 0 renders "n/a (baseline
  made no simulated calls)".
- **Duplicate evidence indicators.** Two counts over all
  saved runs: (1) evidence ids appearing in more than
  one scenario; (2) retrieved sources appearing in more
  than one retrieved record. Counts are of distinct ids /
  sources, not of occurrences.
- **Pending proposals.** Proposals derived from the
  LATEST run (Slice 2 derivation, unchanged) minus
  those marked reviewed or dismissed in panel state
  (see below). The dashboard shows the pending count
  and the latest run's proposal total.
- **Evaluation coverage.** Distinct §45 failure classes
  evidenced by any saved evaluation, out of 7, as
  "N of 7 (P%)". Class mapping from violation
  categories (the only class signal a saved evaluation
  carries): instruction-conflict → conflicting
  instructions; missing-evidence → missing evidence;
  tool-error → simulated tool error; tool-timeout →
  simulated tool timeout; untrusted-instruction →
  untrusted retrieved instruction; constraint-violated
  → cost-limit exhaustion (the max-tool-calls
  mechanism). The seventh class, privacy-boundary
  challenge, CANNOT appear in a saved evaluation:
  sealed inputs are refused before evaluation, so no
  evaluation doc ever evidences it. The dashboard
  states this explicitly instead of silently scoring
  6/7 as a failure. Coverage is therefore at most 6
  of 7 from saved evaluations alone.
- **Nightmare runs history.** Latest run: record
  timestamp + doc runId. Total scenarios executed:
  sum over saved runs. Failure categories: per-category
  violation counts summed over saved runs, sorted by
  category. Improvements proposed: sum of per-run
  proposal counts over saved runs. Regressions: for
  each consecutive pair (older → newer), a regression
  is recorded when the newer run's failure count
  (Refuse + Abort) is strictly greater than the older
  run's; rendered as "runId short → runId short:
  failures A → B". No anthropomorphic claims: no
  "smarter", no improvement claim without the numbers
  beside it (§50).

## B. Proposal review state (§87.7)

Slice 2 has a local-only "Mark as reviewed" toggle
(`reviewed: string[]`). Slice 3 adds a sibling
local-only "Dismiss" toggle (`dismissed: string[]`),
both React state, never sent, never persisted, never
mutating any memory or policy. The proposals section
text is updated to say reviewed and dismissed are
view-local only. The dashboard pending count uses
`reviewed ∪ dismissed`. This satisfies §87.7's
"approve or reject" as review acknowledgement, the
same reading Slice 2 established; live memory remains
unmutated by construction (there is still no apply
path and no mutation op).

## C. Primary action triggers (§50)

Three buttons in the dashboard section:

- "Run Nightmare Test" → focuses the nightmare
  generator section.
- "Review Improvement Proposals" → sets the viewed
  evaluation to the latest run (so the proposals
  section exists) and focuses it.
- "View Results / Compare" → focuses the comparison
  section.

Focus is implemented with a `focusTarget` state
(`'nightmares' | 'proposals' | 'comparison' | null`)
and an effect that, once the target section is
rendered, calls `scrollIntoView` (guarded: jsdom
lacks it) and `focus()` on the section element, which
carries `tabIndex={-1}`. The e2e asserts
`document.activeElement` is the target section after
each click. The generator and comparison sections are
always rendered; proposals requires the viewed run,
hence the state set before focusing.

## D. Forge QA bridge (§51)

A new `SOMNIA Forge QA` section in `Somnia.tsx`:

- "Load Forge concepts" button (explicit user action,
  no mount call) lists registered Forge artifacts in
  this project via `listProjectArtifacts`, filtered to
  types `concept` and `deliverable`, showing title and
  version. An empty project renders "No Forge concepts
  or deliverables registered in this project yet."
- Per item, "Run SOMNIA QA check" generates seven QA
  scenarios from the concept title (pure builder
  `forgeQaScenarios(conceptTitle)`, mirrored in Rust
  as `forge_qa_scenarios`, pinned by tests), one per
  §51 condition, each constructed to exercise the
  existing evaluator:
  - API failure → tool events ok/error → Abort,
    tool-error, 1 prior call.
  - Incorrect user input → instructionConflict →
    Refuse, instruction-conflict.
  - Timeout condition → tool events ok/timeout →
    Abort, tool-timeout, 1 prior call.
  - Unexpected dependency → one untrusted retrieved
    record → Refuse, untrusted-instruction, 0 calls.
  - Agent tool failure → tool events error → Abort,
    tool-error, 0 prior calls.
  - Recovery scenario → tool events ok/ok →
    Complete, 2 calls.
  - Memory/context failure → required evidence id
    absent → Refuse, missing-evidence.
  Scenario ids are `forge-qa-<condition>`; titles are
  `Forge QA [<concept title>]: <condition>`; the seed
  is `somnia-forge-qa-<artifactId>` (deterministic per
  concept, within the 1–128 seed bound).
- The generated input fills the replay textarea and
  seed, and the panel previews it through the existing
  preview/run/save flow: the user runs it with "Run
  evaluation" and saves it with "Save SOMNIA
  evaluation". The saved evaluation's title derives
  from its first scenario title, so the QA summary is
  titled with the Forge concept title and lives in the
  same project's saved evaluations; it is NOT a new
  Forge artifact version and does not mutate the Forge
  concept (no new artifact type, no Forge write path).
- The section prominently displays the §51 honesty
  disclosure, verbatim: "SOMNIA augments QA; it does
  not replace ordinary automated software and security
  testing." It also states that ordinary automated
  software and security testing remain mandatory.
- Export: after a QA run, "Export QA summary JSON"
  and "Export QA summary Markdown" download the
  evaluation doc (JSON) and a fixed-template Markdown
  summary (title, totals, per-scenario decision and
  reason) via Blob download, following the comparison
  export precedent (no timestamps, no wall time).

## E. §87 end-to-end coverage

The Slice 3 spec walks all ten steps in one project:
(1) panel mounted for the project; (2) nightmare
generation started; (3) scenarios visible in preview;
(4) results reviewed; (5) recurring weaknesses visible
as derived proposals; (6) proposals reviewed; (7) one
proposal marked reviewed and one dismissed (UI state
only — the spec asserts no new artifact and no memory
change by checking the saved-evaluation count is
unchanged); (8) re-run via the generator; (9) compare
before/after; (10) backup/restore into a fresh
workspace, then dashboard aggregates identical and
both saved evaluations present. Step 10's "restore
the previous version" is the saved-evaluation history
surviving restore (the Slice 1/2 precedent); version
rollback of a single evaluation is not claimed,
because saves create new artifacts rather than
versions of one artifact.

## F. §88 milestone report

The Slice 3 addendum carries the §88 report for
Milestone 10: features built, files added/modified,
architecture decisions, database migrations (none),
tests passed/failed/not run, security issues (none
found; sealed fail-closed preserved), live-provider
verification (none — zero provider traffic by
design), known limitations, last verified commit,
backup status, current runnable command, next
milestone (none commissioned; the milestone is
closed, not extended).

## Acceptance criteria

- AC-1 Dashboard aggregates match the binding
  definitions on a fixed two-run fixture in Rust and
  TS tests (identical numbers).
- AC-2 Dashboard loads only on explicit action, from
  saved local artifacts; zero provider traffic in the
  spec (fakeHits unchanged).
- AC-3 Forge QA scenarios are deterministic per
  concept, evaluate to the intended decisions via the
  unchanged evaluator, save through the existing op,
  and export deterministically.
- AC-4 The §51 disclosure renders verbatim in the
  Forge QA section.
- AC-5 Proposal reviewed/dismissed state is local
  only: no new artifact, no memory write, dashboard
  pending count updates accordingly.
- AC-6 All three action triggers move focus to their
  target sections (asserted via activeElement).
- AC-7 §87 steps 1–10 walk end to end in the Slice 3
  spec, including backup/restore survival with
  identical dashboard aggregates.
- AC-8 Full verification green except the known
  pre-existing search real-semantic failure; no new
  artifact type, op, schema change, or release-tooling
  edit; contracts drift-free; docs validation passing.
- AC-9 Checksummed ZIP delivery with MANIFEST and
  Windows rerun checklist; AN-015 advanced to
  `complete` for Milestone 10 (Slice 3 scope itself
  still awaits its own Windows rerun — the matrix row
  says so explicitly, and Linux evidence is never
  presented as Windows acceptance).
