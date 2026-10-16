# M1 — Windows desktop .exe and web UI foundation

Status: approved scope under the user's 2026-10-16 continuation instruction after verified M0. M1.0 through M1.4 are verified complete. M1.5 development installer is implemented; certification/publication is recorded in build status. Follow one task at a time. Stop at the end of M1 and do not begin M2 automatically; its detailed planning/approval gate is separate.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 3–8, 63–64, 70–71, 74–75, 82–84; [roadmap](../docs/ROADMAP.md). M0 completion tag is m0-foundation-verified at 268533e. Preserve all source scope and M0 recovery/CI controls.

## Objectives and milestone workflow

A user can install a development Windows NSIS .exe or open the local browser edition, review privacy and skip model/provider setup, create a local workspace, close/reopen it, navigate the implemented workspace/settings views and persist a setting. Both editions invoke the same Rust domain rules and validate the same contracts. No inference, cloud transmission, memory CRUD, arbitrary filesystem commands or generated-product execution is part of M1.

The development installer is distinct from M15 signing/update/release certification. Browser means a local authenticated API client, not a publicly hosted backend with native privileges. No UI claims that future modules work.

## Task sequence and acceptance before coding

### M1.0 — Detailed plan and repeatable readiness audit

Create scripts/doctor.mjs, tests/doctor.test.mjs and docs/evidence/M1.md; update this plan, architecture/decisions/development/status/requirements/state and root scripts. Read actual tool versions and inspect Windows C++ installation metadata, SDK headers/libraries and WebView2 directories without modifying the machine. Foundation mode reports independent Node/pnpm/Git availability; native mode exits nonzero when required Windows prerequisites are absent. Detected prerequisites must never imply a successful build. Tests inject synthetic probe reports for missing tools, wrong versions, platform/MSVC mismatch and Node engine floors. Retain root foundation and backup tests, protected CI publication and independent source checkpoint.

Acceptance: both modes execute; real gaps and remediation are recorded, classifier tests and existing suites pass, full M1 scope is assigned below. Missing native prerequisites block M1.1 completion, not this truthful audit. No native install or product implementation is claimed by M1.0.

### M1.1 — Native toolchain installation and compile/link verification

Dependencies: M1.0. Inventory existing installations again; install or locate Rust stable MSVC, Windows C++ build tools and SDK through official Tauri/Rust/Microsoft sources, and verify publishers before running downloaded installers. Record precise versions and commands. Use an existing authenticated/admin context where available; do not change security software, uninstall existing tools or reboot automatically. Record any genuine permission/install blocker while continuing independent checks. Do not infer a working linker from registry entries.

Files: rust-toolchain.toml (exact verified stable channel, rustfmt/clippy), docs/evidence/M1.md and development/state records. Compile and link a tiny synthetic Rust program in a unique ignored temporary directory using the MSVC host; keep the fixture/logs as evidence. Do not add fake product packages to make this pass. Verify WebView2 version evidence and reserve actual embedding/launch for M1.2/M1.5.

Acceptance: pnpm doctor:native returns 0, rustc -vV identifies windows-msvc, cargo --version succeeds, a real MSVC compile/link exits 0 and fixture execution returns its expected output under the authorized development test. No application completion claim. Do not create Cargo.lock before a real workspace/package exists in M1.2. CI tool versions and local versions must be recorded, not assumed equal.

### M1.2 — Shared readiness vertical slice across both delivery adapters

