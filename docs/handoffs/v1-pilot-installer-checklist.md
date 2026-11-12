# ANANSI v1 pilot — Windows installer qualification checklist

Qualification branch: `muse/v1-pilot-installer-qualification`,
from Windows reviewer tip
`57029b2eb2acfa47b7d180591b9b73fd3c04ec25` (focused
browser journey PASS on runtime `8931d34b`). Product
code is unchanged vs `8931d34b` (verified: empty diff
over `crates/`, `apps/`, `packages/`); this checkpoint
changes release scripts, tests, and docs only. The
accepted journey and the 416/416 recovery aggregate
(scoped to `8453f649`) are preserved, not re-claimed.

Companion documents: the release handoff
(`docs/handoffs/v1-pilot-windows-handoff.md`), the
pilot scope (`docs/handoffs/v1-pilot-scope.md`), the
acceptance criteria
(`docs/handoffs/v1-installer-qualification-acceptance.md`),
and the script audit
(`docs/handoffs/v1-installer-qualification-audit.md`).

## Conventions

- Every item is labeled **AUTOMATED** (exact command,
  machine-checked) or **MANUAL/NATIVE** (human-
  observed on the Windows machine). A MANUAL/NATIVE
  item is never satisfied by a browser test, a mock,
  or a Linux run. No item in this checklist claims
  native, SmartScreen, installer, or credential-store
  acceptance from anything but the observed native
  run.
- Working directory is the fresh worktree root
  `E:\Anansi\worktrees\v1-pilot-rc` unless stated.
  PowerShell 7 for `.ps1` steps. All commands are
  copy-pasteable; do not retype hashes or paths.
- **Stop rules.** Any FAIL stops the sequence: record
  the first failing assertion verbatim and stop; do
  not work around it, do not weaken a check, do not
  proceed to install anything. A refused guard (lease,
  running process, occupied port, owner-data path) is
  a STOP, not a failure of the product: reconcile and
  re-run from the stopped step.
- **Identity rule.** The generic app version `0.1.0`
  is NEVER sufficient build identification. Every
  report cites the full tuple: source commit,
  clean/dirty state, installer SHA-256, application
  SHA-256, config SHA-256, toolchain versions, and
  unsigned status — from `release-manifest.json` /
  `packaging.json`, never from memory.
- **Redaction.** Logs and reports must not contain
  user profile paths, key material, tokens, or pairing
  files. Strip them before sharing. Synthetic data
  and synthetic keys only, everywhere.
- **Isolation.** One build at a time into a
  `CARGO_TARGET_DIR`; never share one target dir
  across concurrent builds or worktrees. The running
  preview and all owner storage stay untouched.

Layout used by the commands below:

```
E:\Anansi\worktrees\v1-pilot-rc\        fresh fetched source; build here
E:\Anansi\qualification\build-cache\    CARGO_TARGET_DIR (isolated)
E:\Anansi\qualification\tmp\            isolated temp root
E:\Anansi\qualification\artifacts\      build outputs + release-manifest.json
E:\Anansi\qualification\fixtures\       deterministic fixture set
E:\Anansi\qualification\installer-test\ installer verification root
E:\Anansi\previews\v1-pilot-rc\data\    isolated synthetic storage root
E:\Anansi\previews\v1-pilot-rc\webview-udf\  isolated WebView2 user data
```

## A. Delivery verification — AUTOMATED

```powershell
git rev-parse HEAD
git status --porcelain
git merge-base --is-ancestor 8931d34b06faf16806a089a587ebd58e6b26f9a5 HEAD
git merge-base --is-ancestor 57029b2eb2acfa47b7d180591b9b73fd3c04ec25 HEAD
git diff --stat 8931d34b06faf16806a089a587ebd58e6b26f9a5 HEAD -- crates apps packages
```

Expected: `rev-parse` equals the delivery MANIFEST
tip exactly; status empty; both ancestry checks pass;
the product-code diff is empty. Record the tip.

## B. Preflight and toolchain — AUTOMATED

```powershell
node scripts/doctor.mjs --require-native
node scripts/windows-release/preflight.mjs
```

Expected: exit 0 from both; preflight prints
`VERDICT: PASS`. Record the toolchain block (Node,
pnpm 11.25.0, Git, rustc 1.99.1 MSVC host, Cargo,
Windows SDK version, WebView2 versions, makensis).
A makensis warning means installer bundling will
fail — resolve it before step C. Running target
processes or an occupied lease BLOCK the run: stop,
reconcile, never steal the lease.

