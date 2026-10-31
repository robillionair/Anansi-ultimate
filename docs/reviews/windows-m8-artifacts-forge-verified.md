# M8 §27 Artifacts & Forge Provisioning — independent Windows review — 2026-10-31

Incoming delivery: `anansi-m8-complete-artifacts-forge-2026-10-31.zip`
- Delivery SHA-256: `5d683ad6269a6bf8c4bfcfcf845c75d3851824fbc5dce8811d50099e211d1cda`
- Extracted bundle SHA-256: `e44d8f41ed3388fa01b0ba34c5fa523a799c8c80a2ba23d62f3440dea350eae0`
- Verified delivery tip: `dbc49bd1689796d978084aacb7d229f5b3201e73`
- Baseline: `7632f94765cbf1f886fd12fb6e59476fb1e8b75b` (§26 Three.js visualization runtime)
- Ancestry: Strict linear progression from baseline `7632f9476`.

## Acceptance Scope & Findings

1. **Deterministic Six-Artifact Derivation (§27)**:
   - Implemented in Rust `crates/core/src/collider_artifacts.rs` and mirrored byte-identically in TypeScript `packages/ui/src/colliderArtifacts.ts`.
   - Derives six markdown documents from the saved experiment record: `CONCEPT.md`, `ARCHITECTURE.md`, `FEASIBILITY.md`, `ROADMAP.md`, `RISKS.md`, `TEST_PLAN.md`.
   - Embedded SHA-256 digests and byte counts in `ColliderExperimentDoc` at save time; recomputed fail-closed during validation, restore, and transfer.
2. **Safe Tabbed Viewer**:
   - `packages/ui/src/ColliderArtifactViewer.tsx`: Accessible tabs for all six artifacts, line-based text search, copy-to-clipboard button, and safe markdown rendering with zero raw-HTML evaluation.
3. **Deterministic Zip Export**:
   - `packages/ui/src/colliderExport.ts`: Deterministic STORE-only zip generator producing `anansi-collider-<id>-v1.zip` containing all six markdown files, `experiment.json`, and `MANIFEST.json`.
   - `MANIFEST.json` records SHA-256 and byte counts that match both the entry bytes and the record's stored digests.
4. **Product Forge Bridge (§28–33)**:
   - `packages/ui/src/ColliderBuildProduct.tsx`: "Build This Product" button transfers approved output combination into Product Forge concept.
   - Sets the record brief as approved brief and derived `ARCHITECTURE.md` as initial technical specification. Structurally idempotent; does not auto-start speculative execution.
5. **Windows Verification Results**:
   - TypeScript compilation (`pnpm typecheck`): **0 errors**.
   - Contract drift (`node scripts/contracts.mjs --check`): **PASS** (zero drift).
   - Component test battery (`pnpm test:ui`): **125/125 PASS** (duration 3.34s).
   - Documentation integrity (`node scripts/validate-docs.mjs`): **PASS** (210 Markdown files, 90 owners, 16 plans, 33 requirements).
   - Production Web build (`apps/web` Vite): **PASS** in 671ms.
   - Backend Rust test suites on Windows:
     - `crates/core/tests/collider.rs`: **49/49 PASS** (duration 434.39s).
     - `crates/core/tests/forge_restore.rs`: **4/4 PASS** (duration 27.90s).
     - `crates/core/tests/transfer.rs`: **19/19 PASS** (duration 80.32s).
     - `anansi-api.exe` rebuild: **PASS**.
   - Browser Playwright vertical slice under shared test lease:
     - `tests/e2e/collider-artifacts-forge.spec.ts`: **PASS** in 37.5s (40.7s total). Covers unapproved live panel gating, six-artifact tab navigation, search filtering, duplicate export byte-identity, manifest hash verification, "Build This Product" Forge concept creation, and backup/fresh restore survival.
6. **Screenshot Evidence**:
   - Visually inspected in `docs/reviews/evidence/windows-m8-artifacts/`:
     - `artifacts-viewer.png`: Clean tabbed interface with search input and contrasting typography.
     - `artifacts-concept.png`: Transferred concept in Product Forge displaying verbatim brief, initial specification, and explicit human approval actions.

## Requirement Coverage & Boundaries

- Satisfies MASTER_SPEC §27 (six versioned artifacts, deterministic export, safe markdown rendering, hash integrity) and bridges into §28 Product Forge concept creation.
- Native Tauri host file save dialog remains deferred (browser download trigger used).
- Shared application-test lease acquired during execution and released cleanly.
