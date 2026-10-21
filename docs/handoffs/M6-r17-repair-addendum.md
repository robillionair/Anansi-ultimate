# R17 repair addendum — exact-revision provenance at restore

Date: 2026-10-21 · Branch `muse/m6-r17-repair` over the
Slice 4 checkpoint `7253b56`. Repair only — no new feature.

## The defect (Windows-confirmed)

Independent Windows verification of R17 passed
`opportunities_generation` 6/6, the opportunity unit tests
5/5, `opportunities_restore` 1/1, UI/contracts 51/51,
TypeScript, strict Clippy and docs — but a focused
diagnostic confirmed one defect:
`opportunities::remap_opportunity_references` rewrote
`memoryId`/`sourceId` and concept citations at restore,
but left `sources[].revisionId` unchanged, so a restored
opportunity set pinned the *source* workspace's revision
id, which resolves to nothing (or worse, the wrong thing)
in the restored workspace. The existing restore test
passed because it asserted memory ids only.

## The fix

- `remap_opportunity_references` now remaps `revisionId`
  through the same restore identity map (which already
  covers every `memory_revisions` id in the package).
- Map membership alone is not trusted. `restore_snapshot`
  builds `revision_owners` — old revision id → (old memory
  id, old workspace id) — from the package's own
  `memory_revisions` rows, and captures each artifact row's
  original workspace before the generic id remap. A pinned
  revision that is **missing** from the package, **belongs
  to a different memory**, or **belongs to a different
  workspace** fails the restore closed during preparation,
  before anything is written.
- Original identities remain recoverable through the
  existing recovery-provenance mechanism: `restore_origins`
  records original → restored for every mapped id,
  revisions included; nothing new was invented and no
  provenance was dropped. Changed bodies still have their
  content hash recomputed in `remap_artifact_row`; supplied
  source text is byte-identical, as before.

## Regression coverage

- Unit (in `opportunities.rs`): revision remaps with its
  memory; missing-from-map, unknown, different-memory and
  different-workspace revisions all fail closed.
- `opportunities_restore` is now 3 tests:
  1. The original identity test, strengthened in place —
     it now asserts the restored body's `revisionId` equals
     the restored memory's revision (and not the original),
     and resolves it to the original text. No existing
     assertion was weakened.
  2. A full round trip with a real (synthetic-provider)
     discovery run: memory revised A → B, discovery runs
     against B (wire capture proves B, not A, was sent),
     memory revised to C, backup, fresh restore. The
     restored set pins the restored counterpart of B —
     resolved by (restored memory id, restored revision id)
     it returns **exactly B's text, not C's** — the
     original workspace is untouched, evaluation references
     resolve on the restored set, and a repeat restore of
     the same uploaded backup is refused via
     `restore_origins`.
  3. Missing and mismatched revision references refuse the
     whole restore (no workspace materialized), with an
     own-revision control case that restores cleanly.

## Verification (Linux, this machine)

- `opportunities_restore` 3/3; opportunity unit tests 5/5.
- Full `anansi-core` suite: see BUILD_STATUS for the exact
  totals on the final tree (the 4 live-Ollama
  `semantic_workflow` tests remain environmental failures
  on this machine).
- Clippy `--all-targets -D warnings` and fmt clean.

**Windows verification of this repair is pending.** The
R17 Windows results above stand for the unrepaired tree;
this checkpoint's changes are Linux-verified only. M5
production gates remain open and unclaimed.
