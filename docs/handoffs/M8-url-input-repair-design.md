# M8 URL input repair — design and acceptance criteria

Baseline: reviewer tip `db8d4cb9320e271709eb4c31b2b8662f8c8aa5dd`
(review branch `codex/review-m8-url-input`), over incoming
runtime `8cf6ff86fe31014257a334a8be4c3cd2bba80890`.
Reviewer probes in `crates/core/tests/review_url_fetch.rs`
(two tests) are preserved unchanged and must pass; they
are the acceptance gate for URL-01 and URL-02. No new
features, no promotion, no pushes, no CI. All tests
synthetic; no external-site or model calls.

## URL-01: globally-routable address classification

Defect: `is_public_v6` was a deny-list with a default-true
tail, so reserved IPv6 space outside the listed blocks
(`fec0::1`, `::2`) was admitted.

Repair: invert the IPv6 rule to an allowlist. An IPv6
address is public only when it is ordinary global unicast:

- IPv4-mapped (`::ffff:0:0/96`) → classify the embedded
  IPv4 address by the IPv4 rules.
- 6to4 (`2002::/16`) → classify the embedded IPv4
  address (bits 16–48) by the IPv4 rules, so a 6to4
  address naming a private IPv4 address is refused.
- Otherwise: inside `2000::/3` (the global unicast
  allocation) AND outside every special-purpose exclusion
  inside that range: Teredo `2001::/32`, benchmarking
  `2001:2::/48`, ORCHID `2001:10::/28`, ORCHIDv2
  `2001:20::/28`, documentation `2001:db8::/32`,
  AS112 `2001:4:112::/48` and `2620:4d:4000::/48`,
  PCP/TURN anycast `2001:1::/32`.

Everything outside `2000::/3` is refused by the rule
itself, which covers loopback, unspecified, link-local,
ULA, deprecated site-local `fec0::/10`, multicast,
discard `100::/64`, NAT64 `64:ff9b::/96` and
`64:ff9b:1::/48`, and all other reserved space without
enumerating it.

IPv4 keeps its explicit deny-list, audited against the
IANA registry: `0/8`, `10/8`, `100.64/10`, `127/8`,
`169.254/16`, `172.16/12`, `192.0.0/24`, `192.0.2/24`,
`192.31.196/24`, `192.52.193/24`, `192.88.99/24`,
`192.175.48/24`, `192.168/16`, `198.18/15`, `198.51.100/24`,
`203.0.113/24`, `224/4` (multicast and `240/4`
reserved). A mixed DNS answer is refused when ANY
resolved address is non-public; the check is extracted
into a pure `all_public` helper so the rule itself is
unit-testable. Literal encodings are covered because the
WHATWG URL parser normalizes numeric IPv4 forms
(hex/decimal/octal) to dotted-decimal before
classification; tests prove `0x7f000001`, `2130706433`,
and `0177.0.0.1` refuse.

Preserved: the connection still goes to the first
validated address (never a re-resolved name), and every
redirect target is re-validated by the same rules
before connecting.

References recorded in code: IANA IPv4 Special-Purpose
Address Registry, IANA IPv6 Special-Purpose Address
Registry, IANA IPv6 Address Space Registry (global
unicast `2000::/3`), RFC 3879 (deprecated site-local).

## URL-02: one total deadline

Defect: the 20s total budget was checked only between
reads and hops, while each socket read waited up to its
own 10s inactivity timeout. A dribbled stream (1
byte/200ms) therefore completed after 23.88s.

Repair: a single `Deadline` (start + total budget)
spans resolution, connect, TLS handshake, request
write, header lines, chunk headers/bodies, trailers,
close-delimited bodies, and every redirect hop.

- Before every socket wait, the socket read/write
  timeouts are set to `min(remaining, 10s)`, so the
  effective wait shrinks as the deadline approaches.
  Overshoot past the deadline is bounded by scheduling
  granularity only, not by a full timeout window.
- The TLS handshake is driven by an explicit loop
  (`wants_write`/`read_tls`/`process_new_packets`) with
  a deadline check and remaining-time timeouts on every
  iteration, instead of relying on the stream's lazy
  handshake.
- DNS resolution waits at most `min(remaining, 5s)`;
  the resolver thread is abandoned on timeout (it
  cannot be killed) but carries no socket, and the
  worker itself still returns by the deadline.
- Connect waits at most `min(remaining, 5s)`.
- The total budget stays 20s and no deadline is
  increased. The policy carries `total_timeout` so
  tests can use a short budget to prove the mechanism
  quickly; production uses 20s, and one regression
  runs at the production 20s budget (the reviewer's
  own dribble probe).

Regressions (all synthetic, short budgets unless
noted): dribbled headers, dribbled Content-Length
body (reviewer probe, production budget), one large
dribbled chunk, dribbled close-delimited body,
dribbled trailers, stalled TLS handshake, dribbled
TLS record header, dribbled TLS body. Size bounds and
certificate checks are unchanged.

UI cancellation: cancelling drops the outcome and
clears loading, while any in-flight backend fetch
still runs to the same bounded total deadline and
saves nothing. The panel help text now says exactly
that, so the behavior is labelled accurately.

## Loopback allowance: compile-gated

Defect: `ANANSI_URL_FETCH_ALLOW_LOOPBACK` was read in
the production command path with no compilation guard,
so a release build would honor an ambient variable.

Repair: the handler reads the variable only under
`cfg(any(test, debug_assertions))`; in any other
build the policy is unconditionally strict. The
selection is a pure `policy_for_fetch(env_flag,
allow_test_policy)` helper, unit-tested in both
branches, so the release branch (strict regardless of
the variable) is covered by a test even though the
test harness itself runs non-release. The existing
`ai_command` fetch test keeps its set/restore
isolation and runs under `cfg(test)`, where the flag
is honored by design. The debug browser end-to-end
harness is unaffected (`debug_assertions` holds).

## Acceptance criteria

1. Reviewer probes pass unchanged:
   `review_nonpublic_ipv6_destinations_are_refused`
   and `review_content_length_dribble_obeys_total_deadline`
   (error, elapsed under 23s at the 20s budget).
2. New classification boundary tests pass: range
   edges, public positives, mapped/6to4 embedded
   handling, reserved/transition blocks, literal
   encodings, mixed-answer helper, redirect to
   reserved IPv6 refused.
3. New slow-stream/TLS regressions pass at short
   budgets, plus the production-budget reviewer probe.
4. Loopback helper tests pass in both branches;
   strict production behavior preserved.
5. Existing URL, collider, transfer, forge_restore,
   backup_restore, backup_artifacts,
   opportunities_restore, entities_roundtrip suites
   pass; node UI passes; affected browser battery
   passes (including the document reviewer probes).
6. Strict Clippy, fmt, tsc, contracts check, docs
   validation, API + web builds clean; full core
   shows only the 4 known live-Ollama failures;
   providers+API clean.
