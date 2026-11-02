# M11 GHOST-PROMPT Slice 2 — Addendum (Linux delivery, Windows rerun pending)

## Baseline and the SHA discrepancy

Branch `muse/m11-ghost-prompt-slice2` from the
Windows review head for Slice 1. The assignment
and the review MANIFEST both name parent
`9ac467fe835c2b3e839218671fc31efbe8855a8f`,
but that full SHA does not exist in any fetched
object: the review bundle's only head is
`9ac467fec54c28c957428187d998d73599a6a965`
(shared prefix `9ac467fe`, different tail), whose
commit message, parent (`0f9eec261`), and diff
(review docs plus the NTFS import fix) are
exactly the review the assignment describes.
The branch was therefore created from that
verifiable bundle head — which is also precisely
the assignment's authorized alternative (clean
tip `0f9eec261` + the NTFS import
qualification). The discrepancy is recorded here,
in the Slice 2 design doc, and in the MANIFEST
so the reviewer can confirm the tail; no work
was built on an unverified commit.

Slice 1 Windows verdict carried forward (review
branch `codex/review-m11-ghost-prompt-slice1`):
tsc 0, contracts 0 drift, docs 231, UI 178/178,
Rust ghost 11/11, workspace 603 + 4
offline-Ollama skips, Vite 2.58s, Playwright
18/18. AN-016 stays `in progress`, now for the
Slice 2 scope.

## Scope delivered: §56, §65

The visual exploration layer for active
GHOST-PROMPT sessions, mounted in the Ask ANANSI
expanded view behind the Review drafts section's
`Expand 3D view` toggle:

- `packages/ui/src/ghostPromptThreeLayout.ts` —
  pure, deterministic layout (no Three.js import,
  usable from node tests and the e2e spec):
  retained baseline along X in [-200, 200] with
  linear offset mapping, one violet node per
  deleted fragment in three staggered rows,
  thread segments down to each baseline edit
  point, pause markers at the next edit's
  baseline point (trailing pauses at the
  baseline end), revision clusters for
  consecutive edits within 8 characters, and the
  fixed-camera projection formula (fov 45 at
  (0, 30, 320), lookAt (0, 30, 0)).
- `packages/ui/src/ghostPromptThreeScene.ts` —
  Three.js scene builder (pinned `three@0.187.0`,
  already a dependency) that creates no renderer:
  white baseline bar, translucent violet spheres
  (opacity 0.55), a single LineSegments object
  for all threads, shared ring geometries for
  pause markers, thin underline bars for
  clusters, raycast `pick` by NDC, `setSelected`
  scaling 1.5×, ring-only breathing animation,
  and disposal of every geometry/material/
  texture (shared objects disposed once).
- `packages/ui/src/GhostPrompt3D.tsx` — the
  component owning the renderer lifecycle
  (low-power preference, DPR clamped to 2,
  resize handling): WebGL failure at mount or
  renderer creation throws, and mid-session
  `webglcontextlost`, all fall back to the
  complete 2D view; reduced motion initializes
  synchronously from `isReducedMotion()`
  (matchMedia or workspace dataset) and is
  toggleable via a `Reduce motion (static view)`
  checkbox; the static layout IS the 2D view.
  The 2D view is an SVG timeline using the same
  layout numbers, with per-node `Select fragment`
  buttons for keyboard users, canvas ArrowLeft /
  ArrowRight node cycling, a selected-fragment
  detail card (text, offset, hash, preceding
  pause), and a semantic `<ol>` edit history.
  The Slice 1 fragments table stays as the
  tabular alternative.
- Mount + NTFS qualification: `GhostPrompt.tsx`
  gains the 3D toggle inside the Review drafts
  section; `AskAnansi.tsx` engine imports moved
  to explicit `./ghostPrompt.ts`, and every new
  or edited import uses an explicit extension
  (`./GhostPrompt3D.tsx`,
  `./ghostPromptThreeLayout.ts`,
  `./ghostPromptThreeScene.ts`) — the Slice 1
  NTFS lesson applied uniformly. Zero
  `innerHTML` in the new files.

