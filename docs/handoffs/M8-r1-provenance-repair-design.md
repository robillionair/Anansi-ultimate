# M8 R1 provenance repair — bounded design (before code)

Baseline: reviewer tip `7388b7fa` (incoming `d0a6c1e1` plus reviewer
probes/docs only). Two new probes fail unchanged; the original two
restore probes, R2 fixpoint and R3 scoped anchors are preserved, not
reverted. No new feature slice. No schema change. No weakening of any
existing check: every rule below only narrows what is accepted.

## Defect A — early success skips whole-document equality

In `allowed_restore_transform` (collider branch), when the original
snapshot has no `projectContext`, the code returns
`normalized.get("projectContext").is_none()` immediately — true when
both lack it — before the final `normalized == original` comparison.
The same shape exists for `projectContext.items` when the original
context has no items. Any other field (result text, controls, beams,
combinations) can then differ freely. The probe forges only
`result.sourceUnderstanding` plus its coherent hash.

Fix: no branch in the transform may return true before the final
equality. Absent/present mismatches return false; absent/absent
continues to the final comparison. Audit of every other success path:

- `check_id_array` (memoryContext, opportunity sources): returns a
  bool to a caller that only continues on true and still reaches the
  final equality — safe, but unit-tested for absent/present/empty.
- `check_id_field` / `check_hash_field`: false on any
  absent/present mismatch, true only on equal or verified change —
  safe, and the final equality still runs.
- All other nine format branches: flat id/hash fields, then fall
  through to the final equality — safe, no early true return.
- Final equality remains the only whole-document gate: every field
  not explicitly normalized away must be byte-identical in JSON
  value terms, including empty vs absent (absent != empty array/object
  in JSON value comparison, and absent/present id mismatches already
  refuse).

Unit tests added for absent/present/empty projectContext and items,
absent/present/empty memoryContext and sources, and invented
result/beam/control/text negatives for collider and evaluation
formats, alongside legitimate controls.

## Defect B — package membership is not an identity mapping

`check_id_change` accepts any differing id when the new id exists
somewhere in a package table and the old id does not. In a
never-restored package that pattern also matches a fabricated
original (fake id, never existed) pointing at a real row id. The
probe's fixture has zero `restore_origins` rows and still passes.

Fix: a differing id is allowed only when the source package's own
`restore_origins` table — written by restore itself, never by any
client API, and copied verbatim into backups — contains a row with
`original_id == old`, `new_id == new`, and `workspace_id` equal to
the workspace owning the validated artifact (the entity's owner
workspace). That row is backend-controlled provenance of an actual
restore, scoped to one workspace. Rejected by construction:

- arbitrary package membership (no pair row);
- matching text alone, client claims, editable body fields or maps
  (none are consulted; `restoredContentHash` remains untrusted);
- wrong pairs (pair lookup is exact, not per-id existence);
- wrong kinds/workspaces (workspace scoping plus the existing
  type/project/privacy/row checks; revision ids are covered because
  restore's identity map includes memories and memory_revisions);
- same-content distinct artifacts (pairs are by id, never by body).

Hash fields are unchanged: a changed pinned hash must still equal
the package row hash for the artifact the transformed body itself
references by id+version, and the id in that reference is itself
subject to the pair rule when it differs. A hash-only divergence
with an unchanged id remains accepted only when every other field
is identical and the hash equals the actual row hash.

## Multi-hop restore: ultimate-origin composition, no schema change

The snapshot inside an outer experiment stays verbatim forever, so
at hop N the comparison is ultimate-original ids vs hop-(N-1) row
ids. Copying only the latest hop's origins would break hop 3+.
Instead, at restore time, when writing each new `restore_origins`
row, resolve the ultimate origin through the source package's own
origins rows: if a row exists with `new_id == source_id` and
`workspace_id == source workspace`, the new row records that row's
`original_id`/`original_workspace`; otherwise it records the source
id/workspace as today. Destination packages therefore always carry
direct ultimate-original → current-id pairs, one row per entity,
and validation stays a single pair lookup — no transitive closure
in the validator, no copied history rows, and the existing
already-restored guard (which keys on `backup_id` of the backup
being restored) is unaffected because new rows carry only the
current backup's id.

Migration / backup-compatibility consequences (fail-closed, not
corrupting):

- No table, column or backup-format change. Backups made before
  this change restore exactly as before when their snapshots are
  pristine or their divergences are hash-only with unchanged ids.
- Packages restored once under old code carry single-hop origins
  that already prove their (only) hop, so their next restore
  validates.
- Packages restored twice or more under old code carry only the
  latest hop's origins while their snapshots still name ultimate
  originals: a further restore under new code refuses
  fail-closed before any destination publication. That is a safe
  refusal of an unprovable package, not data loss — the package
  itself is untouched and the refusal names the boundary. This
  limitation is recorded in BUILD_STATUS and the delivery MANIFEST.
- First restore of a never-restored package: origins are empty,
  so any differing id refuses. That is correct — in a
  never-restored package a snapshot and its row are the same
  generation, so only pristine equality (or a hash-only divergence
  with an unchanged id, per above) can be legitimate.

## Latent audit item (same function, false refusal, not a weakening)

The collider branch's `restoredContentHash` check reads the item's
`artifactId` after that field was normalized to the original id, so
it looks the hash up under an id that is absent from a restored
package. The correct reference is the transformed body's own id,
as `check_hash_field` already uses. Fix it to the pre-normalization
(transformed) id and add a chained-experiment unit test proving a
legitimate second restore with a restored item hash passes. If the
test shows the current code already passes, no change is made to
that line and the test is kept as a regression guard.

## What must keep passing

Original reviewer probes 2/2, new probes 2/2, R2 concept probe,
collider/transfer/forge_restore/backup_restore suites, node
UI/contracts, the four Windows-passing browser specs (review
anchor, instance anchors, project context, integrated double
restore), and repeated legitimate restore of experiment/evaluation/
concept chains and Forge-approved transfers. Invalid originals
refuse before destination publication in every case.
