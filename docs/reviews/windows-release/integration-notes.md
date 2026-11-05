# ANANSI Windows Release Engineering — Integration Notes & Qualification Report

**To**: Lead Integrator & Muse (Parallel Contributor)  
**From**: Independent Windows Release Engineering Agent  
**Date**: October 30, 2026  
**Worktree**: `E:\Anansi\worktrees\review-windows-release`  
**Branch**: `codex/review-windows-release-final`  
**Reviewer Checkpoint**: `628745347a7e875d88b07d6323acc9550deab3bb`  
**Status**: FINDINGS WR-01, WR-01b, WR-01c, WR-01d, WR-02, WR-02b REPAIRED, HARDENED & QUALIFIED — APPLICATION REMAINS STRICTLY UNMODIFIED

---

## 1. Scope & Parallel Ownership Adherence

In accordance with agent execution instructions:
- **Application Code Untouched**: Release engineering authored zero modifications to:
  - `crates/core/`, `crates/api/`, `apps/desktop/src/`, `apps/web/src/`, `packages/`
  - Database schema migrations and restore logic
  - Manifests (`package.json`, `Cargo.toml`, `pnpm-workspace.yaml`, lockfiles)
  - Existing scripts (`scripts/desktop.mjs`, `scripts/build-desktop-test.mjs`, `scripts/verify-installer.ps1`)
  - Shared repository documentation (`docs/BUILD_STATUS.md`, `docs/ROADMAP.md`, `docs/MASTER_SPEC.md`)
  - GitHub workflows (`.github/workflows/`)
- **Exclusive Write Locations**:
  - `scripts/windows-release/`
  - `tests/windows-release/`
  - `docs/reviews/windows-release/`
- **Zero C: Drive Growth**:
  - Build cache strictly on `E:\Anansi\build-cache\antigravity-release`.
  - Temp files strictly on `E:\Anansi\tmp\antigravity-release`.
  - Deliveries and artifacts strictly on `E:\Anansi\artifacts` and `E:\Anansi\deliveries`.
  - All existing user previews (`Anansi-m5-review`, `%LOCALAPPDATA%\ANANSI-Preview\r11`) preserved untouched.

---

## 2. Commit List for Integration

The following linear, reviewable commits represent the release engineering deliverable:

| Commit SHA | Type | Summary |
| :--- | :--- | :--- |
| `a4654d7c4` | `docs(release)` | Define Windows release engineering acceptance criteria prior to implementation |
| `f38fb48bb` | `feat(release)` | Package 1 — system preflight checks, disk accounting and cleanup proposal |
| `0e9b4680e` | `feat(release)` | Package 2 & 3 — build-verify wrapper, offline manifest verifier, and guarded launch acceptance |
| `857618268` | `feat(release)` | Package 4 & 5 — privacy diagnostics, acceptance tests, operator guide, and build report |
| `d71dd4d19` | `docs(release)` | Define qualification, stress testing, and repeatability acceptance criteria |
| `a7e636f87` | `feat(release)` | Harden qualification stress, junction guards, and secret canary redaction |
| `d8c542e72` | `feat(release)` | Multi-run repeatability verification and integration notes |
| `182d3b866` | `docs(release)` | Record final commit list in integration notes |
| `6ff4960c4` | `feat(release)` | Make verify-delivery support arbitrary bundle names and multiple test suites |
| `060597d6b` | `feat(release)` | Package-final-delivery script for packaging and verification |
| `9ceb1c5ee` | `feat(release)` | Robust bundle ref handling and packaging script stage inclusion |
| `c85509e86` | `test(review)` | Reviewer baseline: expose launch false positives and free-form diagnostic leakage |
| `5b8efce51` | `fix(release)` | Repair WR-01 launch qualification, WR-02 diagnostic allowlist, and WR-03 reconciliation |
| `3aa2ff51a` | `docs(release)` | Record repair commit hash in integration notes |
| `b6b3b9654` | `test(review)` | Reviewer follow-up: expose early-zero qualification and untyped diagnostics |
| `ee26c8eda` | `fix(release)` | Repair WR-01b premature exit, storage verification & lease teardown, and WR-02b typed bounds |
| `dfb2ace0d` | `docs(release)` | Record repair commit hash in integration notes |
| `f892d718b` | `test(review)` | Reviewer checkpoint: catalog integrity probe exposing header-only SQLite qualification |
| `f5155b390` | `fix(release)` | Repair WR-01c database integrity check, schema validation, and process cleanup confirmation |
| `640d81cb8` | `docs(release)` | Record exact repair commit hash in integration notes |
| `628745347` | `test(review)` | Reviewer checkpoint: distinguish generic SQLite integrity from ANANSI readiness |
| `6290f9305` | `fix(release)` | Repair WR-01d complete storage contract, record verification across restart, and serial lease suite |

