# Claim-export recovery Windows review

Runtime under review: `d287f6767d09cabb4a0ed0d9b088e05291849e13`.

## Acceptance criteria recorded before the reviewer probe

Ordinary status reads must not reset an exhausted retry budget. An explicit
recovery action may re-arm it. Persistent failure must remain distinguishable
from ordinary queued work through production diagnostics, not only test state.

The focused probe injects persistent claim errors, waits for three attempts and
parking, issues the same job-list operation used by automatic UI polling, and
checks that no fresh retry budget was consumed. It uses the delivered per-root
fault seam, preserves all existing tests, and changes no production code.

Source inspection: `job_command("list")` calls `wake_export_worker`, whose
`WorkerQueue::wake` resets claim failures. Documents automatically calls
`refreshJobs`, which calls `listJobs`. Also, `record_claim_parked` is a no-op in
production; the existing queued log does not explain exhausted recovery.

## Verdict — changes required (2026-11-12)

Delivery ZIP SHA-256 matched `da84e845e83bf6e69ab636656f9d263f2e1212b6b63136456e953c44dc443ad6`. Incremental bundle verification and exact runtime/parent identity passed. Reviewer changes are tests and documentation only.

### CR-01 — status polling re-arms exhausted recovery

The Windows probe fails at its intended assertion: remaining injected faults change from 999997 (three attempts used) to 999994 (six used) after one ordinary job-list request. Documents polls that request automatically every four seconds. Automatic observation can grant fresh budgets repeatedly, including after non-retryable parking. Per-arming boundedness is insufficient for the actual UI polling path.

Keep passive status observation from re-arming exhausted recovery. Preserve startup recovery and provide deliberate retry/new-work behavior. Cover the public polling path, cancellation, and non-retryable errors, not only the private queue.

### CR-02 — parked state has no actionable production diagnostic

The public response after exhaustion still reports `Queued`, with log `Upload complete; queued for import`, and no failure reason, attempt-exhaustion indication, or recovery instruction. The production `record_claim_parked` is a no-op; the test-only parked set cannot support the production observability claim. Normal waiting and intervention-required parking are indistinguishable to the user.

Expose bounded, sanitized recovery diagnostics through an appropriate production boundary and a usable explicit retry path. Do not mutate job rows without a valid lease, bypass unreadable storage, expose raw filesystem errors, or disclose another root's diagnostics.

## Probe and scope

Command: `node scripts/cargo.mjs test --locked -p anansi-core --lib ingestion::tests::reviewer_status_poll_does_not_rearm_exhausted_claim_budget -- --exact --nocapture`.

Final Windows run: 0 passed, 1 failed, 312 filtered, 10.45s. A healthy second root completes as a worker-progress fence before the retry-count assertion. This lets corrected code keep the original root parked; it does not require another parking event. The superseded diagnostic probe waited for a second parking event and also reproduced 999994 versus 999997 (4.54s); its log is retained separately.

These are defect-reproduction failures, not acceptance passes. The wider 380-test suite and browser acceptance were not run on this delivery: the focused findings need repair first. Muse's Linux results remain Linux-only. The preceding 370-test accepted Windows checkpoint is unchanged. No owner data, real credentials, inference, pilot promotion, or push.

