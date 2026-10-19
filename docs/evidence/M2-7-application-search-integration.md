# M2.7 application search integration evidence

The committed M3 repair checkpoints `cb4774ec6dcb92406ec4c68d0866ce0ddd9b3b56` and `f7ba5b3a407c89aacb7b63f80dc18e0b6f3bef24` were verified in the M3 worktree and selectively integrated into M2. Search writes now call the vault-owned FTS boundary; memory writes remove/reinsert rows transactionally, and rebuilds invalidate the workspace before repopulation.

Executed locally in the M2 worktree:

- `node scripts/cargo.mjs test -p anansi-core --test body_search --locked -- --nocapture` — 1 passed (44.57s).
- `node scripts/cargo.mjs test -p anansi-core --test body_search_limits --test vault_search_integration --locked` — passed (body corpus and schema-8 sealing transition; the command output was split across the two test binaries).
- `node scripts/cargo.mjs clippy -p anansi-core --all-targets --locked -- -D warnings` — passed.
- `node scripts/validate-planning.mjs --self-test` — passed.
- `git diff --check` — passed.

This is a real schema-8 search/index integration, including stale-row cleanup and restart/rebuild coverage. It does not activate Sealed creation: the application lacks an adapter-facing vault setup/session command and trusted native credential/prompt path capable of supplying `AcknowledgedSetup`. Therefore live DEK revocation, lock/expiry/epoch publication races, native/browser Sealed workflows, encrypted backup/restore integration, whole-root encryption, secure erasure, and M2.8 remain pending. Retained plaintext originals/backups remain governed by the approved G2–G4 corrections.
