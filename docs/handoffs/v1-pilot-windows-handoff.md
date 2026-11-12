# ANANSI v1 pilot release candidate — Windows release handoff

**Revision 2026-11-12 (installer-qualification
checkpoint):** the Windows browser journey passed
1/1 on this runtime (reviewer `57029b2e`; see the
checklist). This handoff is revised on branch
`muse/v1-pilot-installer-qualification`: §§1–6 now
describe the isolated `build-verify` path, explicit
candidate locations, complete build identity, and
the `-TestRoot` installer option. The per-item
qualification checklist is
`docs/handoffs/v1-pilot-installer-checklist.md`; the
script audit is
`docs/handoffs/v1-installer-qualification-audit.md`.
Product code is unchanged vs the runtime below
(empty diff over `crates/`, `apps/`, `packages/`).

Branch: `muse/v1-pilot-release-candidate`.
Baseline: `dff9bb16688f3af075c395ad8fd5b179af9c3a9d`
(GitHub `origin/master`, 2026-11-12, "docs: plan ANANSI
Corporate for Ultimate 2.0"). Integration merge
`d98a229` brings in the accepted WR-REC repair/review
history through reviewer acceptance `8453f649`
(416/416 focused Windows aggregate).

**Exact source commit:** the tip of this branch — the
commit that contains this file. It is recorded in the
delivery MANIFEST and must be verified after fetching
the bundle:

```powershell
git rev-parse HEAD
```

The fetched tip must equal the MANIFEST tip exactly,
and `git status --porcelain` must be empty before any
build. No other commit is this release candidate.

**Scope of this handoff:** build and acceptance of
the existing product for controlled customer testing.
No new features. No Windows, native, or installer
acceptance is claimed from Linux — every Windows item
below is work for the reviewer. The pilot scope,
including what is NOT ready, is
`docs/handoffs/v1-pilot-scope.md`.

## 1. Toolchain and prerequisites

Verify before building (all commands in the fetched
worktree, PowerShell 7 for the installer scripts):

```powershell
node scripts/doctor.mjs --require-native
node scripts/windows-release/preflight.mjs
```

The doctor checks the toolchain; preflight checks
the whole host (RAM/disk, toolchain incl. makensis,
running target processes, shared-test-lease state)
and exits non-zero unless it prints `VERDICT: PASS`.
A preflight failure blocks the build — reconcile it
first; never steal an occupied lease.

Required:

- Node.js >= 22.13, pnpm exactly **11.25.0**
  (`packageManager` pin; the doctor enforces it).
- Rust **1.99.1** (`rust-toolchain.toml`: minimal
  profile + rustfmt + clippy). Install via rustup;
  the toolchain file pins the channel.
- Visual Studio C++ Build Tools + Windows SDK
  (x64 headers/libraries) + WebView2 installation
  evidence (the doctor's `--require-native` checks).
- Git (bundle fetch + `rev-parse` verification).
- PowerShell 7 (`pwsh`) for `verify-installer.ps1`
  / `verify-native.ps1`.
- NSIS (`makensis.exe`) for installer bundling
  (preflight warns when it is missing; standalone
  desktop builds work without it, installer
  bundling fails).
- A non-elevated host for the installer UI workflow
  (the script refuses elevated runs unless `-SkipUi`
  is explicitly selected).

No paid accounts, no cloud inference, no credentials
are needed to build or to run the Linux-verified
synthetic checks. Any live-provider verification is
a separate, explicitly authorized gate and is NOT
part of this handoff.

## 2. Isolated build / data layout under E:\Anansi

Preserve the running pilot and all existing data.
Do NOT reuse or modify Antigravity's worktree or any
existing preview directory. Use fresh, isolated
locations:

```
E:\Anansi\worktrees\v1-pilot-rc\        (fetched source; build here)
E:\Anansi\previews\v1-pilot-rc\         (candidate test data)
E:\Anansi\previews\v1-pilot-rc\data\    (isolated synthetic storage root)
E:\Anansi\previews\v1-pilot-rc\webview-udf\ (isolated WebView2 user data)
E:\Anansi\qualification\build-cache\    (isolated CARGO_TARGET_DIR)
E:\Anansi\qualification\tmp\            (isolated temp root)
E:\Anansi\qualification\artifacts\      (build outputs + release-manifest.json)
E:\Anansi\qualification\fixtures\       (deterministic fixture set)
E:\Anansi\qualification\installer-test\ (installer verification root)
```

- Fetch the bundle into a FRESH clone/worktree at
  the path above; never fetch into the running
  pilot's tree.
- Builds and temp files stay on E:. The installer
  verification script accepts an explicit `-TestRoot`
  under `E:\Anansi\qualification\`; without it, it
  uses its own GUID temp root under `%TEMP%`.
- One build at a time into a `CARGO_TARGET_DIR`;
  never share one target dir across concurrent
  builds or worktrees (cargo fingerprint collision).
- `ANANSI_STORAGE_ROOT` must point at the isolated
  `data\` directory for every manual run; never at
  a production or shared storage root.
- Synthetic data only for acceptance. Do not open
  real workspaces with the candidate until the
  checklists below pass.

## 3. Reproducible build commands

From the worktree root, in order:

```powershell
# 0. Verify identity and cleanliness
git rev-parse HEAD
git status --porcelain          # must be empty

# 1. Toolchain
node scripts/doctor.mjs --require-native

# 2. Dependencies (frozen lockfile)
pnpm install --frozen-lockfile

# 3. Isolated release build (web + API + desktop + NSIS installer)
node scripts/windows-release/build-verify.mjs build --installer --cargo-target-dir E:\Anansi\qualification\build-cache --artifacts-dir E:\Anansi\qualification\artifacts --temp-dir E:\Anansi\qualification\tmp

# 4. Verify the build manifest offline
node scripts/windows-release/build-verify.mjs verify --manifest E:\Anansi\qualification\artifacts\release-manifest.json

# 5. Installer identity record for the isolated build
$env:CARGO_TARGET_DIR = 'E:\Anansi\qualification\build-cache'
node scripts/installer-checksums.mjs
```

`build-verify.mjs` is the primary isolated build:
it compiles the web client, the API, and the desktop
app with `CARGO_TARGET_DIR`, `TEMP`/`TMP`, and
artifacts all on E:, copies the executables, web
assets, and installer to the artifacts directory,
and writes `release-manifest.json` (source commit,
dirty state, toolchain, artifact hashes, unsigned
status) plus `SHA256SUMS` and `build-execution.log`.
The separate `installer-checksums.mjs` step writes
`packaging.json` + `SHA256SUMS` into the build-cache
bundle directory (it honors the absolute
`CARGO_TARGET_DIR`); cross-check its installer hash
against `release-manifest.json` — the checklist
gives the exact comparisons.

Alternative in-repo path (only if the isolated path
is unavailable): unset `CARGO_TARGET_DIR` and run
`pnpm build:installer` (web build + Tauri NSIS +
checksum manifest). The installer script REFUSES a
`TAURI_CONFIG` override and refuses to run off
Windows. Expected output:

```
target\release\bundle\nsis\ANANSI Ultimate_0.1.0_x64-setup.exe
target\release\bundle\nsis\SHA256SUMS
target\release\bundle\nsis\packaging.json
```

Record which build path was used; do not mix
artifacts between paths.

For a run-from-source desktop check instead of the
installer:

```powershell
pnpm build:desktop
$env:ANANSI_STORAGE_ROOT = 'E:\Anansi\previews\v1-pilot-rc\data'
.\target\release\anansi-desktop.exe
```

For the browser client (paired local API):

```powershell
pnpm dev:api        # prints the pairing file path
pnpm dev:web        # http://127.0.0.1:1420, pair with the token
```

## 4. Version / build identification

What the tester can see, and its limits. The generic
app version `0.1.0` is NEVER sufficient build
identification on its own. The full identity tuple
is: source commit, clean/dirty source state,
installer SHA-256, application SHA-256, config
SHA-256, toolchain versions, and unsigned status.
Every defect report cites the tuple, from the
identity records below — never from memory and
never the app version alone.

- **In the app:** the Core connection panel
  (Connection view) shows Application name, Core
  version, Application version (all `0.1.0`),
  Contract version **1**, OS, and Core uptime.
  The API reports `apiVersion` `0.1.0`.
- **With the isolated build:** `release-manifest.json`
  is the primary record — `git.commit`, `git.dirty`,
  `toolchain` (Node, pnpm, Git, rustc + host +
  commit, Cargo, Windows SDK, WebView2, makensis),
  artifact hashes + sizes, installer
  `signingStatus: "Unsigned (Development Build)"`.
- **With the installer:** `packaging.json` is the
  build identity — `gitCommit` (exact source),
  `dirtySource` (must be `false`), `toolchain`,
  `installerSha256`, `applicationSha256`
  (bundle-marker-adjusted),
  `unbundledApplicationSha256`, `configSha256`,
  `kind: "unsigned-development-nsis"`, `version`,
  `architecture: "x64"`. `SHA256SUMS` carries the
  installer hash. **Keep `packaging.json` with the
  installer; it is the only source-commit record
  that travels with the artifact.**
- **Known limitation:** no git commit hash is
  embedded in the built binary (no vergen /
  git-version in the build). A tester looking only
  at the running app sees `0.1.0` / contract 1 and
  cannot distinguish builds. Every defect report
  must cite the `packaging.json` `gitCommit` and
  installer hash, not just the app version.

## 5. Installer qualification (reuse existing tooling)

Do NOT create parallel installer tooling. Use:

```powershell
$env:CARGO_TARGET_DIR = 'E:\Anansi\qualification\build-cache'
pwsh -NoProfile -File scripts/verify-installer.ps1 -TestRoot 'E:\Anansi\qualification\installer-test'
```

(`pnpm verify:installer` runs the same script with
the default GUID `%TEMP%` root and the in-repo
target dir.) The script honors an absolute
`CARGO_TARGET_DIR` to find the installer +
`packaging.json` produced by the isolated build;
without it, it reads the in-repo
`target\release\bundle\nsis\` directory.

Preconditions enforced by the script: the installer
identity must be `ANANSI Ultimate` / `Robillionair` /
`currentUser`; the signature status must be exactly
`NotSigned` (this is an unsigned development build —
see §8); it REFUSES to run if an ANANSI uninstall
registration or the `HKCU:\Software\Robillionair\
ANANSI Ultimate` product key already exists (do not
run it on a machine with the pilot installed — use a
clean machine or VM); UI mode requires a non-elevated
host; an explicit `-TestRoot` must be absolute, have
an existing parent, and stay outside the repository
and owner-data roots, with no `.git` ancestry or
reparse points.

What it verifies: installer hash vs `packaging.json`;
silent install to the fresh test target; installed
exe hash vs the manifest `applicationSha256`;
registration `InstallLocation` inside the synthetic
target; the `scripts/verify-workspace.mjs` UI workflow
(unless `-SkipUi`, which explicitly leaves the
installed UI untested); uninstall; app, uninstaller,
and registration gone afterwards; synthetic data
byte-identical; only the test-owned remembered-location
key removed. The verification record
(`installer-verification.json`, in the test root and
the bundle directory) carries the build identity
(commit, dirty state, version, toolchain).

**Record the outcome verbatim** (PASS line or the
first failing assertion). A `-SkipUi` run does NOT
qualify the installed UI — say so in the report.

## 6. Native candidate acceptance (reuse existing tooling)

`scripts/native-candidate-acceptance.mjs` is the
7-step CDP suite (startup/readiness, shell-execution
denial, workspace create, memory create+search,
project+artifact register, backup job + process-
restart survival, screenshot). Candidate locations
are explicit environment variables — no defaults,
no editing the script:

```powershell
$env:ANANSI_CANDIDATE_BINARY = 'E:\Anansi\qualification\artifacts\anansi-desktop.exe'
$env:ANANSI_CANDIDATE_STORAGE_ROOT = 'E:\Anansi\previews\v1-pilot-rc\data'
$env:ANANSI_CANDIDATE_WEBVIEW_UDF = 'E:\Anansi\previews\v1-pilot-rc\webview-udf'
$env:ANANSI_CANDIDATE_EXPECTED_SHA256 = '<release-manifest.json artifacts.executable.sha256>'
node scripts/native-candidate-acceptance.mjs
```

The script refuses to run when locations are
missing/relative, storage or UDF dirs are non-empty,
paths touch owner-data roots or the repository, the
CDP port is occupied, target processes are running,
or the shared test lease is held by another identity;
it prints the binary SHA-256 + size and refuses on
hash mismatch when the expected hash is set.
Optional `ANANSI_CANDIDATE_CDP_PORT` overrides the
default 19323.

Notes: the suite sets `ANANSI_VAULT_TEST_PROVIDER=1`
(synthetic credential provider for the harness).
Its pass does NOT constitute native credential-store
acceptance (see §7) — the vault test provider is a
harness substitute, and synthetic credentials never
prove production native-control behavior. The
per-item commands and evidence list are in the
checklist (step F).

## 7. Native credential-store and file-dialog acceptance

These are OPEN gates. No Linux result covers them.
Perform them manually on the candidate build with
synthetic credentials, and record each step. The
per-item commands, fixture set, and evidence list
are in the checklist (steps I–J); this section
summarizes the gates:

**Credential store (CredUI / OS keyring):**

1. Desktop build only: Providers → Connect AI →
   choose a provider → reach "Credential setup".
2. "Generate reference" must open the NATIVE prompt
   (Windows Credential UI, generic / non-persistent /
   always-visible per the integration authority).
   Record: prompt appeared, was native (not a web
   form), and the flow survived cancel.
3. Enter a synthetic key: it must be stored in the
   OS keyring only — grep the storage root and logs
   afterwards; no key material may appear.
4. Restart the app: the connection must remain
   configured and `connectionVerified` must stay
   bound to the saved connection (provider/endpoint/
   credential reference/endpoint-security flags).
5. Change the endpoint or remove the credential:
   verification must clear, orphaned references must
   appear under "Unused credentials", and guarded
   removal must refuse to delete a reference still
   in use.
6. Browser session (paired API): key entry must be
   REFUSED with the desktop direction (no key entry
   in the browser).
7. Confirm no "Continue with ChatGPT" control is
   offered (deferred; dev-gated prototype only).

**File dialogs:**

1. Document import from the desktop app: exercise
   the picker; record whether it is the native
   Windows dialog and that TXT/MD/PDF/DOCX selection
   works and refusals (unsupported type, oversize)
   are understandable.
2. Export / save flows: exercise the save dialog;
   record native vs web behavior. (The interactive
   Windows file dialog is recorded as an open item
   in the matrix — report the observed behavior
   exactly; do not assume it.)
3. Unsupported-file and oversize-file errors must
   name the problem and the limit; no stack traces
   to the tester.

## 8. Installation / upgrade / restart / backup-restore / uninstall

The per-item checklist (steps H, K–M) is
authoritative for commands and evidence; this
section summarizes the gates:

- **Install:** §5 script on a clean machine (or VM).
  Record SmartScreen / reputation behavior exactly
  (expected for an unsigned build — see §9). The
  installer is per-user (`currentUser`), English
  only, x64 only, and embeds the WebView2
  bootstrapper: a machine without WebView2 needs
  internet to install the runtime.
- **Upgrade:** install this candidate over the
  previous development build (if any) in the
  isolated location; verify the workspace, projects,
  documents, and provider configuration survive.
  Downgrades are BLOCKED by the installer
  (`allowDowngrades: false`) — verify a downgrade
  attempt is refused rather than silently applied.
- **Restart:** close and relaunch (desktop and
  browser/API paths); verify the workspace, projects,
  documents, search results, and Ask history state
  persist; verify an in-flight job resumes or parks
  truthfully (no phantom Completed).
- **Backup / restore:** follow the M2.5 procedure in
  `docs/RECOVERY.md` (consent → snapshot → save the
  Completed artifact from Jobs → import into a
  landing workspace → preview schema/scope/counts/
  privacy/size/hash → fresh-ID restore → reopen and
  verify). Verify: unknown schema, hash-invalid, and
  Sealed backups fail closed; restored schedules are
  disabled; originals are preserved. Backups are
  PLAINTEXT snapshots — handle the files accordingly.
- **Uninstall:** uninstall from Settings/Apps or the
  bundled uninstaller; verify the app, uninstaller,
  and registration are gone AND the user storage
  root survives (the uninstaller must not delete
  user data). Record the storage-root path checked.
- **Concurrent duplicate ingest probe (open):** with
  the candidate running, start two simultaneous
  imports of the same document and record the
  outcome. A 2026-11-05 finding (503
  `STORAGE_UNAVAILABLE` under SQLite contention in
  `sweep_staging`) was never re-verified; the queue
  machinery changed since. Report the observed
  behavior, whatever it is.

## 9. Known limitations and unsigned-installer status

- **Unsigned development installer.** No Authenticode
  signature (`NotSigned` is the EXPECTED status in
  `verify-installer.ps1`). SmartScreen and
  reputation warnings are expected; the tester must
  record them, not bypass them with fake signing.
  Signed releases are M15 work.
- **No commit hash in the binary** (§4). Build
  identity lives in `release-manifest.json` /
  `packaging.json` only.
- **Qualification-script limits (this checkpoint).**
  The PowerShell changes (`-TestRoot`, identity
  record, `CARGO_TARGET_DIR` bundle dir) were
  reviewed line-by-line but could not execute on
  Linux — step E of the checklist is their first
  execution. `launch-acceptance.mjs` uses the
  default WebView2 user-data folder; its
  running-process refusal is the mitigation. One
  build at a time per `CARGO_TARGET_DIR`.
- **Windows review outstanding** for: Connect AI
  API-key setup + verification repair; M13 PDF
  integration; M13 DOCX ingestion; SOMNIA compaction
  slices; SwarmTank v2; ChatGPT sign-in spike
  (deferred, dev-gated). Linux evidence only for
  those slices — this handoff does not upgrade it.
- **Full e2e battery NOT re-run on this branch.**
  What ran on Linux is listed in §11; everything
  else carries its historical evidence only.
- **Sealed records fail closed; storage is
  plaintext** (except the isolated M2.7 vault core,
  which the application does not integrate). No
  encryption claim is made for workspaces, backups,
  or exports.
- **No ChatGPT sign-in, no generated-code execution,
  no 2.0 features** (Corporate/Nanzi/Enterprise) —
  none are offered; do not test for them as pilot
  features.
- **Model answers require a reachable local runtime
  or configured provider.** Without one, Ask reports
  Offline with citations — that is correct behavior,
  not a defect.
- **Limits:** document uploads TXT/MD 1–65,536
  bytes and PDF/DOCX 1–1,048,576 bytes
  (extracted text TXT/MD 1–8,000 chars, PDF/DOCX
  1–32,000 chars); 4,000-char Ask context budget;
  single-user local deployment; no multi-machine
  sync; no macOS build.

## 10. Rollback

- **Source rollback:** re-fetch the baseline bundle
  (`dff9bb1`) or the previous candidate's bundle and
  rebuild; or reinstall the previous installer from
  its `packaging.json`-identified build. Never build
  a "rollback" from a dirty tree.
- **Data rollback:** the storage root is the unit of
  safety. Before any upgrade test, take a snapshot
  (§8) and keep the file. Never downgrade a storage
  `user_version`, never copy a live SQLite main file
  alone, never open a newer-schema workspace with an
  older build expecting it to work (the application
  refuses foreign schemas — that refusal is correct).
- **Installer rollback:** uninstall the candidate
  (§8), confirm the storage root survived, reinstall
  the previous build, reopen, and verify the
  workspace. If the storage root was opened by a
  newer build, restore from the pre-upgrade snapshot
  instead of forcing the older build onto it.
- **Report rollback tests** with the exact commits
  and installer hashes involved.

## 11. Linux evidence on this branch (for the record)

Code on this branch is byte-identical to reviewer
acceptance `8453f649` (verified both-ways diff at
merge `d98a229`); branch changes since are docs, one
e2e spec, and one test-helper parameter. Linux checks
run for this checkpoint (logs in the delivery
evidence):

- Docs validation 21/21 (`tests/docs-validation.test.mjs`).
- TypeScript `tsc --noEmit`: 0 errors. (The journey
  spec as first committed failed typecheck with 4
  errors — unresolved `.mjs` helper imports and a
  closure-narrowed gate variable. Both were fixed in
  the spec using the established `@ts-expect-error`
  convention and a holder object; the before/after
  logs are in the delivery evidence. No product code
  changed.)
- Contracts `--check`: exit 0, 0 drift. (The
  "unknown format uint" lines are pre-existing
  schema noise, not failures.)
- UI suite 347/347 (`pnpm test:ui`).
- Node suite 39/40: sole failure is the pre-existing
  Windows-only MSVC-installer check (fails on Linux
  by design; the `pwsh` backup tests were not run on
  Linux). Docs, hygiene, doctor, and installer suites
  all pass.
- Rust lib 417/417 in one run (4 + 349 + 1 + 63),
  matching the integrated lineage count — expected,
  since the Rust code is byte-identical to `8453f649`.
  The full `--tests` battery was NOT repeated (the
  4 live-Ollama environmental failures are unchanged
  and the code is unchanged).
- First-time-user journey walk
  `tests/e2e/pilot-journey.spec.ts` 1/1 across three
  green runs (runs 5–7; earlier runs failed on
  harness issues only, each diagnosed against code —
  see the run logs).
- `tests/e2e/backups.spec.ts` 1/1 after the shared
  helper change.

The full Rust battery at the integrated lineage tip
was 417/417 on Linux and 416/416 on Windows; those
numbers belong to that tip. This branch's delta is
docs + one e2e spec + one test-helper parameter, and
the checks above cover exactly that delta plus one
lib confirmation run.

Environmental exclusions (not defects): 4
`semantic_workflow` live-Ollama failures (no local
model on the Linux VM); the node suite's
MSVC-installer check fails on Linux (Windows-only).

## 12. Short customer pilot guide

**What ANANSI is (say this):** a local-first
intelligence workspace. Your notes, documents, and
projects stay on your machine. AI features use a
local model or a provider you configure; nothing is
sent anywhere without your setup and, for paid use,
your explicit permission and budget.

**First 15 minutes:**

1. Install from the provided installer (expect a
   SmartScreen prompt — unsigned development build;
   proceed only if you were given this build
   directly). Note the build identity from the
   accompanying `packaging.json`.
2. Create a workspace. Read the privacy-limits
   screen: storage is plaintext, there is no
   encrypted vault, backups are not automatic.
3. Skip AI setup, or add a local model (Ollama/LM
   Studio) or an API key via Connect AI (desktop
   only; the key is stored in your OS keyring).
4. Create a project, add a few notes, import a
   small document (TXT, Markdown, PDF, or DOCX).
5. Search for a word from your document. Ask a
   question about it and check the citations.
6. Take a backup (Jobs → snapshot → save the file)
   and keep it somewhere safe.

**Do not:** use real secrets or irreplaceable data;
expect ChatGPT sign-in (not in this pilot); expect
generated code to run (it never runs); share backup
files as if they were encrypted (they are plaintext
snapshots).

**Report problems with:** the `packaging.json`
`gitCommit` + installer hash, the exact steps, what
you expected, what happened, and the synthetic data
involved. Screenshots of error banners help; never
include API keys or real private content.

## 13. What the Windows reviewer must do next

Work the per-item checklist
(`docs/handoffs/v1-pilot-installer-checklist.md`)
in order; this section summarizes it:

1. Verify the delivery (MANIFEST hashes, bundle
   fetch, exact tip, clean tree, empty
   product-code diff vs `8931d34b`) per the
   delivery message (checklist step A).
2. Run preflight + doctor (step B); build per §3
   in the isolated E:\Anansi layout (§2); record
   the full identity tuple and cross-checks
   (step C).
3. Generate + verify the deterministic fixtures
   (step D).
4. Run §5 installer qualification on a clean
   machine/VM with `-TestRoot` (step E); record
   the verification record.
5. Run §6 native candidate acceptance with the
   explicit env-var locations + expected hash
   (step F); optional guarded launch (step G).
6. Perform the fresh install + SmartScreen
   observation (step H), credential-store (step I),
   and file-dialog + concurrent-import probe
   (step J) manually; record each step verbatim.
7. Perform upgrade (step K), restart/backup/fresh
   restore (step L), and uninstall preserving
   storage (step M).
8. Report: exact commit + installer hash, PASS/FAIL
   per checklist item with verbatim evidence, and
   any findings as new review items — do NOT restart
   the closed WR-REC repair loop (416/416 focused
   acceptance stands).
9. Proposed global status updates for Antigravity
   (do not edit on Muse's behalf): README's stale
   fault-test/acceptance paragraph (superseded by
   the 416/416 aggregate at `8453f649`); matrix rows
   AN-008 and AN-026 (implemented, not `planned`);
   a BUILD_STATUS entry for this checkpoint once the
   Windows verdict is known. The release-script
   changes in this checkpoint are flagged for
   reviewer acceptance (see the audit's ownership
   note).

## 14. Boundaries respected

No paid/cloud inference, no credentials, no
owner-data changes, no generated-code execution, no
installer promotion, no pushes, no publication, no
modification of Antigravity's worktree. Antigravity-
owned documents were not edited; the BUILD_STATUS
merge resolution preserved both sides verbatim.
