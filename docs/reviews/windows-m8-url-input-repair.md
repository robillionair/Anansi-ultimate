# Windows URL repair follow-up — 2026-10-30

Incoming947b87b53705a5e1865e5d5be27c39ac3379f669 over reviewerdb8d4cb9320e271709eb4c31b2b8662f8c8aa5dd. ZIP SHA256f4a320c07fb272466525d8cf8ceedf290620f0c3874b4e053d293feed1489698; bundle553e8de8651cbbd3a589442cebaedcf06d86b9ad8486fa0381bd2b9ab88671e3. Bundle verification/hash/ancestry pass. Original reviewer probes unchanged.

Positive scoped results: review_url_fetch2/2 on Windows (plain slow-body deadline and reserved IPv6 refusal), UI/contracts99/99, TypeScript, docs195 files. Release command selection uses a compile-time test/debug gate, so ambient production flag activation is addressed by source inspection. No release binary was built in this review. IPv6 classification repair closes the original examples; this is not a certification of all network routes or future address allocations.

## Remaining URL-02b (P1): deadline sits above TLS record assembly

Independent regression review_url_tls_record.rs creates a real trusted synthetic TLS server, completes handshake, reads the request, encrypts a complete HTTP response into a TLS record and sends that record one raw byte every20ms. No invalid certificate, real site, model or provider is involved. With policy timeout500ms, fetch returns only after2.228s, exceeding the test's generous1500ms allowance. Result refuses eventually, but backend work is not bounded by its deadline. The exact deadline overruns, not answer content, are the finding.

Root cause: CappedReader::fill shrinks TcpStream timeout once then calls FetchStream::Tls.read through rustls StreamOwned. That single read can issue multiple underlying TCP reads to assemble one encrypted record; each fragmented byte resets socket inactivity, and the outer deadline cannot run until rustls returns. Muse's TLS body test flushes each plaintext byte as a separate complete TLS record, so it returns control between records and does not cover fragmentation inside a record. Manual handshake handling does not fix the established-session read path.

Required repair: enforce the same deadline at every underlying TCP read/write beneath rustls, including record assembly, alerts/post-handshake traffic and writes; a deadline-aware I/O wrapper or bounded socket-shutdown mechanism can do this. Keep normal certificate validation, address pinning, size bounds and the original20s production limit. No longer timeouts or UI-only cancellation fix. Preserve the new reviewer test and earlier probes unchanged. Add fragmentation coverage that distinguishes raw encrypted bytes from separate complete plaintext/TLS records, plus slow-but-valid success.

No production changes by reviewer; tests/docs only. Broad native/browser acceptance deferred after deadline blocker. No external-site/model requests, credentials, downloads, owner-data changes, preview promotion, pushes or CI. Synthetic listener/thread joined on completion. Existing affected URL-suite final counts are recorded below once completed; source logs retained under E:/Anansi/reviews/m8-url-input-repair-2026-10-30/.

Final incoming suite:18/18 Windows URL integration tests passed,17.86s; original reviewer probes2/2 passed,20.28s. New intra-record TLS fragmentation probe0/1 (deadline overrun). No other suite failure observed in the scoped runs. Shared application lease was not acquired for these direct synthetic transport tests.
