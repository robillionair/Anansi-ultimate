# Document input repair2 — focused Windows verification — 2026-10-30

Incoming runtime `0c62533c9f4f457d31b72f6f0737f12bde37a86d`, based on reviewer `d3f004bc4a9225c1818402b9a78e0c6b9f797b6d`; the Windows-reviewed lineage checkpoint `c50d6b03` remains an ancestor. ZIP SHA256 `d3ca0d6052faaabcea73bc8e71201239978e7c958faef243ec8392a232769689`; bundle `74cf2e8d8937d50a708dac1117ff14f156361918caeb3a76ea5da7b223692ec7`. Bundle verifies and incoming refs/hashes match.

## Acceptance scope

Close DI-01 (read before size rejection), DI-02 (obsolete rejected read changing loading/error state), DI-03 (oversized selection failing to invalidate an older pending read), only after the unchanged independent reviewer probes and canonical document workflow pass on Windows. No implementation changes by reviewer.

## Review and verification

The repair advances the beam load generation before every selection can return early. The oversized refusal clears obsolete loading state but preserves the last applied text/source. Existing pre-read and post-read limits and success/failure generation guards remain. All three browser specs are byte-identical to the preceding reviewer baseline. Backend Rust, schema and contracts are unchanged from the earlier reviewed document runtime; their prior passes are retained as historical evidence, not new rerun results.

Fresh Windows checks: UI/contracts96/96; TypeScript, contracts drift, docs189 files, web production build and git diff validation pass. Existing large-bundle warning remains. Browser results are recorded below and in the delivery log. Browser tests use rebuilt current web assets plus the previously built unchanged API runtime, real authenticated API commands, synthetic loopback inference, temporary E: roots, and the shared application-test lease. Every workflow includes save/restart/backup/fresh restore.

## Limits

This is focused backend/browser acceptance of the document slice and repairs. Playwright file injection is not native Tauri file-dialog acceptance. Native picker filter/cancel/unicode behavior, full-product acceptance, preview promotion, M5 production gates, G1 external audit, generated execution sandbox, remote CI and remaining M8 scope stay open. No cloud calls, real credentials, model downloads, owner-data migration, pushes, merge or promotion. Antigravity release tooling remains separate.

## Final result

PASS: canonical document workflow31.0s, unchanged DI-01/DI-02 probe30.6s, unchanged DI-03 rejected-selection probe31.4s; 3/3. DI-01, DI-02 and DI-03 closed in focused Windows scope. Shared lease released; clean application shutdown by the test harness.
