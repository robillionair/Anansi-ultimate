# M8 Slices 1–2 independent Windows review

Incoming tip `ffbeee40a4dd3be189e2516d38cbde22b65167ce`, Slice 1
`8718df8a7f3def7f5322e3adbb44d3300cc24f34`, ancestor `08465172a` verified.
ZIP SHA-256 `86edcb90c6433751e9e4f5ae2a1ba8c945f7ebba6af0a08ec7819b437751c737`.
Bundle SHA-256 `28b16b5d3540dd4f325482dc03e7795c6a8998922509a27733eaeafb63601000`.
All 5 refs and bundle integrity verified; separate review worktree on E:.

Acceptance plan before probes: run delivered collider/transfer backend cases,
shared UI/contracts/type/build checks, then probe transfer brief preservation,
consumption of edited provenance before provider traffic, restore of edited
model-derived fields and transfer-panel identity after editing/version changes.
Record failures without weakening expected-success assertions. Production code
and running preview remain unchanged. Synthetic fixtures only; no model downloads,
credentials, paid inference, generated execution, pushes or remote CI.

Inspection additionally found the deduplication lock is process-local and the
persisted scan checks only latest concept versions. Cross-process atomicity
requires separate evidence; no multi-process failure is claimed from inspection.
Results and bounded repair requirements follow after execution.

## Confirmed backend acceptance failures (production code unchanged)

The dedicated expected-success transfer probes ran with `acceptance_` filter:
**0 passed / 5 failed**, 209.55 seconds, exit 101. These are real failures, not
inverted tests whose passing result means broken production behavior.

1. `acceptance_forge_rechecks_transfer_binding_before_provider_traffic`:
   a supported generic revision changed the source experiment id to a nonexistent
   id and the supposedly verbatim combination description. `forge-spec-start`
   returned a run and the synthetic provider received **1 hit**. Full source
   binding must be resolved and checked before any dispatch; a shape validator
   or format marker is not evidence of transfer approval/provenance.
2. `acceptance_generic_edit_does_not_mint_transfer_approval`: changing only the
   approved brief through generic revise, with no new transfer action, was
   accepted by Forge at the new concept version; synthetic provider **1 hit**.
   This demonstrates acceptance as an approved transfer, not creation of a
   separate approval row (the implementation currently has no such receipt).
3. `acceptance_restore_rejects_fabricated_model_derived_provenance`: a concept
   with changed assumptions/provider/model but intact source pins and selected
   combination restored successfully and published the requested workspace.
   Comparing only the selected combination does not validate all derived fields.
4. `acceptance_retry_after_generic_revision_keeps_original_transfer_identity`:
   after generic revise changed the latest brief, retrying the original transfer
   returned a **different concept id**. Latest-version scanning loses the
   original immutable transfer identity retained in history.
5. `acceptance_approved_brief_is_preserved_exactly`: leading indentation and
   the final newline were removed. Both the UI submission and backend constructor
   trim the approved brief, violating exact text preservation.

Additional Collider probe
`acceptance_exact_beams_are_preserved_on_wire_and_save` failed 1/1 in 24.24
seconds. Saved Beam A lost its leading indentation and terminal newline; Beam B
lost its leading tab and newline. The source also trims the wire segments.
Whitespace-only rejection may use trim; accepted source data must stay exact.

## Positive checks and execution accounting

- Incoming bundle independently cloned in a bare scratch repo to the exact tip;
  earlier `08465172a` ancestry confirmed.
- All 12 delivered Collider backend tests passed on Windows (575.78 seconds).
- Delivered UI/contracts 76/76, TypeScript, web build and docs passed.
- Dedicated transfer probes above completed fully. The initial two-target cargo
  run subsequently entered a duplicate transfer executable including the new
  probes; that redundant runner was intentionally stopped after dedicated
  failures were established. Its interrupted exit is not a pass. No complete
  Windows result for the 8 original transfer cases is claimed.
- Strict core-library/transfer-test Clippy passed with warnings denied. API build
  passed. Final contracts and browser results follow after execution.
- Only test files and review records changed; no production repair, source
  migration, preview promotion or acceptance claim is made.

## Browser failure and final scope

The original Collider browser workflow passed 1/1 (3.4 minutes): explicit start,
cancellation, save, restart and fresh restore. The stronger transfer browser
probe failed after 1.2 minutes: the page displayed experiment version 2 while
`#collider-transfer-brief` retained exactly
`UNAPPROVED DRAFT FOR EXPERIMENT VERSION ONE`. The unchanged configured assertion
waited 30 seconds and resolved that stale value 62 times. This is a reproduced
source-identity failure, not a guessed timing explanation.

Inspection found `ColliderTransferPanel` is not keyed to experiment identity and
its draft initializes only once. It also keeps `transferredId` after brief or
selection changes, leaving the old Open in Product Forge confirmation displayed.
The added post-approval-edit probe follows the earlier version-switch assertion
and was not reached in this failed run; do not count it as independently
reproduced. The repair assignment requires separate coverage of both cases.

Browser run: **1 passed / 1 failed**, 4.7 minutes total. The original complete
transfer browser workflow was not newly verified on Windows beyond the failing
probe. No assertion, timeout or production code was changed to hide a defect.
The application-test lease released normally and owned test services stopped.

Contracts drift, strict library/transfer-test and Collider-test Clippy, formatting,
TypeScript, web/API builds and docs validation passed. These compilation/check
results do not override the **six failing backend probes plus one failing browser
probe**. No full Windows core/provider/API suite is claimed. Raw synthetic logs
were retained outside source in the incoming delivery directory on E:.

## Verdict

**Windows acceptance is not granted for Slices 1–2 as delivered. No promotion.**
Use [the bounded repair assignment](../handoffs/M8-slices1-2-Windows-repair.md)
and preserve every expected-success test. The branch contains tests/evidence only;
the original production implementation remains unchanged, deliberately red under
the stronger probes. After repair, rerun the new cases plus existing Collider,
transfer, Forge and recovery regressions. Independent-process transfer atomicity
is an additional required verification, not a reproduced multi-process failure.

Existing running preview, its launchers and personal data were untouched. M7/M8
completion, native acceptance, generated-code execution and production gates are
not claimed. No real credentials, paid/cloud inference, model downloads, generated
software execution, pushes or remote CI occurred.
