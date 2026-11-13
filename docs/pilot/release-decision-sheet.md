# ANANSI v1 pilot — release decision sheet

This sheet records what must still be evidenced
before controlled customer testing, and what must
not be claimed until then. The 2026-11-13 build and
native reviews below close only their named technical
checks; the overall pilot decision remains NOT DECIDED.
A gate closes only on the evidence listed
for it, on the exact build named in the decision
record. Linux evidence never closes a Windows gate;
browser evidence never closes a native or installer
gate.

The pilot documentation pack (user guide, tester
checklist, limitations sheet, bug template,
troubleshooting guide, this sheet) closes no gate
by itself. It exists so testers and reviewers work
from checked facts.

## Decision record (complete at decision time)

- Decision: [NOT DECIDED / APPROVED FOR CONTROLLED
  PILOT WITH CONDITIONS / NOT APPROVED]
- Date:
- Decided by:
- Build under decision — source commit
  (`packaging.json` `gitCommit`):
- Installer SHA-256:
- Application SHA-256:
- Config SHA-256:
- Toolchain (from `release-manifest.json` /
  `packaging.json`):
- Pilot scope approved (workflows included /
  explicitly excluded):
- Conditions attached:
- Open Blocker-severity bugs at decision (must be
  zero for approval):

## Evidence already held (scoped)

These results belong to the exact commits named.
Later code or documentation changes do not automatically
inherit them. The earlier library/browser results do
not establish installer acceptance; later native and
installed-workflow reviews have their own scope.

- **2026-11-13 current build:** source `6f216e3`, clean;
  preflight PASS at 7.13 GiB free RAM, release builds and
  artifact hashes verified. [Build review](../reviews/windows-v1-pilot-release-build.md).
- **Strengthened native smoke:** seven checks passed
  using harness `b0249927` against that exact app;
  NQ-01 closed. [Native review](../reviews/windows-nq01-native-harness-review.md).
- **Installed workflow:** same-host isolated install,
  installed native/browser UI and uninstall/catalog
  preservation passed after a selector-only harness
  repair. Clean-VM and upgrade are still open.
  [Installer review](../reviews/windows-v1-installer-workflow-qualification.md).
- **Actual document picker:** eight observed cases
  passed, four imports persisted with verified hashes.
  [Picker review](../reviews/windows-native-document-picker.md).

- Recovery aggregate **416/416** in one
  default-concurrency invocation, reviewer
  `8453f649` on runtime `4b6b75938`
  (2026-11-12). Focused Windows scope. The
  recovery/witness repair loop is closed and is
  not restarted.
- First-time-user browser journey **1/1 in 2.4
  minutes** on runtime `8931d34b`, reviewer
  `57029b2e` (2026-11-12): restart, backup, fresh
  restore, and reopen, synthetic data, zero
  external requests. Browser journey only.
- Installer-qualification tooling on delivered
  tip `7c3af5e`, reviewer `08fc0954`
  (2026-11-12): Node checks **61/61**, serial
  release-tooling checks **54/54**, fixture
  generate/verify PASS, PowerShell syntax clean.
  Tooling checks only; no installer was built.
- Linux evidence for the v1 RC (docs 21/21, tsc
  0, contracts 0 drift, UI 347/347, Rust lib
  417/417, journey 1/1 across three green runs)
  is recorded in the Windows handoff. Linux-only
  slices (Connect AI, M13 PDF/DOCX, SOMNIA
  compaction, SwarmTank v2, ChatGPT spike) stay
  Linux-only until Windows review.

## Gates that must close before any customer testing

Each gate needs its evidence field completed with
verbatim results on the decision build. "Blocked
by" means the gate cannot even start until the
earlier gate's evidence exists.

