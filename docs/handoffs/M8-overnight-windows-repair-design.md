# M8 overnight Windows repair — design (committed before code)

Baseline: branch `muse/m8-overnight-windows-repair` from reviewer tip
`240af4ef58b09e7ada405286f46f927c902f9c52` (`codex/review-m8-overnight`),
which is `edba7ed3` (overnight hardening) plus reviewer probes/docs only.
No runtime code is changed by the reviewer; the three probes below are
preserved verbatim and are the acceptance tests for R1–R3.

Before state, reproduced on Linux 2026-10-30 (logs kept with the delivery,
not labelled acceptance):

- Rust probes: `review_` filter 0/2 — forged snapshot restores instead of
  refusing (`assert_restore_refused` fails); concept-context probe prints
  `selected concept pins final restored target hash: false` and the second
  fresh restore fails with `InvalidRequest`.
- Browser probe: `tests/e2e/review-collider-anchors.spec.ts` 0/1 — only the
  final saved-target assertion fails (`saved experiment section link
  resolves outside its artifact view`); all earlier live/saved steps pass.

Scope is bounded to R1–R3. No new feature, no provider/cloud call, no
credential, download, preview promotion, push, master merge or remote CI.
M8 3D, URL fetching and document ingestion stay unfinished. Linux evidence
does not close Windows, native or production gates.

## R1 — restoredContentHash must stop authorizing provenance

Defect, in `crates/core/src/restore.rs`
`validate_original_experiment_bindings` (artifact branch): validation
accepted a source row when `row_hash == content_hash` OR the body-supplied
`restored_content_hash == row_hash`, while only checking that the snapshot
content hashes to `content_hash` and that the row body hashes to `row_hash`.
An attacker using only the generic artifact `revise` operation can set an
experiment item's `content` to invented text, `content_hash` to the hash of
that invented text, and `restored_content_hash` to the real source row's
hash. Both self-consistency checks then pass, but the invented snapshot
was never the source row's body. The field is client-editable; it cannot
be authorization.

Repair (backend-defined, verifiable allowed transformation; no new schema,
no new table, no trust in any editable field):

- `restored_content_hash` is treated as untrusted metadata only. It is
  never compared to a row hash for authorization, and restore's fix-up
  overwrites it from the final materialized row. The struct field, its
  format check (64-char lowercase hex, artifact items only) and the UI
  parser stay, so older bodies still parse, but the field grants nothing.
- Pristine case (`row_hash == content_hash`): require, in addition to the
  existing hash checks, exact equality `entry.content == row_body`. A
  generation-time snapshot is verbatim source body, so this holds for every
  legitimate pristine item and cannot be bypassed by a self-consistent hash.
- Divergent case (`row_hash != content_hash`): refuse unless the source
  row body is an allowed restore transformation of the snapshot content,
  checked by a new helper in `restore.rs` against the source package's own
  rows (not against any body field):
  - Both bodies must parse as JSON with the same `format`, and that
    format must be one whose body restore is known to transform:
    `anansi-collider-experiment`, `anansi-evaluation`,
    `anansi-product-spec`, `anansi-source-package`, `anansi-deliverable`,
    `anansi-source-review`, `anansi-specification-approval`,
    `anansi-transfer-approval`, `anansi-forge-concept`,
    `anansi-opportunity-set`. Any other format, or any non-JSON body,
    must be byte-identical to pass — restore does not transform it.
  - Every field except an explicit allowlist must be byte-identical
    (including nested structures). The allowlist is path-scoped, not
    field-name-scoped, so a nested snapshot inside a specification cannot
    borrow a top-level field's permission.
  - Allowed id paths (full root paths, `*` for array indices):
    `memoryContext.*.memoryId`, `memoryContext.*.revisionId`,
    `projectContext.projectId`, `projectContext.items.*.memoryId`,
    `projectContext.items.*.revisionId`, `projectContext.items.*.artifactId`,
    top-level `sourceArtifactId`, `specArtifactId`, `packageArtifactId`,
    `parentPackageArtifactId`, `sourceExperimentArtifactId`,
    `conceptArtifactId`, `rubricArtifactId`, and
    `sources.*.memoryId`, `sources.*.sourceId`, `sources.*.revisionId`.
    A differing id must be 32 lowercase hex, must be present in the
    source package (artifact, memory, revision or workspace id), and the
    original id must not be that package id — a genuine remap always
    replaces an old id with a new package id.
  - Allowed hash paths: top-level `packageContentHash`,
    `specContentHash`, `conceptContentHash`, `sourceExperimentContentHash`,
    and `projectContext.items.*.restoredContentHash` (presence, absence
    or value change). A differing hash must be 64 lowercase hex and must
    equal the source package row's `content_hash` for the artifact
    referenced by the transformed body's own id+version fields for that
    hash (package/spec/concept/experiment, or the item's artifact
    id+version for a restored item hash). This is the verifiable part:
    the hash is recomputed from a package row, never copied from the
    experiment item.
  - Anything else — invented text, a changed title, combination, brief,
    assessment, file content, or a hash that does not match its referenced
    package row — refuses the whole restore before destination
    publication, exactly as other validation failures do.

