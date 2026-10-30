# M8 resumed-work lineage repair — bounded design (before code)

Baseline: reviewer tip `e03c9bde` (incoming `b6a3251c` plus reviewer
probe/docs only). The four prior probes pass; the new R4 probe fails
unchanged. No new feature slice. R1 anti-forgery guards, the R2
global fixpoint and R3 scoped navigation are preserved, not
reverted. Snapshot bodies are never rewritten.

## Cause (verified against the code, not assumed)

Restore writes one `restore_origins` row per entity, and the prior
round compressed it to the ultimate origin (A → C). A snapshot
authored in B names B-era ids, so C → D validation needs the B → C
pair, which no longer exists. A-era and B-era snapshots coexist
legitimately, and neither earliest-only nor latest-only lineage can
prove both. The validator itself is sound: it requires an exact
backend-recorded pair, and that requirement does not change.

## Repair: full alias set, one row per original

At restore time, for each restored entity (source id `s`, fresh id
`f`, owner workspace `w`), write one row per generation that has
named it:

- the self row `(w_new, f, w, s, current_backup)` — the latest hop;
- one row per prior alias from the source package's own origins
  rows for `(w, s)`: `(w_new, f, prior_original_workspace,
  prior_original_id, current_backup)`.

The destination package therefore carries, for every entity, the
complete set of original ids that valid snapshots may still name.
Validation stays a single exact pair lookup
`(original_id, new_id, workspace_id)` — unchanged code, unchanged
strength — and now finds A → C, B → C, and every other genuine
generation. Same-content distinct artifacts stay distinct because
pairs are per id, never per body. Wrong, bogus and foreign pairs
still refuse because the lookup is exact and scoped to the owning
workspace; body-supplied maps and hashes are still never consulted.

## Primary-key constraint, handled explicitly

The current key is `PRIMARY KEY(workspace_id, new_id)`, which
forbids two originals for one current entity — exactly the alias
set above. So the key becomes
`PRIMARY KEY(workspace_id, new_id, original_id)`. This is a real
schema change, done as a migration, not a promise to avoid one:

- New file `crates/core/migrations/0011_restore_lineage.sql`:
  rebuild `restore_origins` (SQLite cannot alter a primary key)
  with the new key, copying all existing rows verbatim.
- `SCHEMA_VERSION` 10 → 11; `CURRENT_SCHEMA` gains 0011; the
  storage migration chain gains `if locked_version < 11`.
- Alias growth is bounded and linear: an entity restored k times
  carries k rows (one per generation that named it), never a
  product of generations, and validation is one indexed lookup.
  No traversal, so no cycle risk is introduced; the existing
  fixpoint cap and pre-validation order are untouched.
- The already-restored guard is unaffected: it keys on
  `backup_id`, and self rows still carry the backup being
  restored. Copied alias rows carry the current backup's id, so
  they cannot falsely trigger (or mask) the guard for another
  backup. Repeat-restore conflict tests must keep passing.

## Backup compatibility (legacy packages, kept distinct)

- New backups (schema 11) verify on the current path; their
  catalogs already carry the new key.
- v10 backups must NOT break: the snapshot verification gate
  gains `10` alongside `8 | 9`, with a legacy branch that
  validates the v10 catalog exactly (new `core10` schema group
  built from the pre-0011 DDL), then migrates only the isolated
  staging copy to 11 (0011 only — 0010 is already present) and
  re-validates against the current group. The package itself is
  never modified. A v10 package's origins rows (one per entity)
  validate exactly as before for their single hop.
- v8/v9 legacy migration extends to current: staging migration
  applies 0010 then 0011 before re-validation.
- Irrecoverably missing history stays fail-closed and is never
  invented: packages restored twice or more under pre-R1b code
  carry only the latest hop's origins while snapshots name
  ultimate originals. Those packages still refuse on a further
  restore, exactly as documented in the prior round. That legacy
  limit is separate from this new-code bug and is not disguised
  as fixed: only packages made entirely under the alias design
  (or with at most one prior hop) carry provable lineage for
  every generation they contain.
- Live databases migrate on open: a v10 live DB keeps every
  existing origins row through the rebuild and can then record
  alias rows for its next restore.

## Tests (focused fixtures first, one representative e2e chain)

- Unit (restore): alias collection returns every prior generation
  for an entity and none for an unknown id; transform accepts an
  A → C and a B → C pair from a two-alias set and refuses an
  unlisted X → C pair and a pair scoped to another workspace.
- Unit (storage/snapshots): v10 live DB migrates to 11 with
  origins rows preserved and a second alias row insertable for
  the same (workspace, new) pair; staging migration of a v10
  catalog reaches 11; the v10 schema group accepts a v10 catalog
  and the current group does not (proving the groups differ
  exactly on the key).
- Integration (collider, public API only): R4 probe unchanged;
  a new chain test with A-era coexistence (an A experiment
  selecting the A experiment), new work at B selecting the
  restored experiment, new work at C selecting the restored
  experiment again, and a further restore after each step
  (A → B → C → D → E), asserting every restore succeeds.
  Forge-transfer admissibility and repeat-backup conflict
  continue to be covered by the existing transfer/forge_restore/
  backup_restore and backup_artifacts suites — rerun, not
  duplicated.
- Existing suites that must stay green unchanged: the four prior
  reviewer probes, collider, transfer, forge_restore,
  backup_restore, restore lib, node UI/contracts, and the
  affected browser specs.

## What this design does not do

No snapshot rewriting, no body-supplied lineage, no membership
fallback, no weakening of pair/workspace/version/privacy checks,
no new backup format (the catalog travels inside the existing
package; only its internal table key changes), and no claim that
old-code multi-hop packages become provable.
