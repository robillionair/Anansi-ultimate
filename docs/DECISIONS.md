# ANANSI Ultimate — Architecture Decisions

## ADR-0001: Shared domain with two delivery adapters

- **Decision:** Desktop and browser clients share contracts, domain operations, models, and UI primitives; they differ only at capability adapters and transport boundaries.
- **Reason:** Prevents divergent business rules and preserves local/native capability separation.
- **Status:** accepted as a target architecture; validate during M1.

## ADR-0002: Local-first and explicit privilege boundaries

- **Decision:** Local persistence is the default. Native filesystem/process access, secrets, MCP, sealed memories, and paid inference require explicit authorization and permission checks.
- **Reason:** Protects ownership and avoids treating a hosted browser as trusted native code.
- **Status:** accepted; detailed feature policy must implement restored sections 71–76.

## ADR-0003: Do not invent missing product requirements

- **Decision:** Track the absent master specification as a blocker and keep only the orchestration brief's named areas as provisional matrix anchors.
- **Reason:** A plausible but fabricated spec would create false traceability and irreversible implementation drift.
- **Status:** historical; resolved by restoration in 3f7571a.

## ADR-0004: Restored specification requires plan reconciliation

- **Observation (2026-10-16):** Pulling commit `3f7571a` restored the master specification. ADR-0003's missing-source blocker is resolved, but its provisional requirements have not yet been fully mapped.
- **Conflict:** Master specification section 82 defines M0–M15, while docs/ROADMAP.md defines M0–M9 with different milestone scopes. Foundation and runnable-shell plans reference absent PHASE-000 and PHASE-001 detailed files. No explicit task-level approval is recorded.
- **Decision:** Follow docs/RECOVERY.md step 6: stop product implementation, record the conflict, and require roadmap reconciliation plus restoration of the approved detailed plan before selecting implementation scope. The current continuation request does not specify which conflicting milestone definition to adopt.
- **Status:** resolved by ADR-0005; no implementation milestone has been completed.


## ADR-0005: Canonical sequence and bounded M0 approval

- **Authority:** User instruction “UNBLOCK DEVELOPMENT”, 2026-10-16, authorizes milestone reconciliation, full M0 planning and documentation checkpoint/push; it explicitly excludes product implementation in this task.
- **Decision:** Retain master section 82's M0–M15 IDs. The roadmap records every previous milestone and legacy plan mapping. The user's 13 requested groups are all represented; semantic intelligence, integrated lifecycle and macOS/Core remain explicit subdivisions.
- **Resolution:** M1 development Windows .exe and web foundation precede M15 signed release certification; SQLite/workspace/settings stay in M1. Search is split into M3 lexical/local AI and M4 semantic intelligence. Minimum privacy/context/cost rules precede M3 inference, with full compiler/Matrix in M4/M5. Security is enforced throughout. Legacy plan filenames are stable entrypoints, not milestone IDs.
- **Scope ambiguities:** Big Brain lacks defined behavior and stays blocked on M4 design; Forge's single-template MVP does not exhaust section 28 product types, which M7 must retain as follow-up slices. Enterprise hosting is future potential scope under section 81 and remains explicitly deferred pending isolation/security design in M14/M15. No source requirement is removed or represented as tested.
- **Repository contradictions:** Empty-repository wording is historical. Existing master/default branch and source history are preserved. Default branch protection applies to master; rename is unnecessary. Required alternate documentation filenames now link to canonical records. Missing detailed M0 plan restored; M1 target restored as an explicitly incomplete design task.
- **Approval boundary:** PHASE-000-foundation.md is the approved M0 implementation scope. First task M0.1 may begin next session. Other milestone files are tracked planning tasks requiring detailed design before coding. Do not start M1 automatically.
- **Status:** accepted planning baseline; no product or M0 implementation acceptance criterion is asserted complete.

## ADR-0006: M0.1 repository policies and measured package manager

