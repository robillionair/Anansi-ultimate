# M8 visualization — delivery addendum

Baseline: reviewer tip `795aab005f4bd896d17fedaeeda3140f4ea03a69`
(review branch `codex/review-m8-url-input-repair2`),
the Windows-verified URL repair 2 runtime.
Branch: `muse/m8-visualization`.
Design: `docs/handoffs/M8-visualization-design.md`,
committed before any implementation code.

## What this slice delivers

A complete project-scoped workflow for visualizing an
exact saved Collider experiment version, mounted in
the saved-experiment view (`ColliderExperimentView`,
rendered from the Projects artifact view with key and
instance scope already pinned to `artifactId` +
`version`; the view now also receives both as props,
and the visualization renders only when both are
present).

- Derivation (`packages/ui/src/
  colliderVisualization.ts`, pure, no I/O, no API
  calls, no randomness): nodes for both beams (with
  document/URL provenance where present), source
  understanding, every primitive, mapping,
  combination, feasibility item plus testability,
  and every productization field — each carrying its
  exact source text. Edges state ONLY section flow
  (beams → understanding → primitives → mappings →
  combinations → feasibility → productization,
  reusing the WP3 pipeline relations verbatim) plus
  section membership. No edge is mined from free
  text: mappings do not link to particular
  primitives, feasibility items do not link to
  particular combinations, because the saved record
  does not say which.
- Layout is a deterministic pure function of the
  scene (section hubs on a ring around a central
  collision point, item nodes orbiting their hub),
  so the same version always lays out identically.
- Selection (canvas click via hit-testing, or the
  equivalent button in the always-rendered text/list
  alternative, which is the keyboard path) reveals a
  detail panel with the node's exact source text and
  provenance: experiment artifact id, version,
  provider, model, plus the beam's document/URL
  record where applicable.
- Reduced motion renders one static frame; otherwise
  a slow orbital animation runs with a Pause/Resume
  control and moves angles only. Canvas failure falls
  back to the list alone. Particle/draw work is
  bounded by the parsed result (3–12 primitives,
  2–12 mappings, 2–8 combinations, bounded
  feasibility lists, 5 productization fields), one
  canvas, one animation loop, device pixel ratio
  capped at 2.
- Stale state: selection and animation state are
  keyed by `artifactId@version` and reset on identity
  change (the Projects view additionally remounts on
  its key). No asynchronous fetch exists in the
  visualization: derivation is synchronous from the
  already-loaded document.
- No inference: the visualization makes zero API
  calls. Opening it, selecting nodes, pausing, or
  switching versions adds no provider traffic (the
  browser workflow test asserts the fake-server hit
  count is unchanged).
- Render as data: all text reaches the screen via
  React text nodes or canvas `fillText`, never via
  `innerHTML` (a node test asserts the new files
  contain neither `innerHTML` assignment nor
  `dangerouslySetInnerHTML`). Saved experiment text
  is untrusted model output; it is displayed, never
  executed.
- No parallel persistence: the scene is derived at
  view time from the artifact body, so restart and
  backup/fresh-restore correctness ride the existing
  artifact restore path (browser-tested).

## Material decision recorded, not fabricated

The repository has no Three.js dependency and no
working package-manager path that preserves the
pinned `node_modules` invariant, so the slice renders
with Canvas 2D on the existing stack — available on
low-powered hardware with no GPU requirement, which
§65 itself demands — behind the renderer-agnostic
derived scene model. Derivation, layout, selection,
provenance, and the text alternative do not depend
on the renderer, so a Three.js/WebGL renderer can
replace the canvas renderer later without changing
them. Stated honestly: the complete visualization
workflow is delivered, but the §26 Three.js renderer
itself remains outstanding and is NOT claimed here.
The canvas renderer still honors the §26 visual
concept (two beam streams in distinct colors,
concept fragments orbiting the ring, a collision
point, recombined output nodes, section
relationships) and the §65 list. SwarmTank 3D
(AN-014) is M9 and untouched.

## Also updated

- The pipeline overview help text is now
  instance-aware: saved views point at the
  visualization below; the live panel keeps the
  "3D presentation remains unfinished" wording,
  which is still true there.
- No backend, schema, or contract change. No
  Antigravity directory touched
  (`scripts/windows-release/`,
  `tests/windows-release/`,
  `docs/reviews/windows-release/`).

## Linux evidence (sequential, idle machine)

- Node UI: 105/105 (99 prior + 6 new visualization
  tests: exact derivation counts, section-flow-only
  edges, deterministic/bounded/hit-testable layout,
  identity switching, list-alternative rendering,
  no-markup assertion).
- Browser battery: 26/26 across 22 specs (collider, document-input, url-input, visualization, instance anchors, integrated acceptance, memory acceptance, memory context, picker reliability x5, pipeline overview, project context, review anchors, review document input, review rejected selection, transfer, transfer concurrency, transfer identity, forge, forge deliverable, forge review, forge revision, backups).
- Strict Clippy (`--workspace --all-targets -- -D
  warnings`): clean. rustfmt `--check`: clean. tsc:
  clean. Contracts `--check`: exit 0. Docs
  validation: PASS, 200 files. API + web builds pass.
- full core 405/4 (only the 4 known live-Ollama semantic_workflow environment failures)
- providers+API 144/0
- No real external site was contacted; no model
  calls; all provider traffic was the synthetic local
  fake.

## Windows rerun checklist (for the independent verifier)

1. Fetch the delivered bundle; confirm the tip and
   ancestry `795aab00` (review tip) over the runtime
   base.
2. Run the node UI suite (105 tests) and the new
   `tests/e2e/collider-visualization.spec.ts`
   unchanged.
3. Run the canonical URL and document browser specs
   unchanged (closed boundaries preserved).
4. Confirm no real external site is contacted.
5. Native Tauri behavior remains out of scope.

## Still open (not closed by this slice)

- Independent Windows rerun of this slice.
- Three.js/WebGL renderer swap (see decision above).
- Windows verification of M8 Slice 4.
- M7 native desktop acceptance + preview promotion.
- §32 execution sandbox; M5 production gates.
- M8 is NOT complete after this slice.
