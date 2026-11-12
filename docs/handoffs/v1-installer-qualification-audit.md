# v1 installer qualification — release-script audit

Audited 2026-11-12 against the accepted candidate
(runtime `8931d34b`, reviewer `57029b2e`). Every
finding below was read in the script source; file
references are to this baseline. The existing
`scripts/windows-release/` suite (preflight,
build-verify, launch-acceptance, verify-delivery,
repeatability-compare, support-diagnostics, common)
is the established qualification system and is
reused, not replaced. The `windows-release` tests
import only `verifyManifest` from `build-verify.mjs`
and the launch-acceptance functions; neither
signature changes here.

## Findings

### F1 — Native candidate script hardcodes a stale candidate

`scripts/native-candidate-acceptance.mjs` hardcodes
the `7e24859e6` candidate: binary, storage root, and
WebView2 user-data folder under
`E:/Anansi/previews/native-candidate-7e24859e6/`.
It cannot run against the v1 RC without editing the
script (the previous handoff told the reviewer to
edit the three constants). It has no platform guard,
no CDP-port check (it could attach to a wrong
process already serving port 19323), no running-
process check, no lease check, and no owner-data
refusal — `mkdirSync(recursive)` will use any path.
It prints no binary hash. Its
`ANANSI_VAULT_TEST_PROVIDER=1` setting is a harness
substitute, not credential-store evidence.
Fix: AC-Q1 (new `candidate-locations.mjs` + script
parameterization, reusing `common.mjs` helpers).

### F2 — Native verifiers use the default WebView2 profile

`scripts/verify-desktop.mjs` and
`scripts/verify-workspace.mjs` set
`ANANSI_STORAGE_ROOT` under isolated temp roots but
never set `WEBVIEW2_USER_DATA_FOLDER`, so WebView2
uses its default profile location (shared with the
running preview / owner profile). `verify-desktop.mjs`
also lacks the port check `verify-workspace.mjs`
already has (`assertPortsFree`). Fix: AC-Q2.
`launch-acceptance.mjs` (`spawnAndTrack`) likewise
leaves the UDF at default, but its
`RUNNING_EXECUTABLE` refusal already blocks any run
while an `anansi-desktop.exe` is active, which covers
the conflict case; it is documented, not changed.

### F3 — Lease identity does not reach wrapped children

`scripts/with-application-test-lease.mjs` writes the
lease record (identity, owner, PID) but does not
export the identity to the child, so a lease-aware
check inside a wrapped script cannot distinguish its
own lease from another holder's. Fix: AC-Q3 (export
`ANANSI_APPLICATION_TEST_LEASE`). Acquisition,
refusal, and release semantics are untouched.

### F4 — Build identity lacks toolchain versions

`packaging.json` (`installer-checksums.mjs`) records
commit, dirty state, installer/application/config
hashes, and unsigned kind — but no toolchain
versions. `release-manifest.json`
(`build-verify.mjs`) records commit, dirty state,
paths, artifact hashes, and unsigned status — but
no toolchain versions either. `preflight.mjs`
already collects the full toolchain (and
`doctor.mjs` exports `collect()`/`evaluate()`), so
the data exists; it is just not attached to the
build artifacts. Fix: AC-Q4 (shared version-only
`toolchainIdentity()` in `doctor.mjs`, used by both
manifest writers; no paths or usernames recorded).

### F5 — build-verify hardcodes the installer identity

`build-verify.mjs` hardcodes the installer filename
`ANANSI Ultimate_0.1.0_x64-setup.exe` and manifest
`version: '0.1.0'` instead of reading
`tauri.conf.json` (which `installer-checksums.mjs`
reads correctly). Worse, a missing installer file
silently yields `installerArtifact: null` and a
successful build result — a version bump would
produce a manifest without the installer and the
failure would surface later, confusingly, in
`verify-installer.ps1`. Fix: AC-Q4 (read the config;
fail closed when the installer was requested but is
missing).

### F6 — Installer verification root contradicts the handoff

`scripts/verify-installer.ps1` installs into a GUID
root under `%TEMP%` (typically `C:`), while the v1
handoff §2 says builds and temp files stay on `E:`.
There is no option to place the verification root
under `E:\Anansi`. Its verification record also
lacks build identity. All other guards are strong
and are preserved: existing-registration refusal,
`NotSigned` expectation, installer/installed-binary
hash checks, `.git`/reparse-point ancestor walks,
data-preservation comparison, remembered-location
cleanup. Fix: AC-Q5 (optional `-TestRoot` with the
guard set generalized; identity fields in the
record; default path byte-identical in behavior).

