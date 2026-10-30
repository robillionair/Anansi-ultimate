# M8 — LEXI-COLLIDER

Current checkpoint: Slice 3 has focused Windows acceptance on
`codex/review-m8-slice3`, including local correction `f54e46695` (reviewed revision
consent and source revision-label validation), regression strengthening and exact
project-identity test synchronization. Incoming Collider 19/19 and transfer 18/18,
corrected binding units 2/2, Forge restore 4/4, UI/contracts 81/81 and final browser
3/3 passed. [Current Windows evidence](../docs/reviews/windows-m8-slice3.md)
records exact trees and diagnostics. Use the corrected review ref, not incoming
`776259a4` alone. Native promotion and remaining M8/production scope stay open;
this review implements no additional slice.

### Earlier Slices 1–2 acceptance checkpoint

Focused Windows acceptance of the repair is passed, including the local receipt
integrity correction `5b3cf958f`: incoming transfer 16/16 and exact-beam 1/1,
new corruption probe refuses with zero provider hits, final browser 4/4 covering
identity, two-process atomicity, Forge, restart and restore. See
[current Windows evidence](../docs/reviews/windows-m8-acceptance-repair.md).
Use `codex/review-m8-acceptance-repair` including that correction as the next
baseline. The earlier [red review](../docs/reviews/windows-m8-slices1-2.md) and
[repair assignment](../docs/handoffs/M8-slices1-2-Windows-repair.md) are historical.
Native acceptance/promotion remains pending. The later owner-authorized Slice 3
assignment and current review above supersede that earlier phase boundary.

### Slice history and current status

Status: Slices 1–4 implemented; Slices 1–3 accepted in the focused Windows scope above,
Slice 4 Linux-verified with Windows verification pending;
the complete milestone remains in progress. Further slices require bounded design
and acceptance criteria before implementation.

Overnight hardening (owner-authorized 2026-10-30, branch `muse/m8-overnight-hardening`, from Slice 4 tip `0b6ba0e8`): WP1 new-dispatch source-eligibility gate (archived/sealed/expired/reassigned heads refuse before provider traffic in both pickers; eligible older revisions still dispatch; saved history untouched) and repeated-restore survival for structured project-context artifacts via recorded `restoredContentHash` with a fix-up before the transfer fix-ups; WP2 project-picker reliability (paged lists past the 50-row default, list errors with retry instead of empty, privacy labels in pre-send reviews, cross-picker duplicate and 8-item guards, bounds warnings blocking submit, review retry, hidden-item counts); WP3 accessible pipeline overview (real run states only, six sections with relationships, section links when complete, static diagram plus full text list, provenance link; 3D retained as unfinished); WP4 integrated browser acceptance (selection → review → collision → save → approved transfer → restart → backup/restore → reopen → second restore, with snapshot and unapproved-brief negatives). Linux verification: collider 34/34, transfer 19/19, forge_restore 4/4, backup_restore 7/7, node pass, Clippy/fmt/tsc/contracts/docs/builds clean, 21 affected browser tests passing, full core 345/4 (only the 4 known live-Ollama environment failures), providers+API 144/0. Windows verification of Slice 4 still in progress; Windows verification of this checkpoint not started. Records: `docs/handoffs/M8-overnight-hardening-wp1-acceptance.md`, `docs/handoffs/M8-overnight-hardening-wp2-acceptance.md`, `docs/handoffs/M8-overnight-hardening-wp3-acceptance.md`, `docs/handoffs/M8-overnight-hardening-wp4-addendum.md`.

Windows repair of that checkpoint (owner-authorized 2026-10-30, branch `muse/m8-overnight-windows-repair`, from reviewer tip `240af4ef`): R1 allowed-transform provenance (restored hash untrusted), R2 global fixpoint pinned hashes, R3 instance-scoped collider headings/links/disclosure. Linux: reviewer probes 2/2 + 1/1, full core 349/4 (only known live-Ollama environment failures), providers+API 144/0, node 90/90, 23 affected browser tests passing. Independent Windows rerun pending; M8 remains in progress.

Slice 1 (text-first, owner-authorized 2026-10-27, branch `muse/m8-slice1-collider`, tip `8718df8a`): delivered — two text beams, six-section validated output, immutable experiment save and restore on the M5 substrate; Linux verification complete (core 268/272, providers+API 144/0, node 73/73, browser 13/13); Windows verification pending. URL fetching, document ingestion, saved-project/memory selection, 3D visualization and approved Forge transfer remained deferred. Design: `docs/handoffs/M8-slice1-collider-design.md`; record: `docs/handoffs/M8-slice1-collider-addendum.md`.

