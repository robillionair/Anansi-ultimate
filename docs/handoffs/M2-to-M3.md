# M2 → M3 vault/search integration handoff

Status: prepared in M2 branch `codex/m2-7-vault-core`; no M3 checkout was edited.

## Boundary

M2 exposes `anansi_core::vault::policy::SearchBoundary` as the only search-facing vault hook. It has three properties:

1. `may_index(privacy)` excludes `Privacy::Sealed` before an FTS/vector index is populated or counted.
2. `publish(...)` permits non-Sealed results without a vault session and requires a current `Sessions` capability for Sealed results.
3. The Sealed publication path rechecks caller identity, token, idle/absolute deadlines, and the durable generation epoch. It returns no DEK, keyslot, or vault payload.
4. `invalidate_record(...)` and `invalidate_workspace(...)` delete plaintext FTS rows and are intended to run in the same transaction as a sealing/revocation/generation transition.

## M3 integration obligations

M3 search must call `may_index` before every persistent-index insert and rebuild, delete stale rows when a record transitions to Sealed or is revoked, and call `publish` immediately before returning any Sealed result. A lock, epoch change, caller mismatch, expiry, or migration race must fail closed without returning snippets, counts, titles, bodies, or graph metadata. Existing non-Sealed behavior remains unchanged.

The persistent FTS5 table must be reviewed during M2 integration for stale rows after sealing, unlock/lock transitions, revocation, rebuild, backup/export and historical revisions. A sealed-create refusal test alone is insufficient.

## Files owned by this handoff

- M2: `crates/core/src/vault/policy.rs`, `crates/core/tests/vault_search_boundary.rs`.
- M3 consumer: its search/indexing implementation and tests, after an explicit merge/integration checkpoint.

This handoff does not activate Sealed commands or claim M2.8 acceptance.

## Runtime integration note (6f9aafb follow-up)

`anansi_core::vault::runtime::VaultRuntime` now exposes `context(workspace)` and `read(caller, token, now)` for adapter integration. Adapters must obtain the context only after a sealed generation exists, keep the returned session token in process memory, and call `read` for every authorized snapshot. These methods preserve the existing second publication authorization check and do not serialize credentials, DEKs or recovery material. Native prompt and desktop/browser command transport remain adapter-owned.

The M2 desktop adapter now owns the process-local `VaultService`, session handles and native `vault_command`. No M3 search interface changed. Search remains responsible for the existing `SearchBoundary` obligations; schema-9 startup must not route through schema-8 memory/search commands until the vault adapter explicitly decrypts and authorizes the snapshot.
