# ANANSI Milestone 8 & Milestone 9: Evidence & Milestone Reconciliation Report
**Date**: 2026-11-01  
**Lead Reviewer & Release Integrator**: Antigravity (Windows Review Environment)  
**Authority Documents**: `docs/MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, `docs/REQUIREMENTS_MATRIX.md`, `docs/BUILD_STATUS.md`  
**Current Application Baseline**: `68ee353291d2a5732ccdeaa340807590e502b572`  
**Release Integration Branch**: `codex/release-tooling-integration` (rebased over `68ee353291d2a5732ccdeaa340807590e502b572`)  
**Coordination Package**: `E:\Anansi\deliveries\anansi-m9-coordination-baseline-2026-11-01\HANDOFF.md`  

---

## 1. Executive Summary & Purpose

This reconciliation document conducts a formal, independent audit of **Milestone 8 (Lexi-Collider / AN-013)** and **Milestone 9 (SwarmTank 3D / AN-014)** against the normative requirements defined in `docs/MASTER_SPEC.md`. 

While recent delivery notes and summary lines marked Milestone 9 as "complete" or "verified", a rigorous audit of the codebase against `MASTER_SPEC.md` reveals specific feature deferrals, scope boundaries, and an honest terminology conflict regarding deterministic sensitivity versus live AI simulation.

This report establishes:
1. Exact requirement coverage audits against `MASTER_SPEC.md` §23–27 (M8) and §34–41 (M9).
2. Honest classification of all deferred, simulated, and synthetic capabilities.
3. Concrete findings on the misleading "🤖 Re-run Full AI Simulation" button and a bounded repair assignment for Muse.
4. Correction and explanation of the delivery-manifest tip identity discrepancy (`68ee3532` vs `bf79643e`).
5. A structured 4-tier taxonomy distinguishing fresh results, historical results, deferred requirements, and untested claims.

---

## 2. Milestone 8 (AN-013) Audit vs MASTER_SPEC.md (§23–27)

### Specification Scope (§23–27)
`MASTER_SPEC.md` defines Lexi-Collider as a 3D idea-collision and discovery environment:
- **§23 Overview**: Multi-concept collision engine generating novel angles and product opportunities.
- **§24 Input Modalities**: Plain text input, markdown document input, web URL ingestion, and saved project context.
- **§25 Collision Model**: Multi-vector semantic distance, cluster scoring, opportunity synthesis.
- **§26 3D Force-Directed Visualization**: Interactive Three.js node arena, edge tension, repulsion physics, raycast picking, alongside an accessible 2D table fallback.
- **§27 Opportunity Export & Bridge**: Transferring collision outputs into Product Forge concepts.

### Delivered & Verified Capabilities (Windows Evidence)
1. **Plain Text & Markdown Ingestion (§24)**: Strict UTF-8 decode, backend-recomputed SHA-256 provenance, refusal of forged/stale contents, save/restart persistence, and fresh-restore survival.
2. **Saved-Project Context (§24)**: Project picker, member revision pinning, artifact versioning, fail-closed restore validation.
3. **URL Beam Input (§24)**: Verified on Windows (2026-10-31) with address/deadline probes, TLS fragmentation checks, 21 URL integration cases, and 4 Playwright browser workflows.
4. **Lineage & Provenance Integrity**: Lineage regressions (R1–R4) repaired and verified on Windows NTFS with strict tamper rejection.
5. **Product Forge Bridge (§27)**: Approved transfer of collision opportunities to Product Forge concept drafts.

### Deferred / Open Scope for M8
- **3D Visualization (§26)**: The interactive Three.js 3D force-directed layout remains deferred; currently Lexi-Collider renders exclusively via the accessible 2D table fallback and 2D cluster views.
- **Live External Web Scraping (§24)**: Live network HTTP fetching is strictly locked under Zero-Cost Lock / offline policy. All URL tests execute against deterministic local test servers or recorded fixtures.
- **Requirement Status**: Accurately classified as **`in progress`** in `docs/REQUIREMENTS_MATRIX.md`.

---

## 3. Milestone 9 (AN-014) Audit vs MASTER_SPEC.md (§34–41)

### Specification Scope (§34–41)
`MASTER_SPEC.md` defines SwarmTank as a synthetic buyer and market scenario arena:
- **§34 SwarmTank 3D**: Synthetic buyer and market scenario arena.
- **§35 Input**: Product description, landing-page URL, price, target audience, business model.
- **§36 Synthetic Personas**: Configurable synthetic buyer populations (20, 50, 100 personas) with budget, sophistication, price sensitivity, feature priorities, and trust thresholds.
- **§37 Decision Model**: Multi-factor decision engine computing Convert (green), Confused (amber), Reject (red).
- **§38 3D Arena**: Three.js instanced amphitheater rings, radial clustering, raycast card inspection, accessible 2D table fallback.
- **§39 Interactive Scenario Controls**:
  - Sliders/inputs for: Product price, Headline, Value proposition, Target audience, Feature availability, Security claims, Free trial availability, Setup difficulty.
  - "A price slider may update decisions immediately using an explicitly defined sensitivity model. Such updates are synthetic calculations, not fresh LLM evaluations or evidence that real customers changed their minds."
  - "Provide: **Re-run AI Simulation** when the user wants a model-generated reassessment. Keep those two modes clearly separate."
- **§40 SwarmTank Report**: Conversion distribution, price elasticity, objection breakdown, recommendations with assumptions, synthetic evidence, and uncertainty.
- **§41 Product Forge Bridge**: Applying scenario recommendations to Forge concept v3 with scenario diff addenda.

### Delivered & Verified Capabilities (Windows Evidence)
1. **Slice 1 (§34–37, §40–41)**: Deterministic seeded personas (20/50/100), 7-factor decision engine in Rust and TypeScript, report metrics, JSON export, and Forge transfer with idempotent addenda.
2. **Slice 2 (§38, §65)**: Three.js amphitheater arena over saved runs, radial decision clustering, raycast picking, synchronized 2D table fallback, DPR clamp, low-power WebGL context, full scene disposal, and reduced-motion toggle.
3. **Slice 3 (§39–41)**: Interactive price slider with presets, feature toggles, trust signal toggles (SOC2, free trial, SLA, money-back), setup difficulty selector, sub-5ms instant sensitivity re-evaluation, 600ms animated arena re-clustering, comparison studio with transition matrix and price elasticity, versioned scenario branching (`anansi-swarmtank-scenario` v1), and Forge v3 revision with diff addenda.

### Honest Classification of Deferred / Missing M9 Items
While Slice 3 delivered the sensitivity engine, the following items from §39 were deferred or omitted:
1. **Deferred Scenario Controls (§39)**:
   - **Headline Input**: Text input to revise product headline is omitted in UI and contracts.
   - **Value Proposition Input**: Text input to revise value proposition is omitted.
   - **Target Audience Input**: Controls to alter audience demographics/targeting are omitted.
2. **Deferred Provider-Backed AI Simulation (§39)**:
   - §39 specifies two distinct operational modes: instant synthetic calculation via sensitivity model, and **Re-run AI Simulation** via model-generated reassessment.
   - Live model-generated reassessment is deferred under Zero-Cost Lock and local-first offline policy.

---

## 4. Misleading Wording Analysis & Bounded Repair Assignment for Muse

### Inspection of the "Re-run Full AI Simulation" Action
In `packages/ui/src/SwarmtankScenarioControls.tsx` (lines 189–197):
```tsx
        <button type="button" onClick={onRerun} disabled={rerunDisabled}>
          🤖 Re-run Full AI Simulation
        </button>
      </div>
      <p>
        Re-run re-evaluates the scenario inputs through the same
        deterministic evaluate operation as the baseline run, with
        the same seed. It makes no model call.
      </p>
