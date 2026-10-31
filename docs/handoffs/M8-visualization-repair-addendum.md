# M8 visualization repair — delivery addendum

Baseline: reviewer tip `5a050f18ff2a4cc424a5f01a720026ba95722587`
(review branch `codex/review-m8-visualization`),
one docs-only reviewer commit over the visualization
runtime `93cf77605`.
Branch: `muse/m8-visualization-repair`.
Design: `docs/handoffs/M8-visualization-repair-design.md`,
committed before any repair code.

## VIS-01 (P1): case-insensitive module resolution

The helper `packages/ui/src/colliderVisualization.ts`
and the component `packages/ui/src/
ColliderVisualization.tsx` differed only in case, so
on a case-insensitive filesystem the extensionless
import resolved to the helper before the component
and TypeScript, Vite, and the UI suite all failed on
Windows (TS2305, TS1261/TS1149, MISSING_EXPORT,
43 pass with 3 failed test-file loads instead of
105 cases). Linux never surfaced it.

Repair: the helper is renamed via `git mv` to the
genuinely distinct base name
`colliderVisualizationScene.ts`; the two imports
that named it are updated
(`ColliderVisualization.tsx`, the node test
including its no-markup file-path assertion). The
component import in `Collider.tsx` is unchanged —
it already names the component. No casing checks
disabled, no module-resolution settings altered, no
test weakened, no extension forced at a single call
site. Public behavior is unchanged; only the module
path changes.

Verification beyond the green suites: a folded-name
scan of all tracked paths (`git ls-files`, folded to
lowercase, duplicates listed) returns empty, and the
tracked `packages/ui/src/` names contain no
case-only pair. Windows case-resolution itself cannot
be reproduced on this case-sensitive Linux
filesystem; the scan plus the reviewer's Windows
rerun is the honest evidence for that half.

## VIS-02 (P2): canvas label contrast

`drawScene` filled the canvas with `#0b2545` and
drew node labels with the exact same `#0b2545` as
`fillText` color — invisible labels. Repair: labels
are now restrained white (`#f5f5f4`, the §26
highlight) over a semi-opaque dark backing rect
sized by `measureText`, so they stay legible over
nodes, edges, and the navy field.

Verification is actual rendered screenshots, not
only list text: the canonical spec captures a paused
normal-DPI frame and a reduced-motion frame, and a
new `collider-visualization-screenshots.spec.ts`
(deviceScaleFactor 2) captures paused and
reduced-motion high-DPI frames. All four were
inspected as images: labels read light-on-dark in
every view ("Beam A", "Beam B", "Source
understand…", "Prototype proposal", "Test plan",
"Concept brief", "Roadmap", "Architecture",
"Testability", "Feasibility", and the rest), and
the selection highlight renders. Screenshots are
shipped in `evidence/screenshots/` and are opt-in
(`VIZ_SHOT_DIR`) so the specs stay green without
an output directory.

Honest limitation observed in the screenshots, not
hidden: in the densest clusters, item labels can
overlap neighboring nodes (orbit radius 52 with
hub radius 26). Labels remain readable and the list
alternative carries every node's exact text, so
this is crowding, not invisibility — but it is not
claimed as perfect layout. A layout-spacing pass
would be a separate change, not part of this
contrast repair.

## Scope guard

Canvas/list remains the accessible fallback; it is
not erased. Three.js and the remaining §26
requirements (Three.js renderer, toroidal particle
streams, instanced rendering, GPU-efficient
materials, adaptive quality) stay recorded as
unfinished in the design, this addendum,
BUILD_STATUS, and plan copy. This repair does NOT
claim M8 complete. No backend, schema, or contract
change. Antigravity directories untouched. No
promotion, pushes, or CI.

## Linux evidence (sequential, idle machine)

- Node UI: 105/105 (all cases load and pass; the
  reviewer's 43-pass/3-file-load-failure mode is
  gone).
- TypeScript: exit 0 with casing checks intact.
  Vite web build: exit 0. Contracts `--check`:
  exit 0. Docs validation: PASS, 202 files. API +
  web builds pass. rustfmt/Clippy: no Rust files
  changed; prior broad totals stand (full core
  405/4, only the 4 known live-Ollama failures;
  providers+API 144/0).
- Focused browser battery, 5/5 specs:
  collider-visualization, collider-visualization-
  screenshots (high DPI), collider-document-input,
  collider-url-input, collider-pipeline-overview —
  all assertions preserved (the canonical workflow
  spec is unchanged except opt-in screenshot steps;
  the document/URL specs mount the renamed
  component's saved view).
- No real external site contacted; no model calls;
  all provider traffic was the synthetic local fake.

## Windows rerun checklist (for the independent verifier)

1. Fetch the delivered bundle; confirm the tip and
   ancestry `5a050f18` (review tip) over the runtime
   base.
2. On a case-insensitive checkout, run TypeScript,
   the Vite web build, and the UI suite: all must
   pass with 105 cases (this is the direct check
   for VIS-01).
3. Run the canonical visualization workflow and the
   screenshots spec unchanged; compare the four
   screenshots against the shipped evidence.
4. Confirm no real external site is contacted.
5. Native Tauri behavior remains out of scope.

## Still open (not closed by this repair)

- Independent Windows rerun of this repair
  (especially the case-insensitive build).
- Three.js/WebGL renderer swap and remaining §26
  requirements.
- Windows verification of M8 Slice 4.
- M7 native desktop acceptance + preview promotion.
- §32 execution sandbox; M5 production gates.
- M8 is NOT complete after this repair.
