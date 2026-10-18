# M2 — Memory OS

Status: the user accepted M2.0 and authorized M2.1. M2.1 acceptance, all three implementation CI jobs, protected source publication and independent restore passed at 39b0e5a; see docs/evidence/M2-1.md. M2.2 acceptance, all three implementation CI jobs, protected master publication and independent restore passed at aac601d; see docs/evidence/M2-2.md. M2.3 initial implementation was recorded at 82789d3; its acceptance gaps are repaired and locally verified at 5b83338 with both required foundation CI jobs passed. The separate native CI job status is linked in docs/evidence/M2-3.md; actual local native/browser restart verifiers passed. Published-source recovery follows the checkpoint rules below. User continuation now authorizes M2.4 jobs/export; local acceptance passed at 8f7ff87 with all actual workflows; CI/publication/source recovery are recorded in [M2.4 evidence](../docs/evidence/M2-4.md). M2.5 local acceptance passed at cfc28af; see [M2.5 evidence](../docs/evidence/M2-5.md) and its final publication/source ledger. Stop after M2.5. M2.6 is next: reconcile the user-supplied conditional approval against audited design and platform evidence; no vault/crypto implementation was started.

Gate identifier: M2.0 — detailed planning and acceptance approval only. Verified baseline: 97babb3ae9f3dc20b5602a7b46a947e833943eb2, tag m1-desktop-foundation-verified. Preserve the master and existing verified history.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 3, 9–10, 63–64, 66, 71, 74–76, 79–80, 82–84; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Memory lifecycle, notes, projects, tasks, people, organizations, decisions, events, documents, relationships, Markdown, backups and restore. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

## Objectives, dependencies and exclusions

A user in either edition can save/reopen knowledge, distinguish proposed from confirmed information, manage notes and related work, export readable Markdown, and preview/restore a verified backup. Rust governs privacy, ownership and revisions; generated contracts and shared components preserve native/browser parity.

Existing M1 foundations: private schema-2 SQLite workspace/settings catalog, transactional migrations, scoped Tauri IPC, authenticated loopback API, exact Host/Origin, generated DTOs/schema/TypeScript/static validators and actual restart verification. None is memory storage. Preserve existing onboarding/preferences/IDs. Search and inference remain M3/M4, providers M5, generated artifact engines M6–M12, ingestion M13, release certification M15. No cloud calls, auto-execution, document parser or hosted tenancy in M2. Optional vault and scheduled workspace backups remain within M2 and block completion until verified.

## M2.0 acceptance and deliverables

This task changes this plan, architecture, decisions, roadmap, requirements, build status, recovery, project-state and docs/evidence/M2-0.md only. No application, migration, dependency or capability change.

Acceptance: inventory source bullets below; inspect actual storage/transport limits; define ordered independently testable tasks, exact file ownership, commands, security, migration, recovery and approval gates. Validate links/state/master identity and existing documentation tests. Publish a protected checkpoint and independently restore source. Documentation checks do not advance product requirements to verified. Approval is required before M2.1 coding.

## Proposed storage and security contract

### Canonical data and crash handling

Keep the catalog and managed workspace IDs. Store immutable UTF-8 bodies below trusted-root/content/<workspace-id>/<record-id>/<revision-id>.md; all components are Rust-generated/validated IDs, never user titles or supplied paths. SQLite is authoritative for metadata, ownership, revisions, relationships and jobs; Markdown is authoritative for body bytes. Search indexes are rebuildable, but canonical metadata is not an expendable index. Restore metadata from verified snapshot/export manifests, never guess from stray files.

M2.1 introduces 0003_memory.sql with records/revisions and composite workspace ownership constraints. Every operation resolves workspace and record together, including history and edges. Writes require expectedRevision and reject stale edits without overwriting. Lists default to 50, maximum 100. Bound title to 200 Unicode scalars, body to 256 KiB UTF-8, sources to 32 references of at most 2 KiB each. Validate unknown fields, timestamps, NUL, sizes and versions before persistence. Render text literally: no raw HTML, scripts, automatic remote images or URL fetching.

