# M11 GHOST-PROMPT Slice 1 — Design (before code)

## Baseline

Branch `muse/m11-ghost-prompt-slice1` from the verified
Windows review tip `d7887ad82` (review branch
`codex/review-m10-somnia-slice3`, closing Milestone
10). The tip was absent locally at assignment time and
no review ZIP had arrived, so work was held until the
owner uploaded
`anansi-m10-somnia-slice3-windows-verified-2026-11-02.zip`;
the package was then verified before branching: ZIP
hash equals its sidecar, bundle hash equals the
review MANIFEST, fetch lands on exact tip `d7887ad82`,
ancestry includes `91db98ea0`, diff vs `91db98ea0`
is reviewer docs plus one reviewer test-synchronization
fix in `somnia-slice3.spec.ts` (taken as-is, never
edited). Review verdict recorded: tsc 0, contracts 0
drift, docs 228, UI 168/168, Rust somnia 21/21,
workspace 592 + 4 offline-Ollama skips, Vite 1.86s,
Playwright 16/16. M10 is complete on Windows.

## Scope: §52–55, §86.1–4, §86.8–9 (Slice 1 only)

Slice 1 is the foundational, privacy-first vertical
slice: strict consent, an in-memory session engine,
and editor revision capture in Ask ANANSI. Deferred
to Slice 2: Three.js visualization (§56). Deferred
to Slice 3: semantic "Review My Draft" advice (§57),
dual-panel comparison (§58), provider-backed model
dispatch of draft context. No Slice 2/3 behavior is
built, stubbed, or implied in this slice.

## A. Consent model (§53, §74, §86.1, §86.3)

- GHOST-PROMPT is OFF by default, per session, per
  editor mount. There is no persisted consent: a fresh
  mount starts OFF, and general ANANSI usage consent
  is never interpreted as recording consent (§74).
- Enabling requires an explicit opt-in modal (role
  `dialog`) carrying the consent text verbatim:
  "When enabled, ANANSI can temporarily track
  revisions made in this editor. Deleted text remains
  available for your review until you discard it or
  explicitly save it." Enable and Cancel buttons;
  Cancel leaves the session OFF.
- While ON, a persistent, highly visible indicator
  reads `Draft Exploration ON` with a `Recording`
  status while recording, `Paused` while paused.
  The indicator is part of the control strip, always
  rendered above the Ask form while the session is
  ON, never hidden behind a menu.
- Session controls, all explicit user actions:
  `Pause recording` / `Resume recording`,
  `Review drafts`, `Clear draft history`,
  `Stop & discard session`.
- Stop & discard wipes the session and returns it to
  OFF. Clear wipes the buffers but keeps the session
  ON (recording continues fresh). Neither writes
  anything to disk, SQLite, or any provider.

## B. Session engine (Rust + TS mirror)

New Rust module `crates/core/src/ghost_prompt.rs`
(declared in `lib.rs`) and new TS module
`packages/ui/src/ghostPrompt.ts`. The Rust engine is
the reference implementation; the TS engine is the
live one driving the Ask editor. Both share the same
semantics, pinned by parallel tests. No server op, no
new artifact type, no schema change: Slice 1 capture
is client-side and in-memory by design, so contracts
regeneration must confirm zero drift rather than add
surface.

### State machine

`Off → On(recording) ⇄ Paused → Off` (via stop) and
`On → Cleared → On`. `enable()` is the only Off→On
transition and requires the consent acknowledgement
(the UI passes it; the engine records a consent
event carrying no text content). `record()` is a
no-op unless state is On-and-recording. `pause()`
and `resume()` toggle recording without wiping.
`clear()` wipes buffers, keeps state On. `stop()`
wipes buffers and returns to Off. An auto-stop
fires when an event arrives with `nowMs` beyond
`maxSessionMs` (default 4 hours): the session wipes
and stops, and the event is refused. This is the
time bound; the size bounds are below.

### Events and diffing

The UI supplies `(previousText, nextText, kindHint,
nowMs)` per editor change, where `nowMs` is
milliseconds since session start (monotonic,
caller-supplied — the engine never reads a clock,
which keeps every test deterministic). The engine
diffs by common prefix + common suffix over
Unicode scalar values (chars, not bytes, so offsets
are character offsets):

- `deleted = old[prefix .. oldLen - suffix]`,
  `inserted = new[prefix .. newLen - suffix]`,
  `offset = prefix`.