---

## 3. Explicit Context, Recovery Status & Boundary Reconciliations

Release engineering explicitly records the following context, addressing reviewer findings WR-01, WR-01b, WR-01c, WR-01d, WR-02, WR-02b, and WR-03:

### 3.1 Historical Recovery Baseline vs Current Application State (WR-03)
- **Baseline Context**: The release engineering tooling was originally developed and exercised against baseline `240af4ef58b09e7ada405286f46f927c902f9c52` (incoming `edba7ed32db2f7c5ea999466061c4e1c3966b2e7`).
- **Historical Defects**: Findings R1–R4 (forged restored-hash provenance, post-fixup stale context hash, saved/live anchor collision, and multi-restore provenance loss across generations B -> C -> D) were defect findings on that older runtime.
- **Current Production State**: Current independent lineage acceptance is recorded on commit `c50d6b03` and schema 11 runtime `06a3f573a`.
- **Strict Boundary**: This worktree does **not** overwrite newer application code or claim its older binary is the current production application. The delivered release tooling is designed to be integrated into trunk without altering core runtime code.

### 3.2 Reproducibility Reconciled with Measured Differences (WR-03)
- **Honest Reproducibility Verdict**: Hash-different executables are **NOT bit-identical reproducible builds**.
- **Measured Section Layout**:
  - 5 of 6 sections (`.text`, `.data`, `.pdata`, `.rsrc`, `.reloc`) are **100% bit-for-bit identical**.
  - Section `.rdata` contains measured differences (`0.0001%` total binary byte variance).
- **PE Header Arithmetic**: The PE signature offset `peOffset` is read from the DOS header at `0x3C` (`e_lfanew`). The COFF File Header begins at `peOffset + 4`, and offset `peOffset + 8` stores the 32-bit compilation epoch (`TimeDateStamp`). Linker timestamps and CodeView debug directory GUIDs embedded at link time account for the measured variance.
- **Frontend Determinism**: The web distribution (`dist/`) is 100% bit-for-bit deterministic.

### 3.3 Strict Diagnostic Allowlist and Type/Bound Constraints (WR-02 & WR-02b)
- Default support diagnostic exports (`support-diagnostics.mjs`) enforce strict type and bound validation across all fields:
  - Strings constrained by length and regex (`validateString`).
  - Numbers strictly checked for finite range, integer constraint, and bounds (`validateNumber`).
  - Dates validated as ISO strings (`validateIsoDate`).
  - UUIDs validated against RFC-4122 pattern (`validateUuid`).
  - Arrays validated element-by-element with count caps (`validateStringArray`).
- Hostile nested objects, arbitrary arrays, out-of-bounds metrics, or strings in numeric fields are rejected/omitted without special-casing canaries.
- Arbitrary build log lines (`build-execution.log`), console excerpts, and free-form error text are **strictly omitted** from default exports by design.

### 3.4 Rigorous Launch Qualification, Storage Contract & Process Teardown (WR-01, WR-01b, WR-01c & WR-01d)
- `runGuardedLaunch` strictly fails on:
  - Premature exits during the smoke window regardless of exit code (early exit code 0 or non-zero).
  - Spawn errors or missing executables.
  - Complete ANANSI Storage Contract (`ANANSI_STORAGE_CONTRACT`):
    - Core catalog required: must create `workspaces.sqlite3`.
    - Marker-only refusal: root containing `.anansi-root` alone returns `ready: false` with actionable `initializationOnly: true`; markers alone never establish full application readiness.
    - True SQLite database integrity validated in read-only mode using Node 22+ built-in `node:sqlite` (`DatabaseSync` with `PRAGMA integrity_check`).
    - Minimum valid SQLite page size enforced (size >= 512 bytes), rejecting header-only and padded corrupt databases.
    - Supported schema version range enforced: `PRAGMA user_version` must be within `1..11`; unrelated valid databases (e.g. `user_version = 0`) are refused.
    - ANANSI application ID verified: `PRAGMA application_id` must match `0x414e414e` (1095647566 decimal) when set.
    - Required schema tables enforced: `workspaces` and `workspace_settings` must exist; databases with foreign tables (e.g. `CREATE TABLE unrelated(...)`) are strictly refused.
    - Required schema columns enforced: `workspaces` must contain `id, name, name_key`; `workspace_settings` must contain `workspace_id, reduced_motion`.
  - Record Identity and Value Verification across Restart:
    - Phase 1 baseline records (`readiness.catalogSnapshot`) are automatically captured and verified against Phase 2 restart state.
    - Deleted records: any workspace record present in Phase 1 that is deleted across restart fails persistence (`persistent: false`).
    - Modified records: any workspace record whose values (`name`, `name_key`) or settings (`reduced_motion`) are altered across restart fails persistence.
    - Replaced records: replacing existing records with different identifiers fails persistence.
    - Explicit expected workspace identity and value verification supported via `{ expectedWorkspace: { id, name } }`.
  - Reparse point and symlink guards:
    - Data root and individual storage artifacts are actively verified and rejected if detected as NTFS junctions, reparse points, or symlinks.
  - Process tree teardown confirmation:
    - Pre-kill inventory: owned process hierarchy is inventoried via `Get-CimInstance Win32_Process` *before* sending termination signals.
    - Parent-first-exit handling: recursive tree enumeration tracks detached child and grandchild processes even if parent exits first.
    - Enumeration failure treatment: any WMI/CIM enumeration failure is treated as an unconfirmed teardown rather than an empty descendant list.
    - Lease retention: if process-tree teardown cannot be confirmed within timeout, the shared application-test lease is **retained on disk** with `blockedReason: 'TEARDOWN_CONFIRMATION_FAILED'`.
