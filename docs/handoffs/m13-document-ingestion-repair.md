# M13 Document Ingestion Slice 1 — Windows Findings Repair

Branch: `muse/m13-document-ingestion-repair`, from Slice 1 tip
`ba3f5a724e4c56c575fae2563cb93d073fdb5cbe`.

## 1. Review evidence verified

Readiness package `anansi-windows-readiness-closure-2026-11-05.zip`
(SHA-256 `11c9a91a43bfd4ca466a75249ec81cff06b51078a1b0ce1f66ec424975bb40cf`,
8,818,842 bytes) — all five MANIFEST file hashes verified OK, bundle
hash matches MANIFEST (`6700e40e…`).

**Manifest discrepancy, confirmed:** the MANIFEST `tip` field reads
`5bb5a7cbf5a1768686940656cbdf7ef961e6191c`, but the bundle's actual head
is `5bb5a7cbf678f62818becfe1bd9de273be084b03`
(`docs(review): record independent Windows review of M13 Document
Ingestion Slice 1`, branch `codex/windows-readiness-closure`, baseline
`7e24859e` an ancestor). The review document itself is
`windows-m13-ingestion-slice1-review.md` (SHA-256 `efc05bf4…`), read in
full. Its verdict: Slice 1 is architecturally sound and bounded;
7/9 Playwright tests pass on Windows; two findings, both reproduced
with exact root causes. The pre-existing `dashboard.spec.ts` quick
actions failure (LARP HUD overlay) is confirmed again on the baseline.

Integration with the Windows readiness branch (`codex/windows-readiness-closure`)
is left to Antigravity; this repair branch contains only the two fixes
below, on top of the Slice 1 tip.

## 2. Finding 1 — restart test not portable (`ss` + missing `.exe`)

Review: `documents-ingestion.spec.ts` lines 338/344 — `ss -ltnp` does not
exist on Windows, and the manual respawn used
`target/debug/anansi-api` without `.exe`.

Fix: the restart is now a harness operation. `startSession` in
`tests/e2e/collider-three-helpers.ts` exposes `restartApi()`, which
kills the API child process the harness already owns, confirms it
exits (10 s failure budget, no silent hang), then re-runs the existing
platform-correct `startApi()` against the same storage and pairing
roots. `session.call` reads the closure `pairing`, so it re-points at
the fresh API automatically; `pairingToken` is now a getter for the
same reason (the MCP specs read it, and a snapshot would go stale
after a restart).

The spec test keeps every assertion (interrupted upload resumes from
offset 0 and publishes exactly one document with `document.fileName`
`resumed.txt`) and drops ~54 lines of spec-side process management:
no `ss`, no `execSync`, no manual `spawn`, no `callFresh`, no
pairing-file parsing. The review's literal recommendation (expose
`apiPid`/`killApi` and respawn in the spec) is superseded by this
shape: the harness owns the child and its pairing, so the restart
belongs there — the remediation (no `ss`, platform-correct executable)
is identical.

## 3. Finding 2 — SQLite contention: sweep vs concurrent `ingest-begin`

Review: test 6 (concurrent duplicate imports) failed on Windows with
`ingest-begin` returning 503 — `SWEEP ERROR in enqueue_ingestion:
StorageUnavailable`. Root cause: `sweep_staging` ran *before* the
Immediate transaction, opening a *separate* connection per staged file
and reading `jobs` while a concurrent begin held an uncommitted write
transaction. On Windows NTFS that cross-connection read fails; on
Linux the same race was the unreproduced flake in the Slice 1 addendum
(§3.6). A second hazard in the old ordering: the staged file was
created *before* the job row, so a concurrent sweep could see a file
without its row and delete a live upload as an orphan.

Fix (`crates/core/src/ingestion.rs`, `enqueue_ingestion` +
`sweep_staging` only):

- The sweep decision now runs **inside** the Immediate transaction, on
  the same connection as the insert. No cross-connection reads, no
  contention; concurrent begins serialize on the transaction.
- The sweep returns stale paths; the caller deletes them **after** its
  transaction commits, so a file is never removed while its job row is
  still uncommitted.