| Gate | Status | Evidence required | Evidence (complete at review) |
| --- | --- | --- | --- |
| G1. RAM preflight + isolated build | PASS on source 6f216e3 | Unchanged preflight and isolated build, clean source and cross-checked identity tuple. Threshold unchanged. | [Build review](../reviews/windows-v1-pilot-release-build.md): 7.13 GiB free RAM; all build steps exit 0, seven artifact hashes/sizes verified. |
| G2. Installer qualification script | PARTIAL: same-host PASS; clean VM OPEN | Full script without SkipUi on a clean machine/VM, including installed UI, uninstall, and preserved data. | [Same-host run](../reviews/windows-v1-installer-workflow-qualification.md) and [record](../evidence/windows-pilot-2026-11-13/installer-verification.json). Fresh isolated target and clean app registration; not clean OS qualification. |
| G3. Native candidate acceptance | PASS, automated scope only | All seven checks pass with explicit locations and expected hash. Synthetic vault provider is not credential-store evidence. | [Strengthened native review](../reviews/windows-nq01-native-harness-review.md), actual readiness, artifact readback/isolation and post-restart persistence. |
| G4. Fresh install + SmartScreen | OPEN | Manual double-click install on a clean machine/VM; exact SmartScreen behavior and default per-user installation observed. | Not tested; silent same-host installation does not satisfy this. |
| G5. Restart, backup, fresh restore (native) | PARTIAL | Native restart, saved backup, import, preview, fresh-ID restore and reopened search. | Native backup/restart passed; [browser fresh restore](../reviews/windows-v1-pilot-rc-review.md) is separate. Manual native fresh restore remains open. |
| G6. Uninstall preserving storage | PARTIAL | App/uninstaller/registration removed; representative storage recursively hash-checked across uninstall. | [Same-host verifier](../reviews/windows-v1-installer-workflow-qualification.md) passed catalog preservation; broader recursive-data/clean-environment coverage remains open. |
| G7. Concurrent-import probe observed | OPEN | Two simultaneous imports of the same document; both outcomes and post-restart check recorded and judged. | Not rerun; the historical contention observation is not silently closed. |
| G8. Tester checklist Sections A–B | OPEN | Every item PASS or NOT TESTED with an accepted reason; zero FAIL without a filed bug report and a disposition. Section A on the browser path, Section B on the installed desktop app, both on the decision build. | Completed checklist: / Bug references: / |
| G9. Zero open Blocker bugs | OPEN | No Blocker-severity bug report open against the decision build. Major bugs need a documented workaround or an explicit scope exclusion. | Bug list reviewed: / |

## Workflow gates (block the named workflow, not the whole pilot)

The pilot may proceed without a gated workflow only
if the decision record explicitly excludes that
workflow. Testers must not use an excluded workflow
and report it as piloted.

| Gate | Workflow blocked until closed | Evidence required | Evidence |
| --- | --- | --- | --- |
| W1. Native credential-store acceptance | Desktop API-key setup (Connect AI) | Manual CredUI steps per qualification checklist step I: native prompt, cancel survival, synthetic-key storage with storage/log grep clean, restart persistence, guarded removal, Unused credentials. | Per-step observations: / |
| W2. Connect AI Windows review | Desktop API-key setup | Windows review of Connect AI setup + verification repair (connection-bound verification, orphan ledger, browser refusal) on the decision lineage. Linux evidence only today. | Review reference: / |
| W3. Native file-dialog observation | PASS for document import on source 6f216e3 | Actual Windows Open dialog, cancel, Unicode TXT, MD, PDF, DOCX, unsupported, oversized and empty fixtures observed. Not export/save dialog or exhaustive format acceptance. | [Eight observed cases](../reviews/windows-native-document-picker.md); post-close four documents/four completed jobs and all body hashes verified. |
| W4. M13 PDF/DOCX Windows review | PDF/DOCX import | Windows review of PDF text-layer import and DOCX ingestion (Linux-verified only today: PDF e2e 5/5, DOCX e2e 7/7). TXT/MD import is Windows-verified (Slice 1) and is not gated by W4. | Review reference: / |
| W5. M8 Slice 4 Windows verification | LEXI-COLLIDER saved-project input | Independent Windows verification of Slice 4 per its own addendum (Linux-only today). Slices 1–3 are Windows-verified. | Review reference: / |
| W6. Live-provider verification | Any live (non-synthetic, non-local) provider use | Explicitly authorized live-provider verification under the M5 production gates, with real-provider evidence. No live use is part of the synthetic pilot. | Authorization + evidence: / |
| W7. M7 native desktop acceptance + preview promotion | Native desktop promotion beyond controlled testing | M7 acceptance evidence and an explicit promotion decision. Controlled testing on the qualified installer is not promotion. | Acceptance reference: / |