- Kind: the hint wins when present (`insert`,
  `delete`, `paste`, `undo`, `redo`, `replace`);
  without a hint, both non-empty → `replace`,
  only deleted → `delete`, only inserted →
  `insert`. No-op changes (identical text) record
  nothing.
- Each edit event stores: sequence number, kind,
  offset, deleted text, inserted text, text length
  after the edit, FNV-1a hash of the deleted
  fragment (when non-empty), and `elapsedMs`.
  The session also keeps the live `currentText`
  and its FNV-1a hash. Retained snapshots are the
  live current text plus its hash — full history is
  deliberately NOT stored (see bounds).
- Deleted fragments for review are exactly the
  events with non-empty `deleted`, in sequence
  order, rendered with strikethrough and a
  `deleted` badge, plus offset.
- Word/fragment granularity: the diff span is the
  contiguous changed span, which in practice is a
  word, phrase, or pasted block; no sub-word
  tokenization is claimed.

### Pauses and cadence (§54)

- A pause event is synthesized when the gap
  between consecutive recorded edits exceeds
  `pauseThresholdMs` (default 2000, configurable
  per session). The pause event carries only the
  factual duration in ms — no emotional or intent
  inference, ever (§52, §54).
- Pauses surface on the next edit (no background
  timers: nothing runs without a user edit).
- Cadence is derived at review time only: total
  inserted characters and edit-event count. No
  pace claim ("fast/slow typist") is rendered.

### Bounds (§55)

- At most 500 edit events; at most 51200 bytes
  (50KB) of buffered event text (sum of deleted +
  inserted byte lengths). On push, oldest events
  drop until both bounds hold (ring behavior).
  Pause events and control events do not count
  toward either bound.
- `currentText` is the live editor content and is
  excluded from the 50KB budget (it is what the
  user sees anyway); it is wiped with the session.
- Sessions are in-memory only. The engine has no
  file, SQLite, or network path, and the UI layer
  never forwards draft text to any provider call
  (the Ask submit path sends only the final prompt
  the user submits, as before).

### Hashes

- FNV-1a (32-bit, offset 2166136261, prime
  16777619, wrapping) is the cross-language
  deterministic hash: fragment hashes and current
  text hash in both Rust and TS, byte-identical.
- SHA-256 of the full current text is computed in
  the Rust engine (sha2, already a workspace
  dependency) as the reference content hash. The
  browser has no synchronous SHA-256, so the TS
  engine does not fake one: it stores the FNV-1a
  hash as its content hash, and the design states
  this split openly (the SOMNIA Slice 2 precedent:
  share exact arithmetic, never pretend a platform
  capability exists).

### Privacy guardrails

- Editor-scoped only: the engine attaches to the
  single Ask prompt input. It never observes global
  keyboard activity, other applications, or other
  inputs.
- Credential fields are refused by construction:
  session creation takes an editor scope
  (`{ id, isPassword }`); a password scope is
  refused (Rust: `Err`; TS: `null`) and records
  nothing. The Ask prompt input is a plain text
  input, never a password field.
- Zero silent transmission: no code path in the
  engine or the strip sends draft text anywhere.
  The e2e asserts provider traffic unchanged.

### Wipe and verification (§86.8–9)

- `wipe()` overwrites every buffered deleted /
  inserted string with zero bytes of the same
  length, then drops the buffer; overwrites
  `currentText` the same way; resets counts.
  (In Rust this is explicit byte zeroing before
  drop, not just `clear()`; in TS the same
  overwrite-then-drop.)
- `verifyWiped()` returns true iff the edit buffer
  is empty, the pause list is empty, `currentText`
  is empty, and both byte counters are zero.
- The UI exposes the verification observably:
  after Clear or Stop, the inspection table shows
  zero rows and a `Draft history purged.` notice;
  the Rust/TS tests additionally assert that
  serializing the wiped session contains no
  fragment text anywhere.

## C. UI integration (Ask ANANSI)

New component `packages/ui/src/GhostPrompt.tsx`
exporting `GhostPromptStrip`, mounted in
`AskAnansi.tsx` directly above the Ask form. The
Ask component owns the engine instance (a ref) and
a version counter bumped after every engine
mutation so the strip re-renders from engine
snapshots. Wiring is minimal and explicit:

