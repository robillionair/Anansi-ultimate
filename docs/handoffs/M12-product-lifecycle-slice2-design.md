# M12 Product Lifecycle Slice 2 — Design: Launch Studio Marketing Copy & Asset Bundler

## Baseline

Branch `muse/m12-product-lifecycle-slice2` from the independently Windows-verified
Slice 1 review tip `27c5754ac` (`codex/review-m12-product-lifecycle-slice1`).
That tip carries, in order: the user's README update `59ea5bfab`, the reviewer's
NTFS import qualification `3ff95281f` (the Slice 1 delivery's
`./PipelineNavigator` import was extensionless; the reviewer fixed it), and the
review record `27c5754ac`. Slice 1 verdict: tsc 0, contracts 0 drift, docs 240
in the report body / 241 with the report, UI 198/198, Rust 616 + 4 known
live-Ollama skips, Playwright 24/24. AN-012 stays `in progress`.

Reviewer wording NOT adopted as claims: the Slice 1 review report describes a
"6-stage" pipeline with a "Define" stage and `opportunity_set` / `nightmare`
artifact types. The implementation is 7 stages (`discover`, `invent`, `build`,
`simulate`, `pressure_test`, `refine`, `launch`); ideas register as `idea`,
SOMNIA evaluations as `somnia-evaluation`; statuses are Complete / Current /
Available / Awaiting Preceding Stage.

## Scope (§61, §85, as bounded by the assignment)

Slice 2 builds the Launch Studio: a deterministic marketing-copy generator over
four channels, plus a deterministic release-asset bundle download. It is the
Launch stage's tool; it does not change the pipeline derivation from Slice 1.

In scope:

1. Deterministic rule-based marketing copy across exactly four channels:
   - Hacker News Show HN pitch (terse title + problem/solution + technical
     differentiators).
   - Product Hunt tagline and description (feature bullets + value proposition).
   - X release thread (5-post structured sequence: hook, problem,
     architecture / local-first guarantee, feature breakdown, call to action).
   - Technical release notes / changelog entry (markdown summary of
     deliverables, capabilities, and verification evidence).
2. A deterministic ZIP bundle download assembling the selected Forge
   deliverable package and specification, the SwarmTank simulation summary and
   objection matrix, the SOMNIA pressure-test notes, and the generated
   marketing copy.
3. `LaunchStudio.tsx`, mounted in `Projects.tsx` when the Launch stage is
   selected (navigator click). Preview cards per channel with copy-to-clipboard
   buttons and markdown source tabs; a "Download Launch Bundle (.zip)" action.
4. Provider generation ONLY on explicit user action, with full prompt preview
   and cost visibility before any provider call (see §74 note below).

Out of scope (stated, not claimed):

- The rest of §61's list — landing-page copy, user onboarding, documentation,
  basic SEO plan, demo script, sales presentation outline, deployment
  checklist, post-launch experiment plan — remains deferred. Slice 2 delivers
  the assignment's four channels, not all of §61.
