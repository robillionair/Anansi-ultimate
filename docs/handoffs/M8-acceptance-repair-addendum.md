# M8 acceptance repair — record

Baseline: independent Windows review branch `codex/review-m8-slices1-2`,
tip `c50de23b1545bc1edb16faecb7c01d78c2acd22f` (bundle SHA-256
`9b741bad248dc9761535b2cf16ba4011c556da106ba39d59099bf860871deca6`;
incoming Slice 2 tip `ffbeee40a4dd3be189e2516d38cbde22b65167ce` verified as
an ancestor; the review commit changed tests and docs only). Repair
branch: `muse/m8-acceptance-repair`. Design:
[M8 acceptance repair design](M8-acceptance-repair-design.md).

## What was wrong

The Slice 2 transfer trusted the concept body's format string and
shape: Forge admitted any shape-valid concept, so a generic revision
changing the source experiment id, the combination description or only
the approved brief reached provider traffic as if approved. Restore
compared only the selected combination, so a concept with fabricated
assumptions/provider/model restored and published. Idempotency
scanned only latest concept versions under a process-local mutex, so
a retry after a generic revision created a second concept, and two
processes sharing a root had no atomic guarantee. Beams and briefs
were trimmed on the wire, in run records, in saves, in the transfer
constructor and in the transfer panel's submission, and the UI
parsers returned trimmed text on read. The transfer panel was not
keyed to its source, so it kept a version-1 draft after the
experiment moved to version 2 and kept the old "Open in Product
Forge" receipt after the brief was edited post-approval.

## What changed

- New backend-only `transfer-approval` receipt (format
  `anansi-transfer-approval` v1), registered by raw SQL like
  `specification-approval` rows and deliberately absent from
  `ARTIFACT_TYPES`, so generic registration of a receipt is refused
  and generic revision can only add a later version. Only the
  version-1 row confers approval. `forge-spec-start` for a concept
  source now requires that receipt (generator kind `transfer`,
  which manual registration can never produce) to pin the same
  concept id/version/content hash, brief, experiment binding,
  combination and model-derived snapshot, and requires that
  snapshot to match the pinned experiment in the same
  workspace/project with matching stored/body hashes — all before
  any provider traffic. Saving or revising a concept still confers
  no approval, and saving a specification still does not approve it.
- Restore pre-validation now compares every model-derived field
  (combination, assumptions, ordered unknowns, productization
  brief, provider, model) and validates receipt rows against their
  pinned concept rows; a fabricated original is refused before any
  destination write, never repaired by recomputing hashes. Remap
  and a new `fix_transfer_receipt_hashes` fix-up pass carry valid
  receipts to the restored equivalents.
- Idempotency is one SQLite `IMMEDIATE` transaction around a
  receipt-bounded scan plus the concept and receipt inserts: the
  process-local mutex and the latest-version scan are gone, a retry
  after a generic revision returns the original identity and
  approved snapshot (not the latest revision's content), and a
  second process sharing the root blocks on the database write
  lock, then finds the receipt and returns the original. No schema
  change: receipts are ordinary `artifact_versions` rows, so
  migration, vault compatibility and snapshot schema checks are
  untouched.
- Beams and briefs stay verbatim from submission through wire, run,
  save, transfer, identity comparison, reopen and restore;
  validation still rejects whitespace-only and overlong input.
  The UI parsers return original text (validating on trimmed
  length, as the backend does), the transfer panel submits the
  brief verbatim, and the approved brief renders in `<pre>` inside
  a labelled concept view section for an honest preview. The
  generic editor is untouched; enforcement stays in the backend.
- The transfer panel, the concept view and the concept-branch Forge
  spec panel are keyed by full workspace/project/artifact/version
  identity. Editing the brief or changing the selection clears the
  displayed receipt, and `approve()` binds its response to the
  submitted snapshot: a late response for a superseded draft
  notifies without installing a receipt.

## Verification (Linux, sequential on an idle machine)

- Review probes, unmodified: 5 transfer `acceptance_` probes and
  the exact-beam probe pass (transfer suite 16/16 including 3 new
  tests: receipt revision leaves the original approval intact,
  generic registration of the receipt type is refused, retry
  returns the original approved snapshot; collider suite 13/13).
- Full core 285/289 with only the 4 known environmental
  live-Ollama `semantic_workflow` failures (289 = 280 at the Slice
  2 era + 8 transfer + 1 collider probe); providers 120/0 + API
  24/0 (144/0); node UI/contracts 78/78 (76 + 2 new verbatim
  parser tests); TypeScript, contracts drift, docs (see below),
  strict Clippy and formatting clean; API + web builds pass.
- Browser canonical battery 16/16 across app_checkpoint,
  artifacts, backups, collider, evaluations, forge,
  forge-revision, forge-deliverable, forge-review, ideas,
  opportunities, providers, vault, the strengthened transfer
  spec (version-switch and post-approval-edit assertions), the
  new independent transfer-identity spec, and the new
  transfer-concurrency spec (two real API processes on one
  synthetic root submitting the identical transfer
  concurrently return the same concept id with exactly one
  concept; a different brief is a second transfer).
- Failed attempts, all fixed without weakening an assertion:
  (a) the receipt INSERT had one stray placeholder (column-count
  mismatch, caught by the transfer suite, fixed); (b) the approval
  pre-validation ran before concept fields were pinned (moved
  out; Forge and restore validate the pinned receipt strictly);
  (c) Clippy `type_complexity` on the expected-fields tuple
  (replaced with a named struct); (d) the first browser run
  failed 3/3 — the brief moved from `<p>` to `<pre>`, so the
  three brief assertions were re-scoped to the labelled concept
  view section's `<pre>` (same text, same surface, tighter
  locator; the editor textarea is excluded), and the two new
  specs' fixtures had one cross-domain mapping where backend
  and UI validation require two (fixed to two). The second run
  passed 3/3, and the full battery passed 16/16.
- No Windows or native pass is claimed from this Linux evidence;
  the independent Windows rerun is the next step. No Slice 3 work,
  no preview promotion, no execution enablement, no schema
  version bump, no new API operation, no pushes or remote CI.

## Owner test guide

1. Open a saved experiment and confirm previewing, selecting and
   editing save nothing and contact no provider.
2. Approve a transfer, then revise the concept's brief through the
   generic editor and try to develop the revised version in Forge:
   it must refuse before any provider traffic.
3. Retry the original transfer (API or panel): the same concept
   returns with the original brief, no duplicate.
4. Back up and restore into a fresh workspace, then develop the
   restored concept: it works, with the restored experiment id.
5. In the browser, approve, edit the brief, and confirm the "Open
   in Product Forge" receipt disappears; switch experiments and
   confirm the draft resets to the newly viewed experiment.
6. Submit a brief with leading indentation and a terminal newline
   and confirm the saved concept and the concept view show it
   exactly.
