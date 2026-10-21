## 2026-10-21 — M7 Slice 1: Product Forge (concept → specification → source package)

Branch `muse/m7-slice1-product-forge` over the verified
R17 preview checkpoint `e915f63` (Windows UI acceptance
recorded by Antigravity at `15affdc2`). One template: a
dependency-free static web application, labelled a
prototype throughout. `forge-spec-*` / `forge-build-*`
runs ride the M5 substrate with the established run
discipline; generation pins exact artifact versions (spec
from the pinned concept version, build from the pinned
specification version); outputs are strict-validated
(package files: 2–12, portable relative paths, allowlisted
extensions, size caps, root index.html + README.md);
saves are immutable `specification` / `source-package`
artifacts; `forge-export` returns a deterministic USTAR
tarball + provenance manifest; restore remaps the
spec/package reference chain fail-closed. Per §32 there
is no execution sandbox in this slice: build/test/
preview/deployment stages are shown Unavailable and
export-for-review is the deliverable; generated code is
never executed. M6 status documentation reconciled in
the same checkpoint (PHASE-006 status, proposal banner;
nothing declared complete that is not). Linux evidence:
core 220/225 (the 4 environmental live-Ollama failures
plus one test list-ordering assumption in the new forge
suite, fixed in this checkpoint and re-verified
standalone: forge unit 6/6, forge_generation 3/3,
forge_restore 2/2); providers+API 144/0; node contracts/UI
54/54; e2e canonical subset 10/10 incl. the new
forge.spec; strict Clippy + fmt clean; docs PASS.
Record: `docs/handoffs/M7-slice1-addendum.md`; design:
`docs/handoffs/M7-slice1-design.md`; AN-012 in progress.
**Pending: Windows verification of this slice; the §32
execution sandbox is a later separately approved slice;
M5 production gates remain open.**

## 2026-10-21 — R17 integrated preview checkpoint (this checkpoint)

