# M8 URL text input — delivery addendum

Baseline: `144acc715a7a76b3a999c13c20b547461fa53ff6`
(the reviewed tip of the document-input repair2 slice).
Branch: `muse/m8-url-input`.
Design: `docs/handoffs/M8-url-input-design.md`, committed
before any implementation code, with one later amendment
recorded below.

## What this slice adds

Explicit URL entry plus a separate Fetch action for either
LEXI-COLLIDER beam, through the existing beam/document/
context and M5 infrastructure. Fetching never starts
inference and never saves an artifact; only the explicit
Collide press sends the reviewed text. The generation-time
fetched text and its provenance survive save, restart, an
approved Forge transfer, and backup/restore. Manual beams,
document input, and saved-project context selection are
unchanged in behavior.

## Corrections and honest notes

1. **Record port restriction removed.** The design first
   restricted saved URL records to scheme-default ports.
   That made the slice unusable end to end: the fetch
   service under its test policy legitimately produces
   records with non-default ports, and `start()` refused
   them. The port is a fetch-policy matter (the
   production policy allows only default ports); a saved
   record is never fetched or connected to, so it is inert
   text and the port is not a trust anchor. The
   restriction was removed from the backend validator,
   the UI parser, and the design doc, and the forged-
   metadata tests were updated accordingly. The text-hash
   anchor is unchanged.
2. **Time bounds are enforced, not wall-clock tested.**
   Connect 5s, per-read 10s, and total 20s budgets are
   constants enforced by deadline checks between reads
   and hops. No integration test waits out a real 20s
   timeout, so the evidence covers the constants and the
   check placement, not observed timeout behavior.
3. **Cancellation is UI-side only.** Cancelling a fetch
   invalidates the UI session and drops the late outcome;
   the in-flight backend fetch runs to its bounded end
   but saves nothing. There is no server-side abort op.
   This is stated in the design, not oversold.
4. **No real external sites were accessed for testing.**
   All fetch tests use synthetic local servers; the
   browser spec uses a synthetic content server with the
   loopback test policy enabled for the API process only.
5. **The client error carve-out.** The contracts client
   maps `INVALID_REQUEST` to a generic message except for
   listed operations. `ai/collider-fetch-url` was added
   to the verbatim list so fetch refusal reasons (private
   destination, oversized text, non-text content, bad
   URL) surface in the panel instead of the generic
   message. Other collider operations keep the generic
   mapping.

## Linux evidence (sequential, idle machine)

- URL fetch unit tests (core lib): 5/5, within lib 81/81.
- URL fetch integration (synthetic servers): 8/8 —
  plain text exact text/hashes, HTML extraction, redirect
  chain final-URL update, redirect-to-private refusal,
  redirect-loop refusal after 6 requests, zero-hit
  adversarial refusals (credentials, 10/8, 169.254/16,
  strict-policy loopback/port, ftp, [::1]), body/type
  bounds (oversized, PDF, latin charset, invalid UTF-8,
  404; chunked accepted), HTTPS via rcgen (IP-SAN
  self-signed as its own root succeeds; wrong-name cert
  refuses).
- Collider URL tests: 4/4 — exact provenance + wire
  text (URLs/hashes absent from the provider wire),
  forged/stale metadata refused before traffic (0
  provider hits), transfer + backup/restore survival,
  fetch op returns reviewed text without saving (0
  provider hits, strict-policy refusal without the
  loopback flag).
- Focused Rust suites: collider 49 passed + 1 ignored
  (real-v10 fixture, ignored by default), transfer 19/19,
  forge_restore 4/4, backup_restore 7/7,
  backup_artifacts 5/5, opportunities_restore 3/3,
  entities_roundtrip 7/7, url_fetch 8/8, lib 81/81.
- Node UI: 99/99 (was 96 + 3 new: session races incl.
  cancel, response-to-provenance mapping, URL parser +
  saved-view rendering + legacy body without url fields).
- Browser battery: 25/25 across 21 specs — collider,
  collider-document-input, collider-url-input (canonical:
  fetch, review, race, source switch both directions,
  4 rejections, cancel, collide, save, restart,
  backup/fresh-restore with identical body), instance
  anchors, integrated acceptance, memory acceptance,
  memory context, picker reliability (5), pipeline
  overview, project context, review anchors, review
  document input, review rejected selection, transfer,
  transfer concurrency, transfer identity, forge,
  forge deliverable, forge review, forge revision,
  backups.
- Strict Clippy (`--workspace --all-targets -- -D
  warnings`): clean. Two findings fixed during the
  slice: a large-enum-variant lint on the fetch stream
  enum (TLS variant boxed, as the save-claim enum does)
  and a bool-comparison lint in a test.
- rustfmt `--check`: clean. tsc: clean. Contracts
  `--check`: exit 0. Docs validation: PASS, 191 files.
- API build and web build: pass.
- full core 386/4 (only the 4 known live-Ollama semantic_workflow environment failures)
- providers+API 144/0

## Windows rerun checklist (for the independent verifier)

1. Fetch the delivered bundle; confirm the tip and the
   ancestry `144acc715` (review tip) over the runtime
   base.
2. Run the canonical URL spec
   (`tests/e2e/collider-url-input.spec.ts`) with the
   loopback test policy enabled for the API process, as
   the spec does.
3. Run the document specs (canonical, DI-01/DI-02 probe,
   DI-03 rejected-selection probe) unchanged.
4. Run the focused Rust suites listed above.
5. Confirm no real external site is contacted: the
   synthetic content server is the only fetch target.
6. Native Tauri file-dialog behavior is out of scope
   for this slice, as it was for document input.

## Still open (not closed by this slice)

- Independent Windows rerun of this slice (checklist
  above) and of M8 Slice 4.
- M7 native desktop acceptance + preview promotion.
- §32 execution sandbox; M5 production gates.
- M8 is NOT complete: 3D visualization remains
  deferred.
