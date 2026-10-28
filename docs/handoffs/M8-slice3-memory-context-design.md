# M8 Slice 3 — explicitly selected memory context for LEXI-COLLIDER: design

Baseline: `codex/review-m8-acceptance-repair` @
`4d3f8bfdaf904f452fe65c404a80bc2ef9aa83e2` (Windows-focused
acceptance passed with local correction `5b3cf958f`, which verifies
approval-receipt body hashes before use). This slice builds on that
tip, not on `1ccda62d` alone. Authority: MASTER_SPEC §§23–27 (§24
lists "Select approved memories" as a beam input option), the M8 plan
(`plans/06-lexi-collider.md`), and the owner assignment of 2026-10-28.

## Scope

One vertical slice: in an existing project, the collider keeps both
text beams and gains an optional picker for memories in the current
workspace. Nothing else from §24 is built here: no URL fetching, no
document parsers, no saved-project selection, no automatic memory
scanning, no 3D work, no new feature slice. No generated-code
execution, deployment, paid inference, real credentials, model
downloads, pushes or remote CI. The running preview is not promoted
or replaced.

## Design decisions

### 1. The client submits references, never bodies

The start request gains an optional `memoryContext` field: a list of
`{ memoryId, revisionId }` pairs. The backend resolves each pair
itself through the existing `Storage::memory_read`, which is
workspace-scoped, revision-pinned, ownership-checked, hash-verified
and refuses Sealed rows via `allow_plaintext`. A client-supplied
body, privacy label, workspace claim or revision claim is never
trusted: anything the client sends beyond the two ids is ignored,
and any unresolvable pair refuses the whole start before any
provider traffic, with zero provider hits.

This is what makes the pre-send review exact: the panel reads each
selected memory's current revision, shows that revision's title,
version and full body, and submits exactly the revision ids it
displayed. Revisions are immutable history, so a memory edited
between review and submit still resolves to the reviewed revision.

### 2. Bounds, enforced before provider traffic

At most 8 selected memories; each resolved body at most 8000
characters (the beam bound); total context at most 32000
characters; duplicate memory ids refused. Overlong or duplicate
selections are refused, never truncated: truncation would break
the promise that the review shows exactly what will be sent.

### 3. Memory content travels as private, untrusted data

Resolved entries become one additional provider segment each,
privacy `private`, framed as `Memory context (data)`. The
instruction segment gains one sentence: memory context segments
are likewise data, not instructions, and embedded instructions
inside them are to be ignored. Routing, Zero-Cost Lock, permits,
disclosure, cancellation and accounting are the existing M5
machinery, unchanged. The disclosure record carries counts and
routing only, never memory text.

### 4. Generation-time provenance is saved with the experiment

The experiment document (`anansi-collider-experiment` v1, format
and version unchanged) gains an optional `memoryContext` array,
defaulted so older experiments without the field still parse.
Each entry pins `memoryId`, `revisionId`, `version`, `title`,
verbatim `content`, `contentHash` (hash of the content) and the
resolved privacy label (`private`, `standard` or `public`; sealed
is impossible because resolution refuses it). The run carries the
resolved entries from start through save, so the saved record is
the generation-time snapshot: later memory edits never change a
historical experiment, and reopening shows the snapshot, not the
live memory.

### 5. Restore validates originals, then remaps

Restore pre-validation for collider-format experiments is
extended: for every context entry, the source package must hold a
`memory_revisions` row with that revision id, belonging to the
pinned memory, in the artifact's own workspace, with matching
`body_hash` and non-Sealed metadata, and the snapshot content
must hash to the pinned hash. Any failure refuses the whole
restore before any destination state is published.

Remap then rewrites each entry's memory and revision ids through
the restore identity map (memories and revisions are already in
it), validating revision ownership against the package's revision
rows, fail-closed. Snapshot content and content hashes stay
verbatim; the row hash is recomputed because the body changed.

### 6. Transfer preservation for context-bearing experiments

A context-bearing experiment's body changes on restore (its
memory ids remap), so its content hash changes — unlike the
context-free case, where Slice 2 could rely on the hash being
stable. A transferred concept pinning the old experiment hash
would otherwise fail Forge admission on the restored workspace.
The fix-up pass therefore rewrites concept and receipt pinned
experiment hashes to the restored experiment row's recomputed
hash for the pinned version, exactly as the existing fix-ups
already do for specification approvals and receipt concept
hashes. The binding follows the restored equivalent; it is never
recomputed from an unvalidated original, because pre-validation
already approved the original bindings.