Slice 2 (explicitly approved collider → Forge transfer, owner-authorized 2026-10-27, branch `muse/m8-slice2-forge-transfer`): delivered — model-free `collider-transfer` op binding an exact experiment version + hash + selected combination + user-approved brief into a durable concept artifact; single-concept support in Forge spec-start; idempotent resubmission; fail-closed restore remapping; transfer panel and concept branch in the Projects UI. Linux verification complete (core 276/280, providers+API 144/0, node 76/76, browser 14/14); Later focused Windows verification is recorded above. Design: `docs/handoffs/M8-slice2-forge-transfer-design.md`; record: `docs/handoffs/M8-slice2-forge-transfer-addendum.md`.

Acceptance repair (owner-authorized 2026-10-27, branch `muse/m8-acceptance-repair`, from Windows review tip `c50de23b`): version-1 transfer-approval receipt + Forge recheck before provider traffic; full model-field restore validation with receipt remap/fix-up; transactional cross-process idempotency with no schema change; verbatim beams/briefs; source-keyed panels with snapshot-bound responses. Linux verification complete (core 285/289, providers+API 144/0, node 78/78, browser 16/16). Focused Windows acceptance of the repair passed on the reviewed tip `4d3f8bfd` including local receipt-integrity correction `5b3cf958f` (see the review record at the top of this plan). Design: `docs/handoffs/M8-acceptance-repair-design.md`; record: `docs/handoffs/M8-acceptance-repair-addendum.md`. Neither slice nor this repair declares M8 complete or waives M7's open native acceptance, execution environment or production gates.

Slice 3 (explicitly selected memory context, owner-authorized 2026-10-28, branch `muse/m8-slice3-memory-context`, from reviewed baseline `4d3f8bfd`): optional memory picker in an existing project (nothing selected automatically, no inference on select/review/change), pre-send review of exact revisions, backend-only reference resolution with Sealed/inaccessible refusal before provider traffic, generation-time provenance pinned on the experiment, fail-closed restore remapping with a transfer experiment-hash fix-up (including a fix-up ordering fix found by the new transfer test). Linux verification complete (core 294/298 with only the 4 known environmental live-Ollama failures, providers+API 144/0, node 81/81, browser 17/17); Later focused Windows verification is recorded above. Design: `docs/handoffs/M8-slice3-memory-context-design.md`; record: `docs/handoffs/M8-slice3-memory-context-addendum.md`.

Slice 4 (explicitly selected saved-project input, owner-authorized 2026-10-30, branch `muse/m8-slice4-project-context`, from `c7327bb7`): project picker in an existing project's Collider (nothing selected automatically, no inference on select/review/change), explicitly chosen member memories at pinned revisions and artifacts at pinned versions from one saved project in the current workspace, pre-send review of exact versions with source identities and privacy labels, backend-only reference resolution immediately before dispatch with Sealed/foreign/other-project/archived/corrupted/duplicate/approval-row refusal before provider traffic, generation-time provenance pinned on the experiment, fail-closed restore validation and remapping (project, memory, revision and artifact ids), server-side `projectId` memory-list filter, and no new transfer fix-up (the existing experiment-hash fix-up already covers the remapped experiment). Linux focused verification: collider 24/24, transfer 19/19, forge_restore 4/4, backup_restore 7/7, node 83/83, strict Clippy/fmt/tsc/contracts/docs/builds clean, new project-context e2e spec passing standalone; full canonical e2e battery 34/35 with the single failure proven pre-existing on pristine `c7327bb7` (real-semantic search spec, same assertion); full core 334/4 (baseline 328/4 + 6 new tests; failures only the known live-Ollama environment ones); providers+API 144/0. Windows verification pending. Design: `docs/handoffs/M8-slice4-project-context-design.md`; record: `docs/handoffs/M8-slice4-project-context-addendum.md`.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 23–27, 65–67, 78–80; [canonical roadmap](../docs/ROADMAP.md).

Required scope: Six-stage pipeline, versioned outputs, approved Forge transfer, real-state 3D and accessible alternative. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. Earlier reconciliation-era restrictions describe their historical checkpoint; the Slice 3 review above is historical evidence, and the later owner-authorized Slice 4 assignment (see Slice history) is the current implementation scope, with its Windows verification still open.
