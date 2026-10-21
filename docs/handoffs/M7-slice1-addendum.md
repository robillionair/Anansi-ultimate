# M7 Slice 1 addendum — Product Forge: concept → specification → source package

Date: 2026-10-21 · Branch `muse/m7-slice1-product-forge`
over the verified R17 preview checkpoint `e915f63` (whose
Windows UI acceptance Antigravity recorded at `15affdc2`).
Design + acceptance criteria:
`docs/handoffs/M7-slice1-design.md`. Authority:
MASTER_SPEC §§28–33; requirement AN-012 (**in progress**:
this slice only).

## What shipped

**Workflow.** From a saved idea/opportunity artifact:
pick a concept → answer three essential questions
(purpose, intended users, must-have behavior) → the model
drafts a product specification through the existing M5
provider machinery → the user reviews it and approves by
saving it as an immutable `specification` artifact → from
the pinned specification version, the model drafts a
development plan + source files → the user inspects files
as inert text → saves an immutable `source-package`
artifact → exports a deterministic `.tar`
(`ANANSI-PACKAGE.json` manifest with provenance +
per-file SHA-256, then the files) → restart/reopen →
revise the specification into a new version and build
again pinned to it → backup/fresh restore resolves the
whole chain to the restored artifacts.

**Pinning.** Spec runs read the concept from the exact
requested artifact version (a v2 concept edit provably
does not leak into a v1-pinned run — wire capture +
spec document); build runs read the exact specification
version; saves cite `artifact://…@v…`; restore remaps
spec→source and package→spec/source references fail-closed
(a package citing an absent specification refuses the
whole restore) with hash recompute.

**Output safety (core-validated).** Specifications are
strict-parsed with bounded fields. Packages: 2–12 files;
paths relative and portable (no traversal, absolute,
backslash, drive letters, dot segments, hidden files);
extensions `.html/.css/.js/.md/.txt` only; no duplicates
(exact or case-folded); no NUL; ≤32 KiB per file; ≤192 KiB
total; root `index.html` and `README.md` required. The
export tar contains regular files only (mode 0600, uid/gid
0, mtime 0 — no links or executable metadata) and export
re-validates the stored set before writing it.

**Honesty (§§30/32/33).** Everything is labelled a
prototype source package for a dependency-free static web
app — not SaaS, no auth/database/deployment. The package
limitations are embedded in every saved package and every
export manifest (including "No license is granted or
implied by ANANSI"). The UI pipeline shows specification,
plan and source generation as completed only when saved,
and Build / Tests / Preview / Deployment as **Unavailable**
with the §32 reason: no execution sandbox exists in this
slice, so export-for-review is the deliverable. Generated
code is never executed, previewed, installed or fetched
by ANANSI.

**M6 documentation reconciliation (first workstream).**
`plans/PHASE-006-innovation-studio.md` status now records
the four delivered, Windows-accepted M6 slices and what
remains planned/gated (broader Opportunity Engine,
Collider = M8, pre-project storage decision, M5
production gates); `docs/handoffs/M6-plan-proposal.md`
gained an appended dated status banner — its historical
text is unchanged. No requirement or gate is declared
complete; AN-027 stays "in progress".

## Verification (Linux, this machine)

- Forge unit tests 6/6 (path/package validation, spec +
  package parsing, tar determinism, base64, remaps).
- `forge_generation` 3/3: full workflow with exact-version
  pinning (wire capture), duplicate save, export tar
  structure + manifest, restart persistence, spec revise →
  v2-pinned build; malformed/traversal output and provider
  failure save nothing; cancellation, wrong-stage save,
  cross-workspace refusal, archived-project refusal.
- `forge_restore` 2/2: chain restores with remapped
  references and the restored package exports; a package
  citing an absent specification fails the restore closed.
- Node contracts/UI 54/54 (incl. 3 new ForgeRunController
  lifecycle tests); tsc clean; contracts check PASS.
- Browser e2e: new `forge.spec.ts` passes (full workflow
  incl. downloaded-tar verification and restart/reopen);
  the canonical subset passes 10/10 sequentially.
- Strict Clippy `--all-targets -D warnings` and fmt clean
  for anansi-core; documentation validation PASS.
- Full anansi-core suite: see BUILD_STATUS (only the 4
  known environmental live-Ollama failures).

A note on process: an early battery run executed suites
in parallel and produced contention failures (23 across
unrelated suites); all suites were re-run sequentially on
an idle machine for the figures above — parallel-run
numbers were discarded, not averaged in.

## Not in this slice (unchanged boundaries)

Execution sandbox, build/test/preview/deployment (a
later, separately approved slice; Antigravity reviews
that boundary independently — its files untouched),
further templates/product types (§28 list stays planned),
dependency installation, licensing generation, launch
material (§33 commercial extras), background generation.
No pushes, master merges, remote CI, real credentials or
paid inference. Windows verification of this slice and
the M5 production gates remain open.
