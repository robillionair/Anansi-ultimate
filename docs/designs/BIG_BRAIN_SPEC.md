# Big Brain (AN-011) — Architectural Specification and Behavior Contract

## 1. Context and Scope

Big Brain is ANANSI's deliberative multi-agent reasoning and orchestration engine. Named in Master Specification §8 as a primary interface mode and tracked under Requirement **AN-011**, this document resolves the M4 design gate by defining its formal behavior contract, state machine, safety boundaries, and integration interfaces with the Memory OS and Context Compiler.

Authority: Master Specification §8, §11, §19, §71, §77–78; Requirements Matrix AN-006, AN-010, AN-011; ADR-0030.

---

## 2. Core Reasoning Cycle

Big Brain operates as a bounded deterministic state machine that breaks down complex user objectives into deliberative reasoning cycles:

```
[ User Goal ]
      │
      ▼
┌──────────────┐
│  1. PLAN     │◄────────────────────────────────┐
└──────┬───────┘                                 │
       │ (Sub-goals & Queries)                   │
       ▼                                         │
┌──────────────┐                                 │
│ 2. RETRIEVE  │ (Section-Aware Context Compiler)│
└──────┬───────┘                                 │
       │ (Grounded Evidence Pack)                │
       ▼                                         │
┌──────────────┐                                 │ (Iterate if
│ 3. CRITIQUE  │ (Detect Missing Evidence/Facts) │  Incomplete,
└──────┬───────┘                                 │  Max 10 steps)
       │ (Sufficient or Actionable)              │
       ▼                                         │
┌──────────────┐                                 │
│  4. EXECUTE  │ (Loopback Local Inference)      │
└──────┬───────┘                                 │
       │                                         │
       ▼                                         │
┌──────────────┐                                 │
│  5. REFLECT  │─────────────────────────────────┘
└──────┬───────┘
       │ (Task Completed)
       ▼
[ Grounded Synthesis & Disclosure Manifest ]
```

### Step Descriptions:
1. **Plan**: Formulates an actionable plan of 1–5 discrete sub-tasks and specific workspace search queries.
2. **Retrieve**: Calls the Section-Aware Context Compiler (`crates/core/src/context.rs`) and Hybrid Retrieval (`crates/core/src/hybrid.rs`), compiling grounded evidence packs with exact section headings and line numbers.
3. **Critique**: Analyzes candidate evidence. Checks for contradictions, temporal recency conflicts, or missing evidence. Refuses hallucination if evidence is absent.
4. **Execute**: Synthesizes intermediate steps using local loopback LLMs (Ollama or LM Studio).
5. **Reflect**: Evaluates whether the user's objective is satisfied or if another bounded reasoning cycle is required.

---

## 3. Safety, Privacy, and Execution Boundaries

1. **Bounded Execution Limit**:
   - Maximum 10 deliberative iterations per session.
   - Cycle loop detection: if the same sub-task query is repeated twice without new evidence, the engine halts immediately and outputs `AskStatus::InsufficientEvidence`.

2. **Zero-Cost Lock & Offline Preference**:
   - Master Spec §67, §78:
   - Inference defaults strictly to local loopback endpoints (`127.0.0.1:11434`, `127.0.0.1:1234`).
   - Any external cloud inference requires explicit per-request authorization. Zero telemetry, zero background downloads.

3. **Fail-Closed Sealed Memory Boundary**:
   - Master Spec §74:
   - Sealed memories are strictly forbidden from retrieval, evidence compilation, or prompt generation.
   - Any attempt to access a sealed record within Big Brain results in immediate fail-closed omission.

4. **Transparent Data Disclosure ("View What ANANSI Sent")**:
   - Every deliberative step records an inspectable `DisclosureManifest` logging:
     - Runtime kind (`ollama`, `lmStudio`, `customLoopback`).
     - Endpoint and model tag.
     - Referenced memory IDs and titles.
     - Exact character count and estimated token usage.
     - Routing and step explanation.

---

## 4. Integration Contracts

- **Core Module**: Orchestrated under `crates/core/src/big_brain.rs` (M5 implementation).
- **Context Engine**: Invokes `crates/core/src/context.rs` (`compile_context`).
- **Retrieval Engine**: Invokes `crates/core/src/hybrid.rs` (`hybrid_search`).
- **Shared UI**: Rendered in navigation under `INNOVATION -> Big Brain`.