- Connected web research for competitor information (§61 permits it only "if
  authorized"): not authorized here, so no web research, no competitor claims,
  no cited sources in the copy.
- Domain registration, payments, live deployment, and publishing (§61 requires
  explicit user approval for each): none of these are built or triggered.
- No new server op, no new artifact type, no schema change. The bundle is a
  download only; it is NOT registered as an artifact. Launch Complete therefore
  still requires manual `launch-material` registration, exactly as in Slice 1.

## §74 citation note (recorded discrepancy)

The assignment cites "§74" for preview and budget approval. In MASTER_SPEC,
§74 is DATA PRIVACY, not a budget-approval section. The requirement's intent —
no surprise provider spend, user sees the cost before approving — is
implemented as specified below, and §74 itself is respected: prompts are built
only from project artifacts the user explicitly selected in the studio, never
from sealed or private records, and nothing is sent anywhere before the user
approves. The studio cannot display the live configured provider budget
because no client method exists to read the provider-config budget
(`budget.maxRequests` / `budget.maxTotalTokens` in Providers); adding one
would be a new server op, out of scope. The existing Ask dispatch path enforces
the configured budget server-side, and the approval panel shows the estimated
input tokens so the user can compare against their own Providers settings
before approving.

## Evidence model

The generator consumes parsed project artifacts, never free text typed into the
studio. Parsing is strict (the established pattern: a body that does not parse
never renders):

- Concept (`concept`, `parseTransferConceptBody`): `selectedCombination.name`
  (≤120 chars), `selectedCombination.description`, `approvedBrief`,
  `productizationBrief`, `assumptions`, `unknowns`, optional
  `initialSpecification`. REQUIRED for generation.
- Specification (`specification`, `parseProductSpecBody`): `spec.title`,
  `spec.summary`, `spec.purpose`, `spec.intendedUsers`, `spec.features[]`,
  `spec.pages[]`, `spec.nonGoals[]`, `spec.dataAndState?`,
  `spec.acceptanceCriteria[]`. Optional; when present it supplies the title,
  summary, and feature bullets.
- Simulation (`simulation`, `parseSwarmtankRunBody`): `input.description`,
  `input.pricing`, `input.priceAmount`, `input.segment`, `input.industry`,
  `input.features[]`, `input.valueProposition`, plus derived report numbers
  (total, converts, confused, rejects, conversion rate, average willingness to
  pay, top objections, top confusion points — same derivation as
  `deriveLocalReport` in Swarmtank.tsx). REQUIRED unless a SOMNIA evaluation
  is present.
- SOMNIA evaluation (`somnia-evaluation`, `parseSomniaEvaluationDoc`):
  `input.scenarios[]` (id, nightmare class, instruction), `results[]`
  (scenarioId → Complete / Refuse / Abort), `metrics` (totals, simulated tool
  calls, violations by category), plus the review-only consolidation proposals
  from `deriveConsolidationProposals`. REQUIRED unless a simulation is present.
- Optional bundle inclusions (bodies included verbatim, not parsed for copy):
  `source-package`, `deliverable`, `scenario` (SwarmTank scenario branch).

Generation gate: the Generate button is enabled only when a concept parses AND
(a simulation parses OR a SOMNIA evaluation parses). Otherwise the studio
lists exactly what is missing. This matches the assignment's "concept +
simulation/pressure-test evidence" gate.

## Copy derivation rules (no invented claims)

- Product name: `spec.title` if a specification is selected, else the concept
  name. Problem: the concept description, verbatim. Solution: `spec.summary`
  if present, else `productizationBrief`. Audience: `spec.intendedUsers` if
  present, else the simulation `input.segment` if present, else omitted.
- Features: `spec.features` verbatim when a specification is present, else the
  simulation `input.features` verbatim. Value proposition: simulation
  `input.valueProposition` when present, else `spec.purpose`.
- Pricing: only the simulation `input.pricing` / `priceAmount` when present.
  No price is stated when absent. No discount, sale, or revenue claim ever.
- Local-first guarantee (X post 3 and Show HN differentiators): included ONLY
  when `/local|offline|on-device|no server/i` matches the specification text
  (summary, purpose, features, dataAndState). When it does not match, post 3
  describes the architecture from the specification (pages, data and state)
  and states the prototype status instead of claiming a guarantee. The Forge
  template being a static web app is never, by itself, presented as proof the
  launched product is local-first.
- Simulation numbers appear ONLY when a simulation is selected, and are always
  labeled simulated: conversion rate, Convert / Confused / Reject counts,
  average willingness to pay, top objections (max 5), top confusion points
  (max 5). No other numbers appear in the copy. No testimonials, reviews,
  sales, or conversion metrics are invented (§61).
- SOMNIA notes appear ONLY when an evaluation is selected: scenario classes
  tested, Complete / Refuse / Abort counts, violations by category, and the
  derived consolidation proposals labeled as review-only proposals — explicitly
  not applied changes and not evidence that anything was fixed.
- Release notes carry a Status section in every generation: "Prototype for
  review; not executed, not deployed." This mirrors the Forge deliverable's
  all-false status and prevents the copy from implying a shipped product.
- X thread: exactly 5 posts in the assignment's order (hook, problem,
  architecture / local-first guarantee, feature breakdown, call to action).
  Each post is capped at 280 characters via a `fit()` helper that truncates
  inserted fragments with an ellipsis. The 280 bound is unit-tested, including
  a long product-name case.
