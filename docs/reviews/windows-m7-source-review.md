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
