# ANANSI Ultimate — Requirements Matrix

## M8 focused Windows acceptance — 2026-10-27

AN-013 remains in progress. The Slices 1–2 repair closes the earlier approval,
provenance, retry, exact-text and UI source-state failures in focused Windows checks.
The accepted implementation includes local receipt-hash correction `5b3cf958f`:
incoming transfer 16/16, exact-beam 1/1, new integrity probe refuses before traffic,
final browser 4/4 including two-process duplicate prevention and restart/restore.
[Current Windows evidence](reviews/windows-m8-acceptance-repair.md) supersedes the
earlier red report. Native acceptance/promotion, remaining M8 scope and production
gates remain open; no whole-milestone completion is claimed.

## M7 bounded acceptance update — 2026-10-27

AN-012 remains in progress: specification/source generation, explicit exact-version
approval, revision, documented exports, restart and restored reopening have
Windows synthetic native evidence at `bf6f1fd9c` in
[the preview record](handoffs/M7-local-native-preview.md). This does not claim
generated execution, other product templates or production certification.
The [discovery correction](handoffs/M7-preview-discovery.md) makes the implemented
project workflow visible without changing capability or consent boundaries.
Source review additionally has focused Windows backend/browser evidence on the
reviewed `befc66f11` application tree in
[the independent review](reviews/windows-m7-source-review.md), including exact
package handoff/editor identity and restored review provenance. Source-review
native desktop acceptance/promotion remain pending; the prior Slices 1–3 native
evidence is preserved. AN-012 and production/execution gates remain in progress.

## Current acceptance correction — 2026-10-20

M2–M4 remain **in progress**. AN-005 and AN-010 verified labels in the historical table below are superseded: application embedding population/query generation, semantic context/Ask integration, reranking, production privacy/error behavior and real-model acceptance are incomplete. AN-006/009 additionally have real small-model browser and desktop streaming, cited-answer/disclosure and cancellation evidence; restart/timeout and broader acceptance remain pending; AN-011 is design-only. AN-018/019/031/033 require real native/browser vault and encrypted recovery workflows. [M5 readiness](evidence/M5-READINESS.md) and [ownership checklist](handoffs/M2-M4-integration-owner.md) control current acceptance. Historical evidence is preserved, never expanded into unexecuted tests. All master requirement owners remain represented; audit monitor paused, external audit pending, CI/publication deferred.

## Owner amendment — M2.7-Core isolation and M2/M3 integration (2026-10-19)

M2.7-Core initial parser/keys/session/policy/native-store/shadow-preparation subset is implemented and tested in isolation; [Increment 1 evidence](evidence/M2-7.md) records its scope. [Increment 2 evidence](evidence/M2-7-increment2.md) tracks explicit schema-9 encrypted history/graph generations, cross-process admission and epochs, full DEK rotation, recovery acknowledgement and encrypted export/fresh restore. M2.7 native application integration adds verified Windows desktop adapter workflow: [evidence](evidence/M2-7-native-application.md). AN-018/019/031/033 gain core and native desktop coverage; they remain in progress for complete browser authorization and certified M2.8 release.

[Owner risk acceptance](approvals/M2-6-owner-risk-acceptance.md) explicitly supersedes historical G1 pre-implementation stop language below. G1-External-Audit remains open for post-M3/pre-production/commercialization certification; no audit coverage is fabricated. M2.5 source/CI recovery baseline was reverified. G2–G4 are unchanged.

M3.1–M3.5 (2026-10-19) are explicitly authorized scheduling increments for local lexical title and full-body retrieval, hardware profiling and runtime discovery, minimum context compilation, and local Ask ANANSI with end-to-end browser and native parity, with [M3.1 evidence](evidence/M3-1.md), [M3.2 evidence](evidence/M3-2.md), [M3.3 evidence](evidence/M3-3.md), [M3.4 evidence](evidence/M3-4.md), and [M3.5 evidence](evidence/M3-5.md). AN-005 stays in progress for these subsets; vector/hybrid retrieval remains M4. AN-006, AN-009, and AN-010 local offline subsets are verified; cloud multi-provider routing and vector integrations remain M4/M5. All 90 master section owners and 33 requirements remain represented.