Dependencies: verified M1.1 and reviewed security/contract design below. Files: Cargo.toml/Cargo.lock, crates/core/{Cargo.toml,src/lib.rs,src/readiness.rs}, crates/api/{Cargo.toml,src/main.rs,src/auth.rs,tests/http_boundary.rs}, crates/contracts/{Cargo.toml,src/main.rs}, packages/contracts/{package.json,schemas/readiness.json,src/generated.ts,src/client.ts}, packages/ui/{package.json,src/App.tsx,src/tokens.css}, apps/web/{package.json,index.html,src/main.tsx,vite.config.ts}, apps/desktop/{package.json,src/main.tsx,src-tauri/Cargo.toml,src-tauri/build.rs,src-tauri/src/lib.rs,src-tauri/src/main.rs,src-tauri/tauri.conf.json,src-tauri/capabilities/default.json}, tsconfig.json, tests/e2e/readiness.spec.ts, playwright.config.ts and root scripts/CI. Add exact package configs and lockfile resolutions rather than hand-writing a lockfile. Icons must be real authored assets or approved generated icons, not missing references.

Contract decision: Rust serde DTOs in core are authoritative; derive JSON Schema and emit deterministic checked-in TypeScript declarations with a tracked generation command. Choose and verify stable schema/binding tools at implementation, record versions, check generated-file drift in CI. Validate external responses at runtime against the same schema; no separately maintained browser business rules. Rejected fields, failures and error codes must have parity fixtures. Readiness reports real app/API/core versions and availability, not hardcoded claims about storage/models.

Browser boundary: API binds only 127.0.0.1; require a per-launch high-entropy bearer credential for every domain endpoint and explicit Origin/Host allowlists. No wildcard CORS, DNS-rebinding host acceptance or unauthenticated data routes. Implement and test a trusted local pairing flow: deliver the credential via a per-user permission-restricted local pairing file chosen by the launcher, enter it explicitly into browser memory, never URL query/fragment, logs, localStorage or repository files. Verify restrictive Windows ACL/POSIX mode creation; fail closed when pairing cannot be protected. A browser client cannot choose arbitrary database paths or invoke native commands. Tauri invoke routes call core directly and need no browser HTTP credential. Tauri capability/CSP allowlists must exclude shell/process/unscoped filesystem plugins and remote pages invoking privileged commands.

Acceptance: launch real Tauri and browser clients; show actual shared core readiness from their real transports. Test missing/wrong credentials, disallowed host/origin, malformed DTOs, unavailable API, no credential persistence, schema parity and deterministic generation. Both clients expose a clear recoverable unavailable state. A fake readiness response or mock-only UI does not pass. Record browser/desktop observations; use synthetic local data only. No workspace/provider/memory claims yet.

### M1.3 — SQLite workspace create/reopen and durable settings

Dependencies: M1.2. Files: crates/core/src/{workspace.rs,storage.rs,settings.rs}, crates/core/migrations/0001_workspace.sql, crates/core/tests/workspace_roundtrip.rs; extend core DTO/schema, native commands/API routes and shared UI; tests/e2e/workspace.spec.ts.

Design: SQLite versioned forward migration with transactional application, foreign_keys=ON, WAL and busy timeout. Workspace metadata contains stable ID/name/schema version and settings; memory schema belongs to M2. Establish exactly one trusted configured storage root outside source checkout. Desktop chooser must have scoped permission; browser sends a workspace name/ID, never an absolute path. Reject invalid names/traversal, reparse escapes and duplicate/conflicting locations. No destructive reset or overwrite of an existing unknown database. Explicit user action creates workspace; each operation checks the configured root and permissions in Rust. Database schema version mismatch and migration failure must be recoverable without deleting originals.

Acceptance: desktop and browser create and reopen the same workspace via their adapters; workspace IDs/settings survive application/API restart. Test transactional migration rollback, foreign keys, WAL, duplicate creation, invalid names/paths, concurrent opens, read-only directory and corrupt/newer-schema failure. Persist one meaningful setting (for example reduced motion/theme preference) with validation and identical behavior in both clients. No memory CRUD or encrypted-vault claim.

