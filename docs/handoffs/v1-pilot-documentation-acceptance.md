# ANANSI v1 customer pilot documentation and acceptance pack —
acceptance criteria

Branch: `muse/v1-pilot-documentation-acceptance`, from
Windows reviewer tip
`08fc09540e97bd4efbca1dffb62eef91330ce864` (installer
qualification tooling review, 2026-11-12). Date:
2026-11-12. Acceptance criteria committed BEFORE
writing, per the standing rule and the repo AGENTS.md
execution policy. No new product features. No release
scripts, application code, README, or Antigravity-owned
files are edited.

## Verified inputs (recorded, not assumed)

- Review package
  `anansi-v1-installer-qualification-windows-review-2026-11-12.zip`
  (SHA-256
  `6e453584e7d23c6299077ee54a6316f4968ba3bfc55b2cb225cd810d26dd5ae0`,
  11,302 bytes): all 6 MANIFEST hashes verified;
  bundle head exactly reviewer tip `08fc0954`; bundle
  prerequisite exactly delivered tooling
  `7c3af5eb5575aca1388e3c924dbfe6fff2a52ffc`; the
  reviewer commit is docs-only on the delivered tip
  (BUILD_STATUS +4, review doc identical modulo
  CRLF). Windows Node checks **61/61**; serial
  release-tooling checks **54/54**; fixture
  generate/verify PASS; PowerShell syntax clean.
  Release build and installer/native qualification
  are **HELD by the preflight RAM gate**: final
  preflight exit 1, free RAM **3.07 GiB < 4 GiB**
  (first run 2.75 GiB). No code defect is reported
  and no repair is requested. The RAM threshold is
  NOT changed and no closed repair work is
  restarted.
- Authoritative docs read before writing: AGENTS.md,
  the pilot scope
  (`docs/handoffs/v1-pilot-scope.md`), the Windows
  handoff (`docs/handoffs/v1-pilot-windows-handoff.md`),
  the installer checklist
  (`docs/handoffs/v1-pilot-installer-checklist.md`),
  the qualification acceptance/audit, and the
  reviewer record
  (`docs/reviews/windows-v1-installer-qualification-review.md`).
- Every user-facing claim below was checked against
  current code before writing (UI labels, limits,
  messages, and behavior). The check log is
  summarized under "Claim verification" below; the
  documents themselves cite the behavior, not the
  check log.

## Acceptance criteria

- **AC-D1 (user guide):** a concise first-time-user
  guide covers installation, workspace/project
  creation, local-model setup, desktop API-key
  setup, document import, search, Ask/citations,
  and backup/restore. Steps use actual UI labels
  and the actual onboarding order. It expands the
  handoff §12 short guide without contradicting
  it, and points to the troubleshooting guide and
  limitations sheet instead of restating them.
- **AC-D2 (tester checklist):** a pilot tester
  checklist using synthetic data only, with clear
  steps, expected outcomes, evidence to capture,
  and PASS/FAIL/NOT TESTED fields per item.
  Browser, native desktop, and installer checks
  are separate sections. Native/installer items
  reuse the qualification checklist's steps by
  reference and start NOT TESTED; no native or
  installer item is marked passed from Linux or
  browser evidence.
- **AC-D3 (limitations/privacy):** a truthful
  limitations/privacy sheet explains plaintext
  storage/backups, provider disclosure, credential
  handling, unsigned-build status, supported
  formats, and actual per-kind limits, each
  verified against code (see below). OAuth
  ("Continue with ChatGPT") and generated-code
  execution are explicitly unavailable. No
  encryption, signing, or certification claim is
  made.
- **AC-D4 (bug-report template):** a template
  asking for exact source/build identity and
  installer hash (from `packaging.json` /
  `release-manifest.json`, never the `0.1.0`
  version alone), reproduction steps, expected
  vs actual behavior, sanitized logs/messages,
  severity, and environment. It never requests
  API keys, pairing tokens, or private documents,
  and warns against including them.
- **AC-D5 (troubleshooting):** a guide covering
  no provider, unreachable local runtime,
  invalid/oversized documents, queued/parked
  imports, cancellation, recovery, failed
  restore, and safe rollback, using actual UI
  labels and the existing recovery behavior
  (including the verbatim recovery diagnostics
  and the Retry recovery action). No invented
  log location, setting, or workaround is
  offered.
- **AC-D6 (release decision sheet):** a sheet
  listing the remaining native/installer gates
  with evidence fields, none marked passed.
  Pilot blockers are identified separately from
  deferred features and 2.0 plans. The RAM-held
  status is recorded exactly as reviewed, with
  the rerun condition (free RAM >= 4 GiB, then
  the unchanged preflight and the isolated
  build/native checklist).
- **AC-D7 (consistency):** new documents reuse
  existing documents by reference and do not
  create contradictory copies. Stale claims
  found in the Muse-authored scope/handoff docs
  are fixed narrowly in place, preserving
  historical evidence; proposed corrections to
  Antigravity-owned files (README, matrix,
  BUILD_STATUS) are listed separately and those
  files are NOT edited. Documentation
  consistency and links validate
  (`tests/docs-validation.test.mjs` /
  `scripts/validate-docs.mjs`); every link in
  the new documents resolves.
