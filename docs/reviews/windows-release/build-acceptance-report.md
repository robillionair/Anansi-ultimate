# ANANSI Windows Release Verification & Acceptance Report

**Date**: October 30, 2026  
**Host**: Windows 11 Pro 64-bit (`win32 x64 10.0.22000`)  
**Worktree**: `E:\Anansi\worktrees\antigravity-release-engineering`  
**Branch**: `codex/antigravity-release-engineering`  
**Source Commit**: `f38fb48bb0df0b22e85252a4fc3fb4092f682ea0` (based on `240af4ef58b09e7ada405286f46f927c902f9c52`)  
**Status**: REPRODUCIBLE BUILD & GUARDED LAUNCH ACCEPTANCE VERIFIED

---

## 1. Executive Summary

A complete, isolated Windows release compilation and launch acceptance sequence was executed strictly utilizing `E:\Anansi` drive storage.

- **Zero C: Drive Growth**:
  - `CARGO_TARGET_DIR`: `E:\Anansi\build-cache\antigravity-release`
  - `TEMP` / `TMP`: `E:\Anansi\tmp\antigravity-release`
  - Artifacts: `E:\Anansi\artifacts\antigravity-release`
- **Build Execution**: Completed in 69.35s with exit code `0`.
- **Offline Manifest Verification**: **PASS** (4/4 artifacts matched byte-for-byte and SHA-256).
- **Guarded Launch Acceptance**: **PASS** (Supervised PID 86536 initialized `.anansi-root`, `vault-admission`, and `workspaces.sqlite3` in an isolated synthetic root on `E:`).
- **Support Diagnostics Export**: **PASS** (Allowlisted diagnostic bundle exported with zero credential or database leakage).

> [!IMPORTANT]
> **Compilation vs. Release Acceptance Distinction & Context Reconciliation**:
> A successful compilation and launch smoke test does **not** constitute production release readiness.
> - **Historical Baseline Context**: Findings R1–R4 were defect findings on the older baseline `240af4ef58b...` / `edba7ed32db...`. Current independent lineage acceptance is recorded on commit `c50d6b03` and schema 11 runtime `06a3f573a`. This release engineering worktree does not overwrite newer application code or claim its older binary is the current production application.
> - **Reproducibility Reconciled**: Hash-different executables are **NOT bit-identical reproducible builds**. The build demonstrates structural reproducibility (exact byte length, identical code size, memory footprint, and identical payloads for 5 of 6 sections), but MSVC linker timestamps (`TimeDateStamp` at `peOffset + 8` relative to `0x3C`) and debug directory CodeView GUIDs prevent bit-level identity.
> - **Installer Verification**: Full NSIS installer acceptance requires execution inside a disposable Windows VM / Sandbox, as documented in `windows-sandbox-installer-procedure.md`. Dynamic execution on the host account is prohibited and marked UNVERIFIED.

---

## 2. Toolchain & Environment Record