## Deliberate deviations and limits (stated, not hidden)

- §65 names instanced geometry; this slice uses
  one mesh per deleted fragment instead. The
  input is bounded (≤500 events, typically a
  handful of fragments), which keeps draw calls
  bounded without instancing, and per-mesh
  picking/disposal stays trivial and explicit.
  If a later slice needs hundreds of visible
  nodes, instancing is the documented next step.
- Pause rings face the frontal camera (XY plane)
  rather than lying flat: a lay-flat ring reads
  as a thin line from this camera. The arena's
  top-down camera is the reason flat rings work
  there; they do not transfer here.
- Context loss falls back permanently to the 2D
  view; the context is not restored. Recovery is
  the fallback, not a re-created renderer.
- The scene rebuilds when the snapshot content
  changes (keyed on the snapshot JSON). No
  incremental scene update in this slice.
- The review report's "FNV-1a 64-bit" wording is
  not repeated: the implementation is 32-bit
  (offset 2166136261) in both engines.
- No server op, no new artifact type, no schema
  change, no Rust change in this slice; contracts
  regen confirms zero drift.

## Genuine defects found and fixed during the slice

- The scene builder never updated world matrices,
  so headless raycast picking returned null:
  `pick()` now refreshes `scene.updateMatrixWorld`
  before raycasting (the component's render loop
  does the same every frame).
- Two test expectations mis-derived the engine's
  diff: the full-text delete maximizes the shared
  prefix (`blue `) and the non-overlap rule caps
  the suffix at 5, so the recorded fragment is
  `fox ` at offset 5 — the implementation was
  right and the expectations were corrected,
  with the reasoning recorded in the test.
- The e2e spec's first import path pointed at a
  nonexistent `tests/src` directory; corrected to
  `../../packages/ui/src/ghostPromptThreeLayout`
  (the arena spec's path pattern).

## Linux verification (sequential, 2026-11-02)

- Rust workspace **603 passed + 4 known
  live-Ollama `semantic_workflow` environment
  failures** (unchanged from baseline; no Rust
  change in this slice), strict fmt clean.
- Node **186/186** (178 + 8 new ghost-prompt-
  three tests: layout determinism and mapping,
  cluster rules, projection formula, headless
  pick/select/animate, disposal event counts,
  capability probes, static renders, no-
  `innerHTML` + three pin).
- tsc 0, contracts `--check` exit 0 (zero drift),
  docs validation PASS (234 files including this
  addendum).
- Web build clean; e2e
  `ghost-prompt-slice2.spec.ts` **2/2** (3D
  canvas + raycast node picking → detail card;
  reduced-motion static view both ways +
  `WEBGL_lose_context` → 2D fallback), zero
  provider traffic.
- Regression battery **20/20**: ghost Slice 1
  2/2, SOMNIA Slices 1–3 2/2 each, evaluations
  1/1, integrated workflow 1/1, opportunities
  1/1, SwarmTank scenario 3/3, arena 3/3,
  slice1 1/1. The known search real-semantic
  e2e failure was not re-run in this battery;
  it is a pre-existing live-Ollama environment
  failure, not a Slice 2 regression.

## Windows rerun checklist (for the reviewer)

- `npm run test:ui` (expect 186/186), tsc 0,
  contracts 0 drift, docs validation PASS.
- `cargo test --workspace` (expect 603 + 4 known
  offline-Ollama skips).
- Playwright under the shared lease:
  `ghost-prompt-slice2.spec.ts` 2/2 plus the
  regression specs above; confirm the Ask strip
  renders in the desktop shell (the Slice 1
  checklist item carries over).
- Confirm the NTFS import qualification holds:
  no extensionless import of any ghost module
  remains in new or edited files.

## Deferred (unchanged)

Slice 3 (§57 semantic "Review My Draft" advice,
§58 dual-panel comparison, provider-backed
model dispatch) remains deferred and
uncommissioned. M7 native desktop acceptance +
preview promotion, §32 sandbox, M5 production
gates, and Windows verification of M8 Slice 4
remain open. Linux evidence here is never
presented as Windows acceptance.