- The Ask prompt input's `onChange` diffs the
  previous prompt value against the new one and
  calls `engine.record(...)` with a kind hint
  derived from the DOM event (paste event →
  `paste`; undo/redo input types → `undo`/`redo`;
  Backspace/Delete keydown → `delete`; a
  non-collapsed selection at change time →
  `replace`; otherwise `insert`). The previous
  value is tracked in a ref so diffs are exact.
- The strip renders, in OFF state, an `Enable
  Draft Exploration` button; in ON state, the
  `Draft Exploration ON` badge with
  Recording/Paused status plus the four controls.
- `Review drafts` toggles the inspection section
  `Retained & Deleted Fragments`: the retained
  current text, a table/timeline of deleted
  fragments (fragment with strikethrough, `deleted`
  badge, offset, FNV-1a hash) and pause events
  (duration in ms, elapsed position), plus the
  derived cadence line. The section renders only
  when the user opens it.
- The strip never renders raw draft text outside
  the inspection section, and never sends it
  anywhere. The zero-`innerHTML` regime of the
  SOMNIA panel is adopted for the new files.

## D. Tests

- Rust (`cargo test -p anansi-core --lib
  ghost_prompt`): lifecycle (off→on→pause→resume→
  stop), consent gating (record refused while off
  or paused), diffing (insert/delete/replace/paste/
  undo/redo spans and offsets), pause detection
  (gap above/below threshold; no inference text),
  ring buffer (501st edit drops the 1st), size
  bound (50KB drops oldest), auto-stop at
  maxSessionMs, password-scope refusal, wipe +
  verifyWiped + no fragment text survives
  serialization.
- UI (`packages/ui/tests/ghost-prompt.test.tsx`,
  added to the `test:ui` list): the same semantics
  in TS (gating, diffing, pauses, ring, size,
  wipe), the password guard, and static renders:
  OFF state (no badge, enable button present), ON
  state (badge present, controls present),
  inspection table renders fragments and pauses
  from a pre-populated engine.
- E2E (`tests/e2e/ghost-prompt-slice1.spec.ts`,
  using the shared `startSession` helper and the
  `Ask ANANSI` nav): (1) OFF by default — no
  badge, enable button visible; (2) enable via the
  modal — verbatim consent text present, then
  `Draft Exploration ON` badge + Recording status;
  (3) type an initial prompt, wait >2s, replace a
  term via selection, delete a phrase via
  Backspace — all through real keyboard/selection
  events; (4) open Review drafts — the deleted
  term and phrase appear with strikethrough, and a
  pause event >2000ms appears; (5) Stop & discard
  — table empty, `Draft history purged.` visible,
  badge gone, and a fresh enable starts empty.
  Provider traffic unchanged throughout.
- Full verification: Rust workspace suite
  (expect 592 + new tests, same 4 known live-Ollama
  environment failures), strict Clippy/fmt, node
  `test:ui`, tsc, contracts `--check` (zero drift,
  no new DTOs), docs validation, web build,
  regression specs (Ask, SOMNIA ×3, evaluations,
  integrated workflow, opportunities, SwarmTank),
  full browser battery (expect only the known
  search real-semantic failure).

## E. Acceptance criteria

- AC-1 OFF by default; no recording before
  explicit modal consent; badge only while ON.
- AC-2 Every recorded edit yields the exact
  deleted/inserted spans and offsets the diff
  defines, in both engines (parallel tests).
- AC-3 Pauses are factual durations only; no
  emotion/intent language anywhere in code, UI,
  or docs for this slice.
- AC-4 Ring (500) and size (50KB) bounds drop
  oldest first, deterministically, in both engines.
- AC-5 Wipe is observable and complete:
  `verifyWiped()` true and no fragment text in
  serialization, in both engines; UI shows empty
  table + purged notice.
- AC-6 Password scopes refused; no global
  keylogging path exists (engine attaches to one
  element, by explicit wiring only).
- AC-7 E2E steps 1–5 green; zero provider traffic;
  no disk/SQLite persistence of drafts anywhere.
- AC-8 Full verification green except the known
  pre-existing search real-semantic failure;
  contracts drift-free; docs validation passing.
- AC-9 Checksummed ZIP delivery with
  `review.bundle`, MANIFEST, and sidecars;
  AN-016 advanced for the Slice 1 scope, with
  Slices 2/3 still deferred.
