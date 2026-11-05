# Fresh Current-Source Release Qualification Report
**Date**: 2026-11-01  
**Lead Reviewer & Release Integrator**: Antigravity (Windows Review Environment)  
**Application Baseline**: `68ee353291d2a5732ccdeaa340807590e502b572` (Verified M9 SwarmTank review tip)  
**Release Tooling Checkpoint**: `167b50dfc8cd6da4dff81c9795aedf567a7129d3` (Clean commit, `dirty: false`)  
**Scope**: Verification of Work Package B requirements — fresh compilation of unsigned release artifacts strictly onto isolated `E:` drives, verification of digests and lengths, guarded launch and restart testing under the shared application-test lease, real application API workspace and memory creation, application readiness verification, diagnostic privacy compliance, and zero process/lease leaks.

---

## 1. Executive Summary & Verification Findings

Prior release artifacts in `E:\Anansi\artifacts\release-tooling-integration` identified an older baseline (`795aab00`) with `dirty: true`. Per release qualification mandates, those historical artifacts were not reused.

A fresh current-source compilation was executed directly from clean checkpoint `167b50dfc8cd6da4dff81c9795aedf567a7129d3` using dedicated, isolated build and temp directories on `E:`.

### Core Verification Highlights
1. **Source Integrity**: Working tree was committed prior to build; `release-manifest.json` records `dirty: false` at `167b50dfc8cd6da4dff81c9795aedf567a7129d3`.
2. **Strict Storage Isolation**:
   - Build cache: `E:\Anansi\build-cache\release-qualification-2026-11-01` (1.88 GiB)
   - Temp dir: `E:\Anansi\tmp\release-qualification-2026-11-01`
   - Artifacts dir: `E:\Anansi\artifacts\release-qualification-2026-11-01`
   - Host `C:` Drive Free Space: 35.96 GiB (0 byte escape/growth during build).
3. **Artifact Completeness**:
   - Web frontend bundle (`apps/web/dist` -> `artifacts/web`): Vite production build in 7.67s.
   - Core API server (`anansi-api.exe`): Compiled via `cargo build --release -p anansi-api --locked` in 226.44s (11.41 MiB).
   - Desktop executable (`anansi-desktop.exe`): Compiled via `tauri build --no-bundle` in 437.22s (18.37 MiB).
   - Total build time: 672.72s.
4. **Offline Manifest Verification**:
   - `build-verify.mjs verify` PASSED: All 5 recorded artifacts match exact byte lengths and SHA-256 hashes.
5. **Real Application / API Path Qualification**:
   - Unlike synthetic tests that hand-craft a minimal SQLite file, acceptance created a real logical workspace and memory note through the live `anansi-api.exe` process over HTTP localhost.
   - Security Boundary: Verified that requests without `Host: 127.0.0.1:<port>` and `Origin: http://127.0.0.1:1420` fail closed with HTTP 403 Forbidden.
   - Live endpoints exercised: `POST /v1/workspaces/create`, `POST /v1/memories/create`, `GET /v1/readiness`, `GET /v1/workspaces/list`.
   - Real catalog generated: Schema `user_version = 11`, `application_id = 0x414e414e`, all 19 domain tables migrated, `.anansi-root` marker verified.
6. **Guarded Launch & Restart Smoke**:
   - Under shared lease `anansi-application-test.lease.json`:
     - Phase 1 Desktop Launch: PID 87628 (7 owned descendant processes), 4000ms smoke window, zero early exits, catalog contract verified. Clean teardown confirmed.
     - Phase 2 Restart Launch: PID 88436 (7 owned descendant processes), 4000ms smoke window, verified persistence of the real workspace (`462edcb0f62e03a615569609027a5657`) and memory note across restart. Clean teardown confirmed.
7. **Process & Lease Cleanup**:
   - Zero leaked processes (`anansi-api.exe`, `anansi-desktop.exe`, `WebView2` descendants confirmed 100% terminated).
   - Shared lease released immediately.
8. **Diagnostic Privacy Compliance**:
   - Synthetic canary redaction tests: 100% PASS.
   - Exported diagnostics bundle: allowlisted bounded telemetry only; zero personal workspaces, notes, prompt contents, tokens, or databases included.

---

## 2. Release Artifact Inventory & Checksums

| Artifact File | Size | Size (Bytes) | SHA-256 Hash |
| :--- | :--- | :--- | :--- |
| **`anansi-desktop.exe`** | 18.37 MiB | 19,266,048 | `4a5842ad9e7cdc10f00f8b7ab1d269b2a2c594106d36251f9bf05c797fe6f0a6` |
| **`anansi-api.exe`** | 11.41 MiB | 11,965,440 | `f874b8609c63c11c614387d63cf3be27523e0563a786e5f27ff868814c3f4f9e` |
| **`web/index.html`** | 397 B | 397 | `c34a5d0c45462cae6614e52dcf99274f7fbcfdb5acb3d5133cd353c2d800386e` |
| **`web/assets/index-Brz0Ux2P.js`** | 2.46 MiB | 2,582,197 | `a2fe3ae0bc32b702a6bf24b79ee6ab798e4fa4b26cd62558517ee5510fbd1595` |
| **`web/assets/index-BwbRqKgE.css`** | 7.50 KiB | 7,675 | `13c28e99375dc0257f885bf549364f36f05448057172a193ca80682178cbe6fc` |

