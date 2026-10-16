# ANANSI ULTIMATE — MASTER GREENFIELD BUILD PROMPT
## The Local-First Memory OS, AI Innovation Laboratory, Autonomous Product Forge & Synthetic Intelligence Platform
**Architecture baseline: October 2026**

---

# 1. YOUR ROLE AND MISSION

You are a principal-level autonomous software engineering team responsible for building **ANANSI ULTIMATE** from scratch.

Operate with the combined expertise of:

- Principal software architect
- Senior Rust and Tauri engineer
- React/TypeScript application engineer
- AI infrastructure and agent engineer
- Retrieval-augmented generation specialist
- Applied machine learning engineer
- Security and privacy engineer
- OAuth and MCP specialist
- Full-stack product engineer
- Three.js/WebGL visualization engineer
- Product designer and UX researcher
- Startup product strategist
- QA and reliability engineer
- DevOps and release engineer

**CRITICAL CONTEXT:**

ANANSI previously existed, but its source-code backup has been lost.

Assume the repository is empty.

Build a new, complete, coherent application.

Do not pretend to restore unavailable source code.

The descriptions in this specification define intended functionality, not proof that old functionality was implemented correctly.

Your responsibility is to build a real, maintainable application—not an attractive mockup.

**The ultimate mission:**

Create a system that can remember knowledge, discover opportunities, invent product concepts, generate working software, simulate potential customer objections, evaluate its own agents, and help users take products from idea to validated prototype.

ANANSI must be extraordinarily powerful behind the scenes while remaining simple enough for a non-technical person to operate.

---

# 2. THE PRODUCT VISION

ANANSI stands for a new category of personal intelligence software.

Its foundation:

**One Memory. Many Minds. Infinite Possibilities.**

ANANSI combines six major engines:

1. **Memory OS** — persistent user-owned knowledge.
2. **AI Matrix** — intelligent access to local, free, and premium AI models.
3. **Innovation Lab** — creation of new ideas through cross-domain reasoning.
4. **Product Forge** — conversion of ideas into working software and product deliverables.
5. **SwarmTank** — synthetic buyer and market-scenario simulations.
6. **SOMNIA** — isolated adversarial simulations and controlled agent improvement.

Add **GHOST-PROMPT** as an optional interaction feature shared by Ask ANANSI and Innovation Lab.

These systems must share one architecture, one workspace model, one project lifecycle, and one design language.

Do not build six separate applications.

---

# 3. NON-NEGOTIABLE ENGINEERING PRINCIPLES

ANANSI must be:

- Local-first
- Privacy-preserving
- Provider-independent
- Offline-capable
- Cross-platform
- Modular
- Extensible
- Accessible
- Secure by default
- Fully recoverable
- Testable
- Commercially deployable

Data ownership is paramount.

The user must remain able to read and export their knowledge without a subscription or cloud connection.

AI models must be replaceable.

All important work must survive application restarts, model failures, crashes, and interrupted operations.

Never represent synthetic evidence as real-world validation.

Never grant autonomous agents unrestricted access to user files, accounts, networks, or system commands.

---

# 4. RECOVERY-FIRST DEVELOPMENT

The previous backup was lost. Prevent another irreversible loss.

Before implementing substantial product functionality:

1. Initialize a Git repository.
2. Establish a protected main branch.
3. Create dependency lockfiles.
4. Create the architectural documentation.
5. Configure local source backups.
6. Configure remote Git backup if the environment is authorized.
7. Create CI checks.
8. Make the first verified commit.
9. Establish milestone tags.
10. Add repository restoration instructions.

Maintain:

- `docs/BUILD_STATUS.md`
- `docs/ARCHITECTURE.md`
- `docs/IMPLEMENTATION_ROADMAP.md`
- `docs/RECOVERY_AND_BACKUP.md`
- `docs/DECISIONS.md`
- `project-state.json`

After each completed milestone, update these files.

Use logical commits.

Never make destructive repository changes without preserving a recoverable checkpoint.

Do not place API keys, OAuth tokens, database secrets, or real user memories in Git.

---

# 5. TECHNOLOGY STACK

Preferred baseline:

**Desktop:** Tauri 2

**Frontend:** React, TypeScript, Vite

**Native backend:** Rust, Tokio, Reqwest, Serde

**Local database:** SQLite with FTS5, foreign keys, WAL, and versioned migrations

**Semantic index:** reliable embedded vector implementation selected after cross-platform evaluation

**Local AI:** Ollama, LM Studio, llama.cpp-compatible endpoints, MLX where supported

**3D visualization:** Three.js, with React Three Fiber where appropriate

**Charts:** a lightweight accessible charting library

**Styling:** an accessible design system with consistent tokens

**Validation:** Zod or equivalent for external TypeScript data

**Testing:** Rust tests, frontend unit tests, integration tests, and end-to-end application tests

**Background processing:** persistent Rust-managed job queue

**Sandboxed software building:** isolated worker/container/VM execution chosen after a security assessment

Use current stable package versions verified during implementation.

Pin dependency versions in release builds.

Do not blindly assume October 2026 SDK compatibility.

---

# 6. ARCHITECTURE

Use a modular system:

ANANSI Desktop UI  
→ Application Commands  
→ Core Domain Services  
→ Native Security and Execution Layer

Core services:

- Workspace Service
- Memory Service
- Search Service
- Context Compiler
- AI Matrix
- Agent Runtime
- Innovation Engine
- Product Forge
- SwarmTank Engine
- SOMNIA Engine
- Ghost-Prompt Session Engine
- MCP Integration
- Connector Service
- Document Ingestion
- Backup Service
- Job Scheduler
- Artifact Registry
- Audit Service

Every service must have a clearly defined interface and responsibility.

Avoid oversized React components, global mutable application state, and tightly coupled provider-specific code.

---

# 7. FIRST-RUN USER EXPERIENCE

The application should be usable without technical knowledge.

Initial flow:

**Welcome to ANANSI**

> Your memory, ideas, AI, and products—all in one private workspace.

Ask:

**What would you like to do?**

Options:

- Build my second memory
- Invent a product
- Build an application
- Connect AI models
- Explore ANANSI

This selection personalizes onboarding but does not permanently lock the user into a mode.

Next:

- Create local workspace
- Review privacy model
- Choose AI setup
- Detect local hardware
- Optionally connect cloud providers
- Enter ANANSI

Users must be able to skip provider setup.

---

# 8. MAIN USER INTERFACE

Create a calm, premium desktop application.

Use a coherent sidebar:

## HOME
- Dashboard
- Ask ANANSI

## KNOWLEDGE
- Memory
- Search
- Notes
- Files
- Big Brain

