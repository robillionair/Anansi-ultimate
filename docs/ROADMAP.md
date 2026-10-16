# ANANSI Ultimate — Approved Development Roadmap

Approved planning baseline: user reconciliation instruction, 2026-10-16. Product scope remains [MASTER_SPEC.md](MASTER_SPEC.md); its section 82 numbering is retained. M0 is verified complete. The user continuation authorizes M1, whose detailed implementation plan now exists. Work one gated task at a time; stop before M2. Later milestone plans remain tracked design tasks.

| Milestone | Scope retained | Plan |
|---|---|---|
| M0 — Source protection and recovery | Repository integrity, CI, lockfile, local and remote source recovery, shared architecture preparation | [PHASE-000-foundation.md](../plans/PHASE-000-foundation.md) |
| M1 — Windows desktop .exe and web UI foundation | Tauri 2, React, shared contracts/UI, SQLite migrations, local workspace, navigation, settings, browser API boundary; development NSIS .exe | [PHASE-001-runnable-shell.md](../plans/PHASE-001-runnable-shell.md) |
| M2 — Memory OS | Memory lifecycle, notes, projects, tasks, people, organizations, decisions, events, documents, relationships, Markdown, backups and restore | [02-memory-system.md](../plans/02-memory-system.md) |
| M3 — Search and local AI | FTS5, authorized local runtimes/hardware detection, Ask ANANSI, streaming, cancellation, grounded citations; minimum context/privacy/cost enforcement | [03-search-and-ollama.md](../plans/03-search-and-ollama.md) |
| M4 — Semantic intelligence | Embedding evaluation, vector/hybrid retrieval, reranking, complete Context Compiler and disclosure manifest; Big Brain tracked design | [PHASE-004-semantic-intelligence.md](../plans/PHASE-004-semantic-intelligence.md) |
| M5 — AI Matrix and OAuth | Official provider verification, OAuth/API keys, vaults, routing, quotas, Zero-Cost Lock, custom/enterprise endpoints | [04-ai-matrix.md](../plans/04-ai-matrix.md) |
| M6 — Innovation Studio foundation | Opportunity Engine, 5–10 ideas, scoring, universal product projects and versioned artifact registry | [PHASE-006-innovation-studio.md](../plans/PHASE-006-innovation-studio.md) |
| M7 — Product Forge | One approved sandboxed application template first; specs, repository, generation, tests/build, preview, revision, export; remaining product types tracked | [05-product-forge.md](../plans/05-product-forge.md) |
| M8 — LEXI-COLLIDER | Six-stage pipeline, versioned outputs, approved Forge transfer, real-state 3D and accessible alternative | [06-lexi-collider.md](../plans/06-lexi-collider.md) |
| M9 — SwarmTank 3D | Synthetic personas, seeded decisions, sensitivity controls, reports, accessible 2D/3D, approved product improvement loop | [07-swarmtank.md](../plans/07-swarmtank.md) |
| M10 — SOMNIA | Isolated redacted replay, nightmares, baseline/candidate/held-out tests, recoverable proposals, consented scheduling | [08-somnia.md](../plans/08-somnia.md) |
| M11 — GHOST-PROMPT | Off-by-default editor consent, bounded in-memory revisions, approved comparisons, deletion verification, accessible 2D/3D | [09-ghost-prompt.md](../plans/09-ghost-prompt.md) |
| M12 — Integrated product lifecycle and Launch Studio | Discover-to-launch guided project flow, version sharing/rollback, Launch Studio deliverables and explicit publishing approval | [PHASE-012-product-lifecycle.md](../plans/PHASE-012-product-lifecycle.md) |
| M13 — MCP and integrations | All specified document formats, local OCR/transcription where possible, resumable jobs, connectors, authenticated MCP client/server | [PHASE-013-integrations.md](../plans/PHASE-013-integrations.md) |
| M14 — macOS and ANANSI Core | Native Apple Silicon, Keychain, MLX where supported, authenticated pairing/headless operation; enterprise isolation design gate | [PHASE-014-core-and-macos.md](../plans/PHASE-014-core-and-macos.md) |
| M15 — Security, performance and release certification | Signed Windows/macOS installers and updates, browser release, security/accessibility/performance/recovery tests, complete workflow certification | [10-release.md](../plans/10-release.md) |

