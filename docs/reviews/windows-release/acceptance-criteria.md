# Windows Release Engineering — Acceptance Criteria

**Milestone / Component**: Windows Release Engineering, Verification, and Support Tooling  
**Branch**: `codex/antigravity-release-engineering`  
**Worktree**: `E:\Anansi\worktrees\antigravity-release-engineering`  
**Baseline Checkpoint**: `240af4ef58b09e7ada405286f46f927c902f9c52`  
**Incoming Checkpoint**: `edba7ed32db2f7c5ea999466061c4e1c3966b2e7`  
**Date**: October 30, 2026  
**Status**: ACTIVE ACCEPTANCE SPECIFICATION (DEFINED PRIOR TO IMPLEMENTATION)

---

## 1. Scope, Responsibilities, and Non-Negotiable Boundaries

### 1.1 Parallel Ownership
- **Muse Owns**: Application domain code (`crates/core`, `crates/api`, `apps/desktop/src`, `apps/web/src`, `packages/ui`), database migrations, restore logic, contracts, and resolving the three open M8 overnight review findings (R1 forged restored-hash provenance, R2 post-fixup stale context hash, R3 saved/live anchor collision).
- **Release Engineering Owns**:
  - Windows build/package verification tooling (`scripts/windows-release/`).
  - Isolated launch acceptance tooling and verification runners.
  - Privacy-safe support diagnostics with secret-canary redaction.
  - Build/cache disk accounting, preflight checks, and read-only inventory with machine-readable cleanup proposals.
  - Release-engineering evidence, artifact manifests, and test suites (`tests/windows-release/`).
- **Exclusive Write Scope**: Files are added strictly under:
  - `scripts/windows-release/`
  - `tests/windows-release/`
  - `docs/reviews/windows-release/`
- **Strictly Untouched**: No modification to application code, package manifests (`package.json`, `Cargo.toml`), lockfiles, existing launchers, shared docs (`BUILD_STATUS.md`, `ROADMAP.md`), or GitHub workflows.

### 1.2 Isolation & Resource Protection
- **Drive Isolation**: All build caches, temporary workspaces, synthetic test vaults, and release artifacts must reside on `E:\Anansi`:
  - Target cache: `E:\Anansi\build-cache\antigravity-release`
  - Temp directory: `E:\Anansi\tmp\antigravity-release`
  - Artifacts: `E:\Anansi\artifacts\antigravity-release`
- **Zero C: Growth**: No build outputs, test caches, or generated files may be written to the `C:` drive.
- **Process Isolation & Safety**:
  - Never terminate, replace, or disturb the user's running previews or personal data (`C:\Users\robin\AppData\Local\ANANSI-Preview\r11`).
  - Respect the repository's shared test lease (`~/.codex/anansi-application-test.lease.json`). If occupied, fail closed or continue independent non-interfering tasks without stealing or expiring the lease.
  - Installer install/uninstall acceptance must only run in a disposable Windows VM / Sandbox. If no disposable environment is available, statically verify the installer artifact, provide an executable procedure, and mark installation unverified. Never install/uninstall on the host Windows user profile.

---

## 2. Package 1: Resource & Disk Preflight

### AC-1.1: System Resource Discovery
- Preflight command (`node scripts/windows-release/preflight.mjs`) must report:
  - Available and total RAM on the host.
  - Available and total storage on drives `C:` and `E:`.
  - Fail-closed thresholds: minimum 4.0 GiB available RAM, minimum 5.0 GiB free on `C:`, and minimum 20.0 GiB free on `E:`. If below threshold, abort with an actionable error before invoking compiler or bundler.

### AC-1.2: Toolchain & Dependency Verification
- Discover and verify exact paths and versions for:
  - Git (`git --version`)
  - Node.js (>= 22.13.0)
  - Pinned `pnpm` (`11.25.0`) with search fallback into local store
  - Rustc and Cargo (`host: x86_64-pc-windows-msvc`)
  - Visual Studio C++ Build Tools (`vswhere.exe` VC tools)
  - Windows 10/11 SDK headers (`Windows.h`, `stdio.h`) and libraries (`kernel32.lib`, `ucrt.lib`)
  - WebView2 runtime installation
  - NSIS / `makensis.exe` (Tauri bundled NSIS in `%LOCALAPPDATA%\tauri\NSIS` or PATH)