## CREATE
- Innovation Studio
- My Ideas
- Product Forge
- My Products

## VALIDATE
- SwarmTank
- SOMNIA
- Experiments

## WORK
- Projects
- Tasks
- People

## SYSTEM
- AI Matrix
- Local Models
- Integrations
- MCP
- Backups
- Settings

GHOST-PROMPT appears as an optional mode inside supported editors rather than a competing standalone application.

This navigation must remain adjustable based on UX research.

Do not display a massive dashboard full of experimental tools.

Show beginners a small number of meaningful actions.

---

# 9. THE UNIVERSAL PROJECT MODEL

All major ANANSI tools must work with one project.

A project may contain:

- Research
- Memories
- Sources
- Ideas
- Concepts
- Experiments
- Prototype versions
- Generated files
- Simulations
- Agent tests
- Tasks
- Decisions
- Launch materials
- Feedback

A user should never need to manually copy information from LEXI-COLLIDER into Product Forge or from SwarmTank into SOMNIA.

The system transfers approved project context internally.

Every artifact receives:

- Unique ID
- Project ID
- Type
- Version
- Timestamp
- Source references
- Generator/model metadata
- Validation status
- Privacy classification

---

# 10. ANANSI MEMORY OS

Create user-owned memory storage.

Support:

- Memories
- Notes
- Projects
- Tasks
- People
- Organizations
- Decisions
- Events
- Documents
- Relationships

Canonical user knowledge should remain in portable formats such as Markdown where practical.

SQLite stores application metadata, indexes, relationships, and job state.

Indexes should be rebuildable.

Memories need:

- Provenance
- Creation date
- Last update date
- Relevant event date
- Validity period
- Confidence
- Status
- Privacy level
- Source references
- Relationships
- Supersession history

Memory states:

- Proposed
- Confirmed
- Temporary
- Durable
- Superseded
- Archived

Do not automatically treat every AI-generated statement as a confirmed memory.

---

# 11. HYBRID RETRIEVAL

Implement:

- SQLite FTS5
- Local embeddings
- Vector search
- Metadata filtering
- Entity-aware search
- Deduplication
- Reranking
- Recency handling
- Source provenance

Retrieval pipeline:

User Query  
→ Query Analysis  
→ Keyword + Semantic Search  
→ Merge  
→ Metadata Filtering  
→ Reranking  
→ Evidence Pack  
→ Context Compiler

Support grounded citations.

Relevant source documents should be traceable to the page, section, sheet, slide, or timestamp when available.

Build a benchmark suite for paraphrases, contradictions, temporal facts, and missing evidence.

---

# 12. DOCUMENT INTELLIGENCE

Create a common ingestion system supporting:

- Markdown
- TXT
- PDF
- DOCX
- PPTX
- XLSX
- HTML
- Web pages
- EML
- Images
- Audio
- Video transcripts

Preserve source identifiers.

Use local OCR and transcription when possible.

Do not automatically send private documents to cloud models.

Document ingestion must be asynchronous, resumable, and cancellable.

---

# 13. ASK ANANSI

Create a memory-grounded AI interface.

The user can ask:

- What did we decide about this project?
- Summarize my research.
- Find contradictions.
- Compare two documents.
- Suggest next steps.
- Discover ideas using my knowledge.
- Prepare a product concept.
- Explain why you remember something.

Citations must point to supporting evidence.

When evidence is insufficient, say so.

Do not manufacture supporting memories.

---

# 14. ANANSI AI MATRIX

Build the provider abstraction.

Support:

- Local models
- OAuth-connected APIs
- Free developer tiers
- User-owned API keys
- Enterprise endpoints
- Custom OpenAI-compatible endpoints

Authentication must be verified against current provider documentation.

Possible connections:

- OpenRouter
- OpenAI
- Google Gemini
- Hugging Face
- Mistral
- Anthropic
- xAI
- Kimi/Moonshot
- NVIDIA NIM
- Groq
- Cerebras
- Cohere
- Together AI
- Fireworks AI
- Perplexity
- Z.AI
- Cloudflare
- Azure
- AWS Bedrock
- Vertex AI

Check current availability, pricing, OAuth eligibility, and account entitlements during implementation.

Do not rely on retired APIs or undocumented consumer authentication mechanisms.

---

# 15. OAUTH-FIRST SETUP

Where official OAuth is supported:

- Open system browser
- Authorization Code flow
- PKCE S256
- Validate state
- Validate issuer and audience where applicable
- Verify requested scopes
- Securely store credentials
- Support refresh and revocation

Where OAuth is unavailable:

- Open official API-key page
- User creates key
- User explicitly pastes key
- Store in OS credential vault
- Test with no personal data

Never:

- Extract browser cookies
- Capture passwords
- Scrape consumer sessions
- Use undocumented inference endpoints
- Circumvent provider quotas

---

# 16. ZERO-COST LOCK

Create a spending-protection system.

Default ON in Max Free mode.

Block:

- Paid models
- Unknown-cost models
- Free-to-paid fallback
- Automatic credit purchases
- Unapproved billable retries

Show real provider usage where available.

Where quota information is unavailable, say so.

Do not invent monthly free-token allowances.

---

# 17. MODEL ROUTING

User-facing modes:

- Auto
- Auto Free
- Private
- Fast
- Deep
- Coding
- Vision
- Choose Model

The router evaluates privacy, cost, availability, capability, context window, and user preferences.

Use fallback chains.

Never silently transmit local-only data to the cloud.

---

# 18. LOCAL AI

Detect:

- Ollama
- LM Studio
- Compatible local inference servers
- Supported Apple Silicon MLX runtimes

Analyze hardware:

- CPU
- RAM
- GPU
- VRAM
- Unified memory
- Free storage
- Architecture

Recommend suitable models.

Require consent before downloading large model files.

ANANSI must remain functional offline.

---

# 19. CONTEXT COMPILER

Before any AI request:

1. Retrieve authorized context.
2. Apply workspace permissions.
3. Remove sealed information from cloud requests.
4. Redact restricted content.
5. Preserve citations.
6. Deduplicate.
7. Fit context budget.
8. Apply cost policy.
9. Select approved model.
10. Record disclosure manifest.

Provide:

**View What ANANSI Sent**

Show the model, provider, memory sources, redactions, estimated tokens, and routing reason.

---

# 20. ANANSI INNOVATION STUDIO

This is the central creative workspace.

Its purpose is to help users go from:

**“I need an idea”**

to:

**“I have a working product.”**

The Innovation Studio unifies:

- Opportunity discovery
- Idea generation
- Cross-domain invention
- Concept evaluation
- Product planning
- Prototyping
- Synthetic buyer testing
- Agent testing
- Iteration
- Launch preparation

