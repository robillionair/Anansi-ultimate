# M3 — Search and local AI

Status: user explicitly authorized vault-independent next-phase work on 2026-10-19 while waiting for upstream audit feedback. M3.1 below is the bounded approved increment. M2.6/G1, M2.7 and M2.8 remain outstanding; this is a scheduling exception, not M2 completion or permission to bypass crypto review.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 11, 13, 18–19, 66–67; [canonical roadmap](../docs/ROADMAP.md).

Required scope: FTS5, authorized local runtimes/hardware detection, Ask ANANSI, streaming, cancellation, grounded citations; minimum context/privacy/cost enforcement. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: verified M2.1–M2.5 records, revision ownership, native/HTTP contracts and recovery suffice for title-only local retrieval. The latest user instruction permits this independent slice before M2 certification. Persistent/full-body indexing and inference must separately verify their privacy, resource and recovery contracts. No encrypted data exists or is indexed here; Sealed remains refused.

## Task sequence and preserved scope

| Task | Scope and dependencies | Coverage |
|---|---|---|
| M3.1 — Local title search | Existing current revision metadata; bounded request-local FTS5, filters, ranked results and exact-revision reader in both editions | AN-005 lexical subset; AN-002/018 shared privacy boundary; AN-033 provenance |
| M3.2 — Body indexing and retrieval evaluation | Detailed job/index design first; rebuildable full-body index, current-pointer synchronization, metadata/entity filters, deduplication, recency and evidence locations. Resolve encrypted index/vault interaction before enabling it | Master 10–11, 66, 74–78; AN-005/010/029/031 |
| M3.3 — Authorized runtime discovery | Detailed plan first; detect Ollama, LM Studio and compatible local inference servers; actual hardware analysis, suitable-model recommendations, user-chosen configuration and explicit download consent. No automatic install/download/inference. Apple Silicon MLX native verification remains M14 | Master 7, 18–19, 69–70; AN-009/020 |
| M3.4 — Minimum context and local Ask | Detailed plan first; deterministic privacy/context/cost checks, evidence pack, grounded citations, insufficient-evidence response, real authorized local inference, streaming/cancellation, restart/resource/error behavior | Master 11, 13, 16–19, 66–67, 71, 74, 78, 84; AN-006/009/010/018/029 |
| M3.5 — Integrated acceptance | Desktop/browser parity, offline/cancellation/restart/privacy/citation evaluation and recovery. M2 unfinished gates remain prerequisites to integrated certification | Master 77–78, 82–84 |

Embeddings, vector/hybrid merging, reranking and the complete Context Compiler stay M4. No paraphrase/semantic quality claim is made for lexical titles. Provider verification, budgets/routing and OAuth remain M5; minimum cost/privacy enforcement must precede M3 inference. Every later task needs a detailed file/test/acceptance plan before coding.

Retained detailed scope for later slices: runtime/hardware analysis must cover CPU, RAM, GPU, VRAM, unified memory, free storage and architecture, with truthful unavailable/unsupported results and offline operation. Before every AI request, enforce authorized retrieval, workspace permissions, Sealed cloud exclusion, restricted-content redaction, preserved citations, deduplication, context budget, cost policy, approved model and disclosure manifest. Provide **View What ANANSI Sent** with provider/model, source memories, redactions, estimated tokens and routing reason; minimum enforcement is not deferred until M4. Retrieval evaluation must retain paraphrase, contradiction, temporal-fact and missing-evidence fixtures, with page/section/sheet/slide/timestamp locations when supplied by ingestion. These are requirements, not current implementation claims.

## M3.1 implementation contract — defined before coding

Objective: a user searches saved record titles in the selected workspace, filters type/archive/expiry, pages ranked results and opens the exact returned revision with provenance. All nine knowledge kinds are covered. Bodies, relationship text, semantic retrieval and inference are excluded and visibly described as unavailable.

Files: create `crates/core/src/search.rs`, `crates/core/tests/title_search.rs`, `packages/ui/src/Search.tsx`, `tests/e2e/search.spec.ts`, `docs/evidence/M3-1.md`; modify `crates/core/src/lib.rs`, `crates/core/src/memory.rs`, `packages/contracts/src/client.ts`, generated schema/types/validators, shared navigation/workspace, native workflow verifier and relevant contract/UI tests. Update roadmap/architecture/decisions/status/matrix/recovery/state. No migration, applied-schema edit, new library, external runtime or cryptographic code.

