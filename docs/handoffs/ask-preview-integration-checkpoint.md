# Ask verified preview integration checkpoint (2026-10-30)

Baseline: `codex/review-ask-windows-findings` at
`532afccf9333522c3e15c97cd42105c71807bf35`, including the
integrator checkpoint `4ef90406d` (native Windows verify-only
permissions, zero helper launches on a clean Ask) and the
incoming findings repair. This branch
(`muse/ask-verified-integration`) is the integration
checkpoint; it does not replace, launch, or modify the
owner's running preview or any existing preview branch.

## What is integrated

- Verified Ask retrieval: internal ranked candidate path,
  eligibility before the candidate cut, best-chunk-first
  compilation, 4,000-byte evidence cap.
- Windows permission verification: native owner/protected-DACL
  checks for verify-only root/catalog paths, operation
  batches, and read-time file/ancestor rechecks. Permission
  creation/repair still uses the original script. No ACL cache.
- Linux regressions on this branch (see the delivery report):
  focused suites, strict Clippy, formatting, contracts,
  TypeScript, docs, API/web builds, browser Ask, and the full
  core suite — with the same 5 known failures as the previous
  round (4 live-Ollama environment, 1 pre-existing benchmark
  A6) and no new failures.
- The local paired quality comparison is NOT integrated as a
  result: it is frozen and unmeasured (no local model
  installed; see `ask-local-quality-acceptance.md`). No
  answer-quality or token-saving claim is made from this
  checkpoint.

## Isolated integration steps (Windows, reviewer/owner only)

1. Check out this branch into a separate directory. Do not
   check it out over the running preview.
2. Build (PowerShell):
   ```powershell
   pnpm install --frozen-lockfile --ignore-scripts
   pnpm build:web
   node scripts/cargo.mjs build -p anansi-desktop --features custom-protocol --locked
   ```
   Place the executable at
   `E:\Anansi\build-cache\preview-ask-verified\debug\anansi-desktop.exe`
   (or edit the launcher's `PREVIEW_EXE` path to match).
3. Run `Start ANANSI Preview Ask Verified.cmd` from this
   folder. It refuses to start if the executable has not been
   built.

## Data isolation (preserved from the existing launchers)

- The launcher sets
  `ANANSI_STORAGE_ROOT=%LOCALAPPDATA%\ANANSI-Preview\ask-verified`,
  a fresh root separate from any existing catalog and from
  the R11, R16, R17, and M7 preview roots. The preview never
  points at existing data automatically.
- Closing and reopening retains this root's data. Do not
  point an older build at it (the registry uses schema 10).
- Test credential/provider environment variables are cleared
  by the launcher, as in the existing preview launchers.
- Use synthetic/non-sensitive content during acceptance.

## What to test (reviewer)

1. Basics: onboarding, workspace, save/revise/search a
   memory, close and reopen (data retained).
2. Ask: a question with a known fact in a saved memory
   (grounded answer with citations), and a question with no
   supporting memory (honest insufficient-evidence response,
   no fabricated answer).
3. Permissions: widening a file's ACL after an Ask must fail
   the next read (refusal test, no longer ignored); a clean
   Ask launches no permission-helper processes.
4. The frozen local quality comparison, once a local model is
   installed (see `ask-local-quality-acceptance.md`); do not
   substitute cloud inference or a different prompt set.

## Explicitly not claimed

- No Windows/native acceptance is claimed from Linux
  evidence. The Windows build, launch, and native-dialog
  behavior of this checkpoint are the reviewer's verification.
- No preview promotion, master merge, remote CI, production
  gate, G1 audit, or generated-execution certification.
- No model-answer score, token saving, or refusal claim
  beyond the recorded evidence: retrieval completeness and
  Windows evidence checks only.
