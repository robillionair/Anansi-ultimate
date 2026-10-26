# M7 Slice 3 — independent Windows review

Date: 2026-10-26. Delivery:
`cbb95a6960f0bb1379741bdc6a6a34e900611e18`, branch
`muse/m7-slice3-deliverable`, based on reviewed Slice 2
`0f586f865a8c4d227729c53449ce171b03b18fe7`.
Isolated review: `E:\Anansi\worktrees\m7-slice3-review`.

## Delivery identity

ZIP SHA-256:
`7d2f0b9d1a6a5f86ac46fad744ed4b4d4a701eaf6a3250b69f831ec1ecef9dea`.
Bundle SHA-256:
`db7299e22f6ae86216d52299cb4ad7439fe4a98142eb3f36be94498b28a047dd`.
Both match. Bundle verification and explicit branch fetch produced the exact
delivery tip. The prior preview ref at `3335248b` is preserved separately.

## Passing focused Windows checks

- Supplied `forge_deliverable` suite: 6/6 on the delivered application code.
- Forge library units: 11/11.
- UI/contracts: 60/60.
- TypeScript, generated-contract drift check, web build, documentation validation.
- Strict core all-target Clippy with warnings denied; API binary build.

These are focused results, not a new full-workspace or native desktop acceptance.
Tests and build output use E:. Only synthetic loopback providers are used;
generated product source is never executed.

## Additional expected-success acceptance probes

The review adds tests, without changing production application code:

1. `review_export_rejects_inconsistent_generic_edits`: use the supported generic
   artifact revision command to create four malformed deliverable versions. Export
   must refuse a false production-ready claim, a specification identity that does
   not match the package, duplicate documentation entries, and an unsupported
   format version. Valid v1 remains the control. These records have valid registry
   content hashes; this is domain consistency validation, not a modified SQL file.
2. `review_restore_rejects_mismatched_pinned_package_hash`: save a deliverable,
   generically revise its package pin to an invalid hash, prove direct export
   refuses it, then back up and fresh-restore. Restore must refuse the invalid
   original binding instead of replacing it with a valid hash.
3. Browser `forge-deliverable-review.spec.ts`: retain the supplied workflow, add
   a switch from deliverable v2 to v1, then open the documentation editor. Its
   architecture text must match v1, not the prior v2 draft. A soft assertion lets
   the existing restart and restore checks continue even if the new probe fails.

The new assertions specify correct behavior: failures indicate acceptance defects,
not successful recovery.

## Confirmed backend acceptance failures

`node scripts/cargo.mjs test -p anansi-core --locked --test forge_deliverable review_`
finished with **0 passed / 2 failed**, with the supplied six tests filtered out.

### Export trusts inconsistent stored deliverable fields

The export probe reported all four malformed records accepted:
`["status", "specification", "duplicate-document", "unknown-format"]`.
The supported generic artifact editor can therefore produce an export manifest
with a false production-ready flag, the wrong specification identity, duplicate
document metadata, or an unsupported source format version. Registry body hashes
are valid: hashing a body does not validate its domain claims.

`export_deliverable` validates the actual package, hashes and one verified-facts
entry, but copies other body fields to the manifest without a complete stored
document validation. The repair must validate exact supported format, all-false
status, exact package/spec identity and versions, a unique complete document set,
permitted origins and bounds. Manifest metadata and archive entries must agree.
Do not merely hide the generic editor or silently relabel an invalid record.

### Restore replaces an invalid original binding instead of refusing it

The restore probe first proved direct export refused a deliverable whose
`packageContentHash` was all zeroes. After a normal backup, fresh restore returned
success and materialized `Invalid pin review`. `fix_deliverable_hashes` overwrites
the pinned hashes using imported rows without validating the original binding.

Validate source-domain relationships and original pinned hashes before remapping
or hash replacement. Only a valid original binding may be translated to restored
equivalents. Preserve legitimate hash changes caused by identity remapping; invalid
originals must not become valid through restore. Add coverage proving destination
state remains unchanged on refusal, with valid round trips as controls.

## Confirmed browser acceptance failure

The sequential browser probe finished **1 failed** in approximately 3.2 minutes.
Its only reported failure was the new soft assertion after selecting deliverable
v1 and opening its documentation editor:

- Expected v1: `Model architecture: index.html loads app.js; state lives in localStorage.`
- Received v2 draft: `Architecture (revised by the user): index.html plus app.js.`

The remaining existing workflow assertions, including restart and fresh restore,
completed after that soft failure. This is not an overall browser pass.
`DeliverableView` initializes its edit buffer from props once and is not keyed
by selected record/version in `Projects.tsx`, so an old edit buffer survives a
version switch. The prepare panel is keyed correctly; the saved-document view
needs the same identity discipline or equivalent safe synchronization. Verify
version switches, record switches and cancellation without stale draft carryover.

## Disposition and reproduction

**Not accepted for preview promotion.** Three repair items remain: strict stored
deliverable validation at export; validation of original bindings before restore
hash fix-up; saved documentation editor identity/lifecycle state.

```text
node scripts/cargo.mjs test -p anansi-core --locked --test forge_deliverable review_
node scripts/with-application-test-lease.mjs node_modules/@playwright/test/cli.js test --config playwright.config.ts tests/e2e/forge-deliverable-review.spec.ts
```

The first probe compilation was attempted while the supplied test executable was
still running and encountered a Windows linker file lock. The definitive probe
results above came from the rerun after all six supplied tests completed. No test
process was killed or deadline extended. Application-test lease released normally.
See `M7-slice3-repair-assignment.md` for the bounded Muse repair assignment.

## Boundaries

No preview promotion, push, master merge, remote CI, real-provider inference or
generated-code execution. The existing running preview and its data are unchanged.
Native credential/live-provider/G1 audit/CI production gates remain open.
