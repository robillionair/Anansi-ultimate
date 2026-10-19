# M3 vault/search boundary evidence

Against M3 checkpoint `1c24df1`, the current persistent FTS5 implementation was reviewed against the M2 handoff from `e67010e`. A focused schema-8 regression test simulates a Private→Sealed metadata transition, rebuilds the index, restarts Core, rebuilds again, and confirms the plaintext canary is absent.

Executed:

- `node scripts/cargo.mjs test -p anansi-core --test vault_search_integration --locked` — 1 passed.
- `node scripts/cargo.mjs clippy -p anansi-core --test vault_search_integration --locked -- -D warnings` — passed.
- `node scripts/validate-planning.mjs --self-test` — passed (7 negative fixtures, 53 Markdown files, 90 section owners, 16 plans, 33 requirements).
- `git diff --check` — passed.

This evidence covers stale plaintext removal during current schema-8 rebuild/restart only. It does not certify live M2 vault sessions, epoch revocation, lock/expiry races, encrypted backup/restore, or M2.8 application acceptance. See [the M3→M2 handoff](../handoffs/M3-to-M2.md).