M2.6 audit follow-up (2026-10-19): AN-018/019/031 remain in progress. [Direct source identities and report scope exclusions](evidence/M2-6-audit-review.md) add evidence without certifying encryption. The user retained pure Rust, p=4 defaults and the independent-audit gate; no native substitute or requirement weakening is adopted. All existing requirement rows and milestone owners remain intact.

Status vocabulary: `provisional`, `planned`, `in progress`, `verified`, `blocked`, `deferred`. No row may be marked verified without workflow evidence.

| ID | Requirement area | Subsystem | Priority | Dependencies | Milestone | Status | Test / evidence required |
|---|---|---|---|---|---|---|---|
| AN-001 | Windows Tauri 2 installer producing an NSIS `.exe` | Desktop/release | P0 | Toolchain, app shell | M1, M15 | in progress | Actual M1 development NSIS/install/launch/persistence/uninstall: [evidence](evidence/M1-5.md); clean-machine signed release remains M15 |
| AN-002 | Responsive browser application | Web/UI | P0 | Shared UI, local API | M1, M2, M15 | in progress | Browser smoke test and responsive checks; M1.2 readiness evidence: [record](evidence/M1-2.md), full scope still open |
| AN-003 | Shared visual design system | UI | P0 | Product UX spec | M1 | verified | Shared native/browser foundation tokens/components, keyboard/focus/motion/responsive workflows and actual visual review: [M1 record](evidence/M1-5.md); future modules must preserve these standards |
| AN-004 | Memory OS data model and lifecycle | Core/memory | P0 | Master spec, SQLite | M2 | in progress | M2.1 create/read/revise/history and restart: [evidence](evidence/M2-1.md); M2.2 notes/lifecycle/archive/history verified: [evidence](evidence/M2-2.md); M2.3 entities/relationships and schema-6 history/membership repair verified: [evidence](evidence/M2-3.md) |
| AN-005 | Hybrid lexical/vector search | Core/search | P0 | Memory model, FTS5, vector index | M3, M4 | verified | M3.1 verified complete: [evidence](evidence/M3-1.md); M3.2 full-body rebuildable FTS5 index, current-pointer sync, recency, deduplication, grounded citations, and retrieval evaluation benchmark verified: [M3.2 evidence](evidence/M3-2.md); M4.1 local embeddings, M4.2 vector search, M4.3 hybrid retrieval (RRF), and M4.5 cross-edition UI and acceptance verified: [M4.1 evidence](evidence/M4-1.md), [M4.2 evidence](evidence/M4-2.md), [M4.3 evidence](evidence/M4-3.md), [M4.5 evidence](evidence/M4-5.md) |
| AN-006 | Ask ANANSI | AI/orchestration | P0 | Search, providers, context | M3, M4 | in progress | M3.4 grounded answering workflow with citations, truthful missing-evidence handling, offline resilience, data disclosure manifest, and M3.5 integrated browser workflow verified: [M3.4 evidence](evidence/M3-4.md), [M3.5 evidence](evidence/M3-5.md); full multi-provider and vector integration remain M4/M5 |
| AN-007 | AI Matrix | AI/orchestration | P1 | Provider contracts | M5 | planned | Multi-provider routing and failure tests |
| AN-008 | OAuth and API keys | Integrations/security | P0 | Secret storage, auth | M5 | planned | Token lifecycle and redaction tests |
| AN-009 | Local LLMs | AI/providers | P0 | Provider adapter, Ollama contract | M3, M14 | in progress | M3.3 runtime discovery, hardware inspection, tiered recommendations, explicit download consent, and M3.5 integrated workflow verified: [evidence](evidence/M3-3.md), [M3.5 evidence](evidence/M3-5.md) |
| AN-010 | Context Compiler | AI/context | P0 | Memory/search, policy | M3, M4 | verified | M3.4 deterministic minimum context compilation, budget enforcement (4,000 chars), deduplication, and M3.5 integrated disclosure verified: [M3.4 evidence](evidence/M3-4.md), [M3.5 evidence](evidence/M3-5.md); M4.4 multi-chunk section-aware compilation, token budgeting, manifest, and M4.5 cross-edition integration verified: [M4.4 evidence](evidence/M4-4.md), [M4.5 evidence](evidence/M4-5.md) |
| AN-011 | Big Brain | AI/orchestration | P1 | Context Compiler, providers | M4 (design gate), M5 | in progress | Architectural design gate resolved via formal specification (`docs/designs/BIG_BRAIN_SPEC.md` and ADR-0030); deliberative loop implementation tracked in M5: [M4.4 evidence](evidence/M4-4.md) |
| AN-012 | Product Forge | Product/workflow | P1 | Core, permissions | M7, M12 | in progress | First bounded slice delivered: saved concept → specification → plan → validated source package → inert inspection → versioned save → export-for-review (static-web prototype template only; no execution sandbox — §32 export-for-review fallback — so build/test/preview/deployment stay planned as a later approved slice); see [M7 Slice 1 addendum](handoffs/M7-slice1-addendum.md). Slice 2 added bounded package revision with server-computed diff and exact parent/specification references (see [M7 Slice 2 addendum](handoffs/M7-slice2-addendum.md)). Slice 3 added the §33 reviewable export deliverable: version-pinned package + specification, five approved documents with user/model/verified origins, deterministic archive with version/hash manifest, unexecuted status throughout, no license/deployment/build claims (see [M7 Slice 3 addendum](handoffs/M7-slice3-addendum.md)). Slice 4 added model-free source review: version-pinned findings against the pinned specification's acceptance criteria, immutable review artifact with fail-closed restore remapping, findings selection into the existing revision workflow, no inference from opening/editing/saving a review (see [M7 Slice 4 addendum](handoffs/M7-source-review-addendum.md)); Linux verification only, Windows verification pending. |
| AN-013 | LEXI-COLLIDER | Language/workflow | P1 | Core, AI contracts | M8 | in progress | Slice 1 (text-first) delivered: two bounded text beams, six-section validated output, immutable experiment save/restore on the M5 substrate (see [M8 Slice 1 addendum](handoffs/M8-slice1-collider-addendum.md)). Slice 2 delivered: explicitly approved collider → Forge transfer — model-free transfer op pinning exact experiment version/hash + selected combination + user-approved brief into a durable concept artifact, single-concept Forge spec entry, idempotent resubmission, fail-closed restore remapping (see [M8 Slice 2 addendum](handoffs/M8-slice2-forge-transfer-addendum.md)). Acceptance repair delivered: version-1 transfer-approval receipt + Forge recheck before provider traffic, full model-field restore validation, transactional cross-process idempotency without a schema change, verbatim beams/briefs, source-keyed panels (see [M8 acceptance repair addendum](handoffs/M8-acceptance-repair-addendum.md)). URL fetching, document ingestion, saved-project/memory selection and 3D visualization remain deferred; focused Windows acceptance passed with local receipt-integrity correction 5b3cf958f (see [Windows review](reviews/windows-m8-acceptance-repair.md)); native acceptance/promotion and production gates remain open. |
| AN-014 | SwarmTank 3D | Visualization | P2 | Shared UI, Three.js | M9 | planned | Interactive render smoke test and performance budget |
| AN-015 | SOMNIA | AI/workflow | P1 | Memory, local-first policy | M10 | planned | Workflow tests and privacy review |
| AN-016 | GHOST-PROMPT | AI/workflow | P1 | Context policy, providers | M11 | planned | Prompt transformation tests and audit trail |
| AN-017 | MCP | Integrations/security | P1 | Permission model, contracts | M13 | planned | Allow/deny, timeout, and audit tests |
| AN-018 | Privacy and security | Cross-cutting | P0 | All privileged features | M0–M15 | in progress | Threat model, redaction, permission and penetration checks; M1.2 readiness evidence: [record](evidence/M1-2.md), full scope still open |
| AN-019 | Backups and restoration | Core/recovery | P0 | Storage schema, encryption | M0, M2, M15 | in progress | M2.5 workspace/application SQLite snapshots, explicit schedule consent, owned saved-file import, strict preview and fresh-ID restoration have native/browser/core [evidence](evidence/M2-5.md). Encrypted/sealed backup and integrated/release certification remain M2.7/M2.8/M15; the full requirement stays in progress |
| AN-020 | ANANSI Core readiness | Core/platform | P0 | Shared contracts, policy | M1 (contracts), M14 (appliance) | in progress | Core API contract suite and readiness checklist; M1.2 readiness evidence: [record](evidence/M1-2.md), full scope still open |
| AN-021 | Enterprise architecture | Platform/deployment | P2 | Security review, tenancy model | M14 (design gate), M15 (review) | deferred | Architecture review, isolation and migration tests |