- Failure evidence is retained in `launch-evidence.json` and the synthetic root is preserved in `failed-runs/failed-<id>`.

### 3.5 Installer Dynamic Execution Boundary
- Dynamic installation and uninstallation on the host is **UNVERIFIED** because `WindowsSandbox.exe` is not enabled on this host. Running unverified installers on the host account is prohibited.
- Disposable VM / Windows Sandbox verification procedures and static NSIS payload verification are documented in [`windows-sandbox-installer-procedure.md`](windows-sandbox-installer-procedure.md).

---

## 4. Reviewer Verification Procedure

The integrator can verify the complete release engineering toolchain using the canonical serial command:

### 4.1 Canonical Serial Shared-Lease Suite Execution (74 Automated Tests)
```powershell
node --test --test-concurrency=1 tests/windows-release/*.test.mjs tests/docs-validation.test.mjs
```
*Expected: 74/74 tests pass cleanly, duration ~67s, zero lease collisions or unreleased leases at completion.*
- `reviewer-storage-contract.test.mjs`: 2/2 pass
- `reviewer-catalog-integrity.test.mjs`: 2/2 pass
- `reviewer-followup.test.mjs`: 2/2 pass
- `reviewer-probes.test.mjs`: 2/2 pass
- `repair-verifications.test.mjs`: 19/19 pass
- `acceptance-criteria.test.mjs`: 15/15 pass
- `qualification-stress.test.mjs`: 11/11 pass
- `docs-validation.test.mjs`: 21/21 pass

### 4.3 Run Preflight System & Resource Check
```powershell
node scripts/windows-release/preflight.mjs --json
```

### 4.4 Run Standalone Build & Offline Manifest Verification
```powershell
node scripts/windows-release/build-verify.mjs --standalone-only
node scripts/windows-release/build-verify.mjs --verify-only
```

### 4.5 Run Isolated Launch & Restart Persistence Acceptance
```powershell
node scripts/windows-release/launch-acceptance.mjs --restart --smoke-duration 4000
```

### 4.6 Run Support Diagnostics & Synthetic Canary Audit
```powershell
node scripts/windows-release/support-diagnostics.mjs --test-canary
node scripts/windows-release/support-diagnostics.mjs --export
```

### 4.7 Run Multi-Run Repeatability & PE Header Comparison
```powershell
node scripts/windows-release/repeatability-compare.mjs
```

---

## 5. Non-Intrusive Proposed Root Script Integration Patch

To maintain strict parallel ownership, release engineering **did NOT edit root `package.json`**.  
When the lead integrator is ready to expose these tools at the workspace root, the following script additions are proposed:

```diff
--- a/package.json
+++ b/package.json
@@ -28,6 +28,13 @@
     "desktop:installer": "node scripts/desktop.mjs installer",
     "desktop:test": "node scripts/build-desktop-test.mjs",
+    "release:preflight": "node scripts/windows-release/preflight.mjs",
+    "release:build": "node scripts/windows-release/build-verify.mjs",
+    "release:verify": "node scripts/windows-release/build-verify.mjs --verify-only",
+    "release:launch": "node scripts/windows-release/launch-acceptance.mjs",
+    "release:launch:restart": "node scripts/windows-release/launch-acceptance.mjs --restart",
+    "release:diagnostics": "node scripts/windows-release/support-diagnostics.mjs --export",
+    "release:repeatability": "node scripts/windows-release/repeatability-compare.mjs",
+    "test:release": "node --test tests/windows-release/*.test.mjs"
   },
```

