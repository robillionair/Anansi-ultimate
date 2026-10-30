# M8 URL input repair 2 — delivery addendum

Baseline: reviewer tip `239dc58eb779a73c93398bbc46be4b8c17cd2232`
(review branch `codex/review-m8-url-input-repair`), over
incoming runtime `947b87b53` (the URL input repair).
Branch: `muse/m8-url-input-repair2`.
Design: `docs/handoffs/M8-url-input-repair2-design.md`,
committed before any repair code. Reviewer probes
preserved unchanged (verified: empty diffs) and
passing: `review_url_fetch` 2/2 and the new
`review_url_tls_record` 1/1.

## URL-02b: deadline beneath rustls

The first repair shrank the socket timeout once in
`CappedReader::fill`, then called `StreamOwned::read`.
One such read can issue many underlying TCP reads
while assembling a single encrypted record, and each
fragment that arrives resets socket inactivity, so
the outer deadline could not run until rustls
returned. The reviewer's probe proved it: one genuine
TLS record dripped one raw byte per 20ms under a
500ms policy returned only after 2.228s.

Repair: a `DeadlineTcpStream` transport that enforces
the deadline on every single underlying socket read
and write beneath rustls — re-checking the deadline
and shrinking the socket timeout to `min(remaining,
10s)` before each call, and reporting expiry as a
timeout error. `StreamOwned` is generic over its
transport, so both the TLS and plain variants run
over it, and the manual handshake loop drives
`write_tls` / `read_tls` on it. Record assembly,
alerts, post-handshake traffic, and fragmented writes
are therefore all bounded by the same deadline. The
`CappedReader` deadline checks and caps stay as
defense in depth.

Preserved unchanged: rustls default certificate and
hostname validation, address pinning (connection to
the first validated address, redirect targets
re-validated), size bounds (32 KiB headers, 1 MiB
body, 8,000 chars text), and the 20s production
deadline. No timeout was increased; there is no
UI-only workaround (no UI change in this repair).

New regression coverage, using the reviewer's own
technique (encrypt a complete HTTP response into one
TLS record with a real trusted synthetic CA, then
send the raw record bytes on a schedule), which
distinguishes raw encrypted bytes from separate
complete plaintext/TLS records:

- Fragmented-but-fast record succeeds (reassembly
  still works).
- Fragmented-and-slow record fails at a 2s budget
  (own variant of the reviewer probe).
- Slow-but-valid fragmented record succeeds (the
  probe's 20ms cadence on a record small enough to
  finish inside a 3s budget — steady progress within
  budget is not punished).

## Linux evidence (sequential, idle machine)

- Reviewer probes, unchanged: `review_url_tls_record`
  1/1 (0.53s, was 2.228s), `review_url_fetch` 2/2.
- URL fetch integration: 21/21 (18 prior + 3 new
  fragmentation tests).
- Focused Rust suites: collider 49 passed + 1 ignored
  (real-v10 fixture), transfer 19/19, forge_restore
  4/4, backup_restore 7/7, backup_artifacts 5/5,
  opportunities_restore 3/3, entities_roundtrip 7/7,
  lib 84/84.
- Node UI: 99/99. Browser battery: 25/25 across 21
  specs (collider, document-input, url-input,
  instance anchors, integrated acceptance, memory
  acceptance, memory context, picker reliability x5,
  pipeline overview, project context, review anchors,
  review document input, review rejected selection,
  transfer, transfer concurrency, transfer identity,
  forge, forge deliverable, forge review, forge
  revision, backups).
- Strict Clippy (`--workspace --all-targets -- -D
  warnings`): clean. rustfmt `--check`: clean. tsc:
  clean. Contracts `--check`: exit 0. Docs
  validation: PASS, 198 files. API + web builds pass.
- full core 405/4 (only the 4 known live-Ollama semantic_workflow environment failures)
- providers+API 144/0
- No real external site was contacted; no model
  calls; all fetch targets were synthetic local
  servers.

Note: one browser-battery attempt during this repair
failed instantly on every spec with MODULE_NOT_FOUND
because the working directory was still `apps/web`
after the vite build step; no test ran or failed.
The battery was rerun from the repo root and passed
as recorded above. The lesson is recorded in
`~/AGENTS.md`.

## Windows rerun checklist (for the independent verifier)

1. Fetch the delivered bundle; confirm the tip and
   ancestry `239dc58e` (review tip) over the runtime
   base.
2. Run `crates/core/tests/review_url_tls_record.rs`
   and `crates/core/tests/review_url_fetch.rs`
   unchanged: all three probes must pass.
3. Run the URL integration suite and the focused
   Rust suites listed above.
4. Run the canonical URL browser spec and the
   document reviewer probes unchanged.
5. Confirm no real external site is contacted.
6. Native Tauri behavior remains out of scope.

## Still open (not closed by this repair)

- Independent Windows rerun of this repair.
- Windows verification of M8 Slice 4.
- M7 native desktop acceptance + preview promotion.
- §32 execution sandbox; M5 production gates.
- M8 is NOT complete: 3D visualization remains
  deferred.
