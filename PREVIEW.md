# ANANSI integrated preview — R16 checkpoint

Run **Start ANANSI Preview.cmd** from this folder. This is a development preview, not a signed release or installer.

## What this checkpoint is

Branch `codex/local-testing-r16`, created directly from the verified R16 tip `a1b0c80724eb30497ecf0ccb69149a0499dc6ee7`. It contains:

- M2–M5 and M6 Slices 1–3 (projects and the versioned artifact registry, project-scoped idea generation, concept evaluation and comparison), the R13 reliability repair, the backup/recovery repair (R15), and the R15 test/Clippy repair (R16) — exactly as delivered and verified at R16.
- The vault schema-ladder reconciliation: snapshot verification no longer migrates the catalog it inspects, so the vault's explicit schema upgrade runs as designed; the `vault_increment2` suite passes in full (9/9 on Linux), covering sealing from both the legacy schema-8 ladder and the current schema-10 ladder. Record: `docs/handoffs/vault-ladder-reconciliation-addendum.md`.
- This preview update. The R11 preview commit `f6cecd2` — including the Tauri capability fix that grants the main window access to the artifact domain command (`apps/desktop/src-tauri/permissions/artifacts.toml`, `allow-artifacts` in `capabilities/default.json`) — is an ancestor of this branch and is preserved unchanged. It is **not** re-applied.

## Preview data

Preview data is separate from any existing catalog and from the R11 preview: `%LOCALAPPDATA%\ANANSI-Preview\r16`. The launcher sets this root; the preview never points at your existing data automatically. Closing and reopening the preview retains this data. Do not point an older build at it: the registry uses schema 10. Use synthetic/non-sensitive content during acceptance.

## Build (Windows, PowerShell)

These are the build steps for this checkpoint; they have not been executed on the machine that prepared this source — the Windows build and launch of this checkpoint are part of your verification.

```powershell
pnpm install --frozen-lockfile --ignore-scripts
pnpm build:web
node scripts/cargo.mjs build -p anansi-desktop --features custom-protocol --locked
```

The preview uses the normal embedded web frontend, without the CDP/native test configuration or synthetic credential feature. The desktop shortcut is **ANANSI Preview R16**.

## Launch

Run **Start ANANSI Preview.cmd**. It refuses to start if the executable has not been built and otherwise opens the preview window against the `r16` data root above.

## What to test

1. **Basics.** Complete onboarding and create a workspace. Save a memory, revise it, inspect its history, and search for the revised wording. Close and reopen the preview; your data is retained.
2. **Projects and artifacts.** Open Projects, create a project, add an artifact, revise it, and inspect both versions. Archive the project and confirm it refuses further artifact writes.
3. **Idea generation.** This needs a local model runtime that **you** have already installed (for example Ollama with a model you already pulled). The preview never downloads a model, asks for cloud credentials, or makes a paid call. Open **AI Providers**, configure the local runtime, and leave the Zero-Cost Lock on. In a project, fill the Idea generation brief, tick one or more context memories (only ticked memories are sent — check the "View what ANANSI sent" disclosure), and generate. You should see 5–10 concepts; save the set as an artifact.
4. **Comparison, evaluation, rubric history.** Open the saved idea set, tick up to three concepts, and assess them under the standard rubric (five dimensions; scores are yours, 1–5 with a rationale — a dimension you skip reads **not assessed**, never zero). Save the evaluation and reopen it. Then edit a rubric weight, save the rubric as a new version, and save a second evaluation: the first evaluation still shows the rubric exactly as originally used (each evaluation embeds its rubric snapshot). Evaluations are your recorded judgement, **not market evidence** — the app labels them that way.
5. **Backup and fresh restore.** Open **Backups**, tick the plaintext consent, and **Create backup**. In **Jobs and export**, download the completed backup file. Back in **Backups**, import the saved file, **Preview restore**, and restore it into a fresh workspace under a new name. Open the restored workspace: the project, its artifacts, the idea set, the rubric and the evaluation are all there — and the restored evaluation cites the **restored** idea set and rubric (references are remapped at restore; the originals are untouched). Restoring the same backup artifact a second time is refused as a conflict rather than duplicating data.
6. **Restart.** Close and reopen the preview once more and reopen both the original and restored workspaces; everything persists.

## Verification status of this checkpoint

- **Verified on Linux** (preparing machine): the full regression and browser-workflow evidence for this checkpoint is summarized in `docs/BUILD_STATUS.md` (top section) and the handoff addenda. The browser workflow was exercised end to end against a synthetic loopback provider: project → ideas → saved artifact → evaluation → backup → fresh restore → restart → reopen, with the restored evaluation verified to resolve to its restored sources.
- **Verified on Windows by the owner, for R16 itself**: the gated eviction/cancellation/accounting test, strict backup-test Clippy, documentation and diff checks; R15's five recovery tests, evaluation tests, validation tests and 47 UI/contracts tests passed Windows verification earlier. This is not full application or production certification.
- **Pending Windows verification**: the Windows build and launch of this preview checkpoint, and the vault reconciliation changes. A Linux pass does not close either.
- **Production gates still open** (unchanged): Windows native/manual acceptance (real CredUI prompt and OS keyring), live-provider verification, the G1 external audit, and remote CI. Vault/native credential dialogs and real cloud providers keep their separate acceptance gates. No real model, cloud or credential-store operation is performed automatically by the launcher.