Why this preserves legitimate repeated restore: after a first restore, a
target body differs from its original snapshot only by the remap and
fix-up steps above (id substitutions and pinned-hash rewrites to the
restored rows' hashes). The second restore's source package contains those
restored rows, so the transformed body passes the same check. A third
restore works the same way, because the check is always snapshot-versus-
current-package-row, never snapshot-versus-a-stored restored field.

Explicitly not done: hiding the generic editor, deleting
`restoredContentHash` (older bodies still carry it), trusting another
editable field, or weakening original-binding validation (workspace,
project, type, version, privacy, sealed refusal, snapshot-hash and
row-hash checks all stay).

Tests (in addition to the unchanged reviewer probe): generic
register/revise attacks — a revised note snapshot with invented content
and the real row hash (the reviewer case, via revise), a revised
experiment item pointing at a different legitimate artifact body, and a
snapshot whose pinned hash field is set to a non-package hash — all must
refuse with no destination published; valid controls — pristine item,
and a genuinely restored experiment/concept chain restored twice — must
pass. Existing restore unit tests for fabricated restored hashes stay
and must still pass (they are now refused by the transform check, not by
the old field comparison).

## R2 — fix-ups must converge against final materialized rows

Defect: `restore_snapshot` ran fix-ups once, in a fixed order
(approval, deliverable, review, experiment-project, transfer-experiment,
transfer-receipt). The experiment-project pass recorded a concept's hash
before the transfer-experiment pass rewrote that concept's body (its
pinned experiment hash follows the restored experiment row), so the
recorded hash went stale. The first restore succeeded; the second refused
because the stale hash matched neither the original nor the final row.

Repair: replace the single ordered sequence with a bounded global
fixpoint loop over the prepared `artifact_versions` rows, in
`restore_snapshot` only:

- One round runs all six fix-up passes in the existing order.
- After each round, compare the prepared rows' bodies and content hashes
  with the previous round. If nothing changed, the rows are final and
  publication proceeds.
- Repeat for at most `values.len() + 1` rounds. Each round propagates a
  dependency at least one step along a chain, and the longest possible
  chain is bounded by the number of artifact rows; a chain longer than
  that, or a cycle that keeps changing hashes every round, fails closed
  with `InvalidRequest` instead of looping or publishing stale pins.
- No fix-up logic changes: each pass still sets pinned fields only to
  the current prepared row hashes of the referenced artifact/version,
  and still fails closed when a referenced row is absent. The
  experiment-project pass keeps its own internal fixpoint for chained
  experiments; the global loop is what lets its results flow through
  transfer, receipt, deliverable, review and approval passes and back
  again.

This covers, not just the reviewer concept case: selected concepts,
receipts, experiments and evaluations (whose bodies change only by
remap), deliverables, reviews, specifications, source packages and
opportunity sets, and chains such as experiment → concept → experiment
→ experiment-item. Unsupported cycles are detected by the round cap,
not by a single-pass reorder: a test with a cyclic experiment/context
graph (built by revising two experiments to reference each other in the
source package) must refuse rather than publish or hang.

