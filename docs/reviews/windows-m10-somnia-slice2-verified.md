# M10 SOMNIA Slice 2: Seeded Nightmare Generator, Inert Proposals, Before/After Comparison & SwarmTank Copy Repair — independent Windows review — 2026-11-02

Incoming delivery: `anansi-m10-somnia-slice2-2026-11-02.zip`
- Delivery ZIP SHA-256: `cfeaa5e34314ae3680dd3342b616cd94bf9d99c23dd6f8cfab4c414a943eb64d`
- Extracted bundle SHA-256: `ee7e11e2a53b854b497b99c6486bbf51400583fe0a23814d6b377c7f9d0fc815`
- Delivery tip commit: `b9f9edfcc0f2945ae8dec2446c19e13ed7017d51`
- Reviewer baseline: `8ba785aebb120e98eee9b7698a1d7eb11c5a3395` (Tip of verified M10 Slice 1)
- Review branch: `codex/review-m10-somnia-slice2`
- Ancestry: Strict linear progression directly from baseline `8ba785aebb120e98eee9b7698a1d7eb11c5a3395`.
- Scope: 12 files changed, +2727 insertions, -18 deletions. Zero release-tooling edits. Credentials check clean.

## Acceptance Scope & Findings

1. **Seeded Nightmare Engine Generator (§45)**:
   - Implemented in Rust `crates/core/src/somnia.rs` and mirrored byte-identically in TypeScript `packages/ui/src/Somnia.tsx`.
   - Uses deterministic FNV-1a seeding and xorshift32 PRNG to generate reproducible nightmare scenario batches (configurable count, default 10).
   - Covers the seven core failure classes specified in §45:
     1. `conflicting-instructions` (R2 refusal)
     2. `missing-evidence` (R3 refusal with absent evidence ID)
     3. `simulated-tool-error` (R4 abort with prior call accounting)
     4. `simulated-tool-timeout` (R5 abort with prior call accounting)
     5. `untrusted-retrieved-instruction` (R1 refusal, 0 simulated calls)
     6. `cost-limit-exhaustion` (`max-tool-calls` constraint violation without decision change)
     7. `privacy-boundary-challenge` (sealed record fail-closed refusal)
   - Golden fixture test pins identical sequence generation across both Rust and TypeScript engines.
   - Preserves fail-closed privacy boundary: sealed records in generated nightmare batches immediately refuse evaluation until explicitly excluded by the user.

2. **Review-Only Inert Consolidation Proposals (§47, §87.6)**:
   - Evaluator derives structured consolidation proposals at view time from saved evaluation documents.
   - Detects recurring weaknesses:
     - Unresolved evidence gaps -> `flag-missing-evidence`
     - Repeated untrusted sources -> `archive-untrusted`
     - Conflicting instructions -> `supersede-conflict`
     - Recurrent tool bottlenecks -> `procedural-rule`
   - Strict safety compliance: proposals are completely inert and review-only in Slice 2; no mutation op, no database write, and no new artifact type is introduced. Live memories and policies remain untouched and recoverable.

3. **Before / After Comparison Studio (§87.9)**:
   - Dynamic comparison view in `packages/ui/src/Somnia.tsx` comparing any saved evaluation against a baseline run.
   - Side-by-side delta metrics: completion delta, refusal delta, abort delta, simulated tool call count differences, constraint violation deltas.
   - Newly exposed failure classes table highlighting vulnerabilities surfaced by nightmare runs.
   - Deterministic export to JSON and Markdown formats.
   - Restored evaluations survive workspace backup and restore cycles with comparison and proposal structures 100% identical.

4. **SwarmTank Scenario Controls Wording Repair (§39)**:
   - In `packages/ui/src/SwarmtankScenarioControls.tsx`, the button formerly labeled `🤖 Re-run Full AI Simulation` was renamed to:
     `Re-run Deterministic Scenario Simulation`
   - Accompanying description updated to state:
     "Re-run invokes the deterministic model-free PRNG evaluator using the specified seed, through the same evaluate operation as the baseline run. It does not perform live AI inference and does not contact a model provider."
   - Completely resolves misleading phrasing flagged in Work Package A.

## Windows Verification Results

- Static analysis (TypeScript `tsc --noEmit`): **0 errors (PASS)**.
- Contract drift (`node scripts/contracts.mjs --check`): **PASS** (zero drift).
- Documentation integrity (`node scripts/validate-docs.mjs`): **PASS** (224 Markdown files validate).
- Component & UI test battery (`npm run test:ui`): **165/165 PASS** in 5.37s (159 preserved + 6 new Slice 2 tests).
- Production Web build (`apps/web` Vite production build): **PASS** in 2.09s (`dist/assets/index-BaNufGld.js` 2,767.64 kB).
- Rust unit tests on Windows:
  - `cargo test -p anansi-core --lib somnia`: **18/18 PASS** in 0.01s (12 preserved Slice 1 + 6 new Slice 2 tests).
  - Full workspace test battery: **589 passed**, 0 failed, 4 known live-Ollama environment failures (`semantic_workflow.rs`, identical to baseline).
  - `anansi-api.exe` rebuild: **PASS** in 2m 22s.
- Browser Playwright battery under shared test lease:
  - `tests/e2e/somnia-slice2.spec.ts`: **2/2 PASS in 35.8s** (nightmares, sealed exclusion, proposals, comparison, exports, backup/restore identical).
  - `tests/e2e/somnia-slice1.spec.ts`: **2/2 PASS in 29.9s** (scoped sealed assertion verified).
  - `tests/e2e/swarmtank-scenario.spec.ts`: **3/3 PASS in 59.4s** (renamed scenario simulation button verified).
  - Regression specs (`evaluations`, `integrated_workflow`, `opportunities`): **3/3 PASS in 1.1m**.
  - SwarmTank regression specs (`swarmtank-arena`, `swarmtank-slice1`): **4/4 PASS in 1.1m**.
  - Total Playwright tests: **14/14 PASS**.

## Milestone Conclusion & Gate Status

- **M10 SOMNIA Slice 2 is verified on Windows NTFS**.
- Requirement `AN-015` (SOMNIA Replay Engine) advances to **in progress (Slices 1 & 2 verified)**.
- Preserved non-goals and deferred scopes:
  - Automatic proposal application or live memory mutation (§48)
  - Autonomous background scheduling (§49)
  - Dashboard aggregates (§50)
  - Product Forge integration bridge (§51)
  - Provider-backed simulation
- Remaining global gates: M7 native desktop acceptance + preview promotion, Section 32 execution sandbox, M5 production gates, Windows verification of M8 Slice 4.
