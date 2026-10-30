# M8 overnight repair — independent Windows review (2026-10-30)

Incoming `d0a6c1e1dd8612c954e9aceee9d735f5653780c8`; branch `codex/review-m8-overnight-repair`. Original reviewer probes verified unchanged before execution. Owner previews/data and Antigravity checkout untouched.

## Acceptance before rerun

- Original two restore probes and browser anchor probe pass without edits.
- R1 allowed-transform check must reject invented snapshot fields in supported structured artifact formats, including context-free Collider experiments. Absence of optional project context is not permission to skip other immutable fields.
- Current-source boundary and affected UI/static/build/browser regressions retain their existing bars.
- Review only: no application implementation changes, cloud calls, pushes, preview promotion or CI.

- A substituted ID must have an evidenced original-to-restored mapping. An arbitrary missing old ID plus an existing new ID is not proof that any restore occurred. In a never-restored fixture, fabricated snapshot project identities must refuse.

## Package identity and verdict

ZIP SHA-256 `cd1cba33e82defa8ae8896661a0423f2562d574a4fd9e989c8ee21c7492503fa`; bundle SHA-256 `6bf17c508347405c8a060416ee6d5f6604b03675231c4d8bcfad532f4146fb65`. Exact incoming tip and reviewed ancestry verify. Original reviewer Rust probes and browser probe were compared against `240af4ef` and are unchanged.

**R2 and R3 pass focused Windows acceptance. R1's original note forgery is refused, but the broader provenance boundary remains open through two reproduced paths below. Do not promote this checkpoint.** Reviewer changes only add two tests and evidence; application code is unchanged.

## Passed Windows checks

- Original Rust reviewer probes: **2/2 pass**. Selected-concept hash diagnostic is now `true`; the second fresh restore succeeds. Original forged note snapshot is refused.
- Original browser reviewer probe, new instance-anchor DOM/keyboard test, canonical project-context workflow and integrated selection/transfer/restart/double-restore workflow: **4/4 pass** sequentially on real Windows API/web with synthetic providers.
- Current-source boundary suite **7/7 passes** (both pickers, archive/Sealed/expiry/reassignment plus eligible older-revision control).
- UI/contracts **90/90**, TypeScript, strict core all-target Clippy, generated contracts, formatting/docs and API/web builds pass. The added second probe also receives targeted strict Clippy. Existing schema ignored-`uint` and web bundle-size warnings remain warnings, not failures.
- No full-core or native desktop certification is claimed. No cloud calls, credential use, downloads, generated execution, pushes, CI or preview promotion. Owner data/previews and Antigravity's checkout unchanged. Shared application-test lease acquired/released normally.

## Remaining R1a — P1: optional context bypasses the final immutable-field comparison

`allowed_restore_transform` in `crates/core/src/restore.rs` returns `normalized.get("projectContext").is_none()` immediately when the original Collider snapshot has no project context. That returns success before the final `normalized == original` comparison. Thus two context-free experiments with the same format need not have the same result text.

Probe creates a genuine saved context-free experiment, uses it as a selected artifact in a second saved experiment, then uses the generic artifact revision API to alter only the nested snapshot's `result.sourceUnderstanding` and its coherent `contentHash`. No `restoredContentHash` is needed. The real source artifact stays unchanged. Fresh restore accepts the forged snapshot rather than refusing it.

Reproduce: `node scripts/cargo.mjs test -p anansi-core --locked --test collider review_forged_structured_snapshot_without_project_context_refuses_restore -- --exact --nocapture`. Expected refusal/no publication; observed accepted restore. Recorded alongside the two old probes: **2 passed / 1 failed** in `review-probes.log`.

Repair: absence of an optional field must not skip comparison of unrelated immutable fields. All successful transform paths must validate the whole normalized document. Add absent/present/empty context cases and invented result/beam/control/text negatives while retaining legitimate restore controls. Audit other success-return paths, not just this exact branch.

## Remaining R1b — P1: package membership is not an original-to-restored identity mapping

`check_id_change` accepts any differing original ID absent from the package when the transformed ID exists somewhere in a package table. It does not verify an actual restore-origin pair, source kind or history. The comment claiming substitutions through the restore identity map is stronger than the implementation.

Probe creates a real experiment WITH project context, selects it as an artifact in another experiment, asserts that the source database has **zero restore_origins rows**, then changes only the nested snapshot's `projectContext.projectId` to a never-existing 32-character hex ID and recomputes its snapshot hash through generic revision. The real source row still names the real project. Fresh restore nevertheless accepts it as an allowed historical transformation. This case traverses the project-context branch and final whole-document equality, independently of R1a's early return.

Reproduce: `node scripts/cargo.mjs test -p anansi-core --locked --test collider review_fabricated_original_project_id_without_restore_mapping_refuses_restore -- --exact --nocapture`. Expected refusal/no publication; observed accepted restore. **0 passed / 1 failed** in `mapping-probe.log`.

Repair: verify genuine original-to-restored substitutions against backend-controlled mapping/history (or an equally strong verifiable contract), including correct kinds, ownership and multi-hop restores. Mere missing-old/present-new membership is not evidence. Preserve the original snapshot and original validation. Test a never-restored package, arbitrary unknown old IDs, wrong mapped identities/kinds/workspaces, genuine multi-generation restores and the existing approved-transfer controls. Do not trust an editable body-supplied map or blanket-refuse all legitimate transformed snapshots.

## Review scope and resume

Across distinct reviewer probes: the original two pass and the two additions fail. Do not describe this as full provenance acceptance or count reruns as extra passes. UI/navigation and final-hash ordering repairs are real progress and should be preserved.

Use `codex/review-m8-overnight-repair` (incoming `d0a6c1e1` plus additive review evidence) as Muse's next bounded R1 repair baseline. Do not change or weaken the original probes. Evidence: `E:/Anansi/benchmarks/m8-overnight-repair-windows-2026-10-30`.

