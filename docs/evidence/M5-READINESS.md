# M5 readiness — integration assessment in progress

Date: 2026-10-20. M5 implementation is not authorized by this task. M2–M4 are not yet locally complete or release-certified.

Inspected checkpoints: integration `5dd40d3`, retained M2 `bd2e2a9`. Both worktrees were clean at inspection. [Ownership/checklist](../handoffs/M2-M4-integration-owner.md).

| Requirement | Observed implementation/evidence | Remaining acceptance |
|---|---|---|
| AN-004 Memory OS; AN-024 project contracts | Existing M2.1–M2.5 workflow evidence retained | Regression after vault/schema integration |
| AN-018/031 vault privacy | Core crypto/generations/session tests; native IPC synthetic-provider workflow | Real native credential/recovery controls, UI workflow, browser authorization, expiry/revocation/publication race |
| AN-019/033 encrypted recovery/export | Core archive/restore functions | Application encrypted export/backup/fresh restore; tamper/wrong-key/cancel/interruption |
| AN-005 retrieval | FTS and synthetic seeded-vector tests | Application document/query embeddings, production schema checks, reranking distinct from RRF, real-model semantic/temporal/contradiction evaluation |
| AN-006/009 local Ask | Discovery, offline and missing-evidence paths | Selected installed runtime/model, successful desktop/browser answers, streaming/backend cancellation/timeouts/restart |
| AN-010 Context Compiler | Separate section compiler; Ask has separate minimum implementation | Unified Ask/compiler policy; semantic query vectors; complete chunks/Unicode/budgets/source lines |
| AN-011 Big Brain | Design document only | Design traceability; no implemented deliberative workflow claim |
| AN-002 cross-edition UI; AN-029 jobs | Earlier scoped workflow evidence | Updated native/browser regression and local M2.8 |

## Current verification

Read-only probes of `http://127.0.0.1:11434/api/tags` and `http://127.0.0.1:1234/v1/models` timed out/refused; `ollama` and `lms` were not found on PATH. No model selected, downloaded or invoked. Real-model acceptance is pending an available Owner-selected model; independent transport/security repairs continue.

New repairs in progress: production provider used `CREDUIWIN_GENERIC` with `CredUIPromptForCredentialsW` (different API flag family); replace with generic/always-show/non-persistent flags, guard cancellation buffers and restrict synthetic bypass to debug builds. Context compilation sliced UTF-8 at arbitrary byte offsets and discarded section tails; repair with bounded UTF-8 chunks and exact content line offsets. Exact verified implementation SHAs and final commands will be recorded after checks and commits.

Remote CI/protected publication: deferred by Owner capacity policy. Independent crypto audit: pending pre-production gate under accepted engineering risk. Audit monitor: paused. None is recorded as passed.

First dependency-safe M5 task: detailed provider/authentication/Zero-Cost-Lock contract inventory and design, after required M2–M4 contracts and local workflows pass. M5 readiness is currently **not established**.

## Verified repair checkpoint and follow-up

### Owner-selected small models — 2026-10-20

Owner explicitly approved installing Ollama if needed and downloading `qwen2.5:0.5b` for generation and `all-minilm:22m` for embeddings. Windows Package Manager installed Ollama 0.40.5 from its official GitHub release and verified the installer hash; exit 0. Runtime responds at `http://127.0.0.1:11434`. No cloud model, sign-in, payment or real user memory was used.

Both CLI pulls exited 0 and verified model digests. Local `/api/tags` reports generation size 397,821,319 bytes, digest `a8b0c51577010a279d933d14c2a8ab4b268079d44c5c8830c0a93900f1827c67`; embedding size 45,960,996 bytes, digest `1b226e2802dbb772b5fc32a58f103ca1804ef7501331012de126ab22f67475ef`. The runtime installer itself was approximately 1.58 GB, separate from model sizes.

Direct local API smoke requests completed successfully with `keep_alive:0` to unload after each call. Generation used 2,048 context, 64 maximum output tokens and temperature 0 with a synthetic fact. It returned 31 tokens, `done:true`, server total duration 45.664 seconds. The answer repeated the fact but added an unsupported geographic interpretation of local-first: this is an observed quality failure, not a correctness pass. `/api/embeddings` returned 384 dimensions for synthetic text. These are real-runtime API checks, not ANANSI desktop/browser inference acceptance. The previous runtime-unavailable observation above is historical and this specific resource blocker is now resolved. ANANSI's current 30-second inference timeout and nonstreaming requests require further implementation/verification on this machine.

`bedba8c` records ownership, authoritative status corrections, production Win32 prompt flags/guarded cleanup/debug-only synthetic bypass, complete UTF-8-safe section chunking and hybrid failure propagation. No production prompt interaction is claimed. Computer-use guidance forbids automating authentication dialogs, so production native credential interaction needs manual verification; synthetic provider coverage is retained separately.

Follow-up implementation routes Ask through the shared section Context Compiler, propagates canonical integrity failures, rejects missing workspaces, removes hybrid/context guessed-schema metadata fallbacks, and caps embedding response allocation at 256 KiB. Source commit follows the final checks.

Executed locally (exit 0 unless noted):

