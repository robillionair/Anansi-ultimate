# ANANSI integrated preview

Run **Start ANANSI Preview.cmd** from this folder. This is a development preview, not a signed release or installer.

The branch combines M2–M5 and M6 Slice 1 from Muse checkpoint `07be39ca`, plus a local correction granting the main Tauri window access to the artifact domain command.

Preview data is separate from any existing catalog: `%LOCALAPPDATA%\ANANSI-Preview\r11`. Closing and reopening the preview retains this data. Do not point an older build at it: the registry uses schema 10. Use synthetic/non-sensitive content during acceptance.

## Try these workflows

1. Complete onboarding and create a workspace.
2. Save a memory, revise it, inspect history, and search for the revised wording.
3. Open Projects, create a project, add an artifact, revise it, and inspect both versions.
4. Close and reopen the preview; confirm your project and versions remain.
5. Open AI Providers to configure an already installed local runtime, or Ask ANANSI to choose an installed local model. No cloud credentials or paid calls are needed for the memory/project workflows.

Vault/native credential dialogs and real cloud providers retain separate acceptance gates. No real model, cloud or credential-store operation is automatically performed by this launcher.

## Rebuild

```powershell
pnpm install --frozen-lockfile --ignore-scripts
pnpm build:web
node scripts/cargo.mjs build -p anansi-desktop --features custom-protocol --locked
```

The preview uses the normal embedded web frontend, without the CDP/native test configuration or synthetic credential feature.

## Local verification — 2026-10-21

- R11 ZIP/bundle checksums and complete Git object history verified.
- Six artifact-registry Rust tests and 39 UI/contract tests passed on Windows; TypeScript and the web production build passed.
- Normal Windows desktop build (`custom-protocol`, no synthetic credential feature) passed.
- Launcher opened the built executable; the responsive window titled `ANANSI Ultimate` and its separate preview catalog were observed.
- Artifact/native UI acceptance, real credential dialogs, live providers, remote CI and production certification are not established by a successful build/launch. The old shared test lease was not removed.

The desktop shortcut is **ANANSI Preview R11**. The primary development checkout and protected `master` were not modified.
