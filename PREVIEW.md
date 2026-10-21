# ANANSI integrated preview — R17 checkpoint

Run **Start ANANSI Preview.cmd** from this folder. This is a development preview, not a signed release or installer.

## What this checkpoint is

Branch `codex/local-testing-r17`, created directly from the verified R17 repair tip `ab95e1410d5483506b7cdfad95aaa9a3956cf59a`. It contains everything the R16 preview contained, plus:

- **M6 Slice 4: project-scoped Opportunity Discovery** — opportunities derived only from information you deliberately supply (a typed or pasted problem, complaint or observation, a document excerpt, and memories you explicitly select), with a pre-send review of exactly what will be sent, per-opportunity source citations and stated assumptions, saved as immutable project artifacts that the existing Compare & evaluate workflow opens directly. Record: `docs/handoffs/M6-slice4-addendum.md`.
- **The R17 repair: exact-revision provenance at restore** — restored opportunity sets now pin the *restored* counterpart of the exact memory revision used at generation (validated against the backup's own revision rows; missing or mismatched references fail the restore closed). Record: `docs/handoffs/M6-r17-repair-addendum.md`.
- The R16 preview content, unchanged: M2–M5 and M6 Slices 1–3, the R13/R15/R16 repairs, and the vault schema-ladder reconciliation. The R11 preview commit `f6cecd2` — including the Tauri capability fix that grants the main window access to the artifact domain command (`apps/desktop/src-tauri/permissions/artifacts.toml`, `allow-artifacts` in `capabilities/default.json`) — is an ancestor of this branch and is preserved unchanged. It is **not** re-applied.

## Preview data

Preview data is separate from any existing catalog and from the R11 and R16 previews: `%LOCALAPPDATA%\ANANSI-Preview\r17`. The launcher sets this root; the preview never points at your existing data automatically. Closing and reopening the preview retains this data. Do not point an older build at it: the registry uses schema 10. Use synthetic/non-sensitive content during acceptance.

## Build (Windows, PowerShell)

These are the build steps for this checkpoint; they have not been executed on the machine that prepared this source — the Windows build and launch of this checkpoint are part of your verification.

```powershell
pnpm install --frozen-lockfile --ignore-scripts
pnpm build:web
node scripts/cargo.mjs build -p anansi-desktop --features custom-protocol --locked
```

The preview uses the normal embedded web frontend, without the CDP/native test configuration or synthetic credential feature. The desktop shortcut is **ANANSI Preview R17**.

## Launch

Run **Start ANANSI Preview.cmd**. It refuses to start if the executable has not been built and otherwise opens the preview window against the `r17` data root above.

## What to test

1. **Basics.** Complete onboarding and create a workspace. Save a memory, revise it, inspect its history, and search for the revised wording. Close and reopen the preview; your data is retained.
2. **Projects and artifacts.** Open Projects, create a project, add an artifact, revise it, and inspect both versions. Archive the project and confirm it refuses further artifact writes.
3. **Idea generation.** This needs a local model runtime that **you** have already installed (for example Ollama with a model you already pulled). The preview never downloads a model, asks for cloud credentials, or makes a paid call. Open **AI Providers**, configure the local runtime, and leave the Zero-Cost Lock on. In a project, fill the Idea generation brief, tick one or more context memories (only ticked memories are sent — check the "View what ANANSI sent" disclosure), and generate. You should see 5–10 concepts; save the set as an artifact.
4. **Comparison, evaluation, rubric history.** Open the saved idea set, tick up to three concepts, and assess them under the standard rubric (five dimensions; scores are yours, 1–5 with a rationale — a dimension you skip reads **not assessed**, never zero). Save the evaluation and reopen it. Then edit a rubric weight, save the rubric as a new version, and save a second evaluation: the first evaluation still shows the rubric exactly as originally used (each evaluation embeds its rubric snapshot). Evaluations are your recorded judgement, **not market evidence** — the app labels them that way.
5. **Opportunity discovery.** In a project, open **Opportunity discovery**. Type a problem (or paste a customer complaint), add a second source row as a *Document excerpt* with a pasted paragraph, and tick one memory under *Approved memory sources*. The review list shows exactly those sources — nothing else is sent, and nothing is scanned or fetched. With your local model configured and the Zero-Cost Lock on, choose **Discover opportunities**: each result names the sources it derives from, and *Explore* shows its assumptions and the validation it requires. These are hypotheses grounded in your supplied material, **not** researched facts. Save the set as an artifact, open it from the Artifacts list, and score it with the same **Compare & evaluate** panel used for idea sets.
6. **Backup and fresh restore.** Open **Backups**, tick the plaintext consent, and **Create backup**. In **Jobs and export**, download the completed backup file. Back in **Backups**, import the saved file, **Preview restore**, and restore it into a fresh workspace under a new name. Open the restored workspace: the project, its artifacts, the idea and opportunity sets, the rubric and the evaluations are all there — and the restored evaluation cites the **restored** idea set and rubric (references are remapped at restore; the originals are untouched). Restoring the same backup artifact a second time is refused as a conflict rather than duplicating data.
7. **Exact-revision provenance (new in R17).** Before backing up, note a memory you used as an opportunity source, then *revise that memory* so its current text differs. Back up and restore as in step 6. In the restored workspace, open the opportunity set: its memory source resolves to the revision that existed when you generated the set — the restored memory's history shows that same earlier text, not the later edit. (The restored set pins the restored counterpart of the exact generation-time revision.)
8. **Restart.** Close and reopen the preview once more and reopen both the original and restored workspaces; everything persists.

## Verification status of this checkpoint

- **Verified on Linux** (preparing machine): the underlying code is exactly the R17 repair checkpoint, whose battery is recorded in `docs/BUILD_STATUS.md` and `docs/handoffs/M6-r17-repair-addendum.md` (core suite 210/214 with only the 4 environmental live-Ollama failures; the restore, backup, evaluation, artifact, idea and vault suites all pass). For this preview branch, documentation validation and the browser workflow subset (app checkpoint, integrated workflow, opportunity discovery end to end against a synthetic loopback provider) were re-run on the final tree; see the top `docs/BUILD_STATUS.md` section.
- **Verified on Windows by the owner**: for R17 — `opportunities_generation` 6/6, opportunity unit tests 5/5, `opportunities_restore` 1/1, UI/contracts 51/51, TypeScript, strict Clippy and documentation; for the R17 repair — restore regressions 3/3, opportunity unit tests 5/5, strict all-target core Clippy and documentation; `vault_increment2` 9/9 including both crash matrices; and the earlier R16/R15 passes. The R16 preview itself was owner-tested and reported good. This is not full application or production certification.
- **Pending Windows verification**: the Windows build and launch of *this* preview checkpoint (`codex/local-testing-r17`). A Linux pass does not close it.
- **Production gates still open** (unchanged): Windows native/manual acceptance (real CredUI prompt and OS keyring), live-provider verification, the G1 external audit, and remote CI. Vault/native credential dialogs and real cloud providers keep their separate acceptance gates. No real model, cloud or credential-store operation is performed automatically by the launcher.
