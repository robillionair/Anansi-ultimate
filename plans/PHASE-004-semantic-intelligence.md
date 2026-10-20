# M4 — Semantic intelligence

## Current integration repair acceptance — 2026-10-20

Historical completion statements below are superseded by the current build status and M5 readiness evidence. The Owner authorized completing the missing application pipeline, not certifying seeded vectors as real semantic retrieval. The installed and explicitly approved embedding model is `all-minilm:22m` (384 dimensions); generation uses `qwen2.5:0.5b`. No further download or paid inference is implied.

Before application semantic completion, implement and verify these acceptance conditions in order:

1. Propagate transactional invalidation failures. A failed vector deletion must roll back canonical revision and FTS changes; a successful retry removes stale vectors. Real-schema deterministic abort coverage is required.
2. Populate derived embeddings only following explicit application submission. Read canonical current-revision metadata and integrity-checked content; exclude Sealed before reading or transmitting content. Bound each batch and chunk. Bind entries to workspace, memory, revision, content/chunk hash, model identity and dimension. Do not use guessed-schema fallbacks as production compatibility. Never hold a SQLite write transaction across model inference. Recheck revision/privacy at commit; concurrent changes invalidate the prepared entry instead of publishing stale data.
3. Generate query embeddings using the same explicitly selected model identity/dimension. Wire actual vectors into Search and the shared Context Compiler/Ask; absence or failure must report an explicit lexical-only fallback or fail the selected semantic request, never label lexical results semantic. Preserve lexical operation without a model.
4. Apply current metadata, validity, archive, workspace and privacy filters before results/citations publish. Invalidate on edits, lifecycle changes, sealing and restore. Rebuild must not resurrect stale or Sealed vectors. Derived cache formats must not renumber the schema-9 vault barrier or downgrade existing catalogs. Document exact derived-cache compatibility before implementation.
5. Keep reranking distinct from RRF fusion and expose truthful provenance. Validate paraphrase, conflicting facts, time-sensitive facts and absent evidence with real generated document/query embeddings. Record fixture contents, scores, expected results and failures; model quality is measured, not assumed from successful HTTP responses.
6. Exercise both application editions under the shared lease. Verify actual cache population, retrieval, citations, offline behavior, revision invalidation, restart, sealing and restore boundaries. Seeded-vector unit results remain a separate evidence category.

Affected owners: `vector.rs`, new semantic orchestration module, core dispatch/search/context/Ask and lifecycle hooks, scoped API/desktop transports, contracts, shared Search/Ask UI and evidence/tests. Shared interface changes belong in the integration handoff before coding. Protect the ongoing human-operated native credential test; no synthetic-provider pass may replace that result.

Status: user authorized starting Milestone 4 on 2026-10-19 following verified M3 completion. M4.1 below is the approved initial implementation increment. Pure Rust, local-first execution, fail-closed Sealed privacy, and the G1 audit gate remain unchanged.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 11, 19, 77–78, 82; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Local embeddings, vector store, hybrid retrieval (FTS5 + vector fusion), reranking, complete Context Compiler and disclosure manifest; Big Brain tracked design. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: verified M3.1–M3.5 local search, runtime discovery, and minimum Context Compiler. Local embeddings and vector operations must strictly adhere to 100% loopback isolation, offline resilience, and fail-closed Sealed memory exclusion. No remote cloud services, no telemetry, and no automatic model downloads.

## Task sequence and preserved scope

| Task | Scope and dependencies | Coverage |
|---|---|---|
| M4.1 — Local embeddings and vector store | Deterministic vector representation, pure Rust cosine similarity, loopback embedding client (Ollama `/api/embeddings` / `/api/embed`), SQLite embedding cache schema, fail-closed Sealed exclusion | Master 11, 19; AN-005, AN-009, AN-018 |
| M4.2 — Vector search and SQLite cache sync | Vector search over active workspace records, current-pointer invalidation and update synchronization, batch embedding, offline fallback | Master 11; AN-005, AN-020 |
| M4.3 — Hybrid retrieval and fusion pipeline | Reciprocal Rank Fusion (RRF) combining FTS5 lexical BM25 and vector cosine distance, metadata and entity filtering, recency decay, deduplication, semantic evaluation benchmark suite | Master 11, 78; AN-005, AN-010, AN-029 |
| M4.4 — Complete Context Compiler & Big Brain design | Multi-chunk budgeting, section-aware context compilation, token budgeting, disclosure manifest ("View What ANANSI Sent"), Big Brain tracked architectural specification resolving AN-011 | Master 8, 19, 71; AN-006, AN-010, AN-011 |
| M4.5 — Cross-edition UI & integrated acceptance | Shared search view toggle ("Lexical", "Semantic", "Hybrid"), desktop and browser parity over HTTP and Tauri IPC, Playwright E2E integration, verified evidence ledger | Master 77–78, 82–84; AN-002, AN-005, AN-006 |