Write a unique private create-new revision, flush/sync bytes, then commit its hash and pointer with metadata in one SQLite transaction. Failed transactions may leave unreferenced files; never auto-import/delete them. Verify committed bytes/hash on read; missing/altered content yields a recoverable error, never blank replacement. Acknowledge only successful file and DB commit. Test process interruption at each boundary on Windows/Linux; SQLite alone does not make external files atomic. Do not claim protection against every hardware/power failure. No automatic file garbage collection or irreversible purge in M2.

Allocate later migrations sequentially when each task starts. Never edit applied migrations, reset unknown catalogs or lower user_version. Test schema 1→2→current and 2→current, preserving existing IDs/settings; transaction rollback, simultaneous migration, corrupt/newer/unknown refusal. Retain root/ACL/link/Git-ancestor protections. All database/disk work runs off UI threads.

### Domain and privacy

Kinds: memory, note, project, task, person, organization, decision, event and document; relationship is a separately identified typed edge. Revision envelope includes stable ID, workspace ID, optional project ID, kind, version, UTC creation/update, optional event date/validity start/end, confidence (0–1 or explicitly unknown), state, privacy, provenance/source references, optional actual generator/model metadata and validation status. Creation is immutable; validity end cannot precede start. User authorship is explicit; never invent generator metadata. M2 document records are user-entered text/source references, not ingestion.

States: Proposed, Confirmed, Temporary, Durable, Superseded, Archived. Default Proposed; explicit user confirmation only. Temporary requires expiry but expiry never silently deletes. Durable expresses explicit retention. Superseded requires an existing same-workspace successor, rejects cycles/self-reference and preserves history. Archive is the reversible delete workflow; restore/history rollback makes a new revision. No hard-delete promise while backups/history retain data. M2.2 defines/tests the full transition table before implementing its actions.

Rust recognizes Sealed, Private, Standard, Public. Default Private. Standard/Public never authorize publishing/transmission. Paired browser and desktop are same-user local clients, not multi-user ACLs. Wrong workspace requests reveal no record existence. Initial plaintext slices reject Sealed create/read/update/export with vault-unavailable, never downgrade to Private. Clearly explain Private files are not encrypted. M2 cannot complete until M2.6/M2.7 vault gates pass. No title/body/source/credential logging; audit IDs/actions/outcomes only. Privacy downgrade requires explicit confirmation. No browser filesystem/process capability or provider dispatch.

### Export, backup and asynchronous work

Export is explicit and scoped to selected records/project/workspace, returning bounded data or an owned opaque artifact/job ID. No arbitrary browser destination paths. Markdown bodies plus versioned JSON manifest preserve metadata, relationships, revisions, hashes, provenance and privacy. Sealed is excluded from plaintext export; encrypted export/recovery is M2.7. Never imply a privacy label encrypts files or include credentials.

The catalog contains multiple workspaces. A selected-workspace backup must build a standalone constrained catalog for that workspace, not leak a raw full catalog. Snapshot metadata in one SQLite read transaction, copy exactly its referenced immutable bodies and verify a versioned integrity manifest before publication. No garbage collector may remove referenced bodies. Full application-catalog recovery is a separate explicitly labeled all-workspaces scope using SQLite's backup API; never copy only a live main DB. Private plaintext snapshots require a private managed destination and explicit confidentiality disclosure/consent; no sealed plaintext/secrets in ordinary backups.

Jobs precede asynchronous export/backup. Durable leases coordinate desktop/API processes; expired Running jobs recover Paused with checkpoints, never duplicate effects. Retry idempotent stages only; cancellation ends/rolls back a bounded stage, never publishes a partial snapshot. One worker per process plus DB lease enforcement; initial export/backup maximum 10,000 records/256 MiB, explicit failure rather than truncation. Jobs expose progress, bounded logs/resource limits and cancellation/retry. Scheduled backup defaults off, needs destination/privacy/retention consent, runs only while an approved process is open and reports missed runs. No OS background-service promise or automatic pruning in M2.