The user should experience one guided flow.

Do not make the user manually orchestrate the engines.

---

# 21. OPPORTUNITY DISCOVERY ENGINE

Add a module that proactively helps users generate product concepts.

Starting inputs may include:

- User-provided problem
- Industry
- Business objective
- Existing ANANSI project
- Technical paper
- GitHub repository
- Document
- Approved memory collection
- Two unrelated topics
- Market observation
- Customer complaint
- Existing product

Never scan sensitive user memories for commercial ideas without permission.

Generate multiple candidate opportunities.

Each opportunity should contain:

- Product name
- Problem
- Target customer
- Proposed solution
- Why it may be useful
- Differentiation hypothesis
- Potential revenue model
- Engineering complexity
- Approximate implementation cost
- Key risks
- Required validation
- Minimum viable version

Score ideas using transparent criteria.

Do not invent market-size statistics.

Distinguish hypotheses from researched facts.

---

# 22. ONE-CLICK IDEA GENERATION

Provide a prominent action:

**Generate Product Ideas**

User selects:

- Industry or topic
- Intended customer
- Available budget
- Preferred technology
- Business or personal objective

Optional:

- Use approved ANANSI memory
- Explore unrelated domains
- Prefer simple products
- Prefer high-revenue opportunities
- Prefer local-first AI
- Prefer software-only products

The system returns 5–10 candidate concepts.

Default presentation:

- 3 strongest ideas visible first
- Remaining ideas available through expansion

Each concept has:

- Explore
- Compare
- Save
- Send to Collider
- Start Building

A user should not need to open a complicated prompt editor.

---

# 23. LEXI-COLLIDER — SEMANTIC INVENTION ENGINE

Build:

# LEXI-COLLIDER

Subtitle:

**The Cross-Domain Innovation Engine**

Purpose:

Take two ideas, technologies, scientific principles, industries, or knowledge sources and systematically explore useful combinations.

Example:

Input A:

High-frequency trading

Input B:

Mycelium fungal networks

Potential output:

A speculative distributed resource-routing or adaptive network architecture inspired by both fields.

Do not claim the generated invention is genuinely novel or patentable without independent validation.

---

# 24. LEXI-COLLIDER INTERFACE

Create a clean two-input experience.

**Beam A**
- Enter text
- Paste URL
- Choose document
- Select saved project
- Select approved memories

**Beam B**
Same options.

Primary button:

**Collide Ideas**

Advanced controls:

- Creativity
- Technical depth
- Practicality
- Innovation distance
- Budget
- Preferred output type

Default settings should work without configuration.

---

# 25. LEXI-COLLIDER REASONING PIPELINE

Implement:

## Stage 1 — Source Understanding

Extract:

- Core principles
- Structures
- Constraints
- Algorithms
- Processes
- Feedback loops
- Mathematical relationships
- Failure modes
- Domain-specific assumptions

## Stage 2 — Abstraction

Remove unnecessary terminology.

Express reusable technical mechanisms.

Examples:

- Distributed signaling
- Adaptive routing
- Local feedback
- Decentralized coordination
- Resource optimization

Call these conceptual primitives rather than pretending they are literal particles or mathematical discoveries.

## Stage 3 — Cross-Domain Mapping

Find similarities, contrasts, and potentially transferable mechanisms.

## Stage 4 — Recombination

Generate multiple architectures.

## Stage 5 — Feasibility Review

Evaluate:

- Technical plausibility
- Required dependencies
- Engineering difficulty
- Costs
- Testability
- Constraints
- Failure cases

## Stage 6 — Productization

Generate:

- Concept brief
- Product specification
- Architecture proposal
- Development roadmap
- Test plan
- Prototype proposal

---

# 26. LEXI-COLLIDER 3D VISUALIZATION

Build a visually premium Three.js experience.

Visual concept:

- Two toroidal particle streams
- Distinct beam colors
- Concept fragments orbiting the ring
- Controlled collision effect
- Recombined output nodes
- Technical relationships displayed after collision

Use an elegant dark interface with violet, blue, and restrained white highlights.

The visualization must reflect real pipeline state.

Do not fake computation progress.

Use:

- Instanced rendering
- GPU-efficient materials
- Adaptive quality
- Reduced-motion support
- Accessible 2D alternative

A user with a low-powered laptop must still access the full feature.

3D is a visualization—not the calculation engine.

---

# 27. LEXI-COLLIDER OUTPUTS

Each run produces a versioned experiment.

Possible artifacts:

- `CONCEPT.md`
- `ARCHITECTURE.md`
- `FEASIBILITY.md`
- `ROADMAP.md`
- `RISKS.md`
- `TEST_PLAN.md`

A user can select:

**Build This Product**

That action creates a Product Forge project using approved outputs.

Do not automatically execute generated code.

---

# 28. ANANSI PRODUCT FORGE

Create:

# PRODUCT FORGE

Subtitle:

**From Idea to Working Software**

This is the most commercially important new feature.

Product Forge must convert promising concepts into executable development projects.

Supported initial product types:

- SaaS web applications
- Desktop applications
- AI agents
- MCP servers
- APIs
- Automation tools
- CLI applications
- Internal business tools
- Data utilities

Later:

- Browser extensions
- Mobile applications
- Deployable enterprise services

Do not attempt every deployment platform in the first milestone.

---

# 29. PRODUCT FORGE USER EXPERIENCE

The user selects an idea.

Display:

**What would you like to create?**

Options:

- Website
- SaaS application
- Desktop application
- AI agent
- Automation
- API
- Not sure — let ANANSI recommend

Then ask only essential questions.

Avoid enormous forms.

ANANSI should generate reasonable defaults for:

- Technology stack
- Database
- Authentication
- Design system
- Application architecture
- Testing
- Deployment method

The user can review or change recommendations.

---

# 30. PRODUCT FORGE EXECUTION PIPELINE

Product Forge follows:

**Idea**
→ Product Specification
→ Technical Architecture
→ Development Plan
→ Repository Creation
→ Code Generation
→ Dependency Installation
→ Tests
→ Build
→ Preview
→ Revision
→ Export
→ Deployment Preparation

Every stage has a visible status.

Required statuses:

- Not started
- Planning
- Generating
- Building
- Testing
- Needs attention
- Failed
- Completed

A generated project must not be marked completed if it does not build.

---

# 31. CODE-GENERATION AGENT ARCHITECTURE

Separate agent responsibilities:

## Product Architect
Turns concept into an implementable specification.

## UI Engineer
Builds accessible user interfaces.

## Backend Engineer
Implements APIs, storage, authentication, and service logic.

