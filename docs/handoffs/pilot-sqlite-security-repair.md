# Pilot SQLite security dependency repair — handoff

Branch: `muse/pilot-sqlite-security-repair`, from the M13 DOCX tip
`6ff0d22e7e2d83428de9f284f689de47c6251187`. Linux implementation +
verification only. Antigravity owns Windows integration; nothing
here is merged, pushed, or promoted. No cloud calls, no paid
inference, no model downloads. New document formats paused.

## Commits

| Commit | Contents |
|---|---|
| `293cb46ff` | Design + acceptance criteria (AC-R1..R10) before the upgrade |
| `50ffb9925` | The repair: rusqlite pin + lockfile, one call-site fix, runtime version test |

## The advisory (verified, not assumed)

RUSTSEC-2026-0459, CVE-2026-119375, GHSA-72w2-rjgg-q729,
CVSS 4.0 `AV:L/AC:L/AT:N/PR:N/UI:P/VC:H/VI:N/VA:N` (6.8 medium,
2026-10-30), category `memory-exposure`. SQLite before 3.54.0
can return uninitialized page-cache memory when a process opens
a database file together with a sibling `-wal` whose page size
differs from the database's; a crafted pair can make ordinary
queries return stale bytes from process memory. Local vector
only; the victim must open the files.

- Before: lockfile `libsqlite3-sys 0.38.2` via `rusqlite
  0.40.2` (`crates/core/Cargo.toml:16`, exact pin, features
  `["bundled", "backup"]`); bundled SQLite confirmed in the
  vendored crate source as **3.53.2** (affected range
  0.6.0–0.38.2 bundles 3.13.0–3.53.2).
- After: `rusqlite 0.41.1`, `libsqlite3-sys 0.39.1`, bundled
  SQLite **3.54.1** — confirmed from the vendored `sqlite3.h`
  (`SQLITE_VERSION "3.54.1"`, `3054001`) AND from a runtime
  test asserting `rusqlite::version()` starts with `3.54.`
  and `version_number() >= 3_054_000`.
- `cargo audit` before the repair reports RUSTSEC-2026-0459;
  after, it reports zero mentions. The other audit findings
  (ctutils RUSTSEC-2026-0385, unmaintained warnings) are
  unchanged and out of scope.
- `libsqlite3-sys` 0.39 declares Rust 1.88, `rusqlite` 0.41
  declares Rust 1.89; the toolchain is 1.99.1.

## Exposure map (design doc §2 has the full table)

The catalog `workspaces.sqlite3` runs in WAL mode on every
open (`storage.rs` sets `journal_mode=WAL`). The open paths
are `Storage::connection` (read-only probe, then migrations
0002–0011 to schema 11), the vault catalog (read-only probe
for `user_version`/`application_id`, then `quick_check` +
`foreign_key_check`), `read_job_readonly`, and restore
(`restore.rs:2060`, read-only + `trusted_schema=OFF`).

Honest assessment: ANANSI's own files can never trigger the
defect — its own `-wal` is written by its own SQLite with a
matching page size, and its own backup packages contain no
`-wal` (the tar unpacker allowlists exactly `manifest.json`,
`snapshot.json`, `catalog.sqlite3`, `bodies/<id>.md`,
`artifacts/<id>.tar`; a `catalog.sqlite3-wal` entry is
refused). The remaining vector is a workspace directory
prepared out-of-band (copied/shared storage root) containing
a crafted DB + mismatched-page-size `-wal`, opened by the
probe. Local, user-interaction, no evidence of exploitation.
The upgrade removes the vulnerable code rather than relying
on the file-shape defences.

## What changed (4 files, 21 insertions, 6 deletions)

- `crates/core/Cargo.toml:16`: `rusqlite = "=0.40.2"` →
  `"=0.41.1"`. Features unchanged (`bundled`, `backup`):
  still the bundled SQLite, still the online backup API, no
  system library, no sqlcipher.
- `Cargo.lock`: exactly 4 lines — the two version + checksum
  pairs. No other crate moved.
- `crates/core/src/restore.rs`: one line — rusqlite 0.41
  changed `Statement::column_names()` from `Vec<&str>` to
  `impl Iterator<Item = &str>`, so the `.iter()` after it no
  longer compiles. The call is dropped; the collected
  `Vec<String>` is unchanged. This was the upgrade's only
  compile break (11 errors, all cascading from this one
  call); the restore e2e and the API-level backup/restore in
  the reopen check exercise the fixed path.
- `crates/core/src/storage.rs`: one test,
  `bundled_sqlite_is_patched`, asserting the linked SQLite
  version at runtime.