## C. Isolated build + identity records — AUTOMATED

```powershell
pnpm install --frozen-lockfile
node scripts/windows-release/build-verify.mjs build --installer --cargo-target-dir E:\Anansi\qualification\build-cache --artifacts-dir E:\Anansi\qualification\artifacts --temp-dir E:\Anansi\qualification\tmp
node scripts/windows-release/build-verify.mjs verify --manifest E:\Anansi\qualification\artifacts\release-manifest.json
$env:CARGO_TARGET_DIR = 'E:\Anansi\qualification\build-cache'
node scripts/installer-checksums.mjs
```

Expected: build exit 0; `verify` prints `VERDICT:
PASS`; the checksum step prints its PASS line and
writes `packaging.json` + `SHA256SUMS` into the
build-cache bundle directory. Record and cross-check
the identity tuple:

- `release-manifest.json`: `git.commit` equals the
  fetched tip, `git.dirty` is `false`, `toolchain`
  is populated (no nulls except `makensis` only if
  NSIS was genuinely absent — but then step B failed),
  `artifacts.installer.signingStatus` is
  `Unsigned (Development Build)`.
- `packaging.json`: `gitCommit` equals the fetched
  tip, `dirtySource` is `false`, `kind` is
  `unsigned-development-nsis`, `toolchain` populated.
- Cross-checks (must be equal, else STOP):
  `packaging.json` `installerSha256` ==
  `release-manifest.json`
  `artifacts.installer.sha256`; `packaging.json`
  `applicationSha256` == `release-manifest.json`
  `artifacts.installer.nsisPayloadSha256`.

Alternative in-repo path (only if the isolated path
is unavailable): unset `CARGO_TARGET_DIR` and run
`pnpm build:installer`; the identity record is then
`target\release\bundle\nsis\packaging.json`. Record
which path was used; do not mix artifacts between
paths.

## D. Deterministic fixtures — AUTOMATED

```powershell
node scripts/qualification-fixtures.mjs --target E:\Anansi\qualification\fixtures
node scripts/qualification-fixtures.mjs --verify E:\Anansi\qualification\fixtures
```

Expected: both print PASS for 9 fixtures. Record
`MANIFEST.json`. (This step also runs on Linux; it
is the only checklist step with Linux evidence.)

## E. Installer qualification — AUTOMATED

Preconditions: clean machine or VM — the script
REFUSES to run if any ANANSI uninstall registration
or the `HKCU:\Software\Robillionair\ANANSI Ultimate`
product key exists. Non-elevated host for the UI
workflow.

```powershell
$env:CARGO_TARGET_DIR = 'E:\Anansi\qualification\build-cache'
pwsh -NoProfile -File scripts/verify-installer.ps1 -TestRoot 'E:\Anansi\qualification\installer-test'
```

Expected: the PASS line verbatim (installed UI
workflow + uninstall + byte-identical preserved
data). Record `installer-verification.json` (copies
in the test root and the bundle directory) including
its identity fields (`gitCommit`, `dirtySource`,
`version`, `toolchain`). A `-SkipUi` run explicitly
does NOT test the installed UI — report it as such.
The default form without `-TestRoot` (GUID `%TEMP%`
root) remains available and behaves as before.

## F. Native candidate acceptance — AUTOMATED

```powershell
$env:ANANSI_CANDIDATE_BINARY = 'E:\Anansi\qualification\artifacts\anansi-desktop.exe'
$env:ANANSI_CANDIDATE_STORAGE_ROOT = 'E:\Anansi\previews\v1-pilot-rc\data'
$env:ANANSI_CANDIDATE_WEBVIEW_UDF = 'E:\Anansi\previews\v1-pilot-rc\webview-udf'
$env:ANANSI_CANDIDATE_EXPECTED_SHA256 = '<release-manifest.json artifacts.executable.sha256>'
node scripts/native-candidate-acceptance.mjs
```

