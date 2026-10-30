# M8 resumed-work lineage repair — after record

Branch `muse/m8-lineage-repair`, from reviewer tip `e03c9bde`
(incoming `b6a3251c` plus reviewer probe/docs only). Design committed
before code: `docs/handoffs/M8-lineage-repair-design.md`. Reviewer
probe unchanged; R1 anti-forgery guards, R2 global fixpoint and R3
scoped anchors preserved. Snapshot bodies are never rewritten.

## Before (reproduced on Linux, same as Windows)

- Reviewer probes: 4 passed / 1 failed. The four prior probes pass;
  `review_new_work_after_restore_survives_further_restore_generations`
  fails at the C → D restore with `InvalidRequest`. This is a
  defect-reproduction pass, not acceptance.

## Cause

The prior round compressed lineage to one ultimate-original →
current pair per entity (A → C). A snapshot authored in B names
B-era ids, so C → D validation needs the B → C pair, which no
longer existed. A-era and B-era snapshots coexist legitimately.

## Repairs

- Restore now writes the full alias set: for each restored entity,
  one `restore_origins` row per generation that named it — the
  latest hop plus every prior alias from the source package's own
  origins rows. Validation is unchanged (a single exact pair
  lookup), so it now finds A → C, B → C and every other genuine
  generation, while wrong, bogus and foreign pairs still refuse.
- Schema change, done as a migration: `restore_origins` key
  widened from `(workspace_id, new_id)` to
  `(workspace_id, new_id, original_id)` in new migration
  `0011_restore_lineage.sql`; `SCHEMA_VERSION` 10 → 11. Alias
  counts grow linearly with restore history (one row per
  generation per entity); validation is one indexed lookup, so
  no cycle risk is introduced.
- Backup compatibility: snapshot verification accepts v10 as a
  legacy version — the v10 catalog is validated exactly against
  a pre-0011 schema group, then only its isolated staging copy
  is migrated to 11 (0011 step only) and re-validated; the
  package itself is never modified. v8/v9 staging migration
  applies 0010 then 0011. Live databases migrate on open with
  existing origins rows preserved verbatim.
- The previous round's `ultimate_origin` helper is replaced by
  `lineage_aliases`; the already-restored guard (keyed on
  `backup_id`) is untouched, and repeat-backup conflict behavior
  is covered by the existing backup_artifacts and
  opportunities_restore suites.

## Tests added

- Restore lib: alias collection (every prior generation, wrong
  workspace, missing table) and alias-set transform acceptance
  (A → C and B → C accept, unlisted X → C refuses).
- Snapshots lib: v10/current schema groups differ exactly on the
  lineage key; a v10 staging catalog migrates to 11 preserving
  its origins row and admitting a second alias row.
- Live-DB migration (entities_roundtrip): a v10 database
  reopens as v11 with its origins row preserved and a second
  alias row insertable for the same (workspace, new) pair.
- Integration (collider, public API only): a chain test with
  A-era coexistence (an A experiment selecting the A
  experiment), new work at B selecting the restored
  experiment, new work at C selecting the restored experiment
  again, and a further restore after each step
  (A → B → C → D → E), asserting every restore succeeds and
  all four experiments survive to E.
- The schema-version assertion in entities_roundtrip moves
  10 → 11 with its comment; no assertion was weakened.

## After (sequential, Linux)

- Reviewer probes: 5/5 (four prior + R4, unchanged).
- Collider 41/41 (39 + R4 probe + chain test), transfer 19/19,
  forge_restore 4/4, backup_restore 7/7, backup_artifacts 5/5
  (includes repeat-restore conflict), opportunities_restore 3/3
  (includes repeat-restore conflict), entities_roundtrip 7/7,
  restore/snapshot lib tests pass (lib total 74/74), node
  UI/contracts 90/90.
- Browser specs, one file per invocation, 23 tests passing: the
  four Windows-passing specs (review anchor, instance anchors,
  project context, integrated double restore) plus collider,
  collider-memory-context, collider-memory-acceptance (2),
  collider-picker-reliability (5), collider-pipeline-overview,
  transfer, transfer-identity, transfer-concurrency, forge,
  forge-review, forge-revision, forge-deliverable, backups, export.
- Strict Clippy `--workspace --all-targets -D warnings` clean
  (one unused-variable in the new chain test found during this
  round and fixed in the implementation commit), fmt clean,
  contracts `--check` clean, TypeScript clean, docs validation
  PASS, API/web builds pass.
- Broadened once: the first full-core run surfaced seven
  non-environment failures beyond the known live-Ollama
  `semantic_workflow` four, all caused by the schema bump
  itself and all fixed in a follow-up commit, not by
  weakening any check: three vault gates still allowlisted
  only versions 8/9/10 (`connection`, seal gate, `upgrade`),
  and current-version assertions pinning 10 in jobs_recovery,
  title_search and vault_increment2 (including the crash
  matrix's current-ladder case, renamed 10 → 11). The
  vault_increment2 PoisonError failures were cascades from
  the first UnsupportedSchema panic through a shared test
  lock, not separate defects. After the fix, jobs_recovery
  7/7, title_search 1/1, vault_increment2 9/9 and
  vault_runtime 2/2 pass. The broad suites were then
  re-run to completion: full core `--no-fail-fast
  -- --test-threads=1` 363 passed / 4 failed — reconciliation
  357 (prior repair) + R4 probe + chain test + migration test
  + alias-set unit test + 2 snapshot unit tests = 363,
  failures only the known live-Ollama `semantic_workflow`
  environment failures; providers+API 144/0. Evidence logs
  are in the delivery ZIP.
- Not rerun: the full canonical e2e battery end to end — only the
  affected specs above, as in prior rounds. A real v10 backup
  restored end to end is a Windows rerun checklist item (the
  mechanism is unit- and migration-tested here, not yet proven
  against a packaged v10 backup on either platform).

## Compatibility note (fail-closed, kept distinct)

This repair does NOT make pre-R1b multi-hop packages provable:
packages restored twice or more under pre-R1b code carry only the
latest hop's origins while snapshots name ultimate originals, so
a further restore still refuses before publication. Only packages
made entirely under the alias design (or with at most one prior
hop) carry provable lineage for every generation they contain.
That legacy limit is separate from the R4 bug and is not claimed
fixed.

## Still open (Linux evidence does not close these)

Independent Windows rerun of this repair by ChatGPT (checklist in
the delivery MANIFEST, including a v10 backup restore);
Windows verification of Slice 4; M7 native desktop acceptance +
preview promotion; M5 production gates; G1 audit; §32 execution
sandbox. M8 is not complete: 3D, URL fetching and document
ingestion remain unfinished.