## Matrix notes

Milestones now follow the approved canonical roadmap and master section 82. Planned means retained and assigned, not implemented or tested. AN-011 design gate is resolved via `docs/designs/BIG_BRAIN_SPEC.md` and ADR-0030, with deliberative reasoning execution tracked in M5. AN-021 remains deferred as future commercial architecture under section 81; tenancy/isolation and security review are prerequisites, with no enterprise claims allowed. Other rows await their milestone dependencies and detailed plans.

[The full coverage ledger](SPEC_COVERAGE.md) retains all 90 source sections, including requirements absent from the original coarse matrix.

| ID | Additional retained scope | Milestone | Status | Required evidence |
|---|---|---|---|---|
| AN-022 | Source protection, CI, repository recovery | M0 | verified | docs/evidence/M0.md: hygiene, Windows/Linux CI, protected master and actual independent bundle/restore with matching HEAD/tree and passing checks; annotated completion checkpoint |
| AN-023 | Onboarding, local workspace, navigation/settings | M1 | verified | Actual installed native/browser local onboarding/privacy/AI skip, persistence and navigation/settings: [M1 evidence](evidence/M1-5.md); hardware/model/provider setup retained under M3/M5 and AN-009/008 |
| AN-024 | Universal projects, artifacts, versioning/rollback | M2, M6, M7, M12 | in progress | M2.3 project/task/entity data model and versioned reversible relationships verified: [evidence](evidence/M2-3.md); cross-module envelope and artifact lifecycle remain |
| AN-025 | All document formats, OCR/transcription, async ingestion | M13 | planned | Format fixtures, provenance, resumability/cancellation and privacy tests |
| AN-026 | Zero-Cost Lock, routing, budgets/disclosure | M3, M4, M5; all inference milestones | planned | Deny unknown/paid fallback and unauthorized data; disclosure fixtures |
| AN-027 | Innovation/Opportunity Studio and idea scoring | M6 | in progress | 5–10 concepts, transparent hypotheses, consent and saved project workflow. Slices delivered: project-scoped idea generation (Slice 2), transparent rubric evaluation/comparison (Slice 3), and Opportunity Discovery from deliberately supplied sources only (Slice 4) — all saved as immutable project artifacts with full source provenance; see [Slice 4 addendum](handoffs/M6-slice4-addendum.md). Remaining Opportunity Studio scope (Collider/Forge and later studio stages) stays planned under its own milestones |
| AN-028 | Unified lifecycle and Launch Studio | M12 | planned | Section 85 workflow; real approval before publishing |
| AN-029 | Persistent jobs and recovery | M2, M3, M6, M13 | in progress | M2.4 export-handler six states, leases, headless wake-up, cancel/retry/restart, ownership and resource limits verified: [evidence](evidence/M2-4.md); remaining job categories stay in their owning milestones |
| AN-030 | Native macOS/Apple Silicon, updates/credential vaults | M14, M15 | planned | Native signed installation/update and offline verification |
| AN-031 | Encryption, privacy classes, optional audited vault | M2, M15 | in progress | M2.1 privacy and Sealed refusal: [evidence](evidence/M2-1.md); [M2.7-Core Increment 2](evidence/M2-7-increment2.md) records isolated encryption/history/rotation/recovery core coverage; the tested vault-owned search authorization hook is documented in [the M2→M3 handoff](handoffs/M2-to-M3.md). Application integration and native/browser certification remain pending; G1-External-Audit remains open under the explicit Owner engineering exception |
| AN-032 | Performance, accessibility, engine evaluation | M3–M15 | planned | 1k/10k/100k datasets, engine suites, keyboard/2D fallback evidence |
| AN-033 | Export/ownership and synthetic provenance | M2–M15 | in progress | M2.1 readable Markdown/user provenance/Proposed state: [evidence](evidence/M2-1.md); exports/engine scope remain later tasks |

