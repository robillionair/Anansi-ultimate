# M8 acceptance repair — independent Windows verification

Incoming `1ccda62d92ae800b6730ee01aee9e8eac3a17b68`; ancestors `c50de23b1`
and `ffbeee40a` confirmed. ZIP SHA-256
`1719952c2d0a81624f519d52898a5fb5f9435eb1ac3b6fe39d219b3e6c2c5153`;
bundle SHA-256 `b940083b1dd943cdc8f865dd48c7a002ae141f5e8c0a7e987f16d309fed8c89a`.
Bundle integrity and all 6 refs checked. Isolated worktree/cache/temp on E:;
existing preview, launchers and personal roots are not modified.

Acceptance plan: verify the unchanged six earlier probes, all 16 delivered
transfer cases, new receipt protections, exact-beam preservation, browser transfer
workflow, independent identity and two-process atomicity. Run builds/contracts,
shared UI/TypeScript, focused strict Clippy, formatting and docs. Keep actual
Windows results separate from Linux claims and native acceptance/promotion.

Inspection identified a receipt-integrity gap: receipt SQL readers bypassed the
body-hash check in ordinary artifact reads. The new expected-success test
`corrupt_receipt_hash_refuses_retry_and_forge_before_provider_traffic` failed on
the incoming production code: ordinary read refused the invalid checksum, while
transfer retry and Forge start both succeeded, with exactly one synthetic provider
hit. Only the receipt's stored checksum was corrupted; its body and bindings were
unchanged. This is a checksum-consistency finding, not evidence of an external
attacker capability or cryptographic authenticity.

The local correction adds the existing body-hash verification to the shared
`transfer_receipt_rows` reader, before either approval admission or transactional
retry may consume a receipt. It returns the existing ContentIntegrity refusal;
no stored row is repaired, no approval minted, no schema or DTO changed. Acceptance:
the same probe must pass with zero provider hits, valid receipts must still support
the browser transfer/retry/restore flow and two-process idempotency, and strict
Clippy/formatting must pass. The new probe now passes (25.56 s, zero provider
hits); before the correction it failed in 26.33 s with one hit. Browser checks
passed 4/4. The correction is committed as
`5b3cf958f9c35e45c9226d3988ae82b414b72d13`; strict core all-target Clippy and
the rebuilt API passed on that implementation. No test assertion or timeout was
weakened. The final reviewed baseline must include this correction, rather than
using incoming `1ccda62d` alone.

All six prior `acceptance_` backend function blocks compare unchanged to `c50de23b`.
Incoming Windows results: all 16 delivered transfer cases passed (479.27 s);
the retained Collider exact-beam probe passed (1/1, 16.93 s, other 12 filtered).
Delivered UI/contracts 78/78, TypeScript, web/API builds, contracts drift and initial
docs passed. Contracts emitted the existing ignored uint/float-format notices;
Vite emitted its existing large-chunk advisory. Neither warning was suppressed.
Final local-correction browser verification: canonical Windows Playwright config,
one worker, shared application-test lease, 4/4 in 7.5 minutes:

- Collider: no inference before submit, cancellation, immutable save, restart and
  backup/fresh restore (2.9 min).
- Transfer concurrency: two real API processes on one synthetic root, identical
  simultaneous transfer returns one concept; a different brief creates another
  (30.0 s).
- Independent transfer identity: edit after approval clears the receipt; switching
  experiments resets the draft (56.9 s).
- Full transfer: preview/edit create nothing, exact experiment-version switch,
  explicit approval, idempotent retry, Forge specification generation as a separate
  action, restart and backup/fresh restore (3.0 min).

Focused Windows backend/browser acceptance is **passed with the local correction**.
Incoming `1ccda62d` alone is not the final accepted implementation. No assertion,
test limit or browser configuration was weakened, and no flaky rerun was needed.
The shared application-test lease was released normally.

Native acceptance/promotion of these changes remains pending; no executable or
launcher was installed or changed. Live providers, real credentials/OS-keyring
acceptance, G1 external audit, remote CI/protected publication and generated-code
execution remain open. The tests used synthetic data and loopback servers only.
No full final-tree Windows core/API/providers suite or live Ollama pass is claimed.

## Reproduction

Use synthetic roots and `TEMP`/`TMP` on E:, the existing cached toolchain and
`CARGO_TARGET_DIR=E:/Anansi/build-cache/preview-m7-slices2-3`. The application
test lease is mandatory; no existing preview process or root is reused.

```text
node scripts/cargo.mjs test -p anansi-core --test transfer --locked
node scripts/cargo.mjs test -p anansi-core --test collider --locked acceptance_exact_beams_are_preserved_on_wire_and_save -- --exact
node scripts/cargo.mjs test -p anansi-core --test transfer --locked corrupt_receipt_hash_refuses_retry_and_forge_before_provider_traffic -- --exact
pnpm test:ui
node node_modules/typescript/bin/tsc --noEmit
node scripts/contracts.mjs --check
pnpm build:web
node scripts/cargo.mjs build -p anansi-api --locked
node scripts/cargo.mjs clippy -p anansi-core --all-targets --locked -- -D warnings
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test tests/e2e/transfer.spec.ts tests/e2e/transfer-identity.spec.ts tests/e2e/transfer-concurrency.spec.ts tests/e2e/collider.spec.ts --config playwright.config.ts
node scripts/cargo.mjs fmt --all -- --check
node scripts/validate-docs.mjs
```

The full 16-case transfer suite ran before the local hash correction; the focused
new probe and subsequent browser cases run after it. Do not relabel this as a
complete final-tree 17-case Rust suite or a full Windows core/API/providers suite.
The browser harness reads `target/debug/anansi-api.exe`: in this isolated worktree
that is a copy of the rebuilt cached API executable, not the running preview.

Final phase-boundary checks passed: formatting, Git diff validation and documentation
validation (133 Markdown files); the application-test lease is absent after normal
cleanup. Current status/matrix/plan/recovery point to this review; the earlier red
report remains preserved as historical evidence. Continue from the delivered
`codex/review-m8-acceptance-repair` ref containing `5b3cf958f`, with further scope
assigned separately. No preview promotion or Slice 3 implementation occurred.
