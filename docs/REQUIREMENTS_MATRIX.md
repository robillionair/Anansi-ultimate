# ANANSI Ultimate — Requirements Matrix

Status vocabulary: `provisional`, `planned`, `in progress`, `verified`, `blocked`, `deferred`. No row may be marked verified without workflow evidence.

| ID | Requirement area | Subsystem | Priority | Dependencies | Milestone | Status | Test / evidence required |
|---|---|---|---|---|---|---|---|
| AN-001 | Windows Tauri 2 installer producing an NSIS `.exe` | Desktop/release | P0 | Toolchain, app shell | M1, M15 | planned | Build installer, install on clean Windows VM, launch |
| AN-002 | Responsive browser application | Web/UI | P0 | Shared UI, local API | M1, M2, M15 | in progress | Browser smoke test and responsive checks; M1.2 readiness evidence: [record](evidence/M1-2.md), full scope still open |
| AN-003 | Shared visual design system | UI | P0 | Product UX spec | M1 | in progress | Component tests and visual review; M1.2 readiness evidence: [record](evidence/M1-2.md), full scope still open |
| AN-004 | Memory OS data model and lifecycle | Core/memory | P0 | Master spec, SQLite | M2 | planned | Create/read/update/archive workflow |
| AN-005 | Hybrid lexical/vector search | Core/search | P0 | Memory model, FTS5, vector index | M3, M4 | planned | Precision/recall fixture tests and UI workflow |
| AN-006 | Ask ANANSI | AI/orchestration | P0 | Search, providers, context | M3, M4 | planned | Grounded answer workflow with citations/provenance |
| AN-007 | AI Matrix | AI/orchestration | P1 | Provider contracts | M5 | planned | Multi-provider routing and failure tests |
| AN-008 | OAuth and API keys | Integrations/security | P0 | Secret storage, auth | M5 | planned | Token lifecycle and redaction tests |
| AN-009 | Local LLMs | AI/providers | P0 | Provider adapter, Ollama contract | M3, M14 | planned | Offline inference smoke test with explicit consent |
| AN-010 | Context Compiler | AI/context | P0 | Memory/search, policy | M3, M4 | planned | Deterministic context fixture tests |
| AN-011 | Big Brain | AI/orchestration | P1 | Context Compiler, providers | M4 (design gate) | blocked | End-to-end reasoning workflow and audit evidence |
| AN-012 | Product Forge | Product/workflow | P1 | Core, permissions | M7, M12 | planned | Real artifact workflow and persistence tests |
| AN-013 | LEXI-COLLIDER | Language/workflow | P1 | Core, AI contracts | M8 | planned | Input/output fixture and safety tests |
| AN-014 | SwarmTank 3D | Visualization | P2 | Shared UI, Three.js | M9 | planned | Interactive render smoke test and performance budget |
| AN-015 | SOMNIA | AI/workflow | P1 | Memory, local-first policy | M10 | planned | Workflow tests and privacy review |
| AN-016 | GHOST-PROMPT | AI/workflow | P1 | Context policy, providers | M11 | planned | Prompt transformation tests and audit trail |
| AN-017 | MCP | Integrations/security | P1 | Permission model, contracts | M13 | planned | Allow/deny, timeout, and audit tests |
| AN-018 | Privacy and security | Cross-cutting | P0 | All privileged features | M0–M15 | in progress | Threat model, redaction, permission and penetration checks; M1.2 readiness evidence: [record](evidence/M1-2.md), full scope still open |
| AN-019 | Backups and restoration | Core/recovery | P0 | Storage schema, encryption | M0, M2, M15 | planned | Backup/restore round trip and corruption handling |
| AN-020 | ANANSI Core readiness | Core/platform | P0 | Shared contracts, policy | M1 (contracts), M14 (appliance) | in progress | Core API contract suite and readiness checklist; M1.2 readiness evidence: [record](evidence/M1-2.md), full scope still open |
| AN-021 | Enterprise architecture | Platform/deployment | P2 | Security review, tenancy model | M14 (design gate), M15 (review) | deferred | Architecture review, isolation and migration tests |

## Matrix notes

Milestones now follow the approved canonical roadmap and master section 82. Planned means retained and assigned, not implemented or tested. AN-011 remains blocked on behavior design: Big Brain is named but not specified. AN-021 remains deferred as future commercial architecture under section 81; tenancy/isolation and security review are prerequisites, with no enterprise claims allowed. Other rows await their milestone dependencies and detailed plans.

[The full coverage ledger](SPEC_COVERAGE.md) retains all 90 source sections, including requirements absent from the original coarse matrix.

| ID | Additional retained scope | Milestone | Status | Required evidence |
|---|---|---|---|---|
| AN-022 | Source protection, CI, repository recovery | M0 | verified | docs/evidence/M0.md: hygiene, Windows/Linux CI, protected master and actual independent bundle/restore with matching HEAD/tree and passing checks; annotated completion checkpoint |
| AN-023 | Onboarding, local workspace, navigation/settings | M1 | planned | Desktop/browser launch, provider skip, persistence and permission tests |
| AN-024 | Universal projects, artifacts, versioning/rollback | M2, M6, M7, M12 | planned | Durable cross-module project round trip and rollback |
| AN-025 | All document formats, OCR/transcription, async ingestion | M13 | planned | Format fixtures, provenance, resumability/cancellation and privacy tests |
| AN-026 | Zero-Cost Lock, routing, budgets/disclosure | M3, M4, M5; all inference milestones | planned | Deny unknown/paid fallback and unauthorized data; disclosure fixtures |
| AN-027 | Innovation/Opportunity Studio and idea scoring | M6 | planned | 5–10 concepts, transparent hypotheses, consent and saved project workflow |
| AN-028 | Unified lifecycle and Launch Studio | M12 | planned | Section 85 workflow; real approval before publishing |
| AN-029 | Persistent jobs and recovery | M2, M3, M6, M13 | planned | Restart/retry/cancellation/resource-limit tests |
| AN-030 | Native macOS/Apple Silicon, updates/credential vaults | M14, M15 | planned | Native signed installation/update and offline verification |
| AN-031 | Encryption, privacy classes, optional audited vault | M2, M15 | planned | Vault/recovery and authorization tests; clear disk-encryption distinction |
| AN-032 | Performance, accessibility, engine evaluation | M3–M15 | planned | 1k/10k/100k datasets, engine suites, keyboard/2D fallback evidence |
| AN-033 | Export/ownership and synthetic provenance | M2–M15 | planned | Open-format exports, labels, seeded replay, no synthetic-to-confirmed promotion |

Section-level coverage is complete for planning; bullet-level acceptance coverage is required at each detailed-plan gate. No rows are verified by documentation validation.

M1.0 planning/readiness evidence is in docs/evidence/M1.md. Missing local Windows/Rust prerequisites block native implementation verification; this audit advances no product requirement to verified. The current M1 authorization does not alter later milestone ownership or the retained 90-section coverage ledger.

M1.1 partial progress: Rust/Cargo installed and pinned; actual local metadata compile passes, full MSVC link fails because link.exe/C++ SDK is missing. Prepared admin installation and real native verification commands are tracked, with evidence in docs/evidence/M1.md. No product row is advanced to verified; AN-001/002/003 and the native workflow remain open.

M1.1 resolution: user completed Microsoft installation and local native doctor plus actual compile/link/run passed. Rust/MSVC/SDK prerequisites are verified with logs and SDK-discovery regression coverage in docs/evidence/M1.md. Earlier M1.0/M1.1 missing-tool notes are historical. No desktop/browser/installer requirement is marked verified by this toolchain-only task.
