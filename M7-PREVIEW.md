# ANANSI M7 Slices 1–3 Windows preview

Application baseline: `cc25155ffe758dbab68b40e9576381738055b2ee`.
Local integration branch: `codex/preview-m7-slices2-3`.

Launch **Start ANANSI Preview M7 Slices 2-3.cmd** after the Windows build.
The integrator also places a copy under `E:\Anansi` and a named desktop shortcut.

- Executable: `E:\Anansi\build-cache\preview-m7-slices2-3\debug\anansi-desktop.exe`.
- Personal data: `%LOCALAPPDATA%\ANANSI-Preview\m7-slices2-3`.
- Existing M7 (`m7`, `preview-m7`) and R17 previews are separate and preserved.
- No data migrates automatically. No provider/model is installed automatically.
- Configure an already-installed local provider for actual model output.
  The synthetic acceptance fixture is separate from this personal preview.

## What to test

1. Create/open a project and a saved idea or opportunity set.
2. Generate/save a specification draft. Explicitly approve its exact version.
3. Generate/save a source package. Inspect its files as text only.
4. Request a source revision. Reject once; request again and accept a new version.
5. Prepare a reviewable deliverable. Inspect/edit documentation, save and export.
6. Revise documentation, switch back to v1 and inspect its editor text.
7. Edit a document, cancel, reopen: the cancelled draft must be gone.
8. Restart/reopen. Back up and restore into a fresh workspace, then reopen/export
   the restored deliverable. Existing workspaces must remain intact.

Generated HTML/JavaScript stays inert. Build/test/interactive preview/deployment
of generated products remain unavailable; export is for human review. This is a
development preview, not a signed release, real-model certification or audit.

## Build on Windows

Close only this preview before rebuilding its executable. Do not overwrite a
running executable or terminate another preview to free a build path.

```powershell
$env:TEMP='E:\Anansi\tmp'
$env:TMP='E:\Anansi\tmp'
$env:CARGO_TARGET_DIR='E:\Anansi\build-cache\preview-m7-slices2-3'
pnpm install --frozen-lockfile --ignore-scripts --store-dir 'E:\.pnpm-store'
pnpm build:web
node scripts/cargo.mjs build -p anansi-desktop -p anansi-api --features anansi-desktop/custom-protocol --locked
```

## Provenance and evidence

The supplied file on 2026-10-27 was the earlier repair ZIP (SHA-256
`a6266e3cc7c68d5a75f9b42ffd060e6d2e7b9a1811990caeecb39a2a14795e99`),
not the reported preview-integration ZIP. Muse's `bed0dc2` was unavailable locally.
The Windows integrator composed this branch from verified `cc25155ff` and
cherry-picked preview-material commit `3335248b`, preserving both build-status
sections. This is a local equivalent integration, not verification of `bed0dc2`.
Application code is unchanged from `cc25155ff`.

Acceptance plan/evidence: `docs/handoffs/M7-local-native-preview.md`.
Native tests use disposable E: data and a synthetic provider; setup/API operations
are labelled separately from actual UI actions.

Windows build and the bounded synthetic native workflow passed on 2026-10-27,
including revision reject/accept, document version/Cancel checks, export hashes,
restart and native reopening after fixture-API fresh restore. The personal preview
is available through **ANANSI Preview M7 Slices 2-3** on the Desktop. Its root is
fresh and contains no synthetic fixture or configured provider. Production gates
and real-provider/native-credential acceptance remain separate.