Steps: (1) add optional `query` to existing list contract, validate before storage; reject history+query and unsupported syntax sizes; (2) build FTS5 in a separate in-memory SQLite connection from one current-metadata query, applying workspace/type/archive/expiry/Sealed policy before inserting; (3) tokenize bounded literal words, use bound MATCH parameters, Unicode61/BM25 and stable ID ties; (4) return the existing validated metadata DTO and pagination; (5) add read-only shared Search view with explicit scope, loading/error/empty/results/pagination and exact-revision literal reader; (6) test and run actual native/browser behavior before status claims.

Additional exact owners identified during implementation: `crates/core/src/readiness.rs` owns the sanitized INVALID_SEARCH error code, `crates/api/tests/memories.rs` verifies search over the existing authenticated route, and `crates/core/tests/title_search_limits.rs` verifies real corpus/metadata bounds and mismatched metadata identity. Existing route/IPC implementation and capability files require no extension.

Update `tests/e2e/onboarding.spec.ts` to require Search as an enabled navigation action while Files remains planned, preserving the keyboard/privacy/restart assertions. Update `docs/DEVELOPMENT.md` with the actual local Search workflow and reproducible entry points.

Bounds: at most 256 UTF-8 query bytes, eight alphanumeric terms, 10,000 eligible records and 32 MiB metadata. No partial corpus on overflow: return an actionable bounded-search error and suggest narrowing record type. Page size 1..100, existing offset cap. Quoted SQL/FTS operators are never passed as operators; each term is quoted and joined by AND. Empty/punctuation-only queries are rejected. No disk index, query history, body scan, external request or AI call. SQLite temporary storage is memory-only; in-memory copies have ordinary process/OS memory limits, not guaranteed forensic erasure.

Acceptance: native and browser search real saved titles, Unicode/diacritic matching, correct workspace/type/default visibility and exact revision/body/provenance opening; current titles replace old matches after edits/restart; stable pagination on unchanged data; literal HTML stays inert; malformed/oversized/unsupported requests fail clearly; Sealed metadata is never indexed; filters never broaden scope. No persistent index files or database schema change. Existing workflows remain passing. Pagination across intervening edits is a new current snapshot, not a promised stable cursor.

Verification: `pnpm check`, `node scripts/validate-planning.mjs --self-test`, `pnpm test`, `pnpm contracts:check`, `pnpm typecheck`, `pnpm test:ui`, `node scripts/cargo.mjs fmt --all --check`, `node scripts/cargo.mjs test --workspace --locked`, `node scripts/cargo.mjs clippy --workspace --all-targets --locked -- -D warnings`, `pnpm build:web`, `node scripts/cargo.mjs build --locked -p anansi-api`, `pnpm test:e2e`, build native test configuration and run the updated actual native/browser verifier. Inspect actual screenshots; retain synthetic evidence. Add targeted core tests for stale revisions, isolation, type/archive/expiry, literal operators, Unicode, bounds and integrity.

Recovery/DoD: preserve master and all original requirements; current schema 8 and backups stay compatible. Reverting this code changes no canonical data. Keep user changes and failed evidence. Commit/push a small checkpoint, verify exact-SHA required CI before protected-master publication, independently restore a source bundle and record actual HEAD/tree/digest/results. Report pending checks honestly. Stop after M3.1; no runtime/inference/crypto implementation. The hourly audit follow-up remains responsible for evidence review, not a waiver.

## M3.2 implementation contract — defined before coding

Objective: a rebuildable full-body FTS5 index across Markdown content files and SQLite metadata, current-pointer synchronization, metadata/entity filtering, grounded evidence locations (Markdown heading and line number), deduplication, recency weighting, Sealed privacy boundary, and an automated retrieval evaluation benchmark suite (paraphrases, contradictions, temporal facts, missing evidence).

Files: create `crates/core/tests/body_search.rs`, `crates/core/tests/retrieval_evaluation.rs`, `docs/evidence/M3-2.md`; modify `crates/core/src/search.rs`, `crates/core/src/memory.rs`, `crates/core/src/storage.rs`, `packages/contracts/src/client.ts`, generated schemas/validators/types, `packages/ui/src/Search.tsx`, `tests/contracts.test.ts`, and verification scripts. Update roadmap/architecture/decisions/status/matrix/recovery/state. No unvetted crypto, external runtimes, or network connections.