## Deferred or non-blocking (do not confuse with gates above)

These do not block controlled customer testing of
the in-scope workflows, because the pilot proceeds
without them. They block wider claims or later work
as noted. None is available in the pilot, and none
may be presented as piloted.

| Item | Status | What it blocks instead |
| --- | --- | --- |
| ChatGPT OAuth ("Continue with ChatGPT") | Deferred, disabled, dev-gated only; eligibility unverified | Any ChatGPT sign-in offering |
| Generated-code execution (§32 sandbox) | No sandbox exists; execution unavailable | Any build/test/run/deploy of generated code |
| ANANSI Corporate / Nanzi / Enterprise Workspaces (2.0) | Planning only | Any 2.0 feature claim |
| OCR, transcription, broad connectors, background ingestion | Planned / separate slices | Scanned-PDF and connector workflows |
| AI Matrix workflow, Big Brain deliberation | Planned / design-gate only | Those workflows |
| Authenticode signing | Later (M15) work; installer is unsigned by design | Wider distribution beyond directly-distributed controlled testing; SmartScreen behavior must be recorded, not bypassed |
| G1 external audit | Open | Any external security certification claim |
| SOMNIA AC-I10 live rerun | Blocked (no local model/credential on the Linux VM; needs the Windows side) | Any claim that compaction improves a real agent; synthetic-benchmark results only |
| M5 production gates (remainder) | Open | Production use beyond the controlled pilot |
| macOS build, hosted multi-user deployment | Planned | Any macOS or hosted claim |
| SwarmTank v2 / SOMNIA compaction Windows reviews | Outstanding (Linux-only) | Windows claims for those slices; pilot use limited to their Windows-verified subsets |

## Historical RAM hold (G1), resolved 2026-11-13

The unchanged preflight later passed with 7.13 GiB
available, and the isolated build completed. The
earlier hold remains in its dated evidence. The
procedure below applies to future builds; it is not
a current blocker for the already-built candidate.

1. Free enough host memory for the unchanged
   preflight to pass (at least 4 GiB available).
   Do not lower the threshold and do not edit the
   preflight to pass.
2. Rerun the preflight and record the new RAM
   reading and verdict.
3. Then build the exact candidate using the
   explicit isolated E: layout in the Windows
   handoff, keeping qualification sequential.
4. Record the build identity, installer hash, and
   per-item native results before any pilot
   approval. Installer install/uninstall checks
   must use a clean machine/VM and retain the
   refusal of an installed owner copy.

## Decision rules

- Approval requires: all Section "Gates that must
  close" evidence fields completed with passing
  verbatim results on the decision build; every
  workflow the pilot will use either ungated or
  with its workflow gate closed; every other
  workflow explicitly excluded in the decision
  record; zero open Blocker bugs.
- A FAIL in any gate stops the decision. Record
  the first failing assertion verbatim, file a
  bug report, and do not work around it.
- A refused guard (lease, running process,
  occupied port, owner-data path, existing
  installation) is a stop-and-reconcile, not a
  product failure and not a pass.
- The decision, with its conditions and
  exclusions, travels with the build identity.
  A different build needs a new decision record;
  evidence does not transfer across builds.
