# URL input repair2 — focused Windows verification — 2026-10-31

Incoming runtime d0a86d5e34884a2d604b5cf21ab586876e170786 over reviewer239dc58eb779a73c93398bbc46be4b8c17cd2232. ZIP SHA256f664228e3c1a724e57e3feb6230131a039072879581d0d863ee3adfaf0824c3c; bundle5efa1ad680c227ab6aa051d9549fd02776ac6ac720dbde06f150544e596ce68f. Bundle verifies; tip/ancestry/hash match. Original review_url_fetch and review_url_tls_record files unchanged.

## Acceptance scope and review

Verify reserved-address refusal, the original20s plain-body deadline and the remaining single encrypted-record deadline at500ms, preserving normal TLS verification, pinned validated destinations, response bounds and production test-policy gating. The repair introduces DeadlineTcpStream beneath rustls and plain transport. Each underlying socket read/write checks the same deadline and shrinks its timeout; record assembly no longer hides repeated TCP waits from the budget. No runtime implementation change by reviewer.

Fresh Windows transport evidence: review_url_fetch2/2, encrypted-record reviewer probe1/1 (0.55s whole test), URL integration21/21 (22.71s), including fast/slow-but-valid fragmented success and slow fragmented refusal, redirects and TLS certificate refusal. UI/contracts99/99, TypeScript/contracts/docs198 files pass. API/web builds, strict core all-target Clippy and browser results are recorded below after completion, not inferred from Linux evidence.

## Boundaries

Synthetic local HTTP/TLS servers and provider only; no live external-site inference, API credentials, model downloads, pushes, CI or promotion. DNS timeout still bounds caller wait rather than terminating the platform resolver thread; this disclosed limitation remains. Browser file injection and API workflows do not certify native Tauri dialogs, the complete product, release installers, M5/G1, remote CI or generated execution. The old release-tooling application baseline remains separate; only its three qualified directories may be integrated onto this newer runtime. Owner previews/data untouched. Broad Linux totals are incoming evidence, not independent Windows totals.

## Final independent Windows results

PASS: original reviewer probes3/3 (encrypted-record whole test0.55s), URL integration21/21; total24 scoped Rust cases. UI/contracts99/99. Strict core all-target Clippy with warnings denied, formatting, TypeScript, contracts drift, documentation, API and web builds pass. Existing large web chunk warning retained. Browser4/4 sequential: document32.4s, URL36.6s, DI-01/02 race30.6s, DI-03 rejected selection28.4s, total2.2min. These workflows include save/restart/backup/fresh restore. URL-01, URL-02 and URL-02b closed in this focused scope. Shared test lease released. No new blocker found; not full-product or production certification.
