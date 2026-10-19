# M4 — Semantic intelligence

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