- `node scripts/cargo.mjs test -p anansi-core --test context_compiler --locked`: 7 passed, including Unicode content preservation/line offsets and tampered canonical Markdown refusal.
- `node scripts/cargo.mjs test -p anansi-core --test hybrid_retrieval --locked`: 6 passed, including invalid query/corrupt FTS and nonfinite weight refusal. Seeded vectors remain synthetic evidence.
- `node scripts/cargo.mjs test -p anansi-core --test ask_anansi --test vector_embeddings --locked`: 5 Ask and 5 embedding tests passed; inference server is synthetic, not a real selected model.
- `node scripts/cargo.mjs check -p anansi-desktop --locked`: passed.
- `node scripts/cargo.mjs clippy -p anansi-core -p anansi-desktop --all-targets --locked -- -D warnings`: passed.
- `node node_modules/typescript/bin/tsc --noEmit`: passed.
- `node --import tsx --test tests/contracts.test.ts packages/ui/tests/components.test.tsx`: 30 passed.
- Web Vite production build: passed with existing >500 KiB bundle warning.
- `node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test tests/e2e/ask_anansi.spec.ts tests/e2e/search.spec.ts`: 2 passed (1.2 minutes), real browser/API offline Ask/disclosure and title search/update/restart, after rebuilding API. Shared external lease released normally.
- Documentation validation: passed, 74 Markdown documents, 90 source section owners, 16 milestone plans and 33 requirements. Master unchanged.

Initial negative-test input used nine words, which is within the implemented 16-term body-query limit; corrected to 17 terms. Initial combined test execution overlapped recompilation and hit Windows executable sharing error; serialized reruns above passed. No full workspace suite or new native UI acceptance is claimed.

### Real local Ask application acceptance — 2026-10-20

This later evidence supersedes the local Ask pending entries above only for the exercised cases. Shared Ask now uses the Context Compiler with the original question separate from extracted retrieval terms. Process-local bounded runs expose partial tokens through scoped start/poll/cancel/forget contracts; cancellation closes the backend socket. Ollama NDJSON and OpenAI-compatible SSE parsing support bounded chunked HTTP responses. The UI selects the installed model and endpoint explicitly and stores neither in browser persistence. Tauri main-window AI permission was missing and is now explicitly declared; remote windows receive no permission.

Executed, exit 0:

- `node scripts/cargo.mjs test -p anansi-core --test ask_streaming --locked`: 1 passed; actual socket closure, partial tokens, cross-workspace denial and forgotten capability denial using a deterministic synthetic server.
- Streaming protocol unit tests: 2 passed; existing Ask tests: 5 passed. These cover parser boundaries and synthetic transport, not model quality.
- `node scripts/cargo.mjs clippy -p anansi-core -p anansi-api -p anansi-desktop --all-targets --locked -- -D warnings`: passed.
- `node node_modules/typescript/bin/tsc --noEmit`: passed; UI/contracts: 30 passed; contract generation drift check passed (existing uint/float schema-format warnings).
- With `ANANSI_REAL_MODEL=1`, `node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test tests/e2e/ask_anansi.spec.ts`: 1 passed, 36.4 seconds. Real browser/API, installed `qwen2.5:0.5b`, visible streamed answer, SQLite answer/citations/disclosure and cancellation, plus missing-evidence/offline cases.
- `node scripts/build-desktop-test.mjs`: passed. `node scripts/with-application-test-lease.mjs scripts/verify-ask-native.mjs`: passed against actual Tauri/WebView2 UI, visible streaming, SQLite answer, selected-model disclosure and cancellation. Synthetic catalog retained at `C:/Users/robin/AppData/Local/Temp/anansi-ask-native-VIw2Hi`; screenshot is an ignored test artifact.
- Documentation validation passed: 74 Markdown files, 90 section owners, 16 milestone plans, 33 requirements; master unchanged.

Observed failures and repairs: first real browser answer echoed a duplicated question; removed the duplicate from the shared compiler prompt. Next real answer was correct but partial output was missed at 300 ms polling; 50 ms polling passed the unchanged streaming assertion. First desktop attempt raced page startup; added bounded readiness wait. Next desktop attempt exposed missing AI IPC permission; direct invocation confirmed `ai_command not allowed`, then scoped permission repair passed. That failed test was ended by closing its own test page after diagnosis; it is not counted as a pass.

Limitations: no full regression suite, restart/timeout acceptance, production vault credential interaction, browser vault authorization, encrypted application restore or real semantic cache workflow is claimed here. Cancellation means the application closes its inference connection; the deterministic test proves socket closure, not internal Ollama scheduler cancellation. Model answers can still be incorrect; UI explicitly asks users to check citations. Remote CI/publication and external audit remain pending.

Ask implementation checkpoint: `1b0ac90509f9711104ae82152056c47c60525221`. Working tree was clean after commit. Complete-history source bundle verified at `C:/Users/robin/Documents/ChatGPT/Anansi-source-recovery/M2-M4-ask-1b0ac90/checkpoint.bundle`, SHA-256 `F130F2F740B77FA3557124E78D3D71CA7F25BEA5A483D321A6B16EF0B9B19384`; independent restore not yet executed. No push was attempted.

### Cache invalidation transaction repair

Acceptance defined before repair: a cache deletion failure must abort the canonical revision and FTS transaction, preserve the original current pointer/title, and allow a later successful retry to remove stale vectors. `sync_index_tx` previously discarded the cache invalidation result. It now propagates failure. `node scripts/cargo.mjs test -p anansi-core --test embedding_invalidation --locked` exited 0, 1 passed, using a real Core-created schema and a deterministic SQLite abort trigger. This does not implement embedding population or semantic retrieval.

Regression command `node scripts/cargo.mjs test -p anansi-core --test body_search --test vector_search --locked` exited 0: 1 body workflow and 5 synthetic-vector ranking/filter/invalidation tests passed. Formatting, diff whitespace and documentation validation passed. Existing vector tests still use synthetic schemas/vectors; they do not establish a real embedding workflow.

Manual production vault verification is in progress with the Owner. `scripts/manual-vault-acceptance.mjs` prepares only synthetic records and a verified plaintext backup, uses the shared application lease and disables the synthetic credential environment. It never operates or captures native credential/recovery dialogs and makes no automated pass claim. The user must close the application to release that lease normally.
