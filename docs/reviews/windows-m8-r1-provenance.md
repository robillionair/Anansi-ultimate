# M8 R1 provenance repair — Windows acceptance review (2026-10-30)

Incoming `b6a3251c4d6fe68c97ca57b461e60f44586850bb`, branch `codex/review-m8-r1-provenance`. All four prior reviewer Rust probes and the browser anchor probe verify unchanged. No runtime changes by reviewer.

## Acceptance before execution

- All four existing reviewer probes and relevant restore unit tests pass on Windows.
- Genuine recovery remains usable when users resume work: restore A to B, generate a new context experiment in B from a restored source, restore B to C, then restore C to D. All generations of snapshot provenance must remain verifiable. The design's ultimate-origin compression must not discard intermediate identities still named by newly authored snapshots.
- Boundary, UI/contracts, static/build and affected browser checks retain their bars.
- Keep Antigravity's checkout and owner previews/data untouched; no cloud, CI, pushes or promotion. Existing older-package fail-closed compatibility limitation remains explicit.

## Identity and focused outcome

ZIP SHA-256 `3964b1135410d59a4aea4657e6b32202a0b64711d8f823f6343b328eb4b7d2ff`; bundle SHA-256 `6defa4837ef08c2df3a4679d8d0cabf9ee45c5601b08ab398fd2e3945e09432a`. Exact tip and reviewed ancestry verify. All prior reviewer tests were compared against `7388b7fa` and are unchanged.

**The four prior restore/provenance probes pass on Windows. One added normal-use recovery probe fails.** Restore unit tests pass **10/10**, UI/contracts **90/90**, and TypeScript, strict core all-target Clippy, contracts, formatting/docs and API/web builds pass. Web retains the existing bundle-size warning; contract generation retains ignored-`uint` schema-format warnings without drift failure.

Browser outcome: **4/4 passed** sequentially (original anchor probe, instance-anchor DOM/keyboard, canonical project context and integrated restart/double-restore). The shared application lease was acquired/released normally.

## R4 — P1: new work after a restore loses its intermediate generation's provenance

The repair's `ultimate_origin` and origins insertion record one earliest-original → current pair per entity. `check_id_change` now correctly requires an exact backend-owned pair, but snapshots can legitimately be created at ANY generation, not only the first one. Collapsing lineage to the earliest ID drops intermediate IDs still named by later-authored snapshots.

Independent public-operation sequence, entirely under this NEW repair (no legacy-code archives):

1. A: create a memory-context experiment and back up the workspace.
2. Restore A into B; confirm the restored source is readable.
3. In B, explicitly generate/save a NEW experiment selecting B's restored experiment as project context. Its immutable generation-time snapshot correctly contains B's identities.
4. Back up B and restore into C successfully.
5. Back up C and restore into D. This refuses with `InvalidRequest`.

Why: when C is built, the source experiment and its memory/revision IDs are recorded as A → C, while the new snapshot inside the B-authored experiment still refers to B. During the next original-binding validation, the required B → C pair is absent. Both A-era and B-era snapshots can coexist legitimately; keeping only the earliest OR only the latest mapping cannot prove both.

Reproduction (expected success, unchanged assertions):

`node scripts/cargo.mjs test -p anansi-core --locked --test collider review_new_work_after_restore_survives_further_restore_generations -- --exact --nocapture`

Actual output: `new work after a restore must remain recoverable at subsequent generations: Err(CommandError { code: InvalidRequest, message: "Use a valid job selection or state transition." })`.

The combined reviewer run reports **4 passed / 1 failed**, 35 unrelated tests filtered, in `review-probes.log`. The four passing cases are the original probes; the one failure is this new valid workflow, not a malformed-package or attack case. No direct SQL writes or archive tampering are used in it.

### Required repair scope

Preserve genuine backend-recorded lineage for ALL identities still named by valid snapshots, including original and intermediate generations. A verifiable lineage chain or alias representation is acceptable; do not weaken the exact identity authority into mere package membership. The existing `restore_origins` primary key is `(workspace_id, new_id)`, so a design assuming several direct original aliases per current entity must account for that constraint explicitly.

Preserve immutable snapshot bodies, exact source/version ownership, original pre-validation, cycle bounds, anti-forgery cases and fresh-destination atomicity. Test new work at B and C, coexistence of A/B/C-era snapshots, at least a further restore after each, approved Forge transfer and repeat-backup refusal behavior. Retain multi-workspace isolation and refusal of fabricated/wrong mappings. Use an appropriately bounded schema/compatibility design if required, rather than promising no schema change at the expense of valid recovery.

This new-code failure is separate from Muse's documented older-code compatibility limit: twice-restored old packages without provable ultimate history may still need explicit fail-closed handling. Do not disguise the current bug as that legacy limit or invent missing historical provenance to accept an old archive.

## Scope and resume

Only the new probe and review documentation were added; runtime code unchanged. No cloud/credentials/downloads/generated execution, pushes, master merges, CI or preview promotion. Owner data/previews and Antigravity's worktree untouched. No full-core rerun or native/security/release certification is claimed; prior boundary passes remain scoped to their recorded trees rather than counted as a fresh rerun here.

Use `codex/review-m8-r1-provenance` (incoming `b6a3251c` plus additive reviewer evidence) for Muse's bounded lineage repair. Preserve all earlier tests and R1–R3 fixes. Evidence: `E:/Anansi/benchmarks/m8-r1-provenance-windows-2026-10-30`.

