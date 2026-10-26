# M7 Product Forge development preview

Application source: `518418b6a955feaadcc6212eab0a3f3fd1e1bcb9`.

Launch `E:\Anansi\Start ANANSI Preview M7.cmd` or the desktop shortcut **ANANSI Preview M7** after the Windows build completes.

- Binary: `E:\Anansi\build-cache\preview-m7\debug\anansi-desktop.exe`.
- Personal preview data: `%LOCALAPPDATA%\ANANSI-Preview\m7` (separate from R17/R16).
- Build/test caches and synthetic acceptance fixtures stay on E:.
- R17 remains the fallback; no existing data is migrated automatically.

## Try Product Forge

1. Configure an already-installed local provider in AI Providers. This preview does not install or download a model.
2. Create/open a project and a saved idea or opportunity set.
3. Open the set and select a concept in Product Forge. Supply purpose, intended users and must-have behavior.
4. Generate and save a specification **draft**.
5. Open the draft, inspect it, and explicitly approve the exact version for **source generation**.
6. Generate the development plan/source, inspect each file as text, and save the source package.
7. Export its TAR package. Do not execute exported files during acceptance.
8. Edit the specification into a new version and confirm it needs renewed approval.
9. Restart/reopen. Back up and restore into a fresh workspace; inspect restored specification/package/approval references.

Generated HTML/JavaScript is not rendered or executed. Build, tests, interactive preview and deployment of generated products remain unavailable. The source is an unverified prototype for human review, not a certified working application. Real-provider/native credential checks, external crypto audit and remote CI retain their separate gates.

The synthetic UI acceptance session uses a disposable E: fixture and fixed loopback responses, not a real model. Its fixture setup is not evidence of user-driven onboarding/provider configuration. Results are recorded separately in `docs/reviews/`.

## Rebuild on Windows

```powershell
$env:CARGO_TARGET_DIR='E:\Anansi\build-cache\preview-m7'
$env:TEMP='E:\Anansi\tmp'
$env:TMP='E:\Anansi\tmp'
pnpm install --frozen-lockfile --ignore-scripts --store-dir 'E:\.pnpm-store'
pnpm build:web
node scripts/cargo.mjs build -p anansi-desktop -p anansi-api --features anansi-desktop/custom-protocol --locked
```

This is a normal embedded development build, not a CDP/debug-credential test build or signed release. Do not rebuild its executable while it is running.
