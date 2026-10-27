# Windows M7 source-review acceptance

Delivery: `7607dc7cf17f5763c5311241835211cc14c6af71`, baseline `e6d22f711`.
ZIP SHA-256: `f3e22b66e152a0ffb06604c946700080d619be7519885da349b347364fe5f7e7`.
Bundle SHA-256: `277744e744ecb5b6747f0220cfd9ada4e12164766d5f994b09cd5e0a2b21c2f4`.
Checksums, all 8 refs, bundle integrity and baseline ancestry verified locally.
Isolated review checkout on E:, no running preview or personal-root changes.

Acceptance plan before corrections:

- Run the delivered review backend suite and shared UI/contracts/type/build checks.
- Exercise new workflow through real API/browser using synthetic providers only.
- Probe review-to-package selection: displayed body, history and generic editor
  must all refer to the target package at the exact requested version.
- Probe restore of a structurally hashed but domain-invalid review whose format
  discriminator was changed via the supported generic revise operation. Refuse
  before a destination workspace is published; preserve source history.
- Keep assessments explicitly inspection-only, including human-readable labels.
- Preserve all original tests and limits. Record initial failures separately.
- Focused Clippy/format/docs checks; restore/revision regressions as appropriate.

No preview promotion, native acceptance claim, real credentials, model downloads,
paid inference, generated-source execution, push, master merge or remote CI.
Results follow after execution; code inspection alone is not a reproduced result.

## Initial checks and corrections

- Delivered ZIP/bundle hashes and baseline ancestry matched; received 8 refs.
- Delivered shared UI/contracts 64/64, TypeScript, web build and documentation
  validation passed on Windows. No Linux-only results were relabelled Windows.
- All 10 delivered backend `forge_review` tests passed on Windows. Initial run
  also included a provisional format probe (11 total, 567.39 seconds). That
  provisional probe used the manual-inspection fixture, whose absent idea
  ancestor can itself cause restore refusal. Its pass is not evidence of the
  desired format guard. The probe was replaced with real generated ancestry
  and an explicit successful-restore control before final verification.
- Parser regression reproduced acceptance of a finding missing `fileRefs`.
  Such a finding reaches `.join`/`.map` in the viewer. Parser now rejects
  unrenderable shapes and unsupported format versions/statuses; normal records
  remain accepted. Focused regression and full UI suite passed after correction.
- Inspection found review-to-package navigation only changed the viewer. History,
  selected id and generic editor still referenced the review; a generic save
  would therefore target that review. Exact-version opening now publishes body,
  selection, history and editor together under the existing epoch guard, with
  ordinary error handling. A browser assertion/captured generic command verifies
  the package target. This finding was identified by code inspection; a failing
  pre-fix browser run is not claimed.
- Restore now rejects an unexpected format discriminator for a registry row
  already typed `source-review` at pre-validation instead of skipping that row.
  This is defensive early validation; an end-to-end pre-fix bypass has not been
  demonstrated by the discarded provisional probe.
- Assessment wire values remain unchanged; visible choices now read Not assessed,
  Needs changes and Satisfied by inspection. This does not claim passing tests.

No production cryptography/provider behavior or generated execution was changed.
Final exact commands/results follow after the corrected checks complete.

## Corrected checkpoint verification (`befc66f11`)

- Original delivered backend suite: all 10 tests passed on Windows. The initial
  run's additional provisional probe is excluded from substantive acceptance.
- Corrected restore regression passed 1/1, 169.26 seconds, after the early-format
  guard: real generated ancestry restores successfully as a control; changing the
  review discriminator then refuses with no named destination workspace and
  source review history still two versions.
- Full shared UI/contracts suite passed 64/64 after the parser/label/handoff
  changes. TypeScript and web build passed.
- Strict focused Clippy passed: `node scripts/cargo.mjs clippy -p anansi-core
  --lib --test forge_review --locked -- -D warnings`. Formatting and docs passed.
- API build passed. API executable SHA-256:
  `9aba23cf09b9cc5333574e9fa48e88ac6903ced708d21626e0fac7a6490fa43c`.
- Canonical `forge-review.spec.ts` passed 1/1 on Windows in 3.5 minutes against
  that API and the rebuilt web bundle, under the exclusive application-test lease.
  This includes the original workflow plus exact package history/editor identity
  and captured generic-editor request target, review Cancel/history, reject/accept,
  restart and fresh restore. No timeout or existing assertion was weakened.
- [Actual browser review screenshot](evidence/windows-m7-source-review.png)
  inspected: pinned versions, inspection-only limits and human-readable statuses
  render clearly. SHA-256:
  `e7987ca04bfea63a523bcabbca8ba0ebcccc3c88e18119860dcad5d7a6c990c5`.

The incoming bundle was also cloned into an independent bare scratch repository;
exact incoming tip `7607dc7cf17f5763c5311241835211cc14c6af71` and baseline ancestry
matched. Existing revision/deliverable browser regressions follow separately.
No full Windows core-suite or native desktop run is claimed for this slice.

Final shared-helper regressions passed, sequential and idle, with original
timeouts/assertions retained: deliverable 1/1 (3.3 minutes), source revision 1/1
(3.0 minutes), 2/2 total (6.4 minutes). Both include restart and fresh restore;
deliverable retains its version-switch and Cancel assertions. Generated-contract
drift passed; existing ignored `uint`/`float` format warnings were retained in
the external check log, not hidden or reported as schema changes.

## Verdict and handoff

Focused Windows backend/browser scope passes on corrected application checkpoint
`befc66f11c7975e1b1e8cd88c9a8bb54a8ebf77d`, followed by this evidence metadata.
Use the reviewed branch `codex/review-m7-source-review` as the continuation
baseline; preserve its parser, selection/history/editor and restore corrections.
The original delivered `7607dc7c` alone does not contain those corrections.

No claim of a pre-fix browser failure or demonstrated end-to-end restore bypass
is made: the handoff issue was found by inspection; parser acceptance was
reproduced; restore early refusal was hardened and its corrected controlled
round trip verified. Evidence is scoped to the checks actually run above.

The initial 10 delivered backend cases, corrected additional restore control,
64 UI/contracts, TypeScript, contracts drift, strict focused Clippy, formatting,
docs, API/web builds and all three browser flows are recorded separately to
avoid implying a new full Windows core/provider/API test-suite run.

The shared application-test lease released normally. Existing running preview,
its launchers, personal data and source branches were untouched. Source-review
native desktop acceptance and promotion remain pending; earlier Slices 1–3
native evidence remains valid at its own checkpoint. M7 as a whole, the execution
environment and M5 production gates remain open. No real credentials, paid/cloud
inference, model downloads, generated execution, pushes or remote CI occurred.