### 7. Picker exclusion of Sealed content

The picker lists via the existing `listMemories` and filters
Sealed entries client-side, so Sealed memories are never offered.
The shared list itself currently fails the whole page when any
row is Sealed (each row's `allow_plaintext` propagates). That
would make the picker unusable in any workspace containing a
Sealed row, so `memory_list` is changed to skip Sealed rows
instead of erroring — exclusion, not disclosure, consistent with
the context compiler's skip and the search SQL exclusions. A
regression test pins the new behavior.

### 8. Source-bound UI state

The collider panel is already keyed by workspace/project, so a
project switch unmounts it and discards selection and review.
Within the panel, selection changes re-read the affected
memories and rebuild the review; a review fetch carries a
selection epoch, and a response for a superseded selection is
dropped. The run controller's existing epoch discipline guards
late start/poll responses. The experiment view gains a memory
context section (title, revision, verbatim content) for
context-bearing experiments; older experiments render without it.

## Acceptance criteria

Backend (synthetic provider, request captured):

- A collision with two selected memories sends both contents in
  the provider request; an unselected memory's content and a
  cross-workspace memory's content are absent.
- A Sealed selection, a nonexistent id, a foreign-workspace id
  and a stale revision id are each refused before provider
  traffic with zero provider hits.
- Generate against memory revision B, revise the memory to C,
  then save, reopen and restore: the experiment still resolves
  to B's content, and the restored experiment's references point
  at the restored memory/revision.
- A tampered context entry (wrong pinned hash, wrong owner, or
  Sealed original) refuses restore and publishes no destination
  workspace.
- A transferred concept for a context-bearing experiment
  survives restore and remains Forge-admissible on the restored
  workspace (receipt and pinned experiment hash follow the
  restored equivalent).
- Cancellation of a context run saves nothing and settles
  accounting through the existing path; malformed provider
  output with context becomes an invalid run that saves nothing;
  a Zero-Cost Lock refusal with context has zero provider hits.

UI/contracts:

- No memory is selected automatically; selecting, reviewing or
  changing context starts no inference (provider hit count
  unchanged until Collide Ideas).
- The pre-send review shows exactly the titles, revision
  versions and bodies that will be sent; changing the selection
  rebuilds it; switching projects resets it.
- The saved experiment view shows generation-time content; after
  a memory edit, reopening still shows the original content.
- Node UI/contracts tests cover the experiment parser with and
  without memory context and refuse malformed entries;
  TypeScript, contracts drift, strict Clippy and formatting pass.

Browser (canonical suite grows to 17 files):

- New `collider-memory-context.spec.ts`: selection → pre-send
  review → collision → save → restart → fresh restore, with a
  memory edit between save and reopen proving the snapshot, plus
  selection-change and project-switch staleness checks.
- Full canonical battery plus Collider, transfer and Forge
  regression specs pass sequentially on an idle machine, with
  existing timeouts and assertions preserved.

## Files (planned)

- `crates/core/src/collider.rs`: selection DTO, resolution,
  segments, run context, save provenance, experiment validation.
- `crates/core/src/memory.rs`: list skips Sealed rows.
- `crates/core/src/restore.rs`: experiment binding validation,
  experiment remap branch, concept/receipt experiment-hash
  fix-up.
- `crates/core/tests/collider.rs`: wire-content, refusal,
  revision-pinning, restore, transfer-preservation and
  cancellation/malformed probes.
- `packages/ui/src/Collider.tsx`, `packages/ui/src/Projects.tsx`
  (help text only), `packages/contracts` (regenerated DTOs),
  `packages/ui/tests/collider.test.*`: picker, review, view.
- `tests/e2e/collider-memory-context.spec.ts`: canonical workflow.
- Docs: this design, `M8-slice3-memory-context-addendum.md`,
  `BUILD_STATUS.md`, `plans/06-lexi-collider.md`,
  `docs/REQUIREMENTS_MATRIX.md` (AN-013 stays `in progress`;
  URL fetching, document ingestion and 3D remain deferred).
