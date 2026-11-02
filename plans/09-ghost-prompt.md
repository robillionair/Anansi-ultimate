# M11 — GHOST-PROMPT

Status: tracked planning task; detailed implementation design and acceptance approval required before coding. This is not an implemented feature or a complete technical design.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 52–59, 65, 74, 78, 86; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Off-by-default editor consent, bounded in-memory revisions, approved comparisons, deletion verification, accessible 2D/3D. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. M0 is the current implementation phase; do not start this milestone during reconciliation.

## Slice 2 delivered (2026-11-02, Linux; Windows rerun pending)

Branch `muse/m11-ghost-prompt-slice2` from verified Windows review head `9ac467fec54c28c957428187d998d73599a6a965` (the assignment's stated full SHA `9ac467fe835c…` does not exist — discrepancy recorded in the Slice 2 addendum). Visual exploration layer (§56, §65): pure deterministic 3D layout, Three.js scene builder (pinned 0.187.0), `GhostPrompt3D` component with WebGL-failure/context-loss fallback to a complete 2D view, synchronous reduced-motion init with toggle, SVG timeline, keyboard selection, detail card, semantic `<ol>` history. Design before code: `docs/handoffs/M11-ghost-prompt-slice2-design.md`; record: `docs/handoffs/M11-ghost-prompt-slice2-addendum.md`. No server op, no new artifact type, no schema change, no Rust change. Linux verification: Rust 603 + 4 known Ollama env failures, node 186/186, e2e ghost Slice 2 spec 2/2, regression battery 20/20. Slice 3 (§57–58) remains deferred and uncommissioned.

## Slice 1 delivered (2026-11-02, Linux; Windows-verified same day)

Branch `muse/m11-ghost-prompt-slice1` from verified Windows review tip `d7887ad82`. Consent model (§53, §74), in-memory session engine (§54–55, Rust reference + TS mirror), and Ask ANANSI revision capture with the Retained & Deleted Fragments inspection table (§86.1–4, §86.8–9). Design before code: `docs/handoffs/M11-ghost-prompt-slice1-design.md`; record: `docs/handoffs/M11-ghost-prompt-slice1-addendum.md`. No server op, no new artifact type, no schema change. Linux verification: Rust 603 + 4 known Ollama env failures, node 178/178, browser 68/69 (single known search real-semantic failure). Windows verification PASSED same day (review tip `9ac467fec54c`, NTFS import fix); Slice 2 delivered 2026-11-02 (Linux; Windows rerun pending); Slice 3 (§57–58) remains deferred and uncommissioned.