---

## 6. Verified Application Baseline Integration Evidence

- **Integration Worktree**: `E:\Anansi\worktrees\release-tooling-integration`
- **Integration Branch**: `codex/release-tooling-integration`
- **Application Baseline Commit**: `7c576b85eab5d76e8713ff2debf89f1e7a3756d4` (incorporating Windows-verified §26 Three.js visualization `4ced13bfe` and §27 Artifacts & Forge bridge `7c576b85e`)
- **Integration Commit**: `b9295cbf583db61a868478ff2efbfd89e49aa2bf`
- **Release Tooling Source Checkpoint**: `df724182928622c244f7cbf3bbd022fc5cf8425d`
- **Scope Boundary**: Integrated strictly into `scripts/windows-release/`, `tests/windows-release/`, and `docs/reviews/windows-release/`. Application code, domain contracts, migrations, package manifests, and documentation repairs remain completely untouched.

### 6.1 Test Suite & Documentation Verification
- **Command**: `node --test --test-concurrency=1 tests/windows-release/*.test.mjs tests/docs-validation.test.mjs`
- **Verdict**: **PASS (74/74 passing, 0 failures, 0 skipped, 71.36s)**
  - All reviewer probe suites (`reviewer-storage-contract`, `reviewer-catalog-integrity`, `reviewer-followup`, `reviewer-probes`) pass cleanly.
  - Full repair suites, qualification stress suites, acceptance criteria, and repository documentation validation pass cleanly.

### 6.2 Application Build Verification
- **Command**: `node scripts/windows-release/build-verify.mjs build --cargo-target-dir E:\Anansi\build-cache\release-tooling-integration --artifacts-dir E:\Anansi\artifacts\release-tooling-integration --temp-dir E:\Anansi\tmp\release-tooling-integration`
- **Web Frontend Build**: Vite production build succeeded in 2.58s (`exitCode: 0`).
- **Desktop Executable Compilation**: Tauri `--no-bundle` compilation succeeded in 416.61s (`exitCode: 0`).
- **Total Build Time**: 420.02s.
- **Built Executable**: `anansi-desktop.exe` (18,486,272 bytes, SHA-256: `3dccb65f170f9b2dd42b4af49bdd8c7b14afa9328b08d740285ccbe6b092e2c7`).
- **Offline Artifact Verification**: `node scripts/windows-release/build-verify.mjs verify --manifest E:\Anansi\artifacts\release-tooling-integration\release-manifest.json` verified all 4 artifacts match recorded hashes and byte lengths (100% PASS).

### 6.3 Guarded Launch & Restart Persistence Acceptance
- **Command**: `node scripts/windows-release/launch-acceptance.mjs --restart --smoke-duration 4000 --artifacts-dir E:\Anansi\artifacts\release-tooling-integration --temp-dir E:\Anansi\tmp\release-tooling-integration --exe-path E:\Anansi\artifacts\release-tooling-integration\anansi-desktop.exe --seed-workspace-id ws-acceptance-integration --seed-workspace-name "Integrated Acceptance Workspace"`
- **Verdict**: **PASS (readinessVerified: true, persistenceVerified: true)**
- **Shared Lease Lifecycle**: Acquired exclusively under launch ID `2d3b2d0c-5ebe-488d-8864-854cdecb37ad` and cleanly deleted upon teardown confirmation.
- **Phase 1 (Initial Launch)**: PID 79108; readiness established on private storage root with root marker, SQLite catalog schema 10, application ID 1095647566, and seeded workspace record `ws-acceptance-integration`. Teardown confirmed across all 7 tree processes.
- **Phase 2 (Restart Persistence)**: PID 84708; persistence established across restart, identical catalog snapshot verified, zero orphaned processes, and teardown confirmed across all 7 tree processes.
- **Evidence File**: `E:\Anansi\artifacts\release-tooling-integration\launch-evidence.json`.

### 6.4 Structured Diagnostic Export
- **Command**: `node scripts/windows-release/support-diagnostics.mjs --export --artifacts-dir E:\Anansi\artifacts\release-tooling-integration`
- **Output Bundle**: `E:\Anansi\artifacts\release-tooling-integration\diagnostics\anansi-support-diagnostics-2026-10-31T00-51-43-641Z.json`.
- **Privacy Guarantees**: Strict allowlist of bounded numeric and string system/toolchain/verification metrics only. Zero credentials, zero raw logs, zero database content.