- Determinism: no dates, no clocks, no randomness, fixed section order and
  fixed wording templates. Generating twice from the same selection yields
  byte-identical channel texts. (Dates would break determinism, so the copy
  and the bundle carry artifact versions and content-hash prefixes instead.)

## Provider path (gated, never the default)

- Default generation is deterministic and needs no approval. The provider path
  is a separate, clearly labeled section: "Generate a provider draft".
- Clicking it first shows the EXACT prompt that would be sent (built from the
  same selected evidence), the estimated input tokens (`ceil(chars / 4)`,
  labeled "estimated", same estimator as GHOST-PROMPT comparison), and a note
  that the configured provider budget is enforced by the Ask dispatch path and
  should be checked in Providers. Nothing is sent at this point.
- Only the explicit "Approve and send to Ask ANANSI" button dispatches, via
  the existing `startAsk` / `pollAsk` client methods (same shape as the
  GHOST-PROMPT comparison dispatch seam: an `askOnce` function injected for
  tests, a recording fake in unit tests). Results are labeled "Provider draft"
  and never overwrite the deterministic copy.
- Bundle inclusion of the provider draft is opt-in and unchecked by default:
  when checked, the draft is added as `marketing/provider-draft.md` and the
  manifest records `providerDraft: true` plus the provider/model labels the
  dispatch reported. The deterministic channels are always included.

## Bundle format (`launchBundle.ts`)

Pure client-side ZIP builder. No dependency is added; the builder is ~150 lines
of explicit ZIP construction, unit-tested against a known-answer CRC vector.

- Stored entries only (method 0, no compression): simpler, and byte output is
  fully determined by entry bytes. Tradeoff stated: larger files than a
  compressed archive; launch bundles are small text/JSON, so this is fine.
- Fixed DOS timestamp 1980-01-01 00:00:00 on every entry (no build-time clock
  in the bytes), UTF-8 filename flag, CRC32 (IEEE, table-driven) per entry,
  central directory + end-of-central-directory record.
- Entry order sorted by path, regardless of selection order, so the same
  selection always yields the same bytes.
- Contents, and nothing else (zero unstated files):
  - `marketing/show-hn.md`, `marketing/product-hunt.md`,
    `marketing/x-thread.md`, `marketing/release-notes.md` (the four channels).
  - `marketing/provider-draft.md` only when the user opted the provider draft
    into the bundle.
  - `swarmtank-summary.md` only when a simulation is selected: totals,
    conversion rate, objection matrix (all objections with counts, not just
    top 5), confusion points, segment breakdown.
  - `somnia-pressure-test-notes.md` only when an evaluation is selected:
    scenario classes, decisions, violations, review-only proposals.
  - `artifacts/<type>-<artifactId>-v<version>.json`: the verbatim bodies of
    every artifact the user checked for inclusion. Source-package files are
    NOT re-expanded (the Forge export already serves that); the deliverable
    body's pins reference them.
  - `LAUNCH-MANIFEST.json`: format `anansi-launch-bundle` v1, generator
    (`deterministic-rule-based` + module version note, or `provider-assisted`
    when a draft is included), the channel list, and for each included
    artifact its id, version, and content-hash prefix (first 16 hex chars;
    full hashes stay in the registry, not the bundle).
- Download via Blob + anchor, fixed filename `anansi-launch-bundle.zip`
  (a fixed name avoids any slug-injection surface from product names).