Expected: all 7 checks pass
(startup/readiness, shell-execution denial, workspace
create, memory create+search, project+artifact
register, backup job + restart survival, screenshot
at `test-results\native-candidate-verified.png`).
NQ-01 repair: [1/7] now invokes `get_readiness` and
asserts it; [5/7] asserts the real `artifactId`,
reads the artifact back through native IPC, and
probes that it is unreadable from a second
workspace; [6/7] reopens the same artifact after
the restart. The [5/7] line must print a 32-hex
artifact id, never `undefined`. See
`docs/handoffs/nq01-native-harness-repair.md`.
Record the printed binary SHA-256 + size and the
full output. The script refuses missing/relative
locations, non-empty storage/UDF dirs, owner-data
paths, occupied ports/processes/leases, and hash
mismatches — a refusal is a STOP, not a product
failure. The suite sets the harness-only vault test
provider: its pass NEVER evidences credential-store
acceptance (see step I). For a re-run, use a fresh
storage/UDF directory (the script refuses non-empty
ones) and record that you did.

## G. Guarded launch with restart — AUTOMATED (optional)

```powershell
node scripts/windows-release/launch-acceptance.mjs --restart --artifacts-dir E:\Anansi\qualification\artifacts --temp-dir E:\Anansi\qualification\tmp
```

Expected: `STATUS: PASS` with a real-API workspace
and restart persistence verified. Record the
evidence path. Note: this harness uses the default
WebView2 user-data folder and refuses to run while
any `anansi-desktop.exe` is active — never run it
concurrently with the preview. If it is skipped,
record that it was skipped; do not claim its
coverage from step F.

## H. Fresh install and first launch — MANUAL/NATIVE

On the clean machine/VM, double-click the installer
from the artifacts directory (NOT silent mode).
Observe and record verbatim:

1. SmartScreen: what appeared (exact text), what you
   clicked, and the outcome. The installer is
   UNSIGNED — record the observed behavior, do not
   assume it.
2. Install to the default per-user location.
   Record: install completed, no errors.
3. First launch: onboarding + privacy review + AI
   setup choice. Record each screen and the empty
   states. Create one workspace (synthetic name).
4. Readiness panel: record Application/Core version,
   Contract version, OS. Cite the build by the
   identity tuple from step C, not by `0.1.0`.

## I. Native credential prompt — MANUAL/NATIVE

Launch the installed app with a FRESH isolated
storage root. Synthetic keys only — use an obviously
fake value (for example `synthetic-qualification-key-001`).
Never a real key. Record each observation verbatim:

1. Providers → Connect AI → choose a provider →
   reach "Credential setup".
2. "Generate reference" opens the NATIVE Windows
   credential prompt (generic / non-persistent /
   always-visible). Record: prompt appeared, was
   native (not a web form).
3. Cancel the native prompt. Record: the flow
   survives cancel with no partial credential and an
   understandable state.
4. Enter the synthetic key. Record: stored in the OS
   keyring only — afterwards, grep the storage root
   and logs for the synthetic value; it must NOT
   appear anywhere on disk or in logs.
5. Restart the app. Record: the connection remains
   configured and `connectionVerified` stays true.
6. Remove the credential via the UI. Record: removal
   succeeds (guarded removal) and the connection
   state updates; check the "Unused credentials" /
   orphan handling behaves as the UI states.
7. ChatGPT sign-in: record that no ENABLED
   "Continue with ChatGPT" control appears (OAuth
   stays disabled/deferred for the pilot).

## J. Native document picker — MANUAL/NATIVE

Launch the installed app with a FRESH isolated
storage root. Use the NATIVE file picker
(Documents → import) for each fixture in
`E:\Anansi\qualification\fixtures`. Record per file:
dialog behavior, outcome, and the EXACT message
shown (quote it; expected messages below come from
the shared UI validation and must be confirmed, not
assumed):

| Fixture | Expected |
|---|---|
| `notes.txt` | Imported; `bramblewick` / `quokka` searchable |
| `notes.md` | Imported; `bramblewick` searchable |
| `normal-3page.pdf` | Imported; 3 pages of text |
| `normal.docx` | Imported; headings + table text |
| `unicode-headings.docx` | Imported; Unicode headings intact |
| `unicode/café-üñïcode-notes.txt` | Imported via the picker navigating the Unicode directory and filename |
| Cancel (open picker, cancel) | No import, no error, app stable |
| `unsupported.exe` | Refused; expected "EXE files are not supported yet." |
| `oversized.txt` | Refused; expected "The file must be at most 65536 bytes." |
| `empty.txt` | Refused; record the exact message |

Concurrent-import probe (open M13 item — report what
happens, do not assume): select `normal-3page.pdf`
twice in immediate succession, starting the second
import before the first completes. Record the exact
outcome for BOTH imports (completed / error + exact
message). Then restart the app and confirm the
imported documents and search survive.

