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

## ADR-0009: M1 authorization, bounded task plan and readiness semantics

- Authority: user continuation after completed M0, with permission to continue until a safe context boundary. M1.0 detailed plan/readiness audit is authorized now; work remains within M1 and task gates. Earlier M0 stop language is historical and superseded by this continuation. No M2 work authorized by inference.
- Decision: break M1 into readiness planning, native toolchain verification, shared readiness transport slice, SQLite workspace/settings, first-run/navigation/design, and actual development installer certification. Full scope remains mapped to master requirements; no mock screen counts as a completed capability.
- Doctor: read-only independent foundation/native detection, exact package-manager check, actual MSVC/SDK file evidence and WebView2 directory evidence. Detected prerequisites never prove compile/link/runtime success. Missing tools are a next-task blocker, not a failing documentation plan or permission question.
- Contracts/security: adopt Rust DTOs as authoritative and generate shared schema/TypeScript in M1.2; implement authenticated loopback API and native capability isolation with explicit pairing/ACL tests before exposing browser domain operations. Detailed tool/library versions are verified at implementation; no provider credentials or integrations invented.
- Status: M1.0 plan and doctor implemented; M1.1 native toolchain not ready locally. Machine installations and product implementation have not occurred in this increment.

## ADR-0010: Pinned user-local Rust and explicit native setup permission boundary

- Pin Rust 1.99.1 MSVC with rustfmt/clippy after verifying official stable metadata and rustup checksum. User-local install leaves global PATH unchanged; development doctor checks CARGO_HOME/default user tool paths as a fallback. This changes tool discovery, not native capability exposure.
- Keep MSVC installation explicit: plan mode is read-only, -Install requires an already elevated shell and a valid Microsoft signer, no self-elevation, no security-setting changes, no automatic restart. Current non-admin install refusal and real link.exe failure remain blockers.
- Native verifier compiles/links/runs only a tiny known synthetic test in a unique retained temp directory. A separate Windows CI job proves runner readiness; it cannot certify this machine, Tauri/WebView2 embedding or an installer. No product crates/workspaces created to mask tool gaps.
- M1.1 partial progress is checkpointed honestly. M1.2 requires the local native task gate, which is still unsatisfied.

## ADR-0011: Verify SDK files across registered and standard installation roots

- Observation: the user completed the signed Microsoft tools installation. Actual local native compilation/link/execution passed, but the doctor incorrectly relied on one obsolete SDK root. A failing detector is not evidence of a failed installation.
- Decision: inspect both SDK registry views and canonical Program Files roots; require actual UM/UCRT headers and x64 kernel32/UCRT libraries. Return the concrete root/version as evidence and retain the original registry value for diagnosis. Add a deterministic stale-root/missing-library regression fixture.
- Resolution: local doctor and synthetic MSVC compile/link/run now pass; ADR-0010's installation/linker blockers are resolved. No security boundary or product scope change. M1.1 complete; M1.2 is next, with real Tauri/API/UI verification still required.

## ADR-0012: Shared readiness contract and explicit local pairing

- M1.2 establishes a real Rust core, authenticated Axum loopback adapter and scoped Tauri command. Both use the same serde request validation and error codes; API build version is transport metadata. The browser never receives filesystem/process operations. No database or provider behavior is included.
- Generate JSON Schema (schemars 1.2.3), TypeScript declarations (json-schema-to-typescript 16.0.0) and static Ajv 8.20.0 validators from authoritative Rust DTOs. CI rejects drift. Actual native launch exposed Ajv runtime compilation blocked by CSP; generate validators ahead of time instead of permitting unsafe-eval. No CSP relaxation is authorized by this fix.
- Browser operations require a per-launch OS-random 256-bit bearer and exact loopback Host/Origin. Startup writes the token only after private directory/file permissions are applied and read back. Windows uses a tracked static PowerShell helper importing the system security module explicitly, avoiding inherited module-path incompatibility. This is a trusted startup operation, never a browser capability. Unix uses restrictive modes. Fail closed on permission errors and reject link/reparse ancestors.
- Users manually pair; credentials stay in tab memory, clear from form fields after submission, and are not sent through URLs, persistent browser storage or logs. Graceful shutdown removes its unique session file; forced termination may leave a private stale file, but launch rotates the token. Protection excludes administrators and compromised same-user processes; no claim of a general hostile-filesystem sandbox.
- Tauri local main-window capability and CSP allow readiness IPC only, with no remote grants or shell/filesystem plugins. Original icons are reproducible source assets. Windows-only desktop dependencies permit shared core/API tests on Linux without implying a Linux desktop product.
- Root build now builds the actual web target; build:desktop builds the Windows application without an installer. M1.3/M1.4/M1.5 remain separate acceptance gates. Test-only WebView2 CDP is scoped to verifier child environment, absent from application launch configuration.
