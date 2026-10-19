# M3 → M2 vault/search integration handoff

Status: prepared against M3 checkpoint `1c24df1` with local working changes in `crates/core/src/search.rs` preserved. The M2 branch was not merged and its worktree was not edited.

## Consumer mapping

M3 currently owns `crates/core/src/search.rs` and the body-search tests. The M2 boundary is expected at `anansi_core::vault::policy::SearchBoundary`:

- `may_index(privacy)` must run before every persistent FTS insert, update, count, and rebuild.
- `publish(...)` must run immediately before returning any result that includes Sealed metadata or decrypted content.
- Search must treat `Locked`, expired session, caller mismatch, durable epoch change, migration, and revocation as fail-closed outcomes. No snippets, counts, titles, bodies, graph metadata or existence hints may escape.

## Existing M3 behavior and regression coverage

The current M3 index is rebuildable FTS5 and excludes Sealed rows by metadata. A new `vault_search_integration` test directly simulates a schema-8 privacy transition, rebuilds the index, restarts Core, rebuilds again, and confirms the plaintext canary is absent both times. This proves stale-row removal for the current non-vault implementation only; it does not prove live vault session/epoch behavior.

## Required integration changes

When the M2 vault API is consumed, update these shared files in one coordinated checkpoint:

1. `crates/core/src/search.rs`: call the boundary before index writes/rebuilds and before publication; delete existing rows when a record seals or its generation is revoked; map vault failures to the existing privacy-safe error contract.
2. `crates/core/src/memory.rs` and lifecycle/privacy dispatch owner: route Sealed transitions through the vault generation writer, and invalidate FTS rows transactionally with the transition.
3. `crates/core/src/storage.rs` or the final vault storage adapter: ensure restart/rebuild never reads a plaintext Sealed source and preserves schema-9 refusal until activation.
4. `crates/core/tests/body_search.rs`, `vault_search_integration.rs`, and the eventual vault integration test: cover sealing, lock, expiry, epoch changes, migration races, historical revisions, backup/export, restore, and stale-row cleanup.
5. `docs/BUILD_STATUS.md`, `docs/REQUIREMENTS_MATRIX.md`, `docs/DECISIONS.md`, and the M2/M3 evidence ledgers: distinguish boundary tests, current schema-8 behavior, and actual integrated vault acceptance.

Do not copy vault internals into Search or expose DEKs/tokens to the frontend. The final integration must be a small domain call and preserve the M2 owner’s file boundary.