## K. Upgrade preserving data — MANUAL/NATIVE

With synthetic data in the installed app (workspace,
project, imported documents from step J), run the
SAME installer again over the existing install.
Record: installer behavior, and after relaunch that
all synthetic data (documents, search, project)
is intact and the registration is unchanged. Note
honestly: this is a same-version reinstall; a true
version upgrade cannot be tested until the version
increments.

## L. Restart, backup, fresh restore — MANUAL/NATIVE

1. Restart the app: synthetic data intact.
2. Jobs → backup (workspace scope, plaintext
   consent) → job reaches Completed → save the
   artifact. Record the job outcome + artifact
   SHA-256.
3. Import the saved backup into a fresh workspace:
   preview the schema/scope/counts, restore with a
   FRESH workspace ID, reopen. Record: preview
   counts, new ID differs from the original, search
   works after reopen.

## M. Uninstall preserving storage — MANUAL/NATIVE

1. Before: hash the synthetic storage directory
   contents (`Get-FileHash` list) and record it.
2. Uninstall via Windows Settings → Apps (or the
   uninstaller). Record: the app, uninstaller, and
   uninstall registration are gone.
3. After: re-hash the storage directory and compare
   byte-for-byte with step 1. Record: storage
   preserved identically. Record the remembered-
   location key behavior (present/absent, value).

## N. Evidence and reporting

Attach / cite, per item: the command, the exit code
or observed outcome, and the verbatim PASS line /
message / screenshot path. Minimum evidence set:

- Step A: fetched tip + empty status + ancestry.
- Step B: preflight + doctor output (redacted).
- Step C: `release-manifest.json`,
  `packaging.json`, `build-execution.log`,
  cross-check results.
- Step D: fixture `MANIFEST.json`.
- Step E: `installer-verification.json` + PASS line.
- Step F: full output + screenshot path.
- Step G (if run): evidence path + PASS line.
- Steps H–M: per-item notes with verbatim messages,
  SmartScreen observation, hash lists for M.

Report per-item PASS / FAIL / SKIPPED (with reason)
in a table. A defect report cites the full identity
tuple from step C. New findings become new review
items; the closed recovery repair loop is not
restarted.

## O. Known blockers and limits

- The PowerShell changes in this checkpoint
  (`-TestRoot`, identity record, `CARGO_TARGET_DIR`
  bundle dir) were reviewed line-by-line but could
  not execute on Linux. Step E is their first
  execution; report any script error verbatim.
- `build-verify.mjs`, `installer-checksums.mjs`,
  the candidate script, and the desktop verifiers
  are Windows-only and were not executed on Linux;
  their new pure logic (location resolution,
  toolchain extraction, fixture generation) is
  covered by Linux unit tests (see below).
- `launch-acceptance.mjs` uses the default WebView2
  user-data folder; its running-process refusal is
  the mitigation (step G note).
- One build at a time per `CARGO_TARGET_DIR`.
- The 416/416 aggregate stays scoped to reviewer
  `8453f649` and is not re-claimed here. Standing
  gates (G1, signing, M7 acceptance/promotion, §32
  sandbox, M5 gates, M8 Slice 4, AC-I10 blocked) are
  unchanged. No live-provider verification, no
  model-quality claims, OAuth disabled/deferred,
  generated-code execution unavailable, 2.0 work
  planning-only.

## P. Linux verification of this checkpoint's tooling

No product code changed. Linux runs for the changed
surface (logs in the delivery package):

- Node suite (docs, hygiene, doctor, native-setup,
  installer, candidate-locations, qualification-
  fixtures): 60/61 — the sole failure is the
  pre-existing Windows-only MSVC installer test,
  which needs `pwsh`/`powershell` and cannot run on
  Linux. All 21 new tests pass (15 location/guard,
  4 fixture, 2 toolchain).
- Docs validation: 21/21 (re-run after these docs
  landed; see the delivery log).
- `node --check` clean on every edited `.mjs` file;
  the candidate script's Windows guard verified to
  refuse on Linux with no side effects; the desktop
  guard and the checksum script's Windows refusal
  verified likewise.
- NOT repeated, with reason: Rust lib/UI/e2e suites
  (no Rust, contract, UI, or spec changes; the
  Windows journey already passed 1/1 on this exact
  runtime). Full `--tests` battery not repeated
  (code unchanged; the 4 live-Ollama exclusions
  unchanged).
