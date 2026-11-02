# M11 GHOST-PROMPT Slice 1 — Addendum (consent, in-memory engine, revision capture)

## Baseline

Branch `muse/m11-ghost-prompt-slice1` from the verified
Windows review tip `d7887ad82` (review branch
`codex/review-m10-somnia-slice3`, closing Milestone
10). The tip was absent locally at assignment time,
so work was held until the owner uploaded the review
package
`anansi-m10-somnia-slice3-windows-verified-2026-11-02.zip`;
the package was verified before branching (ZIP hash
equals sidecar `098ea8d5…`, bundle hash equals the
review MANIFEST `1c8875ab…`, fetch lands on exact
tip `d7887ad82`, ancestry includes `91db98ea0`,
diff vs `91db98ea0` is reviewer docs plus one
reviewer test-synchronization fix, taken as-is).
The review verdict is recorded: tsc 0, contracts 0
drift, docs 228 at review time, UI 168/168, Rust
somnia 21/21, workspace 592 + 4 offline-Ollama
skips, Vite 1.86s, Playwright 16/16. M10 is complete
on Windows.

## What Slice 1 builds (§52–55, §86.1–4, §86.8–9)

- **Consent model (§53, §74).** GHOST-PROMPT is OFF
  by default on every mount; consent is never
  persisted and general ANANSI usage consent is never
  read as recording consent. An opt-in modal carries
  the consent text verbatim; while ON, a persistent
  `Draft Exploration ON` badge shows Recording /
  Paused status, with explicit `Pause recording` /
  `Resume recording`, `Review drafts`,
  `Clear draft history`, and
  `Stop & discard session` controls.
- **In-memory session engine.** Rust
  `crates/core/src/ghost_prompt.rs` (reference) and
  `packages/ui/src/ghostPrompt.ts` (live mirror,
  same semantics, pinned by parallel tests). Editor
  changes diff by common prefix + suffix over
  Unicode scalar values, yielding exact deleted /
  inserted spans and character offsets; kind hints
  come from the DOM `inputType` (paste, undo/redo,
  delete, replace) with inference from the diff when
  absent. Pause events synthesize when the gap
  between edits exceeds 2000 ms (configurable) and
  carry only the factual duration — no emotion or
  intent language anywhere. Cadence is derived at
  review time only (inserted characters, edit
  count). Bounds: 500 edit events, 50KB buffered
  event text, oldest dropped first; 4-hour session
  auto-stop. Retained snapshots are the live current
  text plus its hash — full history is deliberately
  not stored.
- **Hashes.** FNV-1a is the cross-language
  deterministic hash (fragment hashes, current-text
  hash, byte-identical). SHA-256 of the full current
  text is computed in the Rust engine only; the
  browser has no synchronous SHA-256 API, so the TS
  engine does not fake one (SOMNIA Slice 2
  precedent, stated openly).
- **Privacy guardrails.** Editor-scoped to the Ask
  prompt input only — no global keylogging path
  exists. Password scopes are refused at session
  creation (Rust `Err`, TS `null`). No file, SQLite,
  or network path exists in the engine or the strip;
  the e2e asserts provider traffic unchanged.
- **Wipe and verification (§86.8–9).** Rust wipes by
  overwriting buffered bytes with zeros before
  dropping them; JavaScript strings are immutable,
  so the TS wipe drops every reachable reference
  instead (recorded here as a deliberate, honest
  deviation from the design doc's
  "overwrite-then-drop" wording for TS).
  `verifyWiped()` is true only when buffer, pauses,
  current text, and byte counters are all empty,
  and tests assert no fragment text survives
  serialization. The UI shows an empty table plus
  `Draft history purged.` after Clear or Stop.
- **UI.** `GhostPromptStrip` in Ask ANANSI, above
  the form, with the `Retained & Deleted Fragments`
  inspection table (retained text, strikethrough
  deleted fragments with badge, offset, hash, pause
  events, cadence line). Zero `innerHTML` in the new
  files. A resume fix landed during implementation:
  resuming clears the paused interval so the next
  edit does not synthesize a pause for time spent
  paused (mirrored in both engines, tested in both
  suites). A second fix: the UI rounds
  `performance.now()` deltas to integer milliseconds
  to match the Rust `u64` model (the first e2e run
  exposed float durations that broke the pause
  assertion).

