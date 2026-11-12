# ANANSI v1 pilot scope — evidence-backed (release candidate)

Branch: `muse/v1-pilot-release-candidate`, baseline
`dff9bb16688f3af075c395ad8fd5b179af9c3a9d`, integration
merge `d98a229` (repair/review history through reviewer
acceptance `8453f649`). Date: 2026-11-12.

This document states what is actually ready for
controlled customer testing, what is not, and what
remains deferred — from code and recorded evidence,
not from status labels alone. Where status sources
disagree, the resolution is stated with citations.
No completion percentages are claimed: the matrix
uses `provisional / planned / in progress / verified /
blocked / deferred`, and no row is verified without
workflow evidence.

## Reading rules used here

- **Requirement numbering follows
  `docs/REQUIREMENTS_MATRIX.md`** (current). The
  2026-11-05 status ledger
  (`docs/evidence/project-status-ledger-2026-11-05.md`)
  uses a DIFFERENT AN-numbering (e.g. its AN-007 is
  Big Brain, its AN-021 is Opportunity Engine) and
  stale claims; it is a historical snapshot, not the
  authority. See "Contradictions resolved".
- **A Windows verdict belongs to the exact source and
  workflow named in its review.** Later code changes
  do not inherit earlier verdicts. Linux evidence
  never closes a Windows item.
- **README.md is stale on the recovery fault tests.**
  It still says two fault tests time out in the broad
  Windows run with "no clean 403-test Windows
  acceptance claim". That was true at `8df1fcf3d`.
  The 2026-11-12 review at `8453f649` supersedes it:
  **416/416 in one default-concurrency invocation**.
  README is a global status document and was NOT
  edited here; the correction is proposed in the
  release handoff.
- **BUILD_STATUS.md entries are checkpoint-scoped.**
  The newest entries (appended at the end of the
  file) record the WR-REC repair lineage and the
  416/416 focused acceptance. Older entries record
  their own checkpoints; none is re-interpreted as
  current whole-product certification.

## Pilot-ready workflows

