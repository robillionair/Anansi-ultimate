# ANANSI Ultimate — Requirements Matrix

Status vocabulary: `provisional`, `planned`, `in progress`, `verified`, `blocked`, `deferred`. No row may be marked verified without workflow evidence.

| ID | Requirement area | Subsystem | Priority | Dependencies | Milestone | Status | Test / evidence required |
|---|---|---|---|---|---|---|---|
| AN-001 | Windows Tauri 2 installer producing an NSIS `.exe` | Desktop/release | P0 | Toolchain, app shell | M5 | provisional | Build installer, install on clean Windows VM, launch |
| AN-002 | Responsive browser application | Web/UI | P0 | Shared UI, local API | M2 | provisional | Browser smoke test and responsive checks |
| AN-003 | Shared visual design system | UI | P0 | Product UX spec | M1 | blocked | Component tests and visual review |
| AN-004 | Memory OS data model and lifecycle | Core/memory | P0 | Master spec, SQLite | M3 | blocked | Create/read/update/archive workflow |
| AN-005 | Hybrid lexical/vector search | Core/search | P0 | Memory model, FTS5, vector index | M3 | blocked | Precision/recall fixture tests and UI workflow |
| AN-006 | Ask ANANSI | AI/orchestration | P0 | Search, providers, context | M4 | blocked | Grounded answer workflow with citations/provenance |
| AN-007 | AI Matrix | AI/orchestration | P1 | Provider contracts | M6 | blocked | Multi-provider routing and failure tests |
| AN-008 | OAuth and API keys | Integrations/security | P0 | Secret storage, auth | M4 | blocked | Token lifecycle and redaction tests |
| AN-009 | Local LLMs | AI/providers | P0 | Provider adapter, Ollama contract | M4 | blocked | Offline inference smoke test with explicit consent |
| AN-010 | Context Compiler | AI/context | P0 | Memory/search, policy | M4 | blocked | Deterministic context fixture tests |
| AN-011 | Big Brain | AI/orchestration | P1 | Context Compiler, providers | M6 | blocked | End-to-end reasoning workflow and audit evidence |
| AN-012 | Product Forge | Product/workflow | P1 | Core, permissions | M7 | blocked | Real artifact workflow and persistence tests |
| AN-013 | LEXI-COLLIDER | Language/workflow | P1 | Core, AI contracts | M7 | blocked | Input/output fixture and safety tests |
| AN-014 | SwarmTank 3D | Visualization | P2 | Shared UI, Three.js | M8 | blocked | Interactive render smoke test and performance budget |
| AN-015 | SOMNIA | AI/workflow | P1 | Memory, local-first policy | M7 | blocked | Workflow tests and privacy review |
| AN-016 | GHOST-PROMPT | AI/workflow | P1 | Context policy, providers | M7 | blocked | Prompt transformation tests and audit trail |
| AN-017 | MCP | Integrations/security | P1 | Permission model, contracts | M6 | blocked | Allow/deny, timeout, and audit tests |
| AN-018 | Privacy and security | Cross-cutting | P0 | All privileged features | Every milestone | blocked | Threat model, redaction, permission and penetration checks |
| AN-019 | Backups and restoration | Core/recovery | P0 | Storage schema, encryption | M3 | blocked | Backup/restore round trip and corruption handling |
| AN-020 | ANANSI Core readiness | Core/platform | P0 | Shared contracts, policy | M2 | blocked | Core API contract suite and readiness checklist |
| AN-021 | Enterprise architecture | Platform/deployment | P2 | Security review, tenancy model | M9 | deferred | Architecture review, isolation and migration tests |

## Matrix notes

The complete master specification was restored in commit `3f7571a`. Rows remain provisional pending full specification mapping and reconciliation of the conflicting milestone sequences (ADR-0004). Existing blocked rows also require their subsystem dependencies and approved detailed plans; restoring the specification alone does not verify any feature. No requirement status was advanced during this inspection.

Coverage vocabulary explicitly includes: Windows installer, Web UI, Memory OS, Hybrid search, Ask ANANSI, AI Matrix, OAuth and API keys, Local LLMs, Context Compiler, Big Brain, Product Forge, LEXI-COLLIDER, SwarmTank 3D, SOMNIA, GHOST-PROMPT, MCP, Privacy and security, Backups and restoration, ANANSI Core readiness, and Enterprise architecture.
