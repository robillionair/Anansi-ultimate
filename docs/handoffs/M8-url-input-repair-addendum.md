# M8 URL input repair — delivery addendum

Baseline: reviewer tip `db8d4cb9320e271709eb4c31b2b8662f8c8aa5dd`
(review branch `codex/review-m8-url-input`), over incoming
runtime `8cf6ff86fe31014257a334a8be4c3cd2bba80890`.
Branch: `muse/m8-url-input-repair`.
Design: `docs/handoffs/M8-url-input-repair-design.md`,
committed before any repair code. Reviewer probes in
`crates/core/tests/review_url_fetch.rs` preserved
unchanged (verified: empty diff) and passing.

## URL-01: globally-routable classification

The IPv6 classifier was a deny-list with a default-true
tail, admitting reserved space (`fec0::1`, `::2`). It is
now an allowlist: only ordinary global unicast inside
`2000::/3` minus the special-purpose exclusions inside
that range — the whole IANA-special `2001::/23` block
(covering Teredo, PCP/TURN anycast, benchmarking,
ORCHID, ORCHIDv2, AS112-v4), documentation
`2001:db8::/32` and `3fff::/20`, AS112
`2620:4d:4000::/48` — is public. IPv4-mapped
addresses are classified by their embedded IPv4
address; 6to4 addresses by their embedded IPv4
address. IPv4 keeps its explicit deny-list, audited
against the IANA registry. A mixed DNS answer is
refused when ANY resolved address is non-public (now
a unit-tested `all_public` helper). WHATWG numeric
IPv4 literal forms (hex/decimal/octal) are proven to
normalize to dotted-decimal before classification.
Connection pinning is preserved: the connection goes
to the first validated address, never a re-resolved
name, and every redirect target is re-validated.

One correction during the repair: the first allowlist
draft enumerated exclusions individually and missed
documentation `3fff::/20` (inside `2000::/3`), which
its own new boundary test caught. The rule was then
strengthened to refuse the whole `2001::/23` block at
once, so future special assignments inside that block
are refused without enumeration.

## URL-02: one total deadline

The 20s total budget was previously checked only
between reads and hops, while each socket read waited
up to its own 10s inactivity timeout — the reviewer's
dribble (120 bytes at 200ms/byte) completed after
23.88s. Now a single `Deadline` spans DNS, connect,
TLS handshake, request write, header lines, chunk
headers/bodies, trailers, close-delimited bodies, and
every redirect hop. Every socket wait uses
`min(remaining, 10s)`; the TLS handshake is driven by
an explicit loop with deadline checks on every
iteration; DNS waits at most `min(remaining, 5s)` and
connect at most `min(remaining, 5s)`. No deadline was
increased: the total stays 20s. Elapsed time past the
deadline is bounded by scheduling granularity only.
The policy carries `total_timeout` (capped at 20s) so
tests prove the mechanism at short budgets; the
reviewer's own probe runs at the production 20s
budget and passes at 20.22s, under its 23s bound.

DNS resolver threads remain the one unboundable
piece: the system resolver has no timeout, so on
timeout the thread is abandoned (it carries no socket
and exits when the resolver returns) while the worker
itself still returns by the deadline. This is stated
honestly, not hidden.

UI cancellation is now labelled accurately: the panel
help text states that cancelling drops the outcome
while any in-flight backend fetch still ends within
its bounded total time and saves nothing.

## Loopback allowance: compile-gated

The handler read `ANANSI_URL_FETCH_ALLOW_LOOPBACK`
unconditionally, so a release build would honor an
ambient variable. It now reads the variable only under
`cfg(any(test, debug_assertions))`; in any other
build the policy is unconditionally strict. Selection
is a pure `policy_for_fetch` helper tested in both
branches. The existing `ai_command` fetch test keeps
its set/restore isolation and runs under `cfg(test)`.
The debug browser harness is unaffected.

## Linux evidence (sequential, idle machine)

- Reviewer probes, unchanged: 2/2.
- URL fetch integration: 18/18 (8 prior + 10 new:
  dribbled headers, Content-Length body, one large
  chunk, close-delimited body, trailers, slow-but-legal
  stream still succeeding, redirect to reserved IPv6
  and to mapped private, stalled TLS handshake,
  dribbled TLS record header, dribbled TLS body).
- URL fetch unit tests: 8/8 (5 prior + policy
  selection, mixed answers, literal encodings).
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
  validation: PASS, 195 files. API + web builds pass.
- full core 401/4 (only the 4 known live-Ollama semantic_workflow environment failures)
- providers+API 144/0
- No real external site was contacted; no model
  calls; all fetch targets were synthetic local
  servers.

## Windows rerun checklist (for the independent verifier)

1. Fetch the delivered bundle; confirm the tip and
   ancestry `db8d4cb93` (review tip) over the runtime
   base.
2. Run `crates/core/tests/review_url_fetch.rs`
   unchanged: both probes must pass.
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