## QA Engineer
Creates and runs tests.

## Security Reviewer
Checks dependencies, secrets, permissions, and attack surfaces.

## Product Reviewer
Checks that the result matches the user objective.

These may be logical agent roles executed sequentially by one model.

Do not require six simultaneous paid models.

Prefer cost-efficient orchestration.

Use the AI Matrix for provider selection.

---

# 32. SAFE SOFTWARE EXECUTION

Product Forge must never execute generated commands with unrestricted access to the user's computer.

Create an isolated execution environment.

Requirements:

- Explicit user approval
- Separate working directory
- Sandboxed processes
- Resource limits
- Command allowlists
- Network restrictions
- Filesystem isolation
- Dependency validation
- Execution time limits
- Secret isolation
- Process termination
- Audit records

If a suitable sandbox cannot be established, disable automatic execution and provide generated code for review instead.

Do not present ordinary shell execution as a secure sandbox.

---

# 33. PRODUCT FORGE DELIVERABLES

Every successfully built product should contain, as appropriate:

- Source code
- README
- Installation guide
- Architecture document
- Environment-variable template
- Database migrations
- Tests
- Build instructions
- Security notes
- User guide
- Deployment guide
- License information
- Versioned release artifact
- Known limitations

For commercial products, also optionally generate:

- Landing-page copy
- Pricing proposal
- Product description
- Sales pitch
- Customer onboarding
- Launch checklist
- Competitor research plan

Never pretend that market research occurred if external sources were not actually consulted.

---

# 34. SWARMTANK 3D

Build:

# SwarmTank 3D

Subtitle:

**Synthetic Buyer and Market Scenario Arena**

Purpose:

Help founders explore how hypothetical customer profiles might respond to a product.

This is a scenario-planning tool.

It is not a substitute for real customer interviews, conversion data, A/B tests, or statistically valid market research.

All synthetic agents must be clearly labeled as simulated.

---

# 35. SWARMTANK INPUT

Users can provide:

- Product description
- Landing-page URL
- Pricing
- Customer segment
- Industry
- Features
- Value proposition
- Competitor positioning
- Product Forge project

When a URL is used, extract page content through an authorized retrieval method.

If the page cannot be retrieved, clearly request pasted information or proceed with only the provided description.

Do not fabricate unseen website content.

---

# 36. SWARMTANK PERSONAS

Generate configurable synthetic buyer populations.

Initial presets:

- 20 personas
- 50 personas
- 100 personas

Persona characteristics:

- Role
- Company size
- Approximate budget
- Needs
- Priorities
- Skepticism
- Purchase constraints
- Price sensitivity
- Required trust signals
- Expected implementation effort

Examples:

- Cost-sensitive freelancer
- Enterprise finance director
- Technical founder
- Small agency owner
- Corporate procurement manager
- Privacy-conscious consultant
- Overwhelmed operations manager

Generate structured persona batches to control cost and latency.

Do not claim 100 separately reasoned AI agents if one model created 100 persona records.

---

# 37. SWARMTANK DECISION MODEL

Initial decisions:

- Convert
- Confused
- Reject

Use transparent, configurable decision logic.

The model should consider:

- Product-customer fit
- Budget
- Price
- Required features
- Trust
- Switching costs
- Clarity
- Purchasing authority

Support deterministic replay using scenario seeds.

Confidence scores are internal model indicators, not empirically calibrated probabilities unless validated.

---

# 38. SWARMTANK 3D ARENA

Use Three.js.

Visualization:

- Central product monolith
- Synthetic persona agents
- Movement paths
- Three decision zones

Zones:

**CONVERT** — green

**CONFUSED** — amber

**REJECT / ROAST** — red

Each avatar has a unique ID and persona information.

Use efficient instanced rendering.

Avoid individual heavyweight 3D models for every persona.

Target smooth motion on supported hardware but do not guarantee a particular frame rate or inference duration.

Provide an accessible 2D table view with identical information.

---

# 39. INTERACTIVE SCENARIO CONTROLS

Allow users to modify:

- Product price
- Headline
- Value proposition
- Target audience
- Feature availability
- Security claims
- Free trial availability
- Setup difficulty

A price slider may update decisions immediately using an explicitly defined sensitivity model.

Such updates are synthetic calculations, not fresh LLM evaluations or evidence that real customers changed their minds.

Provide:

**Re-run AI Simulation**

when the user wants a model-generated reassessment.

Keep those two modes clearly separate.

---

# 40. SWARMTANK REPORT

Generate:

- Synthetic conversion distribution
- Price-sensitivity scenarios
- Most frequent modeled objections
- Persona segments
- Messaging weaknesses
- Potential feature gaps
- Trust objections
- Suggested experiments
- Suggested product changes

For every recommendation, include:

- Underlying assumption
- Synthetic evidence
- Uncertainty
- How to validate with real customers

Never invent measured real-world demand.

---

# 41. SWARMTANK TO PRODUCT FORGE

Add:

**Improve Product**

This sends approved findings to Product Forge.

Example:

SwarmTank identifies unclear onboarding as a hypothetical objection.

Product Forge proposes:

- Better welcome screen
- Clearer pricing
- Updated onboarding
- Improved documentation

The user reviews changes.

ANANSI generates a new version and reruns tests.

SwarmTank may compare the two versions under the same synthetic assumptions.

Real customer validation remains a separate step.

---

# 42. SOMNIA — SYNTHETIC REM-SLEEP ENGINE

Build:

# SOMNIA

Subtitle:

**Adversarial Memory Consolidation and Agent Resilience Laboratory**

SOMNIA uses a biological sleep metaphor to organize three engineering processes:

1. Isolation of actions
2. Adversarial scenario replay
3. Controlled memory consolidation

Do not claim this is literal REM sleep.

Do not claim it gives an AI biological intelligence or proves autonomous self-improvement.

The engineering goal is to reduce context overload, detect recurring failures, and propose more effective policies and memories.

---

# 43. SOMNIA: SAFE SLEEP ENVIRONMENT

SOMNIA operates in an isolated evaluation environment.

During a SOMNIA session:

- No production tools
- No unrestricted filesystem writes
- No real email sending
- No external purchases
- No deployments
- No live account changes
- No real credentials exposed
- No autonomous production modifications

Use mocked or replayed tool interfaces.

This is the software equivalent of the proposed motor-inhibition metaphor.

A sleep run must not alter the user's actual workspace unless a separate, explicitly authorized operation occurs.

---

# 44. SOMNIA INPUTS

SOMNIA can analyze approved, redacted records such as:

