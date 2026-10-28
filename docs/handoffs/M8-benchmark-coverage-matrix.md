# Coverage matrix — independent verification (Linux)

Branch: `muse/m8-benchmark-tokens` (benchmark work on reviewed baseline
`df8072595e672ead29ad0df82393c31de7b6f7d8`; D1 repair `e8d7f6cd` included).
Inventory sources: `docs/REQUIREMENTS_MATRIX.md` (AN-001–AN-033),
`docs/MASTER_SPEC.md` (§§1–82), `docs/BUILD_STATUS.md`, `plans/02`–`06`,
the Slice 3 addendum, and the test trees. Classifications reflect code +
tests, not matrix labels alone.

**Matrix staleness warning:** AN-007 (AI Matrix), AN-008 (OAuth/API keys)
and AN-026 (Zero-Cost Lock/routing/budgets/disclosure) are still marked
`planned` in the requirements matrix, yet the M5 substrate, routing,
Zero-Cost Lock gates and disclosure are implemented and tested. Any
coverage claim built from matrix status alone would under-report.
Reconcile the matrix before publishing coverage claims.

## Test layout

- `crates/core/tests/` — 44 integration files + `benchmark_tokens.rs`
  (measurement harness, not a product test) + `fixtures/`.
- `crates/providers/tests/` — 8 files (`synthetic_scenarios` 26,
  `retry_safety` 10, `https_scenarios` 9, `wire_binding` 5, `deadline` 4,
  `accounting` 3, `idle_stream_cancel` 2, `full_workflow` 1).
- `crates/api/tests/` — 11 files (tokio tests driving the Axum router).
- `packages/ui/tests/` — 7 files + `tests/contracts.test.ts`.
- `tests/e2e/` — 28 specs: **canonical 17** (`app_checkpoint`,
  `artifacts`, `backups`, `evaluations`, `forge`, `forge-deliverable`,
  `forge-review`, `forge-revision`, `ideas`, `opportunities`, `providers`,
  `vault`, `collider`, `transfer`, `transfer-identity`,
  `transfer-concurrency`, `collider-memory-context`) and **non-canonical
  11** (`ask_anansi`, `entities`, `export`, `integrated_workflow`,
  `memory`, `memory-lifecycle`, `onboarding`, `readiness`, `search`,
  `workspace`, `collider-memory-acceptance`). The last is the Windows
  reviewer's consent/provenance probe; it is NOT silently counted in
  the canonical 17 on Linux. A "browser 17/17" claim must not be
  presented as "all 28 specs."

## Matrix by area

