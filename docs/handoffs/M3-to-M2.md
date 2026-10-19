# M3 → M2 Vault & Search Integration Handover

**Handover Status**: Completed and committed.
**Repair Checkpoint SHA**: `cb4774ec6dcb92406ec4c68d0866ce0ddd9b3b56`
**Ownership Transfer**: Complete ownership of `crates/core/src/search.rs` and shared search integration glue is hereby transferred to ChatGPT (M2/Vault Owner).

---

## 1. Summary of Completed Search Repairs

In commit `cb4774ec6dcb92406ec4c68d0866ce0ddd9b3b56`, the following gaps identified in M3.2 were repaired and verified:

1. **Rebuild Error Propagation**:
   - Eliminated silent error suppression (`let _ = self.rebuild_search_index()`). Automatic and on-demand rebuild failures are now propagated cleanly as `Result<(), CoreError>`.
   - Prevented Windows SQLite table-lock / busy timeout contention by closing active read connections (`drop(conn)`) before initiating the immediate transaction in `rebuild_search_index`.

2. **Global Candidate Ranking & Precise Pagination**:
   - FTS candidate matches are scored globally across all matching records using BM25 blended with temporal decay heuristics.
   - Paging offsets and limits (`offset`, `limit`) are strictly applied to the globally ranked candidate list.
   - Markdown disk access (`content::read`) and snippet/heading location extraction are deferred and performed *only* on the returned page slice, dramatically reducing disk I/O and ACL evaluation overhead.

3. **Fail-Closed Body Integrity Enforcement**:
   - `content::read` failures (checksum mismatch, missing Markdown file, tampered disk content) fail closed with `ErrorCode::ContentIntegrity` rather than silently degrading or returning unverified content.

4. **Corpus & Scalability Bounds**:
   - Enforced `MAX_RECORDS` (10,000 records) upper bound in both `rebuild_search_index` and `body_search`. Queries exceeding the hard corpus limit return `ErrorCode::InvalidSearch` without leaking partial results.

5. **Direct ID Filtering**:
   - Added `r.id` filter support in `body_search` SQL query to enable targeted single-record retrieval and verification.

6. **Epistemic Truth Claim Correction**:
   - In `crates/core/tests/retrieval_evaluation.rs`, corrected assertion documentation: recency weighting is explicitly documented as a chronological decay candidate-ranking heuristic, disclaiming any proof of epistemic truth.

---

## 2. Files Changed in Repair Checkpoint

- `crates/core/src/search.rs`
- `crates/core/tests/body_search.rs`
- `crates/core/tests/retrieval_evaluation.rs`
- `crates/core/tests/body_search_limits.rs`
- `crates/core/tests/vault_search_integration.rs`

---

## 3. Local Verification Commands and Results Observed

All checks executed locally on Windows worktree `c:\Users\robin\Documents\ChatGPT\Anansi ultimate`:

1. **Targeted Search Integration Tests**:
   - `node scripts/cargo.mjs test -p anansi-core --test body_search` -> **PASSED** (all 5 steps, 48.50s)
   - `node scripts/cargo.mjs test -p anansi-core --test body_search_limits` -> **PASSED** (3 tests: tampered Markdown fail-closed, 1,000-record pagination, >10,000 bounds check, 27.62s)
   - `node scripts/cargo.mjs test -p anansi-core --test retrieval_evaluation` -> **PASSED** (all 7 retrieval quality benchmarks, 25.68s)
   - `node scripts/cargo.mjs test -p anansi-core --test vault_search_integration` -> **PASSED** (schema-8 sealing transition rebuild, 8.83s)

2. **Full Workspace Test Suite**:
   - `node scripts/cargo.mjs test --workspace --locked` -> **PASSED** (all unit and integration tests across `anansi-core`, `anansi-desktop`, and helper crates)

3. **Static Analysis & Linting**:
   - `cargo fmt --all --check` -> **PASSED** (clean formatting)
   - `cargo clippy --workspace --all-targets --locked -- -D warnings` -> **PASSED** (0 errors, 0 warnings)

4. **UI Contracts & Components**:
   - `npm run test:ui` -> **PASSED** (23/23 tests passing)

---

## 4. Remaining Defects, Known Limitations, and Untested Behavior

The following areas are intentionally not verified as fully integrated vault workflows and must be addressed by ChatGPT during M2 integration:

1. **Live Vault Session / Epoch Sealing**:
   - `vault_search_integration.rs` and `body_search.rs` currently simulate privacy sealing at the database schema level (`privacy = 'sealed'` under schema 8) and verify FTS exclusion and rebuild cleanup.
   - Live DEK revocation, vault locking races during active searches, memory epoch transitions, and cryptographic unsealing are mock-tested and await final M2 vault engine wiring.

2. **10,000-Record On-Disk ACL Benchmark**:
   - The >10,000 record bound test (`test_body_search_corpus_overflow_limit`) verified the fail-closed error code via mock transactions. It did not create 10,000 individual on-disk Markdown files due to Windows NTFS ACL process spawning overhead.

3. **Native Desktop UI End-to-End Body Search**:
   - UI contract and unit tests pass (23/23). Native desktop Tauri window interactions rely on Playwright/smoke testing rather than local interactive desktop driving.

---

## 5. Explicit Transfer of Ownership

- **Transferred Files**:
  - `crates/core/src/search.rs`
  - Memory lifecycle dispatch integration (`crates/core/src/memory.rs` sealing hooks)
  - Storage & vault search boundary glue (`crates/core/src/storage.rs`)
- **Antigravity Next Scope**:
  - Antigravity will **not** modify `crates/core/src/search.rs`, memory lifecycle dispatch, or storage glue.
  - Antigravity will proceed with M3.3 runtime discovery in dedicated, non-overlapping modules (`crates/core/src/ai_discovery.rs`, `crates/core/tests/ai_discovery.rs`).