- Relationship to the Forge export (M7 Slice 3): the Forge export is a
  deterministic USTAR archive built server-side. The launch bundle mirrors its
  PATTERN — deterministic bytes, a version/hash manifest, portable relative
  paths, zero unstated files — but is a stored ZIP built client-side, because
  every input is already readable by the client and no new server op is
  wanted. The formats differ; the guarantees rhyme. This is stated, not hidden.

## UI (`LaunchStudio.tsx`)

Props: the project's artifact records, a `readArtifact(artifactId, version)`
function (backed by `client.readProjectArtifact`), the Ask `client` for the
provider path, and `notify`.

- Artifact picker: one checkbox row per record of type `concept`,
  `specification`, `source-package`, `deliverable`, `simulation`, `scenario`,
  `somnia-evaluation` (latest record per artifact; versions are not
  re-selected — the studio reads the record's current version). All checked by
  default. Unchecking an artifact removes it from generation inputs AND from
  the bundle.
- On Generate: bodies are read, parsed strictly, and the gate is applied
  (concept + simulation-or-evaluation, else the missing list). Generated
  channels render as preview cards, one per channel, each with a
  copy-to-clipboard button (the `navigator.clipboard.writeText` pattern from
  ColliderArtifactViewer, with the same manual-select fallback notice) and two
  tabs: Preview (paragraph-split plain text, no markup interpretation) and
  Source (raw markdown in `<pre>`; the source is authoritative — no full
  markdown renderer is built, stated plainly).
- Provider draft section as specified above, with its own preview card.
- "Download Launch Bundle (.zip)" button, enabled once copy has been
  generated (deterministic copy suffices; a provider draft is not required).
  After download, `notify` reports the filename and entry count.
- Accessibility: the studio section is `<section
  aria-labelledby="project-launch-heading">`; channel cards are `<article>`
  elements with headings; checkboxes carry the artifact title and type.

## Mounting (Projects.tsx + PipelineNavigator.tsx)

- `Projects.tsx` holds `launchStudioOpen` state (default closed) and renders
  the LaunchStudio section after the Artifacts section, only when open.
  This is the assignment's "mounted when Launch stage is selected or active":
  selection is the navigator click.
- `PipelineNavigator.tsx` gains an `onSelectLaunch` prop (same defensive shape
  as `onNavigateToAsk`: the button is disabled without a callback). The Launch
  row changes from the Slice 1 disabled button to:
  - Awaiting Preceding Stage: "Jump to Launch Studio" (the override, consistent
    with every other Awaiting row) — opens the studio, which shows the
    missing-evidence note.
  - Otherwise: "Open Launch Studio" — opens the studio and scrolls to
    `#project-launch-heading`.
  The Slice 1 "Launch Studio arrives in Slice 2." note is removed; the Launch
  tool now exists.
- Import discipline (NTFS lesson, Slice 1 review): `LaunchStudio.tsx` is
  imported with its explicit extension, and the new pure modules are imported
  with explicit `.ts` extensions everywhere, including the test file.

## Files

New:

- `packages/ui/src/launchCopy.ts` — evidence type, derivation rules, the four
  channel generators, X `fit()` helper, provider-prompt builder, provider
  dispatch seam (`runLaunchProviderDraft(askOnce, prompt)`).
- `packages/ui/src/launchBundle.ts` — `crc32`, `buildLaunchBundle(files)`,
  manifest builder.
- `packages/ui/src/LaunchStudio.tsx` — the studio component.
- `packages/ui/tests/launch-studio.test.tsx` — generator, bundler, and
  component-rendering tests (added to the `test:ui` list in `package.json`).
- `tests/e2e/product-lifecycle-slice2.spec.ts` — see below.

Modified:

- `packages/ui/src/PipelineNavigator.tsx` — Launch row becomes a working
  button via `onSelectLaunch`; Slice 1 note removed.
- `packages/ui/src/Projects.tsx` — `launchStudioOpen` state, LaunchStudio
  section with `#project-launch-heading`, `onSelectLaunch` plumbing.
- `package.json` — `test:ui` list gains the new test file.
- Docs: this design doc, the slice addendum, `docs/BUILD_STATUS.md`,
  `docs/REQUIREMENTS_MATRIX.md` (AN-012 stays `in progress` + Slice 2
  sentence), `docs/RECOVERY.md`, `plans/PHASE-012-product-lifecycle.md`.

No Rust change, no contracts change, no schema change, no new server op.

## Tests

Unit (`launch-studio.test.tsx`):

- Generator fixtures: (a) concept + specification + simulation + SOMNIA
  evaluation; (b) concept + simulation only; (c) concept + SOMNIA only;
  (d) concept only (gate refuses, missing list names simulation/evaluation);
  (e) no concept (gate refuses).
- Channel assertions: Show HN title/problem/solution/differentiators present;
  Product Hunt tagline + feature bullets verbatim; X thread exactly 5 posts in
  the required order, each ≤ 280 chars, including a long-name fixture;
  release notes contain the deliverables, capabilities, verification evidence,
  and the "Prototype for review; not executed, not deployed." status line.
- No-invention assertions: with a simulation present, the copy contains the
  run's conversion rate and top objection and no other percentages; with no
  simulation, no conversion rate appears; no "testimonial", "review" (as a
  customer review), or revenue wording appears in any fixture.
- Local-first rule: guarantee sentence present when the spec mentions local
  operation, absent otherwise (architecture post falls back to pages/data).
- Determinism: generating twice from the same selection is byte-identical.
- Bundler: CRC32("123456789") = `0xCBF43926` (standard check value);
  `buildLaunchBundle` twice over the same files is byte-identical; entry
  names are sorted; the manifest lists exactly the included artifacts; a
  provider-draft file appears only when opted in.
- Provider seam: a recording fake `askOnce` receives the exact previewed
  prompt, and only after the approval action (component test covers the
  approval gating via props/state where static markup allows; the full click
  flow is covered by e2e).
- Component rendering (static markup): channel cards, copy buttons, source
  tabs, disabled Generate with the missing-evidence note when the gate fails.

E2E (`product-lifecycle-slice2.spec.ts`, shared lease, one spec per run):

- Test 1: fresh project → pipeline Launch row shows Awaiting → "Jump to
  Launch Studio" opens the studio → missing-evidence note names the concept
  and simulation/evaluation requirement → Generate disabled.
- Test 2: register a valid concept body via the manual Artifacts form, then
  run a real SwarmTank evaluation (deterministic, no provider traffic) and
  save the run → pipeline Launch becomes Available → "Open Launch Studio" →
  Generate → all four channel cards render with the concept name and the
  run's conversion rate → Download → assert the download starts with the
  `PK\x03\x04` signature and contains `LAUNCH-MANIFEST.json` and
  `marketing/release-notes.md` entry names, and `fakeHits() === 0`
  (zero provider traffic on the deterministic path).

## Verification battery (same as Slice 1, plus the new spec)

- `tsc --noEmit` 0 errors; `node scripts/contracts.mjs --check` 0 drift;
  `node scripts/validate-docs.mjs` PASS.
- `npm run test:ui` (expect 198 + new launch-studio tests, all passing).
- Web build clean (rebuild before e2e; preview serves `apps/web/dist`).
- Rust workspace unchanged by this slice; run the pipeline lib tests + Clippy
  / fmt checks to confirm no drift (no Rust files touched).
- Playwright: slice2 spec 2/2, then the full regression battery in a single
  run (slice1 + slice2 + ghost 1–3 + somnia 1–3 + swarmtank + the rest).

## Open questions / decisions the owner may revisit

- Whether the rest of §61 (landing page, onboarding, SEO, demo script, sales
  outline, deployment checklist, experiments) belongs in a later Launch Studio
  slice or stays deferred.
- Whether the bundle should one day be registered as a `launch-material`
  artifact (which would let Launch reach Complete from the studio); Slice 2
  deliberately does not, to stay inside "download-only".
