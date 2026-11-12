# ANANSI v1 pilot documentation and acceptance pack

Customer pilot documentation for the ANANSI v1
Windows pilot. Documentation only: no product code,
no release scripts, and no Antigravity-owned files
changed in this pack. The pack closes no
verification gate; it exists so testers, reviewers,
and the pilot organizer work from checked facts.

Branch: `muse/v1-pilot-documentation-acceptance`,
from Windows reviewer tip
`08fc09540e97bd4efbca1dffb62eef91330ce864`
(installer qualification tooling review,
2026-11-12). Verify the exact tip from the delivery
MANIFEST after fetching; the generic app version
`0.1.0` never identifies a build.

Acceptance criteria for this pack (committed before
writing): see
[v1 pilot documentation acceptance](../handoffs/v1-pilot-documentation-acceptance.md).

## Contents

- [First-time-user guide](user-guide.md) —
  installation, workspace/project creation,
  local-model and desktop API-key setup, document
  import, search, Ask with citations, backup and
  restore.
- [Tester checklist](tester-checklist.md) —
  synthetic-data checks with expected outcomes,
  evidence to capture, and PASS/FAIL/NOT TESTED
  fields. Browser, native desktop, and installer
  checks are separate sections.
- [Limitations and privacy sheet](limitations-and-privacy.md) —
  plaintext storage/backups, provider disclosure,
  credential handling, unsigned-build status,
  supported formats and actual limits. OAuth and
  generated-code execution explicitly unavailable.
- [Bug-report template](bug-report-template.md) —
  exact build identity, reproduction steps,
  expected/actual behavior, sanitized evidence,
  severity, environment. Never requests keys or
  private documents.
- [Troubleshooting guide](troubleshooting.md) —
  no provider, unreachable runtime, invalid/
  oversized documents, queued imports,
  cancellation, recovery, failed restore, safe
  rollback. Actual UI labels and recovery
  behavior throughout.
- [Release decision sheet](release-decision-sheet.md) —
  remaining native/installer gates with evidence
  fields (none marked passed), pilot blockers
  separated from deferred features and 2.0 plans.
- [Proposed global corrections](proposed-global-corrections.md) —
  corrections proposed to Antigravity for README,
  the requirements matrix, and BUILD_STATUS. Not
  applied here.

## How these documents relate to existing ones

- The [pilot scope](../handoffs/v1-pilot-scope.md)
  is the evidence-backed statement of what is
  ready for pilot use. These documents follow it
  and do not restate its verdicts.
- The [Windows release handoff](../handoffs/v1-pilot-windows-handoff.md)
  is the reviewer's build and acceptance
  instruction. The user guide expands its short
  customer guide (§12); the decision sheet tracks
  its open gates (§§5–9, 13).
- The [installer qualification checklist](../handoffs/v1-pilot-installer-checklist.md)
  is authoritative for reviewer-executed installer
  and native steps. The tester checklist references
  its steps rather than copying them.
- Where the scope or handoff contained stale
  shorthand (per-kind document limits, the
  readiness-panel location, one journey count),
  the Muse-authored text was corrected narrowly in
  this same pack. Historical evidence was
  preserved; nothing was rewritten to change a
  recorded result.

## Status at issue (2026-11-12)

- Windows tooling checks passed on the delivered
  qualification tip: Node 61/61, release tooling
  54/54 (reviewer `08fc0954`).
- Release build and installer/native qualification
  are HELD by the preflight RAM gate (free RAM
  3.07 GiB < 4 GiB). No code defect reported, no
  repair requested, threshold unchanged. See the
  decision sheet for the rerun condition.
- The browser journey passed 1/1 on the v1 RC
  runtime (reviewer `57029b2e`); the recovery
  aggregate 416/416 belongs to reviewer `8453f649`
  in focused scope. Neither is installer or native
  acceptance.
- Windows review remains outstanding for Connect
  AI setup + verification repair, M13 PDF/DOCX
  ingestion, SOMNIA compaction slices, SwarmTank
  v2, and the ChatGPT sign-in spike (deferred,
  dev-gated). The documents state Linux-only
  evidence as Linux-only throughout.
