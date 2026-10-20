# M5 readiness — local integration verification and remaining gates

Date: 2026-10-20. M5 implementation is not authorized by this task. Local engineering implementation and regression across M2–M4 are verified on `codex/m2-m3-integration` through implementation commits `ee92afb` and `b6d472c`, with exact remaining manual/external gates documented below.

Inspected checkpoints: integration starting `5dd40d3`, intermediate `b543c7b`, implementation commits `ee92afb` and `b6d472c`; retained M2 `bd2e2a9`. [Ownership/checklist](../handoffs/M2-M4-integration-owner.md).

| Requirement | Observed implementation/evidence | Remaining acceptance |
|---|---|---|
| AN-004 Memory OS; AN-024 project contracts | Existing M2.1–M2.5 workflow evidence retained; contract/schema/onboarding/search/vault E2E regressions pass | Deferred remote CI/protected publication |
| AN-018/031 vault privacy | Core crypto/generations/session/epoch/publication-race tests; desktop UI and paired browser native authorization (`/v1/vault/*`, no secret DTOs); Win32 prompt flags + zeroized buffers | Manual human operation of real Win32 credential/recovery dialogs (`scripts/manual-vault-acceptance.mjs`); pre-production `G1-External-Audit` |
| AN-019/033 encrypted recovery/export | Application ciphertext backup, encrypted export, locked archive preview, tamper/wrong-key/cancel refusal, and fresh sealed restore in desktop and browser (`verify-vault-desktop.mjs`, `tests/e2e/vault.spec.ts`) | Manual human Win32 dialog confirmation; pre-production `G1-External-Audit` |
| AN-005 retrieval | FTS, explicit Search UI model/endpoint selection and workspace indexing, visible `.status-lexical-fallback` / `.status-semantic-active` indicators, atomic `SAVEPOINT` chunk publication, distinct post-fusion reranking (`rerankScore` + `rerankProvenance`), real-model paraphrase/revision-invalidation/restart E2E (`all-minilm:22m`) | Deferred remote CI/protected publication |
| AN-006/009 local Ask | Separate generation (`modelTag`) and embedding (`embeddingModel`) configuration; selected installed runtime/model (`qwen2.5:0.5b`, `all-minilm:22m`), desktop/browser streaming answers, backend socket cancellation, offline and missing-evidence disclosures | Model output quality remains small-model bound (UI instructs citation verification); deferred remote CI |
| AN-010 Context Compiler | Unified Ask/compiler section pipeline; separate `embeddingModel` query vector generation; UTF-8/line-offset preservation, budget packing, canonical integrity refusal | Deferred remote CI/protected publication |
| AN-011 Big Brain | Design document and formal state machine specification (`docs/designs/BIG_BRAIN_SPEC.md`, ADR-0030) | Design traceability only; no implemented deliberative workflow claim |
| AN-002 cross-edition UI; AN-029 jobs | Desktop Tauri/WebView2 (`verify-ask-native.mjs`, `verify-vault-desktop.mjs`) and Playwright browser (`onboarding`, `search`, `ask_anansi`, `vault`) parity | Manual production Win32 prompt check; deferred remote CI |

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

### Real loopback semantic embedding indexing and retrieval pipeline — 2026-10-20

Real semantic embedding generation and retrieval are implemented and verified against the locally installed `all-minilm:22m` model (384 dimensions) on loopback (`http://127.0.0.1:11434`):
- `crates/core/src/semantic.rs`: `embed_memory`, `embed_workspace_memories`, and `generate_query_vector`. Enforces fail-closed Sealed privacy refusal before inference, reads canonical current-revision metadata and integrity-checked content, performs loopback inference outside write transactions, and checks revision conflict at commit.
- `crates/core/src/vector.rs`: Added chunked HTTP transfer decoding (`dechunk_http_body`) for Ollama's chunked HTTP/1.1 response, added `revision_id` and `content_hash` columns with backwards compatibility to `memory_embeddings`, and verified revision consistency in `vector_search`.
- `crates/core/src/hybrid.rs`: Dynamically resolves semantic query vector using `generate_query_vector` with the configured model when `query_vector` is omitted.
- `crates/api/src/lib.rs` & `crates/core/src/ai_discovery.rs`: Added `/v1/ai/index-embeddings` command route.
- `crates/core/tests/semantic_workflow.rs`: Comprehensive end-to-end integration test validating Sealed creation refusal without active vault (`ErrorCode::VaultUnavailable`), Sealed embedding exclusion (`ErrorCode::InvalidMemory`), single memory embedding, revision/content_hash metadata in SQLite, real paraphrase query retrieval with semantic ranking, Sealed exclusion from search hits, and workspace batch indexing.

