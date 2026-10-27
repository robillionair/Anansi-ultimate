# M8 acceptance repair — design and acceptance criteria

Baseline: independent Windows review branch `codex/review-m8-slices1-2`,
tip `c50de23b1545bc1edb16faecb7c01d78c2acd22f` (bundle SHA-256
`9b741bad248dc9761535b2cf16ba4011c556da106ba39d59099bf860871deca6`;
incoming `ffbeee40a4dd3be189e2516d38cbde22b65167ce` verified as an ancestor;
the review commit changed tests and docs only). Repair branch:
`muse/m8-acceptance-repair`. This is an acceptance repair, not new feature
work, preview promotion or Slice 3.

## Failures being repaired (all preserved as expected-success probes)

1. `acceptance_forge_rechecks_transfer_binding_before_provider_traffic` —
   a generic revision changing the source experiment id and the combination
   description reached Forge (1 synthetic provider hit).
2. `acceptance_generic_edit_does_not_mint_transfer_approval` — a generic
   revision changing only the approved brief was accepted by Forge at the
   new concept version (1 hit).
3. `acceptance_restore_rejects_fabricated_model_derived_provenance` — a
   concept with changed assumptions/provider/model but intact pins restored
   and published.
4. `acceptance_retry_after_generic_revision_keeps_original_transfer_identity`
   — retrying the original transfer after a generic revision of the brief
   created a second concept instead of returning the original.
5. `acceptance_approved_brief_is_preserved_exactly` — leading indentation
   and the terminal newline were trimmed from the approved brief.
6. `acceptance_exact_beams_are_preserved_on_wire_and_save` — leading
   indentation/tab and terminal newlines were trimmed from both beams on
   the wire segments and in the saved record.
7. Browser transfer probe — the transfer panel kept a version-1 draft after
   the experiment moved to version 2, and kept the old "Open in Product
   Forge" receipt after the brief was edited post-approval.

## Repair 1 — authoritative transfer approval receipt

A new backend-only artifact type `transfer-approval` (format
`anansi-transfer-approval` v1), registered by raw SQL exactly like the
existing `specification-approval` rows, so it is deliberately absent from
`ARTIFACT_TYPES`: generic registration of a receipt is refused by the
type list, and generic revision can only add a later version, never
rewrite version 1. The receipt pins the full approved snapshot:

- concept artifact id, concept version (the transfer-created version),
  concept content hash;
- approved brief, exactly as submitted;
- source experiment id, version and content hash;
- selected combination index, name and description;
- assumptions, ordered unknowns, productization brief, provider, model.

`forge-spec-start` for a `concept` source now, before any provider
traffic, (a) looks up the version-1 receipt for the exact concept id,
version and content hash (the same version-1 scan the specification
approval gate uses), (b) requires the receipt record's generator kind
to be `transfer` (manual registration hardcodes `manual`, so no
hand-written row can pass), (c) requires every receipt field to equal
the requested concept body, and (d) resolves the pinned experiment in
the same workspace/project, checks its stored hash against its body
hash and the pinned hash, and requires every model-derived field to
match the experiment (combination name/description, assumptions equal
to feasibility hypotheses in order, unknowns equal to unknown costs
followed by dependencies in order, productization brief equal to the
experiment's concept brief, provider, model). Any mismatch refuses
with zero provider hits. Saving or revising a specification still does
not approve it, and original historical approvals are preserved
because version-1 rows are immutable.

## Repair 2 — complete restore validation

`validate_original_transfer_bindings` is extended from combination-only
to the full model-derived comparison of Repair 1, and additionally
validates every `transfer-approval` row: body hash recomputation,
strict receipt parsing, receipt fields equal to its pinned concept
row's body, and the pinned concept row present with a matching hash.
No fabricated original is repaired by recomputing hashes: a mismatch
refuses the whole restore before any destination write. For valid
originals, the existing remap pattern is extended: a
`transfer-approval` remap branch rewrites the pinned concept and
experiment ids through the identity map (fail closed), and a new
`fix_transfer_receipt_hashes` fix-up pass rewrites each receipt's
pinned concept content hash to the restored concept row's recomputed
hash (the concept body changes on remap because its experiment id
changes), so approval transfers only to the restored equivalent of
the validated original snapshot. The experiment's own pinned hash is
unchanged by restore, as before.

## Repair 3 — durable atomic idempotency, no schema change

The process-local `TRANSFER_LOCK` and the latest-version scan are
removed. The transfer op now calls a new storage method that runs one
SQLite `IMMEDIATE` transaction on a writable connection: scan the
project's version-1 `transfer-approval` rows for the exact
(experiment id, version, combination index, approved brief) match;
on a hit, roll back and return the original concept read at the
receipt's pinned version (the approved snapshot, not the latest
generic revision); on a miss, insert the concept row and its receipt
row and commit. Cross-process safety comes from SQLite's own write
lock, not an application mutex: a second desktop/API process sharing
one storage root blocks on `BEGIN IMMEDIATE` (existing 5s busy
timeout) until the first transaction commits, then sees the receipt
and returns the original instead of duplicating. The scan is bounded
by the project's receipt count, never the whole registry, and the
lock is held only for that scan plus two inserts. No schema change
is needed: receipts live in the existing `artifact_versions` table,
so forward migration, vault compatibility and snapshot schema checks
are untouched (no `user_version` bump, `check_schema` derives from
the unchanged `CURRENT_SCHEMA`), and backup/restore carries receipts
as ordinary artifact rows. No downgrade or assertion-only version
fix is used anywhere.

