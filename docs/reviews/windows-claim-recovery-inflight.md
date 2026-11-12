# Recovery in-flight polling Windows review

Runtime: `b5dc7fcb4c589777bb1e65bde28fa484c77687c4`.

## Acceptance recorded before probe

Passive observation must preserve retry accounting for all three root states:
pending, actively claimed, and exhausted. Only deliberate work/recovery may
reset it. The delivered tests cover pending and exhausted but not active.

The deterministic queue probe takes an active entry, models two failures,
performs a passive wake while that entry is out of the queue, then requeues
the failed entry. Its next failure count must remain two, not reset to zero.
No timing, SQLite, or fabricated permission failure is needed for this ordering.

Production changes are prohibited in this reviewer checkpoint.

## Verdict — changes required (2026-11-12)

Incoming ZIP SHA-256 matched
`f2d6dd84372e5a0836b0b49ca3b48c35d4cb54b8277f828d85b0916426a6c6ac`.
Bundle verification passed against reviewer parent `acf2d7f97`, and the exact
runtime tip was imported. Original reviewer test remained unchanged.

Windows command:

```text
node scripts/cargo.mjs test --locked -p anansi-core --lib reviewer_ -- --nocapture
```

Observed: 1 passed, 1 failed, 326 filtered, 10.05 seconds. The original
`reviewer_status_poll_does_not_rearm_exhausted_claim_budget` passes unchanged.
Its public response carries `recoveryDiagnostic` with attempts 3, reason
`storageUnavailable`, and the fixed sanitized Retry recovery instruction.
This confirms exhausted-root passive handling and production diagnostic
serialization in this focused scope, not native UI/button acceptance.

### CR-01b — passive polling resets an active root's failure budget

New deterministic test
`jobs::tests::reviewer_worker_queue_passive_wake_preserves_inflight_budget`
fails with actual failure count 0, expected 2. While the worker owns the active
entry, the pending queue has no matching entry. `wake(Passive)` inserts an
entry with zero failures. `requeue(active)` sees that entry and drops the
active counter. Thus passive polling during a claim can refresh the budget
before exhaustion; Documents' existing four-second polling can encounter it
when storage reads/locks take time. No filesystem timing or forced SQLite
failure is needed to establish the queue ordering.

Track or merge active/pending state so passive observation cannot discard its
failure budget. Preserve genuinely deliberate re-arming, deduplication, FIFO,
startup recovery, and wake-during-shutdown behavior. Cover passive/deliberate
wakes across pending, active, and exhausted states. Do not disable polling or
weaken the probe. Avoid stale in-flight failure results overriding a newer
deliberate recovery decision.

Queue-only run: 11 delivered tests pass; the new probe fails (12 total).
These are defect reproductions, not acceptance passes. The full 395-test
Windows library suite, builds, UI, and browser acceptance were not rerun after
this focused finding. Muse's reported Linux results are not Windows evidence.
The new diagnostic and Retry recovery UI need meaningful interaction coverage;
the existing green browser battery alone does not exercise the new failure UI.

No production edits, live provider calls, credentials, preview changes, or push.
Only reviewer tests and documents are changed; prior accepted pilot is intact.
