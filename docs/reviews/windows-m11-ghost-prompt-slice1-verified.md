# M11 GHOST-PROMPT Slice 1: Consent Model, In-Memory Session Engine & Editor Revision Capture — independent Windows review — 2026-11-02

Incoming delivery: `anansi-m11-ghost-prompt-slice1-2026-11-02.zip`
- Delivery ZIP SHA-256: `3fc8f005ac821d86a6f122704ae461aac740182cb0dc69dfe0afc91f3c21c4ee`
- Extracted bundle SHA-256: `6a529458b571e16b81e9b8d0adafb56e3cc1598f3e5986d256a0eca20fbba15a`
- Delivery tip commit: `0f9eec261fbe179706975fb399cfa2f6fed22394`
- Reviewer baseline: `d7887ad82ea9069552528e2283e69663c3d0d0c9` (Windows review tip closing Milestone 10)
- Review branch: `codex/review-m11-ghost-prompt-slice1`
- Ancestry: Strict linear progression directly from baseline `d7887ad82`.
- Scope: 14 files changed, +1917 insertions, -6 deletions. Zero release-tooling edits. Credentials check clean.

## Acceptance Scope & Findings

1. **Informed Consent & Privacy Guardrails (§53, §74, §86.1, §86.3)**:
   - GHOST-PROMPT is strictly **OFF by default**.
   - Requires explicit user opt-in via an informed consent modal displaying the verbatim disclosure:
     > *"When enabled, ANANSI can temporarily track revisions made in this editor. Deleted text remains available for your review until you discard it or explicitly save it."*
   - Persistent `Draft Exploration ON` status badge indicates active recording state (`Recording` or `Paused`).
   - Session controls provided: `Pause recording`, `Resume recording`, `Review drafts`, `Clear draft history`, and `Stop & discard session`.

2. **In-Memory Session Engine (§54, §55)**:
   - Implemented in Rust `crates/core/src/ghost_prompt.rs` and mirrored in TypeScript `packages/ui/src/ghostPrompt.ts`.
   - Dual-ended prefix/suffix diffing captures exact deleted spans, lengths, and character offsets on edit events.
   - Factual 2000ms pause events record interval durations without any emotional or psychological inference (§52).
   - Strict size and memory bounds: 500 edit events / 50KB ring buffer dropping oldest entries first; 4-hour automatic session termination.
   - Deterministic content hashing (FNV-1a 64-bit in TS and Rust; SHA-256 in Rust).
   - Password scope refusal: input scopes with `isPassword: true` immediately refuse session creation.
   - Safe in-memory storage: events exist only in memory by default; zero database persistence, zero server operations, zero background telemetry, and zero provider traffic.

3. **Session Deletion & Verification (§53, §55, §86.8–9)**:
   - "Delete Session" or "Clear" immediately zeroes and purges in-memory event buffers.
   - Observable memory wipe confirmed through dedicated unit tests and e2e assertions verifying no deleted fragments remain in memory.

4. **UI Integration in Ask ANANSI (§86.4)**:
   - `GhostPromptStrip` mounted in `AskAnansi.tsx` above prompt input.
   - Collapsible "Retained & Deleted Fragments" table renders retained text, deleted fragments (`<s>` text with offset and content hash), and pause interval events.

5. **Reviewer Case-Collision Correction on Windows NTFS**:
   - On Windows case-insensitive filesystem (NTFS), `packages/ui/src/GhostPrompt.tsx` and `packages/ui/src/ghostPrompt.ts` collided when imported as `./GhostPrompt`. Node's ESM resolution resolved `./GhostPrompt` to `ghostPrompt.ts` (which did not export the React components or consent string), breaking `components.test.tsx` and `ghost-prompt.test.tsx`.
   - Fix: updated imports to explicitly target `./GhostPrompt.tsx` in `AskAnansi.tsx` and `ghost-prompt.test.tsx`, resolving the NTFS resolution collision cleanly while preserving full Linux ext4 compatibility.

## Windows Verification Results

- Static analysis (TypeScript `tsc --noEmit`): **0 errors (PASS)**.
- Contract drift (`node scripts/contracts.mjs --check`): **PASS** (zero drift).
- Documentation integrity (`node scripts/validate-docs.mjs`): **PASS** (231 Markdown files validate).
- Component & UI test battery (`npm run test:ui`): **178/178 PASS** in 4.84s (including 10/10 new ghost-prompt tests).
- Production Web build (`apps/web` Vite production build): **PASS** in 2.58s (`dist/assets/index-CRRtb5Bs.js` 2,784.63 kB).
- Rust unit tests on Windows:
  - `cargo test -p anansi-core --lib ghost_prompt`: **11/11 PASS** in 0.05s.
  - Full workspace test battery: **603 passed**, 0 failed, 4 known live-Ollama environment failures (`semantic_workflow.rs`, identical to baseline).
  - `anansi-api.exe` rebuild: **PASS** in 2m 19s.
- Browser Playwright battery under shared test lease:
  - `tests/e2e/ghost-prompt-slice1.spec.ts`: **2/2 PASS in 15.8s** (consent, revision capture, review, purge, pause/clear).
  - Regression specs (`somnia-slice1`, `somnia-slice2`, `somnia-slice3`, `evaluations`, `integrated_workflow`, `opportunities`, `swarmtank-scenario`, `swarmtank-arena`, `swarmtank-slice1`): **16/16 PASS in 4.2m**.
  - Total Playwright tests: **18/18 PASS**.

## Gate Status & Next Steps

- **Milestone 11 GHOST-PROMPT Slice 1 is verified on Windows**.
- Requirement `AN-016` status is **`in progress`** (Slice 1 verified).
- Preserved non-goals and deferred scopes:
  - Slice 2: Three.js 3D visualization (§56)
  - Slice 3: Semantic advice ("Review My Draft"), dual-panel comparison mode, and provider dispatch (§57, §58)
- Remaining global gates: M7 native desktop acceptance + preview promotion, Section 32 execution sandbox, M5 production gates, Windows verification of M8 Slice 4.
