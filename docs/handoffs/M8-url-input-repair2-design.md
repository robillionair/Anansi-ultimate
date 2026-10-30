# M8 URL input repair 2 — design and acceptance criteria

Baseline: reviewer tip `239dc58eb779a73c93398bbc46be4b8c17cd2232`
(review branch `codex/review-m8-url-input-repair`), over
incoming runtime `947b87b53` (the URL input repair).
Reviewer probes preserved unchanged and must pass:
`crates/core/tests/review_url_fetch.rs` (2 tests) and
the new `crates/core/tests/review_url_tls_record.rs`
(1 test). No new features, no increased timeouts, no
UI-only workaround, no external-site or model calls.

## Defect (URL-02b): deadline sits above TLS record assembly

The first repair shrank the socket timeout once in
`CappedReader::fill`, then called `FetchStream::read`
through rustls `StreamOwned`. One such read can issue
many underlying TCP reads while assembling a single
encrypted record; each fragment that arrives resets
socket inactivity, and the outer deadline cannot run
until rustls returns. The reviewer's probe proves it:
one genuine TLS record dripped one raw byte per 20ms
under a 500ms policy returns only after 2.228s. The
earlier TLS body test did not cover this because it
flushed each plaintext byte as a separate complete TLS
record, returning control between records.

## Repair: deadline-aware transport beneath rustls

Wrap the TCP stream in a `DeadlineTcpStream` that
implements `Read` and `Write` and enforces the deadline
on every single underlying socket call:

- Before each socket read or write: re-check the
  deadline (expired → `TimedOut` I/O error) and set
  the socket timeout to `min(remaining, 10s)`, so the
  wait shrinks as the deadline approaches.
- On a socket error after the deadline: report it as a
  timeout, so rustls surfaces deadline expiry instead
  of a generic I/O failure.

`StreamOwned` is generic over its transport, so the
TLS variant becomes
`StreamOwned<ClientConnection, DeadlineTcpStream>`
and the plain variant holds the same wrapper type.
The manual handshake loop drives `write_tls` /
`read_tls` on the wrapper, so handshake, record
assembly, alerts, post-handshake traffic, and
fragmented writes are all bounded by the same
deadline. The `CappedReader` deadline checks and caps
stay as they are (defense in depth for the plain
path and for header/body accounting); the transport
is what closes the intra-record gap.

Preserved unchanged: rustls default certificate and
hostname validation, address pinning (connection to
the first validated address, redirect targets
re-validated), size bounds (32 KiB headers, 1 MiB
body, 8,000 chars text), and the 20s production
deadline (`TOTAL_BUDGET`; `with_total_timeout` still
caps at it).

## New regression coverage

All synthetic, using the reviewer's own technique
(encrypt a complete HTTP response into one TLS
record with a real trusted synthetic CA, then send
the raw record bytes on a schedule) to distinguish
raw encrypted bytes from separate complete
plaintext/TLS records:

1. Fragmented-but-fast record succeeds: the whole
   record sent back-to-back well inside the budget —
   reassembly still works.
2. Fragmented-and-slow record fails at a short
   budget (own variant of the reviewer probe at a 2s
   budget, dripped past it).
3. Slow-but-valid fragmented record succeeds: a
   small record dripped at 20ms/byte finishing inside
   the budget — steady progress is not punished.

## Acceptance criteria

1. Reviewer probes pass unchanged:
   `review_url_tls_record` (error, elapsed under
   1500ms at a 500ms policy) and `review_url_fetch`
   2/2.
2. New fragmentation regressions pass (fast success,
   slow failure, slow-but-valid success).
3. Earlier URL, collider, transfer, forge_restore,
   backup_restore, backup_artifacts,
   opportunities_restore, entities_roundtrip suites
   pass; node UI passes; affected browser battery
   passes (including the document reviewer probes).
4. Strict Clippy, fmt, tsc, contracts check, docs
   validation, API + web builds clean; full core
   shows only the 4 known live-Ollama failures;
   providers+API clean.
