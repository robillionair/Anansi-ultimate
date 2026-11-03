# LARP Mode Telemetry Engine — Slice 4 Addendum (LARP Completion)

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
"Attention spider overview". (Note: the Windows-created review ZIP
uses backslash path separators, so plain `unzip` fails on it — the
package was extracted with Python's zipfile module instead. The
delivery ZIPs Muse ships use forward slashes and are unaffected.)

## Scope (Slice 4 — LARP completion)

View 5 `ATTENTION TENSOR HEATMAP // TOKEN ATTENTION MATRIX`, View 6
`COLLIDER FIELD // CONCEPT RESONANCE ACCELERATOR`, HUD wiring for
views 5–6, the two additive producers they need, and the all-views
cycle check. No server op, no new artifact type, no schema change, no
Rust change, no contracts change. After this slice no centerpiece
placeholders remain — all six views are live.

## Honest telemetry (the load-bearing decisions)

- **No attention heads are measured.** The assignment's "multi-head
  attention activation grids" is implemented as ONE genuine alignment
  matrix — raw-prompt words × compiled-prompt words, cell = word
  presence — plus a token sequence strip. No heads are faked, no
  activation numbers are invented; the legend states that no attention
  heads are measured.
- **Tokens are whitespace words, not provider tokens.** The app has no
  tokenizer; its only token figures are `len()/4` estimates. View 5's
  sequences are words split on whitespace, lowercased with end
  punctuation stripped, capped (raw 32, compiled 64), and labeled as
  words. The compiled prompt text itself is genuine (it comes from the
  Ask response manifest).
- **Insufficient-evidence runs push no token event.** Their compiled
  prompt is empty, so View 5's empty state persists until an answered
  run — honestly, not papered over with a fabricated matrix.
- **Collider discharges are real generations.** The collider's single
  completion site (`polled.status === 'complete'` → `setResult`) now
  pushes a `collision` event with the beam input labels (truncated).
  View 6's flash + counter respond ONLY to those events. The beams
  themselves are a stylized continuous visualization, and the legend
  says so; Doppler coloring is a visual effect (beam color far away,
  white-hot at the interaction point), not measured physics.
- **Hover without breaking click-through.** Views 5–6 reuse the View 4
  window-listener + rect-check pattern, so hover works and the
  underlying app stays clickable.
- **Redraw discipline (Slice 3 lesson applied).** View 5 redraws in an
  effect on `[events]`, never only inside a subscription callback.

## Implementation

- `telemetry.ts` — new event kinds `ask-tokens` { rawTokens (cap 32),
  compiledTokens (cap 64), each token truncated to 64 chars } and
  `collision` { beamA, beamB (truncated to 80 chars) }, both sanitized.
- `telemetryProducers.ts` — `tokenizeWords` (pure); `AskRunResponseLike`
  manifest gains optional `compiledPrompt`; `reportAskRun` pushes an
  `ask-tokens` event only when the compiled prompt is non-empty;
  `reportCollision` (try/catch, best-effort).
- `views/tokenAlignment.ts` — pure `alignmentMatrix` (normalizes both
  sides before comparing, then caps) and `latestAskTokens`.
- `views/View5AttentionHeatmap.tsx` — high-DPI 2D canvas (Canvas 2D
  chosen over a WebGL shader grid, same rationale as View 2: token
  labels need crisp text; the spec allows "WebGL / Canvas"): row
  labels, match cells amber, non-match cells dark, hover readout,
  `data-raw-tokens` / `data-compiled-tokens` / `data-matches`
  counters, explicit empty state.
- `views/colliderField.ts` — pure beam beziers (A from the left, B
  from the right, converging at the origin) and `dopplerColor`.
- `views/View6ColliderField.tsx` — two `THREE.Points` clouds (2,000
  particles each) streaming along the beziers through the shared
  `createLarpComposer` pipeline, interaction-point marker, slow
  rotation (skipped when reduced motion), discharge flash animating
  composer bloom strength 0.6 → 1.6 → decay over ~1.2 s, seeded from
  collision events in the last 60 s, `data-frames` / `data-collisions`
  counters, WebGL-failure fallback, Slice 1/2/3 disposal pattern.
  `LarpComposer` handle gains `bloomPass` (additive; the dispose loop
  already covers it via the passes list).