Restore preview shows schema, workspace identity, counts, privacy, hashes, size and conflicts. Validate every member/path/limit/hash/foreign key before writes. Restore into a new managed workspace ID, map internal IDs consistently and retain original IDs as provenance; never overwrite an existing workspace/root. Preserve originals and failed staging. Closed-root application recovery remains a distinct operator procedure. Development uses synthetic external roots only.

## Ordered task plan

Every product task inherits the commands, recovery and Done rules below. Listed new files are future implementation targets, not existing features. Extend existing adapters rather than create another backend. Stop after each task.

### M2.1 — Save, reopen and revise one memory

Dependency satisfied by user approval. Local implementation/evidence: docs/evidence/M2-1.md. Files created: crates/core/src/memory.rs, crates/core/src/content.rs, crates/core/src/privacy.rs, crates/core/migrations/0003_memory.sql, crates/core/tests/memory_roundtrip.rs, crates/api/tests/memories.rs, packages/ui/src/Memory.tsx, tests/e2e/memory.spec.ts, scripts/verify-memory.mjs and docs/evidence/M2-1.md.

Extend existing core storage/lib/readiness errors, API lib/auth allowlist, Rust contract generator and generated outputs, shared client/App/Navigation/tokens, desktop lib.rs and new permissions/memories.toml scoped to local default capability. Use a separate 320 KiB memory request limit; do not globally enlarge workspace limits or remote capabilities.

Steps: write ownership/schema/conflict/crash fixtures; implement migration/private immutable bodies; implement create/list/read/revise through both adapters; add shared literal-text editor with privacy/storage limits, real save/conflict/error feedback; implement actual browser/native restart verifier. Include full metadata envelope but only Proposed state initially; no unimplemented promotion or all-kinds CRUD controls.

Acceptance: Unicode Proposed Private memory saves/reopens/edits with revision history, survives both process restarts and retains identical metadata/bytes. Another workspace cannot read it. Stale simultaneous edit conflicts; acknowledged prior revisions survive interrupted saves. Test invalid dates/confidence/privacy/unknown fields, size bounds, corrupt/missing body, path/junction/ACL/root rejection, authentication/Host/Origin and adapter error parity. Observe zero non-loopback requests and actual native/browser UI. Run common suite, node scripts/build-desktop-test.mjs then node scripts/verify-memory.mjs. The new verifier must actually exist and execute; absence is failure. Stop before M2.2.

### M2.2 — Notes, lifecycle and supersession

Dependency: M2.1 verified. Create crates/core/src/lifecycle.rs, crates/core/tests/memory_lifecycle.rs, packages/ui/src/Notes.tsx, tests/e2e/memory-lifecycle.spec.ts and docs/evidence/M2-2.md. Extend memory/privacy/content/contracts/adapters/editor and verify-memory.mjs; add next forward migration if required.

Define/test the six-state transition table; implement explicit confirmation, durable retention, expiry without deletion, same-workspace acyclic supersession, reversible archive, note CRUD and comparison/restore as a new revision. Show provenance/event/validity/confidence/history. Archived/expired records remain available through explicit filters, excluded from default active list.

Acceptance: user creates note, confirms/durably retains memory, creates temporary memory, supersedes, archives/restores and compares history after restart in both editions. Cover all allowed/denied transitions, self/cycle/cross-workspace successor, stale writes, immutable creation date, privacy downgrade confirmation and generated-origin Proposed fixture never automatically confirmed. No model call. Run common suite and expanded verify-memory.mjs. Stop before M2.3.

### M2.3 — Projects, entities and relationships

Dependency: M2.2 verified. Create crates/core/src/entities.rs, crates/core/src/relationships.rs, crates/core/tests/entities_roundtrip.rs, crates/api/tests/entities.rs, packages/ui/src/Projects.tsx, packages/ui/src/Tasks.tsx, packages/ui/src/People.tsx, packages/ui/src/KnowledgeRecords.tsx, tests/e2e/entities.spec.ts, scripts/verify-entities.mjs and docs/evidence/M2-3.md. Extend common schema/contracts/adapters/navigation with next forward migrations.

