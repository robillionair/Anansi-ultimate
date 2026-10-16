# M0 — Source protection, repository integrity and recovery

Status: approved implementation plan under the user's 2026-10-16 reconciliation instruction. Planning is complete; M0 implementation is not complete. This approval defines scope, not evidence of passed acceptance tests. Do not start M1 automatically.

## Objectives and scope

Establish a recoverable, verifiable development repository before substantial product functionality (master sections 4, 5, 6, 71, 76, 82, 83, 88). Preserve the existing Git history, master specification, identity and remote. Prepare shared desktop/browser workspace boundaries without creating placeholder product features. Finish one numbered task at a time and checkpoint it; first unfinished task is M0.1.

## Dependencies and preflight

- Read AGENTS.md, master specification, roadmap, coverage, architecture, status, decisions and recovery guide.
- Confirm origin is the authorized robillionair/Anansi-ultimate remote and master is the default branch. Inspect local changes before fast-forward pull; never reset or force-push to resolve divergence.
- Inventory Git, Node, pnpm/Corepack, Rust/Cargo, Windows C++ build tools, Windows SDK and WebView2 using actual versions. Recheck official tool documentation when choosing/installing versions. Do not infer availability from an earlier session. Rust/native tools are M1 prerequisites; record a missing tool with remediation rather than claiming a desktop build.
- Existing package.json pins pnpm 10.0.0 but this is not a verified compatibility claim. Confirm or explicitly revise this pin in a recorded decision. No provider accounts, credentials, model downloads or inference are needed.
- GitHub branch protection requires suitable account rights; use existing authentication only. A permissions failure is a recorded open acceptance item, not a reason to fabricate protection or block independent local work.

## Exact deliverables

Create during implementation:

| Path | Purpose |
|---|---|
| .gitignore | Exclude dependencies, target/dist, local workspaces, databases, secrets, credentials and local backup payloads while preserving safe fixtures/templates |
| .editorconfig | Consistent UTF-8, newline and indentation policy |
| .gitattributes | Explicit text/binary handling; avoid repository-wide destructive renormalization |
| pnpm-lock.yaml | Real resolved lockfile from the selected package manager, never hand-written |
| .github/workflows/ci.yml | Linux and Windows foundation checks using pinned action revisions and least privilege |
| scripts/validate-docs.mjs | Dependency-free planning/link/coverage validator promoted from reconciliation prototype after review |
| tests/docs-validation.test.mjs | Positive and negative fixtures: missing target, duplicate/missing section, missing owner/plan, invalid state, out-of-root path |
| scripts/backup-source.ps1 | Explicit source bundle creation/verification outside checkout; no secrets or workspace content |
| tests/backup-source.Tests.ps1 | Bundle/restore smoke and invalid destination/failure coverage; choose documented runner in M0.2 |
| docs/DEVELOPMENT.md | Exact setup/check commands and desktop/browser prerequisite inventory |
| docs/evidence/M0.md | Commands, outcomes, CI run URLs, protection state, backup/restore evidence and open failures |
| project-state.json | Machine-readable state: schemaVersion, activeMilestone, activeTask, plan, implementationStatus, lastVerifiedCommit, evidence, blockers |

Modify package.json (explicit check:docs/test:docs/backup:source commands; make root check/test/build honest for a foundation with no product packages), README.md, pnpm-workspace.yaml only as needed, docs/ARCHITECTURE.md, docs/BUILD_STATUS.md, docs/DECISIONS.md, docs/RECOVERY.md, docs/RECOVERY_AND_BACKUP.md and the requirements/coverage records. Retain docs/IMPLEMENTATION_ROADMAP.md as a canonical-roadmap entrypoint.

Future directory ownership must be documented in DEVELOPMENT and ARCHITECTURE: apps/desktop, apps/web, crates/core, crates/api, packages/contracts, packages/ui, tests. Do not add fake Rust/React packages, a Cargo.lock without a Rust package, or empty “working” application screens. Actual Tauri/React scaffolding and native lockfile belong to M1.

## Development tasks and acceptance criteria

### M0.1 — Repository hygiene and development contract

1. Verify clean/current history and specification hash; capture environment versions and current script behavior.
2. Add ignore/editor/Git policies, development instructions and project-state.json. Preserve master content and existing default branch name.
3. Document command/domain/DTO ownership, authenticated browser transport and native capability isolation. Do not invent provider APIs, database schemas or a vector implementation.
4. Install/resolve only necessary foundation dependencies after checking versions, generate pnpm-lock.yaml and test frozen resolution. If zero dependencies, use the manager's valid empty-workspace lockfile rather than product packages.

Acceptance: actual tooling gaps documented; secrets/data excluded by git check-ignore fixtures; expected source/fixtures remain trackable; state is truthful; reproducible lockfile generated; no product implementation. Commit only reviewed files.

