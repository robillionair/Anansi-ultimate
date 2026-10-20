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
