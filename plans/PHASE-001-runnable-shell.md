# M1 — Windows desktop .exe and web UI foundation

Status: tracked planning task; detailed implementation design and acceptance approval required before coding. This is not an implemented feature or a complete technical design.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 5–8, 63–64, 70–71; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Tauri 2, React, shared contracts/UI, SQLite migrations, local workspace, navigation, settings, browser API boundary; development NSIS .exe. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. M0 is the current implementation phase; do not start this milestone during reconciliation.