Steps:
1. Define query parser in `crates/core/src/search.rs` supporting literal multi-word AND queries, exact phrase matching (`"..."`), and prefix queries (`*`), bounded to 512 UTF-8 bytes and 16 terms with safe sanitization.
2. Implement rebuildable SQLite FTS5 table `memory_body_fts` and rebuild function `rebuild_search_index` scanning current active Markdown revisions and metadata, strictly excluding `WHERE privacy != 'sealed'`.
3. Implement current-pointer synchronization so memory creation, revision, archive, and restoration update the indexed record to match the active revision only.
4. Implement `body_search` returning ranked matching records with snippets, enclosing Markdown section headings (`# Heading`), and approximate line numbers.
5. Implement deduplication flagging (identical body SHA-256 hashes) and recency weighting combining BM25 relevance with temporal decay from `updated_at` and `event_date`.
6. Implement the retrieval evaluation benchmark suite in `crates/core/tests/retrieval_evaluation.rs` testing paraphrase baselines, contradiction resolution, temporal facts, and missing evidence.
7. Update shared contracts and UI Search component to display snippet, matched heading, line number, and full-body toggle.
8. Validate locally across Rust workspace, UI tests, evaluation suite, and native/browser workflows before status claims.

Bounds: at most 512 UTF-8 query bytes, 16 terms, 10,000 active records, 256 KiB body limit per record. Sealed memories are strictly excluded from the FTS5 index. Results paged with limit 1..100.

Acceptance: defined in [M3.2 evidence](../docs/evidence/M3-2.md). Rebuildable index accurately reflects all current plaintext records; Sealed records are never indexed; exact phrase, prefix, and metadata filters work reliably; grounded section headings and lines are returned; evaluation suite passes all benchmarks.

## M3.3 implementation contract — defined before coding

Objective: Detect local AI runtimes (Ollama, LM Studio, and compatible loopback inference endpoints) and perform hardware resource inspection (CPU cores, total/free system RAM, GPU detection/VRAM where available, free storage on workspace drive, system architecture). Provide model recommendations scaled to detected capacity with strict offline resilience, user-chosen endpoint configuration, explicit download consent requirements, and refusal of automatic model downloads or unprompted background inference.

Authority: Master Spec §7, §18–19, §67, §69–70; Requirements Matrix AN-009, AN-020, AN-026.

Files:
- Create: `crates/core/src/ai_discovery.rs`, `crates/core/tests/ai_discovery.rs`, `docs/evidence/M3-3.md`
- Modify: `crates/core/src/lib.rs`, `crates/core/src/storage.rs`, `crates/core/Cargo.toml`, `packages/contracts/src/client.ts`, `packages/ui/src/Navigation.tsx`, `packages/ui/src/Workspace.tsx`, `tests/contracts.test.ts`
- Update: `docs/DECISIONS.md`, `docs/BUILD_STATUS.md`, `project-state.json`

Steps:
1. Define hardware analysis types (`HardwareProfile`: architecture, logical CPU cores, total/available physical RAM, disk available space, optional GPU name and VRAM) and runtime probe types (`RuntimeKind`: Ollama, LmStudio, CustomLoopback; `RuntimeStatus`: Available, Offline, Unsupported).
2. Implement native hardware discovery in `crates/core/src/ai_discovery.rs`:
   - On Windows, read memory status via standard Win32 API (`GlobalMemoryStatusEx`) and disk free space via standard filesystem metadata.
   - Capture CPU architecture and thread count using `std::env::consts::ARCH` and `std::thread::available_parallelism`.
   - Provide fallback heuristics when high-privilege GPU discovery is unavailable without crashing or blocking.
3. Implement non-blocking loopback runtime probe (TCP connect / HTTP GET probe to configured loopback ports: Ollama default `http://127.0.0.1:11434/api/tags`, LM Studio `http://127.0.0.1:1234/v1/models`) with a strict 2-second timeout.
4. Implement model recommendations based on detected RAM/VRAM:
   - >= 16 GB RAM/VRAM: 8B–14B quantized models recommended (e.g. Llama 3 8B, Qwen 2.5 7B/14B).
   - 8–16 GB RAM/VRAM: 7B–8B Q4 quantized models recommended.
   - < 8 GB RAM: lightweight 1B–3B models or CPU-only advisory.
5. Guarantee strict constraints:
   - Probing must never trigger automatic downloads, model pulling, or cloud telemetry.
   - Offline environments return truthful `Offline` / `Unavailable` statuses without crashing.
   - Explicit consent is required before any model configuration or future inference.
6. Expose the `ai_discovery` query through the Core API and TypeScript contracts.
7. Integrate UI component under "SYSTEM -> Local Models" in navigation, displaying hardware capabilities, detected local servers, and recommendation cards.
8. Validate across Rust unit/integration tests, UI tests, and planning validators before status claims.