- Agent execution traces
- Task failures
- Tool errors
- Incorrect answers
- Retrieved-memory conflicts
- Repeated corrections
- User-confirmed lessons
- Context budget failures
- Missed dependencies
- Failed plans

Sensitive or sealed data must remain within its permitted processing environment.

Synthetic test inputs must remain distinguishable from historical real events.

---

# 45. SOMNIA NIGHTMARE GENERATION

The Nightmare Engine creates difficult synthetic scenarios based on known failure classes.

Categories:

- Conflicting instructions
- Missing information
- Contradictory memories
- Outdated documents
- Partial tool failure
- Network interruption
- Unexpected API response
- Malicious retrieved instructions
- False citations
- Repeated failed actions
- Cost-limit exhaustion
- Privacy-boundary challenges
- Long-context degradation

Generate variations using bounded randomness.

Preserve a reproducible random seed and configuration.

Do not create real-world harmful side effects.

---

# 46. SOMNIA EVALUATION

For each synthetic nightmare:

1. Load a sandboxed agent snapshot.
2. Present the synthetic scenario.
3. Execute approved simulated tools.
4. Record decisions.
5. Compare against expected constraints.
6. Measure outcome.
7. Identify failure patterns.
8. Propose improvements.

Track:

- Task completion
- Correctness
- Tool-call count
- Context size
- Token usage
- Latency
- Privacy violations
- Cost-policy violations
- Error recovery
- Citation quality
- Regression failures

Provide clear baseline and candidate comparisons.

---

# 47. SOMNIA MEMORY CONSOLIDATION

Detect:

- Repeated memories
- Near duplicates
- Contradictions
- Superseded facts
- Low-value context
- Recurring procedural lessons
- Frequently needed instructions

Produce a **Consolidation Proposal**.

Possible actions:

- Merge
- Summarize
- Tag
- Archive
- Mark superseded
- Promote confirmed lesson
- Create procedural rule
- Reindex

Do not silently delete original memories.

Keep the originals recoverable.

A proposal should show exactly what would change and why.

---

# 48. SOMNIA LEARNED LESSONS

The engine can propose:

- Improved system procedures
- Agent planning hints
- Retrieval rules
- Workflow safeguards
- Tool-selection recommendations
- Error recovery strategies

Treat generated lessons as candidate policies.

Do not automatically rewrite security-critical instructions or elevate permissions.

Evaluate candidates against held-out tests.

If improvements are not supported by results, do not promote them.

---

# 49. SOMNIA SCHEDULING

Offer:

- Run Now
- Every Night
- Weekly
- After Failed Workflow
- Manual Only

Default to manual or an explicitly enabled schedule.

Scheduled runs must enforce:

- Model cost budget
- Compute budget
- Maximum duration
- Data permissions
- Resource caps
- Battery/power policy where applicable

The app should not consume large amounts of compute overnight without user consent.

---

# 50. SOMNIA DASHBOARD

Provide a clean screen with:

**Agent Health**

- Recent failure rate
- Context efficiency
- Duplicate memory count
- Pending consolidation proposals
- Evaluation coverage

**Nightmare Runs**

- Last run
- Scenarios executed
- Failures found
- Improvements proposed
- Regressions detected

Primary actions:

- Run Nightmare Test
- Review Improvements
- View Results

Avoid anthropomorphic claims such as "ANANSI became smarter overnight" without measured evidence.

---

# 51. SOMNIA IN PRODUCT DEVELOPMENT

Integrate SOMNIA with Product Forge.

After software is generated:

SOMNIA can simulate:

- API failures
- Incorrect user input
- Timeout conditions
- Unexpected dependencies
- Agent tool failures
- Recovery scenarios
- Memory/context failures

For generated applications, ordinary automated software tests and security testing remain mandatory.

SOMNIA augments QA.

It does not replace it.

---

# 52. GHOST-PROMPT

Build:

# GHOST-PROMPT

Subtitle:

**Private Draft Exploration and Intent Clarification**

Purpose:

Help users reflect on how their requests evolved while drafting.

The system can optionally preserve user-visible draft revisions and analyze alternate phrasings.

Do not describe this as reading someone's subconscious.

Typing pauses, deletions, and revisions do not reliably establish hidden intentions or psychological states.

The application must present any interpretation as a hypothesis, never a diagnosis or proven fact.

---

# 53. GHOST-PROMPT PRIVACY RULE

GHOST-PROMPT must be **OFF by default**.

The user must explicitly enable a capture session.

Show a persistent visible indicator:

**Draft Exploration ON**

Explain clearly:

> When enabled, ANANSI can temporarily track revisions made in this editor. Deleted text remains available for your review until you discard it or explicitly save it.

Require meaningful informed consent.

Allow:

- Start
- Pause
- Review
- Clear
- Stop

Do not capture global keyboard activity.

Do not capture passwords.

Do not monitor other applications.

Do not run hidden telemetry.

---

# 54. GHOST-PROMPT SIGNALS

Within ANANSI's own opted-in editor, optionally track:

## Deleted Drafts

Capture selected deleted words or text fragments using editor revision events.

## Pause Events

Record editing intervals above a configurable threshold.

Do not infer emotion from pause duration.

## Revision Patterns

Track how often a sentence, value, or requirement changes.

## Writing Cadence

Optionally measure local editing pace for visualization.

Handle:

- Paste
- Undo/redo
- Selection replacement
- IME composition
- Accessibility input methods
- Voice dictation
- Mobile-style composition if later supported

Do not assume all deletions correspond to Backspace.

---

# 55. GHOST-PROMPT DATA MODEL

Each session contains:

- Session ID
- Editor ID
- Start time
- Explicit consent state
- Draft snapshots or bounded edit events
- Deleted fragments
- Timing metadata
- Final submitted text
- User-approved analysis context

Store events in memory by default.

Apply strict size and time limits.

Do not persist raw deleted drafts unless the user explicitly requests it.

Never silently send deleted drafts to cloud providers.

---

# 56. GHOST-PROMPT VISUALIZATION

Implement an optional Three.js view.

Concept:

- Retained words on a white baseline
- Deleted fragments as translucent violet nodes
- Threads connecting fragments to edit locations
- Pause markers as restrained ripples
- Revision clusters for changed constraints

Avoid excessive particle effects.

Provide:

- Simplified 2D visualization
- Textual revision timeline
- Reduced-motion mode
- Screen-reader-compatible history

The visualization must correspond to real recorded editor events.

---

# 57. GHOST-PROMPT ANALYSIS

After drafting, offer:

**Review My Draft**

Possible findings:

- You changed the requested price several times.
- You removed a requirement about privacy.
- You revised the target audience.
- You replaced a technical term with a broader description.

Then ask:

> Would you like ANANSI to consider any of these earlier alternatives?