No DOCX, PDF, migration, vault, or feature source changes.
No schema or SQL changes. No contract changes (regen is a
no-op).

## Verification (Linux, this VM)

| Check | Result |
|---|---|
| `cargo audit` before / after | RUSTSEC-2026-0459 present → 0 mentions |
| Vendored `sqlite3.h` | 3.53.2 → 3.54.1 |
| Rust lib (`cargo test --locked --workspace --lib`) | **359/359** (baseline 358; +1 new version test; api 4, core 292, desktop 1, providers 62) |
| Provider-dispatch integration | 22/22 |
| Focused lib filters | storage 3/3, vault 4/4, restore 11/11, snapshots 7/7, search 4/4, docx 9/9, pdf 24/24, ingestion 20/20 |
| tsc (`pnpm typecheck`) | 0 errors |
| Contracts (`node scripts/contracts.mjs` re-run) | 0 drift |
| Docs (`node scripts/validate-docs.mjs`) | 338 PASS (was 336; +2 new handoff docs: design + this handoff) |
| UI (`pnpm test:ui`) | 338/338, unchanged |
| E2E `documents-docx.spec.ts` | 7/7 |
| E2E `documents-pdf.spec.ts` | 5/5 |
| E2E `documents-ingestion.spec.ts` (TXT/MD) | 9/9 |
| E2E `ask_anansi.spec.ts` | 1/1 |

**Existing-workspace reopen** (`evidence/reopen-check.mjs`,
log `evidence/reopen-check.log`): an API binary built from
the DOCX tip (SQLite 3.53.2) created a workspace, a note, and
a DOCX import (297 chars, 4 sections; catalog page_size
4096, WAL checkpointed on stop). The new binary (SQLite
3.54.1) reopened the same storage root and passed every
check: workspace listed, document intact (297 chars, 4
sections), note intact, FTS search finds the document, a new
write succeeds, backup job Completed, artifact 146,432 bytes
with its hash matching the job record and containing
`manifest.json` + `catalog.sqlite3`, preview ok, fresh
restore ok, restored workspace contains the document.

Pre-existing, unchanged: `mcp.rs:715` dead-code warning;
`providers.spec.ts` 240 s timeout; `larp-somnia.spec.ts`
instruments test; `backups.spec.ts` 'Workspace' button;
4 `semantic_workflow` live-Ollama failures. None re-run
here except where noted; none is a regression.

## Windows rerun checklist (for Antigravity)

```powershell
git fetch <bundle> ; git checkout muse/pilot-sqlite-security-repair
node scripts/cargo.mjs audit
node scripts/cargo.mjs test --locked --workspace --lib
node scripts/cargo.mjs test --locked -p anansi-core --test provider_dispatch
pnpm typecheck
node scripts/contracts.mjs ; git status --short packages/contracts/  # expect clean
node scripts/validate-docs.mjs
pnpm test:ui
node scripts/cargo.mjs build --locked -p anansi-api
cd apps/web ; node node_modules/vite/bin/vite.js build ; cd ../..
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/documents-docx.spec.ts
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/documents-pdf.spec.ts
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/documents-ingestion.spec.ts
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/ask_anansi.spec.ts
```

Expected: audit clean of RUSTSEC-2026-0459, lib 359/359,
provider-dispatch 22/22, tsc 0, contracts 0 drift, docs 338
PASS, UI 338/338, DOCX e2e 7/7, PDF e2e 5/5, TXT/MD e2e 9/9,
Ask e2e 1/1. The reopen check is Linux evidence only; its
script is in the delivery evidence folder if Windows wants
an equivalent run (it needs both binaries, old tip +
repaired).

## Proposed status update (for Antigravity's documents — not edited here)

- No requirement-matrix change is proposed: this repair
  closes no feature requirement; it resolves the
  RUSTSEC-2026-0459 advisory flagged during the M13 DOCX
  slice. If Antigravity tracks advisories, record
  RUSTSEC-2026-0459 as resolved by rusqlite 0.41.1 /
  libsqlite3-sys 0.39.1 / bundled SQLite 3.54.1 on this
  branch, pending Windows verification.

## Not done / follow-ups

- No legacy-schema fixture exists in the repo (schema 8–10
  catalogs are only covered by the migration chain code and
  the vault's accepted-version check); the reopen check
  covers a current-schema workspace written by the old
  binary, not a schema-8 file.
- ctutils RUSTSEC-2026-0385 and the other audit warnings
  remain open, out of scope for this repair.
- The `bundled-sqlcipher` feature in libsqlite3-sys 0.39.x
  still bundles SQLite 3.53.4 (affected per the advisory);
  ANANSI does not use it.
