# Vault schema-ladder reconciliation — addendum

Date: 2026-10-21 · Branch `muse/vault-ladder-reconciliation`
from the R16 tip `a1b0c80`, on explicit owner authorization
to cross the historical M2/M6 ownership boundary for this
task. All results below are **Linux (this VM)**; Windows
verification remains the owner's / independent reviewer's
step. The two `vault_increment2` failures are closed by this
checkpoint (suite: **9 passed / 0 failed**, including one
new test).

## The failures, reproduced

At `a1b0c80`: `vault_increment2` = 6 passed / 2 failed.
`canonical_history_graph_epoch_rotation_and_explicit_rollback`
panics at the post-seal `user_version` assertion (left 10,
right 9); `deterministic_process_crash_cutover_and_rollback_matrix`
fails its schema assertion (left 10, right 8, first case
`seal/PreJournal`). (The `epoch_worker` panic visible in the
same run is fallout: the canonical test dies before writing
the handshake marker its child waits on.)

## What the evidence showed — the implementation was also wrong

The R15 root-cause note blamed stale expectations alone:
since M6 Slice 1, fixtures are built at schema 10 while the
tests encode the pre-0010 ladder (8 pre-vault, 9 sealed).
Probing the actual durable behaviour (temporary probe,
removed after use) proved a second, production-side defect
that made the honest half of those expectations unreachable:

1. **Sealing a genuine schema-8 catalog landed at
   `user_version` 10**, not the documented schema-9 barrier.
   The seal's explicit upgrade (`vault::catalog::upgrade`,
   8 → apply `0009_vault.sql` → 9) never observed an 8: the
   backup verification that precedes it opened the live
   catalog through the migrating `Storage::connection()` in
   two incidental places — `verify_snapshot`'s `read_job`
   lookup and `stage()`'s root-validation open — silently
   applying 0010 and bumping the counter first.
2. **A pre-switch abort on a genuine schema-8 catalog left
   it at 10 with `artifact_versions` created.** The vault
   transaction itself rolled back correctly (plaintext
   intact, no vault state), but the durable schema had
   already been rewritten by the verification side effect —
   against the vault plan's acceptance ("failures before
   commit leave old source authoritative") and ADR-0026
   ("schema upgrade is explicit and transactional").
3. Everything else probed matches the documented design:
   the vault catalog accepts {8, 9, 10}; sealing from 10
   applies the vault tables at 10 and the counter never
   moves backwards; ordinary Core coexists with a sealed
   catalog at the current schema (its list succeeds; the
   sealed workspace's records are simply absent from the
   plaintext tables).

## The production fix (commit `76f5322`)

Verification is now side-effect-free with respect to the
live catalog's schema:

- `verify_snapshot` reads its job through a new
  `Storage::read_job_readonly` — a `SQLITE_OPEN_READ_ONLY`
  open feeding the same row parser. `read_job`'s own Core
  contract (migrating open) is unchanged.
- `stage()` validates the trusted root via
  `Storage::root_ready` instead of a migrating connection.
  Its other caller, `execute_backup`, opens the migrating
  connection itself immediately afterwards, so backup
  creation behaviour is unchanged.

Post-fix probes: sealing a genuine schema-8 fixture lands
at **9** with all records readable after unlock; an abort
at `PreSwitch` leaves **schema 8, no registry table,
plaintext intact**. No schema downgrade path was added
(the fixture rewind lives in tests only), no authentication
was relaxed, and no crash assertion was weakened.

## The test reconciliation (commit `82f4eb3`)

- `deterministic_process_crash_cutover_and_rollback_matrix`
  keeps its original assertions verbatim and runs on the
  fixture it was written for: a genuine legacy schema-8
  catalog (a Core-built fixture rewound by dropping the
  registry table — the only ladder object over migrations
  0001–0008 — so the schema is exactly the legacy object
  set plus the lazily created derived tables a used legacy
  database can also carry).
- `deterministic_process_crash_matrix_on_schema_10` (new)
  runs the same ten crash boundaries from a schema-10 start
  (landing at 10), sharing the matrix body. Both runs now
  also assert the vault's tables appear exactly when a
  generation change commits, and the schema-10 run asserts
  a registered artifact's rows and content hashes survive
  every boundary byte-identically.
- `canonical_history_graph_epoch_rotation_and_explicit_rollback`
  stays on the current fixture. Its two genuinely stale
  expectations are reconciled with reasoning recorded in
  the test: post-seal version 9 → 10 (the schema-9 landing
  is covered by the legacy matrix), and the "ordinary Core
  list must fail" assertion — true only when a sealed
  catalog (9) was newer than Core's schema (8) — becomes
  the substantive protection: the root still lists, and
  the sealed workspace exposes zero plaintext memories
  through Core. All history/graph/epoch/rotation/rollback
  and stale-writer trigger assertions are unchanged.

## Regression evidence (Linux)

- `vault_increment2` 9/9 (~280 s; the matrix now runs on
  both ladders). Other vault suites: `vault_certification`
  11/11, `vault_runtime` 2/2, `vault_credential_store` 1/1,
  `vault_search_boundary` 3/3, `vault_search_integration`
  1/1, `vault_search_invalidation` 2/2.
- Verification-callers regression: core lib 34/34,
  `backup_artifacts` 5/5, `backup_restore` 7/7.
- Clippy `-D warnings` clean (lib + `vault_increment2`
  target); fmt clean.

## Still open

- Windows verification of this branch (the owner has
  verified R16 on Windows; this reconciliation has not been
  Windows-verified).
- G1 external audit and the M5 pending checks are untouched
  by this work and remain open. Nothing here certifies the
  vault for production.
