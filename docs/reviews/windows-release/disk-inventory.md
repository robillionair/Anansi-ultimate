# ANANSI Windows Build & Disk Accounting Report

**Date**: October 30, 2026  
**Component**: Windows Release Engineering Preflight & Disk Accounting  
**Host**: Windows 11 Pro 64-bit (`win32 x64`)  
**Worktree**: `E:\Anansi\worktrees\antigravity-release-engineering`  
**Baseline**: `240af4ef58b09e7ada405286f46f927c902f9c52`  
**Status**: READ-ONLY INVENTORY COMPLETED — CLEANUP PROPOSAL GENERATED

---

## 1. Executive Summary

A comprehensive, non-destructive, read-only disk and system resource audit was conducted across the development environment (`C:` and `E:` drives).

- **Current Free Storage**:
  - `C:` Drive: **33.74 GiB free** (14.2% available of 237.35 GiB total)
  - `E:` Drive: **225.04 GiB free** (32.3% available of 697.54 GiB total)
- **Current Available RAM**:
  - **6.04 GiB free** (38.0% available of 15.89 GiB total)
- **Total Reclaimable Space (Safe to Purge)**: **43.08 GiB** across stale build targets and duplicate dependency caches.
- **Preserved Mandatory Data (Never Purged)**: **49.03 GiB** encompassing the active M5 review preview, primary workspace target, R17 fallback preview binary, Git history/databases, and user preview databases in `%LOCALAPPDATA%\ANANSI-Preview`.

---

## 2. Resource Preflight Thresholds & Status

| Resource / Prerequisite | Minimum Required | Measured Value | Preflight Status | Action / Guidance |
|---|---|---|---|---|
| **Available RAM** | 4.0 GiB | 6.04 GiB | **PASS** | Sufficient headroom for multi-threaded MSVC C++ & Rust compilation. |
| **Free Disk on C:** | 5.0 GiB | 33.74 GiB | **PASS** | Meets safety limit; all release builds strictly routed to `E:`. |
| **Free Disk on E:** | 20.0 GiB | 225.04 GiB | **PASS** | Generous headroom for `build-cache`, temporary roots, and installer artifacts. |
| **Node.js** | >= 22.13.0 | v26.7.0 | **PASS** | Native host Node.js runtime located in Hermes tools. |
| **pnpm** | 11.25.0 | 11.25.0 | **PASS** | Discovered in local package manager store and wrapped without machine mutation. |
| **Git** | Any functional | 2.53.0.windows.2 | **PASS** | Clean Git binary and repository ancestry. |
| **Rust Toolchain** | x86_64-pc-windows-msvc | rustc 1.99.1 | **PASS** | MSVC host toolchain. |
| **Cargo** | 1.99.x | cargo 1.99.1 | **PASS** | Locked build capable. |
| **Visual Studio C++** | VC Tools x86/x64 | Installed | **PASS** | Discovered via `vswhere.exe`. |
| **Windows SDK** | 10.0.x headers/libs | 10.0.26100.0 | **PASS** | `Windows.h`, `kernel32.lib`, `ucrt.lib` verified. |
| **WebView2 Runtime** | Evergreen / Installed | 155.0 & 156.0 | **PASS** | Edge WebView2 client installed. |
| **NSIS / makensis** | v3.x | v3.13 | **PASS** | Tauri NSIS bundled in `%LOCALAPPDATA%\tauri\NSIS\makensis.exe`. |
| **Active Target Exes** | None running | None running | **PASS** | No active locks on `anansi-desktop.exe` or `anansi-api.exe`. |
| **App-Test Lease** | Free / Unoccupied | Free (Unoccupied) | **PASS** | `~/.codex/anansi-application-test.lease.json` is clear. |

---

## 3. Categorized Disk Inventory

### 3.1 Build Caches (`target/` and `build-cache/`)
Build caches represent the largest consumers of disk space. Each independent review worktree created in earlier milestones retained its own full Rust incremental target folder.

