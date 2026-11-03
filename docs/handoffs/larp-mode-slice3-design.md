# LARP Mode Telemetry Engine — Slice 3 Design

Branch `muse/larp-mode-slice3` from Windows-verified LARP Slice 2 review tip
`c8f61c8441e0c23621535fb7539e09f35b2fd285` (review branch
`codex/review-larp-mode-slice2`, docs-only over delivery tip `43ff2cdbb`).
Baseline note: no review ZIP was uploaded for this assignment — the
spec states the review branch was pushed to origin, so the commit was
verified directly from origin by hash: head is exactly `c8f61c84…`,
parent is exactly delivery tip `43ff2cdbb`, diff is docs-only
(BUILD_STATUS + RECOVERY reword to "verified on Windows" + the 73-line
review report). The report confirms the Slice 2 delivery (ZIP
`8ed7cb59…`, bundle `cde4927a…`, 23 files, verdict PASS, zero findings,
all counts matching). The spec's review-package hashes (`9038cc2b…` /
`36ce1064…`) were NOT independently re-verified — the file was not
uploaded here — but they are not needed: this slice branches from the
verified commit itself, which is content-addressed by its hash. Two
reviewer wordings are NOT adopted: the View 1 fallback panel's
aria-label is `SECOND BRAIN // 3D MEMORY VAULT`, not "Memory vault
overview", and View 2's inputs carry no semantic labels in the
implementation (they are four amber/gold streams, not "Notes, Vault
Memories, Pinned Rules, User Input").

## Scope (Slice 3 only)

Centerpiece View 3 (attention spider) and View 4 (quant drift spectrum),
HUD wiring for views 3–4, and the additive telemetry fields they need.
No server op, no new artifact type, no schema change, no Rust change,
no contracts change. Views 5–6 stay placeholders; a later slice owns
them.

## Grounding: what is real and what is not

Read before designing (not assumed):

- `MemoryMetadata` carries `relationships: string[]` (record ids from
  the relationships table) and `confidence?: number | null`. These are
  the only real edge and confidence sources available to the HUD.
- `GroundedCitation` carries `memoryId`: citations reference memory
  records, which ARE memory nodes. So "cited entities" are a subset of
  memory nodes — but only if the bus knows which were cited. It
  currently does not, hence the additive `citedIds` field below.
- There are NO attention weights anywhere in this app: no model
  internals are exposed, and relationship records carry no weight. The
  assignment's "edges represent cross-attention weights" is therefore
  implemented honestly, not literally: edges are recorded relationships
  (bright) plus deterministic layout links (dim, labeled as such), and
  NO numeric weight is displayed anywhere. Presenting layout geometry
  as measured attention would violate the slice's own guardrail.
- There is NO collision-resonance score anywhere in this app: collider
  outputs carry no resonance number, and SwarmTank/SOMNIA artifacts are
  not reachable from the HUD bus. View 4 therefore does NOT render a
  resonance series; its legend states plainly that resonance is not
  measured. What View 4 does render is all real bus data (below).
- Drift events exist in the bus union but no producer fills them yet;
  View 4 renders the drift series only when samples exist, with an
  explicit empty state otherwise. No synthetic drift is generated.

## Module map

All new-module imports carry explicit `.ts`/`.tsx` extensions (NTFS rule).

- `telemetry.ts` (modified, additive only) — `MemoryNode` gains
  optional `relationships?: string[]` and `confidence?: number | null`;
  the `memory` sanitize case picks known fields (relationships filtered
  to strings, confidence kept only when a finite number, else null)
  instead of blindly spreading the node. `compilation` gains optional
  `citedIds?: string[]` (strings only, capped at 50).
- `telemetryProducers.ts` (modified) — `reportMemorySnapshot` maps
  relationships + confidence; `AskRunResponseLike` gains optional
  `citations?: { memoryId: string }[]` and `reportAskRun` fills
  `citedIds` from them on every completed run (including
  insufficient-evidence runs, which still cite nothing — empty list
  omitted, not pushed as empty).
- `views/forceLayout.ts` — pure deterministic force layout:
  `buildGraph(nodes, citedIds)` (nodes capped at 64, first 64 of the
  snapshot; edges = relationships matched to rendered nodes only, plus
  a deterministic ring of layout links as the skeleton) and
  `layoutGraph(nodes, edges, seed, iterations)` (seeded sphere start,
  repulsion + springs + centering, fixed 150 iterations, damping;
  bounded output, no NaN). Unit-tested for determinism, finiteness,
  bounds, and ring/relationship edge construction.
- `views/View3AttentionSpider.tsx` — `THREE.Points` (nodes: memory
  amber `#ff8a3d`, note cyan `#00f0ff`, artifact magenta `#ff1e7a`,
  cited nodes white) + `THREE.LineSegments` (relationship edges bright
  cyan-white, layout links dim) through the shared `createLarpComposer`
  pipeline (the spec's shared-pipeline requirement), slow rotation
  (skipped when reduced motion), per-frame camera-projected tags
  (capped at 24), `data-frames` / `data-nodes` / `data-edges` counters
  via refs, WebGL-failure fallback panel, and the Slice 1/2 disposal
  pattern (composer, geometries, materials, renderer, forced context
  loss, store disposer). Empty stream → explicit panel ("No memory
  nodes yet — open Memory to load records."), never a fake graph.
- `views/spectrum.ts` — pure `spectrumSeries(events)` mapping: drift
  points (score 0..1), compression points ((raw − compiled) / raw from
  token-delta events, clamped to −100..100), compilation markers, and
  confidences (finite confidence values from the latest memory event).
  X is EVENT ORDER (index-normalized 0..1), not wall time — the bus
  mixes Date.now() producer events with small-int test events, and
  labeling the axis "EVENT ORDER" is honest where a time axis would
  not be. Empty inputs → empty series (unit-tested).
- `views/View4QuantDrift.tsx` — high-DPI 2D canvas drawing the four
  series plus a legend carrying per-series sample counts and the
  explicit "Collision resonance: not measured" line. Hover crosshair +
  readout (nearest sample's drift/compression values, marker labels)
  via mousemove, readout div updated imperatively. `data-markers` /
  `data-confidences` counters via refs. Subscription + snapshot seed
  like View 2 (re-render on new events).
- `LarpHud.tsx` (modified) — view 3 → `View3AttentionSpider`, view 4
  → `View4QuantDrift`; the full-size centerpiece class now applies to
  views 1–4; views 5–6 placeholders unchanged. Keys 1–6 already switch
  views (Slice 2) — no switcher change needed.
- `larp-mode.css` (appended) — spectrum readout line. View containers
  reuse `.larp-view-container`.

## Frame loop + disposal, precisely

Same contract as Views 1–2: each view owns its RAF loop (the HUD
scheduler's ticks remain no-ops); unmount (switch or OFF) cancels RAF,
disposes composer + geometries + materials, disposes the renderer, and
forces context loss; the same cleanup is registered as a store disposer
with idempotent guards.

## Tests

- `packages/ui/tests/larp-views3.test.tsx` (added to `test:ui`): force
  layout determinism/bounds/finiteness; graph edge construction
  (relationships matched only to rendered nodes, ring links present,
  cited set extraction); spectrum mapping (drift/compression/markers/
  confidences, empty inputs); producer mapping (relationships,
  confidence, citedIds, sanitize caps).
- `tests/e2e/larp-mode-slice3.spec.ts`: seed 3 memories + 1 real
  relationship via the harness API, visit Memory (producer push),
  toggle on, View 3 → canvas + `data-frames` ≥ 1 + `data-nodes` ≥ 3 +
  `data-edges` ≥ 3 + a node tag; View 4 → canvas + legend + hover
  readout (mouse move over canvas → readout visible); keys `4`/`3`
  switching; OFF disposal; View 3 empty-state leg (fresh session, no
  Memory visit → explicit panel, no canvas). Click/key behavior is
  e2e-only, as established.

## Verification commands

`./node_modules/.bin/tsc --noEmit`, `node scripts/contracts.mjs --check`,
`node scripts/validate-docs.mjs`, `npm run test:ui`,
`cd apps/web && node node_modules/vite/bin/vite.js build` (rebuild before
e2e — preview serves `dist`), then the Slice 3 spec plus the full
regression battery (prior 33 + new spec tests) sequentially on an idle
machine, then the Rust suite (no Rust touched; expect 616 + 4).