Executed locally (exit 0):
- `node scripts/cargo.mjs test -p anansi-core --test semantic_workflow --locked`: passed in 13.41s against live `all-minilm:22m`.
- `node scripts/cargo.mjs test -p anansi-core --locked`: all 22 core unit tests, 7 AI discovery, 5 Ask, 1 streaming, 7 backup/restore, 11 vault, 8 vault increment2, 1 vault runtime, 3 vault boundary, 1 vault search integration, 2 vault invalidation, 5 vector embedding, 5 vector search, 10 workspace roundtrip, and 0 doctests passed.
- `node scripts/cargo.mjs fmt --all -- --check`: passed (0 diffs).
- `node scripts/cargo.mjs clippy --workspace --all-targets --locked -- -D warnings`: passed (0 warnings).
- `npm run test:ui`: 31/31 passed.
- `node scripts/validate-planning.mjs --self-test`: passed (74 Markdown files, 90 section owners, 16 milestone plans, 33 requirements).
- `node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test tests/e2e/ask_anansi.spec.ts tests/e2e/search.spec.ts`: 2/2 passed (59.3s).
- `node scripts/with-application-test-lease.mjs scripts/verify-ask-native.mjs`: passed (actual native UI streaming, answer, disclosure, cancellation).

### Semantic UI indexing, visible lexical fallback, atomic publication, and reranking — `ee92afb` (2026-10-20)

Implementation commit `ee92afb` (`feat(search): wire explicit embedding indexing, visible lexical fallback, atomic publication, and reranking`) completes Tasks 1–5 and 7:
- **Separate Embedding and Generation Model Configuration (Task 1)**: Separated `embeddingModel` from `modelTag` across `ContextCompilationRequest`, `AskRequest`, `packages/contracts`, and `packages/ui/src/AskAnansi.tsx` (`#ask-embedding-model`). `ContextCompiler` generates semantic query vectors using `embedding_model` while recording the generation `model_tag` in the disclosure manifest.
- **Explicit Search UI Indexing and Visible Lexical Fallback (Tasks 2 & 3)**: Added `IndexEmbeddingsRequest` / `IndexEmbeddingsResponse` and extended `HybridSearchResponse` (`lexicalOnly`, `fallbackReason`, `embeddingModel`, `rerankScore`, `cosineSimilarity`, `rerankProvenance`). Wired `#search-embedding-model`, `#search-embedding-endpoint`, and `#search-index-embeddings` into `packages/ui/src/Search.tsx`, alongside explicit `.status-lexical-fallback` and `.status-semantic-active` banners so users always know when hybrid results are lexical-only vs. semantically fused and reranked.
- **Atomic Multi-Chunk Embedding Publication (Task 4)**: `embed_memory` in `crates/core/src/semantic.rs` splits document sections into bounded chunks and publishes the complete chunk set inside `SAVEPOINT anansi_embed_publish`, rechecking `current_revision`, `body_hash`, `privacy != 'sealed'`, and `status = 'active'` immediately before replacing prior vectors. Verified by `concurrent_edit_and_privacy_change_abort_atomic_embedding_publication` and `partial_chunk_insert_failure_rolls_back_complete_chunk_set` in `crates/core/tests/semantic_workflow.rs`.
- **Real UI Semantic Workflow (Task 5)**: Verified in `tests/e2e/search.spec.ts` (`real semantic UI workflow: save record -> index -> retrieve paraphrase with zero keyword overlap -> revise -> verify old embeddings invalidated -> restart and search again`) against live `all-minilm:22m` over loopback: creates a record, observes `.status-lexical-fallback` (`no_workspace_embeddings`) before indexing, indexes via `#search-index-embeddings`, retrieves a zero-keyword-overlap paraphrase with `.status-semantic-active`, revises the record to a new topic, verifies old embeddings are transactionally invalidated on edit, re-indexes, restarts the API server, and verifies persistent semantic retrieval after restart.
- **Post-Fusion Reranking & Temporal/Contradiction Evaluation (Task 7)**: `hybrid_search` in `crates/core/src/hybrid.rs` computes a distinct second-stage `rerank_score` combining RRF base score with recency decay, `MemoryStatus` lifecycle multipliers (`Active` > `Superseded`/`Contradicted`), confidence weighting, and cosine similarity boost, emitting human-readable `rerank_provenance` and filtering expired records via `validUntil`. Verified in `reranking_temporal_validity_and_contradiction_lifecycle_evaluation` (`crates/core/tests/semantic_workflow.rs`) against live `all-minilm:22m`.

