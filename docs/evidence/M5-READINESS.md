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

## Verified repair checkpoint and follow-up

`bedba8c` records ownership, authoritative status corrections, production Win32 prompt flags/guarded cleanup/debug-only synthetic bypass, complete UTF-8-safe section chunking and hybrid failure propagation. No production prompt interaction is claimed. Computer-use guidance forbids automating authentication dialogs, so production native credential interaction needs manual verification; synthetic provider coverage is retained separately.

Follow-up implementation routes Ask through the shared section Context Compiler, propagates canonical integrity failures, rejects missing workspaces, removes hybrid/context guessed-schema metadata fallbacks, and caps embedding response allocation at 256 KiB. Source commit follows the final checks.

Executed locally (exit 0 unless noted):

- `node scripts/cargo.mjs test -p anansi-core --test context_compiler --locked`: 7 passed, including Unicode content preservation/line offsets and tampered canonical Markdown refusal.
- `node scripts/cargo.mjs test -p anansi-core --test hybrid_retrieval --locked`: 6 passed, including invalid query/corrupt FTS and nonfinite weight refusal. Seeded vectors remain synthetic evidence.
- `node scripts/cargo.mjs test -p anansi-core --test ask_anansi --test vector_embeddings --locked`: 5 Ask and 5 embedding tests passed; inference server is synthetic, not a real selected model.
- `node scripts/cargo.mjs check -p anansi-desktop --locked`: passed.
- `node scripts/cargo.mjs clippy -p anansi-core -p anansi-desktop --all-targets --locked -- -D warnings`: passed.
- `node node_modules/typescript/bin/tsc --noEmit`: passed.
- `node --import tsx --test tests/contracts.test.ts packages/ui/tests/components.test.tsx`: 30 passed.
- Web Vite production build: passed with existing >500 KiB bundle warning.
- `node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test tests/e2e/ask_anansi.spec.ts tests/e2e/search.spec.ts`: 2 passed (1.2 minutes), real browser/API offline Ask/disclosure and title search/update/restart, after rebuilding API. Shared external lease released normally.
- Documentation validation: passed, 74 Markdown documents, 90 source section owners, 16 milestone plans and 33 requirements. Master unchanged.

Initial negative-test input used nine words, which is within the implemented 16-term body-query limit; corrected to 17 terms. Initial combined test execution overlapped recompilation and hit Windows executable sharing error; serialized reruns above passed. No full workspace suite or new native UI acceptance is claimed.
