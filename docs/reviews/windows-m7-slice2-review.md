# M7 Slice 2 — independent Windows review

Date: 2026-10-26. Delivered application:
`9928bf5cb09f07cb90c911b7c5279ad69bc748cb`, parent baseline
`518418b6a955feaadcc6212eab0a3f3fd1e1bcb9`.
Isolated checkout: `E:\Anansi\worktrees\m7-slice2-review`.
Existing M7 preview at `3335248b` is preserved and is not an ancestor of this delivery.

## Delivery verification

ZIP SHA-256:
`607fa6176597c5ab302b956c7523da4e5e6dad2670659cd15c0e10371e61da38`.
Bundle SHA-256:
`d2523e9d07c5761f09f59515416671d20b160aa2c31a092a32a1db119b972edf`.
Both match the handoff. Bundle verification succeeded; fetching its explicit
Slice 2 branch produced the exact stated tip, preserving the repaired baseline.

## Focused Windows results on the delivered tip

- `forge_revision`: 5/5.
- `forge_generation`: 5/5.
- `forge_restore`: 4/4.
- Forge library units: 10/10.
- UI/contracts: 57/57.
- TypeScript, generated-contract drift check, web build and documentation validation passed.
- Core all-target Clippy with warnings denied passed.
- The API binary built successfully on Windows.

Builds, dependency installation and synthetic test roots were kept on E:.
No generated product code, real provider or paid inference was executed.
This is focused acceptance, not a new full-workspace or production certification.

## Browser review

The delivered browser spec first failed before reaching Forge revision: its
project-open path displayed "Local core unavailable" and the idea-set button
was absent. Rust suites were concurrently exercising Windows storage checks.
This failure is retained as observed evidence; it does not establish a revision
backend defect. A sequential rerun with unchanged timeouts and an additional
package-version-switch assertion was started after all Rust suites finished.

Final browser outcome and any resulting correction are recorded below.

### Confirmed defect and bounded correction criteria

The sequential browser probe reached a completed proposal against package v2,
then selected v1. The v2 proposal and its Accept button remained visible while
the page described v1; the added assertion failed with one Accept button instead
of zero. `ForgeRevisePanel` retained React state between identity/version changes;
its effect reset the controller but not the completed proposal or disclosure.

The correction must remount the revision panel when workspace, project, package
identity or package version changes, cancelling an active old controller through
its existing cleanup. Completed proposals, instructions, disclosure and save
state must not transfer. A regression must generate against v2, switch to v1,
prove proposal/Accept are absent and instructions reset, then finish the existing
restart and backup/restore assertions without weakening them. No backend policy
or approval rule changes are required.

### Correction and final result

`Projects.tsx` now keys `ForgeRevisePanel` by workspace, project, artifact id and
artifact version. React therefore discards old local state and runs the old
controller's cleanup when any parent identity changes. No backend or timeout was
changed. The regression is retained in `tests/e2e/forge-revision.spec.ts`.

The final Windows browser workflow passed 1/1 in approximately 2.5 minutes using
the canonical Playwright configuration and installed Chromium: generate/save
specification, approve, generate/save package, request/reject revision, retry/accept
v2, request another proposal, switch to v1 and prove no stale proposal/Accept or
instructions remain, restart/reopen, backup/fresh restore and reopen the restored
package. The original restart/restore assertions remain intact. Both package
versions remain present; the discarded proposal creates no third version.

After this UI correction: UI/contracts 57/57, TypeScript, web rebuild,
documentation validation, formatting and diff checks passed. Rust source did not
change after its passing focused checks and strict Clippy. Application-test
leases were released normally. Browser results use a synthetic loopback provider
and do not certify real inference. Native Tauri acceptance of Slice 2 has not
been run; the existing Slice 1 desktop preview remains unchanged.

For Muse: use the reviewed branch as the next baseline rather than reverting
this state-reset correction. The recovery bundle also includes the previously
unavailable `codex/local-preview-m7` branch at `3335248b` for later preview
integration. That branch contains prior preview materials, not Slice 2 promotion.

## Preserved limits

No preview promotion, push, master merge or remote CI was performed during review.
Generated-code execution and the standing native credential, live-provider,
external crypto audit and remote-CI production gates remain open.