- `Collider.tsx` (3 lines + import) — `reportCollision(beamA, beamB)`
  after `setResult(parsed)` in the completion branch, try/catch
  wrapped; no other collider behavior changes.
- `LarpHud.tsx` — views 5–6 render the new components; the full-size
  centerpiece class now applies to all views (no placeholders remain).
  Keys 1–6 already switched views — no switcher change.
- `larp-mode.css` — one comment line (heatmap reuses the spectrum
  readout + legend styles).

## Placeholder removal (test maintenance, same pattern as Slices 2–3)

No placeholders remain, so two assertions moved, not deleted:

- `packages/ui/tests/larp-mode.test.tsx`: the HUD test asserted the
  placeholder text for `initialView={5}`; View 5 is now live with an
  empty state, so the assertion is now the View 5 empty text ("No
  compiled prompt yet").
- `tests/e2e/larp-mode-slice1.spec.ts`: the toggle test clicked view 5
  and asserted its placeholder text (moved there in Slice 3); it now
  clicks view 6 and asserts the collider container is visible — still
  proving all six buttons exist and a live view renders.

## Tests

- `packages/ui/tests/larp-views4.test.tsx` (8 tests): tokenize
  normalization + caps; alignment matrix (matches,
  case-insensitivity, caps); latest ask tokens; ask-tokens producer
  mapping (pushed only when a compiled prompt exists); collision
  producer mapping (label truncation, sanitize caps); beam endpoints
  / bounds; Doppler color interpolation.
- `tests/e2e/larp-mode-slice4.spec.ts` (4 tests): (1) seed a memory,
  ask a related question, wait for an answered run, View 5 → canvas +
  raw ≥ 1 + compiled ≥ 1 + matches ≥ 1 + not-measured legend + hover
  readout change. (2) Real collider generation via the harness
  `collideAndSave` helper, View 6 → canvas + frames ≥ 1 + collisions
  ≥ 1 + legend shows a beam label substring. (3) All-views cycle —
  keys 1–6 in sequence, then HUD buttons 6–1, then Ctrl+Shift+L
  disposal (HUD and all six view containers gone). (4) Empty states —
  fresh session: View 5 empty panel (no canvas), View 6 "No collisions
  yet" legend.

## In-slice fixes (disclosed)

- The alignment unit test caught a contract gap: the matrix compared
  tokens with raw `===` while the design promised normalized
  comparison. Fixed by normalizing both sides inside
  `alignmentMatrix` (it already imported `tokenizeWords`), so direct
  callers with unnormalized tokens still get correct matches.

## Verification (Linux)

- `tsc --noEmit`: 0 errors.
- `node scripts/contracts.mjs --check`: 0 drift.
- `node scripts/validate-docs.mjs`: PASS (258 files).
- `npm run test:ui`: 270/270 (262 + 8 new LARP view tests).
- Production web build clean (`.larp-spectrum-readout` confirmed in
  bundled CSS).
- `larp-mode-slice4.spec.ts`: 4/4 PASS.
- Full regression battery: 40/40 (run sequentially on an idle machine,
  with REAL spec filenames and FULL summaries per the Slice 3 lesson).
- Backend Rust tests: 616 passed + 4 known live-Ollama env failures (no
  Rust files touched; identical to baselines).
- Credentials sweep over the diff vs baseline: clean (only the benign
  `ask-tokens` event identifier matched the `sk-` pattern).

## Known limitations

- View 5 shows the latest answered run only (not run history); rows
  capped at 32 raw words, columns at 64 compiled words.
- View 6's beams are stylized; only the discharge flash + counter are
  driven by real events. No collider-run producer existed before this
  slice, so generations from before this build leave no collision
  events.
- The heatmap's hover readout shows the nearest cell, not an exact
  cursor value.

## Next

- Windows rerun of this slice (checklist in the delivery MANIFEST).
- LARP Mode is feature-complete after this slice (all six views live);
  no further LARP slice is commissioned. Remaining work elsewhere:
  no MATRIX row exists for LARP (none invented).

Last verified commit: tip recorded in the delivery MANIFEST.
