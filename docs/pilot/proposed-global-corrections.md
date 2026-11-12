# Proposed global corrections (for Antigravity — not applied here)

These corrections concern Antigravity-owned global
status documents. They are PROPOSALS ONLY. No file
listed here was edited in this documentation pack.
Antigravity should apply, amend, or reject each item
and record the outcome in its own review. Wording
below is suggested text, not committed status.

Stale claims inside Muse-authored pilot documents
(the scope and handoff) were fixed narrowly in place
in this same pack and are NOT listed here; see the
delivery summary for that list.

## P-1. README.md — stale recovery fault-test status

Current text (line 5):

> "The latest recovery fixes pass focused Windows
> checks, but two fault tests still time out in the
> broad Windows library run."

Current text (verification table and paragraph,
around lines 90–92):

> "Broad Windows library acceptance | Held: two
> fault-test parking timeouts remain in the quieter
> rerun" ... "There is no clean 403-test Windows
> acceptance claim."

Why stale: the 2026-11-12 review at reviewer
`8453f649` supersedes this: 416/416 in one
default-concurrency invocation, PASS in focused
Windows scope, with the repair loop closed. The
two named fault tests (`deliberate_wake_rearms_
exhausted_root`,
`diagnostic_absent_for_other_states_and_roots`)
belong to the pre-repair broad run at `8df1fcf3d`.

Proposed replacement (status paragraph):

> "The recovery fault tests pass in the focused
> Windows aggregate: 416/416 in one
> default-concurrency invocation at reviewer
> `8453f649` (2026-11-12). The earlier broad-run
> parking timeouts are superseded by that review;
> see the build status ledger and the witness-gate
> review."

Proposed replacement (verification table row):

> "Broad Windows library acceptance | 416/416 in
> one default-concurrency invocation at reviewer
> `8453f649` (focused scope, 2026-11-12); the
> repair loop is closed."

Also stale in the same line-5 paragraph: "latest
delivered application source at `8df1fcf3d...`".
Proposed: update the "latest delivered source"
reference to the current verified checkpoint
lineage (v1 RC tip `8931d34b`, qualification tip
`7c3af5e`, review tip `08fc0954`, as Antigravity
judges current at apply time), or remove the
specific hash and point at the build status ledger
as the current-checkpoint authority.

## P-2. REQUIREMENTS_MATRIX.md — AN-008 status

Current row:

> `| AN-008 | OAuth and API keys | ... | M5 |
> planned | Token lifecycle and redaction tests |`

Why stale: Connect AI API-key setup is implemented
and Linux-verified (guided flow, native-prompt
credential setup, connection-bound verification,
orphan handling), and the ChatGPT token-lifecycle
repairs are implemented with repair 4
Windows-verified. What is genuinely open: Windows
review of Connect AI setup + verification repair,
live-provider verification, and ChatGPT OAuth
(deferred, disabled).

Proposed: change status `planned` to
`in progress`, and extend the evidence cell to
record the delivered API-key scope separately from
the deferred OAuth scope, for example:

> "API-key setup implemented and Linux-verified
> (Connect AI guided flow; native-prompt
> credentials; connection-bound verification);
> Windows review and live-provider verification
> open. ChatGPT OAuth deferred/disabled;
> token-lifecycle repair 4 Windows-verified."

## P-3. REQUIREMENTS_MATRIX.md — AN-026 status

Current row:

> `| AN-026 | Zero-Cost Lock, routing,
> budgets/disclosure | M3, M4, M5; all inference
> milestones | planned | Deny unknown/paid fallback
> and unauthorized data; disclosure fixtures |`

Why stale: the Zero-Cost Lock is implemented in
provider dispatch and the workflow gates, with
dispatch tests covering preflight refusals, and
disclosure manifests are implemented in the Ask,
Forge, Collider, idea, and opportunity workflows.
The row predates the implementation.

Proposed: change status `planned` to
`in progress` (full multi-provider + production
scope remains open under the M5 gates), and extend
the evidence cell, for example:

> "Zero-Cost Lock implemented in dispatch
> (default on; unknown-cost treated as paid) with
> disclosure records in the generation workflows;
> production gates incl. live-provider
> verification remain open."

## P-4. REQUIREMENTS_MATRIX.md — AN-013 Slice 4 scope note

Current row status is `verified` and its evidence
cell states "Slices 1–4 ... verified in the native
preview", while every gate list records M8 Slice 4
Windows verification as open.

Why it needs a note (not a status change): both
statements are true in their scopes — Slice 4
behavior exists in preview builds, but its
independent Windows verification is pending per
its own addendum (Linux-only evidence).

Proposed: append to the AN-013 evidence cell:

> "Slice 4 (saved-project input): independent
> Windows verification pending per its addendum;
> Linux-only evidence to date."

No status-word change is proposed for AN-013; the
note removes the apparent contradiction.

## P-5. BUILD_STATUS.md — entry for this documentation pack

Proposed, for Antigravity to append once the
Windows verdict on the installer qualification
rerun is known (not before): a dated entry naming
the documentation branch/tip, the review package
verified (`08fc0954`), and the fact that the pack
is documentation-only with no product-code change
and closes no gate. No entry is proposed until the
verdict exists, so the ledger never anticipates a
Windows result.

## Handling

- Apply each proposal as a separate, reviewable
  change in Antigravity's own commit(s), or record
  a reason for rejecting it.
- Do not edit the proposals in this file to mark
  them applied; record application in the review
  that applies them, so this file stays an accurate
  record of what was proposed.