### Paired browser native vault authorization and encrypted backup/export/restore workflows — `b6d472c` (2026-10-20)

Implementation commit `b6d472c` (`feat(vault): add paired browser native authorization and encrypted backup/export/restore workflows`) completes Task 6:
- **Origin/Workspace/Process-Bound Browser Native Authorization**: Added `WindowsCredentialProvider::for_browser` (`crates/core/src/vault/native_prompt.rs`) naming the paired requesting browser origin and workspace in the native prompt caption, while keeping initial vault `setup` and `seal` strictly desktop-only (`runtime_rejects_browser_setup_before_any_credential_input`). Exposed `/v1/vault/{operation}` in `crates/api/src/lib.rs` bound to verified loopback session/CSRF/origin headers and rejecting any secret-bearing DTO keys (`passphrase`, `password`, `recoveryKey`, `dek`, `token`).
- **Encrypted Backup, Export, Locked Preview, and Fresh Sealed Restore**: Extended `VaultRuntime` and `VaultService` (`crates/core/src/vault/runtime.rs`, `crates/core/src/vault/service.rs`) with `AdapterVaultState`, `rotate`, `ciphertext_backup`, `encrypted_export`, `preview_archive`, `restore_archive`, and `restore_archive_observed`, setting private leaf DACLs (`private_permissions`) on written `.anansi-vault-archive` artifacts. Wired desktop IPC (`apps/desktop/src-tauri/src/lib.rs`), HTTP API (`crates/api/src/lib.rs`), typed client (`packages/contracts/src/client.ts`), and shared UI (`packages/ui/src/Vault.tsx`, `packages/ui/src/Workspace.tsx`).
- **Observed Desktop & Browser Vault Verification**:
  - `node scripts/cargo.mjs test -p anansi-core --test vault_runtime --locked`: 2/2 passed (including desktop-only setup/seal enforcement, paired browser unlock/read/backup/export/preview/restore, wrong-key/tamper/cancel refusal, and interrupted restore preservation).
  - `node scripts/cargo.mjs test -p anansi-api --test vault --locked`: 1/1 passed (paired browser HTTP native authorization, secret DTO rejection, ciphertext backup/export/preview/fresh restore, wrong-key/tamper refusal, lock/restart denial).
  - `node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test tests/e2e/vault.spec.ts`: 1/1 passed (1.8m) in real browser UI against loopback API.
  - `node scripts/build-desktop-test.mjs` + `node scripts/with-application-test-lease.mjs scripts/verify-vault-desktop.mjs`: exited 0 against actual Windows Tauri/WebView2 application (`http://tauri.localhost/`), verifying setup, seal, unlock, authorized read, ciphertext backup, locked preview, encrypted export, tamper refusal, fresh sealed restore into a clean workspace root, manual lock denial, and restart persistence (`C:/Users/robin/AppData/Local\Temp/anansi-vault-desktop-VNaSJ7`).

### Local regression battery — 2026-10-20 (Task 8)