### M0.2 — Executable validation and CI

1. Implement dependency-free documentation validator and fixture tests. Validate 90 master-section anchors, canonical milestone plans, coverage owners, local links, required state keys and referenced evidence. Handle code fences and anchors deliberately; do not equate text coverage with feature verification.
2. Add positive/negative tests for ignore/backup behavior where appropriate. Use temporary fixtures, no real memories or credentials.
3. Wire explicit root scripts. Existing recursive pnpm commands over no packages are not product build/test evidence. Report product build as not available until M1.
4. Create least-privilege CI for clean checkout, pinned runtime/package-manager setup, frozen dependency install, docs tests and whitespace checks on Windows/Linux. Add Rust/frontend build jobs only once real targets exist in M1.
5. Push authorized checkpoint, inspect the actual workflow run and record its URL/conclusion. Configure required checks/default-branch protection if authorized access permits; otherwise retain an explicit blocked acceptance item and exact manual remediation. No branch rename, bypass or force push.

Acceptance: tests pass locally on the available platform; both configured CI runners actually pass; negative fixtures fail as expected; no success inferred merely from workflow YAML; branch protection verified, or M0 remains incomplete with a documented external blocker.

### M0.3 — Independent source backup and restoration

1. Implement source backup script taking an explicit absolute destination outside checkout and outside .git. Verify resolved paths; refuse overwrite, invalid destinations and missing Git history. Record safe quoting and error propagation. Git bundle includes committed source only; dirty source must be committed or exported separately with an explicit warning and recovery instructions.
2. Create an independent local bundle with all required branch/tag refs, verify it, clone into a unique temporary sibling directory and compare restored HEAD/tree to the checkpoint. Run foundation checks in restored checkout and confirm no credential/database payload is tracked. Avoid recursive deletion; keep drill output for inspection or remove only a verified task-owned temporary path.
3. Verify the pushed remote commit matches local HEAD. Document that an on-device backup does not protect against device loss; remote Git plus independent local copy protect source, not private application data.
4. Document fresh remote clone and bundle clone recovery, uncommitted patch recovery, failed CI recovery and divergence handling. Add an annotated m0-foundation-verified tag only after all M0 criteria pass; never move an existing tag.

Acceptance: real local restore reproduces source commit and passes checks; remote matches; recovery steps and backup location/hash are recorded without secrets; no claim of workspace or credential backup.

## Required tests and verification commands

The following are the intended M0 implementation commands; scripts not yet created are not available today. Run sequentially and check each exit status. Record versions and logs in docs/evidence/M0.md.

~~~powershell
git status --short
git remote -v
git log -5 --oneline
git fsck --full
git hash-object docs/MASTER_SPEC.md
node --version
pnpm --version
rustc --version
cargo --version
pnpm install --frozen-lockfile
pnpm run check:docs
pnpm run test:docs
git diff --check
git check-ignore --no-index .env node_modules/example target/example dist/example
pwsh -NoProfile -File scripts/backup-source.ps1 -Destination <absolute-independent-new-bundle-path>
git bundle verify <absolute-independent-new-bundle-path>
git clone <absolute-independent-new-bundle-path> <unique-restore-path>
git -C <unique-restore-path> rev-parse HEAD
git ls-remote origin refs/heads/master
~~~

Also run the backup test runner selected/documented in M0.2, frozen install/docs tests inside restored checkout, inspect CI results for both OSes, and read actual branch-protection settings. Test missing/corrupt bundle, dirty worktree and refused in-repo destination. Record unavailable prerequisites as failures/blockers; do not install native tools or run generated software implicitly. No application launch applies to M0 because it creates no application; M1 requires actual desktop/browser launch and installation evidence.

## Recovery and checkpoint rules

Before edits inspect status and preserve unrelated work. On failed checks repair the current task only. On divergence fetch/inspect and reconcile without reset, force push, history rewrite or deleting user files. On failed restore keep original repository/bundle intact, record failure and retry in a new temporary directory. Follow RECOVERY.md on every resume.

Use the configured Robin-kevin Vettik repository identity; do not request it again. Small commits after passing task checks, with exact staged paths and no secrets. Update BUILD_STATUS and project-state with the prior verified commit and evidence; a following metadata commit may record the new implementation checkpoint to avoid self-referential hashes. Verify remote SHA after authorized push. Never claim a push or CI success from local commit alone.

## Definition of Done

All M0.1–M0.3 acceptance items have command/workflow evidence; required CI and branch protection are verified; source restore succeeds from an independent local bundle and remote recovery is documented; lockfile is reproducible; status/requirements/state agree; every remaining requirement remains assigned; tagged checkpoint is pushed and recorded. If external permissions prevent a required item, mark it blocked and M0 incomplete. Stop and report built files, actual tests, failures/not-run checks, risks, checkpoint/backup status and next task. Do not scaffold M1 automatically.