```

### Analysis of the Defect
1. **Direct Contradiction**: The button is badged with an AI robot emoji `🤖` and titled **"Re-run Full AI Simulation"**. However, clicking this button triggers `onRerun`, which merely dispatches the local deterministic evaluator with the identical PRNG seed.
2. **Violation of MASTER_SPEC §39**: Section 39 strictly mandates:
   > *"Such updates are synthetic calculations, not fresh LLM evaluations or evidence that real customers changed their minds. Provide: **Re-run AI Simulation** when the user wants a model-generated reassessment. Keep those two modes clearly separate."*
3. **Violation of Honesty Principles (§67, §73)**: Calling a deterministic local PRNG evaluation "Full AI Simulation" violates ANANSI's core principle of zero fabrication and transparent synthetic labeling. The explanatory caption admits it makes no model call, but the prominent CTA claims the opposite.

### Bounded Repair Assignment for Muse

**Target Files**:
- `packages/ui/src/SwarmtankScenarioControls.tsx`
- Related tests: `packages/ui/tests/swarmtank-scenario.test.ts`, `tests/e2e/swarmtank-scenario.spec.ts`

**Required Changes**:
1. **Relabel Button**:
   - Change button text from `🤖 Re-run Full AI Simulation` to `Re-run Deterministic Simulation` or `⚡ Recompute Baseline Simulation`.
   - Remove the robot emoji `🤖` to prevent false implication of LLM reasoning.
2. **Add Explicit Mode Disclosure Badge**:
   - Above or beside the button, render an honest badge: `[Deterministic Heuristic — Local PRNG Seeded]`.
3. **Handle Provider-Backed AI Mode Honestly**:
   - If a placeholder for the future provider-backed reassessment is displayed, label it:
     `Request Model Reassessment (Deferred / Zero-Cost Mode Active)`
   - Keep it disabled with an explanatory tooltip stating that provider-backed reassessment requires active model credentials and is currently deferred.
4. **Update Spec & Tests**:
   - Update Playwright locators in `tests/e2e/swarmtank-scenario.spec.ts` if they match the exact button text.

---

## 5. Delivery Manifest Identity Discrepancy Reconciliation

### The Discrepancy
- The historical delivery manifest inside `anansi-m9-swarmtank-slice3-2026-10-31.zip` identified delivery commit `bf79643eb64fbefc6da91ae66d33a14ce015cf37`.
- However, after independent Windows verification, the reviewer commit was recorded as:
  **`68ee353291d2a5732ccdeaa340807590e502b572`** (`docs(review): verify M9 SwarmTank Slice 3 on Windows (Milestone 9 complete)`).
- Some references mistyped or conflated the pre-review delivery SHA (`bf79643e`) with the post-review tip (`68ee3532`).

### Reconciliation
- **Authoritative Review Tip**: The true Git HEAD of the application baseline is **`68ee353291d2a5732ccdeaa340807590e502b572`**.
- **Preservation of Evidence**: The original packages (`anansi-m9-swarmtank-slice3-2026-10-31.zip` and `anansi-m9-swarmtank-slice3-windows-verified-2026-11-01.zip`) are preserved untouched on `E:\Anansi\deliveries\` as historical evidence.
- **Coordination Baseline**: The fresh coordination package `E:\Anansi\deliveries\anansi-m9-coordination-baseline-2026-11-01\` correctly references tip `68ee353291d2a5732ccdeaa340807590e502b572` in both `MANIFEST.json` and `HANDOFF.md`.

---

## 6. Structured Evidence Taxonomy

To prevent confusion between what has been executed and what remains deferred, the current status of all M8 and M9 capabilities is classified below:

| Requirement / Sub-Feature | Classification | Status | Evidence Reference | Notes |
| :--- | :--- | :--- | :--- | :--- |
| **M8 Plain Text & MD Ingestion** | Fresh / Historical | Verified | `evidence/M8-document-input-verified.md` | Strict UTF-8, backend hash provenance. |
| **M8 Saved-Project Context** | Fresh / Historical | Verified | `handoffs/M8-slice4-project-context-addendum.md` | Pinned revisions, fail-closed restore. |
| **M8 URL Beam Input** | Historical | Verified | `reviews/windows-m8-url-input-verified-2026-10-31.md` | 21 cases, 4 browser workflows pass. |
| **M8 Lineage R1–R4 Controls** | Historical | Verified | `reviews/windows-m8-lineage.md` | Forgery & multi-generation chains pass. |
| **M8 3D Lexi-Collider Arena** | Deferred | Planned | `docs/REQUIREMENTS_MATRIX.md` | 2D table accessible fallback active. |
| **M8 Live External Scraping** | Deferred | Blocked | Zero-Cost Lock Policy | Offline deterministic fixtures only. |
| **M9 Synthetic Personas** | Fresh | Verified | `reviews/windows-m9-swarmtank-slice3-verified.md` | 20/50/100 personas, deterministic PRNG. |
| **M9 Decision Model** | Fresh | Verified | `reviews/windows-m9-swarmtank-slice3-verified.md` | 7-factor engine (Convert/Confused/Reject). |
| **M9 Three.js Arena Visualization** | Fresh | Verified | `reviews/windows-m9-swarmtank-slice3-verified.md` | Amphitheater rings, raycast selection, 60fps. |
| **M9 Dynamic Sensitivity Engine** | Fresh | Verified | `reviews/windows-m9-swarmtank-slice3-verified.md` | Sub-5ms price, feature & trust sensitivity. |
| **M9 Animated Arena Re-clustering** | Fresh | Verified | `reviews/windows-m9-swarmtank-slice3-verified.md` | 600ms radial transitions, reduced-motion snap. |
| **M9 Scenario Comparison Studio** | Fresh | Verified | `reviews/windows-m9-swarmtank-slice3-verified.md` | Transition matrix, price elasticity metrics. |
| **M9 Product Forge Bridge v3** | Fresh | Verified | `reviews/windows-m9-swarmtank-slice3-verified.md` | Scenario branching, diff addenda to v3. |
| **M9 Headline / Value Prop Inputs** | Deferred | Untested | This reconciliation report | Omitted from UI and schema. |
| **M9 Target Audience Controls** | Deferred | Untested | This reconciliation report | Omitted from UI and schema. |
| **M9 Provider AI Reassessment** | Deferred | Misleading | This reconciliation report | Button invokes deterministic evaluator. |
| **M9 Per-Agent 3D Floating Labels** | Deferred | Planned | `handoffs/M9-swarmtank-slice2-addendum.md` | Performance preservation on low-power GPUs. |
| **M7 Unsigned Release Tooling** | Fresh | In Progress | `docs/reviews/windows-release/` | Tooling rebased, current build qualification underway. |

---

## 7. Conclusion & Gate Directives

1. **AN-014 Milestone Integrity**: Milestone 9 is certified for its synthetic and 3D visual capabilities (Slices 1–3). However, claims of "Milestone 9 100% complete" must be qualified by recording the deferred text inputs and provider-backed reassessment.
2. **Zero Fabrication**: Live-model reassessment claims must be eliminated from UI labels before user-facing promotion.
3. **Execution Guardrails**: The application baseline `68ee353291d2a5732ccdeaa340807590e502b572` remains strictly read-only while Muse works on SOMNIA Slice 1. All release qualification tasks are confined to `scripts/windows-release/`, `tests/windows-release/`, and `docs/reviews/windows-release/`.
