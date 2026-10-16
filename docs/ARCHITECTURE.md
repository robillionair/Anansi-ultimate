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

M1.2 establishes this workspace shape for readiness. M1.3 adds the shared SQLite workspace metadata/settings catalog; search, memory policy and provider subsystems remain target architecture. Both adapters use the same bounded core operations and trusted storage configuration; see ADR-0013 and evidence/M1-3.md.

## Security boundaries

Private/sealed memories, credentials, OAuth tokens, MCP tools, local filesystem access, process execution, and inference billing are privileged capabilities. Each requires explicit authorization, permission checks, auditability, and tests. Public multi-user hosting is a later security-reviewed milestone and must not reuse a local trust assumption.

## Open architectural decisions

- The restored specification defines memory metadata/states, privacy classes and an optional audited vault. M1.3's schema/migration covers workspace metadata/settings only; memory and encryption-library designs remain M2 planning decisions.
- Exact vector index implementation awaits performance and data-shape requirements.
- Section 14 lists candidate providers; official authentication/entitlement verification is required in M5. Section 70 specifies Windows Credential Manager and macOS Keychain; no credentials integration is implemented.
- M1.2 verifies readiness command/API parity; M1.3 extends the same generated contracts and real cross-adapter tests to workspace/settings commands. Each subsequent command must preserve these boundaries.


## Reconciled ownership

[The canonical roadmap](ROADMAP.md) follows master section 82. M0 prepares repository tooling only. M1 owns shared DTO/command contracts, local API authentication design, desktop/browser launch and SQLite/workspace foundations. Rust domain rules remain common to Tauri and Axum adapters; the browser never receives native process/filesystem capabilities. Axum and the proposed layout are accepted targets requiring validation, not master-mandated implementation details. Vector implementation, sandbox technology, provider details and Big Brain behavior remain explicit design gates. See ADR-0005.

M0.1 records [directory and contract ownership](DEVELOPMENT.md). Core owns domain authorization/validation; desktop commands and authenticated API routes adapt the same operations. Contracts must have one authoritative schema or generated source, chosen and tested in M1. UI consumes these contracts through transport adapters. Browser authentication/origin protection is an M1 design and test obligation, not an implemented claim. The repository policies and development tests add no native capabilities or product domain behavior.

## M1.0 domain/transport design gate

The M1 detailed plan chooses authoritative Rust DTOs with generated shared schemas/bindings, an authenticated loopback browser adapter with explicit pairing/ACL and Origin/Host tests, and direct scoped Tauri commands invoking the same core rules. M1.2 implements and tests these boundaries for the readiness command only; see evidence/M1-2.md. A read-only repository toolchain doctor is development infrastructure, not ANANSI Core/product readiness. M1.1 verified native compile/link before M1.2 used that toolchain for the application slice.

M1.4 extends the shared settings contract with typed intent and onboarding completion. Recognized schema-1 catalogs migrate transactionally to schema 2 with preserved workspace fields. These UI preferences grant no cloud/native capability; the permission/transport boundary is unchanged. Shared onboarding, navigation, workspace and settings components follow ADR-0014; hardware/providers, memory and installer remain their approved tasks.