| Directory Path | Measured Size | Classification | Recommended Action |
|---|---|---|---|
| `C:\Users\robin\Documents\ChatGPT\Anansi ultimate\target` | 32.72 GiB | Rebuildable | **PRESERVE_PRIMARY** — Primary workspace cache. Run `cargo clean` manually only if C: space is critically constrained. |
| `C:\Users\robin\Documents\ChatGPT\Anansi-m5-review\target` | 9.91 GiB | Active Application | **PRESERVE_MANDATORY** — Contains the running M5 preview binary currently under test by the user. Do not delete or rebuild into. |
| `C:\Users\robin\Documents\ChatGPT\Anansi-preview-r16\target` | 5.39 GiB | Stale Historical Cache | **SAFE_TO_PURGE** — Superseded by R17 and M8 overnight checkpoints. Reclaimable: 5.39 GiB. |
| `C:\Users\robin\Documents\ChatGPT\Anansi-r11-acceptance\target` | 7.13 GiB | Stale Historical Cache | **SAFE_TO_PURGE** — Completed R11 review. Reclaimable: 7.13 GiB. |
| `C:\Users\robin\Documents\ChatGPT\Anansi-r12-acceptance\target` | 6.05 GiB | Stale Historical Cache | **SAFE_TO_PURGE** — Completed R12 review. Reclaimable: 6.05 GiB. |
| `C:\Users\robin\Documents\ChatGPT\Anansi-r16-acceptance\target` | 5.39 GiB | Stale Historical Cache | **SAFE_TO_PURGE** — Completed R16 review. Reclaimable: 5.39 GiB. |
| `E:\Anansi\build-cache\preview-r17` | 5.42 GiB | Fallback Preview Binary | **PRESERVE_PREVIEW_FALLBACK** — Contains `anansi-desktop.exe` launched by `Start ANANSI Preview R17.cmd`. |
| `E:\Anansi\build-cache\preview-m7` | 5.94 GiB | Historical Build Cache | **SAFE_TO_PURGE** — Historical M7 preview build cache on E:. Reclaimable: 5.94 GiB. |
| `E:\Anansi\build-cache\r17-review` | 2.10 GiB | Historical Build Cache | **SAFE_TO_PURGE** — Historical R17 repair cache. Reclaimable: 2.10 GiB. |
| `E:\Anansi\build-cache\windows-acceptance` | 8.86 GiB | Historical Build Cache | **SAFE_TO_PURGE** — Historical windows acceptance cache. Reclaimable: 8.86 GiB. |

### 3.2 Dependencies (`node_modules`)
Every checkout on C: contains a duplicate copy of workspace `node_modules` (~448 MiB each).
- `C:\Users\robin\Documents\ChatGPT\Anansi ultimate\node_modules`: 447.51 MiB (**PRESERVE**)
- `C:\Users\robin\Documents\ChatGPT\Anansi-m5-review\node_modules`: 447.92 MiB (**PRESERVE**)
- `Anansi-preview-r16`, `Anansi-r11-acceptance`, `Anansi-r12-acceptance`, `Anansi-r15-review`, `Anansi-r16-acceptance`: ~2.24 GiB total (**SAFE_TO_PURGE_REINSTALLABLE**)

### 3.3 User & Recovery Data
- `C:\Users\robin\AppData\Local\ANANSI-Preview`: 584.13 KiB. Holds personal preview databases, SQLite schemas, and user memory vectors for R11, R16, and R17. **STRICTLY PRESERVED**.
- `.git` directories in `Anansi ultimate` (5.28 MiB) and `Anansi-m5-review` (96.34 MiB). **STRICTLY PRESERVED**.

---

## 4. Machine-Readable Cleanup Proposal

The exact machine-readable proposal is persisted at:
[`docs/reviews/windows-release/cleanup-proposal.json`](cleanup-proposal.json)

Operators can review and selectively execute this proposal using standard PowerShell or filesystem tools. Release engineering scripts perform **zero automatic deletions or moves**, adhering strictly to the non-destructive safety policy.
