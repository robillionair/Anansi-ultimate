# Muse assignment — repair M8 acceptance before Slice 3

Use the attached independent review bundle's `codex/review-m8-slices1-2` tip as
the baseline. Record its full SHA before working and verify incoming
`ffbeee40a4dd3be189e2516d38cbde22b65167ce` is an ancestor. Read
`docs/reviews/windows-m8-slices1-2.md` and retain every expected-success probe.
This is an acceptance repair, not new feature work or preview promotion.

Create `muse/m8-acceptance-repair`. Read AGENTS.md, the M8 design/addendum,
MASTER_SPEC privacy/provenance/cost requirements, architecture, current status
and relevant decisions. Define repair acceptance before changing production code.

## Required repairs

1. **Authoritative exact transfer approval and provenance.** A format string,
   shape-valid JSON, client flag or editable generator metadata is not an approval
   receipt. Generic registration/revision must not mint approval for a new brief,
   source identity/version or selected combination. Before Forge dispatch, resolve
   the exact experiment in the same workspace/project, check its stored/body hash,
   selected result and every model-derived field. Require the approved snapshot
   to match the requested concept data. Preserve original historical approvals;
   saving a specification still does not approve it. All denials must yield
   zero synthetic provider hits. Do not hide the generic editor as the fix.
2. **Complete restore validation before publication.** Check selected combination,
   assumptions, ordered unknowns/dependencies, productization brief, provider and
   model against the pinned experiment, along with ids/versions/hashes/ownership.
   Preserve approval only for the restored equivalent of the validated original
   approved snapshot. Do not repair fabricated originals by recomputing hashes.
3. **Durable atomic idempotency.** Retrying the original transfer after a supported
   generic revision must resolve to its original identity/approved snapshot, not
   create another concept or return altered content as the original approval.
   Make check/create safe across two independent desktop/API processes sharing
   one synthetic root. A process-local mutex plus scanning latest versions is not
   a durable cross-process guarantee. Avoid an unbounded registry/body scan under
   an application-wide mutex. Choose a coherent existing storage/transaction
   mechanism; if a schema change is necessary, cover forward migration, vault
   compatibility and backup/restore together. No downgrades or assertion-only
   version fixes. Verify retry after restart and fresh restore.
4. **Preserve exact accepted text.** Validate whitespace-only and length limits
   without trimming accepted beams or approved briefs. Preserve indentation,
   tabs and terminal newlines from submission through wire, disclosure, saved
   records, identity comparison, reopen and restore. Render approved text inertly
   with whitespace preserved where necessary for an honest preview.
5. **Source-bound UI state.** Transfer draft, selection, pending response and
   displayed approved concept must belong to the exact workspace/project/artifact
   and version. Switching experiments or versions must not reuse an old draft or
   receipt. Editing selection/brief after approval must not keep the old concept
   displayed as approval of the new draft. Bind late responses to their submitted
   snapshot; guard asynchronous list/read/navigation callbacks by identity/epoch.
   Apply the established keying discipline to new stateful transfer/Forge panels.

## Evidence and verification

The review adds five `acceptance_` transfer probes, one exact-beam probe, and
stronger browser assertions. Their failures are acceptance defects, not flaky
timeouts. Preserve them and make them pass without weakening the requirements.
The first browser assertion can stop the flow: separately exercise post-approval
edit/selection and source/version-switch cases so later defects are not masked.

Run focused cases first, then existing Collider/transfer tests and Forge
revision/deliverable/source-review regressions. Verify generic-edit adversarial
cases at consumption and restore, no destination mutation, zero network on
denials, exact submitted text and independent-process concurrency. Synchronize
concurrency explicitly rather than relying on guessed sleeps or racing retries.
Run browser suites sequentially on an idle machine with existing limits.

No generated-code execution, iframe preview, deployment, real credentials,
paid/cloud inference, model downloads, native preview promotion, personal-data
changes, pushes, master merge or remote CI. Do not start M8 Slice 3 or claim M8
complete. Existing deferred execution/production gates are not waived or reopened
as prerequisites to fixing these bounded defects.

Work autonomously through design, repairs and verification. Deliver small commits,
exact results and failed-attempt history, updated status, a short owner guide and
a checksummed complete-history bundle inside a ZIP. Independently verify the ZIP's
enclosed bundle and fetch the exact reported tip. Distinguish Linux evidence from
the later independent Windows rerun; no Windows/native pass may be inferred.
