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

This is a target shape, not an implemented claim. The first coding phase must validate the toolchain and choose the minimum viable workspace layout.

## Security boundaries

Private/sealed memories, credentials, OAuth tokens, MCP tools, local filesystem access, process execution, and inference billing are privileged capabilities. Each requires explicit authorization, permission checks, auditability, and tests. Public multi-user hosting is a later security-reviewed milestone and must not reuse a local trust assumption.

## Open architectural decisions

- Exact schema and encryption model await the missing master specification.
- Exact vector index implementation awaits performance and data-shape requirements.
- OAuth provider set and secret-storage mechanism await provider requirements.
- Tauri command/API parity needs contract tests in the first vertical slice.