"Ready" means: implemented in this tree, exercised by
a named workflow (browser spec and/or Windows review),
and within the pilot conditions below. The v1 pilot
journey walk (`tests/e2e/pilot-journey.spec.ts`,
1/1 on Linux across three green runs (runs 5–7,
2026-11-12) covers the first-time path
end to end with synthetic data and zero external
requests.

- **First run / onboarding (§7, AN-023).** Five
  intents, workspace creation, privacy-limits review
  (plaintext, no encrypted vault, no automatic
  backups) gating AI setup, skip-AI entry, persisted
  preferences, restart persistence. Windows-verified
  at M1.5; journey walk re-verified the flow on Linux.
- **Workspace / projects / memory lifecycle (§8–§10,
  AN-004/AN-024 in-progress scope).** Nine knowledge
  kinds, six lifecycle states, typed relationships,
  immutable revision history, archive/restore,
  project assignment with same-workspace enforcement.
  M2.1–M2.3 Windows evidence; schema 11 in this tree.
- **Search (§11, AN-005 delivered scope).** Title and
  full-body FTS5, recency/BM25 ranking, privacy and
  current-revision gating, Sealed exclusion, grounded
  snippets with headings/line numbers. Hybrid
  lexical/vector retrieval and reranking have M4
  evidence; full-scope AN-005/AN-010 labels are
  superseded (see contradictions) — the pilot claim
  is the delivered retrieval behavior, not the
  historical "verified" label.
- **Ask ANANSI (§13/§19, AN-006 delivered scope).**
  Grounded answers with citations, truthful
  insufficient-evidence and offline handling, data
  disclosure manifest ("View What ANANSI Sent"),
  streaming cancellation, 4,000-char context budget.
  Answered generation requires a reachable local
  runtime (Ollama/LM Studio loopback) or a configured
  provider; without one the product reports Offline
  with compiled citations instead of inventing an
  answer. Windows Ask/storage evidence 2026-10-30
  (68/68 answerable, 80/80 bindings, zero helper
  launches on the clean path); recall repair
  2026-10-29 (90/90 answerable on the fixture).
  Model-answer quality and token measurement are NOT
  pilot claims.
- **Provider setup — local + API key (§14–§18,
  AN-008/AN-009 delivered scope).** Local runtime
  discovery with hardware inspection and explicit
  consent boundary (no background downloads).
  Connect AI guided API-key setup: additive
  provider configuration preserving other providers
  and budgets, connection-bound verification status,
  orphaned-credential handling, Zero-Cost Lock on by
  default, browser sessions refused key entry with a
  desktop direction. Linux-verified (connect-ai e2e
  1/1, provider-dispatch 39/39); **Windows review of
  Connect AI is outstanding, and native
  credential-store (CredUI/keyring) acceptance is
  open** — the pilot must complete the handoff
  checklist before treating key setup as accepted.
- **Document import (§12, AN-025 delivered scope).**
  TXT/Markdown (Windows-verified Slice 1), bounded
  PDF text-layer import with page-aware citations
  (Linux-verified, PDF e2e 5/5; **Windows review
  outstanding**),
  text-bearing DOCX with heading provenance
  (Linux-verified, DOCX e2e 7/7; **Windows review
  outstanding**). Resumable chunked upload, restart
  resume, cancellation cleanup, strict refusals
  (format, size 1–65536 bytes, encoding, empty).
  OCR, scanned PDFs, broad connectors, and
  background ingestion remain planned.
- **Jobs, export, recovery (§66/§79–§80, AN-029
  delivered scope).** Six-state durable jobs, leases,
  cancel/retry/resume, bounded retries for
  unavailable storage, non-retryable parking,
  sanitized recovery diagnostics with explicit
  Retry recovery. FIFO scheduling and the
  passive-wake budget preservation are covered by
  the 416/416 Windows aggregate and the reviewer
  probes. The Documents-view recovery-button click
  is still not browser-exercised (needs the upload
  flow); the Jobs-view interaction is covered.
- **Backup and fresh restore (§76, AN-019 delivered
  scope).** Workspace and application snapshots as
  hashed USTAR packages, plaintext-with-consent,
  schedule consent (defaults off), saved-file import
  with strict validation, preview with schema/scope/
  counts/privacy/hash, fresh-ID restore preserving
  originals, restored schedules disabled. Journey
  walk re-verified backup → saved-file round trip
  (including invalid-archive refusal) → fresh
  restore → reopen on Linux. Encrypted/sealed
  backup and release certification remain open.
- **Product Forge (§28–§33, AN-012 delivered
  scope).** Saved concept → specification → plan →
  validated source package → inert inspection →
  versioned save → export-for-review; bounded
  revision with server-computed diff; reviewable
  export deliverable; model-free source review.
  Static-web prototype template only. **Generated
  code is never executed, built, tested, or
  deployed** (§32 sandbox stays open; verified in
  code: no process execution in the Forge path).
  M7 native desktop acceptance + preview promotion
  stay open.
- **LEXI-COLLIDER (§23–§27, AN-013 delivered
  scope).** Text-first collider, Forge transfer,
  memory context, document/URL beam input with
  bounded fetch, Canvas 2D + Three.js visualization
  with accessible fallback, provenance guards.
  Slices 1–3 Windows-verified; **Slice 4
  (saved-project input) is Linux-only — its Windows
  verification is pending per its own addendum**.
  The interactive Windows file dialog remains open;
  live inference stays provider-gated.
- **SwarmTank (§34–§41, AN-014).** Deterministic
  seeded personas, decision engine, 3D arena + 2D
  fallback, scenario controls, comparison studio,
  timeline/what-if branches (v2), Forge bridge.
  Windows-verified through Slice 3. Provider-backed
  simulation requires explicit provider
  authorization; the calibration plan is shipped but
  NOT performed — persona outcomes are comparison
  material, not real-world validation.
- **SOMNIA (§42–§51, AN-015).** Deterministic
  replay, nightmare generation, inert proposals,
  comparison studio, dashboard aggregates, Forge QA
  bridge. Windows-verified through Slice 3.
  Automatic compaction (import → compact → inspect
  → approve → compare → save) is Linux-verified
  against a frozen oracle (40/40) and a synthetic
  benchmark; **the live-model rerun (AC-I10) is
  blocked** and compaction must not be presented as
  proven to improve a real agent. Lesson promotion,
  scheduling, historical ingestion, and
  provider-backed evaluation are deferred.
- **GHOST-PROMPT (§52–§59, AN-016).** Opt-in draft
  exploration, revision capture, 3D visualization
  with 2D fallback, draft analysis, dual-panel
  comparison. Windows-verified through Slice 3.
- **Lifecycle + Launch Studio (§60–§62, AN-028).**
  Pipeline navigator, Guided Walk (§85), Launch
  Studio deterministic copy + ZIP bundling.
  Windows-verified through M12 Slice 3. Real
  approval is required before publishing.
- **Dashboard, LARP, MCP (§8, §68, AN-017/AN-022).**
  Dashboard (health, projects hub, quick actions)
  Windows-verified. LARP six telemetry views +
  cinematic framing Windows-verified; visualizations
  label synthetic/ornamental content separately from
  observed records. MCP stdio client (review-only
  UI, explicit approval for invocation) and server
  (7 tools incl. mutation-gated `ideas.create`,
  pairing auth, sealed/private boundary) closed on
  Windows; SSE remote hosting, subscription pushes,
  and per-tool grants deferred (ADR-0032).

## Incomplete requirements (pilot-relevant)

Matrix rows `in progress` or `planned` whose full
scope is NOT in the pilot. The delivered subsets
above are usable; the remainder is not claimed.

- **AN-001 installer:** development NSIS exists
  (currentUser, embedded WebView2 bootstrapper,
  unsigned by design). Signed clean-machine release,
  updater qualification, and uninstall audit remain
  M15. **One integrated installer must be qualified
  on a clean machine by the Windows reviewer.**
- **AN-002 browser app:** full browser product /
  release scope open; the pilot uses the paired
  local-API browser client, not a hosted deployment.
- **AN-004/AN-024:** cross-module envelope and
  artifact lifecycle remainder open.
- **AN-006:** full multi-provider + vector
  integration, real-model acceptance, restart/
  timeout breadth open.
- **AN-007 AI Matrix:** planned. Multi-provider
  routing exists in dispatch; the Matrix workflow
  itself is not a pilot claim.
- **AN-008:** the matrix row says `planned`, which
  is stale: Connect AI API-key setup is implemented
  and Linux-verified (see above). What is genuinely
  open: Windows review, native prompt acceptance,
  live-provider verification, and ChatGPT OAuth
  (deferred, disabled — see below).
- **AN-009:** local-model subsets verified; model
  download/setup breadth and bundled weights are
  not pilot claims.
- **AN-011 Big Brain:** design gate resolved
  (spec + ADR-0030); deliberative-loop
  implementation is M5 work, not present.
- **AN-018/AN-020:** cross-cutting security and
  Core/appliance scope open; G1 external audit open
  (see gates).
- **AN-019/AN-029/AN-031/AN-033:** encrypted/sealed
  backup, remaining job categories, vault
  application integration + native/browser
  certification, and export/engine remainder open.
  **Sealed records fail closed everywhere; no
  encryption is claimed for plaintext storage.**
- **AN-025:** OCR, transcription, broad connectors,
  background ingestion planned. DOCX Windows review
  outstanding (above).
- **AN-026 Zero-Cost Lock:** the matrix row says
  `planned`, which is stale: the lock is
  implemented in provider dispatch and the workflow
  gates (verified in code; dispatch tests cover
  preflight refusals). The row predates the
  implementation.
- **AN-027:** Opportunity Studio slices 2–4
  delivered; later studio stages stay planned.
- **AN-030 macOS / AN-032 performance-a11y breadth:**
  planned. No macOS claim; no performance
  certification.
- **AN-021 Enterprise architecture:** deferred
  (2.0 foundation; see below).

## Known defects and open findings

- **M13 Slice 1 Windows findings (2026-11-05):**
  (1) the ingestion spec's Linux-only `ss` usage and
  missing `.exe` — the spec no longer uses `ss`
  (verified in the current tree); (2) concurrent
  duplicate `ingest-begin` returning 503 under
  SQLite contention in `sweep_staging` — **not
  re-verified on this branch**; the queue machinery
  changed substantially since (FIFO, claim budgets,
  recovery). The Windows reviewer should include a
  concurrent duplicate-begin probe; this checkpoint
  makes no claim about it.
- **Pre-existing environmental exclusions** (not
  pilot defects, recorded so the reviewer does not
  chase them): 4 `semantic_workflow` live-Ollama
  failures on Linux (no local model on the VM);
  the node suite's MSVC-installer check fails on
  Linux (Windows-only). The full e2e battery was
  NOT re-run on this branch (see the handoff for
  exactly what ran); specs not listed there carry
  their historical evidence only.
- **Journey walk result:** no reproducible
  pilot-blocking product defect was found on the
  2026-11-12 walk (2/2 green). Walk-harness issues
  found along the way (dashboard landing, search
  default mode, Ollama path selection, fixture
  record counts) were diagnosed against code and
  fixed in the harness; the runs are preserved in
  the delivery evidence.

## Deferred 2.0 work (planning only)

No code, no claims, no pilot availability:

- **ANANSI Corporate** (`plans/post-v1-anansi-corporate.md`,
  CORP-01..08): company file inventory, CRM-style
  pipelines, chat-driven authorized work. Starts
  after v1.
- **Nanzi / 2.0 scope** (`plans/post-v1-nanzi.md`):
  AI teammates, structured decisions, opt-in
  computer scanning into Second Brain bubbles,
  conversation-led workflows.
- **Enterprise Workspaces**
  (`plans/post-v1-enterprise-workspaces.md`):
  institution-managed models/storage, compartmented
  access, protected audit logs; read-only synthetic
  pilot + isolation review gates.
- **ChatGPT OAuth ("Continue with ChatGPT"):** owner
  decision 2026-11-12 — deferred for the first
  pilot, control disabled. The prototype (dynamic
  registration, PKCE, keyring-only tokens, lifecycle
  repairs) stays dev-gated; "open-source partner"
  eligibility is unverified. No sign-in button is
  offered in any build. Codex is a separate slice
  with no generated-code execution.
- **Execution sandbox (§32 / AN-032):** no sandbox
  exists; generated-code execution and deployment
  require a separately approved boundary.
- **Signed installers, updates, G1 audit, macOS,
  hosted multi-user deployment:** M15 / later gates.

## Standing gates (unchanged by this checkpoint)

G1 external audit · Authenticode signing · M7
native desktop acceptance + preview promotion ·
§32 execution sandbox · M5 production gates incl.
live-provider verification · M8 Slice 4 Windows
verification · native credential-store/file-dialog
acceptance · clean-machine installer qualification
· SOMNIA AC-I10 live rerun (blocked: no local
model/credential on the Linux VM; requires the
Windows side).

## Contradictions resolved

| Conflict | Resolution |
| --- | --- |
| Matrix AN-005/AN-010 rows say `verified`, but the 2026-10-20 correction says those labels are superseded (embedding population, semantic Ask integration, reranking, production privacy/error behavior, real-model acceptance incomplete). | The correction governs. Later focused evidence (recall repair, Ask/storage verification) addresses retrieval behavior only and explicitly claims no milestone completion. Pilot scope claims the delivered retrieval/Ask behavior, not the labels. |
| Matrix AN-013 row says Slices 1–4 verified in the native preview, but every gate list says M8 Slice 4 Windows verification is open. | Both true in their scopes: Slice 4 behavior exists in preview builds; its INDEPENDENT Windows verification is pending per its own addendum (Linux-only evidence). The gate stays open. |
| Matrix AN-008/AN-026 rows say `planned`, but Connect AI setup and the Zero-Cost Lock are implemented. | Rows are stale; code + tests govern (Connect AI ops and dispatch lock verified in this tree; dispatch 39/39 on Linux). Windows review + live verification remain open for AN-008. |
| 2026-11-05 ledger uses different AN-numbering and claims AN-019 "encrypted catalog snapshots" and AN-004 "schema 9". | Ledger is a superseded snapshot with divergent numbering. Current authority is the matrix + code: snapshots are plaintext-with-consent (ADR-0021; the UI requires the plaintext-consent checkbox), Sealed fails closed, application schema is 11. |
| README says two fault tests time out / no clean 403-test claim. | Stale at `8df1fcf3d`; superseded by the `8453f649` review (416/416 aggregate). Proposed README update is in the release handoff; README was not edited here. |
| BUILD_STATUS newest entries sit at the end of the file while older entries prepend at the top. | Both conventions are present in the file; entries are read by date and checkpoint, not by position. The merged tree preserves all four 2026-11-12 sections verbatim (Corporate plan + three review records, chronological). |

## Pilot conditions

1. Synthetic or non-important data first; the app
   itself discloses "development preview, not
   production-certified" on the Dashboard.
2. Local models and API-key providers only, each
   explicitly configured and authorized; Zero-Cost
   Lock stays on unless the tester explicitly
   permits paid use with a budget.
3. No ChatGPT sign-in, no generated-code execution,
   no 2.0 features — none are offered.
4. Backups are plaintext snapshots: treat backup
   files and exports as data-bearing; store them
   accordingly.
5. The installer is an UNSIGNED development build;
   SmartScreen/reputation behavior is expected and
   must be recorded, not bypassed with fake signing.
6. Any defect found in pilot testing is reported
   against the exact source commit + `packaging.json`
   identity (the running app shows version 0.1.0 /
   contract 1; the commit hash is not embedded —
   see the handoff).
