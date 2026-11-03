# LARP Mode Telemetry Engine — Slice 3 Addendum

Branch `muse/larp-mode-slice3` from Windows-verified LARP Slice 2 review
tip `c8f61c8441e0c23621535fb7539e09f35b2fd285` (review branch
`codex/review-larp-mode-slice2`, docs-only over delivery tip
`43ff2cdbb`).

## Baseline verification (no discrepancy, one channel note)

The Slice 2 review package was not uploaded with this assignment; the
spec states the review branch was pushed to origin, so the commit was
verified directly from origin by hash: head is exactly `c8f61c84…`,
parent is exactly delivery tip `43ff2cdbb`, diff is docs-only
(BUILD_STATUS + RECOVERY reworded to "verified on Windows" + the
73-line review report). The report confirms the Slice 2 delivery (ZIP
`8ed7cb59…`, bundle `cde4927a…`, 23 files, verdict PASS, zero
findings, all counts matching). The spec's review-package hashes
(`9038cc2b…` / `36ce1064…`) were NOT independently re-verified — the
file was not uploaded here — but they are not needed: this slice
branches from the verified commit itself, which is content-addressed
by its hash. Two reviewer wordings are NOT adopted: the View 1
fallback panel's aria-label is `SECOND BRAIN // 3D MEMORY VAULT`, not
"Memory vault overview", and View 2's inputs carry no semantic labels
in the implementation (four amber/gold streams, not "Notes, Vault
Memories, Pinned Rules, User Input").

## Scope

View 3 `ATTENTION SPIDER // GRAPH DRIFT ENGINE` (WebGL 3D
force-directed graph through the shared `LarpComposer` pipeline),
View 4 `QUANT DRIFT & COLLIDER SPECTRUM` (high-DPI 2D canvas over real
bus series), HUD wiring for views 3–4, and the additive telemetry
fields they need. No server op, no new artifact type, no schema change,
no Rust change, no contracts change. Views 5–6 stay placeholders.

## Honest telemetry (the load-bearing decisions)

- **No attention weights exist in this app.** Relationship records
  carry no weight and no model internals are exposed, so the
  assignment's "edges represent cross-attention weights" is implemented
  honestly, not literally: edges are recorded relationships (bright)
  plus deterministic layout links (dim, labeled as such), and NO
  numeric weight is displayed anywhere.
- **No collision-resonance score exists in this app.** Collider outputs
  carry no resonance number and SwarmTank/SOMNIA artifacts are not
  reachable from the HUD bus, so View 4 renders NO resonance series;
  its legend states "Collision resonance: not measured."
- **Cited entities are real.** `GroundedCitation.memoryId` references
  memory records, so compilation events now carry optional `citedIds`
  (from the Ask answer's citations) and the spider renders cited nodes
  white-hot. Citation membership is the only new Ask-side field.
- **Drift stays empty until measured.** Drift events exist in the bus
  union but no producer fills them yet; View 4 renders the drift series
  only when samples exist, with an explicit empty state otherwise. No
  synthetic drift is generated.
- **X is event order, not time.** The bus mixes Date.now() producer
  events with small-int test events; the spectrum axis is labeled
  "EVENT ORDER" so it never implies a time scale it does not have.
- **Node/edge sources.** Spider nodes are the latest memory-event
  snapshot (capped at 64), edges are relationships matched to rendered
  nodes only, plus a deterministic ring of layout links. The spider
  snapshots nodes at mount, like View 1's clusters — no re-cluster
  until remount.
- **Confidence strip** uses finite confidence values from the latest
  memory event only (nulls skipped); the memory producer maps
  relationships + confidence from the same records the Memory view
  already loads (sealed records are excluded by the list query itself).
- **Hover without breaking click-through.** The HUD frame is
  pointer-events none, so the spectrum canvas never receives mouse
  events; View 4 listens on window and hit-tests against the canvas
  rect, so hover works and the underlying app stays clickable.

## Implementation

- `telemetry.ts` — `MemoryNode` gains optional `relationships` /
  `confidence`; sanitize picks known fields (relationships filtered to
  strings, non-finite confidence → null) instead of blindly spreading
  the node. `compilation` gains optional `citedIds` (strings only,
  capped at 50).
- `telemetryProducers.ts` — `reportMemorySnapshot` maps relationships
  + confidence; `reportAskRun` fills `citedIds` from the response's
  citations on every completed run.
- `views/forceLayout.ts` — pure deterministic force layout (seeded
  sphere start, repulsion + springs + centering, 150 iterations,
  bounded to radius 200) and graph construction.
- `views/View3AttentionSpider.tsx` — `THREE.Points` (memory amber,
  note cyan, artifact magenta, cited white) + `THREE.LineSegments`
  (relationship bright, layout dim) through `createLarpComposer`,
  slow rotation (skipped when reduced motion), per-frame
  camera-projected tags (capped at 24), `data-frames` / `data-nodes` /
  `data-edges` counters, WebGL-failure fallback, Slice 1/2 disposal
  pattern (composer, geometries, materials, renderer, forced context
  loss, store disposer). Empty stream → explicit panel, never a fake
  graph.
- `views/spectrum.ts` — pure `spectrumSeries` mapping (drift,
  compression percent clamped to −100..100, compilation markers,
  latest confidences).
- `views/View4QuantDrift.tsx` — high-DPI 2D canvas (drift band,
  compression band with centered zero line, compilation ticks,
  confidence strip), DOM legend with per-series sample counts and the
  not-measured resonance line, hover crosshair + readout.
- `LarpHud.tsx` — views 3–4 render the new components; the full-size
  centerpiece class now applies to views 1–4; views 5–6 placeholders
  unchanged. Keys 1–6 already switched views (Slice 2) — no switcher
  change.
- `larp-mode.css` — spectrum readout + legend styles.

## Tests

- `packages/ui/tests/larp-views3.test.tsx` (9 tests): graph edge
  construction (relationships matched only to rendered nodes, ring
  links, cited flags, 64-cap), force-layout determinism / finiteness /
  bounds, cited-id collection, compression math + clamping, spectrum
  mapping (drift / compression / markers / latest confidences, empty
  input), producer mapping (relationships, confidence, citedIds,
  sanitize cap).
- `packages/ui/tests/larp-mode.test.tsx` (modified): the HUD
  placeholder test now renders `initialView={5}` — view 3 is live, so
  the placeholder assertion moved to a still-placeholder view (same
  pattern as Slice 2's `initialView={3}` fix; assertion kept
  meaningful, not deleted).
- `tests/e2e/larp-mode-slice3.spec.ts` (3 tests): (1) seed 3 real
  records + 1 real relationship via the harness API, visit Memory
  (producer push), View 3 → canvas + frames ≥ 1 + nodes ≥ 3 + edges ≥
  3 + node tag; View 4 → canvas + not-measured legend + confidences ≥
  1 + hover readout change; keys `3`/`4` switching; Ctrl+Shift+L
  disposal. (2) Live Ask run → compilation marker in View 4. (3)
  Fresh session → View 3 explicit empty panel, no canvas.

## In-slice fixes (disclosed)

- The Ask-marker e2e leg caught a real bug: View 4 first drew only
  inside a stream-subscription callback, which fires BEFORE React
  re-renders — so after a completed Ask run the redraw read the stale
  series and `data-markers` stayed 0. Fixed by moving `draw()` /
  `onMove()` to component scope and redrawing in an effect on
  `[events]` (after the render that refreshes the series); the
  subscription now only drives the state update. The spec re-ran 3/3
  after the fix.
- The View 4 hover first listened on the canvas, which never receives
  events under the click-through HUD frame; fixed with the window
  listener + rect check described above.

## Verification (Linux)

- `tsc --noEmit`: 0 errors.
- `node scripts/contracts.mjs --check`: 0 drift.
- `node scripts/validate-docs.mjs`: PASS (255 files with this addendum).
- `npm run test:ui`: 262/262 (253 + 9 new LARP view tests).
- Production web build clean (`.larp-spectrum-readout` confirmed in
  bundled CSS).
- `larp-mode-slice3.spec.ts`: 3/3 PASS.
- Full regression battery: 36/36 (product-lifecycle 1–3 2/2 each,
  LARP Slice 1 2/2, LARP Slice 2 3/3, LARP Slice 3 3/3, ghost 1–3 2/2
  each, somnia 1–3 2/2 each, evaluations 1/1, integrated_workflow 1/1,
  opportunities 1/1, swarmtank-scenario 3/3, swarmtank-arena 3/3,
  swarmtank-slice1 1/1), run sequentially on an idle machine. The
  first battery pass hid two gaps that were caught and fixed before
  delivery: the loop's spec list used `lifecycle-slice*.spec.ts`
  filenames that do not exist (real names are
  `product-lifecycle-slice*.spec.ts`), so three specs silently never
  ran; and the summary pipe kept only the last matching line, hiding
  the Slice 1 placeholder failure. Both are recorded here so the next
  battery uses real filenames and full summaries.
- Backend Rust tests: 616 passed + 4 known live-Ollama env failures
  (no Rust files touched; identical to baselines).
- Credentials sweep over the diff vs baseline: clean (only benign
  matches: token-count identifiers and the 'Session token' form label).

## Known limitations

- Spider nodes snapshot at mount; new memory loads do not re-cluster
  until remount (same as View 1).
- View 3 tags are capped at 24 nodes for readability.
- No drift producer exists yet, so the drift band is empty in normal
  use until one lands.
- The spectrum's confidence strip shows only the latest memory
  event's values, not history.
- View 4 hover readout shows the nearest sample, not an exact cursor
  value (samples are discrete events).

## Next

- Windows rerun of this slice (checklist in the delivery MANIFEST).
- Slice 4 (Views 5–6) not commissioned.
- No MATRIX row exists for LARP (new work, no requirement ID assigned;
  none invented).

Last verified commit: tip recorded in the delivery MANIFEST.
