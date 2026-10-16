# ANANSI Ultimate — Architecture Decisions

## ADR-0001: Shared domain with two delivery adapters

- **Decision:** Desktop and browser clients share contracts, domain operations, models, and UI primitives; they differ only at capability adapters and transport boundaries.
- **Reason:** Prevents divergent business rules and preserves local/native capability separation.
- **Status:** accepted as a target architecture; validate during M1.

## ADR-0002: Local-first and explicit privilege boundaries

- **Decision:** Local persistence is the default. Native filesystem/process access, secrets, MCP, sealed memories, and paid inference require explicit authorization and permission checks.
- **Reason:** Protects ownership and avoids treating a hosted browser as trusted native code.
- **Status:** accepted; detailed policy awaits the master specification.

## ADR-0003: Do not invent missing product requirements

- **Decision:** Track the absent master specification as a blocker and keep only the orchestration brief's named areas as provisional matrix anchors.
- **Reason:** A plausible but fabricated spec would create false traceability and irreversible implementation drift.
- **Status:** active until the source specification is restored.

## ADR-0004: Restored specification requires plan reconciliation

- **Observation (2026-10-16):** Pulling commit `3f7571a` restored the master specification. ADR-0003's missing-source blocker is resolved, but its provisional requirements have not yet been fully mapped.
- **Conflict:** Master specification section 82 defines M0–M15, while docs/ROADMAP.md defines M0–M9 with different milestone scopes. Foundation and runnable-shell plans reference absent PHASE-000 and PHASE-001 detailed files. No explicit task-level approval is recorded.
- **Decision:** Follow docs/RECOVERY.md step 6: stop product implementation, record the conflict, and require roadmap reconciliation plus restoration of the approved detailed plan before selecting implementation scope. The current continuation request does not specify which conflicting milestone definition to adopt.
- **Status:** unresolved; no later milestone started.
