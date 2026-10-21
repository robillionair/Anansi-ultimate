# M6 backup/recovery repair — addendum

Date: 2026-10-21 · Branch `muse/m6-backup-recovery` from the
R14 tip `bb7cc24` (the R11 preview commit `f6cecd2` is an
ancestor of R14 and was NOT cherry-picked again; its Tauri
permissions and preview launcher are untouched). Design:
`docs/handoffs/M6-backup-recovery-design.md`. All results
below are **Linux (this VM)**; Windows verification of this
checkpoint is the owner's / independent reviewer's step —
nothing here claims it.

## The four findings, reproduced before repair

The new suite `crates/core/tests/backup_artifacts.rs` was
written against the unrepaired tree first:

- Workspace round trip failed at restore pairing — the
  restored workspace contained **no artifacts at all**
  (findings 1 + 3: `snapshots::TABLES` omitted
  `artifact_versions`; `restore_snapshot` never imported it).
- Application backup failed at creation with
  `InvalidRequest` from `execute_backup`'s own
  self-verification, on a database that had been searched
  (FTS present) and semantically indexed (embeddings table
  present) (finding 2). This matches Antigravity's
  R12-DEF-02 exactly, including the `memory_embeddings`
  contribution their report identified.
- Legacy schema-8/9 packages were refused at upload finish
  (finding 4, Antigravity R12-DEF-04).

Antigravity's evidence branch `codex/r12-windows-acceptance`
(tip `02bec968`, reproduction commit `549371c9`) was fetched
verbatim and is preserved unmodified; their defect catalog
(R12-DEF-01…04) maps 1:1 onto the four findings. Their
reproduction tests assert the defects exist on R12; the
success side is this branch's `backup_artifacts` suite.

## What changed

**Backup creation** (`snapshots.rs`): `artifact_versions`
joins the workspace copy list (bodies live in the rows; no
new archive members). A publish-time recheck refuses a
plaintext backup when any included artifact is `sealed`,
mirroring the memory privacy recheck.

**Schema verification** (`snapshots.rs`): the monolithic
"must equal a fresh CURRENT_SCHEMA database" comparison is
replaced by four named groups, each built by executing the
same DDL the runtime executes (shared constants, so they
cannot drift): **core** (migrations 0001–0008 + 0010,
required, exact), **vault** (0009, optional), **fts** (the
`memory_body_fts` DDL, optional), **embeddings**
(`memory_embeddings` + index, optional). Every catalog
object must belong to a group with an identical definition;
an optional group must be complete or absent; anything
else fails closed. This does not ignore derived structures
— it names them exactly. One documented exception:
`memory_embeddings` is compared by column fingerprint, not
CREATE text, because `vector::ensure_embeddings_table` has
a supported in-place ALTER history for that table.
`search::FTS_DDL` is now the single source for the FTS
definition used by both the runtime and the verifier.

**Artifact validation in verification**: every
`artifact_versions` row in the staged catalog is checked —
ids well-formed, `content_hash == sha256(body)`, privacy
in {standard, private} (sealed refuses a plaintext
package), versions contiguous from 1, project is a memory
of the same workspace, generator null or a JSON object,
`source_references` a string array whose `artifact://`
entries resolve to contained artifact versions (malformed
`artifact:` entries fail), and evaluation bodies whose
source/rubric ids + versions resolve to contained rows.

**Restore** (`restore.rs`): artifact ids join the identity
map; `artifact_versions` rows are imported with
workspace/artifact/project ids remapped. Reference
semantics (as designed): contained `artifact://`
references are rewritten to restored ids in
`source_references` and in evaluation bodies
(`sourceArtifactId`/`rubricArtifactId`, via
`evaluations::remap_evaluation_references`), with
`content_hash` recomputed over the rewritten body so the
registry integrity check holds; a reference to an artifact
not in the package fails the restore closed. All other
bodies are preserved byte-for-byte; `memory://` and other
non-artifact references pass through intact. Provenance is
preserved via `restore_origins` for every entity,
artifacts included. Restoring the same backup artifact
twice is refused as a conflict naming the workspace it was
already restored as (no duplicate imports from retries;
re-importing the package as a new artifact is the explicit
path to another copy). Derived state: archived FTS and
embedding data is never imported; inside the import
transaction the destination FTS is rebuilt from the
restored canonical revisions via the existing
`search::sync_index_tx` (which applies the SearchBoundary
privacy rule). Embeddings are not rebuilt (they require a
model); a restored workspace is semantically unindexed
until explicit indexing, like a fresh one.

**Legacy packages** (`snapshots.rs`): a package declaring
schema 8 or 9 — with the catalog `user_version` in
agreement — is validated structurally against the exact
legacy object set (0001–0008 for v8, plus the vault group
for v9; fts/embeddings optional-exact), with
application_id, integrity and foreign-key checks all
passing **before** anything changes. Only then is the
isolated staging copy migrated using the storage
migrator's own step (apply 0010, set version 10) and the
standard v10 verification runs on it unchanged. Declared
versions outside {8, 9, 10}, declared/actual disagreement,
unknown objects and corruption fail closed. The source
package is never modified. Fixtures in the test are built
with the actual older format: catalogs created from the
schema-8/9 migration files (including the vault
maintenance-window behaviour a real v9 database exhibits),
populated from a real workspace's rows, packaged with the
current tar/manifest code.