## Repair 4 — exact text preservation

Validation keeps the trim-for-emptiness check and the length check on
the raw text; storage keeps the raw text. Fixed sites: collider wire
segments, collider run beams, collider save beams, the transfer
constructor's approved brief, and the transfer panel's submission
(which sent a trimmed brief). The UI parsers (`parseColliderResult`
and `parseTransferConceptBody`) currently return trimmed text on
read; they now validate on the trimmed/length check but return the
original string, so reopening a saved record cannot alter it. The
disclosure record contains no beam or brief text (counts and routing
only), so nothing there needs changing. The approved brief renders
in a `<pre>` in `TransferConceptView` (beams already render in
`<pre>`), so the honest preview preserves whitespace; all rendered
text stays inert (no execution, no iframe).

## Repair 5 — source-bound UI state

`ColliderTransferPanel` is keyed by workspace/project/experiment
artifact/version (the key `ColliderExperimentView` already carries),
so switching experiments or versions remounts the panel with a fresh
default draft for the newly viewed version; the same keying is applied
to the concept branch's `TransferConceptView` and `ForgeSpecPanel`.
Inside the panel, editing the brief or changing the selection clears
the displayed receipt, so an old concept is never shown as approval
of a new draft. `approve()` captures its submitted snapshot
(experiment id/version, combination, brief) and compares it against
a draft ref on completion: a response for a superseded draft notifies
without installing a receipt. The generic artifact editor is not
hidden or disabled; enforcement stays in the backend (Repair 1).

## Acceptance criteria (all must pass, none weakened)

- The six review backend probes (5 transfer + 1 beam) pass unmodified.
- The review's strengthened browser spec passes, including the
  version-switch assertion and the post-approval-edit assertion.
- A second, independent browser spec exercises post-approval
  edit/selection and experiment/version switching without depending
  on the first spec's flow, so a failure in one case cannot mask the
  other.
- Existing collider (12), transfer (8), Forge revision/deliverable/
  source-review and restore tests pass; generic-edit adversarial
  cases are refused at Forge consumption and at restore with no
  destination mutation and zero provider hits on denials.
- A generic revision of a receipt leaves the original approval
  intact (version-1 lookup); generic registration of a
  `transfer-approval` type is refused.
- Retry after generic revision, after restart, and after fresh
  restore returns the original concept identity and approved
  snapshot with exactly one concept artifact.
- Two independent API processes sharing one synthetic storage root,
  submitting the identical transfer concurrently, return the same
  concept id and leave exactly one concept; synchronization is by
  concurrent submission against the transactional check, not by
  guessed sleeps or racing retries.
- Submitted beams and briefs round-trip exactly (indentation, tabs,
  terminal newlines) through wire, disclosure-adjacent records,
  saved records, identity comparison, reopen and restore.
- Full core/providers/API, node UI/contracts, TypeScript, contracts
  drift, docs, strict Clippy, formatting, API/web builds and the
  canonical browser battery pass sequentially on an idle machine.

## Out of scope

No Slice 3 work, no preview promotion, no execution enablement, no
schema version bump, no new API operation (the receipt is internal),
no changes to the generic editor's availability, and no claim of
Windows or native acceptance from Linux evidence.
