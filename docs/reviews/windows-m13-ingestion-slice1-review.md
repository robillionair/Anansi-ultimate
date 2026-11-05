# M13 Document Ingestion Slice 1 — Windows Review & Independent Verification Report

**Review Date**: 2026-11-05  
**Reviewer**: Antigravity (Windows Integration & Readiness Owner)  
**Author**: Muse (Linux Ingestion Owner)  
**Baseline**: `7e24859e6c042aaa135d632ad7009d20ab3c35e1` (`codex/coordination-progress-2026-11-05`)  
**Delivery Archive**: `anansi-m13-document-ingestion-2026-11-05.zip`  
- Archive SHA-256: `f1f810cae2f18acb6b4f117736727941b86b372e26031de14d808d9334149dfe` (8,830,730 bytes)  
**Delivery Bundle**: `anansi-m13-document-ingestion.bundle`  
- Bundle SHA-256: `49e1227fa45a149a75a658d9bc0e97d196c6928dadaffdf3965d9c621aa50d02` (8,866,360 bytes)  
**Branch**: `muse/m13-document-ingestion`  
**Tip Commit**: `ba3f5a724e4c56c575fae2563cb93d073fdb5cbe`  
**Review Worktree**: `E:\Anansi\worktrees\review-m13-document-ingestion`  
**Review Branch**: `codex/review-m13-document-ingestion`

---

## 1. Executive Verdict & Summary

Muse's M13 Document Ingestion Slice 1 delivery is a high-quality, architecturally sound, and strictly bounded implementation of TXT and Markdown document ingestion under MASTER_SPEC §12 and AN-025.

- **Design Precedence**: Design doc (`70ebe4665`) committed before code; scope explicitly bounded to TXT/MD (M13 is honestly declared incomplete; AN-025 remains in progress).
- **Architecture**: Clean reuse of existing identities, persistent SQLite `jobs` table (without schema migration, `SCHEMA_VERSION` remains 11), bounded chunked uploads (`ingest-begin` / `ingest-part`, 64 KiB sequential parts), atomic publish guard with deduplication, inert React Markdown rendering (zero `dangerouslySetInnerHTML`), and FTS full-body indexing.
- **Verification Results**:
  - `npm run typecheck`: **0 errors**
  - `node scripts/contracts.mjs --check`: **0 contract drift**
  - `node scripts/validate-docs.mjs`: **289 Markdown files PASS**
  - `npm run test:ui`: **320 passed; 0 failed** (314 baseline + 6 new document tests)
  - `cargo test --workspace --locked --lib`: **262 passed; 0 failed** across all crates (`anansi_core`: 196, `anansi_providers`: 60, `anansi_api`: 5, `anansi_desktop_lib`: 1)
  - Web production build (`apps/web` Vite): 137 modules transformed clean in 5.88s
  - Playwright browser suite: **7 of 9 tests PASS**; **2 Windows findings identified** with exact root causes and reproducible probes.
  - Confirmed pre-existing baseline finding: `dashboard.spec.ts:47` quick actions failure is a pre-existing LARP HUD pointer interception issue on baseline `7e24859e`.

---

## 2. Windows Rerun Evidence

| Verification Tier | Command | Linux Result | Windows Result | Status |
|-------------------|---------|--------------|----------------|--------|
| TypeScript Typecheck | `npm run typecheck` | 0 errors | 0 errors | **PASS** |
| Contracts Check | `node scripts/contracts.mjs --check` | 0 drift | 0 drift | **PASS** |
| Documentation Links | `node scripts/validate-docs.mjs` | 289 files PASS | 289 files PASS | **PASS** |
| UI Unit Tests | `npm run test:ui` | 320/320 PASS | 320/320 PASS | **PASS** |
| Rust Workspace Lib | `cargo test --workspace --locked --lib` | 262/262 PASS | 262/262 PASS | **PASS** |
| Production Web Build | `node node_modules/vite/bin/vite.js build` | Clean | 137 modules clean (5.88s) | **PASS** |
| API Executable Build | `cargo build --locked -p anansi-api` | Clean | Clean (1m 55s) | **PASS** |

