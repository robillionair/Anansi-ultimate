# Overnight hardening — Work Package 1 acceptance criteria

Branch: `muse/m8-overnight-hardening`, from Slice 4 tip
`0b6ba0e8723edb9a223db63a00a69565d7d77b7c` (ancestry
`c7327bb7` / `532afccf` / `4ef90406d` preserved). These criteria
were committed BEFORE any WP1 implementation. Windows review of
Slice 4 is still in progress; nothing here claims it passed.

## Lifecycle rules established from the codebase (not invented)

Read from `crates/core/src/memory.rs`, `lifecycle.rs`,
`privacy.rs`, `collider.rs`, `artifacts.rs` and `AGENTS.md`:

- Sealed is a privacy restriction, per revision row in storage.
  Production writes refuse Sealed plaintext
  (`MemoryDraft::validate` → `allow_plaintext`); reads refuse
  Sealed rows (`metadata` → `allow_plaintext`, plus the list-SQL
  exclusion). AGENTS.md: "Do not transmit private or sealed
  memories without explicit authorization." Sealed rows in
  production arise only via the future vault or imported
  packages; tests use the established SQL fixture
  (`seal_memory`), labeled as such wherever it stands in for
  a vault seal.
- Archived is a lifecycle withdrawal, reversible to
  `archived_state`. Lists exclude archived current rows by
  default; an archived project is refused as a source
  (`require_project_writable`). Archival creates a NEW
  revision; older revisions keep their own status.
- Expired is Temporary + `validUntil` in the past. Lists
  exclude expired current rows by default. A new Temporary
  revision must carry a future `validUntil` at write time;
  an existing Temporary revision becomes expired only as
  time passes, so tests simulate passage with an SQL
  `validUntil` update, labeled as such.
- Reassignment is a new revision carrying a different
  `projectId`. The picker lists CURRENT members only.
- Saved experiments are historical evidence, not live
  references: their snapshots render verbatim and restore
  validates pinned rows only, never current-head state.

## The rule for a NEW dispatch (both pickers)

A new `collider-start` sends content to a provider, so it must
not bypass a CURRENT restriction via a stale pinned reference.
The pinned revision must be currently disclosable AND the
source must be currently eligible:

- Pinned revision: not Sealed (existing), not Archived
  (projectContext already; memoryContext must gain it), not
  expired (both pickers must gain it).
- Source current head: not Sealed (both pickers must gain
  it — a sealed head seals the memory's content on new
  dispatch, even when an older revision row is plaintext),
  not Archived (both pickers must gain it), not expired
  (both pickers must gain it, for consistency with list
  exclusion).
- projectContext only: the current head's `projectId` must
  equal the selected project (reassignment out of the
  project blocks new dispatch as that project's content).
  The pinned-row `projectId` check stays.

What this rule does NOT do (no general prohibition on
historical reads):

- `memory_read`, search, saved-experiment views and restore
  are untouched: an old revision of a sealed/archived/moved/
  expired source remains readable as history, and a saved
  experiment keeps its verbatim snapshot and remains
  restorable after its sources change state.
- Older revisions of a source that REMAINS eligible (head
  Confirmed/Durable, unexpired, still in the project) must
  still dispatch. Every refusal below carries a positive
  control proving this.

## A. Boundary refusals (pre-network, zero provider hits)

In every refusal case: `collider-start` returns an error
(no run is created, so there is no network work to cancel),
the fake provider records exactly 0 hits, and the error
names only the 1-based item position (existing convention).
The refusal battery runs for memoryContext AND
projectContext unless a case is picker-specific.

- A1. Pinned archived revision via memoryContext refuses.
  (projectContext already refuses; prove both.)
- A2. Older plaintext revision of an archived-head memory
  refuses via both pickers. Control: the same older
  revision dispatches while the head is still Confirmed.
- A3. Older plaintext revision of a sealed-head memory
  refuses via both pickers. Fixture: seal ONLY the head
  revision row (SQL, labeled as a vault-seal stand-in);
  the pinned revision row stays plaintext. Control:
  plaintext memory dispatches; pinned sealed revision
  already refuses (existing test).
- A4. Archived source project refuses via projectContext.
  (Already enforced by `require_project_writable`; prove
  it with a test, do not assume it.) Control: the active
  project dispatches.
- A5. Member reassigned out of the project (revise with a
  new `projectId`) refuses via the OLD project's
  projectContext. Controls: the same member dispatches
  via the NEW project's projectContext, and a saved
  experiment referencing it under the old project still
  restores and views verbatim (history preserved).
- A6. Expired pinned revision (Temporary + `validUntil`
  moved to the past by labeled SQL fixture) refuses via
  both pickers. Expired head with an older durable pinned
  revision refuses via both pickers. Control: an
  unexpired Temporary revision dispatches.
- A7. Saved history is never rewritten by any of the
  above: after sealing/archiving/moving/expiring a source,
  the previously saved experiment still opens showing its
  pinned snapshot verbatim.

## B. Structured artifacts across repeated recovery

Recovery semantics (defined before repair):

- Generation-time snapshot content is historical evidence
  and stays verbatim forever; the saved view shows what
  was actually sent, never a restored substitute.
- Restored references must resolve consistently across
  any number of restores.
- Original identity/hash and restored identity/hash must
  not be conflated: `contentHash` stays the original
  generation-time hash; a new optional
  `restoredContentHash` records the restored row's hash
  when (and only when) trusted restore itself changed it.
- Original bindings are validated before any remapping
  (unchanged order); invalid input refuses before
  destination publication (unchanged).
- A fabricated original is never "repaired" by
  recomputing its hash: validation still refuses any
  item whose snapshot, pinned hash, restored hash and
  package row do not line up; the fix-up only records
  what the restore pass itself produced from a valid
  original.

Acceptance:

- B1. Before-state reproduction (committed first, failing
  before repair): experiment with memory context →
  selected as project context for a second collision →
  saved → backup/restore #1 succeeds → backup/restore #2
  refuses. After repair, restore #2 succeeds and the
  reopened experiments keep valid provenance.
- B2. After repair, the second experiment's snapshot
  content is byte-identical to what was sent, its
  artifact item's `restoredContentHash` equals the
  restored source row's hash, and its item count is
  unchanged (nothing silently dropped).
- B3. A second representative structured artifact, created
  through its real product operation (a real evaluation
  record or a real source package, not arbitrary JSON
  labeled as that type), passes the same double-restore
  sequence as project context.
- B4. A transferred concept derived from the experiment
  survives the double restore and stays Forge-admissible
  (the new fix-up runs before the existing transfer
  experiment/receipt fix-ups, so concept and receipt pins
  follow the final experiment hash).
- B5. Negative controls stay refused: tampered snapshot,
  misbound item, and a fabricated `restoredContentHash`
  matching neither the pinned nor the row hash all refuse
  restore before publication (existing tamper tests keep
  passing).
- B6. The repair is bounded to artifact items. Memory
  items need no tracking field: memory bodies are copied
  verbatim on restore, so their body hashes never change.

## Focused acceptance gate for WP1

- New boundary + recovery tests pass; all pre-existing
  collider / transfer / forge_restore / backup_restore /
  node suites pass unchanged (counts recorded, no
  assertion weakened).
- Strict Clippy, fmt, tsc, contracts check clean.
- The WP1 record (this file, updated with before/after
  evidence) is committed before WP2 criteria are written.