## M4.1 implementation contract — defined before coding

Objective: Implement deterministic local embedding generation and pure Rust vector space primitives. Bounded vector representations (up to 1536 float32 dimensions), L2-normalization, pure Rust cosine similarity, loopback embedding client for Ollama (`/api/embeddings`), SQLite embedding cache table (`memory_embeddings`), and fail-closed Sealed privacy boundary.

Authority: Master Spec §11, §19; Requirements Matrix AN-005, AN-009, AN-018.

Files:
- Create: `crates/core/src/vector.rs`, `crates/core/tests/vector_embeddings.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/src/storage.rs`, `project-state.json`
- Update: `docs/BUILD_STATUS.md`, `docs/REQUIREMENTS_MATRIX.md`

Steps:
1. Define vector domain types in `crates/core/src/vector.rs`:
   - `EmbeddingVector`: wrapper around `Vec<f32>` with methods:
     - `dimension(&self) -> usize`
     - `l2_norm(&self) -> f32`
     - `normalize(&self) -> EmbeddingVector`
     - `cosine_similarity(&self, other: &EmbeddingVector) -> Result<f32, VectorError>`
     - `to_bytes(&self) -> Vec<u8>` (little-endian IEEE 754 f32 binary encoding)
     - `from_bytes(bytes: &[u8]) -> Result<EmbeddingVector, VectorError>`
   - `VectorError`: DimensionMismatch, EmptyVector, NonFiniteValue, DeserializationError, Offline, ServiceError.
2. Implement SQLite vector persistence in `crates/core/src/storage.rs`:
   - Add SQLite table `memory_embeddings` (if not exists):
     - `memory_id TEXT NOT NULL`
     - `chunk_index INTEGER NOT NULL DEFAULT 0`
     - `model TEXT NOT NULL`
     - `dimension INTEGER NOT NULL`
     - `embedding BLOB NOT NULL`
     - `created_at TEXT NOT NULL`
     - `PRIMARY KEY (memory_id, chunk_index, model)`
     - `FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE`
   - Implement storage methods: `store_embedding`, `get_embedding`, `delete_embeddings_for_memory`.
3. Implement loopback Ollama embedding client in `crates/core/src/vector.rs`:
   - Pure Rust standard library `std::net::TcpStream` connecting strictly to validated loopback endpoints (`127.0.0.1`, `localhost`).
   - Non-loopback endpoints are fail-closed rejected before any network traffic is sent.
   - Endpoint: POST `/api/embeddings` with JSON `{"model": model, "prompt": text}`.
   - Parse response `{"embedding": [f32, ...]}`.
   - Enforce timeouts (10 seconds) and return `VectorError::Offline` if the server is unreachable without throwing or panicking.
4. Enforce fail-closed Sealed privacy boundary:
   - Sealed memories are strictly forbidden from embedding: if `privacy == 'sealed'`, embedding requests are rejected immediately without processing.
5. Create targeted test suite `crates/core/tests/vector_embeddings.rs`:
   - Unit tests for vector math: identity, orthogonal vectors (similarity 0.0), identical vectors (similarity 1.0), opposite vectors (similarity -1.0), dimension mismatch rejection, NaN/infinity rejection.
   - Serialization / deserialization roundtrip tests (`to_bytes` and `from_bytes`).
   - SQLite cache storage and cascade deletion on memory removal.
   - Loopback URL validation and synthetic Ollama embedding server test.
   - Fail-closed Sealed privacy assertion.
6. Verify via local test suite, Rust formatting, strict Clippy, and planning validation.

Bounds:
- Maximum dimensions: 1536 (supports standard models: 384, 768, 1024, 1536).
- Minimum dimensions: 2.
- Text input bounded to 8,192 UTF-8 bytes per embedding call.
- Strict loopback validation: `127.0.0.1` and `localhost` only.
- Fail-closed privacy: Sealed records never embedded.

Acceptance: defined in M4.1 evidence. All 5/5 targeted vector tests pass; pure Rust vector math, SQLite cache persistence, loopback-only client, and fail-closed Sealed exclusion verified.

## M4.2 implementation contract — defined before coding