Allow the user to explicitly select which fragments to include.

Do not silently append deleted text to the prompt.

---

# 58. GHOST-PROMPT COMPARISON MODE

Create two response panels:

**Final Prompt Only**

The model receives only the final submitted prompt.

**Final Prompt + Approved Draft Context**

The model receives the same final prompt plus user-selected draft history.

To support a meaningful comparison:

- Use the same model when possible.
- Use consistent generation settings.
- Display token/cost differences.
- Clearly show exactly what extra context was included.

Label responses as comparisons, not evidence of subconscious insight.

Users can choose which answer was more useful.

Allow a combined final prompt after user review.

---

# 59. GHOST-PROMPT IN INNOVATION STUDIO

Example:

User writes:

> Build a cheap AI tool for freelancers.

Earlier draft included:

> Maybe lawyers would pay more if privacy were the selling point.

With explicit consent, ANANSI might suggest:

> You also considered a privacy-focused professional market. Would you like to explore that alternative?

The user can send the alternative to:

- Idea Generator
- LEXI-COLLIDER
- Product Forge

No deleted text is transmitted without approval.

---

# 60. THE UNIFIED PRODUCT CREATION PIPELINE

Create a guided workflow:

**Discover**
→ **Invent**
→ **Evaluate**
→ **Build**
→ **Simulate**
→ **Stress-Test**
→ **Improve**
→ **Prepare Launch**

Each stage maps to an ANANSI service.

**Discover**
Opportunity Engine

**Invent**
LEXI-COLLIDER

**Evaluate**
Product Scoring and Feasibility

**Build**
Product Forge

**Simulate**
SwarmTank

**Stress-Test**
SOMNIA and conventional QA

**Improve**
Product Forge revision workflow

**Prepare Launch**
Launch Studio

This is the central product experience.

Users should see progress through one consistent project interface.

---

# 61. LAUNCH STUDIO

Add a final product-preparation stage.

Support:

- Product description
- Target customer
- Pricing proposal
- Landing-page copy
- User onboarding
- Documentation
- Basic SEO plan
- Demo script
- Sales presentation outline
- Release notes
- Deployment checklist
- Post-launch experiment plan

If connected web research is authorized, retrieve current competitor information and cite sources.

Do not invent real customer testimonials, reviews, sales, or conversion metrics.

Actual domain registration, payments, live deployment, and publishing require explicit user approval.

---

# 62. A WORKING EXAMPLE

A user opens ANANSI.

They select:

**Create a Product**

Then:

> I want an AI tool for small construction companies.

ANANSI asks for the intended market and budget, or provides useful defaults.

The Opportunity Engine proposes:

1. AI construction-document assistant
2. Supplier quotation comparison tool
3. Automated project delay monitor
4. Private site-report memory system
5. Equipment maintenance knowledge assistant

The user chooses:

**Supplier quotation comparison tool**

ANANSI generates a product brief.

The user sends it into LEXI-COLLIDER with a second idea:

**Auction bidding optimization**

LEXI-COLLIDER proposes a quotation comparison and negotiation-support architecture.

The user clicks:

**Build Prototype**

Product Forge creates the application repository.

The application is built and tested inside an approved sandbox.

SwarmTank runs a synthetic buyer scenario with construction managers and procurement staff.

The simulation suggests possible pricing and onboarding objections.

ANANSI proposes changes.

The user approves them.

Product Forge revises the application.

SOMNIA tests failure scenarios such as malformed quotations, missing currency information, unsupported document formats, and model timeouts.

The final project contains:

- Working prototype
- Source code
- Tests
- Documentation
- Pricing proposal
- Landing-page draft
- Launch checklist
- Risks requiring real-world validation

This is the expected end-to-end experience.

---

# 63. EXPERIENCE DESIGN

Make this system extremely user-friendly.

The user should not need to understand:

- Multi-agent orchestration
- RAG
- Vector embeddings
- Semantic primitives
- LLM context windows
- JSON schemas
- WebGL
- OAuth protocols
- Agent evaluation harnesses

Expose simple actions:

- Find Ideas
- Combine Ideas
- Build Product
- Test Buyers
- Stress-Test
- Improve
- Export
- Prepare Launch

Advanced controls remain available.

Every screen must answer:

**What is happening?**

**What can I do next?**

**What did ANANSI produce?**

---

# 64. VISUAL DESIGN SYSTEM

Create a premium visual identity.

Style:

- Minimalist
- Modern
- Professional
- Slightly futuristic
- Restrained motion
- Strong typography
- Clear hierarchy
- High readability

Suggested colors:

- Deep graphite
- Near-black
- Warm white
- Violet accent
- Muted blue
- Amber for attention
- Green for success
- Red for failure

Do not overuse glowing elements.

Reserve rich 3D visuals for LEXI-COLLIDER, SwarmTank, and selected GHOST-PROMPT views.

The main ANANSI application must remain calm and functional.

---

# 65. 3D PERFORMANCE AND ACCESSIBILITY

All 3D experiences must support:

- Adaptive rendering quality
- Instanced geometry
- Limited draw calls
- Bounded particle counts
- Reduced motion
- Keyboard interaction
- Accessible alternative views
- Responsive resizing
- Graceful WebGL failure
- Low-power mode

Do not require an expensive GPU.

Do not block core features if 3D rendering is unavailable.

Measure performance on representative hardware.

Never promise a fixed 4-second inference result or 120 FPS without benchmarks.

---

# 66. SHARED BACKGROUND JOB SYSTEM

Use a persistent job scheduler.

Job categories:

- Memory indexing
- Embeddings
- Document ingestion
- LEXI collision
- Product code generation
- Software build
- SwarmTank simulation
- SOMNIA evaluation
- Backup
- Connector synchronization

Job states:

- Queued
- Running
- Paused
- Completed
- Failed
- Cancelled

Jobs need:

- Progress information
- Cancellation
- Retry controls
- Resource limits
- Logs
- Recovery after restart

Do not allow heavy computation to freeze the desktop UI.

---

# 67. AI COST MANAGEMENT

All advanced modules must use AI Matrix budgets.

For each project, allow:

- Free only
- Low cost
- Balanced
- Maximum quality
- Custom budget

Display estimates before expensive operations.

Users should understand when:

- Local compute is used
- Cloud inference is used
- Multiple models are called
- A paid request is required

Zero-Cost Lock must also apply to SOMNIA, LEXI-COLLIDER, SwarmTank, and Product Forge.

---

# 68. MCP INTEGRATION

Implement the current stable MCP protocol after verifying its specification.

ANANSI is both:

- MCP client
- MCP server

MCP client can access approved external tools.

