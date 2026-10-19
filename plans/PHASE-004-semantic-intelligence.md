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