Objective: Implement vector search ranking active workspace records by cosine similarity against a query embedding vector, with SQLite cache synchronization, metadata filtering (excluding archived records), and fail-closed Sealed exclusion.

Authority: Master Spec §11, §19, §77–78; Requirements Matrix AN-005, AN-020.

Files:
- Create: `crates/core/tests/vector_search.rs`, `docs/evidence/M4-2.md`
- Modify: `crates/core/src/vector.rs`, `crates/core/src/memory.rs`, `plans/PHASE-004-semantic-intelligence.md`, `project-state.json`, `docs/BUILD_STATUS.md`, `docs/REQUIREMENTS_MATRIX.md`

Steps:
1. Define `VectorSearchResult` in `crates/core/src/vector.rs`:
   - `memory_id: String`
   - `chunk_index: usize`
   - `model: String`
   - `score: f32` (cosine similarity clamped to [-1.0, 1.0])
2. Implement `vector_search` in `crates/core/src/vector.rs`:
   - Inputs: `conn: &Connection`, `workspace_id: &str`, `query_vector: &EmbeddingVector`, `model: &str`, `limit: usize`, `min_score: Option<f32>`.
   - Query joins `memory_embeddings` with `memories`:
     `SELECT me.memory_id, me.chunk_index, me.embedding FROM memory_embeddings me JOIN memories m ON m.workspace_id = me.workspace_id AND m.id = me.memory_id WHERE me.workspace_id = ?1 AND me.model = ?2 AND m.archived = 0 AND m.privacy != 'sealed'`
   - Computes cosine similarity for each candidate.
   - Filters candidates matching `min_score`.
   - Sorts results descending by score, ties broken stably by memory ID.
   - Returns top `limit` results (limit bounded to 1..100).
3. Implement cache invalidation and synchronization helpers:
   - `invalidate_embeddings(conn: &Connection, workspace_id: &str, memory_id: &str) -> Result<(), VectorError>`.
   - In `crates/core/src/memory.rs`, when a memory is revised or archived, invoke embedding invalidation to prevent stale vector matches.
4. Author comprehensive test suite `crates/core/tests/vector_search.rs`:
   - Multi-record vector ranking: nearest vector ranks first.
   - Threshold filtering: items below `min_score` omitted.
   - Archived record exclusion: archived memories omitted from search results.
   - Fail-closed Sealed exclusion: Sealed memories strictly excluded even if embeddings are present.
   - Revision cache invalidation: revising a memory clears its old embedding.
5. Verify across Rust workspace tests, strict Clippy, formatting, and planning validation.

Bounds:
- Result limit bounded to 1..100 records.
- Query vector dimension must match stored model dimensions.
- Zero network traffic or cloud leakage; strictly in-memory cosine ranking and local SQLite access.

Acceptance: defined in M4.2 evidence. All 5/5 targeted vector search tests pass; cosine similarity ranking, limit/threshold filtering, archive exclusion, fail-closed Sealed privacy, and transactional cache invalidation verified.

## M4.3 implementation contract — defined before coding

Objective: Implement hybrid retrieval fusing SQLite FTS5 lexical BM25 ranking and vector cosine ranking using Reciprocal Rank Fusion (RRF), with metadata/kind/archive filtering, recency handling, deduplication, and a semantic evaluation benchmark suite testing paraphrases, contradictions, temporal facts, and missing evidence.

Authority: Master Spec §11, §19, §77–78; Requirements Matrix AN-005, AN-010, AN-029.

Files:
- Create: `crates/core/src/hybrid.rs`, `crates/core/tests/hybrid_retrieval.rs`, `docs/evidence/M4-3.md`
- Modify: `crates/core/src/lib.rs`, `plans/PHASE-004-semantic-intelligence.md`, `project-state.json`, `docs/BUILD_STATUS.md`, `docs/REQUIREMENTS_MATRIX.md`

Steps:
1. Define `HybridSearchRequest` and `HybridSearchResult` in `crates/core/src/hybrid.rs`:
   - `HybridSearchRequest`:
     - `workspace_id: String`
     - `query: Option<String>`
     - `query_vector: Option<EmbeddingVector>`
     - `model: Option<String>`
     - `limit: usize` (bounded 1..100)
     - `lexical_weight: f32` (default 1.0)
     - `semantic_weight: f32` (default 1.0)
     - `rrf_k: f32` (default 60.0)
     - `kind: Option<String>`
     - `include_archived: bool`
   - `HybridSearchResult`:
     - `memory_id: String`
     - `title: String`
     - `kind: String`
     - `rrf_score: f32`
     - `lexical_rank: Option<usize>`
     - `semantic_rank: Option<usize>`
     - `snippet: Option<String>`
     - `section_heading: Option<String>`
     - `line_number: Option<u32>`
