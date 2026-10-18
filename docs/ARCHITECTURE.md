# ANANSI Ultimate — Architecture

## Architectural principles

1. Local-first ownership: user data remains local by default.
2. Shared domain: desktop and browser clients call the same authorized domain operations.
3. Capability separation: native Tauri capabilities are never exposed directly to an untrusted hosted browser.
4. Explicit trust boundaries: local desktop, local browser, local API, inference providers, and future hosted multi-user deployment are separate boundaries.
5. Evidence over appearance: a screen or stub is not a completed feature.

## Target system

```text
React + TypeScript + Vite shared UI/domain client
          |                         |
   Tauri 2 adapter             HTTP API adapter
          |                         |
      Rust core          Axum local API service
          |                         |
     SQLite + FTS5 + embedded vector index
          |
  authorized provider adapters (Ollama, official inference providers, OAuth/API keys)
```

The desktop application may use native adapters for approved local capabilities. The browser application uses authenticated HTTP APIs. Domain commands, DTOs, validation, authorization rules, and persistence contracts are shared or generated from a single source of truth.

## Proposed repository shape

```text
apps/desktop/        Tauri shell and native adapter
apps/web/            Vite browser client
crates/core/         Rust domain, persistence, search, policy
crates/api/          Axum API and authentication boundary
packages/contracts/  shared schemas and API contracts
packages/ui/         shared React components and design tokens
docs/                durable specifications and evidence
plans/               phase plans
tests/               cross-layer and acceptance tests
```

M1.2 establishes this workspace shape for readiness. M1.3 adds the shared SQLite workspace metadata/settings catalog; M2.1 adds local memory revisions and native privacy classification. M2.2 adds notes and explicit lifecycle revisions. M2.3 adds entities and versioned relationships through the same catalog/adapters. Search, jobs/export, encrypted vault and provider subsystems remain later tasks. Both adapters use the same bounded core operations and trusted storage configuration; see ADR-0013, ADR-0017 and evidence/M2-1.md.

## Security boundaries

Private/sealed memories, credentials, OAuth tokens, MCP tools, local filesystem access, process execution, and inference billing are privileged capabilities. Each requires explicit authorization, permission checks, auditability, and tests. Public multi-user hosting is a later security-reviewed milestone and must not reuse a local trust assumption.

## Open architectural decisions

- M2.1 memory revisions and M2.2 schema-4 lifecycle storage implement the accepted [M2 plan](../plans/02-memory-system.md) subset: immutable Markdown bodies, transactional SQLite revision pointers, scoped ownership and native plaintext privacy policy. Audited vault library/key/recovery design remains the explicit M2.6 gate and blocks M2 completion until M2.7 verification.
- Exact vector index implementation awaits performance and data-shape requirements.
- Section 14 lists candidate providers; official authentication/entitlement verification is required in M5. Section 70 specifies Windows Credential Manager and macOS Keychain; no credentials integration is implemented.
- M1.2 verifies readiness command/API parity; M1.3 extends the same generated contracts and real cross-adapter tests to workspace/settings commands. Each subsequent command must preserve these boundaries.


## Reconciled ownership

[The canonical roadmap](ROADMAP.md) follows master section 82. M0 prepares repository tooling only. M1 owns shared DTO/command contracts, local API authentication design, desktop/browser launch and SQLite/workspace foundations. Rust domain rules remain common to Tauri and Axum adapters; the browser never receives native process/filesystem capabilities. Axum and the proposed layout are accepted targets requiring validation, not master-mandated implementation details. Vector implementation, sandbox technology, provider details and Big Brain behavior remain explicit design gates. See ADR-0005.

M0.1 records [directory and contract ownership](DEVELOPMENT.md). Core owns domain authorization/validation; desktop commands and authenticated API routes adapt the same operations. Contracts must have one authoritative schema or generated source, chosen and tested in M1. UI consumes these contracts through transport adapters. Browser authentication/origin protection is an M1 design and test obligation, not an implemented claim. The repository policies and development tests add no native capabilities or product domain behavior.

## M1.0 domain/transport design gate

The M1 detailed plan chooses authoritative Rust DTOs with generated shared schemas/bindings, an authenticated loopback browser adapter with explicit pairing/ACL and Origin/Host tests, and direct scoped Tauri commands invoking the same core rules. M1.2 implements and tests these boundaries for the readiness command only; see evidence/M1-2.md. A read-only repository toolchain doctor is development infrastructure, not ANANSI Core/product readiness. M1.1 verified native compile/link before M1.2 used that toolchain for the application slice.

M1.4 extends the shared settings contract with typed intent and onboarding completion. Recognized schema-1 catalogs migrate transactionally to schema 2 with preserved workspace fields. These UI preferences grant no cloud/native capability; the permission/transport boundary is unchanged. Shared onboarding, navigation, workspace and settings components follow ADR-0014; hardware/providers, memory and installer remain their approved tasks.

M1.5 adds per-user NSIS development packaging, checksum/payload manifests and isolated install/uninstall verification. The installed application uses the same core, contracts and UI. No automatic API service, elevation, updater, signing, filesystem grant or model setup is introduced. See ADR-0015 for WebView2 prerequisites, CI's explicit installed-UI limitation and the M15 boundary.

## M2.0 proposed domain and recovery boundaries

The proposed Memory OS extends the existing core/catalog and generated adapter contracts. Markdown stores immutable body bytes; SQLite owns metadata, revisions, workspace ownership, relationships and jobs. Search indexes are derived; canonical metadata is not discarded on index rebuild. File write/sync precedes transactional publication, with hash verification and recoverable orphan/missing-file handling rather than an unsupported cross-filesystem atomicity claim.

