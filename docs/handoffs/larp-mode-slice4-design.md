# LARP Mode Telemetry Engine — Slice 4 Design

Branch `muse/larp-mode-slice4` from Windows-verified LARP Slice 3 review
tip `6f3ce75231f28a6507a4da388f0af5e22b14368e` (review branch
`codex/review-larp-mode-slice3`, docs-only over delivery tip
`932b357bb`).

## Baseline verification (fifth discrepancy in the series)

The uploaded review package verified exactly (ZIP SHA-256 `9fc38df2…`
equals the assignment's; bundle SHA-256 `99fae079…` equals the
MANIFEST's and the assignment's; MANIFEST tip and bundle head agree).
BUT the assignment text named review commit
`6f3ce75231c5ee9191e30900f074d289069d35a3`, which does NOT exist —
not locally and not on origin (origin's `codex/review-larp-mode-slice3`
head is exactly the bundle head). Per the standing rule, bundle heads
are authoritative: this slice branches from `6f3ce75231f28a…`, and the
assignment's tail is recorded here as wrong, not silently used. The
review report confirms the Slice 3 delivery (ZIP `bebf74d5…`, bundle
`8de32cb6…`, tip `932b357bb`, verdict PASS, zero findings, docs 255).
One reviewer wording is NOT adopted: the View 3 fallback panel's
aria-label is `ATTENTION SPIDER // GRAPH DRIFT ENGINE`, not
"Attention spider overview".

## Scope (Slice 4 only — LARP completion)

Centerpiece View 5 (attention heatmap) and View 6 (collider field),
HUD wiring for views 5–6, the two additive producers they need, and
the all-views cycle check. No server op, no new artifact type, no
schema change, no Rust change, no contracts change. After this slice
no centerpiece placeholders remain.

## Grounding: what is real and what is not

Read before designing (not assumed):

- **No attention weights exist in this app** (established in Slice 3):
  no model internals are exposed. The assignment's "multi-head
  attention activation grids" is therefore implemented honestly, not
  literally: View 5 renders ONE genuine alignment matrix — raw-prompt
  words × compiled-prompt words, cell = word presence — plus a token
  sequence strip. No heads are faked, no activation numbers are
  invented. The legend states that no attention heads are measured.
- **Tokens are whitespace words, not provider tokens.** The app has no
  tokenizer; its only token figures are `len()/4` estimates. View 5's
  sequences are therefore words split on whitespace, lowercased with
  end punctuation stripped, capped (raw 32, compiled 64), and labeled
  as words. They are genuine prompt text, not provider token ids.
- **Compiled prompt text is real and available.** `AskResponse`
  manifest carries `compiledPrompt: string`. The Ask producer can
  therefore push the genuine sequences (see below). Insufficient-
  evidence runs have an empty compiled prompt and push NO token event
  — View 5's empty state persists until an answered run, honestly.
- **Collider completion has one clear site.** `Collider.tsx`
  `polled.status === 'complete' && polled.result` → `setResult(parsed)`
  is where a completed synthesis lands. A 3-line try/catch producer
  call there (same pattern as Ask/Memory) is cheap and safe (bus push
  only, no setState), so View 6's discharges are driven by REAL
  collider generations, not by Ask runs. The beams themselves are a
  stylized continuous visualization (not measured collider telemetry)
  — the legend says so — and the discharge flash + counter respond
  only to real collision events.
- **Doppler coloring is a visual effect, not measured physics.**
  Particle color is a function of the simulated trajectory parameter
  (approaching the interaction point → blue-shifted, at the point →
  white-hot). It is labeled stylized in the design and the legend.

## Module map

All new-module imports carry explicit `.ts`/`.tsx` extensions (NTFS rule).

- `telemetry.ts` (modified, additive only) — new event kind
  `'ask-tokens'` { at, rawTokens: string[], compiledTokens: string[] }
  (sanitized: strings only, each truncated to 64 chars, capped at 32
  raw / 64 compiled) and new event kind `'collision'` { at, beamA:
  string, beamB: string } (labels truncated to 80 chars).
- `telemetryProducers.ts` (modified) — `tokenizeWords(text)` (pure:
  split on whitespace, lowercase, strip end punctuation
  `[.,;:!?()\"']`, drop empties); `AskRunResponseLike.manifest` gains
  optional `compiledPrompt?: string`; `reportAskRun` pushes an
  `ask-tokens` event only when the compiled prompt is non-empty, plus
  its existing compilation/token-delta events. New
  `reportCollision(beamA, beamB, stream)` pushing a `collision` event
  with truncated labels.
- `views/tokenAlignment.ts` — pure `alignmentMatrix(rawTokens,
  compiledTokens)` returning { rows, cols, matches: boolean[][],
  matchCount } (cell = lowercased word equality after the same
  normalization; rows/cols capped as above).