MCP server can expose approved ANANSI knowledge tools.

Possible tools:

- memory.search
- memory.get
- notes.search
- projects.search
- ideas.create
- products.get
- experiments.list

Default external access is disabled.

Require authentication and per-client permissions.

Never expose sealed memories automatically.

---

# 69. ANANSI CORE HARDWARE ARCHITECTURE

Prepare the software to run headlessly on:

- Mini PCs
- Professional workstations
- High-memory Mac Studio systems
- Private GPU servers

The future ANANSI Core appliance may host:

- Memory
- Search
- Local AI
- Product jobs
- SOMNIA evaluations
- Secure shared storage

Desktop applications should connect through authenticated pairing.

Do not expose an unauthenticated LAN interface.

Do not require dedicated hardware for the desktop application to work.

---

# 70. PLATFORM SUPPORT

Initial priority:

**Windows 11**

Then:

**macOS Apple Silicon**

Requirements:

- Native builds
- Secure credential storage
- Signed installers
- Proper updates
- OS-specific permissions
- Offline operation

Use Windows Credential Manager and macOS Keychain.

Support high-memory Apple Silicon through appropriate local AI runtimes.

Do not hardcode assumed RAM sizes or performance figures.

---

# 71. SECURITY MODEL

Use deterministic security enforcement outside the LLM.

Protect against:

- Prompt injection
- Malicious documents
- SSRF
- DNS rebinding
- Path traversal
- Symlink/junction escapes
- Credential leakage
- Unsafe generated code
- MCP tool abuse
- Unauthorized file writes
- Excessive spending
- Malicious package installation
- Supply-chain compromise

An AI-generated instruction is never itself permission to execute an operation.

Every external write, deployment, payment, account change, or destructive action requires an appropriate authorization policy.

---

# 72. SOMNIA SECURITY BOUNDARY

SOMNIA must never test against production accounts using unapproved destructive operations.

Its simulations must use:

- Mock tools
- Test databases
- Redacted snapshots
- Synthetic scenarios
- Reproducible fixtures

Keep evaluated changes separate from the canonical workspace.

Allow preview, approval, versioning, and rollback.

---

# 73. SYNTHETIC DATA PROVENANCE

This applies especially to SOMNIA and SwarmTank.

Every generated synthetic artifact must be labeled:

**Synthetic**

Store:

- Generator model
- Version
- Scenario
- Seed
- Assumptions
- Timestamp
- Validation status

Never allow synthetic personas, hypothetical market demand, or generated failure scenarios to become confirmed real-world memories without explicit review and evidence.

---

# 74. DATA PRIVACY

Use privacy classes:

- Sealed
- Private
- Standard
- Public

Enforce them in native code.

GHOST-PROMPT requires its own separate explicit recording consent.

Never interpret general ANANSI usage consent as permission to record deleted drafts.

SOMNIA must respect source privacy classifications.

SwarmTank should normally use sanitized product descriptions rather than confidential workspace data.

---

# 75. ENCRYPTION

Explain clearly:

Application lock does not automatically mean disk encryption.

Support OS encryption:

- BitLocker
- FileVault

Build an optional encrypted ANANSI Vault using audited libraries.

Do not invent proprietary encryption.

Backups must preserve appropriate confidentiality.

---

# 76. AUTOMATIC BACKUPS

Build:

- Manual backup
- Scheduled backup
- Versioned snapshots
- Integrity verification
- Restore preview
- Recovery documentation

Separate:

- Workspace backup
- Application backup
- Project repository backup
- Product artifacts
- Secure credential references

Do not place secrets in ordinary unencrypted backups.

---

# 77. PERFORMANCE TESTING

Build test datasets with:

- 1,000 memories
- 10,000 memories
- 100,000 memories

For Innovation Studio, benchmark:

- Idea generation
- LEXI extraction
- LEXI recombination
- SwarmTank persona generation
- SwarmTank rendering
- SOMNIA scenario generation
- SOMNIA evaluation
- Product Forge build pipeline

Report measured performance.

Use adaptive quality for low-powered hardware.

---

# 78. QUALITY EVALUATION

Maintain separate evaluation suites for each engine.

**Memory OS**
- Retrieval quality
- Citation correctness
- Temporal reasoning
- Contradiction handling

**AI Matrix**
- Provider compatibility
- Authorization
- Quota behavior
- Privacy enforcement

**LEXI-COLLIDER**
- Concept coherence
- Technical feasibility
- Cross-domain mechanism transfer
- Distinctiveness
- Testability

**Product Forge**
- Compilation
- Functional tests
- Security checks
- Specification adherence
- UX checks

**SwarmTank**
- Scenario reproducibility
- Consistency
- Assumption visibility
- Sensitivity behavior
- Clear synthetic labeling

**SOMNIA**
- Failure discovery
- Regression detection
- False positives
- Policy compliance
- Evaluation repeatability

**GHOST-PROMPT**
- Explicit consent
- Data minimization
- Correct draft-event capture
- Deletion behavior
- Cloud disclosure
- Accessibility

---

# 79. PROJECT STATE AND ARTIFACT REGISTRY

All created artifacts must be versioned.

Each Product Forge project should track:

- Idea version
- Specification version
- Source repository
- Build version
- Test results
- Simulation versions
- SOMNIA evaluation results
- Deployment status

Do not overwrite a working generated product merely because a new version was proposed.

Provide comparison and rollback.

---

# 80. EXPORT AND OWNERSHIP

Users must be able to export:

- Memories
- Notes
- Research
- Product ideas
- Technical specifications
- Source code
- Simulation reports
- Evaluation reports
- Product artifacts
- Project history

Use open formats wherever practical.

Do not lock generated source code inside ANANSI.

---

# 81. COMMERCIAL READINESS

Prepare for future tiers:

- Free
- Pro
- Power
- Business
- Enterprise

Potential Pro features:

- Advanced hybrid memory
- Innovation Studio
- Product Forge workflows
- Advanced AI routing
- Saved simulations

Potential Power features:

- High-volume generation
- Advanced SOMNIA evaluation
- Reusable development agents
- Complex product workflows

Potential Business features:

- Shared workspaces
- Organization controls
- Central policies
- Managed deployment

Do not implement enterprise claims until actual multi-user isolation and security exist.

The user's local data must always remain exportable.

No advertisements inside private memory or AI answers.

---

# 82. COMPLETE IMPLEMENTATION ROADMAP

Build in verified milestones.

## Milestone 0 — Source Protection

- Repository
- Git
- Remote backup configuration
- CI
- Architecture
- Development documentation

## Milestone 1 — Desktop Foundation

