# M2.7 integration increment — vault/search boundary

Status: local acceptance passed on 2026-10-19 in the isolated M2 worktree. This increment does not activate Sealed commands or certify M2.8.

Implemented `anansi_core::vault::policy::SearchBoundary` and its focused integration tests. The boundary excludes Sealed records before indexing, requires a current vault session for Sealed publication, and provides transactional record/workspace FTS invalidation for sealing or revocation. Publication rechecks caller identity, token, idle/absolute deadlines and durable generation epoch. It returns no key or decrypted payload.

Verification:

- `node scripts/cargo.mjs fmt --check` — passed.
- `node scripts/cargo.mjs test -p anansi-core --test vault_search_boundary --locked` — 3 passed.
- `node scripts/cargo.mjs clippy -p anansi-core --all-targets --locked -- -D warnings` — passed.
- `node scripts/validate-planning.mjs --self-test` — passed (7 negative fixtures, 56 Markdown files, 90 section owners, 16 plans, 33 requirements).
- `git diff --check` — passed.

The M3 persistent FTS implementation was inspected read-only. Its consumer obligations, including stale-row removal during sealing/revocation/rebuild/backup transitions, are recorded in [the M2→M3 handoff](../handoffs/M2-to-M3.md). No M3 files, adapters, workflows or protected master branch were changed.