### Playwright Ingestion Suite (`tests/e2e/documents-ingestion.spec.ts`)
1. `documents UI: import a text file, open it, provenance visible, search finds it`: **PASS** (14.0s)
2. `documents UI: markdown renders inertly and a PDF pick is refused`: **PASS** (9.4s)
3. `documents API: invalid UTF-8 forced through the API fails the job and publishes nothing`: **PASS** (8.2s)
4. `documents API: oversize, sealed, hash-mismatch and unsupported-format refusals`: **PASS** (8.7s)
5. `documents API: cancel during upload publishes nothing and removes staging`: **PASS** (5.8s)
6. `documents API: concurrent duplicate imports create exactly one record`: **FAIL (503 vs 200)** — *See Finding 2*
7. `documents API: killing the API mid-upload, restarting, and resuming imports once`: **FAIL (ss command)** — *See Finding 1*
8. `documents API: backup and fresh restore preserves the import`: **PASS** (30.6s)
9. `documents Ask: imported content is cited, and a missing topic is honest`: **PASS** (10.9s)

---

## 3. Disclosed Findings for Partner Agent Muse

Per ANANSI policy, findings are handed over to the owning agent as exact reproducible probes with diagnosed root causes rather than overlapping edits.

### Finding 1: Windows Incompatibility in Test 7 (`ss` command and missing `.exe`)
- **Location**: `tests/e2e/documents-ingestion.spec.ts` lines 338 and 344.
- **Symptom**:
  ```
  Error: Command failed: ss -ltnp "sport = :57100"
  'ss' is not recognized as an internal or external command, operable program or batch file.
  ```
- **Root Cause**:
  1. `ss` is an iproute2 Linux utility; it does not exist on Windows.
  2. `spawn(path.resolve('target/debug/anansi-api'))` at line 344 lacks `.exe` on Windows.
  3. `session` in `tests/e2e/collider-three-helpers.ts` already spawns the child process (`child` at line 120 / 151), but does not expose `child.pid` or a `killApi()` helper on the returned `Session` object.
- **Recommended Remediation**:
  Expose `apiPid: child!.pid` (or `killApi: () => child!.kill('SIGKILL')`) on `Session` in `tests/e2e/collider-three-helpers.ts`. In `documents-ingestion.spec.ts`, replace the `ss` invocation with `process.kill(session.apiPid, 'SIGKILL')` and use `path.resolve('target/debug/anansi-api' + (process.platform === 'win32' ? '.exe' : ''))` when respawning.

### Finding 2: Deterministic SQLite Lock Contention in Test 6 (`ingest-begin` returns 503)
- **Location**: `crates/core/src/ingestion.rs` `sweep_staging` and `enqueue_ingestion`.
- **Symptom**:
  ```
  Error: expect(received).toBe(expected)
  Expected: 200
  Received: 503
  ```
  API log:
  ```
  API STDERR: SWEEP ERROR in enqueue_ingestion: CommandError { code: StorageUnavailable, message: "Local storage is unavailable. Check its configured folder and permissions, then retry." }
  ```
- **Root Cause**:
  In `crates/core/src/ingestion.rs`:
  ```rust
  self.open(&r.workspace_id)?;
  sweep_staging(self, &r.workspace_id)?;
  let mut connection = self.connection(false)?.ok_or_else(unavailable)?;
  let tx = connection
      .transaction_with_behavior(TransactionBehavior::Immediate)
      .map_err(|_| unavailable())?;
  ```
  When two `ingest-begin` calls run concurrently via `Promise.all`:
  1. Request A creates a staged file `<id>.upload` on disk, then acquires `TransactionBehavior::Immediate` inserting into `jobs`.
  2. Request B calls `sweep_staging`, which reads the directory, sees Request A's newly created file, and opens a separate connection executing `SELECT state FROM jobs WHERE workspace_id=?1 AND id=?2`.
  3. On Windows NTFS file locking, reading `jobs` while Request A holds an uncommitted immediate transaction on another connection hits SQLite lock contention and fails with `unavailable()` (HTTP 503).
  4. Muse observed this on Linux as an "unreproduced flake" (Addendum §3.6); on Windows NTFS, it reproduces deterministically.
- **Recommended Remediation**:
  In `crates/core/src/ingestion.rs`:
  - Either ignore/tolerate `DatabaseBusy` or transient errors inside `sweep_staging` so a cleanup sweep never aborts an active `enqueue_ingestion`, OR
  - Pass the active connection to `sweep_staging` and only sweep before creating new staged files.

---

## 4. Milestone 13 Scope Status

- **MCP Integration (Slices 1–5)**: Closed and verified on Windows (AN-017 verified).
- **Document Ingestion (Slice 1 TXT/MD)**: Vertical slice functional; 7/9 Playwright tests passing on Windows; 2 Windows findings diagnosed and reported for Muse remediation.
- **AN-025**: Retained as `in progress` (TXT/Markdown foundation delivered; PDF text extraction planned next).