Executed locally on `codex/m2-m3-integration` at `b6d472c` (all exited 0):
- `node scripts/contracts.mjs --check`: passed (0 contract or schema drift).
- `node node_modules/typescript/bin/tsc --noEmit`: passed.
- `npm run test:ui`: 33/33 passed (`tests/contracts.test.ts` and `packages/ui/tests/components.test.tsx`).
- `node scripts/cargo.mjs fmt --all -- --check`: passed (0 formatting diffs).
- `node scripts/cargo.mjs clippy --workspace --all-targets --locked -- -D warnings`: passed (0 warnings).
- `node scripts/cargo.mjs test -p anansi-api --locked`: 17/17 integration tests passed (`ai`, `backup_restore`, `csrf_session`, `export_delete`, `fts_rebuild`, `health_workspace`, `jobs`, `lifecycle_history`, `memories`, `onboarding_workspace_settings`, `projects`, `relationships`, `search`, `security_headers`, `static_csp`, `vault`).
- `node scripts/cargo.mjs test -p anansi-desktop --locked`: passed.
- `node scripts/cargo.mjs test -p anansi-core --test semantic_workflow --test context_compiler --test hybrid_retrieval --test ask_anansi --test ask_streaming --test vector_embeddings --test vector_search --test body_search --test embedding_invalidation --test vault_search_integration --locked`: 36/36 passed (5 `ask_anansi`, 1 `ask_streaming`, 1 `body_search`, 7 `context_compiler`, 1 `embedding_invalidation`, 6 `hybrid_retrieval`, 4 `semantic_workflow`, 1 `vault_search_integration`, 5 `vector_embeddings`, 5 `vector_search`).
- Playwright E2E suites (under exclusive application lease):
  - `tests/e2e/onboarding.spec.ts`: 3/3 passed.
  - `tests/e2e/search.spec.ts`: 2/2 passed (including real `all-minilm:22m` semantic UI indexing, zero-keyword-overlap paraphrase retrieval, revision invalidation, and API restart persistence).
  - `tests/e2e/vault.spec.ts`: 1/1 passed.
  - `tests/e2e/ask_anansi.spec.ts` (with `ANANSI_REAL_MODEL=1`): 1/1 passed.

### Remaining local development prerequisite

1. **Manual Production Win32 Credential & Recovery Dialog Check (Owner-in-the-loop)**:
   - Because computer-use rules forbid automating OS authentication prompts, a human operator must run `node scripts/with-application-test-lease.mjs scripts/manual-vault-acceptance.mjs` (which launches the desktop app without `ANANSI_VAULT_TEST_PROVIDER` against a synthetic workspace and verified backup) and confirm real Win32 `CredUIPromptForCredentialsW` and `MessageBoxW` dialogs for setup, seal, unlock, wrong-passphrase refusal, prompt cancellation, and recovery unlock.
   - The Owner reported an unsuccessful recovery acknowledgement after repeating the original login credentials; the UI displayed an obsolete future-vault message. Prompt and error repairs are committed at `1d03b99` and retained in the later native-provider integration. No successful human retry, sealing, restart or recovery result has been reported. The manual prerequisite remains open. No credentials or recovery keys are recorded here.

The first bounded M5 implementation task is not yet prepared or started. Reassess after this local prerequisite passes and reconcile the current provider/authentication/Zero-Cost-Lock plan. Deferred production audit and publication checks below are separate from this local development prerequisite.

### Deferred production and publication gates

1. **Pre-Production External Cryptographic Audit (`G1-External-Audit`)**:
   - Remains an open pre-production/commercialization gate under explicit [Owner engineering risk acceptance](../approvals/M2-6-owner-risk-acceptance.md); never counted as passed by local tests.
2. **Remote CI & Protected-Branch Publication**:
   - Deferred by Owner capacity policy; local verification does not substitute for remote CI or protected `master` publication.

### Source publication checkpoint — 2026-10-20

Inspected clean implementation/documentation HEAD `5b7a87b3433e1023daa9f1aa9caedf4d9181f8a0`, three commits ahead of the fetched integration branch, with no remote-only commits. Fresh local checks exited 0: documentation validation, seven planning negative fixtures, TypeScript typecheck, all 33 UI/contract tests and diff whitespace validation. Earlier Rust/application results above are retained evidence, not reruns performed for this push. The Owner requested one source publication to `codex/m2-m3-integration`; its documentation checkpoint uses `[skip ci]` to avoid consuming unavailable Actions capacity. This does not certify required checks, publish protected master, or close the audit/manual gates. Exact pushed identity is verified separately against the remote after the push.

