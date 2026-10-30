# M8 R1 provenance repair — after record

Branch `muse/m8-r1-provenance-repair`, from reviewer tip `7388b7fa`
(incoming `d0a6c1e1` plus reviewer probes/docs only). Design committed
before code: `docs/handoffs/M8-r1-provenance-repair-design.md`. Reviewer
probes unchanged; no application code from the previous repair was
reverted (R2 global fixpoint and R3 scoped anchors intact).

## Before (reproduced on Linux, same as Windows)

- Reviewer probes: 2 passed / 2 failed. The original two restore
  probes pass; the two new R1 probes fail (forged context-free
  structured snapshot accepted; fabricated project id without any
  restore mapping accepted). These are defect-reproduction passes,
  not acceptance.

## Repairs (all in `crates/core/src/restore.rs`)

- R1a: the collider branch no longer returns success early when the
  original snapshot lacks `projectContext` or its items. Absent/
  absent continues to the final whole-document equality, which is
  now the only gate for result text, controls, beams and every other
  field; any absent/present mismatch refuses. Audited: `check_id_array`
  results are caller-gated and still reach final equality, id/hash
  field checks refuse on mismatch, and all ten format branches fall
  through to final equality.
- R1b: a differing id is allowed only when the source package's own
  `restore_origins` table records that exact original → restored
  pair for the owning workspace. Package membership, matching text,
  editable fields and client claims are never consulted. A missing
  table means no mapping, so a differing id refuses. Pinned-hash
  checks are unchanged (hash must equal the package row hash for
  the transformed body's own referenced id+version).
- Multi-hop: restore now records ultimate origins. When writing
  each new `restore_origins` row it resolves the ultimate origin
  through the source package's own origins rows, so destination
  packages always carry direct ultimate-original → current-id
  pairs and hop 3+ validation stays a single pair lookup. No schema
  or backup-format change; no history rows are copied, so the
  already-restored guard (keyed on the backup being restored) is
  unaffected.
- Audit correction in the same function: the collider branch's
  `restoredContentHash` check now looks the hash up under the
  transformed body's own artifact id (as `check_hash_field` already
  does), not the id after normalization to the original — in a
  restored package the original id is absent, so the old lookup
  could not match a legitimate restored row. Covered by a new
  chained-experiment unit test.

## Tests added (restore lib: 4 → 10)

Context-free forgery (result/beam/control/invented sections),
project-context absent/present/empty and exact-pair/wrong-pair/
wrong-workspace/no-pair cases, memory-context and opportunity-source
absent/present/empty and pair cases, all ten formats (identical
accepts, invented text refuses, paired remap accepts, unpaired
refuses, hash-only forge-concept control), chained-experiment
restored item hash (accepts legitimate, refuses wrong hash), and
ultimate-origin resolution (through source rows, wrong workspace,
missing table).

## After (sequential, Linux)

- Reviewer probes: 4/4 (original two + two new).
- Collider 39/39 (37 + 2 new probes), transfer 19/19, forge_restore
  4/4, backup_restore 7/7, restore lib 10/10, node UI/contracts
  90/90.
- Browser specs, one file per invocation, 23 tests passing: the
  four Windows-passing specs (review anchor, instance anchors,
  project context, integrated double restore) plus collider,
  collider-memory-context, collider-memory-acceptance (2),
  collider-picker-reliability (5), collider-pipeline-overview,
  transfer, transfer-identity, transfer-concurrency, forge,
  forge-review, forge-revision, forge-deliverable, backups, export.
- Strict Clippy `--workspace --all-targets -D warnings` clean
  (one collapsible-if found during this round and fixed in the
  implementation commit), fmt clean, contracts `--check` clean,
  TypeScript clean, docs validation PASS, API/web builds pass.
- Broadened once: full core and providers+API totals are recorded
  in the delivery MANIFEST (see evidence logs); the only expected
  failures are the known live-Ollama `semantic_workflow`
  environment failures.
- Not rerun: the full canonical e2e battery end to end — only the
  affected specs above, as in prior rounds.

## Compatibility note (fail-closed, documented)

No schema or backup-format change. Backups made before this change
restore as before when pristine or hash-only divergent. Packages
restored once under old code carry single-hop origins that already
prove their only hop. Packages restored twice or more under old
code carry only the latest hop's origins while their snapshots
still name ultimate originals: a further restore under new code
refuses fail-closed before any destination publication. The package
itself is untouched. Packages produced by new code carry ultimate
origins, so 2nd/3rd/Nth restores validate.

## Still open (Linux evidence does not close these)

Independent Windows rerun of this repair by ChatGPT (checklist in
the delivery MANIFEST); Windows verification of Slice 4; M7 native
desktop acceptance + preview promotion; M5 production gates; G1
audit; §32 execution sandbox. M8 is not complete: 3D, URL fetching
and document ingestion remain unfinished.