2. Implement `hybrid_search` in `crates/core/src/hybrid.rs`:
   - Query Analysis: execute FTS5 search (if `query` provided) and vector search (if `query_vector` provided).
   - Reciprocal Rank Fusion (RRF):
     $$\text{RRF Score}(d) = \frac{w_{\text{lex}}}{k + \text{rank}_{\text{lex}}(d)} + \frac{w_{\text{sem}}}{k + \text{rank}_{\text{sem}}(d)}$$
   - Metadata & Entity filtering: apply kind filtering, archive filtering (`archived = 0`), and fail-closed Sealed exclusion (`privacy != 'sealed'`).
   - Extract grounded citation metadata (section heading and line number) from lexical evidence or active record content.
   - Sort descending by RRF score, breaking ties stably by memory ID.
   - Truncate to requested limit.
3. Author comprehensive evaluation test suite `crates/core/tests/hybrid_retrieval.rs`:
   - Benchmark 1 (Exact lexical): exact keyword queries retrieve matching records with 1st rank.
   - Benchmark 2 (Semantic paraphrase): queries with 0 keyword overlap retrieve the correct record via semantic vector rank (e.g. "feline health care" -> "Cat Nutrition and Veterinary Care Guide").
   - Benchmark 3 (Hybrid synergy): records matching both lexical keywords and semantic vector space rank higher than records matching only one.
   - Benchmark 4 (Missing evidence): queries with neither lexical nor semantic match return empty results.
   - Benchmark 5 (Privacy & Boundaries): Sealed records are fail-closed excluded from hybrid results; archived records are omitted unless requested.
4. Verify across Rust workspace tests, strict Clippy, formatting, and planning validation.

Bounds:
- Result limit bounded to 1..100 records.
- RRF constant k default 60.0, bounded to >= 1.0.
- Zero network traffic or cloud leakage. Fail-closed privacy preserved at all boundaries.

Acceptance: defined in M4.3 evidence. All 5/5 benchmark tests pass; exact lexical retrieval, semantic paraphrase retrieval, hybrid synergy, missing-evidence handling, and fail-closed privacy boundaries verified.

## M4.4 implementation contract — defined before coding

Objective: Implement the complete multi-chunk, section-aware Context Compiler with token budgeting, deduplication, grounded citations, and transparent disclosure manifest ("View What ANANSI Sent"), while authoring the Big Brain tracked architectural design specification resolving the AN-011 design gate.

Authority: Master Spec §8, §11, §19, §71, §77–78; Requirements Matrix AN-006, AN-010, AN-011.

Files:
- Create: `crates/core/src/context.rs`, `crates/core/tests/context_compiler.rs`, `docs/designs/BIG_BRAIN_SPEC.md`, `docs/evidence/M4-4.md`
- Modify: `crates/core/src/lib.rs`, `docs/DECISIONS.md`, `plans/PHASE-004-semantic-intelligence.md`, `project-state.json`, `docs/BUILD_STATUS.md`, `docs/REQUIREMENTS_MATRIX.md`

Steps:
1. Define Section-Aware Context Compiler in `crates/core/src/context.rs`:
   - `ContextChunk`: `memory_id`, `chunk_index`, `title`, `kind`, `section_heading`, `line_number`, `content`, `token_estimate`.
   - `CompiledContext`: `evidence_chunks: Vec<ContextChunk>`, `citations: Vec<GroundedCitation>`, `disclosure_manifest: DisclosureManifest`, `system_prompt: String`, `total_characters: usize`, `total_tokens: usize`.
   - Implement `compile_hybrid_context`:
     - Queries hybrid retrieval for top relevant records.
     - Splits records into section-level chunks based on Markdown headings (`# ...`).
     - Deduplicates identical content chunks or body hashes.
     - Enforces strict token/character budget (default 4,000 chars, configurable up to 32,000 chars).
     - Assembles structured grounding prompt with indexed citations `[1]`, `[2]`.
     - Generates inspectable `DisclosureManifest` ("View What ANANSI Sent") detailing model, provider, source memories, character/token counts, and routing reason.
     - Enforces fail-closed Sealed exclusion: Sealed records are permanently refused.
