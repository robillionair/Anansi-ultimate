# Windows follow-up — document input repair

Runtime under review: `ef0b9843f129cf98d29aa60bd8d678b1a694eddb` over reviewer `97ab19b43ee9b72bf6dff868c49e23f4cab6d864`. ZIP SHA-256 `03ec8047d7773c0392e3f26bd96ea4b6ec2697d24d0d2c1f46fdafb68a537f24`, bundle `52edb7fe862e2d8be3f34a1de71c672a1341aede60900864f95756b41df9a86c`; verified. Existing reviewer probe unchanged. Runtime Rust and contracts unchanged from previous reviewed document runtime; their existing passes are retained as prior evidence, not rerun results.

Acceptance checks: rerun the unchanged DI-01/DI-02 probe and canonical document workflow; additionally test rejection of a newer oversized selection while an older valid read is still gated. This is a source-selection identity boundary, independent of provider behavior.

## DI-03 — pre-read refusal does not invalidate an older pending read (P2)

The new `file.size` early-return branch in `pickDocument` sets an error but neither invalidates `DocumentLoadSession` nor resets loading. Sequence: reviewed beam text exists; select small file A whose read is gated; select oversized file B and see rejection; release A. A still owns the session generation and replaces the reviewed text/source despite the newer rejected selection. Refusal should preserve the last applied beam and attribution, invalidate pending prior reads, clear their loading state, and display the current rejection. It must not resume an obsolete selection.

Fix the boundary once for every new selection, before any early return. Do not remove the pre-read size guard, weaken the original failure guard, clear the last applied beam on rejection, or merely suppress the new probe. Keep original success and failure cases, edit/clear/unmount behavior, and same-name reselect.

## Scoped checks

UI/contracts95/95; TypeScript, contracts drift, docs186 files, web build pass (existing chunk warning). Browser logs supplied separately. No new backend/schema/contracts edits, native picker certification, full regression, preview promotion, cloud calls, credentials, push or CI. Existing owner data/previews untouched.

Final Windows browser verdict: canonical document workflow1/1 (34.0s), unchanged DI-01/DI-02 reviewer probe1/1 (27.3s). New rejected-selection probe fails only its expected-success identity assertion: actual beam became 'Obsolete content must not win' instead of retaining 'Keep this reviewed text'; inherited save/restart/restore completed. DI-01 and DI-02 closed in this scope; DI-03 remains open. Shared lease released.