Implementation interpretation: [ADR-0013](../docs/DECISIONS.md#adr-0013-managed-workspace-catalog-and-scoped-id-chooser) records one private SQLite catalog, a managed-ID chooser behind allow-workspaces, trusted process-level root selection and conservative recovery. Additional tracked files are crates/core/src/protect.ps1, packages/ui/src/WorkspacePanel.tsx, crates/api/tests/workspaces.rs, apps/desktop/src-tauri/permissions/workspaces.toml, scripts/verify-workspace.mjs and docs/evidence/M1-3.md. The native/browser verifier executes both adapters and process restarts against isolated synthetic storage. This does not add an arbitrary OS-folder picker or memory schema.

### M1.4 — First-run, navigation and shared accessible design

Dependencies: M1.3. Files: packages/ui/src/{Onboarding.tsx,Navigation.tsx,Settings.tsx,Workspace.tsx,tokens.css}, corresponding component tests, tests/e2e/onboarding.spec.ts and responsive/keyboard fixtures.

Onboarding retains five intent choices from section 7, local workspace creation/privacy review and provider setup skip. Intent is changeable, not a permanent mode. Display actual hardware/model setup only when supported: M3 owns full hardware/runtime detection and model recommendations; M5 owns cloud-provider connections. Track these in the matrix rather than showing fabricated success. First-run is usable offline. Navigation follows section 8's groups but shows a small beginner set; future modules are explicitly unavailable/explanatory, never working-looking stub flows. GHOST-PROMPT remains an optional future editor mode, not a standalone capture feature.

Use graphite/warm-white/violet shared tokens, strong contrast/typography, restrained motion, visible focus and keyboard semantics. Native/web share the same components. No external fonts or telemetry required for basic offline use.

Acceptance: a new user chooses an intent, creates a workspace, reviews privacy, skips providers, enters workspace/settings and changes/reloads a preference in both editions. Keyboard-only and screen-reader labels, reduced motion, narrow/wide layouts and error recovery verified in the actual UI. Show current action/result/next step, with no fake dashboard data. Run component/E2E tests and record visual inspection evidence.

Implementation details: schema migration crates/core/migrations/0002_onboarding.sql persists typed intent/onboarding preferences through the existing scoped settings command and generated contracts. Split the earlier WorkspacePanel into the planned components; keep root commands and native/browser verifiers current. Component tests live in packages/ui/tests/components.test.tsx to resolve the UI package's existing React dependencies. [ADR-0014](../docs/DECISIONS.md#adr-0014-durable-first-run-preferences-and-small-shared-navigation) records the migration, navigation and consent boundaries; [M1.4 evidence](../docs/evidence/M1-4.md) records actual outcomes. Full hardware/model/provider onboarding is retained under M3/M5.

### M1.5 — Development Windows installer and milestone certification

Dependencies: all preceding tasks. Files: installer configuration in apps/desktop/src-tauri/tauri.conf.json, CI Windows build/artifact jobs, docs/evidence/M1.md, docs/DEVELOPMENT.md and recovery/status/state records. Preserve both existing Foundation required check names; add real Rust/frontend build jobs rather than replacing checks with empty recursion. Pin new CI actions by verified SHA and keep least privilege. Rust/API core tests run on Windows/Linux; native packaging is Windows-only until the later macOS milestone.

Acceptance: generate actual NSIS .exe, verify artifact checksum, install to a separate test location/user profile, launch, perform workspace/settings flow, quit/relaunch and verify persistence. Inspect uninstallation without deleting private workspace data. Document unsigned development trust prompts honestly; signed distribution, updater and clean-machine release certification remain mandatory M15 work. Confirm browser production build plus authenticated local API workflow, security tests, offline behavior, cross-adapter contracts and all regressions. No claim of clean-machine/signed certification from a local development install.

Tracked support: scripts/installer-artifact.mjs, scripts/installer-checksums.mjs, scripts/verify-installer.ps1, scripts/verify-ports.mjs, tests/installer.test.mjs, tests/installer-preflight.Tests.ps1 and docs/evidence/M1-5.md. Keep build:desktop unbundled; build:installer creates release NSIS/checksum evidence. CI packages release artifacts, tests install/uninstall with explicit -SkipUi on elevated hosts and retains its separate actual native UI checks. Local non-elevated installed-release UI verification is mandatory. See [ADR-0015](../docs/DECISIONS.md#adr-0015-per-user-unsigned-nsis-development-packaging) and [evidence](../docs/evidence/M1-5.md).

## Required commands and evidence

Available now: pnpm doctor:toolchain, pnpm doctor:native, pnpm check, pnpm test, pnpm install --frozen-lockfile --ignore-scripts, git diff --check, git hash-object docs/MASTER_SPEC.md. Native mode failure is expected until prerequisites are installed; never report that failure as a passed native build. The doctor never installs anything.

M1.2 now supplies these real target commands (installer certification remains M1.5): cargo test --workspace --locked, cargo fmt --check, cargo clippy --workspace --all-targets --locked -- -D warnings, pnpm contracts:generate, pnpm contracts:check, pnpm typecheck, pnpm test:ui, pnpm test:e2e, pnpm dev:web, pnpm dev:desktop, pnpm build:web, pnpm build:desktop. Scope root build to actual targets and keep installer execution an explicit test action. API secrets and workspace paths must not appear in test logs.

Each task records exact commands, exit codes, dependency versions, real workflow observations, CI run URLs, checkpoint and remaining failures in docs/evidence/M1.md. Native/window UI inspection may require the supported browser/computer-use tools; no generated screenshots substitute for actual behavior.

## Scope/requirement preservation

| Source requirement | M1 owner / later boundary |
|---|---|
| Sections 3–5 local-first/offline/recovery/testable stack | All tasks; M0 checks remain required; no cloud or paid requests |
| Section 6 common services and interfaces | Core/transport separation in M1.2; workspace/settings in M1.3; all other services retained in their roadmap milestones |
| Section 7 welcome, five intents, privacy, skip providers | M1.4; local hardware/runtime setup in M3, cloud/OAuth in M5; neither prerequisite to first use |
| Section 8 all navigation groups and future modules | M1.4; product tools retain later milestone ownership, no stub completion |
| Sections 63–64 simple actions, hierarchy, readable identity | M1.2/M1.4; future tool actions appear only when implemented |
| Sections 70–71 Windows, permissions, security, offline | M1.1–M1.5; macOS in M14, signed updates/release certification in M15; authentication/native isolation enforced now |
| Sections 74–75 privacy classes and encryption | No private data transmission in M1; do not misrepresent app lock as disk encryption; full memory privacy/vault in M2/M15 |
| Section 82 M1 Tauri/React/design/SQLite/workspace/navigation/settings | M1.2 through M1.5; no item omitted |
| Browser edition and installer, AN-001/002/003/018/020/023 | M1.2–M1.5 evidence; full AN-001/002 release/parity coverage remains open until their later gates |

## Recovery, checkpoints and Definition of Done

Use codex/ task branches and small commits. Wait for actual required Windows/Linux checks on each exact SHA before fast-forwarding protected master; never bypass protection or rewrite M0 tag. Capture independent source bundles outside checkout after clean checkpoints and compare restored source as in M0. Record prior verified SHA in metadata to avoid self-reference.

Before workspace migration changes, preserve original test/user databases outside Git; never auto-reset/delete a damaged workspace. On failed tools/tests, preserve logs, repair current scope, and leave unavailable features explicitly blocked. No real private memories, credentials or paid inference in tests. Existing master specification must stay byte-equivalent after Git newline normalization.

M1 is done only when all task criteria have recorded workflow and test evidence, both delivery editions use the same domain/contracts, native storage is isolated from browser capabilities, real NSIS development installation/persistence is verified, CI passes, status/coverage agree and a new independent source checkpoint exists. Tag M1 only after those conditions. Stop before M2; unresolved installer/runtime/tool permissions remain blockers, not successful completion.
