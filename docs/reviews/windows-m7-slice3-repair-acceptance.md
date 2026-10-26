# Windows M7 Slice 3 repair verification — 2026-10-26

Repair baseline: `74c4196744e1623e368bda302c0cbc3c62a9e38c`.
Review branch: `codex/review-m7-slice3-repair`.
The earlier failing review checkpoint `bd72a4ffe` is an ancestor. The local
selection-order correction below is applied on top of the repair baseline.

## Delivery verification

ZIP SHA-256:
`a6266e3cc7c68d5a75f9b42ffd060e6d2e7b9a1811990caeecb39a2a14795e99`.
Bundle SHA-256:
`74463ee148eb1468dadec4bf8bf66a2431b3c08293b7c08cf9ee3cc648539eb5`.
Both match the handoff. Bundle verification succeeded and explicit branch fetch
produced the exact application checkpoint. The prior preview ref at `3335248b`
remains separate and unchanged.

## Review scope

Rerun the three independently reproduced acceptance defects: inconsistent stored
deliverables at export, invalid original pinned hashes at restore, and stale
saved-document editing state after version changes. Preserve valid round trips,
immutable versions and the no-execution boundary.

Code inspection confirms export now calls the shared stored-record validator;
restore validates original rows and bindings before remapping; the saved editor
is keyed by full identity and resets its buffer on Cancel. Rust backend code is
unchanged in this review; a local frontend selection-order correction is described
below. Four browser-test lines additionally verify
that an edited-and-cancelled draft is absent after reopening the editor.

## Focused Windows verification

UI/contracts 60/60, TypeScript, generated-contract drift, web/API builds,
documentation validation and strict core all-target Clippy passed.
The complete Windows `forge_deliverable` suite passed **9/9**, including both
formerly failing review probes, all supplied controls, and the additional
inconsistent-record cases. Forge library units passed **11/11**; formatting passed.
The original invalid-hash restore now refuses on repeated attempts, publishes no
restored workspace, and leaves source versions unchanged. Final browser results
are recorded below after completion.

Build output and synthetic test roots use E:. Responses are synthetic loopback
fixtures; no real credentials, model download, paid inference or generated
software execution occurred. Browser acceptance runs through the shared test
lease wrapper after the Rust workload ends, using the canonical Playwright
configuration and installed Chromium.

## Remaining scope

### Additional Windows selection-order race

The first repaired browser run failed the retained version-switch assertion.
The error snapshot showed the page had returned to **Viewing version 2** after
the test had observed v1. An asynchronous post-save list refresh called
`openArtifact` for latest after a newer explicit history selection. Component
keys alone cannot stop an older parent request from changing the selection.

The bounded correction must guard artifact reads and post-save refreshes by a
selection generation, invalidating earlier work on a newer selection, project
change or unmount. A save callback must retain its original viewed generation,
including when a selection is pending. Keep the original hard browser assertion
and verify cancelled drafts stay discarded. No backend or timeout changes.

The local correction tracks artifact selection generations through exact-version
reads and automatic post-save refreshes. A save callback captures the generation
of the rendered artifact, so it cannot override a newer pending or completed
selection. Older reads also refuse to publish after a later selection, project
open, workspace change or unmount. This applies to the shared source-revision and
deliverable post-save reload paths. UI/contracts 60/60, TypeScript and the web
rebuild passed again after this correction.

A second rerun retained v1 but still showed the text used to create v2 when
reopening its editor: reselecting the same version does not change the component
key or trigger its identity effect. The editor now also resets from the displayed
saved document whenever editing opens. Neither failure occurred in the added
Cancel assertion; both failed the original retained v1-text assertion. These
failed reruns are preserved as diagnostic evidence, not passing acceptance.

### Final deliverable browser result

After both local frontend corrections, the complete canonical Windows
`forge-deliverable.spec.ts` workflow passed **1/1** in approximately 2.3 minutes.
The original hard version-switch assertion passed; an edited Cancel draft was
absent on reopen; export, documentation revision, restart, backup/fresh restore
and restored export all completed. No assertion or timeout was weakened.
The final frontend tree again passed UI/contracts 60/60, TypeScript and the web
build. The existing canonical `forge-revision.spec.ts` regression also passed
**1/1** on the final tree in approximately 2.5 minutes, including revision
rejection, acceptance, exact-parent version switching, restart and fresh restore.
Both browser sessions released their application-test leases normally.

**Disposition:** the three review findings are closed for focused Windows
backend/browser acceptance on this review branch, including the additional local
selection-order and editor-reopen corrections. Keep these frontend changes when
integrating; the unmodified Muse tip alone is not the final browser-verified tree.
No full-workspace or native desktop acceptance is implied.

This is focused Windows backend/browser verification, not a new full-workspace,
native Tauri, real-model or production certification. Native desktop acceptance
and preview promotion of Slices 2–3 remain separate tasks. Existing preview code,
launcher and personal data are preserved. The execution sandbox, real native
credential/provider checks, G1 audit and remote CI gates remain open. No pushes,
master merges or remote CI runs.