- **Decision:** Revise packageManager from pnpm 10.0.0 to exactly 11.25.0, matching the actual environment command and registry-verified package (Node >=22.13). Both available Node runtimes meet the metadata requirement. Generate a real dependency-free workspace lockfile and verify frozen installation; no app packages or dependencies are invented.
- **Reason:** The environment's pnpm wrapper ignores the earlier pin and executes 11.25.0. Explicit version verification and exact npm-exec fallback are documented to avoid pretending the 10.0.0 pin was honored. No global installation is changed. This is a measured foundation baseline, not a product compatibility claim.
- **Policies:** UTF-8/LF defaults, CRLF command files and binary attributes; no bulk renormalization of existing files. Ignore runtime data/secrets/builds; preserve source, safe environment examples and synthetic JSON/SQL/Markdown fixtures. Ignore files do not provide secret scanning or encryption.
- **Boundary:** Root script wiring, broader validators, CI/protection and independent backup/restore stay in M0.2/M0.3. This task adds direct Node tests for its own behavior only. Shared Rust core/transport ownership remains as previously approved.
- **Status:** accepted for M0.1; native build prerequisites remain unverified/incomplete for M1. See docs/evidence/M0.md for actual command evidence.

## ADR-0007: Executable foundation checks and CI boundary

- Decision: Promote the planning audit to dependency-free validate-docs, retain its compatibility entrypoint, use Node's built-in test runner and make root commands report their actual scope. Build and source backup intentionally fail with an explanatory message until their own tasks exist.
- Links: inline/image and reference Markdown links, relative files, GitHub-style heading anchors including duplicate headings are checked outside fenced/inline code. Repository paths come from Git's tracked/nonignored inventory; no symlinks are followed. This is a repository documentation convention checker, not a full Markdown renderer or product completion validator.
- CI: Windows/Linux matrix, exact Node/pnpm versions and official action commit pins, least privilege and no product scaffolds. Branch protection will require both actual foundation check names; protection state and CI evidence must be verified separately.
- Scope: backup script/restore drill remain M0.3. Choose dependency-free PowerShell assertions for the future backup test script. No application architecture or privacy capability has changed.

M0.2 external verification completed: both actual runners passed and master protection was applied/read back. Required checks are strict and bound to GitHub Actions; administrators are included, force pushes/deletion denied. No mandatory reviewer count is added for the sole-maintainer repository. Future checkpoints must pass checks before updating master; protection must not be bypassed. See M0 evidence for exact API outcomes and run URL.

## ADR-0008: Independent committed-source bundle recovery

- Decision: backup-source.ps1 requires an explicit absolute .bundle destination with an existing parent outside the checkout, Git dir/common dir and .git ancestry. Reject existing destinations, reparse-point/symlink/junction ancestors, shallow/empty history and any tracked/untracked worktree change. Use native argument arrays, check every Git exit code, stage a unique external bundle, verify it, recheck clean HEAD/refs and use non-overwriting File.Move. Remove only a generated temporary file on failure; never recursively delete directories.
- Scope: include HEAD plus local branches, remote-tracking refs and tags; exclude stash/tool-private refs and raw .git configuration/credentials. Git bundles preserve committed history, not ignored/untracked files, editor buffers, account secrets, application databases or application backups. This is not a secret scanner or a guarantee that old commits never contained private material. Reviewed source history is a prerequisite; Git ignore rules prevent ordinary accidental additions.
- Limits: conservative link rejection means even benign aliases must use their direct paths. User-controlled backup directories must not be concurrently changed; this is a development recovery tool, not an adversarial multi-user filesystem sandbox. On-device independent backup plus authorized remote Git reduces source-loss risk but is not off-device protection for local private data.
- Verification: plain PowerShell synthetic fixture runner checks real Git bundles, restore HEAD/tree, tags/branches, dirty-source refusal, no overwrite, path/link rejection and corrupt/missing bundle handling. Test fixtures are retained outside the repository. Actual source clone/restore and CI outcomes are recorded separately in M0 evidence.