Implement projects/tasks/people/organizations/decisions/events/document text records using common privacy/revisions. Add project membership, task due/completion, event dates and typed edges with stable IDs/provenance/revisions. Validate type-specific fields independently from memory lifecycle. Require existing same-workspace endpoints; archive without cascading data loss. Shared detail forms may serve the less common entity types with clear labels.

Acceptance: project links a memory/note/task/person/organization/decision/event/document; complete task, restart desktop/API and reopen all relationships/history. Test each kind, invalid dates/status, dangling/cross-workspace edges, archived endpoints, ownership and no leaked counts. Reserved artifact envelope is not a working Forge registry. Run common suite and verify-memory.mjs/verify-entities.mjs sequentially. Stop before M2.4.

### M2.4 — Persistent jobs and open-format export

Dependency: M2.3 verified. Create crates/core/src/jobs.rs, crates/core/src/export.rs, crates/core/tests/jobs_recovery.rs, crates/core/tests/export_roundtrip.rs, crates/api/tests/jobs.rs, packages/ui/src/Jobs.tsx, tests/e2e/export.spec.ts, scripts/verify-export.mjs and docs/evidence/M2-4.md. Extend catalog with next migration, adapters/contracts/UI and scripts/CI/private artifact rules.

Implement Queued/Running/Paused/Completed/Failed/Cancelled states (six named states in section 66), owner leases, versioned checkpoints, bounded progress/logs/resources/cancel/retry. Export selected knowledge as owned Markdown/JSON artifacts. Native output remains managed private storage until explicit trusted export; HTTP never accepts paths. Download/retrieve only owned complete artifacts.

Acceptance: project export is readable outside ANANSI without network/subscription and round-trips metadata/history. Cancel/retry and restart interrupted job; no partial publication. Test competing workers, lease expiry, every state, quota/disk failures, unauthorized artifact ID, Sealed refusal, hostile text/links and no body/credential logs. Run common suite and verify-export.mjs with actual file/download observations. Stop before M2.5.

### M2.5 — Snapshots, scheduling and restore preview

Dependency: M2.4 verified. Create crates/core/src/backup.rs, crates/core/src/restore.rs, crates/core/tests/backup_restore.rs, crates/api/tests/backups.rs, packages/ui/src/Backups.tsx, tests/e2e/backups.spec.ts, scripts/verify-backup.mjs and docs/evidence/M2-5.md. Extend jobs/privacy/storage/schema/contracts/adapters/navigation and recovery instructions.

Implement manual/versioned and opt-in scheduled workspace backup, selected-workspace standalone catalog, manifest verification, preview and fresh-target restore. Full application backup must be separately labeled and cover all referenced data. Reject source/destination recursion, member traversal, links and overwrite. Credential references are identifiers only. Product/source artifacts remain distinct backup scopes. Until M2.7 Sealed fails closed.

Acceptance: back up during authorized concurrent edit, verify referenced revision consistency, preview/restore as a fresh workspace and compare content/entities/edges/states/settings/history after restart in both editions. Second workspace must be absent from selected snapshot. Test consent/missed runs, disk full, cancel/retry/restart, missing/tampered members, oversized manifest, unsupported schemas, malicious paths/duplicate IDs, interrupted publication and corruption refusal before restore writes. Verify original bytes unchanged. Run common suite and verify-backup.mjs; inspect actual preview/errors/restore. Stop before M2.6.

### M2.6 — Vault security design and approval gate

Dependency: M2.5 verified. Create plans/M2-vault-security.md and docs/evidence/M2-6.md; update decisions/architecture/recovery/coverage. No crypto implementation before this gate.

Resolve audited library/platform/version, authenticated format, threat model, key generation/derivation/storage, unlock lifetime and restart locking, recovery key/passphrase, rotation, zeroization limits, metadata leaks, backup confidentiality and disposal. Verify current official library/audit sources then; M2.0 selects no library and claims no audit. Explain BitLocker/FileVault versus application lock/encryption. Use non-destructive opt-in import, never promise secure deletion of old plaintext/WAL/snapshots.

Acceptance: reviewable per-operation allow/deny table for list/read/history/edges/export/backup/restore, reproducible known-vector/tamper/wrong-key/recovery tests, exact implementation/dependency/key-storage/migration files and commands, and explicit approval. Inability to meet audit/platform/recovery requirements blocks M2 completion. Ordinary continuation or M2.0 approval cannot waive this gate. Stop for vault-plan approval before M2.7.