| Tool / Runtime | Resolved Path | Version / Host | Verification Method |
|---|---|---|---|
| **Node.js** | `C:\Users\robin\AppData\Local\hermes\tools\node-26.7.0-win32-x64\node.exe` | v26.7.0 | `process.version` |
| **pnpm** | `C:\Users\robin\AppData\Local\pnpm\package-manager-store\v11\links\@pnpm\exe\11.25.0\...\bin\pnpm.cmd` | 11.25.0 | Pinned version check |
| **Git** | `git.exe` on system PATH | 2.53.0.windows.2 | `git --version` |
| **Rustc** | `C:\Users\robin\.cargo\bin\rustc.exe` | 1.99.1 (x86_64-pc-windows-msvc) | `rustc -vV` |
| **Cargo** | `C:\Users\robin\.cargo\bin\cargo.exe` | 1.99.1 (5f94df478) | `cargo --version` |
| **MSVC C++** | Visual Studio Build Tools | VC Tools x86/x64 | `vswhere.exe` |
| **Windows SDK** | `C:\Program Files (x86)\Windows Kits\10\` | 10.0.26100.0 | `Windows.h`, `kernel32.lib` |
| **WebView2** | System Evergreen Runtime | 155.0 / 156.0 | Registry Application inspect |
| **makensis** | `C:\Users\robin\AppData\Local\tauri\NSIS\makensis.exe` | v3.13 | NSIS CLI inspect |

---

## 3. Verified Artifact Manifest

Artifacts are stored in `E:\Anansi\artifacts\antigravity-release\`:

| Category | Relative File Path | Size (Bytes) | Human-Readable Size | SHA-256 Checksum |
|---|---|---|---|---|
| **Executable** | `anansi-desktop.exe` | 18,015,232 | 17.18 MiB | `504efea4a038fd6d377ce99f76417ca369ee16eb48ac76520d9088f3a6993a06` |
| **Web Asset** | `web/index.html` | 397 | 397 B | `28dad93f2f627ca5c7bacb0479b7e1ca1ea3490e3aa13bf344cac993513cb1b2` |
| **Web Asset** | `web/assets/index-BwbRqKgE.css` | 7,675 | 7.50 KiB | `13c28e99375dc0257f885bf549364f36f05448057172a193ca80682178cbe6fc` |
| **Web Asset** | `web/assets/index-CM3JF7Jy.js` | 1,545,492 | 1.47 MiB | `28de71c6a412eb31ed059f258a428af08367c08ad16de12ada062f48e287faac` |

Manifest: `release-manifest.json`  
Checksums: `SHA256SUMS`

---

## 4. Offline Manifest Verification Verdict

Executed command:
```powershell
node scripts/windows-release/build-verify.mjs verify --manifest E:\Anansi\artifacts\antigravity-release\release-manifest.json
```

**Result**:
- Verified Files: **4 / 4**
- Missing Files: **0**
- Truncated Files: **0**
- Checksum Mismatches: **0**
- Unexpected Extraneous Files: **0**
- **Verdict**: **PASS — All artifacts match recorded SHA-256 and byte length.**

---

## 5. Guarded Isolated Launch Acceptance Evidence

Executed command:
```powershell
node scripts/windows-release/launch-acceptance.mjs
```

**Execution Parameters & Observed Results**:
- Shared Lease Check: Acquired cleanly at `~/.codex/anansi-application-test.lease.json` (ID: `b54920b8-c963-4bc9-afbd-acfdd68b3382`).
- Running Target Check: 0 existing `anansi-desktop.exe` processes running.
- Hash Validation: Computed `504efea4...` matched `release-manifest.json`.
- Synthetic Root: `E:\Anansi\tmp\antigravity-release\acceptance-data-b54920b8-c963-4bc9-afbd-acfdd68b3382`.
- Environment Sanitization: Stripped all `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `TAURI_CONFIG`, and test provider flags.
- Supervised Child Process: PID `86536` spawned and observed for 5000 ms.
- Clean Teardown: Process tree terminated cleanly; zero orphan background daemons left.
- Observed Synthetic Storage Initialization:
  1. `.anansi-root`
  2. `vault-admission`
  3. `workspaces.sqlite3`
- Lease Release: Cleanly unlinked.
- **Verdict**: **PASS — Guarded launch acceptance verified offline without model calls.**

---

## 6. Restart Persistence & Storage Re-admission Evidence

Executed command:
```powershell
node scripts/windows-release/launch-acceptance.mjs --restart --smoke-duration 4000
```

**Execution Parameters & Observed Results**:
- Lease: Acquired cleanly at `~/.codex/anansi-application-test.lease.json` (ID: `05b3e166-e831-4d38-9b21-9384fc5c8201`).
- Synthetic Data Root: `E:\Anansi\tmp\antigravity-release\acceptance-data-05b3e166-e831-4d38-9b21-9384fc5c8201`.
- Phase 1 Initial Launch: Supervised child PID `40504` ran for 4000 ms, creating `.anansi-root`, `vault-admission`, and `workspaces.sqlite3`. Cleanly terminated.
- Cooldown: Process tree terminated cleanly without zombie threads.
- Phase 2 Restart Launch: Supervised child PID `34672` reconnected against the exact same synthetic root for 4000 ms.
- Persistence Assertion: All 3 database/root files persisted intact; database opened cleanly without schema collision or re-initialization error.
- Evidence Written: `E:\Anansi\artifacts\antigravity-release\launch-evidence.json`.
- **Verdict**: **PASS — Application startup and restart persistence verified.**

---

## 7. Multi-Run Repeatability & PE Header Analysis Evidence