Tests (in addition to the unchanged reviewer probe): the reviewer
concept-context twice-restored case; an existing experiment/evaluation
chain restored twice; a selected deliverable and a selected
specification restored twice with Forge admission of a transferred
concept after the second restore; a cyclic reference graph refusing
within the cap. The WP1 double-restore battery (experiment-as-context,
real evaluation-as-context, transferred concept Forge-admissible) must
still pass.

## R3 — instance-scoped overview, result and disclosure targets

Defect: `ColliderResultView` and `ColliderPipelineOverview` used global
fixed ids (`collider-*-heading`, `providers-disclosure-title`). The live
Collider panel stays mounted while a saved experiment opens in Artifacts,
so a saved overview link `#collider-combinations-heading` resolved via
`document.querySelector` to the live panel's heading, not the saved one.
The existing browser test scoped its locator to the saved overview but
never checked the link's actual target.

Repair, in `packages/ui/src/Collider.tsx`,
`packages/ui/src/ColliderOverview.tsx` and `packages/ui/src/Providers.tsx`:

- `ColliderResultView` and `ColliderPipelineOverview` gain an instance
  scope. The six section headings are built from slugs
  (`understanding`, `primitives`, `mappings`, `combinations`,
  `feasibility`, `productization`) as
  `collider-{scope-}{slug}-heading`, where the live panel uses an empty
  scope (legacy ids unchanged, so the live URLs and locators in the
  reviewer probe and existing specs keep working) and a saved view uses
  scope `saved-{artifactId}-v{version}` (stable, from the artifact record
  already used as the saved view's React key). Each result section's
  `aria-labelledby` matches its own heading id, and each overview link's
  `href` matches its own result heading id.
- `ColliderExperimentView` receives the artifact id/version from
  `Projects.tsx` and passes the saved scope to both its overview and its
  result view. The live `ColliderPanel` passes no scope.
- The collider disclosure link is scoped the same way: the live panel's
  `DisclosurePanel` receives a collider scope so its heading id becomes
  `collider-providers-disclosure-title` and the live overview's
  provenance link points to it, instead of to the first
  `providers-disclosure-title` on the page (Forge/Idea panels keep the
  legacy id when no scope is given). The saved experiment view renders
  no disclosure link, unchanged.
- Deliberate, documented exception: the overview section's own pipeline
  heading (`collider-pipeline-heading`) is not a link target and is left
  at its legacy id in both instances, because the reviewer probe locates
  the saved overview by
  `section[aria-labelledby="collider-pipeline-heading"]` inside the
  Artifacts region, and changing it would break that unchanged probe on
  rerun. Navigation correctness is carried by the six section targets,
  which are scoped; the pipeline heading duplicate does not affect any
  link target. This exception is stated in the code comment so a future
  change does not silently reintroduce it.

Tests (in addition to the unchanged reviewer probe): a new browser spec
with live and saved results carrying different content and versions
mounted simultaneously — assert, by DOM query (not href string alone),
that the saved overview's combinations link resolves to a heading inside
the Artifacts region whose section contains the saved combination text,
that the live overview's link resolves to the live section with the live
text, and that keyboard Tab/Enter from the saved overview's link lands
on the saved heading (URL fragment is the saved scoped id). The existing
overview spec and the reviewer anchor spec must pass unchanged.

## Verification order

1. Focused probes first (Rust `review_` filter; browser anchor spec),
   then affected suites sequentially: collider, transfer, forge_restore,
   backup_restore, node UI/contracts.
2. Strict Clippy (`--workspace --all-targets -D warnings`), formatting,
   contracts `--check`, TypeScript, docs validation, API/web builds.
3. Broaden once after focused acceptance passes: full core
   `--no-fail-fast -- --test-threads=1` with the known live-Ollama
   `semantic_workflow` environment failures kept distinct, providers+API,
   and the affected browser specs one file per invocation. No scale or
   model benchmark campaign.
4. Update `docs/BUILD_STATUS.md`, `docs/RECOVERY.md` and
   `docs/REQUIREMENTS_MATRIX.md` precisely: repair tip, before/after
   probe counts, what still needs independent Windows rerun, and the
   remaining gates (Windows verification of this repair, M7 native
   desktop acceptance + preview promotion, M5 production gates, G1 audit,
   §32 sandbox; M8 not complete).