### M2.7 — Optional encrypted vault and Sealed workflows

Dependencies: approved M2.6 and verified library prerequisites. Reserved files: crates/core/src/vault.rs, crates/core/tests/vault_roundtrip.rs, crates/api/tests/vault.rs, packages/ui/src/Vault.tsx, tests/e2e/vault.spec.ts, scripts/verify-vault.mjs and docs/evidence/M2-7.md. M2.6 must refine format/key-storage/migration files; these reserved paths alone are not a sufficient security design.

Implement approved optional vault, unlock/lock, Rust policy in both editions, sealed bodies/metadata under the reviewed leak model and encrypted backup/export/restore in a fresh target. No persistent browser passphrases/unlock secrets or secret logs. All list/count/history/relationship/export paths must enforce locked-state policy before Sealed becomes usable.

Acceptance: actual native/browser sealed create/edit/lock/reopen and encrypted backup/restore; no plaintext in ordinary artifacts; wrong-key/tamper/locked denial preserves originals. Explain recovery/consent; run common suite, approved library tests and verify-vault.mjs. No OS encryption configuration or forensic secure-erasure claim. Stop before M2.8.

### M2.8 — Integrated Memory OS certification

Dependency: all prior M2 tasks verified. Create docs/evidence/M2.md; update plan/status/matrix/state/recovery/development/roadmap. Extend installed-app verification only for the actual Memory OS acceptance workflow; no scope expansion.

Acceptance: installed desktop and authenticated browser save/edit/lifecycle/entity/export/backup/restore/vault survive restart, preserve M1 onboarding/settings/isolation, work offline without external requests, and remain keyboard/motion/viewport accessible. Run common suite, all new verifiers sequentially, doctor:native/verify:native, build:installer, installer preflight and full local installed workflow plus CI. Record limits; later retrieval performance benchmarks are not claimed. Tag m2-memory-os-verified only after protected publication and independent source restore. Stop before M3.

## Source bullet acceptance inventory

| Master scope | Retained task ownership and acceptance |
|---|---|
| Section 9 project contents | M2.3 projects/research text/memories/sources/tasks/decisions; M6/M7/M9/M10/M12 ideas/concepts/experiments/prototype/generated files/simulations/agent tests/launch/feedback and approved internal cross-engine transfer. No manual-copy requirement added. |
| Section 9 artifact envelope | M2.1/M2.3 unique ID, project ID where assigned, type, version, timestamp, sources, actual optional generator/model, validation and privacy; generated registry M6/M7/M12. |
| Section 10 all ten types | M2.1 memories, M2.2 notes, M2.3 projects/tasks/people/organizations/decisions/events/documents/relationships. |
| Section 10 storage/indexes | M2.1 Markdown bodies and SQLite metadata/relationships; M2.4 job state; rebuildable derived search indexes M3/M4. Canonical metadata/bodies are never discarded in an index rebuild. |
| Section 10 metadata | M2.1 provenance/creation/update/event/validity/confidence/status/privacy/source fields; M2.2 supersession history, M2.3 relationships. |
| Section 10 six states/AI trust | M2.2 explicit transitions and confirmation; M3/M6/M10 must reuse no-automatic-confirmation policy. |
| Sections 63–64 | All tasks shared UI/tokens/errors/focus/keyboard/reduced motion/responsive workflows; M2.8 integrated acceptance. |
| Section 66 categories and states | M2.4 all six named states/progress/cancel/retry/resources/logs/restart; M2.5 backup handler. Index/embedding M3/M4, ingestion/connectors M13, collision M8, generation/build M7, SwarmTank M9, SOMNIA M10 remain tracked, not fake handlers. |
| Sections 71/74 | M2.1 native four-class policy/isolation/path/auth; M2.7 sealed enforcement; separate GHOST consent M11, SOMNIA privacy M10 and sanitized SwarmTank descriptions M9 remain binding. |
| Section 75 | M2.1 truthful plaintext limits; M2.6 audited-library/BitLocker/FileVault/lock/recovery design; M2.7 optional encrypted vault/confidential backups; release review M15. |
| Section 76 | M2.5 manual/scheduled/versioned/integrity/preview/recovery; workspace/application distinct; source repository M0, project/product artifacts M7 and secure credential references M5 retained. Never secrets in ordinary backups. |
| Section 79 | M2 versioned knowledge/projects; M6/M7/M12 idea/spec/repository/build/test/simulation/SOMNIA/deployment registry and comparison/rollback. |
| Section 80 exports | M2.4 memories/notes/research/project history; M6/M7/M9/M10/M12 ideas/specifications/source/simulation/evaluation/product artifacts retain open exports. |
| Section 82 M2 bullets | CRUD M2.1/M2.2 (reversible archive), notes M2.2, projects/tasks/people M2.3, Markdown M2.1/M2.4, backups M2.5/M2.7. |
| Sections 83–84 | Every task real app/tests/evidence/checkpoint/backup; storage/recovery only in M2. Import/search/models/citations/cloud/cost M3–M5/M13; no full-core completion claim. |

