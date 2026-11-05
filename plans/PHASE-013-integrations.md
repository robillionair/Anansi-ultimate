# M13 — MCP and integrations

Status: In progress. Model Context Protocol (MCP) Slices 1–5 verified and closed on Windows NTFS (stdio client with explicit review-only UI, stdio server exposing 7 tools with live pairing auth, strict privacy gating, and mutation-gated `ideas.create`). Ingestion vertical slice (text/Markdown) underway per PROMPT-MUSE.md. Master status ledger in [docs/evidence/project-status-ledger-2026-11-05.md](../docs/evidence/project-status-ledger-2026-11-05.md).

Authority: [master specification](../docs/MASTER_SPEC.md), sections 12, 66, 68, 71, 74; [canonical roadmap](../docs/ROADMAP.md).

Required scope: All specified document formats, local OCR/transcription where possible, resumable jobs, connectors, authenticated MCP client/server. All applicable cross-cutting constraints and section 82 milestone items remain mandatory.

Dependencies: complete the preceding milestone and verify relevant shared contracts. Before implementation, inventory every bullet in the cited sections, define task-sized vertical slices, exact files, security boundaries, migrations, tests, observable workflow evidence, rollback and acceptance criteria. Record unresolved design choices in decisions; do not invent integrations or claim mocks are live verification.

Planning completion requires requirement-level traceability, explicit deferred sub-slices with rationale, current official dependency/provider verification where relevant, executable verification commands and reviewable Definition of Done. Stop at each approved task boundary. Historical planning baseline follows:
