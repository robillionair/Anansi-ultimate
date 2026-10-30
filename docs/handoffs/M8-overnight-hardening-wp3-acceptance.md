# M8 overnight hardening — WP3 acceptance criteria

WP3 implements one bounded product slice: an accessible
overview that makes the Collider's REAL pipeline state
understandable. It is inventory-first: the remaining M8
requirements are URL fetching, document ingestion and 3D
visualization (REQUIREMENTS_MATRIX AN-013, in progress),
plus the M9 SwarmTank 3D item (AN-014, planned, not M8).
WP3 does not build any of those. 3D stays explicitly
unfinished: no visualization dependency exists in the UI
(no three.js, d3 or chart library), and adding a stack
for a visual effect would violate the assignment's
dependency constraint. The slice below is the useful
accessible overview, with a static 2D diagram whose full
equivalent is a text list.

Criteria are committed before implementation (WP2 gate
passed at `5ee51c84`: e2e reliability spec 5/5, prior
project-context spec 1/1, collider 34/34, transfer 19/19,
forge_restore 4/4, backup_restore 7/7, node/tsc/
contracts/fmt clean).

## The honest pipeline model (the slice must not invent more)

The backend makes ONE provider request per collision
(collider.rs `start`: a single `providers-stream-start`
dispatch). There are no six live processing stages, no
per-section progress and no percentages. The states that
genuinely exist, and the overview's only vocabulary, are:

- not started — no run exists yet.
- generating — the single provider stream is in flight.
  Stream characters received may be shown as a count;
  they are stream progress, never section progress, and
  must be labeled as such.
- validating — the completed output is being strictly
  re-parsed before anything renders (the panel's existing
  validating state).
- complete — the output passed validation and all six
  sections render.
- failed — the run failed (provider error, refused start
  shown as a failed run) with its error text.
- invalid — completed output failed validation, with its
  validation reason. A genuinely existing backend state,
  shown distinctly from failed, not collapsed into it.
- cancelling — cancel requested, stream still stopping.
- cancelled — the run was cancelled; nothing was saved.

Before complete, every section shares the run's state:
all six read "not started", "generating" or "validating"
together. After complete, all six read "complete"
together. Any per-section live progress, staged
animation or percentage is fabrication and is forbidden.

## The six sections and their relationships (output structure, not stages)

The overview lists the six result sections in order with
a one-line account of what each holds and how it relates
to the others, taken from the result contract itself:

1. Source understanding — the shared patterns across
   the two beams; grounds everything below.
2. Conceptual primitives — distilled concepts from that
   understanding.
3. Cross-domain mappings — connections between the
   primitives across the two domains.
4. Proposed combinations — built from the mappings
   (at least two).
5. Feasibility — hypotheses, dependencies, unknown
   costs, risks and testability for the combinations.
6. Productization — brief, architecture, roadmap, test
   plan and prototype proposal for the path forward.

The wording must present these as the structure of one
model output, never as six executed stages.

## Deliverable: `ColliderPipelineOverview`

A new component (`packages/ui/src/ColliderOverview.tsx`,
no new dependency) rendered in two places:

- In the panel, above the result area, driven by the
  panel's run state (run null / running / validating /
  complete / invalid / failed / cancelling / cancelled,
  plus partialChars and the run error).
- In the saved-experiment view, in the complete state
  with the saved result, so a reopened experiment shows
  the same overview and section navigation.

It renders:

- A `role="status"` line naming the current pipeline
  state in plain words, plus the error text when failed
  or invalid, plus "stream characters received" only
  while generating (labeled as stream progress).
- A navigation list of the six sections. When complete,
  each entry is a link to that section's existing
  heading id in the result view
  (`collider-understanding-heading`,
  `collider-primitives-heading`,
  `collider-mappings-heading`,
  `collider-combinations-heading`,
  `collider-feasibility-heading`,
  `collider-productization-heading`) with its status
  ("complete"). Before complete, entries are plain text
  with the shared run state — never dead links to
  sections that do not exist yet.
- A static SVG flow diagram of the six sections in
  order (boxes and arrows, no animation, no scripted
  motion), marked as an image with a summary label.
- Directly beside it, the same relationships as an
  ordered text list: the list is the full equivalent,
  not a caption. Either presentation alone conveys the
  whole picture.
- A provenance link to the disclosure section
  (`providers-disclosure-title`) whenever a disclosure
  exists, and a saved line naming the artifact id once
  the collision is saved.

## Accessibility

- Keyboard: navigation is native links and buttons in
  document order (Tab / Enter); no custom key handling
  that could trap or reorder focus. Focus order is
  overview first, then result sections, then disclosure.
- Reduced motion: the overview contains no animation,
  transition or auto-moving content in any mode, so it
  complies under both the workspace reduced-motion
  setting and the system preference without a branch.
- Screen readers: the state line uses `role="status"`,
  the section list is a real list, the diagram is a
  single labeled image, and the text list carries the
  complete relationships.
- No generated HTML/JS execution, no iframe preview,
  no shell execution, no deployment, no sandbox work.

## Non-goals for WP3

- No 3D presentation (retained as unfinished, with the
  reason: no existing dependency, M9 owns SwarmTank 3D).
- No URL fetching, no document ingestion.
- No per-section progress, percentages, staged
  animation or inferred timing.
- No change to what a collision sends, saves or
  restores; the overview is read-only state display
  plus anchor navigation.

## Focused acceptance gate for WP3

- New node tests pass: state mapping for every genuine
  backend state; static markup for each state (status
  text, link-vs-text entries, diagram present with no
  animation attributes, text list complete); saved-view
  overview renders complete with links.
- New e2e spec passes sequentially on the real API with
  a synthetic provider: not-started overview before any
  run; generating state during a slow stream; complete
  overview with working section links after a run;
  keyboard Tab through the overview links and Enter
  navigating to a section; saved view shows the
  overview with links.
- Pre-existing collider / transfer / forge_restore /
  backup_restore / node suites pass unchanged (counts
  recorded, no assertion weakened); prior collider
  e2e specs keep passing.
- Strict Clippy, fmt, tsc, contracts check clean.
- The WP3 record (this file, updated with before/after
  evidence) is committed before WP4 begins.