Section-level coverage is complete for planning; bullet-level acceptance coverage is required at each detailed-plan gate. No rows are verified by documentation validation.

M1.0 planning/readiness evidence is in docs/evidence/M1.md. Missing local Windows/Rust prerequisites block native implementation verification; this audit advances no product requirement to verified. The current M1 authorization does not alter later milestone ownership or the retained 90-section coverage ledger.

M1.1 partial progress: Rust/Cargo installed and pinned; actual local metadata compile passes, full MSVC link fails because link.exe/C++ SDK is missing. Prepared admin installation and real native verification commands are tracked, with evidence in docs/evidence/M1.md. No product row is advanced to verified; AN-001/002/003 and the native workflow remain open.

M1.1 resolution: user completed Microsoft installation and local native doctor plus actual compile/link/run passed. Rust/MSVC/SDK prerequisites are verified with logs and SDK-discovery regression coverage in docs/evidence/M1.md. Earlier M1.0/M1.1 missing-tool notes are historical. No desktop/browser/installer requirement is marked verified by this toolchain-only task.

M1.3 coverage: AN-002/003/018/020 now include actual native/browser workspace create/reopen, validated persistent reduced-motion settings, bounded commands and storage-permission/migration failure tests; [evidence](evidence/M1-3.md). AN-023 advances to in progress for this verified subset; onboarding/navigation/provider skip remain M1.4. AN-004 memory, AN-019 automatic application-data backups, AN-031 vault/encryption and AN-001 installer remain planned. No full product row is completed by this increment, and the 90-section ledger remains unchanged.