| Area | Req/plan | Implementation | Tests (backend / browser) | Linux result |
|---|---|---|---|---|
| Memory / history / isolation | AN-004, AN-024, AN-031 (in progress); §10 | `memory.rs`, `lifecycle.rs`, `storage.rs`, `content.rs`, `privacy.rs`, `workspace.rs`, `entities.rs`; schema v10 | `memory_roundtrip` 9, `memory_lifecycle` 6, `workspace_roundtrip` 11, `entities_roundtrip` 6, vault-search suites; API `memories`/`workspaces`/`entities`; browser memory specs non-canonical, `artifacts` canonical | Automated passed except 1 intermittent: `memory_roundtrip` 9/9, `memory_lifecycle` 6/6, `workspace_roundtrip` 9/10 in clean run (concurrent-migration failure, 10/10 alone), `entities_roundtrip` 6/6; browser memory, memory-lifecycle, entities, artifacts pass; real-model workflow pending |
| Lexical retrieval | AN-005 (verified); §11 | `search.rs` (FTS5), `hybrid.rs` lexical path + RRF | `body_search` 1, `body_search_limits` 1, `title_search` 1, `title_search_limits` 1, `hybrid_retrieval` 6, `retrieval_evaluation` 1; browser `search` non-canonical | Automated passed: `body_search` 1/1, `body_search_limits` 1/1, `title_search` 1/1, `title_search_limits` 1/1, `hybrid_retrieval` 6/6, `retrieval_evaluation` 1/1; browser search local-title test passes |
| Semantic retrieval | AN-005 (verified); M4.1–M4.3 | `semantic.rs`, `vector.rs` | `vector_embeddings` 5, `vector_search` 5, `embedding_invalidation` 1, `hybrid_retrieval` 6; `semantic_workflow` 4 environment-gated (live embedding via default endpoint) | Automated passed with live model: `vector_embeddings` 5/5, `vector_search` 5/5, `embedding_invalidation` 1/1, `hybrid_retrieval` 6/6, `semantic_workflow` 4/4 with `all-minilm:22m` installed (0/4 before install, reported separately); browser semantic UI workflow passes |
| Context compilation | AN-010 (verified); AN-006 (in progress); §§13, 19 | `context.rs` (1000-char chunks, budget default 4000 / clamp 500–32000, dedup, Sealed exclusion), `ask.rs` (budget `min(max_items*800, 4000)`), `ask_stream.rs` | `context_compiler` 7, `ask_anansi` 5, `ask_streaming` 1, `ai_discovery` 7; browser `ask_anansi` non-canonical; `benchmark_tokens` harness new | Automated passed: `context_compiler` 7/7, `ask_anansi` 5/5, `ask_streaming` 1/1, `ai_discovery` 7/7, `benchmark_tokens` 1/1; browser ask_anansi synthetic passes; real-model quality pending |
| Provider policies / accounting | AN-026, AN-007, AN-008 (matrix stale); §§14–17, 19, 67 | `provider_dispatch.rs` (permits, budgets, ledger, disclosure), `providers/` (routing, dispatcher, gates, adapters) | `provider_dispatch` 18; providers suites incl. `accounting` 3; API `provider_boundary`; browser canonical `providers` | Automated passed: `provider_dispatch` 18/18, providers+API 144/0 overall; browser providers canonical passes; D1 disclosure regression passes in harness |
| Cancellation | AN-029 (in progress); §66 | `jobs.rs`, `ask_stream.rs`, stream-run cancellation, collider cancel | `jobs_recovery` 7 (crash-root phase env-gated), `ask_streaming` 1, collider/provider cancel tests; providers `deadline` 4, `idle_stream_cancel` 2; backend-only dedicated coverage | Automated passed: `jobs_recovery` 7/7 (env-gated phases skipped as documented), `ask_streaming` 1/1, providers+API 144/0 overall; browser providers + collider cancel paths pass |
| Artifacts | AN-024 (in progress); §79 | `artifacts.rs` (registry, versioning, `source-review` type, transfer-approval receipt), `export.rs` | `artifacts_registry` 6, `export_roundtrip` 3, `backup_artifacts` 5; API `artifacts`; browser canonical `artifacts`, non-canonical `export` | Automated passed: `artifacts_registry` 6/6, `export_roundtrip` 3/3, `backup_artifacts` 5/5; browser artifacts canonical + export non-canonical pass |
| Ideas | AN-027 (in progress, M6 Slices 2–4); §22 | `ideas.rs` (project-scoped generation on M5) | `ideas_generation` 7, `ideas_validation` 4; API `ideas`; browser canonical `ideas`; UI `idea-run` | Automated passed: `ideas_generation` 7/7, `ideas_validation` 4/4; browser ideas canonical + integrated_workflow pass; UI idea-run in node 81/81 |
| Evaluations | AN-027 (M6 Slice 3); §78 | `evaluations.rs` | `evaluations` 3; API `evaluations`; browser canonical `evaluations` | Automated passed: `evaluations` 3/3; browser evaluations canonical + integrated_workflow pass |
| Opportunities | AN-027 (M6 Slice 4); §21 | `opportunities.rs` | `opportunities_generation` 6, `opportunities_restore` 3; browser canonical `opportunities`; UI `opportunity-run` | Automated passed: `opportunities_generation` 6/6, `opportunities_restore` 3/3; browser opportunities canonical passes; UI opportunity-run in node 81/81 |
| Collider | AN-013 (in progress, M8 Slices 1–3 + repair); §§23–27 (except §26 3D deferred) | `collider.rs` (beams, controls, selected-memory segments, six-section validation, save/restore), `Collider.tsx` | `collider` 19; browser canonical `collider`, `collider-memory-context`, non-canonical `collider-memory-acceptance`; UI `collider`, `collider-run` | Automated passed: `collider` 19/19; browser collider + collider-memory-context canonical + acceptance probes 2/2 pass; UI collider/collider-run in node 81/81; real-model collider pending |
| Approved Forge transfers | AN-013 (M8 Slice 2 + repair); AN-012 | `transfer.rs` (model-free transfer, receipt v1, transactional idempotency), `restore.rs` fix-ups | `transfer` 18; browser canonical `transfer`, `transfer-identity`, `transfer-concurrency`; UI `transfer` | Automated passed: `transfer` 18/18; browser transfer + transfer-identity + transfer-concurrency canonical pass; UI transfer in node 81/81 |
| Specifications | AN-012 (in progress, M7 Slice 1); §§28–30 | `forge.rs` (spec-start/poll/cancel/save/approve, approval gating) | `forge_generation` 5, `forge_restore` 4; browser canonical `forge`; UI `forge-run` | Automated passed: `forge_generation` 5/5, `forge_restore` 4/4; browser forge canonical passes; UI forge-run in node 81/81 |
| Source revision | AN-012 (M7 Slice 2); §§28–30 | `forge.rs` (package diff, revise ops, exact parent/spec references) | `forge_revision` 5, `forge_restore` 4; browser canonical `forge-revision` | Automated passed: `forge_revision` 5/5, `forge_restore` 4/4; browser forge-revision canonical passes |
| Source review | AN-012 (M7 Slice 4); §§28–30 | `forge.rs` (source-review v1 validation/save/restore remapping), `artifacts.rs` type | `forge_review` 11; browser canonical `forge-review` | Automated passed: `forge_review` 11/11; browser forge-review canonical passes |
| Deliverables | AN-012 (M7 Slice 3 + repair); §33 | `forge.rs` (five fixed documents, unexecuted status, export, remap), `export.rs`, `restore.rs` (record/binding validation) | `forge_deliverable` 9, `forge_restore` 4; browser canonical `forge-deliverable` | Automated passed: `forge_deliverable` 9/9, `forge_restore` 4/4; browser forge-deliverable canonical passes |
| Backup / fresh restore | AN-019 (in progress); §76 | `backup.rs`, `backup_commands.rs`, `backup_upload.rs`, `restore.rs` (fail-closed pre-validation, remapping, fix-ups), `snapshots.rs` | `backup_restore` 7, `backup_artifacts` 5, `forge_restore` 4, `opportunities_restore` 3, restore paths in collider/transfer; API `backups`; browser canonical `backups` | Automated passed: `backup_restore` 7/7, `backup_artifacts` 5/5, `forge_restore` 4/4, `opportunities_restore` 3/3; browser backups canonical + collider/transfer restore paths pass |

