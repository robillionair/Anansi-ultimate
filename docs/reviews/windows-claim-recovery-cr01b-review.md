# Recovery CR-01b Windows acceptance review — 2026-11-12

Runtime: `8df1fcf3d69f6d1da532bb890234a7db0352efff`.
Parent reviewer: `c54c67b545a19d1d651bb51a5235afd50afb3829`.
Delivery ZIP SHA-256 matched
`0ae9a2b0d7a1878cded6be539a22233c4fd40ba69abfb5f24eecbf5cca26a47e`.
The incremental bundle verified and exact tip was imported. Both reviewer
probes are unchanged. This review changes documentation only.

## Verdict

Changes required for broad Windows test reliability. Focused CR-01/CR-01b
reviewer probes pass, as do queue and recovery UI checks. No new production
recovery defect is established by the remaining failures. Do not claim a
clean 403-test Windows library run.

## Passing evidence

- Reviewer probes in isolation: 2/2 (9.14s).
- Queue module in isolation: 19/19.
- Sign-in integration: 32/32 (220.38s); provider dispatch: 38/38 (97.25s).
- TypeScript exit 0; contracts drift exit 0; UI 347/347.
- Documentation tests 21/21; consistency validator 362 Markdown files before
  this review document was added.
- Offline frozen dependency installation with scripts disabled: exit 0,
  zero downloads. No tracked dependency changes.
- API and web builds: exit 0. Existing large-chunk, uint-format, color and
  unused-test-import warnings retained; no strict Clippy claim.
- Browser specs, each run separately under the shared lease: job-recovery
  1/1, connect-ai 1/1, DOCX 7/7, PDF 5/5, TXT/MD 9/9, Ask 1/1: total 24/24.
- Recovery interaction: the real rendered app against a synthetic mock API
  showed the diagnostic and attempt count, submitted exactly one `recover`
  action with observed version, and cleared the notice. This is frontend
  interaction coverage paired with Rust backend tests, not an injected-fault
  end-to-end backend browser test. Documents' own upload-driven recovery
  button remains indirectly covered through the shared component.

## WR-REC-TEST-01 — fault tests time out before observing parking

Exact command: `node scripts/cargo.mjs test --locked --workspace --lib`.
First run overlapped compilation, web/dependency setup and other verification.
API passed 4/4; core passed 331 with four failures (332.15s):

- `corrupt_claim_failure_parks_immediately_and_recovers_on_wake`
- `deliberate_wake_rearms_exhausted_root`
- `diagnostic_absent_for_other_states_and_roots`
- `diagnostic_shown_for_exhausted_queued_job_without_row_mutation`

Every failure is the helper at ingestion.rs:818: the root was not observed
parked within 10s; job remained Queued v2, log `Upload complete; queued for
import`. All four pass when each runs alone in its own process.

A second unchanged broad command was run after all reviewer builds, browser
tests and other test runs finished, with default Rust concurrency intact.
API passed 4/4; core passed 333 with two failures (170.79s):

- `deliberate_wake_rearms_exhausted_root`
- `diagnostic_absent_for_other_states_and_roots`

Both again fail the 10s parked wait at ingestion.rs:818. This quieter rerun
rules out dismissing the finding as solely overlapping build work. It does
not establish a production deadlock, lost wake, or a precise starvation cause.
The Cargo command stops at the core failure; remaining desktop/provider
library binaries were not executed by either broad invocation. Their expected
inventory must not be counted as passing.

The helper starts its 10s bound immediately while many fixtures share the
single process-global worker. There is no per-root claim-admission handshake.
Investigate and characterize that scheduling mechanism first. Establish
explicit worker admission/attempt/parking synchronization or suitable fixture
isolation while retaining an overall safety bound, cross-root fairness tests,
cancellation and exact attempt-count assertions. Do not merely lengthen the
deadline, sleep, weaken assertions, or claim serial execution alone proves the
default parallel broad command. Fix production only if a deterministic probe
establishes a production defect.

## Reproduction and preserved boundaries

Worktree: `E:\Anansi\worktrees\review-claim-recovery-final`.
CARGO_TARGET_DIR=`E:\Anansi\target`; TEMP/TMP=`E:\Anansi\tmp`.
No owner data, running pilot, sign-in gate, credentials, or cloud inference
was used or changed. Shared browser lease released. No promotion or push.

Isolated runs used the test binary produced by the broad command, preserving
its compiled feature set. Original failures and all isolated/rerun logs are
included separately. The earlier fully accepted 370-test checkpoint retains
its recorded scope; this runtime is not promoted.
