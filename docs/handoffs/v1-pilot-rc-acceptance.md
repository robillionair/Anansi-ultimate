# ANANSI v1 Windows pilot release candidate — acceptance criteria

Branch: `muse/v1-pilot-release-candidate`
Baseline: `dff9bb16688f3af075c395ad8fd5b179af9c3a9d`
(origin/master at fetch, 2026-11-12; also the tip of
`origin/codex/anansi-corporate-2-plan` and
`origin/codex/development-status-2026-11-12`)
Date: 2026-11-12. Acceptance criteria committed BEFORE
implementation, per the standing rule and the repo
AGENTS.md execution policy. No feature expansion: the
work is integration, evidence, repair of pilot-blocking
defects only, and a reviewable Windows handoff.

## Verified inputs (recorded, not assumed)

- Review package
  `wr-rec-witness-gate-windows-reviewed-2026-11-12.zip`
  (SHA-256 `9c951a5b…`, 15,111 bytes): all 7 manifest
  hashes verified; bundle head exactly reviewer tip
  `8453f649e385472e86055bd8f8a55abdad8c009f`; runtime
  exactly `4b6b75938fb625659e892fdc108151857a824943`;
  incoming ZIP hash in the review matches the delivered
  ZIP (`23f89aef…`); reviewer commit is docs-only on the
  runtime (BUILD_STATUS +4, review doc). Windows
  aggregate **416/416 in one default-concurrency
  invocation** (API 4, core 348, desktop 1, providers
  63); probes 2/2; queue/mirror/gate 24/24; witness
  test 3×1/1; docs 27/27. Verdict: PASS in focused
  Windows scope; WR-REC-WITNESS-01 closed in that
  scope. The recovery/witness repair loop is NOT
  restarted.
- Baseline inspection: `dff9bb1` is exactly one
  docs/plans commit on `01ee5a74` (README +2,
  BUILD_STATUS +4, REQUIREMENTS_MATRIX +1, ROADMAP,
  new `plans/post-v1-anansi-corporate.md`, enterprise/
  nanzi/provider-plan touches; +221/−2, no code). The
  repair lineage `01ee5a74..8453f649` (13 commits) is
  NOT in master and must be integrated. All lineage
  tips for the chained slices (Connect AI, DOCX, PDF,
  SwarmTank v2, SOMNIA expected-target, MCP Slice 5,
  SQLite repair, ingestion reliability, ChatGPT spike)
  are ancestors of `dff9bb1` — verified, not assumed.
- Authoritative docs read before implementation:
  AGENTS.md, REQUIREMENTS_MATRIX.md, ARCHITECTURE.md,
  README.md, DECISIONS.md, RECOVERY.md,
  plans/10-release.md, plans/v1-provider-onboarding.md,
  plus the release tooling (see AC-V1-06). MASTER_SPEC
  and BUILD_STATUS are covered by a cited digest that
  the scope document verifies claim by claim.

## Acceptance criteria

- **AC-V1-01 (integration):** the branch integrates
  the accepted repair/review history
  (`01ee5a74..8453f649`, including the reviewer
  acceptance commit with its exact hash) onto baseline
  `dff9bb1`, preserving the newer README, Corporate
  2.0 plans, and all other accepted changes. No older
  baseline is substituted silently; any unavailable
  required object is identified precisely (none is
  expected — all objects verified present). The merge
  result is verified both ways: diff vs `8453f649`
  shows exactly the `dff9bb1` changes; diff vs
  `dff9bb1` shows exactly the repair lineage.
- **AC-V1-02 (pilot scope):** an evidence-backed v1
  pilot scope document identifies workflows actually
  ready for pilot use, and separately lists incomplete
  requirements, known defects, optional features, and
  deferred 2.0 work. Contradictory status claims
  (including stale README/BUILD_STATUS/matrix wording)
  are resolved using code and recorded evidence, with
  citations. No invented completion percentages; no
  entire milestone marked complete from isolated
  tests. Antigravity-owned status documents are NOT
  edited; proposed status updates go in the handoff
  only.
- **AC-V1-03 (boundaries):** ChatGPT OAuth stays
  disabled and deferred (dev-gated prototype only,
  verified in code); generated-code execution stays
  unavailable (Forge exports inert, verified in code);
  Corporate, Nanzi, and other 2.0 work remain
  planning only (no code, no claims). No paid/cloud
  inference, no credentials, no owner-data changes,
  no installer promotion, no pushes or publication,
  no modification of Antigravity's worktree.
- **AC-V1-04 (journey):** the first-time-user journey
  is walked end to end with synthetic data only:
  fresh start → workspace/project → local or API-key
  provider setup → document import → search → Ask
  with citations → restart → backup → fresh restore
  → reopen. Empty states, understandable errors,
  cancellation, missing provider, unsupported files,
  and recovery are checked. Reproducible
  pilot-blocking defects found on this walk are
  repaired within the workflow, each with a focused
  regression and before/after evidence. No defect is
  repaired by weakening a test; no unrelated feature
  is added.
- **AC-V1-05 (release handoff):** a Windows release
  handoff names the exact source commit and
  reproducible build commands, required toolchain and
  prerequisites, isolated build/data locations under
  `E:\Anansi`, version/build identification visible
  to the tester (including its limits), a native
  credential-store and file-dialog acceptance
  checklist, installation / upgrade / restart /
  backup-restore / uninstall checklists, known
  limitations, unsigned-installer status, rollback
  instructions, and a short customer pilot guide.
  Existing release tooling is reused after inspection
  (installer build + checksums, installer
  verification, native candidate acceptance); no
  parallel tooling is created. No Windows, native, or
  installer acceptance is claimed from Linux.
- **AC-V1-06 (verification):** relevant Linux checks
  run sequentially where contention matters, with
  actual results recorded including failures and
  environmental exclusions. Unchanged full batteries
  are not repeated unnecessarily. No benchmark or
  marketing claims are made.
- **AC-V1-07 (delivery):** one reviewable checkpoint:
  clean branch, exact baseline/tip, change summary,
  updated status and pilot checklist, a full-history
  Git bundle in a checksummed ZIP sufficient for an
  independent scratch fetch without missing
  prerequisites, a MANIFEST with file hashes, raw
  verification logs, and reproduction commands.
  Extraction, hashes, bundle, ancestry, and the exact
  fetched tip are verified. The package is swept for
  credentials and private data. The delivery lists
  what the Windows reviewer must do next. Work stops
  after delivery; no new feature milestone begins.

## Known limits carried into this checkpoint

- The in-app readiness panel shows version 0.1.0 /
  contract 1 / OS / uptime; no git commit hash is
  embedded in the build. Build identity for the
  installer comes from `packaging.json` (git commit,
  dirty flag, hashes). Embedding the commit would be
  a feature change and is out of scope; the handoff
  records the limit and the workaround.
- The native candidate acceptance script hardcodes
  one older candidate's paths; the handoff instructs
  adapting its three path constants rather than
  creating parallel tooling.
- Standing gates stay open and are not closed by this
  checkpoint: G1 external audit, Authenticode
  signing, M7 native desktop acceptance + preview
  promotion, §32 execution sandbox, M5 production
  gates incl. live-provider verification, M8 Slice 4
  Windows verification (if commissioned), native
  credential-store/file-dialog acceptance, clean-
  machine installer qualification, and the SOMNIA
  AC-I10 live rerun (blocked).