## Gating and skip rules (report as skipped/env, never as passed)

- `semantic_workflow` (4 tests): live embedding via the default endpoint
  (`http://127.0.0.1:11434`) with the real installed `all-minilm:22m`
  model (`DEFAULT_EMBEDDING_MODEL`). Without that model they fail on
  environment, not on product behavior. Rerun after installing the
  model and report before/after separately.
- Env-gated two-phase tests that return early without variables set:
  `ANANSI_JOB_CRASH_ROOT` (`jobs_recovery.rs`), `ANANSI_PROVIDER_RESTART_*`
  (`provider_dispatch.rs`), `ANANSI_VAULT_EPOCH_*` / `ANANSI_VAULT_TEST_*`
  (`vault_increment2.rs`). A plain run silently skips them.
- No dedicated test files found for `settings.rs` and
  `provider_credentials.rs` (coverage via other suites plausible but
  unverified); several `src/` modules have no in-file test module but
  are covered by integration tests — that alone is not a gap.

## Explicitly not implemented / deferred (never counted as verified)

- §32 execution sandbox; build/test/preview/deployment of generated
  products (execution disabled by design; generated products inert).
- Collider §26 3D, URL fetching, document ingestion, saved-project
  selection; SwarmTank 3D (AN-014), SOMNIA (AN-015), GHOST-PROMPT
  (AN-016), MCP (AN-017), Launch Studio (AN-028), document formats/OCR/
  async ingestion (AN-025), native macOS/Apple Silicon + signed updates
  (AN-030), enterprise architecture (AN-021), Windows NSIS signed
  release (AN-001, clean-machine release still M15).
- OAuth-first setup (AN-008): local credential storage exists; OAuth
  flows unverified. Encrypted/sealed backup, G1 external audit, remote
  CI, M5 production gates.

## Cannot be certified on Linux (explicit limitations)

Windows native dialogs (CredUI) and OS credential storage; native
desktop acceptance (Tauri/WebView2, installer, launch, persistence,
uninstall); external cryptographic audit (G1); real cloud providers
(live behavior, billing, rate limits); remote CI; live-provider
certification generally; preview promotion / production readiness.