- The staged file is created **after** the job row commits (file-after-
  row, mirroring the backup upload shape). A file never exists without
  its row, so a concurrent sweep can never mistake a live upload for
  an orphan.
- If staged-file creation fails after commit, the job is failed
  honestly (`fail_paused_ingestion`: state Failed, checkpoint
  `discarded`, log "Upload staging could not be created.") instead of
  sitting Paused with no staging, and the error surfaces.
- Genuine storage failures still surface: the sweep propagates its
  directory and job-lookup errors (a directory-list failure is no
  longer swallowed), and the enqueue path keeps returning
  `StorageUnavailable` for real store failures. Only a single
  unreadable directory *entry* is skipped, and its file waits for the
  next sweep.

New Rust tests (ingestion suite 11 → 14):

- `sweep_preserves_active_uploads_and_removes_stale_staging` — an
  active Paused upload's file survives the sweep while a completed
  job's planted staging and an ownerless `.upload` file are removed.
- `enqueue_surfaces_genuine_storage_failure` — a regular file planted
  where the workspace export directory belongs makes `ingest-begin`
  return `StorageUnavailable` and creates no job.
- `concurrent_begin_calls_all_succeed_and_publish_once` — eight
  concurrent `ingest-begin` calls for the identical file on shared
  `&Core` all succeed, all eight jobs complete, all share one
  `artifactId`, exactly one document is published.

## 4. Verification evidence (Linux, this repair)

- `cargo test --workspace --locked --lib`: **265/265 PASS** (262 + 3 new).
- Full workspace suite: **666 passed, 4 failed** — the 4 are exactly the
  pre-existing `semantic_workflow` live-Ollama environmental failures;
  nothing new.
- Ingestion unit tests: **14/14 PASS**.
- `npm run typecheck`: **0 errors**.
- `node scripts/contracts.mjs --check`: **0 drift** (no contract change).
- `node scripts/validate-docs.mjs`: **290 PASS** (289 + this file).
- UI suite: **320/320 PASS**.
- API build clean (1 pre-existing Windows-only warning,
  `preserved_env_entries` at `crates/core/src/mcp.rs:715`); Vite build clean.
- `tests/e2e/documents-ingestion.spec.ts`: **9/9 PASS** (30.5 s).
  Test 6 (concurrent duplicates) passes in 3.0 s; test 7 (restart)
  passes in 1.9 s via the harness restart.
- Full Playwright regression battery (25 specs): **67 passed, 1 failed**
  — the single failure is the pre-existing `dashboard.spec.ts:47`
  quick-actions failure, identical on the pristine baseline (LARP HUD
  overlay covers the Quick actions after the LARP toggle). All MCP
  specs pass, including the three that read `session.pairingToken`
  through the new getter.

Pre-existing, unchanged: the 4 `semantic_workflow` live-Ollama
environmental failures in the full workspace suite, the dashboard
quick-actions failure (identical on the pristine baseline), and the 4
pre-existing strict-Clippy lints in `mcp.rs`/`mcp_server.rs` (this
repair adds none; `cargo fmt --check` still fails on the pristine
baseline under this toolchain — no bulk reformat was run).

## 5. Status and next steps

- **Both Windows findings remain OPEN until the independent Windows
  rerun passes.** This repair changes the code and the tests; only
  Antigravity's rerun on Windows closes them.
- **PDF ingestion comes after that rerun**, per owner instruction. No
  PDF work is started here.
- Proposed global status updates for Antigravity (not applied here):
  none beyond the Slice 1 proposals already in
  `docs/handoffs/m13-document-ingestion-addendum.md` §7 (AN-025 stays
  `in progress`; §12 stays `planned` with the TXT/MD foundation noted).
  This repair does not change any requirement's status.

## 6. Files changed

- `crates/core/src/ingestion.rs` — sweep/transaction/file ordering,
  `fail_paused_ingestion`, 3 new tests.
- `tests/e2e/collider-three-helpers.ts` — `Session.restartApi`,
  `pairingToken` getter.
- `tests/e2e/documents-ingestion.spec.ts` — test 7 rewritten on
  `restartApi` (assertions preserved); unused `spawn`/`execSync`
  imports removed.
- `docs/handoffs/m13-document-ingestion-repair.md` — this file.