### F7 — desktop.mjs fails obscurely outside pnpm

`scripts/desktop.mjs` spawns `process.env.npm_execpath`
without checking it; run as plain
`node scripts/desktop.mjs installer` it fails
obscurely. The documented command is
`pnpm build:installer`, but the failure mode hides
that. Fix: AC-Q4 (clear error naming the pnpm
command). The delegated commands themselves
(`tauri build --no-bundle`, `tauri build --bundles
nsis --ci --no-sign`) are correct and pinned by
`tests/installer.test.mjs`; they are unchanged.

### F8 — Handoff omits the primary isolated build path

The v1 handoff §3 documents only
`pnpm build:installer` (in-repo `target/` build). The
established isolated build is
`node scripts/windows-release/build-verify.mjs build
--installer`, which isolates `CARGO_TARGET_DIR`,
`TEMP`/`TMP`, and artifacts on `E:` and writes
`release-manifest.json`. The handoff also tells the
reviewer to edit the candidate script's constants
(superseded by AC-Q1) and does not state the full
identity tuple or the single-build
`CARGO_TARGET_DIR` warning (never share one target
dir across concurrent builds/worktrees — fingerprint
collision). Fix: AC-Q7 (handoff revision).

### F9 — Reviewer friction: inexact Vite invocation

The Windows review records one failed command that
pointed Vite at the root `node_modules` directory
(`MODULE_NOT_FOUND`), corrected to the existing
`apps/web` dependency directory. `verify-workspace.mjs`
already uses the correct `apps/web/node_modules/vite`
path. The checklist (AC-Q7) therefore gives exact,
copy-pasteable commands and names the working
directory for each step.

### F10 — No deterministic fixture set for manual checks

TXT/MD fixtures do not exist under `tests/fixtures/`
(PDF and DOCX fixtures exist with hash manifests).
The manual picker checks (formats, Unicode paths,
unsupported, oversized) have no reproducible inputs.
Fix: AC-Q6 (generator reusing the existing PDF/DOCX
fixtures byte-identical, generating the rest
deterministically, with a hash manifest and a
`--verify` determinism mode).

## Out of scope (documented, untouched)

- Historical M7 session scripts (`m7-ui-acceptance-
  session.mjs`, `m7-ui-reopen-session.mjs`,
  `verify-m7-native-export.mjs`,
  `m7-s23-native-session.mjs`) hardcode `E:/Anansi`
  paths. They are one-off acceptance evidence, not
  the qualification path.
- `preflight.mjs --inventory` hardcodes host-specific
  owner paths. Read-only; preserves the running
  preview and user data by design.
- `launch-acceptance.mjs`, `verify-delivery.mjs`,
  `repeatability-compare.mjs`,
  `support-diagnostics.mjs`, `common.mjs`,
  `preflight.mjs`, `installer-artifact.mjs`,
  `verify-native.ps1`, `install-msvc.ps1`,
  `build-desktop-test.mjs`: no changes. Their
  behavior is reused as-is.
- `tauri.conf.json`, contracts, UI, Rust code, e2e
  specs: no changes. The product is untouched.

## What can run on Linux (AC-Q8 classification)

Runs on Linux (executed for this checkpoint): docs
validation; the node suite including the new
location/toolchain/fixture tests; fixture generation
+ `--verify` determinism; `node --check` on edited
`.mjs` files. The new location/toolchain logic is
pure and dependency-injected, so Windows-only rules
(the `E:` requirement, default-root paths) are
tested on Linux with injected fakes.

Cannot run on Linux (Windows-verified-only, recorded
as known blockers): the PowerShell `-TestRoot` path;
`installer-checksums.mjs` and `build-verify.mjs`
build paths (win32-only, spawn Tauri/cargo);
`native-candidate-acceptance.mjs`,
`verify-desktop.mjs`, `verify-workspace.mjs`
(Windows binaries + WebView2/CDP); all MANUAL/NATIVE
checklist items (CredUI, file dialogs, SmartScreen,
install/upgrade/uninstall). No Linux result is
presented as evidence for any of these.

## Ownership note

Release scripts are Antigravity-owned. The edits in
this checkpoint are minimal and reuse-first; the
handoff flags them for reviewer acceptance. If the
reviewer prefers different env-var names or
locations, the checklist's commands change but the
guard semantics in AC-Q1..Q5 stand.