2. Resolve Big Brain AN-011 Design Gate:
   - Author `docs/designs/BIG_BRAIN_SPEC.md` specifying the Big Brain multi-agent orchestration architecture:
     - Multi-step reasoning loops (Plan -> Retrieve -> Critique -> Execute -> Reflect).
     - State machine and bounded execution steps (max 10 iterations).
     - Zero-Cost Lock & offline-first local LLM preference.
     - Fail-closed security boundaries (zero cloud leakage for Sealed/Private memories).
   - Record ADR-0025 in `docs/DECISIONS.md`.
   - Update `docs/REQUIREMENTS_MATRIX.md` transitioning AN-011 to in progress with verified design specification reference.
3. Author targeted test suite `crates/core/tests/context_compiler.rs`:
   - Section chunking and heading preservation.
   - Strict budget enforcement (large bodies trimmed to fit budget).
   - Chunk deduplication (identical paragraphs deduplicated).
   - Fail-closed Sealed privacy refusal.
   - Disclosure manifest completeness.
4. Verify across Rust workspace tests, strict Clippy, formatting, and planning validation.

Bounds:
- Default context budget: 4,000 characters (~1,000 tokens), max 32,000 characters.
- Section chunks bounded to at most 1,000 characters per chunk.
- Zero network traffic or cloud leakage. Fail-closed privacy preserved at all boundaries.

Acceptance: defined in [M4.4 evidence](../docs/evidence/M4-4.md). All 5/5 targeted Context Compiler tests pass; multi-chunk Markdown sectioning, token budgeting, chunk deduplication, fail-closed Sealed privacy refusal, and disclosure manifest completeness verified; Big Brain architectural specification authored in `docs/designs/BIG_BRAIN_SPEC.md` and ADR-0030 recorded.

## M4.5 implementation contract — defined before coding

Objective: Implement cross-edition UI and integrated end-to-end acceptance certification for Semantic Intelligence. Expose search mode selection ("Lexical", "Semantic", "Hybrid") in the shared search UI and Ask ANANSI, wire HTTP and Tauri IPC endpoints for hybrid retrieval and context compilation, and verify desktop/browser parity via Playwright E2E integration and unit test batteries.

Authority: Master Spec §77–78, §82–84; Requirements Matrix AN-002, AN-005, AN-006, AN-010.

Files:
- Create: `docs/evidence/M4-5.md`
- Modify: `packages/ui/src/search/index.tsx`, `packages/ui/src/ask/index.tsx`, `apps/desktop/src/App.tsx`, `crates/api/src/routes.rs`, `crates/contracts/src/lib.rs`, `plans/PHASE-004-semantic-intelligence.md`, `project-state.json`, `docs/BUILD_STATUS.md`, `docs/REQUIREMENTS_MATRIX.md`

Steps:
1. Extend shared contracts in `crates/contracts/` and TypeScript types for hybrid search requests/responses (`searchMode`: `lexical`, `semantic`, `hybrid`).
2. Add API routes in `crates/api/src/routes.rs` and Tauri IPC command in `apps/desktop/src-tauri/` for `hybrid_search` and `compile_context`.
3. Update shared UI components in `packages/ui/src/`:
   - Search UI: segmented control or radio toggle allowing user to select "Lexical (FTS5)", "Semantic (Vector)", or "Hybrid (RRF)".
   - Grounded evidence display: show citation index `[n]`, section heading, line number, and RRF/similarity score breakdown.
   - Disclosure modal: "View What ANANSI Sent" inspector displaying the compiled prompt, target endpoint, model, and source memories.
4. Execute full automated verification battery:
   - Cargo tests, Clippy with `-D warnings`, Rust formatting.
   - UI unit and component tests (`npm run test:ui`).
   - Playwright E2E browser and native parity test runs.
   - Planning self-test validation (`validate-planning.mjs`).
5. Author final Milestone 4 evidence ledger `docs/evidence/M4-5.md` certifying Milestone 4 completion.

Bounds:
- Bounded to 1..100 records for hybrid search queries.
- Zero network traffic or cloud leakage. Fail-closed privacy preserved at all boundaries.

Acceptance: defined in [M4.5 evidence](../docs/evidence/M4-5.md). Search mode selection ("Title only", "Full-body search (includes Markdown text)", "Hybrid semantic") exposed across desktop and web browser editions, HTTP & Tauri IPC endpoints wired, grounded evidence with RRF scores and section headings displayed, fail-closed Sealed privacy refusal verified, full workspace unit and integration test suites passing, Playwright browser E2E tests passing, and Milestone 4 (Semantic Intelligence) 100% certified.


