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
