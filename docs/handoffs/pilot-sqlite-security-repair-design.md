# Pilot SQLite security dependency repair — design + acceptance criteria

Branch: `muse/pilot-sqlite-security-repair`, from the M13 DOCX tip
`6ff0d22e7e2d83428de9f284f689de47c6251187`. Linux implementation +
verification only. Antigravity owns Windows integration; nothing
here is merged, pushed, or promoted. No cloud calls, no paid
inference, no model downloads. New document formats are paused.

## 1. Advisory verification (measured, not assumed)

RUSTSEC-2026-0459, CVE-2026-119375, GHSA-72w2-rjgg-q729,
CVSS 4.0 `AV:L/AC:L/AT:N/PR:N/UI:P/VC:H/VI:N/VA:N` (6.8 medium,
2026-10-30), category `memory-exposure`, keywords
`sqlite, wal, uninitialized-memory, information-disclosure`.

SQLite before 3.54.0 can return uninitialized page-cache memory
when a process opens a database file together with a sibling
write-ahead-log (`-wal`) file whose page size differs from the
database's. A crafted pair of files can make ordinary queries
return stale bytes from the process's memory. Local vector only;
the victim must open the files (UI:P).

- Workspace lockfile (verified 2026-11-09): `libsqlite3-sys
  0.38.2`, via `rusqlite 0.40.2` (`crates/core/Cargo.toml:16`,
  exact pin, features `["bundled", "backup"]`).
- Bundled SQLite confirmed in the vendored crate source
  (`libsqlite3-sys-0.38.2/sqlite3/sqlite3.h`):
  `SQLITE_VERSION "3.53.2"`, `SQLITE_VERSION_NUMBER 3053002` —
  inside the affected range (0.6.0–0.38.2 bundle 3.13.0–3.53.2).
- Patched: `libsqlite3-sys >= 0.39.0` (0.39.0 bundles SQLite
  3.54.0; 0.39.1 bundles 3.54.1), released with `rusqlite`
  0.41.0. `libsqlite3-sys` 0.39 declares Rust 1.88, `rusqlite`
  0.41 declares Rust 1.89; this VM's toolchain is 1.99.1.
- `bundled-sqlcipher` in 0.39.0/0.39.1 still bundles SQLite
  3.53.4 (affected) — not used here (features are `bundled` +
  `backup` only), so it does not change the fix.
- A build linking a system SQLite instead of the bundled source
  is not affected by the copy in this crate; ANANSI builds the
  bundled source, so it is affected.

## 2. Exposure map — ANANSI's database-open and backup/restore paths

All paths open the catalog `workspaces.sqlite3`
(`storage::DATABASE_NAME`), which the app runs in WAL mode
(`storage.rs` sets `journal_mode=WAL` on open, tolerating only
`DatabaseBusy`).

| Path | File | What it opens | Exposure |
|---|---|---|---|
| Main catalog open | `storage.rs:354` `Storage::connection` | `workspaces.sqlite3` + `-wal`/`-shm`/`-journal` (symlink-rejected, permission-checked); a **read-only probe** (`READ_ONLY \| NO_MUTEX`) runs before the read-write open, then migrations 0002–0011 to schema 11, then WAL | Reads a sibling `-wal` during the probe if one is present in the storage root |
| Vault catalog | `vault/catalog.rs:130` `Catalog::connection` | Same four files (symlink-rejected); read-only probe for `user_version`/`application_id` (expects `0x414e414e`, schema 8–11), then read-write open + `PRAGMA quick_check(1)` + `foreign_key_check` | Same probe exposure; `trusted_schema=OFF` is set but does not cover the WAL page-size defect |
| Read-only job lookup | `jobs.rs:308` `read_job_readonly` | Catalog opened `READ_ONLY`, no migration | Reads a sibling `-wal` if present |
| Restore | `restore.rs:2060` | `catalog.sqlite3` from a staged backup package, `READ_ONLY` + `PRAGMA trusted_schema=OFF`, then identity queries | **Not exposed via packages:** the tar unpacker (`snapshots.rs` `unpack_with_tick`) allowlists exactly `manifest.json`, `snapshot.json`, `catalog.sqlite3`, `bodies/<id>.md`, `artifacts/<id>.tar` — a `catalog.sqlite3-wal` entry is refused, so a crafted package cannot smuggle a `-wal` sibling |
| Staging migration | `snapshots.rs:640` `migrate_staging_catalog_10_to_current` | Staged catalog from a verified package, read-write + `trusted_schema=OFF` | Same allowlist protection as restore |
| Backup | `snapshots.rs:182` `execute_backup_with_snapshot_hook` | Fresh DELETE-mode target catalog (`PRAGMA journal_mode=DELETE`); source checkpointed (`wal_checkpoint(TRUNCATE)` + `journal_mode=DELETE`, line 357); online backup API (`rusqlite::backup::Backup`, which "includes committed WAL pages from this read snapshot") | Reads ANANSI's own WAL (page size matches the DB by construction); the packaged catalog is DELETE-mode and checkpointed, no `-wal` in the package |