---

## 3. Real Application API Acceptance Evidence

### Execution Trace
1. **Isolated Pairing Root**: Created on `E:\Anansi\tmp\release-qualification-2026-11-01\api-pairing-<uuid>` with strict NTFS ACL (`protect.ps1`).
2. **Server Spawn**: Spawned `anansi-api.exe` with `ANANSI_STORAGE_ROOT = E:\Anansi\tmp\release-qualification-2026-11-01\acceptance-data-82ad66c2-8001-4dff-8423-f99f9159d4e9`.
3. **Session Handshake**: Discovered session pairing token and endpoint `http://127.0.0.1:14728`.
4. **Security Enforcement**:
   - Initial call without CSRF origin failed closed (HTTP 403 Forbidden).
   - Valid call with `Origin: http://127.0.0.1:1420` and `Host: 127.0.0.1:14728` authenticated via bearer token.
5. **Real Domain Commands**:
   - `POST /v1/workspaces/create` -> Created logical workspace `462edcb0f62e03a615569609027a5657` ("Qualified Real Acceptance Workspace").
   - `POST /v1/memories/create` -> Created memory note `b8973e4aa67283fa0744d4139f6c5ee7` with revision `08b88ab75d31f67ca3b0863001e2aa50`.
   - `GET /v1/readiness` -> Verified `status: "Ready"`.
   - `GET /v1/workspaces/list` -> Verified workspace listed.
6. **API Shutdown**: `anansi-api.exe` terminated cleanly with zero orphan processes.
7. **Storage Contract Verified**:
   - `PRAGMA application_id`: `1095647566` (`0x414e414e`).
   - `PRAGMA user_version`: `11` (fully migrated).
   - Tables verified: `workspaces`, `workspace_settings`, `memories`, `memory_revisions`, `lifecycle_format`, `relationships`, `jobs`, `backup_schedules`, `artifact_versions`, `memory_embeddings`.

---

## 4. Desktop Guarded Launch & Restart Verification

- **Launch ID**: `82ad66c2-8001-4dff-8423-f99f9159d4e9`
- **Data Root**: `E:\Anansi\tmp\release-qualification-2026-11-01\acceptance-data-82ad66c2-8001-4dff-8423-f99f9159d4e9`
- **Smoke Duration**: 4000ms per phase.
- **Phase 1 (Initial Launch)**:
  - PID: `87628`
  - Descendant PIDs tracked: `[87628, 37120, 46444, 77636, 45832, 82608, 88980]`
  - Early Exit: `false`
  - Teardown Confirmed: `true`
  - Application Readiness: `VERIFIED`
- **Phase 2 (Restart Persistence)**:
  - PID: `88436`
  - Descendant PIDs tracked: `[88436, 86748, 86472, 70160, 62196, 19888, 35472]`
  - Early Exit: `false`
  - Teardown Confirmed: `true`
  - Workspace Preserved: `462edcb0f62e03a615569609027a5657`
  - Memory Note Preserved: `b8973e4aa67283fa0744d4139f6c5ee7`
  - Restart Persistence: `VERIFIED`
- **Zero Orphan Processes**: Confirmed all PIDs terminated upon exit.

---

## 5. Support Diagnostics Privacy Audit

- **Canary Redaction**: Tested synthetic secret canaries (`sk-...`, bearer tokens, private file paths). Redaction rate: **100%**.
- **Export Bundle**: `E:\Anansi\artifacts\release-qualification-2026-11-01\diagnostics\anansi-support-diagnostics-2026-11-01T22-46-09-426Z.json`
- **Export Digest**: `3180a8b0d2e37b787cbd9ca88ccc64eee0d687ace9eced9e9a13f855d907f0a4`
- **Privacy Audit**: Inspected JSON file character-for-character. Verified strictly allowlisted fields only: system CPU/RAM, drive space, toolchain versions, manifest validation booleans, launch phase exit codes. Zero user data, secrets, or memory content exported.

---

## 6. Qualification Status & Verdict

- **Unsigned Desktop & API Local Release Qualification**: **PASS**
- **Clean Source Checkpoint**: `167b50dfc8cd6da4dff81c9795aedf567a7129d3` (`dirty: false`)
- **Gates Remaining**: Host installer execution (requires disposable VM), production code signing (M5/M15), preview promotion.
