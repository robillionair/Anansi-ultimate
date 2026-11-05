# ANANSI Windows Release Engineering — Operator Guide

**Version**: 1.0  
**Target Environment**: Windows 11 / Windows 10 x64  
**Tooling Location**: `scripts/windows-release/`  
**Test Suite**: `tests/windows-release/`  
**Isolated Storage**:  
- Build Cache: `E:\Anansi\build-cache\antigravity-release`  
- Temporary Root: `E:\Anansi\tmp\antigravity-release`  
- Artifacts & Manifests: `E:\Anansi\artifacts\antigravity-release`  

---

## 1. Quick Start Workflow

Operators can execute the complete release-engineering pipeline sequentially:

```powershell
# 1. Run system & prerequisite preflight
node scripts/windows-release/preflight.mjs

# 2. Inspect disk accounting and cleanup proposal (read-only)
node scripts/windows-release/preflight.mjs --inventory

# 3. Build standalone application or installer into isolated E: storage
node scripts/windows-release/build-verify.mjs build --standalone-only

# 4. Verify generated artifacts offline (zero process execution)
node scripts/windows-release/build-verify.mjs verify --manifest E:\Anansi\artifacts\antigravity-release\release-manifest.json

# 5. Run guarded isolated launch smoke test
node scripts/windows-release/launch-acceptance.mjs

# 6. Preview or export privacy-safe support diagnostics
node scripts/windows-release/support-diagnostics.mjs --preview
node scripts/windows-release/support-diagnostics.mjs --export

# 7. Execute release engineering regression test suite
node --test tests/windows-release/acceptance-criteria.test.mjs
```

---

## 2. Command Reference

### 2.1 Preflight & Disk Accounting (`preflight.mjs`)
- **System Preflight**:
  ```powershell
  node scripts/windows-release/preflight.mjs
  ```
  Checks RAM (>= 4 GiB), free disk on C: (>= 5 GiB) and E: (>= 20 GiB), toolchains (Node >= 22.13, pinned pnpm 11.25.0, Git, Rust MSVC, Cargo, C++ build tools, Windows SDK, WebView2, NSIS), active processes locking executables, and the shared test lease.
  *Exit Code*: `0` if ready, `1` if blocked by missing prerequisites or insufficient resources.

- **JSON Output**:
  ```powershell
  node scripts/windows-release/preflight.mjs --json
  ```

- **Read-Only Disk Accounting & Cleanup Proposal**:
  ```powershell
  node scripts/windows-release/preflight.mjs --inventory
  ```
  Surveys target directories, dependencies, Git history, and user data across C: and E:. Classifies folders as rebuildable or recovery data, and writes `docs/reviews/windows-release/cleanup-proposal.json`. Performs **zero** automatic deletions.

---

### 2.2 Reproducible Build & Verification (`build-verify.mjs`)
- **Standalone Build (Vite Web + Tauri Binary)**:
  ```powershell
  node scripts/windows-release/build-verify.mjs build --standalone-only
  ```
  Executes Vite web build and Tauri desktop build strictly routing outputs to `E:\Anansi\build-cache\antigravity-release`. Copies artifacts to `E:\Anansi\artifacts\antigravity-release`, computes SHA-256 sums, and writes `release-manifest.json` and `SHA256SUMS`.

- **Installer Build (Standalone + NSIS Setup Bundle)**:
  ```powershell
  node scripts/windows-release/build-verify.mjs build --installer
  ```
  Compiles desktop application and bundles NSIS installer using `tauri build --bundles nsis --ci --no-sign`. Computes payload hash per Tauri bundle marker conventions.

- **Offline Manifest Verification**:
  ```powershell
  node scripts/windows-release/build-verify.mjs verify --manifest <path>
  ```
  Validates every file in the manifest strictly offline. Detects missing files, altered sizes, checksum mismatches, and unexpected files. Does **not** launch or spawn any binary.

---

### 2.3 Guarded Launch Acceptance (`launch-acceptance.mjs`)
```powershell
node scripts/windows-release/launch-acceptance.mjs
```
Guarded smoke runner that:
1. Acquires the shared application-test lease (`~/.codex/anansi-application-test.lease.json`); fails gracefully if occupied.
2. Refuses launch if `anansi-desktop.exe` is already running on the host.
3. Validates executable SHA-256 against `release-manifest.json`.
4. Provisions an isolated synthetic root on `E:\Anansi\tmp\antigravity-release\acceptance-data-<uuid>`.
5. Scrubs test credentials, model tokens, and debug overrides from the environment.
6. Spawns binary, observes basic smoke startup, and cleanly terminates the process tree.
7. Releases lease and records `launch-evidence.json`.

---

### 2.4 Privacy-Safe Support Diagnostics (`support-diagnostics.mjs`)
- **Preview Diagnostics (Dry Run)**:
  ```powershell
  node scripts/windows-release/support-diagnostics.mjs --preview
  ```
  Displays the allowlisted diagnostic object (build SHA, OS info, toolchain versions, verification summary, sanitized logs) to stdout. Confirms that no private data is collected.

- **Export Diagnostics**:
  ```powershell
  node scripts/windows-release/support-diagnostics.mjs --export
  ```
  Writes a timestamped JSON diagnostic bundle to `E:\Anansi\artifacts\antigravity-release\diagnostics\`.

- **Test Secret Canary Redaction**:
  ```powershell
  node scripts/windows-release/support-diagnostics.mjs --test-canary
  ```
  Validates that API keys (`sk-...`), Bearer headers, passwords, and custom canary tokens are 100% stripped and replaced with redaction markers.

---

## 3. Operational Safety Policies

1. **Drive Isolation**: Never change `CARGO_TARGET_DIR` or `TEMP` to C:. Keep all build caches and temporary fixtures strictly under `E:\Anansi`.
2. **Preview Preservation**: Do not terminate or alter the running preview in `C:\Users\robin\Documents\ChatGPT\Anansi-m5-review` or user preview data in `%LOCALAPPDATA%\ANANSI-Preview\r11`.
3. **Lease Rules**: Never delete or bypass the application-test lease when occupied by another agent or PID. Wait for the leaseholder to release it or conclude independent tasks.
4. **Installer Testing**: Do not execute the NSIS installer on the host user profile. Follow `docs/reviews/windows-release/windows-sandbox-installer-procedure.md` in a disposable Windows Sandbox.