Assignments are not passed tests. No bullet is removed by subdivision. Vault remains inside M2, rather than silently postponed to release hardening.

## Verification commands and evidence

Every product task runs pnpm check; pnpm test; pnpm typecheck; pnpm contracts:check; pnpm test:ui; pnpm test:rust; node scripts/cargo.mjs fmt --all --check; node scripts/cargo.mjs clippy --workspace --all-targets --locked -- -D warnings; pnpm build:web; pnpm test:e2e; git diff --check. When DTOs change, first pnpm contracts:generate and inspect generated diffs. Verify new dependencies against official sources at implementation, update lockfiles through tools and run pnpm install --frozen-lockfile --ignore-scripts.

New task verifiers must be created and executed before claiming acceptance. Run UI/native verifiers sequentially due to fixed ports and retain occupied-port refusal. Evidence records exact commands/exit codes/counts/versions/failures/fixes, actual screenshots, restart observations, CI URL and verified SHA. Only private synthetic external roots; no real catalog targets. No mock or generated screenshot proves a working product.

M2.0 runs pnpm check; node --test tests/docs-validation.test.mjs tests/repository-hygiene.test.mjs; node scripts/validate-planning.mjs --self-test; git diff --check; git hash-object docs/MASTER_SPEC.md (expected b95eae4a3813fb2cd87c00360fcdd9b2489cedec). These validate documentation, not Memory OS.

## Recovery, checkpoint rules and Definition of Done

Before migrations/test installs, preserve verified SHA and an independent committed-source bundle with scripts/backup-source.ps1. Private-data recovery closes all native/API processes, preserves the complete original managed root and uses a new compatible copy; source bundles exclude catalogs. Never downgrade schema, discard a failed root or overwrite original content. Fault tests use generated synthetic roots; integrity failures remain visible and recoverable.

After each task update evidence/status/requirement subset/state and decisions, commit using existing identity, push a codex/ branch, wait exact-SHA required checks, publish safely to protected master, verify remote/clean tree and independently bundle/restore source. Do not move existing tags. Record previous verified SHA where necessary to avoid self-reference; inspect subsequent metadata on recovery. M2 tag belongs only to M2.8.

M2.0 Done: validated, recoverable, reviewable plan with every retained requirement and unresolved gate visible; coding pending approval. Product task Done: actual native/browser workflow, security/failure/restart/regression evidence, honest limits and recoverable published checkpoint. M2 Done additionally needs M2.1–M2.8 including vault/backup certification. Stop after the approved task and report works/tests/blockers/next task.

## Reference checks

Reviewed official [SQLite backup documentation](https://www.sqlite.org/backup.html), [SQLite atomic commit documentation](https://www.sqlite.org/atomiccommit.html) and [Tauri capability documentation](https://v2.tauri.app/security/capabilities/) during M2.0. These support separating catalog snapshots, external-file consistency and scoped native grants. No dependency/version/crypto selection is made here. Recheck exact pinned library APIs and platform behavior during each implementation task.
