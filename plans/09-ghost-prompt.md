# M11 — GHOST-PROMPT

Status: tracked planning task; detailed implementation design and acceptance approval required before coding. This is not an implemented feature or a complete technical design.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 52–59, 65, 74, 78, 86; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Off-by-default editor consent, bounded in-memory revisions, approved comparisons, deletion verification, accessible 2D/3D. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. M0 is the current implementation phase; do not start this milestone during reconciliation.

## Slice 1 delivered (2026-11-02, Linux; Windows rerun pending)

Branch `muse/m11-ghost-prompt-slice1` from verified Windows review tip `d7887ad82`. Consent model (§53, §74), in-memory session engine (§54–55, Rust reference + TS mirror), and Ask ANANSI revision capture with the Retained & Deleted Fragments inspection table (§86.1–4, §86.8–9). Design before code: `docs/handoffs/M11-ghost-prompt-slice1-design.md`; record: `docs/handoffs/M11-ghost-prompt-slice1-addendum.md`. No server op, no new artifact type, no schema change. Linux verification: Rust 603 + 4 known Ollama env failures, node 178/178, browser 68/69 (single known search real-semantic failure). Slices 2 (§56) and 3 (§57–58) remain deferred and uncommissioned.
