# M6 backup/recovery repair — design and acceptance criteria

Date: 2026-10-21 · Branch `muse/m6-backup-recovery` from the
R14 tip `bb7cc24` (which already contains the R11 preview
commit `f6cecd2` as an ancestor — it is NOT cherry-picked
again; its permissions and launcher are preserved untouched).

Source: the independent Windows review of R12 confirmed four
backup/recovery defects; the integrator confirmed
`snapshots.rs`/`restore.rs` are unchanged in R14. Each
finding was reproduced against the code before repair (see
the addendum for the repro evidence).

## The four findings, as verified in the code

1. **Workspace backups omit artifacts.** `snapshots::TABLES`
   (the per-workspace copy list) has no `artifact_versions`,
   so a single-workspace backup silently drops every project
   artifact — idea sets, rubrics, evaluations.
2. **Application backups fail verification once the app has
   been used.** Verification compares the staged catalog's
   `sqlite_schema` for exact equality with a fresh
   `CURRENT_SCHEMA` database. Real databases legitimately
   contain more: the runtime lazily creates
   `memory_body_fts` (search), `memory_embeddings` (semantic
   indexing) and the vault objects (migration 0009, incl. its
   guard triggers). Any of these present ⇒ strict equality
   fails ⇒ the backup is refused.
3. **Restore ignores artifacts.** `restore_snapshot` neither
   collects artifact ids nor imports `artifact_versions`;
   its column remapper also lacks `project_id`, and nothing
   remaps the references *inside* artifact rows
   (`source_references`, evaluation bodies).
4. **Legacy packages are refused.** Verification requires
   `schema_version == 10` and the v10 schema exactly, so
   schema-8 and schema-9 packages — produced by the actual
   older format (same tar/manifest, older catalog) — cannot
   be restored at all.

## Design

### Schema verification: named groups, exact definitions

Verification stops demanding one monolithic schema and
instead recognises **four named object groups**, each built
by executing the group's own DDL — the same text the runtime
executes, shared as constants so verifier and runtime cannot
drift:

- **core** (required, exact): `CURRENT_SCHEMA`
  (migrations 0001–0008 + 0010).
- **vault** (optional, all-or-nothing): migration 0009.
- **fts** (optional, all-or-nothing): the
  `memory_body_fts` DDL from `search::ensure_fts_table`.
- **embeddings** (optional, all-or-nothing):
  `memory_embeddings` + its index from the vector store.

Rule: every core object present with identical definition;
every object in the catalog belongs to exactly one group
with identical definition; an optional group is either
complete or absent. Anything else — an extra table, a
modified trigger, a partial group — fails closed. This does
not "ignore" derived structures; it names them exactly.

### Backup creation

- `artifact_versions` joins the workspace copy list (same
  budget accounting; bodies live in the rows, so no new
  archive members).
- Artifact privacy is re-checked under the publish lock,
  like memory privacy today: a `sealed` artifact row refuses
  a plaintext backup. (Creation already self-verifies, so
  the verification rules below gate both paths.)

### Package verification of artifact rows

For every `artifact_versions` row in the staged catalog:
ids well-formed; `content_hash == sha256(body)`; privacy in
{standard, private} (sealed refuses a plaintext package);
versions per artifact contiguous from 1; `project_id` is a
memory of the same workspace; `source_references` is a JSON
string array in which every `artifact://<id>@v<n>` entry
resolves to a contained artifact version (a malformed
`artifact://` entry fails closed); `generator` is null or a
JSON object. Evaluation bodies additionally parse and their
`sourceArtifactId` / `rubricArtifactId` + versions resolve
to contained rows. All checks run on the staged copy before
anything is imported.

### Restore: import, remap, rebuild

- Artifact ids join the identity map (per workspace);
  `artifact_versions` rows are imported with `workspace_id`,
  `artifact_id` and `project_id` remapped.
- **Reference semantics after restore** (the definition this
  repair establishes): every id is freshly minted, and every
  reference that names an artifact *contained in the
  package* is rewritten to the restored id — in the
  `source_references` column and, for evaluation documents,
  in the body's `sourceArtifactId`/`rubricArtifactId`, with
  `content_hash` recomputed over the rewritten body so the
  registry's integrity check holds. A reference naming an
  artifact *not* in the package fails the restore closed
  rather than importing a link that could resolve into the
  original workspace. Bodies of all other artifact types are
  opaque and preserved byte-for-byte. Historical provenance
  is preserved through `restore_origins` (original ↔ new id
  for every entity, artifacts included) — the original
  workspace is never modified.
- **Idempotent retries:** if `restore_origins` already
  records this backup artifact, restore returns the original
  result instead of importing a duplicate. (Re-uploading the
  same package as a new artifact is the explicit path to a
  second copy.) The import itself stays a single transaction
  after all bodies are staged, so an interrupted restore
  leaves no partial rows.
- **Derived state:** archived FTS/embedding data is never
  imported. Inside the import transaction, the destination's
  FTS rows for each restored memory's current revision are
  rebuilt from the canonical restored content via the
  existing `sync_index_tx` (which itself applies the
  `SearchBoundary` privacy rule and excludes Sealed).
  Semantic embeddings are derived state keyed to a model;
  they are not rebuilt at restore (a restored workspace is
  in the same state as a fresh one until explicit indexing).

### Legacy schema-8/9 compatibility: validate, then migrate staging only

A package declaring schema 8 or 9 (and whose catalog
`user_version` agrees) is validated **structurally first**:
its object set must equal the legacy core for that version
(migrations 0001–0008 for v8, plus the vault group for v9)
with the fts/embeddings groups optionally present, exact —
plus `application_id`, `integrity_check` and
`foreign_key_check`. Only then is the **isolated staging
copy** migrated with the same step the storage migrator
uses (apply 0010, set `user_version` 10). The standard v10
verification (groups, content, references) then runs on the
migrated staging copy unchanged. Declared versions outside
{8, 9, 10}, disagreement between the declared and actual
versions, unknown objects, or corruption fail closed. The
source package is never modified.

## Acceptance criteria

1. Workspace backup → fresh restore round trip preserves
   projects, all artifact versions, idea set, rubric v1/v2,
   evaluations (bodies, hashes, privacy, generator,
   references) — and evaluations reopen onto the restored
   source concepts + rubric snapshot.
2. Application backup → fresh restore round trip, same
   assertions, with the source database in a *used* state
   (FTS created by a real search; embeddings table created).
3. Archived projects' artifacts round-trip; restore into a
   storage that still contains the original workspace stays
   isolated (no cross-workspace resolution).
4. Legacy fixtures built with the actual older format (v8
   and v9 catalogs packaged by the current tar/manifest
   code) restore successfully; v7/v11/tampered packages
   fail closed.
5. Corruption (tampered artifact body/hash, dangling
   reference, malformed `artifact://` entry) refuses the
   package before import; an interrupted restore leaves the
   destination unchanged; retrying a completed restore does
   not duplicate.
6. Restored content is searchable (FTS rebuilt from restored
   revisions); archived FTS rows are not trusted or copied.
7. Existing backup/vault/artifact/ideas/evaluation/provider
   regressions stay green; exact counts recorded, with the
   corrected 187-test core accounting kept visible.
8. A real application backup → fresh restore → reopen
   workflow runs end to end (browser/API-level evidence).

Out of scope: new features, preview promotion, pushes, CI,
real credentials or inference. Antigravity's files are not
touched.