M1.4 coverage: AN-002/003/018/020/023 include shared first-run intents/privacy/provider skip, grouped navigation, persisted intent/motion, version-1 migration preservation, keyboard/offline/resume workflows and actual desktop/browser observations in [M1.4 evidence](evidence/M1-4.md). These areas remain in progress pending complete milestone/release coverage. Actual hardware/model setup is retained under M3/AN-009, provider/OAuth setup under M5/AN-008, and installer certification under M1.5/AN-001. No future tool is marked implemented by its planned navigation entry.

M1 certification: AN-003 and AN-023 advance to verified for the scoped M1 foundation workflows, now including actual installed-release native/browser acceptance. Hardware/model/provider setup is still retained under M3/M5 and AN-009/008. AN-001 remains in progress for signed clean-machine release, AN-002 for full browser product/release, AN-018 for cross-cutting security and AN-020 for later Core/appliance scope. Memory/AI/vault/data-backup requirements are not advanced by installer certification. Full master coverage remains intact; see [M1.5 evidence](evidence/M1-5.md).

## M2.0 acceptance traceability — planning only

The [detailed plan and source inventory](../plans/02-memory-system.md) assign AN-004 to M2.1–M2.3 (metadata, all six states and ten record/relationship types), AN-019 to M2.5/M2.7 (workspace/application backup, scheduling, integrity, preview and fresh-target restore), AN-024 to M2.3/M2.8 (shared project envelope, later generated registry retained), AN-029 to M2.4/M2.5 (persistent leases, all six job states and restart/cancel/retry), AN-031 to M2.1/M2.6/M2.7 (native privacy, audited-vault design gate and actual encrypted workflows), and AN-033 to M2.2/M2.4/M2.7 (provenance, readable exports, encrypted sealed export and no automatic confirmation).