Dependencies follow section 82: each milestone requires the preceding foundation and its relevant verified contracts. Privacy, permissions, provenance, accessibility, cost limits, recovery and export apply when each feature is introduced; M15 certifies them rather than delaying enforcement. M3 must implement minimum context and spending checks before inference even though full Context Compiler and AI Matrix arrive in M4/M5. M6 introduces durable job/artifact contracts; earlier asynchronous operations require their own recoverable job slice from M2/M3.

M1 includes a buildable/installable development Windows NSIS `.exe` and working browser foundation. It does not satisfy signed production release, updates or clean-machine certification, retained in M15. M1 also includes SQLite/workspace foundations from section 82, not merely a navigation mockup.

## Previous roadmap mapping

| Previous docs/ROADMAP milestone | Canonical milestones | Resolution |
|---|---|---|
| M0 Foundation | M0 | Retained, source recovery and CI made explicit |
| M1 Runnable shell | M1 | Add SQLite/workspace/settings and development installer; preserve browser parity |
| M2 Core readiness slice | M1, M2, M14 | Contracts in M1; usable memory in M2; actual appliance/pairing in M14 |
| M3 Memory/search combined | M2, M3, M4 | Separate memory, FTS5/local AI and semantic retrieval to honor dependencies |
| M4 Ask/context | M3, M4 | Minimum authorized context before inference; full compiler in M4 |
| M5 Desktop release | M1, M15 | Early development .exe, final signed release certification |
| M6 Providers/tools/Big Brain | M4, M5, M13 | Big Brain design in M4; providers in M5; MCP in M13 |
| M7 Advanced workflows | M7, M8, M10, M11, M12 | Independent milestones with integrated lifecycle retained |
| M8 SwarmTank | M9 | Full synthetic model/report and 2D parity included |
| M9 Enterprise hosting | M5, M14, M15 | Enterprise endpoints in M5; future tenancy/hosting design tracked in M14; no multi-user claim before isolation review |

## Existing plan mapping and conflicts

Legacy numeric filenames are stable references, not canonical milestone IDs: 00→M0, 01→M1, 02→M2, 03→M3/M4, 04→M5, 05→M7, 06→M8, 07→M9, 08→M10, 09→M11, 10→M15. New files cover M4/M6/M12/M13/M14. The two missing PHASE-000/001 targets now exist.

The previous roadmap omitted dedicated Innovation Studio, integrated lifecycle/Launch Studio, integrations and macOS milestones and conflated semantic search with Memory OS. These scopes are now explicit. The old plans also treated SOMNIA requirements as absent and GHOST-PROMPT as prompt transformation; the restored specification defines isolated evaluation and explicit editor revision consent instead. All plans now cite the authoritative behavior.

The specification’s empty-repository/start-now wording (sections 1/90) describes the original greenfield start, not authority to replace this existing repository or bypass phase gates. Its section 28 supported product types exceed section 82’s single-template Forge MVP: M7 must track every type and design gated follow-up slices, without claiming the MVP covers all types. Big Brain is named in section 8 but lacks a behavior specification: M4 must design and resolve it before implementation. Section 81 lists potential commercial tiers; enterprise architecture is retained as future gated design, not invented mandatory billing or a deployed service. “Protected main branch” is a protection requirement on the existing default `master`, not a mandate to rename it. See [DECISIONS.md](DECISIONS.md).

All 90 original sections are indexed in [SPEC_COVERAGE.md](SPEC_COVERAGE.md). No product requirement is marked complete by this reconciliation.
