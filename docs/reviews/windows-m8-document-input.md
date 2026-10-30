# Windows review — M8 document input — 2026-10-30

Incoming: `faafa1c26a4c36ae57fa4371664178bc764c4690`, baseline `c50d6b032f71779000b39dc6abb6afef57adb4ef` preserved. ZIP SHA-256 `6f975691fb7dabece6fca50c24c6ee4dad8b4cf33a6ffb2aa9f9ca7f1877af5d`; bundle `70823a5473e88b36e37485f0ffb86308101fff733b931769e025e024472f0c11`. Bundle verification passed.

## Verdict

Changes required before document-input acceptance. Two independent frontend findings; no production changes by reviewer.

### DI-01 — file allocation is not bounded before reading (P2)

`packages/ui/src/Collider.tsx:1458` calls `file.arrayBuffer()` before `decodeDocumentBytes` checks the 65,536-byte limit. Picking an arbitrarily large file allocates/reads its entire contents first. The decoder's rejection prevents dispatch but does not enforce the intended ingestion resource bound. The independent browser probe intercepts `File.prototype.arrayBuffer` and observes a read of a 65,537-byte file. Refuse using `File.size` before reading, retain the post-read check, and test that a rejected file is never read.

### DI-02 — obsolete rejection mutates a newer load (P2)

The success path checks the load generation, but the catch at `Collider.tsx:1462` checks only mounted state. Sequence: existing valid beams; begin read A; begin read B; reject A while B is gated. A's catch displays its obsolete error and clears the shared loading flag, enabling Collide Ideas while B is still pending. The controlled browser probe confirms the button is enabled. This can send the previous beam instead of waiting for the user's newer selection. Gate failures by the same generation identity as successes; obsolete failures must change neither error nor loading state. Test edit/clear/new-load rejection interleavings without timing sleeps.

## Scoped evidence

- Four new backend document integration tests: 4/4, including exact wire/provenance, forgery refusal, bounds and transfer/backup/restore.
- UI/contracts: 94/94; TypeScript passes.
- Strict core all-target Clippy with warnings denied passes.
- Contracts drift, documentation validation (183 files before this review), API build and web build pass. Web retains its known large-chunk warning.
- Browser final results are recorded in the delivered evidence log; review assertions intentionally remain expected-success assertions and fail until repaired.

## Harness diagnostics

An initial browser invocation was started before the API rebuild completed, causing the old binary to remain in use and preventing Windows from replacing it. That run is diagnostic only, not application acceptance. The build then completed after test shutdown and the browser was rerun. The reviewer probe initially left its temporary Beam B text populated before the inherited canonical empty-beam checks; cleanup was added to isolate the probe from the inherited workflow. These harness corrections do not alter the two finding assertions.

Native Tauri file-picker behavior, full Windows workspace regression, preview promotion and standing production gates are not certified by this focused review. No cloud calls, credentials, model downloads, pushes or CI. Existing previews/data untouched.

Final Windows browser result: canonical document workflow 1/1 passed (32.0s). Independent review workflow failed only the two expected-success finding assertions (59.9s); its remaining inherited save/restart/restore assertions completed. Final lease released. Reproduce with `node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test tests/e2e/collider-document-input.spec.ts tests/e2e/review-collider-document-input.spec.ts --config playwright.config.ts` after building API/web. Evidence: `browser-final.log` in the delivery.