- `views/View5AttentionHeatmap.tsx` — high-DPI 2D canvas (Canvas 2D
  chosen over a WebGL shader grid, same rationale as View 2: token
  labels need crisp text; the spec allows "WebGL / Canvas"): rows =
  raw words with left labels (truncated to 12 chars), columns =
  compiled words, match cells amber `#ff8a3d`, non-match cells dark
  `#18181f`, hairline grid. DOM legend with raw/compiled/match
  counts, the "no attention heads measured" line, and the "words, not
  provider tokens" line. Hover readout via the View 4 window-listener
  + rect-check pattern (HUD is click-through): nearest cell's (raw
  word, compiled word, match). `data-raw-tokens` /
  `data-compiled-tokens` / `data-matches` counters via refs. Redraws
  in an effect on `[events]` (the Slice 3 View 4 lesson: never draw
  only inside a subscription callback). Empty state ("No compiled
  prompt yet — ask a question in Ask ANANSI.") when no `ask-tokens`
  event exists, never a fabricated matrix.
- `views/colliderField.ts` — pure beam math: `beamPoint(beam, t)`
  (cubic bezier from a beam origin to the interaction point at the
  origin; beam A from the left, beam B from the right, slight vertical
  arcs) and `dopplerColor(t)` (beam color → white-hot as t → 1).
  Unit-tested for endpoints, t bounds, and color interpolation.
- `views/View6ColliderField.tsx` — two `THREE.Points` clouds (2,000
  particles each) streaming along the beam beziers toward the origin
  through the shared `createLarpComposer` pipeline, slow group
  rotation (skipped when reduced motion), `data-frames` counter.
  Discharge: a subscription + mount-time seeding from collision events
  in the last 60 s (View 2's pulse-seeding pattern) sets a flash that
  animates composer bloom strength 0.6 → 1.6 → decay over ~1.2 s;
  `data-collisions` counts collision events seen. DOM legend shows
  the last collision's beam labels, or "No collisions yet — run
  LEXI-COLLIDER in a project." WebGL-failure fallback panel. Disposal
  mirrors Views 1–3 (composer, geometries, materials, renderer,
  forced context loss, store disposer).
- `Collider.tsx` (modified, 3 lines + import) — after
  `setResult(parsed)` in the completion branch, try/catch
  `reportCollision(beamA, beamB)` (beam input texts, truncated by the
  producer). No other collider behavior changes.
- `LarpHud.tsx` (modified) — view 5 → `View5AttentionHeatmap`, view
  6 → `View6ColliderField`; the full-size centerpiece class now
  applies to all views (no placeholders remain). Keys 1–6 already
  switch views — no switcher change.
- `larp-mode.css` (appended) — heatmap readout + legend styles
  (same pattern as the spectrum styles).

## Placeholder removal (test maintenance, same pattern as Slices 2–3)

No placeholders remain after this slice, so two assertions must move,
not be deleted:

- `packages/ui/tests/larp-mode.test.tsx`: the HUD test rendered
  `initialView={5}` and asserted the placeholder text. View 5 is now
  live with an empty state, so the assertion becomes the View 5 empty
  text ("No compiled prompt yet").
- `tests/e2e/larp-mode-slice1.spec.ts`: the toggle test clicked view 5
  and asserted its placeholder text (moved there in Slice 3). View 5
  is now live, so the test clicks view 6 and asserts the collider
  container is visible instead — still proving all six buttons exist
  and a live view renders.

## Tests

- `packages/ui/tests/larp-views4.test.tsx` (added to `test:ui`):
  tokenize normalization + caps; alignment matrix (matches,
  case-insensitivity, caps); ask-tokens producer mapping (raw/compiled
  from prompt + manifest, no event when compiled is empty);
  collision producer mapping (label truncation); beam endpoints /
  bounds; Doppler color interpolation.
- `tests/e2e/larp-mode-slice4.spec.ts`: (1) View 5 — seed a memory,
  ask a related question, wait for an answered run, View 5 → canvas +
  raw ≥ 1 + compiled ≥ 1 + matches ≥ 1 + hover readout change. (2)
  View 6 — run a real collider generation via the harness
  `collideAndSave` helper, View 6 → canvas + frames ≥ 1 + collisions
  ≥ 1 + legend shows a beam label substring. (3) All-views cycle —
  keys 1–6 in sequence (each view container visible) + HUD buttons +
  Ctrl+Shift+L disposal. (4) Empty states — fresh session: View 5
  empty panel, View 6 "No collisions yet" legend.

## Verification commands

`./node_modules/.bin/tsc --noEmit`, `node scripts/contracts.mjs --check`,
`node scripts/validate-docs.mjs`, `npm run test:ui`,
`cd apps/web && node node_modules/vite/bin/vite.js build` (rebuild before
e2e — preview serves `dist`), then the Slice 4 spec plus the full
regression battery (prior 36 + new spec tests) sequentially on an idle
machine, then the Rust suite (no Rust touched; expect 616 + 4).
Battery discipline (Slice 3 lesson, recorded): use the REAL spec
filenames (`product-lifecycle-slice*.spec.ts`, not
`lifecycle-slice*.spec.ts`) and capture FULL summaries, never a
tail-only grep line.