## Linux evidence

- `backup_artifacts` (new): 5/5 — workspace round trip
  (idea set, rubric v1/v2, one evaluation per rubric
  version, archived project, `memory://` preservation,
  originals untouched, restored archived project refuses
  writes); application round trip from a used database
  (FTS searched + embeddings table present): creation,
  verification and restore all succeed, FTS rebuilt under
  the new workspace id, zero rows under the old one;
  legacy v8 + v9 restore, and declared 7 / 11 /
  declared≠actual refused; tampered body / dangling
  evaluation reference / sealed row refused with an
  untouched control package previewing fine; repeat restore
  refused as a named conflict with no duplicate import,
  and a failed (name-taken) restore importing nothing.
- Existing suites (focused run): backup_restore (incl.
  the settled same-name refusal), artifacts_registry,
  evaluations, ideas_generation, ideas_validation —
  27 passed / 0 failed. Core lib unit tests 34/34.
  Providers 120/120. API 24/24. UI/contracts (node):
  47/47.
- Browser e2e (ideas, evaluations, artifacts,
  app_checkpoint, providers): 5/5.
- Clippy (`-D warnings`), fmt, tsc, contracts:check, docs
  validation: clean.
- Full anansi-core suite: **193 passed / 6 failed of 199**
  (194 pre-repair tests + the 5 new ones). The 6 failures
  are exactly the known set: the 4 live-Ollama
  `semantic_workflow` tests (no Ollama at 127.0.0.1:11434
  on this VM) and the 2 `vault_increment2` tests
  investigated below. The corrected accounting from the
  R13 addendum stands: the anansi-core suite held 187
  tests before Slice 3, 194 before this repair — earlier
  ~497/506 aggregate figures remain retracted.

## The two vault_increment2 failures — investigated

`canonical_history_graph_epoch_rotation_and_explicit_rollback`
and `deterministic_process_crash_cutover_and_rollback_matrix`
fail on this branch, as they did at R13/R14. The R13-era
"load-sensitive" classification is **retracted**; the
evidence now is:

- Both failures are `user_version` assertions encoding the
  pre-0010 schema ladder: the crash matrix expects a
  PreJournal rollback to land at version **8** and observes
  **10**; the canonical-history test expects **9** after a
  vault sequence and observes **10**.
- Root cause: M6 Slice 1 (migration 0010) advanced the
  storage ladder to v10, so fixture databases are created
  at v10 and vault rollback/sequence endpoints land at 10.
  This is consistent with the approved Slice 1 design (the
  vault catalog accepts {8, 9, 10}); another test in the
  same file asserts `user_version == 10` after Core usage
  and passes.
- Baseline comparison on this same VM: at `fc2540f`
  (pre-0010 M2–M3 integration tip) the suite passes **8/8**;
  at `cb8c1cb` (first post-Slice-1 checkpoint) the failures
  reproduce identically; on this branch they are
  deterministic across repeated isolated runs on a calm
  machine — not load-dependent.
- Disposition: pre-existing since Slice 1, **not** caused
  by this repair (no vault code is touched by it). The two
  assertions belong to the M2.7 vault acceptance suite;
  updating version-ladder expectations is a vault-domain
  decision, so the tests are left **unchanged** here and
  the failures are reported, not hidden. No test was
  weakened.

Also recorded for honesty: during this session the VM's
/tmp (a 512 MB tmpfs) filled completely from a scratch
baseline build, which made several unrelated suites fail
spuriously with storage/write errors (including one
snapshots corruption test and two API tests). After
freeing the space, all of those suites pass; the counts in
the delivery report are from the clean rerun.

## Antigravity checklist mapping

- Artifact versions preserved with exact sequences and
  original hashes — yes for every artifact whose body
  embeds no references; evaluations are the one defined
  exception: their bodies are remapped to the restored
  source/rubric ids and the hash recomputed (a hash cannot
  both cover a remapped body and stay original). Original
  bodies and hashes remain in the source workspace and in
  the backup package itself.
- Restored project ownership remapped; cross-workspace
  isolation post-restore — yes (registry reads stay
  workspace-scoped; ids are freshly minted; the archived
  project's restored artifacts stay attached and refuse
  writes while archived).
- Idea/rubric/evaluation references survive —
  yes, per the reference semantics above; `memory://`
  references preserved intact (tested).
- Application backup succeeds with FTS + embeddings
  present — yes (tested from a used database).
- Legacy schema-8/9 ingestion via the upload flow —
  yes (tested end to end through upload-begin/part,
  preview and restore).
- Corruption/interruption resilience — yes (tamper,
  truncation-class and dangling-reference packages refused
  before any mutation; failed restore imports nothing;
  repeat restore cannot duplicate).

## Still open (unchanged by this repair)

M5 pending checks (Windows native/manual acceptance,
live-provider verification, G1 external audit, remote CI);
Windows verification of R13 + Slice 3 and of this
checkpoint; preview promotion (awaits Windows verification
of the repaired checkpoint); M6 Slice 4 (awaits owner
decisions). Not merged to master, not pushed, no CI.
