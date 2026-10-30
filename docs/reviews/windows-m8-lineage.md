# M8 lineage repair — Windows acceptance review (2026-10-30)

Incoming `06a3f573af1cda51dab9fa1e0959b93c5897fd6a`; branch `codex/review-m8-lineage`, isolated review checkout on E:. Prior five reviewer probes and browser probe preserved. Owner data/previews and Antigravity worktree untouched.

## Acceptance before execution

- All five previous reviewer probes pass unchanged; the resumed-work case must now survive C→D.
- Verify new full-alias chain with new work at B/C and original-era coexistence, migration/unit tests and vault legacy/current crash matrices where schema 11 changed gates.
- Create an actual schema-10 package through the saved previous API binary, independently inspect its real catalog, restore through schema-11 public operations, verify all memory/artifact versions and original bytes, and preserve repeat-restore conflict. Do not substitute a downgraded current SQL fixture.
- Relevant UI/contracts/static/build checks and affected browser workflows pass. Respect the shared application-test lease; no cloud, downloads, pushes, CI or promotion.

## Identity and outcome

ZIP SHA-256 `3ba10adf7eac21e08c377f6789b57d6115f9d5da50eb7d1f1d53cd5f0cfc2441`; bundle SHA-256 `8c6bdaf40852dc416daaae0520e2b806bbbce4803b94ddc21bee8fe4d10c06ef`. Exact incoming tip and ancestry verify. Prior reviewer probes are preserved. No runtime code was changed in this review.

- Prior reviewer probes: **5/5 passed**, including the previously failing resumed-work C→D recovery.
- Extended new-work-at-B/C chain through A→B→C→D→E: **1/1 passed**, with original and later snapshots preserved.
- Core library tests: **74/74 passed**, including the eleven restore binding/alias/fixpoint tests and snapshot schema checks.
- Live schema-10→11 migration: **1/1 passed**, preserving prior origins and admitting multiple aliases under the new key.
- Genuine schema-10 archive import: **1/1 passed** (details below).
- Vault increment-2: **9/9 passed**, including genuine legacy 8→9 and current 11→11 process-crash matrices, artifact preservation, rotation/rollback, epoch/admission checks (755.08 s total).
- UI/contracts: **90/90 passed**; TypeScript, strict core all-target Clippy, generated-contract drift, formatting/docs and API/web builds passed. Existing ignored-`uint` schema-format and web bundle-size warnings are retained as warnings, not failures.
- Browser on real Windows API/web with synthetic providers: **4/4 passed** sequentially (original saved-anchor probe, instance-scoped DOM/keyboard anchors, canonical project-context workflow, integrated transfer/restart/double-restore). Shared test lease acquired/released normally.

**Verdict: the lineage repair passes focused Windows acceptance.** All planned checks completed without a new failure. In total, 91 reported Rust test cases passed across the six scoped runs (5 reviewer + 74 library + 9 vault + 1 real-v10 import + 1 live migration + 1 extended chain), plus 90 UI/contracts and four browser specs. This is not a full-workspace, native-dialog, cryptographic-audit or production certification claim.

## Genuine v10 backup compatibility — independently exercised

Before compiling the new API, the reviewer preserved the previous Windows API executable (SHA-256 `97d92238fe9e7ad3f82c0d41d805355156d074600f295eb80f37160e5807105a`). Through its public authenticated HTTP operations, an isolated synthetic workspace was created with one project, a memory revised twice and an artifact revised twice. Its normal backup job completed and the archive bytes were retrieved through the artifact endpoint.

Independent read-only archive inspection verifies:

- `catalog.sqlite3` actually has `PRAGMA user_version=10` and the original `(workspace_id,new_id)` primary key.
- Three memory revision rows (project plus both note revisions), two artifact-version rows, integrity `ok`, zero foreign-key violations.
- Archive size 138,240 bytes; SHA-256 `834505445db994e138f662b5a758b40a4a81e1276775b06db94a1cef1dea6ee7`.
- Catalog SHA-256 `b2b9a15b6ff1db14528e9bab7cf74b4cd55b99509fa4163fea4c6bd2f973e59d`.

The added `review_real_schema10_archive_restores_to_schema11` test imports these exact archive bytes via the new public Core upload/preview/restore workflow. It verifies both memory bodies/history and both artifact bodies/versions, remapped project ownership, target schema 11, same-backup `RevisionConflict`, and byte-identical original archive after completion. **It was explicitly executed with `--ignored --exact --nocapture` and passed**; ignored-by-default is only to avoid a missing external-fixture dependency in normal checkouts.

Reproduce with the fixture delivered under `windows-evidence/`:

```powershell
$env:ANANSI_SCHEMA10_ARCHIVE = 'E:/path/to/extracted/windows-evidence/real-schema10-backup.tar'
node scripts/cargo.mjs test -p anansi-core --locked --test collider review_real_schema10_archive_restores_to_schema11 -- --ignored --exact --nocapture
```

The fixture producer and read-only `inspect-schema10-fixture.py` are included with evidence. The large producer executable and its temporary source databases are deliberately excluded from the handoff. An initial fixture-helper route typo (`memory` versus established `memories`) was corrected before creating the successful fixture; its failure log is retained and is not a product failure or acceptance pass. The initial string-format PK check was replaced by structural `PRAGMA table_info` inspection, avoiding DDL whitespace assumptions. No current catalog was downgraded or relabelled to fabricate this control.

## Scope, compatibility and next use

Review additions only: the optional external-fixture acceptance test and evidence/status docs. Source data, older previews, and Antigravity's separate worktree remain untouched. All fixture/build/temp paths are on E:. No provider/cloud inference, owner credentials, downloads, generated execution, push, master merge, CI or preview promotion.

Muse's explicit old-code compatibility limit remains: archives whose historical mappings were already lost before these repairs can refuse fail-closed; this review does not invent missing history or certify all historical packages. The genuine v10 control above is a supported, actual prior-format archive. Old builds must retain separate data roots; schema-11 roots are not downgraded for older executables.

M8 remains incomplete (document/URL inputs and 3D are still deferred); native/manual production gates, G1 audit and remote CI remain open. This review does not rerun the complete workspace or claim the known live-Ollama environment failures have disappeared.

Use the reviewed branch `codex/review-m8-lineage`, incoming runtime `06a3f573a`, for the next integration. Preserve this evidence and prior fixes; do not restart the repaired lineage work without a concrete new failure. Coordinate separate release-tooling integration after its own review; do not overwrite the newer runtime with Antigravity's older baseline.

