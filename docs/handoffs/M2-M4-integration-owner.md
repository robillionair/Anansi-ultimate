# M2–M4 integration ownership and acceptance

Owner: ChatGPT, authorized by repository Owner on 2026-10-20.
Integration checkout: `C:/Users/robin/Documents/ChatGPT/Anansi ultimate`, branch `codex/m2-m3-integration`, starting HEAD `5dd40d3`.
Retained M2 checkout: `C:/Users/robin/.codex/worktrees/m2-7-vault-core/Anansi ultimate`, HEAD `bd2e2a9`; preserve unchanged.

All M2–M4 shared adapters, UI, vault, retrieval, storage, contracts and status files belong to this integration task. Other agents must stop overlapping edits and propose committed handoffs. No destructive reset, model download, remote CI invocation or protected publication is authorized.

## Ordered acceptance slices

1. Production native vault: correct Win32 credential flags, guarded buffers on every exit, explicit recovery acknowledgement, responsive commands, UI setup/seal/read/lock/restart with synthetic records but real native controls. Synthetic-provider IPC evidence remains distinct.
2. Vault application parity: origin/workspace/process-bound browser native authorization; no secret DTOs; expiry/epoch and publication race checks; encrypted export/backup/fresh restore; wrong key/tamper/cancel/interruption preserve originals.
3. Local Ask: explicitly selected installed model; successful desktop/browser inference; streaming/cancellation bounded at backend; malformed/offline/timeout/restart; complete context/disclosure; no downloaded model or mock inference claim.
4. Semantic workflow: real-schema document/chunk embedding generation, query embedding, model/revision/dimension binding and transactional invalidation; hybrid/context/Ask integration; reranking distinct from fusion; paraphrase/contradiction/temporal/missing-evidence evaluation.
5. M2.8/regression and readiness: preserve memory/lifecycle/relationships/jobs/backups; record exact commits, command results and actual UI observations in `docs/evidence/M5-READINESS.md`. Deferred CI/publication and independent audit never count as passed.

Native/browser tests require one exclusive shared lease outside both checkouts, with owner/PID/time; do not steal another process's lease. Isolate artifacts and synthetic catalogs. The audit monitor is paused per Owner direction.

## Ask streaming interface increment

Shared files changed: core Ask/context/readiness/dispatch, API route allowlist, contract schema/generated validators/client, shared Ask UI, desktop main-window capability and new AI permission. New operations are `ai/ask-start`, `ai/ask-poll`, `ai/ask-cancel`, `ai/ask-forget`; run capabilities are random, workspace-bound, process-local, limited to four retained runs and never persisted. Native adapter uses its existing blocking command transport. No vault session/key type is added to these DTOs. Other agents must retain this contract when proposing changes. Real browser and desktop small-model acceptance and exact limitations are recorded in M5 readiness evidence. Owner subsequently authorized the recorded Ollama/model installation, superseding the initial no-download boundary for those models only.
