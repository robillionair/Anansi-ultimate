# Muse assignment — bounded M7 Slice 3 acceptance repair

Continue from the attached review branch `codex/review-m7-slice3`, based exactly
on your delivered `cbb95a6960f0bb1379741bdc6a6a34e900611e18`. The package manifest
records its full review SHA. Application production code is unchanged: the review
adds evidence and expected-success regression tests. Do not restart the slice or
implement new features. Read `windows-m7-slice3-review.md` in this directory first.

The supplied Windows deliverable suite passed 6/6, Forge units 11/11,
UI/contracts 60/60, TypeScript/contracts, web/API builds, strict core all-target
Clippy and docs validation. Preserve those results and their existing assertions.
Additional review probes expose missing acceptance cases; their failures are not
successful recovery results.

## Repair the stored-record consumption boundary

`review_export_rejects_inconsistent_generic_edits` uses supported generic artifact
revisions, with valid registry hashes. Export currently accepts all four cases:
false `productionReady`, a specification identity inconsistent with the package,
duplicate document entries, and an unsupported format version.

Validate the full saved deliverable before exporting it: supported format and
version, immutable package/spec identity/version/hash relationships, all-false
unexecuted status, exact unique complete document set, permitted origins, text
bounds and matching verified facts. Manifest metadata must match the actual
archive contents and the backend-resolved provenance. Cover other status flags,
wrong versions, absent/extra documents and invalid origins as appropriate.
Do not hide the generic editor as the sole fix, silently override invalid claims,
or assume a valid body hash means a valid domain record.

## Validate original bindings before restore remapping

`review_restore_rejects_mismatched_pinned_package_hash` proves the source export
refuses an invalid pin, yet backup/fresh restore succeeds. The restore hash fix-up
currently replaces that invalid original hash with the imported row's hash.

Before changing ids or hashes, validate original package/spec relationships,
workspace/project ownership, versions and original pinned hashes. Only then
translate valid references and recompute hashes for changed bodies. Preserve valid
round trips and approval behavior. Invalid originals must refuse before publishing
destination state, leaving source and existing destination workspaces intact.
Keep the new failing test and strengthen it with no-mutation controls.

## Inspect the browser review result

`forge-deliverable-review.spec.ts` retains your workflow and adds a version-switch
probe. The Windows browser probe failed because selecting v1 loaded the prior v2
architecture draft. All other original workflow assertions completed, including
restart/restore; the overall result remains failed. Ensure the saved
deliverable documentation editor always loads the exact selected artifact/version,
and that cancelled or superseded edits never carry into another version or record.
Reapply the existing full-identity lifecycle discipline where needed. Consolidate
this assertion into the canonical spec after fixing it, avoiding two permanently
duplicated browser workflows. Preserve restart and fresh-restore coverage.

## Verification and delivery

First reproduce the added failures. Define acceptance criteria, then repair the
causes without warning suppressions, removed assertions or longer deadlines.
Run the full deliverable suite (including added tests), relevant Forge and recovery
regressions, the browser workflow, UI/contracts, TypeScript, contracts drift,
strict Clippy, formatting and docs checks. Report exact Linux results separately
from pending independent Windows verification.

Review reproduction commands:

```text
node scripts/cargo.mjs test -p anansi-core --locked --test forge_deliverable review_
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/forge-deliverable-review.spec.ts
```

Use the existing lease policy for application tests; never steal a lease.
Build the matching API and web assets before browser acceptance. The diagnostic
browser spec expects the API binary at the repository's existing `target/debug`
location, as the canonical specs do.

No new features, preview promotion, generated-code execution, real credentials,
paid/cloud inference, model downloads, deployment, pushes, master merges or remote
CI. Preserve the reviewed Slice 2 fix and prior preview ref. Keep production gates
open. Deliver a small repair checkpoint in a checksummed bundle-in-ZIP with exact
ancestry, evidence and an independent fetch verification. Complete the repair and
verification autonomously; ask only for a genuinely missing requirement.