Honest assessment: ANANSI's own files can never trigger the
defect — its own `-wal` is written by its own SQLite with a
matching page size, and its own packages contain no `-wal`.
The remaining vector is a workspace directory prepared
out-of-band (copied or shared storage root) containing a
crafted `workspaces.sqlite3` + mismatched-page-size `-wal`,
opened by the probe in `Storage::connection` / the vault
catalog. That is a local, user-interaction vector, not a
remote one, and there is no evidence of exploitation. The
upgrade is still the correct fix: it removes the vulnerable
code from the build rather than relying on the file-shape
defences, and the advisory is explicit that bundled builds
are susceptible.

## 3. Upgrade choice (smallest compatible)

- `crates/core/Cargo.toml:16`: `rusqlite = { version = "=0.40.2", ... }`
  → `=0.41.1` (latest patch in the 0.41 line; the advisory's
  fix release is 0.41.0). `rusqlite` 0.40.2 requires
  `libsqlite3-sys ^0.38`, so a lockfile-only bump of
  `libsqlite3-sys` cannot resolve — the rusqlite minor bump is
  the smallest resolution that satisfies the advisory.
- Expected resolution: `rusqlite 0.41.1`, `libsqlite3-sys
  0.39.1`, bundled SQLite 3.54.1. Verified after the upgrade
  from the vendored `sqlite3.h` and from the runtime
  `sqlite_version()` (a test asserts the version string).
- Features stay `["bundled", "backup"]`: bundled SQLite
  (no system library), online backup API, no sqlcipher.
- Lockfile diff must be focused: `rusqlite`, `libsqlite3-sys`,
  and only their direct transitive changes. No other crate
  version moves; the `Cargo.toml` change is the single pin
  line. If the resolver pulls in anything else, that is
  reported, not silently accepted.
- rusqlite 0.41 may carry API changes vs 0.40; any compile
  break is fixed narrowly at the call site and reported. No
  DOCX or other source changes unless the upgrade exposes a
  concrete regression.

## 4. Acceptance criteria

- AC-R1: Lockfile resolves `rusqlite 0.41.1` + `libsqlite3-sys
  0.39.1` (or the latest 0.39.x the resolver picks, recorded
  exactly); `cargo audit` no longer reports RUSTSEC-2026-0459;
  no second rusqlite/libsqlite3-sys version in the lock.
- AC-R2: The build actually uses bundled SQLite 3.54.x —
  verified from the vendored `sqlite3.h` AND from a runtime
  version query in a test, not from the lockfile alone.
- AC-R3: Features preserved: `bundled` + `backup`; no system
  SQLite, no sqlcipher; FTS5 search still works (search tests
  pass).
- AC-R4: Migrations preserved: legacy catalogs (schema 8, 9,
  10 — the versions the vault accepts — and 1..11 through the
  migration chain) open and migrate to schema 11; vault
  seal/open, `quick_check`, and `foreign_key_check` behave as
  before.
- AC-R5: Backup + fresh restore round-trip preserves data;
  an existing pre-upgrade workspace directory reopens with
  the new binary (data intact, WAL checkpointed normally).
- AC-R6: DOCX + PDF ingestion and search still pass (they
  exercise the catalog write/read paths); no new failures in
  the focused suites.
- AC-R7: Locked Rust checks clean: `cargo test --locked
  --workspace --lib` (baseline at DOCX tip: 358/358),
  provider-dispatch 22/22, `tsc` 0, contracts 0 drift, docs
  PASS. Any regression is recorded with its cause, not hidden.
- AC-R8: Lockfile diff focused per §3; `Cargo.toml` change is
  the one pin line.
- AC-R9: No DOCX or feature source changes; no other advisory
  is folded into this repair (ctutils RUSTSEC-2026-0385 and
  the other audit warnings stay out of scope).
- AC-R10: Delivery is a small checksummed Git bundle ZIP with
  exact SHAs, a Windows rerun checklist, and this handoff.

## 5. Verification plan

1. `cargo audit` before (confirms RUSTSEC-2026-0459 on
   0.38.2) and after (confirms it is gone).
2. `cargo update -p rusqlite --precise 0.41.1 --locked` (or
   the manifest-pin edit + `cargo update -p rusqlite`),
   then `git diff --stat Cargo.lock` reviewed line by line.
3. Vendored `sqlite3.h` version check + runtime version test.
4. `node scripts/cargo.mjs test --locked --workspace --lib`
   (full lib count vs 358 baseline).
5. Focused: storage/vault/restore/snapshot/search/docx/pdf
   Rust tests; provider-dispatch 22/22.
6. Existing-workspace reopen: build the API binary, open a
   pre-upgrade workspace dir (created with the DOCX-tip
   binary), verify data + a write + a checkpoint.
7. Backup/fresh restore: e2e `backups` flow via the existing
   spec where it does not hit its known pre-existing
   'Workspace' button failure; otherwise the Rust-level
   backup/restore tests + a CLI-level round-trip.
8. E2E (one spec per invocation, with the lease):
   `documents-docx`, `documents-pdf`, `documents-ingestion`,
   `ask_anansi`, and the search path. `tsc`, contracts regen,
   docs validator.

## 6. Out of scope

DOCX/PDF feature work; new document formats (paused); other
advisories; system-SQLite builds; sqlcipher; preview
promotion; pushes; remote CI. Global status documents are
Antigravity's — proposed updates go in the handoff only.
