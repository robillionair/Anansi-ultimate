# M8 — LEXI-COLLIDER

Status: tracked planning task; detailed implementation design and acceptance approval required before coding. This is not an implemented feature or a complete technical design.

Slice 1 (text-first, owner-authorized 2026-10-27, branch `muse/m8-slice1-collider`): in progress — two text beams, six-section validated output, immutable experiment save and restore on the M5 substrate; URL fetching, document ingestion, saved-project/memory selection, 3D visualization and approved Forge transfer remain deferred to later slices. Linux verification in progress; Windows verification pending. Design: `docs/handoffs/M8-slice1-collider-design.md`; record: `docs/handoffs/M8-slice1-collider-addendum.md`. This slice does not declare M8 complete and does not waive M7's open native acceptance, execution environment or production gates.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 23–27, 65–67, 78–80; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Six-stage pipeline, versioned outputs, approved Forge transfer, real-state 3D and accessible alternative. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. M0 is the current implementation phase; do not start this milestone during reconciliation.