Branch `codex/local-testing-r17`, created directly from the
Windows-verified R17 repair tip `ab95e14` (history
preserved; the R11 preview commit `f6cecd2` and the Tauri
`artifact_command` capability fix are ancestors, verified
present in-tree, not re-applied). Contents: everything in
the R16 preview plus M6 Slice 4 (Opportunity Discovery)
and the R17 exact-revision restore repair. This checkpoint
changes only preview materials: `PREVIEW.md` rewritten for
R17 (composition, owner test guide extended with
opportunity discovery and the exact-revision provenance
check) and `Start ANANSI Preview.cmd` pointed at the new
separate data root `%LOCALAPPDATA%\ANANSI-Preview\r17`
(never the owner's existing data; R11/R16 roots untouched).
The owner has confirmed Windows verification of the R17
repair (restore regressions 3/3, opportunity units 5/5,
strict all-target Clippy, docs) on top of the R17 pass.
Linux evidence for this branch: documentation validation
PASS; browser e2e subset re-run on the final tree —
app checkpoint, integrated workflow, opportunities —
**3/3**. Application code is identical to the repair
checkpoint (core 210/214, only the 4 environmental
live-Ollama failures). **Pending: Windows build/launch of
this preview checkpoint.** M5 production gates remain open.

## 2026-10-21 — R17 repair: exact-revision provenance at restore (this checkpoint)

Branch `muse/m6-r17-repair` over the Slice 4 checkpoint
`7253b56`. Windows verification of R17
passed opportunities_generation 6/6, opportunity units 5/5,
opportunities_restore 1/1, UI/contracts 51/51, TypeScript,
strict Clippy and docs — and a focused diagnostic confirmed
one defect: restored opportunity sets kept the SOURCE
workspace's `revisionId` for memory sources. The repair
remaps `revisionId` through the restore identity map and
validates it against the package's own `memory_revisions`
rows (the revision must belong to the referenced memory in
the artifact's own workspace; missing or mismatched fails
the restore closed before anything is written). Original
identities remain recoverable via `restore_origins`.
`opportunities_restore` grows to 3 tests: the original
identity test strengthened with revision assertions, a
real discovery round trip proving the restored set
resolves its pinned historical revision to exactly the
text generation used (not the memory's later revision),
and missing/mismatched revision refusals with a control.
Full record: `docs/handoffs/M6-r17-repair-addendum.md`.
Linux evidence on the final tree: core **210/214** (only
the 4 environmental live-Ollama failures), clippy
`--all-targets` + fmt clean. **Windows verification of
this repair is pending**; M5 production gates remain open.

## 2026-10-21 — M6 Slice 4: project-scoped Opportunity Discovery (this checkpoint)

Branch `muse/m6-slice4-opportunity-discovery` over the
integrated preview checkpoint `2665e22` (tip `4e24f02`).
Owner-approved scope: discovery from **deliberately supplied
information only** — typed/pasted problem, complaint or
observation, a supplied document excerpt, and explicitly
selected memories; no background scanning, web research,
connectors or autonomous actions. New `opportunities-*`
operations ride the existing M5 substrate and Slice 2 run
discipline (no parallel system): supplied sources are
run-identified, privacy-classed wire segments framed as data
never instructions; parsing enforces grounding (citations
to non-supplied ids dropped; a set citing nothing supplied
is refused); saved sets are immutable `idea` artifacts in
the `anansi-opportunity-set` format, directly evaluable by
the Slice 3 rubric workflow; restore remaps body memory
source ids to the restored memories (fail-closed) while
supplied text passes through byte-identical. UI gains an
Opportunity discovery section with a pre-send review of
exactly what will be sent. Full record:
`docs/handoffs/M6-slice4-addendum.md`; design + acceptance:
`docs/handoffs/M6-slice4-design.md`.
Linux evidence on the final tree: core **208/212** (only
the 4 environmental live-Ollama `semantic_workflow` tests
fail; new opportunities unit 5/5, generation 6/6, restore
1/1; restore-change regressions artifacts 6/6,
backup_artifacts 5/5, backup_restore 7/7, evaluations 3/3,
ideas 7/7 + 4/4); providers+API **144/0**; node contracts+UI
**51/51**; tsc/contracts clean; web build ok; browser e2e
**9/9** sequential incl. the new `opportunities.spec.ts`
(discover → save → evaluate → backup → fresh restore →
restored sources resolve); clippy `--all-targets` + fmt
clean. **Windows verification of Slice 4 is pending** — the
owner's R16 preview pass and Windows `vault_increment2` 9/9
do not cover it. M5 production gates remain open.

## 2026-10-21 — R16 integrated preview checkpoint + vault ladder reconciliation (this checkpoint)

Branch `codex/local-testing-r16`, from the verified R16 tip
`a1b0c80` directly (the R11 preview commit `f6cecd2` is an
ancestor; the Tauri `artifact_command` capability fix is
preserved). Composition: R16 + the vault schema-ladder
reconciliation (branch `muse/vault-ladder-reconciliation`;
production fix: vault snapshot verification/staging no
longer migrates the live catalog it opens, so a genuine
schema-8 seal now lands at 9 and a pre-switch abort leaves
schema 8 with no registry table; crash matrix runs on real
legacy-8 and explicit schema-10 fixtures — see
`docs/handoffs/vault-ladder-reconciliation-addendum.md`) +
updated PREVIEW.md/launcher (separate data root
`%LOCALAPPDATA%\ANANSI-Preview\r16`; Windows build/launch
instructions; owner test guide) + a new integrated browser
spec (project → ideas → saved artifact → evaluation →
backup → fresh restore → restart → reopen; restored
evaluations resolve to restored sources) + a production UI
fix for a rubric-load race the verification exposed (rubric
edits made while a rubric read was in flight were silently
overwritten; the form is now disabled during loads and
superseded reads never apply) + an order-agnostic
evaluations e2e history check. Full record:
`docs/handoffs/R16-preview-checkpoint-addendum.md`.
Linux evidence on the final tree: core suite 196/200 (only
the 4 live-Ollama `semantic_workflow` tests fail —
environmental, no Ollama on this machine; `vault_increment2`
9/9, ladder closed on Linux); `backup_artifacts` 5/5,
`backup_restore` 7/7, lib 34/34; vault suites 11/11 + 2/2 +
1/1 + 3/3 + 1/1 + 2/2; providers+API 144 passed / 0 failed;
node contracts+UI 47/47, tsc clean, contracts check clean,
web build ok; browser e2e 8/8 (integrated workflow,
checkpoint, artifacts, ideas, evaluations, providers,
vault, backups); clippy `-D warnings` clean incl.
`--all-targets`; fmt clean. **Windows verification of this
checkpoint is pending** (the owner's pass covers R16 and
R15's suites); the PREVIEW.md Windows steps were not
executed on the preparing (Linux) machine. M5 production
gates remain open (Windows native acceptance incl. real
CredUI/keyring, live-provider verification, G1 audit,
remote CI); Slice 4 stays owner-gated. No pushes, no
master merge.

## 2026-10-21 — M6 R15 repair: eviction-test synchronization + strict clippy (this checkpoint)

Repairs the two remaining findings from the owner's Windows
review of R15, on branch `muse/m6-r15-repair` from `d56ed52`.
Test files only — no production code changed; the R15
recovery implementation is preserved byte-for-byte. The
cap-eviction test's timing assumption (32 merely-slow streams
could finish naturally before the evicting start on a slow
machine) is replaced with server-gated synchronization: the
synthetic provider holds streams behind a gate, observes
client closes directly as socket EOF, and attributes
connections to runs by topic marker; the test proves
oldest + neighbour active (running + one ledger token hold
each), eviction closes exactly the oldest connection, the
evicted reservation is released (32 holds / 33 total
requests), and cleanup drains every test-owned stream to
zero reservations. A mutation check (eviction-cancel
disabled, then reverted) fails the test immediately, so the
backend behaviour is confirmed sound. Strict clippy on the
`backup_artifacts` target is fixed at the cause (unused
`legacy_package` parameter removed; one-element loop in
`rich_fixture` inlined), no suppressions. Linux evidence:
eviction test 10/10 repeats + 3/3 under CPU contention;
focused suites `ideas_generation` 7/7, `backup_artifacts`
5/5, `ideas_validation` 4/4, `evaluations` 3/3,
`backup_restore` 7/7, `artifacts_registry` 6/6, lib 34/34;
clippy `-D warnings` clean for lib, both touched test
targets and `--all-targets`; fmt clean; full core suite
193/199 with only the known failures (4 live-Ollama; 2
`vault_increment2` ladder assertions, left unchanged for
the vault owner). Full record:
`docs/handoffs/M6-r15-repair-addendum.md`. Not merged, not
pushed. Windows re-verification of the rewritten test,
application acceptance and preview promotion remain
pending; the vault assertions stay explicitly open.

## 2026-10-21 — M6 backup/recovery repair (R15 checkpoint)

Repairs the four independently reviewed backup/recovery
defects (Antigravity R12-DEF-01..04) on branch
`muse/m6-backup-recovery` from the R14 tip `bb7cc24`
(the R11 preview commit `f6cecd2` is an ancestor, not
re-cherry-picked): workspace backups include
`artifact_versions`; schema verification recognises the
exact core/vault/FTS/embeddings object groups instead of
one monolithic schema, so application backups of used
databases verify; artifact rows are validated in the
staged catalog (hashes, privacy, version contiguity,
reference resolution incl. evaluation provenance);
restore imports artifacts with ids and embedded references
remapped (fail-closed on references outside the package),
refuses duplicate restores of the same backup artifact,
and rebuilds FTS from restored content; schema-8/9
packages restore through a validated staging-copy
migration. Linux evidence: new `backup_artifacts` suite
5/5, focused backup/artifact/ideas/evaluation suites
27/27, lib 34/34, providers 120/120, UI/contracts 47/47,
browser e2e 5/5, full core suite 193/199 with only the
known failures (4 live-Ollama; 2 `vault_increment2`
assertions root-caused to the Slice 1 schema ladder -
baseline `fc2540f` passes that suite 8/8 - left unchanged
for the vault owner). Design and full evidence:
`docs/handoffs/M6-backup-recovery-design.md`,
`docs/handoffs/M6-backup-recovery-addendum.md`. Not
merged, not pushed. Windows verification of this
checkpoint and preview promotion remain pending.

# ANANSI Ultimate — Build Status

## M6 Slices 2–3 + R13 reliability repair — 2026-10-21 (Linux evidence)

Branch `muse/m6-slice3-evaluation` (over the R13 repair checkpoint on
`muse/m6-slice2-idea-generation`; R11 preview ancestry `f6cecd2` preserved).
Last verified commit: see the delivery addenda
(`docs/handoffs/M6-r13-repair-addendum.md`, `M6-slice3-addendum.md`).

- **R13 repair** of the independent Windows review's four findings:
  atomic save idempotency (claim under lock, register outside it),
  idea-run lifecycle controller (cancel on project switch, stale-response
  guards, serialized polling), bounded ideas registry (expiry/eviction
  cancels active provider streams outside the lock, with drain-to-terminal
  settlement), strict clippy clean at the causes. New regressions for each.
- **M6 Slice 3 — transparent concept evaluation & comparison** (the M6
  plan's "Scoring rubric v1", with owner decision 1 settled): user-entered
  1–5 assessments with rationales under a versioned rubric artifact
  (owner-approved standard rubric: problem clarity, customer specificity,
  MVP feasibility, budget/technology fit, validation readiness; equal
  weights); the only computation is core's deterministic weighted summary
  over assessed dimensions (missing = "not assessed", never zero);
  evaluations are immutable artifacts citing exact source/rubric versions
  with the rubric snapshot embedded; rubric revision never rewrites
  history; saves are content-hash idempotent. No model calls in this slice.
- Linux evidence: evaluations units 4/4 + integration 3/3; ideas 11/11 +
  eviction units 2/2; providers 120/120; API 24/24; UI/contracts 47/47;
  clippy `-D warnings` clean; fmt/tsc/contracts/docs clean; browser e2e
  5/5 (checkpoint, artifacts, ideas, evaluations, providers).
- Full anansi-core suite: 188 passed / 6 failed of 194 — the 4 known
  live-Ollama semantic tests plus 2 `vault_increment2` tests whose harness
  deadlines are load-sensitive on this VM; the identical failure was
  reproduced on the pre-repair tip `cb8c1cb`, i.e. not a regression of
  these changes. (Correction recorded in the R13 addendum: R11/R12 reports'
  ~497/506 core pass totals were miscounted aggregates; the suite holds
  187 tests pre-Slice-3, 194 now.)
- Still open and unclaimed: Windows verification of the repair + Slice 3,
  M5's native/manual acceptance, live-provider verification, G1 external
  audit, remote CI. M6 Slice 4 (Opportunity Engine) awaits the owner's
  remaining plan decisions; no further milestone work is started.


## Local integrated preview — 2026-10-21

Branch `codex/local-testing-r11` contains Muse R11 (`07be39ca`, including integrated M2–M5 and model-free M6 Slice 1), with a local repair granting the main native window access to `artifact_command`. A normal embedded Windows development executable was built and launched with a responsive `ANANSI Ultimate` window and an isolated preview catalog. See [preview instructions](../PREVIEW.md). Six registry tests, 39 UI/contract tests, TypeScript, web build and documentation checks passed. This is build/launch evidence, not full native artifact workflow acceptance or release certification. Existing audit, manual credential, remote CI and source-publication deferrals remain. The primary checkout and protected master were not changed.

## Current authoritative summary — 2026-10-20

Local M2–M4 integration and regression tasks on `codex/m2-m3-integration` are complete through implementation commits `ee92afb` and `b6d472c` (starting from `5dd40d3` / `b543c7b`, retained M2 checkpoint `bd2e2a9`). This summary supersedes historical completion/certification and stop instructions below. Integration owner/worktree and acceptance checklist: [handoff](handoffs/M2-M4-integration-owner.md).

Verified locally in desktop (Tauri/WebView2) and browser (Playwright + loopback HTTP API):
- **Semantic Search, Explicit Indexing, Visible Fallback, Atomic Publication & Reranking (`ee92afb`)**: Separate generation (`modelTag`) and embedding (`embeddingModel`) configuration; explicit Search UI model/endpoint controls and workspace indexing (`#search-index-embeddings`); visible `.status-lexical-fallback` and `.status-semantic-active` status banners; atomic multi-chunk `SAVEPOINT` publication with concurrent-edit and partial-failure rollback tests; post-fusion `rerankScore` with `rerankProvenance`, `validUntil` temporal filtering, and contradiction lifecycle evaluation; full UI E2E workflow (`save record -> index -> retrieve paraphrase with zero keyword overlap -> revise -> verify old embeddings invalidated -> restart and search again`) against live `all-minilm:22m`.
- **Vault Browser Native Authorization & Encrypted Backup/Export/Restore (`b6d472c`)**: Origin/workspace/process-bound browser native authorization via `WindowsCredentialProvider::for_browser` (`/v1/vault/*` with secret-DTO rejection and desktop-only initial `setup`/`seal`); encrypted ciphertext backup, encrypted export, locked archive preview, tamper/wrong-key/cancel refusal, and fresh sealed restore verified in both Playwright (`tests/e2e/vault.spec.ts`) and Windows Tauri/WebView2 (`scripts/verify-vault-desktop.mjs`).
- **Local Ask & Context Compiler**: Real-model Ask streaming (`qwen2.5:0.5b`), cited SQLite answers, disclosure manifest, and backend socket cancellation pass in browser and Windows Tauri UI (`scripts/verify-ask-native.mjs`). Big Brain (`AN-011`) remains design-only (`docs/designs/BIG_BRAIN_SPEC.md`, ADR-0030).

Remaining local prerequisite and separate deferred release/publication gates (recorded in [M5 readiness](evidence/M5-READINESS.md)):
1. **Local development prerequisite — Manual Win32 Credential & Recovery Dialog Check**: Prepared via `node scripts/with-application-test-lease.mjs scripts/manual-vault-acceptance.mjs` (without `ANANSI_VAULT_TEST_PROVIDER`) for human execution of real Win32 `CredUIPromptForCredentialsW` and `MessageBoxW` dialogs (never automated). The initial human recovery acknowledgement failed; a successful retry and remaining manual workflow have not been reported. M5 task preparation remains pending that local result.
2. **Pre-Production External Cryptographic Audit (`G1-External-Audit`)**: Open pre-production gate under accepted engineering risk; audit monitor remains paused per Owner direction.
3. **Remote CI & Protected Publication**: Deferred by Owner capacity policy. No M5 implementation is started.

## Historical evidence and claims (superseded where inconsistent above)

## Owner amendment — M2.7-Core isolation (2026-10-19)

M2.7-Core Increment 1 is recoverable at `a7b4794fd0a0f4d39d9999f278a536711efa767d`; all three exact-SHA jobs passed in [run 39195589192](https://github.com/robillionair/Anansi-ultimate/actions/runs/39195589192), including Ubuntu Foundation, confirming the optional Linux Secret Service feature repair, Windows Foundation and native smoke. [Increment 1 evidence](evidence/M2-7.md) preserves its exact test sequencing and limitations.

Increment 2 local core acceptance passed: explicit schema-9 encrypted full metadata/history/graph generations, cross-process root/KDF admission, durable epochs and atomic cutover, full DEK rotation, recovery acknowledgement and encrypted backup/export/fresh restore. Passed across runs: 87 distinct Rust entries (one opt-in native-store test ignored), all ten process-crash boundaries, 40 tooling tests and 13 recovery checks, fmt/clippy/docs. [Increment 2 evidence](evidence/M2-7-increment2.md) preserves exact sequencing, including a host-sleep expiry refusal and clean full matrix rerun without bypass. This is not a complete or application-enabled vault. Search/UI/adapters/workflows, the live schema-8 Core and Sealed refusal remain unchanged. Legacy plaintext copies and interrupted stages are retained; no forensic erasure or complete whole-root at-rest conversion is claimed. Prior scoped verified source checkpoint is `a7b4794`; final new checkpoint/tree/CI/source restore belong in the external Increment 2 ledger after commit. No M3/master publication is performed here.

M2 integration increment: the vault-owned `SearchBoundary` is now a tested domain hook for the later M3 consumer. It excludes Sealed records before persistent indexing and rechecks caller, session lifetime and durable generation epoch immediately before any Sealed result publication. [Handoff](handoffs/M2-to-M3.md) records the boundary and the required stale-FTS/rebuild/lock/revocation review. This does not activate Sealed commands, modify M3 search/UI files, or claim M2.8 acceptance.

M2 integration increment 2 adds vault-owned transactional FTS invalidation helpers for one record or an entire workspace, with focused in-memory SQLite tests. They are designed to run in the same sealing/revocation/generation transaction and do not expose vault keys. M3 application wiring, native/browser workflows, encrypted backup/restore integration, and M2.8 acceptance remain pending.

M2/M3 shared search integration is now applied at `94c27b8` plus local contract repairs: M3’s bounded body search and rebuild/error-integrity fixes are present, `MemoryMetadata` carries optional response-only ranking/evidence fields, and memory writes synchronize or invalidate FTS rows in the same SQLite transaction. Local body-search, corpus-limit, and schema-8 sealing-transition tests pass. The true Sealed vertical slice remains blocked by the missing adapter-facing vault setup/session command and trusted native credential/prompt path; Sealed creation continues to fail closed. No whole-root encryption or secure erasure is inferred from search exclusion.

M2.7 runtime increment adds `VaultRuntime`, which owns the non-serializable acknowledged setup and process-local sessions behind a native-only credential provider. Browser setup is rejected before credential input; setup cannot publish without a verified M2.5 backup reference. [Runtime evidence](evidence/M2-7-runtime-acceptance.md) records the focused check. Adapter command wiring and trusted Windows prompt implementation remain next; no Sealed activation or M2.8 claim is made.

The follow-up runtime increment adds adapter-facing `context` and authenticated `read` methods. They return only a vault context/snapshot after the existing session, caller, deadline and epoch checks; no key or credential transport is introduced. Focused runtime test and warnings-denied Clippy pass. Native prompt, command transport, browser authorization and integrated Sealed workflow remain pending.

The process-owned `VaultService` increment now provides a cloneable adapter handle over a mutex-protected `VaultRuntime`, with setup, seal, unlock, authenticated read, lock and context operations. This preserves process-local secret/session ownership and is locally compiled and linted. It does not claim desktop/browser activation; native prompt and transport wiring remain incomplete.

M2.7 native application integration now passes the actual Windows Tauri/WebView2 workflow: persistent adapter-owned service, trusted native credential/recovery controls, verified-backup sealing of a synthetic Private record, authorized encrypted read, manual-lock denial, restart-locked denial and restart unlock/read. [Evidence](evidence/M2-7-native-application.md) records the exact boundary and synthetic evidence root. Browser authorization, encrypted portable backup/restore and full M2.8 remain pending; remote CI/publication is deferred and G1-External-Audit remains open.

[Owner risk acceptance](approvals/M2-6-owner-risk-acceptance.md) explicitly supersedes historical G1 pre-implementation stop language below. G1-External-Audit remains open for post-M3/pre-production/commercialization certification; no audit coverage is fabricated. M2.5 source/CI recovery baseline was reverified. M2.7-Core is authorized in a separate worktree; M3 Search/UI wiring and shared-checkout files remain untouched. G2–G4 are unchanged. No vault completion, production certification or implemented UI disclosure is claimed. AN-018/019/031 remain in progress.

## Current state — 2026-10-19

M4.5 cross-edition UI and integrated acceptance certification is verified under [M4.5 evidence](evidence/M4-5.md), completing Milestone 4 (Semantic Intelligence). Exposes 3 search modalities ("Title only", "Full-body search (includes Markdown text)", and "Hybrid semantic") across shared UI and Playwright E2E browser testing, wires scoped HTTP (`/v1/ai/hybrid-search`, `/v1/ai/context`) and Tauri IPC (`ai_command` with `'hybrid-search'` and `'context'`) endpoints, presents grounded evidence with RRF scores, distinct lexical/semantic ranks, line numbers, and section headings, and enforces fail-closed Sealed privacy refusal across all search and context boundaries. Passed: full workspace Cargo test battery (`anansi-core`, `anansi-api`), 30/30 UI/contract tests, 2/2 Playwright browser integration tests (`search.spec.ts` and `ask_anansi.spec.ts`), 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift, and full repository planning self-test validation. Milestone 4 is 100% certified.

M4.4 complete Context Compiler and Big Brain deliberative design gate are verified under [M4.4 evidence](evidence/M4-4.md). Multi-chunk section-aware context compilation (`crates/core/src/context.rs`), strict token/character budget packing with clean chunk truncation, grounded citation generation (`[1]`, `[2]`), chunk deduplication across memories, fail-closed Sealed privacy refusal, complete data disclosure manifest ("View What ANANSI Sent"), and architectural resolution of the Big Brain (AN-011) design gate via formal behavior contract and state machine specification (`docs/designs/BIG_BRAIN_SPEC.md` and ADR-0030) are verified. Passed: 5/5 targeted context compiler tests, 19/19 combined M4/Ask tests, 27/27 UI/contracts tests, 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. Next scope is M4.5 cross-edition UI and integrated acceptance certification.

M4.3 hybrid retrieval fusing SQLite FTS5 lexical BM25 ranking and vector cosine ranking via Reciprocal Rank Fusion (RRF) is verified under [M4.3 evidence](evidence/M4-3.md). Dual-modality retrieval pipeline, configurable RRF scoring ($w_{\text{lex}} / (k + \text{rank}_{\text{lex}}) + w_{\text{sem}} / (k + \text{rank}_{\text{sem}})$), grounded evidence locations (Markdown heading and line number), metadata and entity filtering (`kind`), archive exclusion (`archived = 0`), fail-closed Sealed exclusion (`privacy != 'sealed'`), and a semantic evaluation benchmark suite covering exact lexical matches, semantic paraphrases, dual-modality synergy, and missing evidence are verified. Passed: 5/5 targeted hybrid retrieval tests, 15/15 combined M4 tests, 27/27 UI/contracts tests, 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. Next scope is M4.4 complete Context Compiler and Big Brain tracked design.

M4.2 vector search and SQLite cache synchronization are verified under [M4.2 evidence](evidence/M4-2.md). In-memory cosine similarity vector search (`vector_search`), descending similarity ranking with stable tie-breaking, threshold filtering (`min_score`), limit pagination (1..100), active memory filtering (`archived = 0`), fail-closed Sealed exclusion (`privacy != 'sealed'`), and transactional cache invalidation upon memory mutation (`invalidate_embeddings` in `sync_index_tx`) are verified. Passed: 5/5 targeted vector search tests, 5/5 targeted vector embedding tests, 27/27 UI/contracts tests, 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. Next scope is M4.3 hybrid retrieval and Reciprocal Rank Fusion (RRF).

M4.1 local embedding generation and pure Rust vector space primitives are verified under [M4.1 evidence](evidence/M4-1.md). Mathematical vector operations (L2 norm, normalization, dot product, cosine similarity clamped to `[-1.0, 1.0]`), strict dimension bounds (2 to 1536 float32 dimensions), NaN/infinity validation, IEEE 754 little-endian binary serialization (`as_chunks::<4>()`), SQLite embedding cache persistence (`memory_embeddings` table with upsert, workspace listing, and memory cascade deletion), loopback-only Ollama embedding client (`/api/embeddings` with 10s timeout), and fail-closed Sealed memory exclusion are verified. Passed: 5/5 targeted vector tests, 27/27 UI/contracts tests, 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. Next scope is M4.2 vector search and SQLite cache synchronization.

M3.5 integrated certification across full-body search, local runtime discovery, minimum Context Compiler, and local Ask ANANSI is verified under [M3.5 evidence](evidence/M3-5.md). Parity between the native desktop and browser editions (over authenticated loopback HTTP API and native IPC `ai_command`), graceful offline transitions, grounded citations with Markdown section headings and line numbers, truthful missing-evidence handling without hallucination, data disclosure manifest ("View What ANANSI Sent"), coordinated fail-closed Sealed exclusion with M2 vault boundary, and 100% loopback isolation with zero external requests (`external === 0`) are verified. Passed: 1/1 Playwright end-to-end integration test (`tests/e2e/ask_anansi.spec.ts`), 27/27 UI/contracts tests, 4/4 targeted Ask tests, 7/7 AI discovery tests, 2/2 vault search invalidation tests, 0 Clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. M3 milestone local acceptance is complete.

M3.4 minimum Context Compiler and local Ask ANANSI query pipeline is verified under [M3.4 evidence](evidence/M3-4.md). Core grounding engine, citation compiler (`GroundedCitation` with index, memory ID, title, kind, section heading, line number, snippet), strict context budget enforcement (bounded to 4,000 characters / ~1,000 tokens), deduplication, truthful missing-evidence handling (`AskStatus::InsufficientEvidence`), offline resilience (`AskStatus::Offline`), full data disclosure manifest ("View What ANANSI Sent"), pure loopback inference over Ollama (`/api/generate`) and LM Studio (`/v1/chat/completions`), Core/Tauri/API routes, generated TypeScript contracts, and the "HOME -> Ask ANANSI" shared UI view are verified. Passed: 4/4 targeted Ask tests, 27/27 UI/contract tests, 0 clippy warnings (`-D warnings`), 0 format diffs, 0 contract drift. Next scope is M3.5 integrated certification.

M3.3 authorized local AI runtime discovery and hardware profiling is verified under [M3.3 evidence](evidence/M3-3.md). Native hardware inspection (CPU architecture/cores, total/available physical RAM via Win32 `GlobalMemoryStatusEx`, storage free space via `GetDiskFreeSpaceExW`, discrete GPU name/VRAM inspection), strict loopback isolation and offline resilience (<2s timeouts, loopback-only validation), tiered model recommendations (1.5B/3B, 7B/8B, 14B), explicit download consent enforcement (no automatic pulls/telemetry), Core/Tauri/API command routing, typed contracts, and the "SYSTEM -> Local Models" UI are verified. Passed: 94 Rust tests, 25 UI/contract tests, 40 tooling and 13 recovery tests, zero clippy warnings with `-D warnings`, zero format diffs, zero contract drift.

M2 vault/search handoff consumed read-only from isolated checkpoint `e67010ebd2461868874e311c9bd764554f574326`. M3 added `vault_search_integration`, which simulates a schema-8 Private→Sealed transition and verifies rebuild plus restart remove a plaintext FTS canary. This is a current-index privacy regression only; live `SearchBoundary` session/epoch wiring remains pending coordinated M2 integration. Exact shared-file obligations are recorded in [the M3→M2 handoff](handoffs/M3-to-M2.md). Local test passed; remote CI remains deferred.

Current authorized increment: **M3.2 full-body indexing and retrieval evaluation**, verified under [the detailed plan](../plans/03-search-and-ollama.md) and [evidence ledger](evidence/M3-2.md). M3.1 is verified complete at published checkpoint `8c9406b`. The user explicitly authorized work independent of audit feedback. This supersedes earlier blanket stop-before-M3 scheduling text for M3 search increments. M2 remains incomplete and G1 remains mandatory. [RustCrypto member feedback](https://github.com/RustCrypto/password-hashes/issues/940#issuecomment-6256202461) says he does not think argon2 has ever been audited; [our authorized acknowledgement](https://github.com/RustCrypto/password-hashes/issues/940#issuecomment-6256248503) retains the gate. An active hourly follow-up checks for new actionable evidence and checkpoint CI. No audit commission, crypto dependency or inference was introduced.

M2.6 G1 follow-up: the user selected **“Retain pure Rust; obtain matching audit evidence.”** [The evidence review](evidence/M2-6-audit-review.md) records verified direct-package source identities and report scope exclusions. [The library comparison](../plans/M2-vault-library-options.md) retains native libsodium as unselected research. G1 remains open for matching independent Argon2 evidence and current AEAD/dependency mapping. After explicit user authorization, [the precise maintainer inquiry](reviews/M2-6-audit-inquiry.md) was published as [upstream issue 940](https://github.com/RustCrypto/password-hashes/issues/940). No paid review occurred; a request does not close G1. No vault product work or dependency change. Prior design checkpoint is `57abb23c8fa2065ae888dc4cbb1cf7325cfe6c10`; publication/recovery for this follow-up is recorded after validation in the external audit-review ledger.

M2.5 is verified and published at `6c193ef8c9891e966c547451b4e01b7db74215b4` (implementation `cfc28af`). All three implementation and checkpoint CI jobs passed, including native backup workflows and unsigned installer verification; protected master and independent source recovery are verified. See [M2.5 evidence](evidence/M2-5.md) and its permanent ledger. M2 remains incomplete.

Retained M2 blocker: M2.6 security-design reconciliation. The user's conditional M2.7 approval is preserved in [the original contract](approvals/M2-6-user-contract.md), correcting the obsolete awaiting-approval claim. [The detailed plan](../plans/M2-vault-security.md) and [review evidence](evidence/M2-6.md) record official source/version checks and the adopted [contract corrections](approvals/M2-6-corrections.md). G2–G4 are resolved at design level; G1 audit coverage remains open. No crypto dependency, vault code, schema change or secret-store write was added. M2.7 is blocked on that audit evidence; no repeated blanket user approval is needed. Do not start M2.7 until G1 closes; the independently authorized M3.1 scheduling exception is recorded above.

M0 and M1.0–M1.5 are verified complete. M1 completion checkpoint 97babb3ae9f3dc20b5602a7b46a947e833943eb2 passed all three CI jobs in run 38955878511, was published to protected master and tagged m1-desktop-foundation-verified. Final source bundle restored identical HEAD/tree/tag and passed documentation checks. The downloaded CI installer passed the full local installed-release native/browser workflow and data-preserving uninstall.

M2.1 is verified complete. Implementation checkpoint 39b0e5ae440223a87ac394c60706160a8f237ca6 passed all three jobs in [CI run 38962588037](https://github.com/robillionair/Anansi-ultimate/actions/runs/38962588037), including actual native memory verification and release packaging/install/uninstall. It was published to protected master; independent source restore matched HEAD/tree and passed frozen install, types, client/tooling/recovery tests. Last verified implementation SHA is recorded rather than this metadata commit to avoid self-reference. See [M2.1 evidence](evidence/M2-1.md). M2.2 completion is recorded below.

M2.2 is verified complete. Implementation checkpoint aac601dcee12067a1bbee44b62d72db0a3a6122b passed all three jobs in [CI run 38990088817](https://github.com/robillionair/Anansi-ultimate/actions/runs/38990088817), including actual native/browser lifecycle/restart verification and development release packaging/install/uninstall. It was published to protected master. Independent source restore matched HEAD/tree and passed frozen install, docs, types, 13 client/component tests, 40 tooling tests and 13 recovery checks. Local acceptance included 46 Rust tests and nine browser workflows; the final pending-save repair passed both memory workflows again. See [M2.2 evidence](evidence/M2-2.md). Last verified implementation SHA is recorded rather than this metadata commit to avoid self-reference.

M2.3 acceptance repair is verified at implementation checkpoint 5b83338d08884349558a7145de9a5f30c75dd469, superseding the incomplete 82789d3 completion claim. Schema 6 adds immutable relationship revisions/provenance and reversible optimistic archive/restore; historical record reads retain saved link snapshots; project membership requires a same-workspace project, omission retains assignment and explicit null clears it. All eight linked record kinds, history, completed tasks, clear/reassign and both desktop/API process restarts passed actual shared-UI verification with zero external requests. Narrow filter/action readability was inspected and tested after correction. Local checks: 56 Rust tests, 18 UI/contract tests, 10 browser tests, 40 tooling/hygiene tests and 13 recovery checks; types/contracts/format/clippy/build/docs passed. Both required foundation jobs in [CI run 39078924117](https://github.com/robillionair/Anansi-ultimate/actions/runs/39078924117) passed; the separate native CI job was still running when this record was authored (the same native verifiers already passed locally). Consult the linked run for its final packaging results. See [M2.3 evidence](evidence/M2-3.md). Last verified implementation SHA is used rather than the subsequent evidence checkpoint to avoid self-reference. Stop before M2.4.

## M1 certification and recovery

- Actual current-user unsigned NSIS install, full installed native/browser onboarding/workspace/settings/process-restart workflow and uninstall passed. Executable/uninstaller/registration removed, private synthetic catalog files preserved byte-for-byte. Actual screenshots inspected; no real user catalog was targeted.
- CI artifact 12055044353 contains only release setup, SHA256SUMS and two JSON evidence files. Downloaded ZIP digest, installer checksum and clean source identity were verified. CI's installed-release mode explicitly skips UI on elevated hosts; separate native UI checks and actual local downloaded-installer UI tests passed.
- 40 Node foundation/tool tests, 13 source-recovery checks, 26 Rust tests, 11 client/component tests and seven browser workflows passed. Native discovery/compile/link/run, formatting, Clippy, types, contract drift, frozen installation and docs/whitespace checks passed. Initial bundle-marker/overlapping-test failures were repaired and retained as history.
- Independent source bundle restored matching HEAD/tree; restored docs, frozen install, types, 11 client/component tests, 40 foundation/tool tests and 13 recovery checks passed. Source backups exclude private catalogs, credentials and installers. Master specification and all 90 section owners, 16 plans and 33 requirements are retained.

## Remaining scope and next gate

M1 is development-foundation certification only. AN-003 shared foundation design and AN-023 local onboarding/workspace/navigation/settings have verified workflow evidence. AN-001/002/018/020 retain release/security/later-platform scope and remain in progress. Signed installers, updater and clean-machine certification remain M15; no fully offline runtime installation claim. Existing WebView2 supports tested offline operation; its embedded bootstrapper needs internet if the runtime is absent.

M2.1–M2.5 acceptance gates passed within their recorded scopes. M2.6 reconciliation now records the supplied approval and concrete unresolved prerequisites. Next work: close G1 audit coverage in the M2.6 plan, then M2.7 implementation under the existing conditional authorization; M2.8 integrated certification remains required before M2 completion and integrated downstream certification; the user-authorized M3.1 independent slice is the explicit scheduling exception. Documentation checks do not verify encrypted product behavior.


## Historical M2.3 acceptance repair — 2026-10-18

The authorized audit repair is complete; ADR-0019 records the schema/history/membership decisions and [repair evidence](evidence/M2-3.md) records actual results and intermediate failures. The source checkpoint ledger is written after protected publication at `C:/Users/robin/Documents/ChatGPT/Anansi-source-recovery/M2-3-verified-b4a8e110d61042fbb11a3faf434bf9b4/source-verification.json` and contains the actual source HEAD/tree, bundle digest and independent restore results. Screenshots and machine verification records are retained alongside it. Source recovery excludes private application catalogs. The master specification and M0/M1 tags are unchanged. M2.4 is the next approved task and remains unstarted; M2 as a whole is incomplete.

## M2.4 local acceptance — 2026-10-18

User authorized continuation after M2.3. Work is on `codex/m2-4-jobs-export`; [M2.4 acceptance criteria and verification ledger](evidence/M2-4.md) were created before implementation. The previous verified checkpoint remains c141939 (production implementation 5b83338). M2.4 local acceptance now passed at 8f7ff87, superseding the earlier in-progress entry. Remote publication/source recovery are finalized through the ledger referenced above. M2.5 has not started.