AN-002/018/020 cross-cutting adapter/isolation/security obligations apply to every M2 task. Existing statuses remain unchanged: planning is not verified product evidence. Vault design approval and implementation remain inside M2 rather than deferred silently. Search/AI/ingestion/engine registry ownership and all 90 master sections/33 requirement IDs are retained. See [planning evidence](evidence/M2-0.md).

## M2.2 implementation traceability

AN-004 gains note CRUD through reversible archive, all six lifecycle states, expiry visibility, supersession and historical comparison/restore with local workflow acceptance, all three implementation CI jobs, protected publication and independent source restore passed at aac601d. AN-002/018/020 retain shared-adapter and isolation obligations; AN-031 privacy downgrade consent and Sealed refusal remain enforced; AN-033 origin metadata and no-automatic-confirmation are preserved. Verification status is in [M2.2 evidence](evidence/M2-2.md). These rows remain in progress: entities/relationships M2.3, exports M2.4, backups M2.5, vault M2.6/M2.7 and integrated certification M2.8 are still required. All 90 sections and 33 requirement IDs remain represented.

## M2.3 implementation traceability

AN-004 and AN-024 gain all 9 user knowledge kinds from Master Spec §9–10 (`memory`, `note`, `project`, `task`, `person`, `organization`, `decision`, `event`, `document`), type-specific attributes (`dueDate`, `completed`, `eventDate`, `projectId`), typed directed relationships (`predicate`, `sourceId`, `targetId`, `metadata`) with same-workspace enforcement, self-loop rejection, composite foreign keys preventing cross-workspace links, Schema 5 forward migration preserving Schemas 1–4, contracts and DTOs across Rust and TypeScript, shared UI views for Projects, Tasks, People, and Knowledge Records, and cross-adapter verification with process restarts. Verification status is in [M2.3 evidence](evidence/M2-3.md). All 90 sections and 33 requirement IDs remain represented.
### M2.3 acceptance repair status

AN-004/AN-024 remain in progress for their full cross-task scope. M2.3 acceptance repair at 5b83338 adds schema-6 immutable edge revisions, honest legacy provenance, reversible archive/restore, scoped optimistic actions, stable historical link snapshots, valid project targets, retained omitted assignments and explicit clearing. Actual native/browser verification covers all eight project-linked kinds, history, task completion, clear/reassign and both process restarts; narrow control readability was inspected and asserted. AN-002/018/020 shared-adapter/isolation and AN-033 provenance obligations are preserved. See [repair evidence](evidence/M2-3.md) for exact local/CI results and source recovery. All 90 sections and 33 requirement IDs remain represented. Jobs/export M2.4, backups M2.5, vault M2.6/M2.7 and integrated certification M2.8 remain required and unstarted.

M2.4 local acceptance: AN-029 persistent export-job recovery and AN-033 portable knowledge export have verified native/browser, core and HTTP evidence in [M2.4 evidence](evidence/M2-4.md). AN-024 history/export coverage is partial; AN-019 full backups/restore stays M2.5 and vault work stays M2.6/M2.7. AN-004/024/029/033 retain their remaining backup, vault, registry and future engine/export scopes. All 90 sections and 33 requirements remain represented; no full Memory OS completion is claimed.