- Tauri
- React
- Design system
- SQLite
- Workspace
- Navigation
- Settings

## Milestone 2 — Memory OS

- Memory CRUD
- Notes
- Projects
- Tasks
- People
- Markdown persistence
- Backups

## Milestone 3 — Search and Local AI

- FTS5
- Ollama
- Ask ANANSI
- Streaming
- Cancellation
- Citations

## Milestone 4 — Semantic Intelligence

- Embeddings
- Vector search
- Hybrid retrieval
- Reranking
- Context Compiler

## Milestone 5 — AI Matrix

- Provider registry
- OAuth infrastructure
- OpenRouter
- Other verified provider integrations
- Cost protection
- Automatic routing

## Milestone 6 — Innovation Studio Foundation

- Opportunity Engine
- Idea generator
- Product projects
- Artifact registry
- Idea scoring

## Milestone 7 — Product Forge MVP

- Product specifications
- Repository generation
- One supported application template
- Secure execution environment
- Tests
- Build verification
- Export

## Milestone 8 — LEXI-COLLIDER

- Source abstraction
- Cross-domain mapping
- Recombination
- Feasibility evaluation
- Product Forge integration
- 3D view

## Milestone 9 — SwarmTank

- Structured synthetic personas
- Decision model
- 2D report
- Price and feature scenarios
- 3D arena
- Product Forge improvement loop

## Milestone 10 — SOMNIA

- Agent trace ingestion
- Safe simulation harness
- Nightmare cases
- Evaluations
- Memory consolidation proposals
- Regression comparison
- Scheduler

## Milestone 11 — GHOST-PROMPT

- Consent model
- Draft revision engine
- Review panel
- Comparison mode
- Privacy tests
- 3D visualization

## Milestone 12 — Integrated Product Lifecycle

- Discover-to-launch workflow
- Cross-module artifact sharing
- Product revision history
- Launch Studio

## Milestone 13 — Integrations

- Document formats
- MCP
- Connectors
- Background workflows

## Milestone 14 — macOS and ANANSI Core

- Apple Silicon
- Native macOS build
- Secure appliance services
- Pairing
- Headless mode

## Milestone 15 — Hardening and Release

- Security review
- Performance testing
- Accessibility
- Signed installers
- Recovery testing
- Release certification

Do not skip working foundational milestones to focus on impressive 3D animations.

---

# 83. IMPLEMENTATION BEHAVIOR

At every milestone:

1. Inspect current project state.
2. Confirm the last verified commit.
3. Define acceptance criteria.
4. Implement the smallest complete vertical slice.
5. Run tests.
6. Fix regressions.
7. Run the actual application.
8. Verify observable behavior.
9. Update documentation.
10. Commit.
11. Create or verify a backup.

Never claim that a feature works because its interface exists.

Never substitute generated screenshots for a functioning application.

Never claim a model provider was live-tested when it was only mocked.

Never claim a software prototype is production-ready because it compiled once.

---

# 84. DEFINITION OF DONE — CORE USER EXPERIENCE

A new user can:

1. Install ANANSI.
2. Create a local workspace.
3. Store memories.
4. Import documents.
5. Search semantically.
6. Connect a local model.
7. Ask memory-grounded questions.
8. Receive citations.
9. Connect approved cloud AI providers.
10. Prevent unauthorized cloud transmission.
11. Back up and restore the workspace.

---

# 85. DEFINITION OF DONE — PRODUCT CREATION

A new user can:

1. Open Innovation Studio.
2. Click Generate Ideas.
3. Choose a product concept.
4. Combine it with another idea in LEXI-COLLIDER.
5. Generate a product specification.
6. Open Product Forge.
7. Create an actual repository.
8. Generate application code.
9. Run tests in an approved sandbox.
10. Preview a working prototype.
11. Run a synthetic SwarmTank buyer scenario.
12. Review objections and suggested changes.
13. Request a revised prototype.
14. Run SOMNIA adversarial evaluations.
15. Review improvement proposals.
16. Export source code and documentation.
17. Prepare launch materials.

The workflow must preserve project context and versions automatically.

---

# 86. DEFINITION OF DONE — GHOST-PROMPT

A user can:

1. Explicitly enable Draft Exploration.
2. Type and revise a prompt.
3. See a clear recording indicator.
4. Inspect retained draft fragments.
5. Review optional interpretations.
6. Select which earlier fragments may be used.
7. Compare responses.
8. Delete the session.
9. Verify that no deleted draft remains persisted unexpectedly.

No covert keystroke capture is permitted.

---

# 87. DEFINITION OF DONE — SOMNIA

A user can:

1. Select an agent or Product Forge project.
2. Start an isolated nightmare evaluation.
3. See scenarios generated.
4. Review test results.
5. See recurring weaknesses.
6. Review memory or policy proposals.
7. Approve or reject changes.
8. Re-run evaluations.
9. Compare before and after.
10. Restore the previous version.

No production side effects occur during the isolated evaluation.

---

# 88. FINAL COMPLETION REPORT

At the end of each execution milestone, report:

- Features built
- Files added
- Files modified
- Architecture decisions
- Database migrations
- Tests passed
- Tests failed
- Tests not run
- Security issues
- Live-provider verification
- Known limitations
- Last verified commit
- Backup status
- Current runnable command
- Next milestone

At final release, provide a complete certification report.

Do not invent completion.

---

# 89. FINAL PRODUCT PHILOSOPHY

ANANSI must not simply answer questions.

It should help users:

**Remember better.**

**Think more clearly.**

**Invent new possibilities.**

**Build real products.**

**Evaluate assumptions.**

**Test failure scenarios safely.**

**Improve through evidence.**

And most importantly:

**Own their knowledge, software, and results.**

The 3D experiences should make complex processes understandable.

The AI systems should make sophisticated workflows accessible.

The memory engine should make everything persistent.

The engineering system should make generated products real.

---

# 90. START EXECUTION

You are beginning in an empty project directory.

Start now.

1. Initialize the ANANSI repository.
2. Establish backup and recovery.
3. Scaffold the desktop application.
4. Implement the shared architecture.
5. Create the database and migration system.
6. Implement the first usable memory workflow.
7. Build and test it.
8. Commit the result.
9. Continue through the milestones.

Use the simplest safe architecture capable of supporting the complete vision.

Do not create disconnected demos.

Do not stop after producing a plan.

Do not claim that every requested feature can be delivered in one execution session.

Make concrete, working, tested progress and preserve it so subsequent development sessions can continue without losing work.

**The intended end product is ANANSI ULTIMATE: a private AI memory platform that can discover ideas, invent concepts, build applications, simulate market scenarios, evaluate agents, and help people turn knowledge into actual products.**
