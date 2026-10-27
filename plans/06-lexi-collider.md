# M8 — LEXI-COLLIDER

Current independent Windows review of `ffbeee40a` requires acceptance repair
before promotion or Slice 3: six backend probes and one source-identity browser
probe fail. See [Windows evidence](../docs/reviews/windows-m8-slices1-2.md) and
[the bounded repair task](../docs/handoffs/M8-slices1-2-Windows-repair.md).
Positive delivered tests and Linux history below do not waive these failures.

Status: tracked planning task; detailed implementation design and acceptance approval required before coding. This is not an implemented feature or a complete technical design.

Slice 1 (text-first, owner-authorized 2026-10-27, branch `muse/m8-slice1-collider`, tip `8718df8a`): delivered — two text beams, six-section validated output, immutable experiment save and restore on the M5 substrate; Linux verification complete (core 268/272, providers+API 144/0, node 73/73, browser 13/13); Windows verification pending. URL fetching, document ingestion, saved-project/memory selection, 3D visualization and approved Forge transfer remained deferred. Design: `docs/handoffs/M8-slice1-collider-design.md`; record: `docs/handoffs/M8-slice1-collider-addendum.md`.

Slice 2 (explicitly approved collider → Forge transfer, owner-authorized 2026-10-27, branch `muse/m8-slice2-forge-transfer`): delivered — model-free `collider-transfer` op binding an exact experiment version + hash + selected combination + user-approved brief into a durable concept artifact; single-concept support in Forge spec-start; idempotent resubmission; fail-closed restore remapping; transfer panel and concept branch in the Projects UI. Linux verification complete (core 276/280, providers+API 144/0, node 76/76, browser 14/14); Windows verification pending. Design: `docs/handoffs/M8-slice2-forge-transfer-design.md`; record: `docs/handoffs/M8-slice2-forge-transfer-addendum.md`.

Acceptance repair (owner-authorized 2026-10-27, branch `muse/m8-acceptance-repair`, from Windows review tip `c50de23b`): version-1 transfer-approval receipt + Forge recheck before provider traffic; full model-field restore validation with receipt remap/fix-up; transactional cross-process idempotency with no schema change; verbatim beams/briefs; source-keyed panels with snapshot-bound responses. Linux verification complete (core 285/289, providers+API 144/0, node 78/78, browser 16/16); Windows rerun pending. Design: `docs/handoffs/M8-acceptance-repair-design.md`; record: `docs/handoffs/M8-acceptance-repair-addendum.md`. Neither slice nor this repair declares M8 complete or waives M7's open native acceptance, execution environment or production gates.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 23–27, 65–67, 78–80; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Six-stage pipeline, versioned outputs, approved Forge transfer, real-state 3D and accessible alternative. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. M0 is the current implementation phase; do not start this milestone during reconciliation.
