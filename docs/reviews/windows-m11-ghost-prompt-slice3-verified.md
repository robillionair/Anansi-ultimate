# M11 GHOST-PROMPT Slice 3: Draft Analysis, Fragment Selection, Dual-Panel Comparison & Milestone 11 Closure — independent Windows review — 2026-11-03

Incoming delivery: `anansi-m11-ghost-prompt-slice3-2026-11-03.zip`
- Delivery ZIP SHA-256: `2823aeec5fb39fada8e4b634e290593761b8cd37b3c4ffffddca84eb486e9c6d`
- Extracted bundle SHA-256: `d4717c54053cff1851c26c77cfd920418d805791391a80998d72a2b1eeeabf4f`
- Delivery tip commit: `03eb38001a99aad7e73703e064d0b385fb263498`
- Reviewer baseline: `3698833646bd92353791a5948d823e05e73c1144` (Windows review tip of Slice 2)
- Review branch: `codex/review-m11-ghost-prompt-slice3`
- Ancestry: Strict linear progression directly from baseline `36988336`.
- Scope: 14 files changed, +1855 insertions, -30 deletions. Zero release-tooling edits. Credentials check clean.

## Acceptance Scope & Findings

1. **Deterministic Draft Analysis ("Review My Draft" — §57, §86.5)**:
   - Implemented in Rust `crates/core/src/ghost_prompt.rs` and mirrored in TypeScript `packages/ui/src/ghostPrompt.ts`.
   - Analyzes retained text and deleted fragments against four evolution categories:
     - Price modifications (numeric currencies and adjectives like "cheap", "free")
     - Privacy removals (dropped keywords like "privacy", "confidential", "local", "sealed")
     - Audience changes (shifts in persona/market qualifiers like "freelancers", "enterprise", "lawyers")
     - Technical term replacements (substitutions of technical terminology)
   - Outputs objective, factual hypothesis statements (§52) using the verbatim §57 template phrases. Zero subconscious or intent inference claims.

2. **Explicit Fragment Selection (§57, §86.6)**:
   - Renders the exact §57 inquiry: *"Would you like ANANSI to consider any of these earlier alternatives?"*
   - Offers granular checkbox selection for earlier alternatives.
   - Guardrail verified: unselected fragments are never incorporated into Panel B or sent to any model dispatch (proven by `tests/e2e/ghost-prompt-slice3.spec.ts` test 2).

3. **Dual-Panel Comparison Mode (§58, §86.7)**:
   - Side-by-side comparison studio mounted in `packages/ui/src/GhostPromptCompare.tsx`:
     - **Panel A: "Final Prompt Only"**
     - **Panel B: "Final Prompt + Approved Draft Context"** (renders explicit `Approved earlier draft context:` block)
   - Displays character count, estimated tokens, and simulated cost deltas.
   - User feedback toggle allows marking which response was more useful.
   - "Combine into updated prompt" populates the editor with approved context without automatic submission.
   - Offline reference responses provided by default; provider calls occur strictly upon explicit user click ("Send both to Ask ANANSI").

4. **Definition of Done Walk & Milestone Closure (§86, §88)**:
   - Full 9-step §86 sequence verified in Playwright:
     1. Enable Draft Exploration -> 2. Type & revise prompt -> 3. Persistent indicator visible -> 4. Inspect fragments table -> 5. Review draft analysis -> 6. Select fragments -> 7. Dual-panel comparison -> 8. Delete session -> 9. Verify wiped memory.
   - Milestone 11 report documented in `docs/handoffs/M11-ghost-prompt-slice3-addendum.md`.

5. **Windows NTFS Compatibility**:
   - All touched and added ghost prompt imports (`GhostPromptCompare.tsx`, `GhostPrompt.tsx`, `ghostPrompt.ts`, `AskAnansi.tsx`) cleanly specify `.tsx` and `.ts` extensions, completely avoiding case-insensitive resolution ambiguity on Windows NTFS.

## Windows Verification Results

- Static analysis (TypeScript `tsc --noEmit`): **0 errors (PASS)**.
- Contract drift (`node scripts/contracts.mjs --check`): **PASS** (zero drift).
- Documentation integrity (`node scripts/validate-docs.mjs`): **PASS** (237 Markdown files validate).
- Component & UI test battery (`npm run test:ui`): **192/192 PASS** in 7.76s (including all 6 new `ghost-prompt-analysis.test.tsx` tests).
- Production Web build (`apps/web` Vite production build): **PASS** in 2.47s (`dist/assets/index-B66_st7p.js` 2,800.18 kB).
- Rust unit tests on Windows:
  - `cargo test -p anansi-core --lib ghost_prompt`: **17/17 PASS** in 0.04s.
  - Full workspace test battery: **609 passed**, 0 failed, 4 known live-Ollama environment failures (`semantic_workflow.rs`, identical to baseline).
  - `anansi-api.exe` rebuild: **PASS** in 2m 33s.
- Browser Playwright battery under shared test lease:
  - `tests/e2e/ghost-prompt-slice3.spec.ts`: **2/2 PASS in 19.8s** (§86 walk end to end; unselected fragments never reach Panel B).
  - Full regression battery (`ghost-prompt-slice1`, `ghost-prompt-slice2`, `somnia-slice1..3`, `evaluations`, `integrated_workflow`, `opportunities`, `swarmtank-scenario`, `swarmtank-arena`, `swarmtank-slice1`): **20/20 PASS in 5.5m**.
  - Total browser battery: **22/22 PASS**.
- Test lease: Released immediately following browser test execution.

## Milestone Conclusion & Gate Status

- **Milestone 11 GHOST-PROMPT is 100% verified and complete on Windows NTFS**.
- All three milestone execution slices are verified:
  - Slice 1: Consent Model, In-Memory Session Engine & Editor Revision Capture (§52–55, §86.1–4, §86.8–9).
  - Slice 2: Three.js 3D Visualization & Accessible Alternative (§56, §65).
  - Slice 3: Draft Analysis, Fragment Selection & Dual-Panel Comparison Mode (§57–59, §86.5–7, §88).
- Requirement `AN-016` (GHOST-PROMPT) is verified and marked complete for Milestone 11.
- Remaining global gates: M7 native desktop acceptance + preview promotion, Section 32 execution sandbox, M5 production gates (including live-provider verification), Windows verification of M8 Slice 4.
