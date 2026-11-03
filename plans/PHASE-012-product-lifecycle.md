# M12 — Integrated product lifecycle and Launch Studio

Status: Slice 1 delivered 2026-11-03 (Linux; Windows-verified 2026-11-03, review tip `27c5754ac`); Slice 2 delivered 2026-11-03 (Linux; Windows rerun pending); Slice 3 not commissioned.

## Slice 1 delivered (2026-11-03, Linux; Windows-verified 2026-11-03)

Branch `muse/m12-product-lifecycle-slice1` from verified Windows review head `9222e9698a7f…` (M11 CLOSED on Windows; the review MANIFEST's stated tail `9222e969a5316…` does not exist — discrepancy recorded in the Slice 1 addendum). Foundational pipeline navigator (§60, §62): pure 7-stage engine (Rust `pipeline.rs` + TS `lifecycle.ts` mirror; `lifecycle.rs` already owns the memory-state machine), `PipelineNavigator` stepper in the project view, stage → tool navigation (anchors + Ask-view plumbing for Refine), Launch honestly disabled until Slice 2, "Jump to tool" override on Awaiting rows. Design before code: `docs/handoffs/M12-product-lifecycle-slice1-design.md`; record: `docs/handoffs/M12-product-lifecycle-slice1-addendum.md`. No server op, no new artifact type, no schema change. Linux verification: Rust 616 passed + 4 known Ollama env failures (609 + 7 new pipeline tests), node 198/198, e2e lifecycle spec 2/2, regression battery per MANIFEST. AN-012 stays `in progress`.

## Slice 2 delivered (2026-11-03, Linux; Windows rerun pending)

Branch `muse/m12-product-lifecycle-slice2` from Windows-verified Slice 1 review tip `27c5754ac` (the first upload for this assignment was byte-identical to the Slice 1 delivery, so work was HELD until the actual review package arrived — see the Slice 2 addendum). Launch Studio (§61): deterministic 4-channel marketing copy (Show HN, Product Hunt, X thread, release notes) from parsed concept / spec / simulation / SOMNIA evidence, provider generation only on explicit approval after prompt preview and cost estimate, deterministic client-side stored-ZIP bundle with manifest and zero unstated files, `LaunchStudio.tsx` mounted in the project view when Launch is selected, navigator Launch row now working. Design before code: `docs/handoffs/M12-product-lifecycle-slice2-design.md`; record: `docs/handoffs/M12-product-lifecycle-slice2-addendum.md`. No server op, no new artifact type, no schema change; bundle is download-only. Linux verification: Rust 616 passed + 4 known Ollama env failures (no Rust change), Clippy/fmt clean, node 220/220, e2e lifecycle slice2 spec 2/2, regression battery 26/26. AN-012 stays `in progress`.

Status: tracked planning task; detailed implementation design and acceptance approval required before coding. This is not an implemented feature or a complete technical design.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 20, 41, 51, 59–62, 79–80, 85; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Discover-to-launch guided project flow, version sharing/rollback, Launch Studio deliverables and explicit publishing approval. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. M0 is the current implementation phase; do not start this milestone during reconciliation.