### AC-1.3: Active Process & Lease Collision Detection
- Check whether any active processes are running target executables (`anansi-desktop.exe`, `anansi-api.exe`) that would lock files or prevent clean rebuilds.
- Inspect `~/.codex/anansi-application-test.lease.json` without modifying or acquiring it:
  - If present and valid, report `leaseOccupied: true` and identify the leaseholder.
  - Preflight must report lease status clearly to callers.

### AC-1.4: Read-Only Disk Inventory & Cleanup Proposal
- Inspect disk usage across ANANSI checkouts without modifying, moving, or deleting any directory:
  - Distinguish:
    1. Source (Git-tracked workspace code)
    2. Git objects (`.git/objects`)
    3. Dependencies (`node_modules`)
    4. Build caches (`target/`, `build-cache/`)
    5. Application data / test fixtures (`%LOCALAPPDATA%\ANANSI*`, temp fixtures)
  - Identify duplicated `target` and build cache directories across `C:\Users\robin\Documents\ChatGPT\Anansi*` and `E:\Anansi\build-cache\*`.
  - Explicitly categorize each entry as `rebuildable: true` or `recoveryData: true` (e.g., untracked notes, user previews, backups).
  - Produce a machine-readable JSON proposal (`docs/reviews/windows-release/cleanup-proposal.json`) with exact paths, human-readable sizes, and safety classifications.

---

## 3. Package 2: Reproducible Build & Offline Artifact Verification

### AC-2.1: Build Wrapper & Execution Guard
- Build wrapper (`node scripts/windows-release/build-verify.mjs build`) must wrap supported build steps:
  1. `apps/web` Vite production build (`pnpm --filter @anansi/web build`)
  2. `apps/desktop` Tauri build (`tauri build --no-bundle` or `tauri build --bundles nsis --ci --no-sign`)
  3. Installer checksum generation
- Pre-execution checks:
  - Record exact Git commit SHA (`git rev-parse HEAD`) and dirty status (`git status --porcelain`).
  - Enforce `CARGO_TARGET_DIR=E:\Anansi\build-cache\antigravity-release` and `TEMP`/`TMP=E:\Anansi\tmp\antigravity-release`.
  - Capture child process stdout/stderr into an execution log on `E:`.
  - Preserve native process exit codes; stop immediately on any step failure and propagate the exit code.

### AC-2.2: Comprehensive Manifest Generation
- After build completion, inspect all generated artifacts and write `release-manifest.json`:
  - Categories:
    - `executable`: `anansi-desktop.exe`
    - `installer`: `ANANSI Ultimate_0.1.0_x64-setup.exe` (if installer requested)
    - `web`: `apps/web/dist/**` assets
    - `metadata`: `packaging.json`, `SHA256SUMS`, licenses
  - For every file: compute exact SHA-256 and byte length.
  - Distinguish bundled NSIS payload hash from unbundled application hash per Tauri `__TAURI_BUNDLE_TYPE_VAR_NSS` conventions.
  - Record build metadata: commit SHA, toolchain versions, start time, end time, duration, host platform.

### AC-2.3: Offline Manifest Verifier
- Verifier command (`node scripts/windows-release/build-verify.mjs verify --manifest <path>`) must verify artifacts strictly offline:
  - Check every file listed in the manifest exists, has matching byte length, and matching SHA-256 hash.
  - Detect missing files, modified bytes, truncated files, and unexpected/extra files in output directories.
  - Never execute or spawn the binary during verification.
  - Report structured verification verdict (`PASS` / `FAIL`) with exact discrepancy details.

---

## 4. Package 3: Guarded Isolated Launch Acceptance

### AC-3.1: Guarded Execution Preconditions
- Acceptance launcher (`node scripts/windows-release/launch-acceptance.mjs`) must satisfy:
  - Check and acquire the shared application-test lease (`with-application-test-lease.mjs`); fail gracefully if occupied without stealing.
  - Refuse to run if an existing `anansi-desktop.exe` is already running on the host.
  - Verify that the executable matches the expected SHA-256 hash recorded in the build manifest before launching.
  - Configure isolated data root strictly on `E:\Anansi\tmp\antigravity-release\smoke-data-<uuid>`.
  - Assert that data root does not collide with or touch `%LOCALAPPDATA%\ANANSI-Preview` or any existing workspace.
  - Scrub all environment variables: strip `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `OPENROUTER_API_KEY`, `OLLAMA_HOST`, `TAURI_CONFIG` debug overrides, and any test credentials.

### AC-3.2: Controlled Smoke Lifecycle
- Launch executable with non-interfering test flags:
  - Track the launched child process PID and its process tree.
  - Ensure zero cloud / model network egress.
  - Verify application startup: successful window/process creation or initial socket/IPC readiness.
  - Bounded execution window (timeout limit, e.g. 15 seconds for smoke test).
  - Explicit graceful termination (SIGINT / WM_CLOSE), falling back to forceful process-tree kill if unresponsive.
  - Guarantee that no child processes or background daemons survive the test.

### AC-3.3: Failure Evidence Preservation
- In the event of launch failure or startup timeout:
  - Do not purge the synthetic data root or stderr logs; preserve them in `E:\Anansi\tmp\antigravity-release\failed-runs\<id>`.
  - Record exit code, observed stdout/stderr, and timing in `launch-evidence.json`.

### AC-3.4: Installer Sandbox Verification Policy
- Host environment inspection:
  - Check for Windows Sandbox (`WindowsSandbox.exe`) or disposable VM availability.
  - Since Windows Sandbox is absent on this host, **mark installer installation/uninstallation unverified**.
  - Author an executable procedure (`docs/reviews/windows-release/windows-sandbox-installer-procedure.md`) including WSB configuration and silent install/uninstall verification script.
  - Never execute `ANANSI Ultimate_0.1.0_x64-setup.exe` on the host user profile.

---

## 5. Package 4: Privacy-Safe Support Diagnostics

### AC-4.1: Allowlist-Only Diagnostics Schema
- Support diagnostic command (`node scripts/windows-release/support-diagnostics.mjs`) must collect only allowlisted diagnostic telemetry:
  - Application version, Git commit SHA, dirty-tree status.
  - Host OS version, architecture, RAM/disk availability.
  - Toolchain versions (Node, pnpm, Rust, MSVC, SDK, WebView2).
  - Artifact manifest summary and verification results.
  - Sanitized build/launch exit codes and execution durations.
  - Allowlisted logs (compiler stdout/stderr with strict redaction).

### AC-4.2: Default-Deny Exclusion & Redaction
- Strictly exclude from diagnostic exports:
  - Environment variable dumps (`process.env`).
  - User credentials, API keys (`sk-...`, `Bearer ...`, `token=...`).
  - Vault databases (`*.sqlite3`, `*.sqlite3-wal`), encryption keys, salts, DEKs.
  - Memory bodies, Markdown files, user prompts, generated source code.
  - Usernames and personal file paths (redact `C:\Users\<username>` to `<USERPROFILE>`).

### AC-4.3: Synthetic Secret Canary Battery
- Test redaction engine against synthetic canaries:
  - `CANARY_API_KEY=sk-test-canary-secret-1234567890abcdef`
  - `CANARY_AUTH_HEADER=Authorization: Bearer test-bearer-token-secret-xyz`
  - `CANARY_PATH=C:\Users\robin\Documents\Private\secret-file.txt`
  - `CANARY_PASSWORD=password=SuperSecretPassword123!`
- Assert that every canary pattern in simulated build/launch logs is strictly redacted to `[REDACTED_SECRET]` or sanitized tokens before export.

### AC-4.4: Interactive / Explicit Export & Zero Automatic Egress
- Diagnostics tool must support a preview mode (`--preview`) showing the exact list of files and character preview of contents without writing an archive.
- Export requires explicit flag (`--export --output <path>`).
- Output format: single `.zip` or `.json` bundle located strictly under `E:\Anansi\artifacts\antigravity-release\diagnostics\`.
- Zero automatic cloud upload or network egress.

---

## 6. Package 5: Tests and Verification Suite

### AC-5.1: Negative & Failure Case Coverage
- Test suite (`tests/windows-release/acceptance-criteria.test.mjs`) must test:
  1. Low disk / RAM failure: preflight rejects simulated insufficient space.
  2. Missing prerequisite: preflight rejects missing toolchain entry.
  3. Source identity mismatch: verifier rejects altered Git SHA or unexpected dirty tree.
  4. Build command failure: wrapper propagates non-zero exit code and preserves failure log.
  5. Artifact corruption: offline verifier detects bit flips, missing files, truncated files, and extraneous files.
  6. Occupied lease: launch acceptance refuses to proceed when lease is held.
  7. Running executable: launch acceptance refuses when target process is already active.
  8. Unsafe paths: launcher rejects data root or output paths on `C:` or inside source trees.
  9. Secret canaries: diagnostics exporter redacts all synthetic secret canary forms.
  10. Process cleanup: launcher reliably terminates process tree upon completion or timeout.

### AC-5.2: Real Build & Smoke Verification
- Execute one end-to-end isolated build on `E:\Anansi\build-cache\antigravity-release`.
- Verify generated manifest offline with zero errors.
- Run guarded launch smoke check in isolated synthetic root on `E:`.
- Generate and verify a privacy-safe support diagnostics package.
- Package delivery into Git bundle and checksummed ZIP, verify in clean scratch repository.

---

## 7. Tooling Qualification, Interruption & Stress Testing

### AC-7.1: Delivery Integrity & Assumption Audit
- Verification script (`node scripts/windows-release/verify-delivery.mjs`) must:
  - Extract the delivery ZIP into a clean temporary directory.
  - Verify every file listed in `MANIFEST.txt` against its byte length and SHA-256 hash.
  - Assert the absolute absence of forbidden files: no `node_modules`, no `target/` directories, no `.sqlite*` databases, no model files (`.gguf`, `.bin`, `.onnx`), and no credential/key files (`*.pem`, `*.pfx`, `id_rsa`).
  - Verify the Git bundle ref and fetch the exact tip commit into a scratch git clone referencing the baseline `240af4ef58b09e7ada405286f46f927c902f9c52`.
  - Document all unsatisfied assumptions (e.g. requirement for disposable Windows Sandbox for installer execution; unresolved application-level recovery findings R1-R4).

### AC-7.2: Resilience Under Environmental Failure & Interruption
- Test suite (`tests/windows-release/qualification-stress.test.mjs`) must verify:
  1. **Paths with Spaces & Unicode**: Preflight, build wrapper, and launch acceptance handle paths like `E:\Anansi\tmp\antigravity-release\path with spaces & ⚡\` without shell escaping or quotation errors.
  2. **Reparse Points & Junctions**: Tools detect and refuse to treat symlinks or directory junctions as safe isolated synthetic roots.
  3. **Read-Only / Inaccessible Destinations**: Tools fail cleanly with actionable errors if output destinations are write-protected.
  4. **Interrupted Runs & Atomic Manifest Publication**: The build runner publishes `release-manifest.json` and `SHA256SUMS` strictly upon 100% build success; interrupted or failed builds leave no complete manifest.
  5. **Native Exit Code Propagation**: When child compilers fail, exact non-zero exit codes are preserved and propagated to callers.
  6. **Clean Retry After Failure**: A failed or aborted run leaves no locking processes, and subsequent runs succeed cleanly.

### AC-7.3: Deep Privacy-Safe Support-Diagnostic Qualification
- Diagnostic exporter must withstand a multi-category synthetic canary attack battery:
  - Authorization headers and Bearer tokens (`Authorization: Bearer canary-token-12345`).
  - API keys (`sk-live-0123456789abcdef...`, `ghp_...`, `aws...`).
  - Pairing codes and session tokens (`pairing_code=...`, `session_token=...`).
  - Personal paths and usernames (`C:\Users\robin\...`).
  - Memory, prompt, and database data dumps (`prompt: "secret idea..."`, SQL table dumps).
- Exporter must enforce strict allowlist schema: non-allowlisted keys are dropped, and all text excerpts pass multi-layer redaction.

### AC-7.4: Multi-Run Repeatability & PE Binary Comparison
- Run two consecutive isolated builds from the exact commit into separate output directories:
  - `E:\Anansi\artifacts\antigravity-release\repeatability-run1`
  - `E:\Anansi\artifacts\antigravity-release\repeatability-run2`
- Compare:
  - Git commit identity and dirty status.
  - Dependency and toolchain versions.
  - Generated web asset hashes and sizes.
  - Application executable sizes and byte hashes.
- Document whether PE headers (e.g., compile timestamps, debug directory GUIDs, incremental link data) introduce non-determinism, and report findings honestly without fabricating identical hashes.

### AC-7.5: Real Isolated Startup & Persistence Across Restart
- Launch the exact verified executable into an isolated synthetic root on `E:` under the shared application-test lease.
- Perform a two-phase lifecycle smoke test:
  1. **Phase 1 (Initial Startup)**: Observe clean process startup, verify initialization of `.anansi-root`, `vault-admission`, and `workspaces.sqlite3`, and terminate process tree cleanly.
  2. **Phase 2 (Restart Persistence)**: Spawn the binary against the **same** synthetic root; verify that the application detects existing storage without corruption and starts up cleanly.
- Supervised cleanup must ensure zero lingering `anansi-desktop.exe` processes or child background tasks.