## Files

Added: `crates/core/src/ghost_prompt.rs`,
`packages/ui/src/ghostPrompt.ts`,
`packages/ui/src/GhostPrompt.tsx`,
`packages/ui/tests/ghost-prompt.test.tsx`,
`tests/e2e/ghost-prompt-slice1.spec.ts`,
`docs/handoffs/M11-ghost-prompt-slice1-design.md`,
this addendum. Modified: `crates/core/src/lib.rs`
(module declaration), `packages/ui/src/AskAnansi.tsx`
(strip mount + change wiring),
`package.json` (`test:ui` list),
`docs/BUILD_STATUS.md`, `docs/RECOVERY.md`,
`plans/09-ghost-prompt.md`,
`docs/REQUIREMENTS_MATRIX.md` (AN-016). No schema
change, no new op, no new artifact type, no contract
change: contracts regenerate with zero drift.

## Verification (Linux; independent Windows rerun pending)

- Rust: `ghost_prompt` 11/11; full workspace **603
  passed + 4 known live-Ollama environment failures**
  (592 baseline + 11 new; the 4 are the pre-existing
  `semantic_workflow` `StorageUnavailable` group,
  identical to baseline). Strict Clippy clean
  (one genuine finding fixed: collapsible pause
  `if`), fmt clean.
- Node: `test:ui` **178/178** (168 + 10 new), tsc 0,
  contracts `--check` exit 0 (zero drift; the
  "unknown format uint" lines are pre-existing
  validator noise on untouched schemas), docs 231
  files PASS, web build clean.
- Browser battery: **68 passed, 1 failed of 69
  tests** — the single failure is the known
  pre-existing `search.spec.ts` real-semantic test
  (live Ollama absent). New ghost spec 2/2:
  consent → capture → review → purge, and
  pause/clear behavior. Regression specs all green
  (Ask, SOMNIA ×3, evaluations, integrated
  workflow, opportunities, SwarmTank). Provider
  traffic unchanged throughout (fake hits 0).
- §86 walk for this slice: (1) enable via modal,
  (2) type and revise, (3) recording indicator,
  (4) inspect retained/deleted fragments,
  (8) delete the session, (9) verify nothing
  remains. Steps (5) interpretations, (6) fragment
  selection for use, and (7) response comparison
  belong to Slices 2/3 and are not claimed.

## Report (§88 fields for this slice)

- Features built: listed above.
- Files added/modified: listed above.
- Architecture decisions: client-side in-memory
  engine with a Rust reference (no server op), FNV-1a
  shared hash + Rust-only SHA-256, integer-ms event
  times, ring + size bounds with oldest-first drop.
- Database migrations: none.
- Tests passed: Rust 603, node 178, e2e 68.
- Tests failed: Rust 4 (known Ollama env), e2e 1
  (known search real-semantic).
- Tests not run: none in scope; Windows verification
  is the reviewer's step, not run here.
- Security issues: none. No credentials, no provider
  traffic, no persistence of drafts, password scopes
  refused.
- Live-provider verification: none by design (zero
  provider traffic is the requirement).
- Known limitations: single-line Ask prompt input
  (maxLength 500) bounds what the editor can capture
  in this slice; pauses surface on the next edit
  (no background timers); TS wipe drops references
  rather than zeroing bytes (JS immutability).
- Last verified commit: see MANIFEST (tip after
  closure docs).
- Backup status: no new artifact type, so no new
  backup surface; the Ask view needs no backup.
- Current runnable command: the standard web + API
  dev flow; e2e via the shared lease script.
- Next milestone: M11 Slice 2 (Three.js
  visualization, §56) and Slice 3 (semantic advice,
  dual-panel comparison, provider dispatch, §57–58)
  — neither commissioned yet; do not start unasked.
