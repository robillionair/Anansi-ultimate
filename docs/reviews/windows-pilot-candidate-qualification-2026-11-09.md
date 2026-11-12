# ANANSI Windows Pilot Candidate Qualification Report

**Report Date**: 2026-11-09T23:45:00+01:00  
**Candidate Source SHA**: `9eec1c2f6d2a648e64d6c219013098b2f2ae81ef` on `codex/windows-preview-candidate`  
**Base Commit**: `d638edc988936595e7bd963278263121e12d290f`  
**Integrated Repairs**:
- SOMNIA mid-run expected-target contract repair (`af2d4b1646bb7c5a7712efcfd4ee2b95f2e4703d`)
- Qualification reconciliation & benchmark evidence (`90562dec02178dc46ac9b76e09e1f5e624482fff`)
- Workspace navigation dashboard fix (`aee3cb9f012a2d69f02b117838785a646e6dc197`)
- M13 DOCX text-layer ingestion (`6ff0d22e7e2d83428de9f284f689de47c6251187`)
- Pilot SQLite security repair (`5925c93e6a68e83c3f63e0206dcf5b66669eb668` — rusqlite 0.41.1, libsqlite3-sys 0.39.1, SQLite 3.54.1, resolving RUSTSEC-2026-0459 / CVE-2026-119375)

---

## 1. Verified Pilot Installer Artifact

- **File Path**: `E:\Anansi\deliveries\anansi-windows-pilot-candidate-2026-11-09\ANANSI Ultimate_0.1.0_x64-setup.exe`
- **Build Output**: `E:\Anansi\worktrees\review-windows-pilot-qualification\target\release\bundle\nsis\ANANSI Ultimate_0.1.0_x64-setup.exe`
- **File Size**: `10,475,832 bytes` (9.99 MiB)
- **SHA-256 Checksum**:
  `f1ec97c93193c8f9858e3d66f0a9475a890c96b914c8e3ce379ca221ff47899f`
- **Signing Status**: `NotSigned` (Unsigned development NSIS installer built with `--no-sign`)
- **Manifests**: `packaging.json` and `SHA256SUMS` verified and stored alongside installer.

---

## 2. Test Execution Environment & Binary Separation

To prevent ambiguity, every check in this qualification is mapped to its exact execution vehicle:

| Category | Execution Vehicle | What Was Tested |
|---|---|---|
| **Installer Package** | `ANANSI Ultimate_0.1.0_x64-setup.exe` | NSIS package structure, payload extraction identity (`bca7ae56...`), unbundled executable (`0715615d...`), bundle marker `__TAURI_BUNDLE_TYPE_VAR_NSS`, Authenticode `NotSigned` validation. |
| **API Backend Executable** | `target/debug/anansi-api.exe` | Bundled SQLite 3.54.1 runtime linking, schema 11 catalog migration, cross-version storage reopen, FTS5 search, backup creation, and fresh restore. |
| **Web Client Production Bundle** | `apps/web/dist` (Vite 8.3.4) | Production SPA bundles (`index-eBSdsuV8.js`, `index-BaytqDFH.css`), inert Markdown rendering, section provenance, and UI reactivity. |
| **Browser E2E Automation** | Playwright 1.64.1 under shared test lease | 21 focused ingestion/search/citation tests, 16 SOMNIA tests, frozen compaction benchmark, and provider lifecycle specs using file-chooser automation. |

---

## 3. Patched SQLite Reopen, Search, Write & Restore Evidence

**Log File**: `E:\Anansi\evidence\windows-pilot-reopen-check.log`  
**JSON Evidence**: `E:\Anansi\evidence\windows-pilot-reopen-check.json`

### Verification Sequence:
1. **Old Build Population (SQLite 3.53.2 via `anansi-api.exe` @ `6ff0d22e7`)**:
   - Created workspace `433670df06c2f0e80690c4a9ab4faebe`
   - Created note `d84b3c8cb78fe35ee75a5ded540dac04` ("Pre-Upgrade Arch Note")
   - Ingested `normal.docx` (37,050 bytes, sha256 `7574407f...`): job `1e1cb3c2...` Completed, doc id `9cd53e4b...` with 4 sections.
   - Clean shutdown: WAL checkpointed cleanly, `workspaces.sqlite3` = 167,936 bytes.
2. **New Build Reopen (SQLite 3.54.1 via `anansi-api.exe` @ `9eec1c2f6`)**:
   - Reopened same workspace at schema 11.
   - Verified pre-upgrade note `d84b3c8c...` and DOCX `9cd53e4b...` survived with all 4 sections (`["Harbor Ledger", "Customs", "Manifest", "Notes"]`).
   - Verified search query: FTS5 query for `"copper"` returned `normal.docx` (`9cd53e4b...`).
   - Verified active write: created post-upgrade record `803f335ec21bc21da0efeef39228c2bf`.
   - Verified backup: job `f364e7d1...` created 146,432-byte package (SHA-256 `3feee1f407d484d81ea9c3478bd53a906acfebc27ef38049e1c8b03253c2d736`).
   - Verified fresh restore: unpacked into `f5767c1d3735d03f0c16e9dec8efcc33`; all 3 records survived intact.

---

## 4. Consolidated Browser Battery Results (Exact SHA `9eec1c2f6`)

All suites executed sequentially under `scripts/with-application-test-lease.mjs`:

1. `tests/e2e/documents-docx.spec.ts`: **7/7 PASS** (1.8m)
2. `tests/e2e/documents-pdf.spec.ts`: **5/5 PASS** (1.4m)
3. `tests/e2e/documents-ingestion.spec.ts`: **9/9 PASS** (2.0m)
4. `tests/e2e/ask_anansi.spec.ts`: **1/1 PASS** (13.4s)
5. `tests/e2e/providers.spec.ts`: **1/1 PASS** (17.6s)
6. `tests/e2e/somnia-compaction.spec.ts`: **16/16 PASS** (1.8m)
7. `tests/e2e/somnia-compaction-benchmark.spec.ts`: **1/1 PASS** (12.8s)  
   *Result*: 21/50 original, 21/50 compacted, 0 failed pairs, 1600 tokens, 14,078 µs.
8. `tests/e2e/larp-somnia.spec.ts`: **2/2 PASS** (1.4m)

---

## 5. Qualification Status & Open Boundaries

| Qualification Gate | Verdict | Detail |
|---|---|---|
| **Source Integrity & Contracts** | **QUALIFIED** | 0 drift, 338 docs PASS, UI unit tests 338/338 PASS. |
| **SQLite Security Dependency Repair** | **QUALIFIED** | Patched SQLite 3.54.1 verified via runtime tests and cross-version database reopen. |
| **Ingestion & Search Workflows** | **QUALIFIED** | DOCX, PDF, TXT/MD automated browser tests passed 100%. |
| **Clean-Machine Installer Execution** | **PENDING** | Held pending testing on an isolated clean Windows machine/VM. Runbook provided below. |
| **Real Win32 File-Dialog Check** | **PENDING** | Native OS dialog inspection (Cancel click, filter dropdown, Unicode file pick) remains an interactive operator step. |
| **Hardware Vault / Credential Manager** | **PENDING** | Hardware DPAPI/TPM backing checks remain open for operator hardware testing. |
| **Connect AI API-Key Setup** | **SEPARATED** | Muse's `muse/connect-ai-api-key-setup` is held on a separate branch for independent review and packaging. |
