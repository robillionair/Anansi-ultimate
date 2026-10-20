# M5 readiness — integration assessment in progress

Date: 2026-10-20. M5 implementation is not authorized by this task. M2–M4 are not yet locally complete or release-certified.

Inspected checkpoints: integration `5dd40d3`, retained M2 `bd2e2a9`. Both worktrees were clean at inspection. [Ownership/checklist](../handoffs/M2-M4-integration-owner.md).

| Requirement | Observed implementation/evidence | Remaining acceptance |
|---|---|---|
| AN-004 Memory OS; AN-024 project contracts | Existing M2.1–M2.5 workflow evidence retained | Regression after vault/schema integration |
| AN-018/031 vault privacy | Core crypto/generations/session tests; native IPC synthetic-provider workflow | Real native credential/recovery controls, UI workflow, browser authorization, expiry/revocation/publication race |
| AN-019/033 encrypted recovery/export | Core archive/restore functions | Application encrypted export/backup/fresh restore; tamper/wrong-key/cancel/interruption |
| AN-005 retrieval | FTS and synthetic seeded-vector tests | Application document/query embeddings, production schema checks, reranking distinct from RRF, real-model semantic/temporal/contradiction evaluation |
| AN-006/009 local Ask | Discovery, offline and missing-evidence paths | Selected installed runtime/model, successful desktop/browser answers, streaming/backend cancellation/timeouts/restart |
| AN-010 Context Compiler | Separate section compiler; Ask has separate minimum implementation | Unified Ask/compiler policy; semantic query vectors; complete chunks/Unicode/budgets/source lines |
| AN-011 Big Brain | Design document only | Design traceability; no implemented deliberative workflow claim |
| AN-002 cross-edition UI; AN-029 jobs | Earlier scoped workflow evidence | Updated native/browser regression and local M2.8 |

## Current verification

Read-only probes of `http://127.0.0.1:11434/api/tags` and `http://127.0.0.1:1234/v1/models` timed out/refused; `ollama` and `lms` were not found on PATH. No model selected, downloaded or invoked. Real-model acceptance is pending an available Owner-selected model; independent transport/security repairs continue.

New repairs in progress: production provider used `CREDUIWIN_GENERIC` with `CredUIPromptForCredentialsW` (different API flag family); replace with generic/always-show/non-persistent flags, guard cancellation buffers and restrict synthetic bypass to debug builds. Context compilation sliced UTF-8 at arbitrary byte offsets and discarded section tails; repair with bounded UTF-8 chunks and exact content line offsets. Exact verified implementation SHAs and final commands will be recorded after checks and commits.

Remote CI/protected publication: deferred by Owner capacity policy. Independent crypto audit: pending pre-production gate under accepted engineering risk. Audit monitor: paused. None is recorded as passed.

First dependency-safe M5 task: detailed provider/authentication/Zero-Cost-Lock contract inventory and design, after required M2–M4 contracts and local workflows pass. M5 readiness is currently **not established**.
