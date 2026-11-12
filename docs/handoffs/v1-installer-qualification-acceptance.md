# v1 installer qualification — acceptance criteria (before implementation)

Branch: `muse/v1-pilot-installer-qualification`, from Windows
reviewer tip `57029b2eb2acfa47b7d180591b9b73fd3c04ec25`
(focused Windows browser journey PASS on runtime
`8931d34b06faf16806a089a587ebd58e6b26f9a5`; installer/
native outstanding). No new product features. The
accepted journey, the 416/416 recovery aggregate
(scoped to `8453f649`), and all standing gates are
preserved; the recovery repair loop is not restarted.

The audit behind these criteria is
`docs/handoffs/v1-installer-qualification-audit.md`.
Release scripts are Antigravity-owned; changes here
are minimal, reuse-first, and flagged for reviewer
acceptance in the handoff. No parallel release system
is created.

## AC-Q1 — Candidate locations are explicit and guarded

- `scripts/native-candidate-acceptance.mjs` takes the
  executable, storage root, and WebView2 user-data
  folder ONLY from explicit environment variables
  (`ANANSI_CANDIDATE_BINARY`,
  `ANANSI_CANDIDATE_STORAGE_ROOT`,
  `ANANSI_CANDIDATE_WEBVIEW_UDF`). No defaults, no
  hardcoded candidate paths; the stale `7e24859e6`
  identity is gone.
- Resolution refuses: missing/relative paths; a
  missing binary; a non-empty storage/UDF directory;
  storage/UDF inside the repository, inside the host
  default storage root (`%LOCALAPPDATA%\ANANSI\
  workspaces` / `~/.local/share/anansi/workspaces`),
  or on a path containing `anansi-preview`; on
  Windows, any location outside `E:`; reparse points
  in existing ancestors; storage equal to UDF.
- Before spawning, the script refuses: an occupied
  CDP port, any running `anansi-desktop.exe` /
  `anansi-api.exe`, and an occupied application-test
  lease held by another identity (its own lease via
  the wrapper is allowed). It refuses to run off
  Windows.
- It prints the binary SHA-256 + size, and refuses on
  mismatch when `ANANSI_CANDIDATE_EXPECTED_SHA256`
  is set.
- All 7 existing checks and their assertions are
  preserved byte-for-byte in behavior. The
  `ANANSI_VAULT_TEST_PROVIDER=1` harness setting is
  kept and labeled harness-only: it never evidences
  credential-store acceptance.

## AC-Q2 — Existing native verifiers isolate WebView2 state

- `scripts/verify-desktop.mjs` and
  `scripts/verify-workspace.mjs` set
  `WEBVIEW2_USER_DATA_FOLDER` inside their own
  isolated temp roots (no more default-profile use).
- `verify-desktop.mjs` asserts its CDP port free
  before spawning (same helper `verify-workspace.mjs`
  already uses). No other behavior changes.

## AC-Q3 — Lease identity propagates to wrapped children

- `scripts/with-application-test-lease.mjs` exports
  the lease identity to the child environment, so a
  lease-aware check can tell its own lease from
  another holder's. Lease acquisition, refusal, and
  release semantics are unchanged; nothing is ever
  stolen by age/PID.

## AC-Q4 — Build identity is complete and version-honest

- `packaging.json` (via `installer-checksums.mjs`)
  and `release-manifest.json` (via `build-verify.mjs`)
  both record a version-only `toolchain` block: Node,
  pnpm, Git, rustc (+host), Cargo, Windows SDK
  version, WebView2 versions. No filesystem paths,
  no usernames.
- `build-verify.mjs` reads the installer filename
  and manifest version from `tauri.conf.json`
  (no hardcoded `0.1.0`), and fails closed when an
  installer build was requested but the installer
  artifact is missing (no silent null manifest).
- `desktop.mjs` fails with a clear message when run
  outside pnpm (`npm_execpath` missing) instead of
  spawning undefined.
- The handoff states the full identity tuple
  (source commit, clean/dirty, installer + application
  hashes, config hash, toolchain, unsigned status)
  and never presents the generic `0.1.0` version
  alone as sufficient build identification.

## AC-Q5 — Installer verification accepts an explicit test root

- `scripts/verify-installer.ps1` accepts an optional
  `-TestRoot`. Without it, behavior is unchanged
  (GUID `%TEMP%` root). With it, the same guard set
  applies generalized: absolute, non-existent,
  existing parent, no `.git` ancestry, no reparse
  points, outside the repository, outside owner-data
  roots (`$env:LOCALAPPDATA\ANANSI*`).
- Its verification record gains the build identity
  from `packaging.json` (commit, dirty state,
  version, toolchain, unbundled hash). All existing
  safety checks are preserved.
- PowerShell changes cannot execute on Linux; they
  are minimal, mirror existing constructs, and are
  recorded as Windows-verified-only.

## AC-Q6 — Deterministic synthetic fixtures

- `scripts/qualification-fixtures.mjs` generates a
  deterministic fixture set into an explicit target
  directory (refusing non-empty targets, repository
  overlap, default storage roots, and preview
  paths): reused PDF/DOCX fixtures (copied
  byte-identical), generated TXT/MD with fixed
  synthetic content, a Unicode-named file in a
  Unicode-named directory, an oversized TXT
  (70,000 bytes), an unsupported `.exe`, and an
  empty file. It writes a sorted `MANIFEST.json`
  with SHA-256, byte counts, and expected outcomes.
- A `--verify` mode proves determinism: regenerate
  elsewhere and compare manifests. Generation and
  verification run on Linux.

## AC-Q7 — Checklist separates automated from manual/native

- `docs/handoffs/v1-pilot-installer-checklist.md`
  covers, per item with exact commands, expected
  results, and evidence to record: preflight, build,
  manifest verification, fresh install + first
  launch, native credential prompt (cancel, storage,
  restart, removal), native document picker (all
  four formats, cancel, Unicode paths, unsupported,
  oversized), concurrent imports, upgrade preserving
  data, restart/backup/fresh restore, uninstall
  preserving storage.
- Every item is labeled AUTOMATED (with the exact
  command) or MANUAL/NATIVE (human-observed, never
  a browser-test substitute). No item claims native,
  SmartScreen, installer, or credential-store
  acceptance from mocks or browser tests.
- The Windows handoff is updated consistently
  (build-verify as the primary isolated build,
  `-TestRoot`, env-var candidate script, identity
  tuple, single-build `CARGO_TARGET_DIR` warning,
  log-redaction reminder).

## AC-Q8 — Verification and delivery

- New unit tests cover the location resolution, the
  host-free guards, the toolchain extraction, and
  fixture determinism; all run on Linux via
  `node --test` and are wired into `pnpm test`.
  Existing assertions are preserved; no test is
  weakened.
- Linux regressions run for the changed surface:
  docs validation, the node suite, fixture
  generation + verification. Rust/UI/e2e suites are
  NOT repeated: no product code, contracts, UI, or
  specs change, and the Windows journey already
  passed 1/1 on this exact runtime. Reasons are
  recorded.
- Delivery: clean branch, small reviewable commits,
  full-history bundle ZIP + MANIFEST + SHA-256
  sidecar, independent extraction/hash/bundle/fetch
  verification, credentials sweep, raw logs, known
  blockers, and a concise Windows reviewer task list
  with executable commands.
- Boundaries: no cloud/model calls, credentials,
  generated-code execution, owner-data changes,
  installer promotion, publication, pushes, or
  remote CI. OAuth stays disabled/deferred; 2.0 work
  stays planning-only. Antigravity's worktree is
  untouched. Stop after delivery.
