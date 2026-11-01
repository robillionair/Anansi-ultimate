# M10 — SOMNIA

Status: tracked planning task; detailed implementation design and acceptance approval required before coding. This is not an implemented feature or a complete technical design.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 42–51, 66–67, 72–74, 77–78, 87; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Isolated redacted replay, nightmares, baseline/candidate/held-out tests, recoverable proposals, consented scheduling. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. M0 is the current implementation phase; do not start this milestone during reconciliation.

## Slice 1 checkpoint — 2026-11-02 (Linux delivered; Windows rerun pending)

First implemented slice (branch `muse/m10-somnia-slice1`, from coordination baseline `68ee3532`): isolated, manual, data-only replay with four curated cases plus user-supplied input, preview-approve workflow, deterministic seeded reference evaluator, immutable evaluation artifacts, restart/reopen and backup/restore survival. Design: `docs/handoffs/M10-somnia-slice1-design.md`; delivery record: `docs/handoffs/M10-somnia-slice1-addendum.md`. AN-015 advanced `planned` → `in progress`. Deferred, not claimed: §45 seeded nightmare generation, §47 consolidation, §48 lesson promotion, §49 scheduling, §50 dashboard aggregates, §51 Forge integration, historical trace ingestion, provider-backed evaluation. Recommended next slice: §45 seeded nightmare generation + §47 review-only consolidation proposals + §87.9 before/after comparison.