- **AC-D8 (delivery):** clean branch, exact
  baseline/tip, small reviewable commits, a
  self-contained full-history Git bundle ZIP
  with MANIFEST and SHA-256 sidecar, verification
  logs, independent scratch verification of
  extraction/hashes/bundle/exact fetched tip, and
  a credentials/private-data sweep. Boundaries:
  no inference, downloads, credentials,
  owner-data changes, installation, publication,
  pushes, promotion, or remote CI. No
  Windows/native acceptance is claimed from
  Linux. Work stops after delivery.

## Claim verification (code sources checked)

- Installer identity: `apps/desktop/src-tauri/
  tauri.conf.json` — NSIS-only, `currentUser`,
  English only, `allowDowngrades: false`,
  embedded WebView2 bootstrapper, publisher
  `Robillionair`, version `0.1.0`.
- Readiness panel: `packages/ui/src/App.tsx`
  ("Core connection": Application, Core version,
  Operating system, Core uptime, Contract
  version, Application version, API version).
  The panel is NOT on the Dashboard; the
  development-preview disclosure is the
  Workspace footer (`Workspace.tsx`). The
  handoff/scope shorthand is corrected under
  AC-D7.
- Onboarding: `packages/ui/src/Onboarding.tsx`
  (five intents, privacy review, skip-AI entry)
  and `Workspace.tsx` (Steps 1–4, name rule
  1–64 ASCII, "Create workspace").
- Navigation labels: `packages/ui/src/
  Navigation.tsx`. Local runtime discovery:
  `LocalModels.tsx` (Re-probe System, loopback
  endpoints only, no model download control).
- Connect AI: `packages/ui/src/Providers.tsx`
  (5 steps, credential setup, Generate
  reference, Discard draft, Unused credentials,
  browser refusal, Zero-Cost Lock default on,
  unknown-cost models treated as paid).
  Credentials: `crates/core/src/
  provider_credentials.rs` (CredUI generic /
  non-persistent / always-visible, keyring
  service `com.robillionair.anansi.providers`,
  reference-only config under
  `provider-config/` in the storage root).
- Documents: `packages/ui/src/documents.ts`,
  `documentInput.ts`, `Documents.tsx`, and
  `crates/core/src/document.rs` — per-kind
  limits: TXT/MD raw <= 65,536 bytes and text
  1–8,000 chars; PDF/DOCX raw <= 1,048,576
  bytes and extracted text 1–32,000 chars;
  strict UTF-8, BOM recorded, label <= 255
  chars basename-only, max 8 concurrent
  uploads, Sealed import refused, scanned PDFs
  need OCR (separate slice). The "1–65536
  bytes" shorthand in the scope/handoff is
  corrected under AC-D7.
- Search: `packages/ui/src/Search.tsx` (Title
  only default; Full-body; Hybrid semantic;
  bounds 256/512 bytes, 10,000 records, top
  100; Sealed excluded; no provider traffic
  for title/body search).
- Ask: `packages/ui/src/AskAnansi.tsx`
  (local loopback only; OFFLINE /
  INSUFFICIENTEVIDENCE / ANSWERED banners;
  disclosure manifest) and `crates/core/src/
  ask.rs` (4,000-char context budget).
- Jobs/recovery: `packages/ui/src/Jobs.tsx`,
  `JobRecovery.tsx`, `crates/core/src/jobs.rs`
  (six states; Pause/Resume/Cancel/Retry;
  limits 10,000 records, 256 MiB, 32 pending
  jobs; verbatim recovery diagnostics, attempts
  3/1, Retry recovery while Queued, in-memory
  exhaustion state).
- Backups: `packages/ui/src/Backups.tsx`
  (plaintext consent, 256 MiB saved-file cap,
  preview fields, fresh-ID restore) and the
  backup workflow helpers in `scripts/`.
- Forge inertness: `packages/ui/src/Forge.tsx`
  and `crates/core/src/forge.rs` (no build,
  test, preview, execution, or deployment;
  §32 sandbox open; no process execution in
  the Forge path).
- OAuth gate: `packages/ui/src/Providers.tsx`
  (`__ANANSI_CHATGPT_SIGNIN_DEV__` only; the
  pilot journey asserts zero "Continue with
  ChatGPT" controls in a normal build).
- Storage: `crates/core/src/readiness.rs`
  (`Core::for_host` default root:
  `%LOCALAPPDATA%\ANANSI\workspaces` on
  Windows); plaintext throughout, Sealed fails
  closed, no encryption claim.
- File dialog: no Tauri dialog plugin is
  configured for document import; import uses
  the in-app file control (basename only, no
  path stored). Whether the desktop build
  presents the native Windows dialog is
  UNVERIFIED and is recorded as an observation
  item, not a claim.

## Known limits carried into this checkpoint

- The installer/native gates stay HELD (RAM
  preflight) or open exactly as reviewed; this
  documentation pack closes none of them.
- Windows review remains outstanding for
  Connect AI setup + verification repair, M13
  PDF/DOCX ingestion, SOMNIA compaction slices,
  SwarmTank v2, and the ChatGPT sign-in spike
  (deferred, dev-gated). The documents state
  Linux-only evidence as Linux-only.
- Standing gates unchanged: G1 external audit,
  Authenticode signing, M7 native desktop
  acceptance + preview promotion, §32 execution
  sandbox, M5 production gates incl.
  live-provider verification, M8 Slice 4
  Windows verification, native credential-store/
  file-dialog acceptance, clean-machine
  installer qualification, SOMNIA AC-I10 live
  rerun (blocked).
