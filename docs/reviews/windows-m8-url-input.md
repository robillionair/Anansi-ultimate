# M8 URL input — focused Windows security review — 2026-10-30

Incoming branch muse/m8-url-input at8cf6ff86fe31014257a334a8be4c3cd2bba80890, verified baseline144acc715a7a76b3a999c13c20b547461fa53ff6. ZIP SHA25618d1ba84fac9ca442a21253629cdb2994e9c4cffa774f185eee7a6be6155bff4; bundle1d336af4ace0e17f7ee4ac8bb077f678add4dd1232794509cc70ed0030b4b4c3. Bundle verifies, tip/hash/ancestry match. This ZIP is a complete implementation, not merely the short progress message supplied alongside it.

Verdict: changes required. Security probes first; browser/native acceptance and broad Windows certification not run after these blockers.

## URL-01 (P1): public address classifier admits reserved nonpublic IPv6

is_public_v6 denies selected blocks but returns true for the rest of IPv6, including fec0::1 (former site-local, now reserved) and ::2 (reserved). The independent expected-success test fails at fec0::1. Public-only fetch must not permit connection attempts to these destinations. IANA IPv6 Address Space identifies fec0::/10 as reserved and ::/8 as reserved with specific exceptions: https://www.iana.org/assignments/ipv6-address-space/ . RFC3879 describes the former site-local use: https://www.rfc-editor.org/rfc/rfc3879 . No connection to these addresses was made; this probe calls the production classification function.

Repair with an auditable globally-routable classification covering special-purpose/reserved/transition/mapped address handling for IPv4 and IPv6. Do not special-case only these two strings. Cover range edges, representative public positives, mixed DNS answers, redirect targets and literal encodings. Continue connecting to the already validated address, not a re-resolved name. Record policy/reference version.

## URL-02 (P1): total deadline missing within read loops

The independent synthetic local server sends a120-byte Content-Length body at one byte/200ms. Every socket read arrives before the10s inactivity timeout. fetch_url_text succeeds after23.88s despite its20s total deadline. CappedReader fill/read_exact_capped/read_to_end_capped/read_line do not enforce the deadline; a content-length response, one long chunk, a header line, trailers or a TLS handshake can keep a worker alive while dribbling data. This is not just the documented single blocked-read allowance: repeated reads never inspect elapsed time. Larger slow streams can continue far beyond any stated total bound.

Use one deadline across resolution/connect/handshake/write/headers/body/trailers/redirects, with remaining-time socket waits and bounded worker cleanup. Keep size bounds and certificate checks. Add regressions for dribbled plain/TLS handshakes, headers, Content-Length, one large chunk, close-delimited body and trailers. No increasing deadlines. UI cancellation currently discards outcomes while backend work continues; ensure the residual work is genuinely bounded, and label that behavior accurately.

## Additional source-level integration caution

ANANSI_URL_FETCH_ALLOW_LOOPBACK is read in the production command without a debug/test compilation guard; test_loopback relaxations compile in release. Ensure synthetic allowances cannot activate in a production build through an ambient environment variable. Preserve the strict public-only production behavior and test via explicitly isolated test configuration.

## Evidence and scope

Fresh Windows incoming URL suite8/8, UI/contracts99/99, TypeScript/contracts/docs192 files pass. Independent review_url_fetch0/2: slow-drip accepted after23.88s, reserved IPv6 admitted. Earlier document reviewer probes are preserved but not rerun on this new runtime. No app/browser build or promotion after blockers; no live external site/model/credential calls. Public IANA/RFC documentation was read separately to verify address classification. Reviewer changes are tests/docs only; no implementation or assertion weakening. All synthetic workers joined; E: test roots; owner data/previews and Antigravity tooling untouched.
