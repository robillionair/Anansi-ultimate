# M9 — SwarmTank 3D

Status: tracked planning task; detailed implementation design and acceptance approval required before coding. This is not an implemented feature or a complete technical design.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 34–41, 65–67, 73, 77–78; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Synthetic personas, seeded decisions, sensitivity controls, reports, accessible 2D/3D, approved product improvement loop. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. M0 is the current implementation phase; do not start this milestone during reconciliation.

## Slice 1 checkpoint — 2026-10-31

Slice 1 (synthetic personas & decision engine) implemented on branch `muse/m9-swarmtank-slice1` from Windows-verified baseline `7c576b85`; design before code in `docs/handoffs/M9-swarmtank-slice1-design.md`, addendum in `docs/handoffs/M9-swarmtank-slice1-addendum.md`. Linux verification: Rust 562 passed + 4 known live-Ollama environment failures, node 131/131, browser battery 15/15. Independent Windows rerun pending. Deferred, not claimed: §38 3D arena, scenario engine, provider-backed generation, URL import, spec auto-start, native save dialog.