Executed command:
```powershell
node scripts/windows-release/repeatability-compare.mjs
```

**Results**:
- Run 1 Artifacts: `E:\Anansi\artifacts\antigravity-release\repeatability-run1`
- Run 2 Artifacts: `E:\Anansi\artifacts\antigravity-release\repeatability-run2`
- **Web Frontend Distribution**: 3 / 3 files (`index.html`, CSS, JS) are **100% bit-for-bit identical** between Run 1 and Run 2.
- **Desktop Executable Size**: Exactly **18,015,232 bytes** across both runs.
- **Section Layout**: 5 of 6 sections (`.text` at 12.78 MiB, `.data` at 25.50 KiB, `.pdata` at 614.50 KiB, `.rsrc` at 1.50 KiB, `.reloc` at 29.50 KiB) are **100% byte-for-byte identical**.
- **PE Header Variance**: Section `.rdata` contains measured differences (0.0001% total binary byte variance) attributable strictly to MSVC `link.exe` `TimeDateStamp` (`1793379344` vs `1793379408`, 64s apart) stored at `peOffset + 8` (with `peOffset` read from DOS header `0x3C`) and the debug directory CodeView GUID.
- Detailed Report: [`repeatability-report.md`](repeatability-report.md).
- **Verdict**: **RECONCILED — Structural and web reproducibility verified; hash-different binaries are honestly documented as NOT bit-identical.**

---

## 8. Tooling Qualification, Reviewer Probes & Stress Battery Evidence

Executed commands:
```powershell
node --test tests/windows-release/reviewer-probes.test.mjs
node --test tests/windows-release/acceptance-criteria.test.mjs
node --test tests/windows-release/qualification-stress.test.mjs
```

**Results**:
- `reviewer-probes.test.mjs`: **2 / 2 PASS**
  - WR-01 Probe: Non-zero early exit (where.exe exit code 2) and absent persistence verified to fail launch qualification (`ok: false`).
  - WR-02 Probe: Support diagnostic export verified to strictly omit arbitrary unallowlisted log text.
- `acceptance-criteria.test.mjs`: **15 / 15 PASS**
  - AC-1.1: Preflight fails closed on low RAM, C: free space, and E: free space.
  - AC-1.3: Preflight warns on active test lease; fails closed on running target executables.
  - AC-1.4: Read-only disk inventory proposal matches expected safety schema.
  - AC-2.3: Offline artifact verifier catches missing, truncated, bit-flipped, and unexpected files.
  - AC-3.1: Guarded launch acceptance fails closed on lease collision, hash mismatch, and C: drive escape.
  - AC-4.1 / AC-4.3: Support diagnostics strictly redacts secrets, passwords, tokens, and excludes private state.
- `qualification-stress.test.mjs`: **11 / 11 PASS**
  - AC-7.1: Preflight fails closed on unsupported Node (< 22.13), pnpm mismatch, missing rustc/cargo.
  - AC-7.2: Data root rejects repo overlap, NTFS junctions/symlinks, verifies spaces & Unicode, handles read-only destinations cleanly.
  - AC-7.3: Exhaustive redaction across compound text with multiple canary categories.
  - AC-7.4: Failure root preservation isolates crashed or failed runs in `failed-runs/failed-<id>`.
  - AC-7.6: Launch qualification fails closed on early non-zero exit and readiness failure.
  - AC-7.7: Diagnostic export allowlist strictly omits arbitrary log content and free-form errors.
- Total Automated Test Coverage: **28 tests passed, 0 failures**.

---

## 9. Privacy-Safe Support Diagnostics Verification

Executed command:
```powershell
node scripts/windows-release/support-diagnostics.mjs --export
```

**Export Metadata**:
- Location: `E:\Anansi\artifacts\antigravity-release\diagnostics\anansi-support-diagnostics-*.json`
- Privacy Verification:
  - Synthetic Canary Redaction Battery: **PASS (100% redacted)**.
  - Bounded Allowlist: Strict structured numeric and boolean fields only.
  - Raw Build Logs / Console Text: **Zero exported by design**.
  - User Paths: Sanitized to `<USERPROFILE>`.
  - Sensitive Environment Variables: Zero dumps.
  - Memory Bodies / Vault Databases: Zero included.
