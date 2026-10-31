# M9 — SwarmTank 3D

Status: **Slice 1 in implementation** by Muse under reviewer assignment; Slice 2 (3D Arena) and Slice 3 (Interactive Scenario Controls) tracked for sequential delivery.

Authority: [master specification](../docs/MASTER_SPEC.md), sections 34–41, 65, 67, 73, 77–78; [canonical roadmap](../docs/ROADMAP.md).
Requirement: `AN-014` (SwarmTank 3D).

Subtitle: **Synthetic Buyer and Market Scenario Arena**

Purpose: Help founders explore how hypothetical customer profiles might respond to a product concept. This is a scenario-planning tool, not a substitute for real customer interviews, empirical conversion data, A/B tests, or statistically valid market research. All synthetic agents must be clearly and prominently labeled as simulated.

---

## Milestone Execution Slices

### Slice 1: Synthetic Personas & Probabilistic Decision Engine (§34–37, §40–41)
- **Input Ingestion (§35)**: Accepts product description, pricing, customer segment, industry, features, value proposition, competitor positioning, or direct import from a saved §27 Collider experiment / Product Forge concept. The live panel must not trigger speculative LLM inference on mount.
- **Configurable Persona Population (§36)**: Presets for 20, 50, and 100 synthetic buyer personas. Structured persona attributes:
  - Role and title (e.g., enterprise finance director, technical founder, cost-sensitive freelancer)
  - Company size and structure
  - Approximate budget and purchasing authority
  - Needs, priorities, and pain points
  - Skepticism index (0.0 to 1.0) and price sensitivity (0.0 to 1.0)
  - Purchase constraints and required trust signals
  - Expected implementation effort
  - Structured batch generation to bound cost and latency; no deceptive claims of 100 independently reasoned instances if one model created 100 persona records.
- **Decision Model (§37)**: Evaluates personas against product inputs into three explicit, verifiable states:
  - `Convert`: High perceived value exceeding price and low blocking friction.
  - `Confused`: Unclear value proposition, missing trust signals, or implementation uncertainty.
  - `Reject`: Budget mismatch, unacceptable trade-offs, or critical missing capabilities.
  - Captures exact reasoning, key objection, perceived value, and willingness to pay per persona.
- **SwarmTank Report (§40)**:
  - High-level metrics: Conversion rate percentage, average willingness to pay, and price elasticity estimates.
  - Top 3 confusion points, top 3 objections, and recommended messaging/feature adjustments.
  - Accessible tabular breakdown with search, filter by decision, and detailed objection inspection.
- **Bridge to Product Forge (§41)**:
  - "Apply Recommendations to Forge" action: copies top feedback and objections into an actionable revision note or brief addendum for the associated Forge concept.
- **Persistence & Integrity**: Immutable versioned run records, fail-closed restore validation, and zero unprompted inference on load.

### Slice 2: SwarmTank 3D Arena Visualization & Accessible Alternative (§38, §65)
- **Three.js 3D Arena (§38)**:
  - Pedestal/stage at center representing the product concept.
  - Personas rendered as instanced dynamic agents in a circular amphitheater or market floor.
  - Decision clustering: `Convert` (green) move closer to the pedestal; `Confused` (yellow/amber) orbit the perimeter with hesitation markers; `Reject` (red) turn away and retreat toward the outer boundary.
  - Raycast picking on 3D agents maps to persona details and objection inspection popover.
- **Performance & Accessibility (§65)**:
  - Device pixel ratio clamped to 2, low-power WebGL preference, full geometry/material disposal on unmount and version switch.
  - Static view under `prefers-reduced-motion` or workspace reduced-motion setting.
  - Complete accessible 2D list/table alternative always rendered and keyboard navigable.

### Slice 3: Interactive Scenario Controls & Dynamic Sensitivity (§39)
- **Live Scenario Adjustments**: Interactive sliders for price points, feature toggles, and messaging emphasis.
- **Dynamic Re-evaluation**: Local rule-based and model-assisted decision re-evaluation showing delta conversion rates and shifted persona clusters.
- **Comparison Mode**: Side-by-side run comparison showing how price changes or feature additions impacted objections.

---

## Security, Privacy & Integrity Boundaries

1. **Zero-Cost Lock & Privacy**:
   - Zero-Cost Lock enforced: Unconfigured or paid providers are rejected before traffic.
   - Persona generation treats user input as private data, never as public training data.
   - Sealed and private workspace memories are protected; only explicitly authorized context is dispatched.
2. **Deterministic Derivation & Storage**:
   - SwarmTank runs are saved as immutable records with SHA-256 digests in SQLite catalog.
   - Restored runs re-verify content hashes fail-closed; restore remapping preserves cross-entity relationships without duplicate generation.
3. **Honesty & Transparency**:
   - Clear disclosure that persona responses are model-generated simulations, not real consumer data.
   - Generation parameters, token usage, and provider attempts are recorded in the provenance disclosure.

---

## Slice 1 checkpoint — 2026-10-31

Slice 1 (synthetic personas & decision engine) implemented on branch `muse/m9-swarmtank-slice1` from Windows-verified baseline `7c576b85`; design before code in `docs/handoffs/M9-swarmtank-slice1-design.md`, addendum in `docs/handoffs/M9-swarmtank-slice1-addendum.md`. Linux verification: Rust 562 passed + 4 known live-Ollama environment failures, node 131/131, browser battery 15/15. Independent Windows rerun pending. Deferred, not claimed: §38 3D arena, scenario engine, provider-backed generation, URL import, spec auto-start, native save dialog.