All four privacy classes are native policy. Initial plaintext slices reject Sealed and disclose encryption limits; optional vault remains mandatory milestone scope behind a separate security approval gate. Selected-workspace snapshots must exclude other catalog workspaces; live full-catalog backup is a distinct explicit scope. Restore creates a fresh target, never overwrites originals. See the [task plan](../plans/02-memory-system.md) and ADR-0016. No product or security capability changes occur in this planning checkpoint.

## M2.1 memory storage implementation

The user accepted the M2 plan. M2.1 implements schema-3 local Proposed memories, immutable Markdown revisions, paginated metadata/history and optimistic edit conflicts through the common Rust service. Scoped native IPC and authenticated memory HTTP routes share generated DTOs and literal-text UI. File/ACL work precedes a short SQLite transaction that rechecks expectedRevision. Integrity failures preserve originals and report recovery errors. See ADR-0017 and [evidence](evidence/M2-1.md).

Dates/confidence/sources are validated; timestamps/authorship/state are core-owned. Notes, lifecycle actions, project relations, jobs, automated backups and encrypted vault remain later tasks. Private storage is plaintext with local permissions, Standard/Public grant no sharing consent, and Sealed requests fail closed. Browser operations accept managed IDs only. Older binaries reject schema 3; preserve the whole closed root including content before manual rollback.

## M2.2 lifecycle revision boundary

Schema 4 retains original bodies/revisions and introduces a semantic compatibility barrier against schema-3 writers. Notes use the same storage and editor; lifecycle changes, archive restoration and historical content restore use the existing scoped revise command with expectedRevision. Rust owns the explicit six-state transition table and checks successor cycles under the publication lock. Default latest lists exclude archived/expired records; history ignores visibility filters. Historical restore creates a revision and preserves current privacy, state, creation and origin metadata. See ADR-0018 and [M2.2 evidence](evidence/M2-2.md) for the transition table and verification status. Project/entity relationships are now implemented in the M2.3 slice below; jobs, export, backups and vault remain later M2 tasks.


## M2.3 entities and relationship history

M2.3 extends the same memory catalog/contracts/adapters to all nine knowledge kinds. The acceptance repair uses schema 6: stable typed edge IDs, immutable relationship revisions, reversible archive/restore and optimistic expectedVersion actions. Historical record reads retain their saved link snapshot; separate edge history records later actions. Project IDs resolve only to same-workspace projects; omission preserves an assignment and explicit null clears it. See ADR-0019 and [repair evidence](evidence/M2-3.md). Jobs/export, backups and vault remain later M2 tasks; the master specification is unchanged.

## M2.4 jobs and portable export — locally verified

Schema 7 adds durable workspace-owned export jobs. Shared Rust services own claims, leases, state transitions, resource limits, immutable attempt files and fenced completion. Both adapters use the same generated job DTOs; the React jobs view remains responsive while a background worker performs catalog/file work. Expired Running jobs pause instead of silently resuming. Checkpoints version the idempotent snapshot stage, not individual imported/output effects.

Exports are owned complete USTAR artifacts with Markdown revisions, hashes and a versioned JSON manifest. Selected project membership is current; selected record history remains complete within the byte bound. Cross-selection references are opaque and do not broaden disclosure. Native files stay in managed private storage until the trusted system Save dialog; HTTP supports only bounded owned-artifact chunks. See ADR-0020 and [M2.4 evidence](evidence/M2-4.md). Backups, product restore and vault remain separate tasks.

M2.4 local workflows and recovery behavior passed at 8f7ff87; exact CI/publication/source restoration are tracked in its evidence/independent ledger. The native dialog exits with its actual desktop parent; final verification observed that exit before cleanup. No generic browser filesystem/process grant was added.

## M2.5 backup and recovery boundary — local acceptance verified

Schema 8 adds opt-in schedule consent, bounded owned upload state and restore-origin mappings. Workspace snapshots construct a fresh standalone SQLite catalog containing only the selected workspace; application snapshots use SQLite's online backup API and include referenced immutable bodies and completed artifacts. Both produce versioned, hashed USTAR packages through the existing durable job worker. A read transaction fixes metadata/revision references, and owner/lease checks fence publication after package verification and a final privacy check. All Sealed records fail closed.

The shared jobs command surface adds manual backup, preview, fresh restore, schedule and bounded saved-file import. Uploads and previews use private staging, exact schema/member/hash validation and no client paths. Restore remaps IDs, retains origin mappings, copies immutable files before committing new catalog rows, never overwrites originals and disables restored schedules. Failed staging is preserved; no automatic pruning or secure deletion claim. The same generated validators and shared Backups UI run through authenticated HTTP and scoped native job IPC. See ADR-0021 and [M2.5 evidence](evidence/M2-5.md) for actual verification status.

## M2.6 security design reconciliation — no implementation

The [user-approved contract](approvals/M2-6-user-contract.md) is preserved verbatim and [the reconciliation plan](../plans/M2-vault-security.md) records exact dependency proposals and their audit/platform limitations. Desktop and API remain separate Rust processes, with no existing shared vault broker or secret-input capability. Proposed per-process guarded keys, native-only secret entry, scoped ephemeral handles and cross-process revocation require implementation tests. Current schema remains 8 and all Sealed operations continue to fail closed. [The adopted addendum](approvals/M2-6-corrections.md) resolves G2–G4 at design level; G1 audit evidence remains open. Authorization is not an executed test or verified audit; see ADR-0022. No accepted runtime architecture is silently changed.
