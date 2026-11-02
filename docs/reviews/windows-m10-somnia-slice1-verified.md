# M10 SOMNIA Slice 1: Deterministic Replay Evaluator, Save Op & Restore Gate — independent Windows review — 2026-11-02

Incoming delivery: `anansi-m10-somnia-slice1-2026-11-02.zip`
- Delivery ZIP SHA-256: `0363dc59f08c238484774112b2ba0301b7a323f6ed02f751859cebf5ca2c321a`
- Extracted bundle SHA-256: `6df8a4e705a4551f44637cff2753131e5b962cfd6b4f745b8f9ba49b7c0175bb`
- Delivery tip commit: `8ba785aebb120e98eee9b7698a1d7eb11c5a3395`
- Reviewer baseline: `68ee353291d2a5732ccdeaa340807590e502b572` (Windows-verified M9 SwarmTank review tip)
- Review branch: `codex/review-m10-somnia-slice1`
- Ancestry: Strict linear progression directly from baseline `68ee3532`.
- Scope: 24 files changed, +5592 insertions, -8 deletions (including generated contracts, JSON schemas, validators, client methods, core engine, restore gates, and e2e specs). Zero release-tooling edits. Credentials check clean.

## Acceptance Scope & Findings

1. **Deterministic Reference Replay Evaluator (§42–51, §72–74, §87)**:
   - Implemented in Rust `crates/core/src/somnia.rs` and wired into `crates/api/src/lib.rs` and `crates/core/src/provider_dispatch.rs`.
   - Pure, deterministic, model-free evaluation algorithm enforcing rules R1–R5 strictly in order:
     - **R1 (Untrusted Retrieved Instruction)**: Scenarios containing untrusted retrieved records immediately refuse without counting simulated tool calls.
     - **R2 (Conflicting Instructions)**: Contradictory instructions trigger refusal.
     - **R3 (Missing Evidence)**: Unresolved required evidence citations refuse with first missing evidence ID.
     - **R4 / R5 (Tool Event Evaluation & Error/Timeout Abort)**: Walks simulated tool events; first error or timeout aborts execution with no automatic retry, correctly counting preceding successful simulated calls.
   - Per-decision constraint evaluation:
     - `no-untrusted-execution`, `evidence-required`, `max-tool-calls`, and `disallow-external-side-effects` constraints applied to every completed or aborted decision without duplicating existing refusals.
   - Wall clock evaluation time is kept ephemeral in the evaluation response and excluded from canonical hashing so document hashes remain 100% deterministic and reproducible across runs and architectures.

2. **Immutable Save Operation & Server-Side Recomputation (§9, §43)**:
   - Registered new artifact type `somnia-evaluation` in `crates/core/src/artifacts.rs`.
   - `save_op` performs full server-side recomputation from embedded scenario inputs before writing to disk; tampered evaluation documents fail closed and register nothing.
   - Saved artifact format: `anansi-somnia-evaluation` v1 with SHA-256 content hashing.

3. **Tamper-Resistant Restore Gate (§10, §43)**:
   - Added `validate_original_somnia_bindings` in `crates/core/src/restore.rs`.
   - Restore recomputes all decisions and metrics self-contained from embedded inputs; any hash or decision tampering fails closed, blocking corrupted backup restoration.
   - Restored evaluations survive workspace backup and fresh restore cycles with provenance intact.

4. **UI Panel & Workflow Integration (§42, §43, §47)**:
   - Project-scoped `SomniaPanel` mounted in `packages/ui/src/Projects.tsx` under dedicated `#project-somnia-heading`.
   - Preview-approve-run-save workflow: users supply synthetic/redacted scenario input, inspect the parsed preview, explicitly trigger evaluation, review findings, and save.
   - Sealed input refusal: sealed memory or evidence records fail closed at preview, evaluate, and save steps.
   - Honesty disclosures: unmeasured metrics (model tokens, model accuracy, production resilience, real latency) are clearly presented as "Not measured" rather than zero-filled or fabricated.
   - Regression repair: resolved button label collision with M6 evaluation panel by renaming action to "Save SOMNIA evaluation".

## Windows Verification Results

- Static analysis (TypeScript `tsc --noEmit`): **0 errors (PASS)**.
- Contract drift (`node scripts/contracts.mjs --check`): **PASS** (zero drift).
- Documentation integrity (`node scripts/validate-docs.mjs`): **PASS** (222 Markdown files validate).
- Component & UI test battery (`npm run test:ui`): **159/159 PASS** in 6.41s (152 preserved + 7 new SOMNIA tests).
- Production Web build (`apps/web` Vite production build): **PASS** in 2.23s.
- Rust unit tests on Windows:
  - `cargo test -p anansi-core --lib somnia`: **12/12 PASS** in 0.00s.
  - Full workspace test battery: **583 passed**, 0 failed, 4 known live-Ollama environment failures (`semantic_workflow.rs`, identical to baseline).
  - `anansi-api.exe` rebuild: **PASS** (compiled in 1m 54s).
- Browser Playwright battery under shared test lease:
  - `tests/e2e/somnia-slice1.spec.ts`: **2/2 PASS in 35.8s** (preview, approve, run, save, restart, backup/restore; abandonment/sealed refusal; zero provider traffic).
  - Regression specs (`evaluations.spec.ts`, `integrated_workflow.spec.ts`, `opportunities.spec.ts`): **3/3 PASS in 1.3m** (verifying button label collision resolution).
  - SwarmTank regression specs (`swarmtank-scenario.spec.ts`, `swarmtank-arena.spec.ts`, `swarmtank-slice1.spec.ts`): **7/7 PASS in 2.3m**.
  - Total Playwright browser tests: **12/12 PASS**.

## Milestone Conclusion & Gate Status

- **M10 SOMNIA Slice 1 is verified on Windows NTFS**.
- Requirement `AN-015` (SOMNIA Replay Engine) advances to **in progress (Slice 1 verified)**.
- Preserved non-goals and deferred scopes:
  - Nightmare generation (Slice 2)
  - Consolidation proposals and before/after comparison (Slice 3)
  - Scheduling, lesson promotion, dashboard aggregates, and historical ingestion
  - Provider-backed simulation
- Remaining global gates: M7 native desktop acceptance + preview promotion, Section 32 execution sandbox, M5 production gates, Windows verification of M8 Slice 4.
