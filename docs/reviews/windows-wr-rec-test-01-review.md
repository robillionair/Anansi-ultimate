# WR-REC-TEST-01 Windows review — 12 November 2026

Incoming tip: `e32456574f9ef434d4657f6f44784c0ed7baf370`.
Baseline: `01ee5a74d9b4866352525284e963ae17be5716fa`.
ZIP SHA-256 matched
`7eaf07565503b7880f334b33513984a37c566a2d3a3dae5d9c7ffeae78e13432`.
Incremental bundle verification and exact identity checks passed. Review used
this exact delivery, not a substitute baseline. Current published Corporate
planning commit `dff9bb166` remains separate and untouched.

## Verdict

Changes required: one default-concurrency Windows fault test still fails.
This is not a clean 405-test Windows acceptance. No production recovery defect
has been established. New instrumentation improves the diagnosis but does not
yet resolve the pre-admission waiting case.

## Broad command and decisive evidence

```text
node scripts/cargo.mjs test --locked --workspace --lib
```

No other reviewer build or test ran alongside this command. API passed 4/4.
Core passed 336 and failed 1 (204.70s):
`ingestion::tests::deliberate_wake_rearms_exhausted_root`.
Cargo stopped at the core failure; desktop/provider library test execution
must not be counted as passing from this invocation.

The panic at ingestion.rs:888 reports:

```text
no admission/attempt progress within the 10s stall bound after 10.0073093s
job state Queued v2 log="Upload complete; queued for import"
admitted 0 -> 0, attempted 0 -> 0, remaining faults 1000000
progress history (elapsed_ms, admitted, attempted): [(0, 0, 0)]
```

Thus no injected claim attempt occurred before this wait failed. Calling the
same 10-second limit a stall bound still starts that clock before the root's
first admission. The counters do not yet distinguish a registered pending root
from a root that was never scheduled. The previous finding is narrowed to that
case; it is not evidence that the retry implementation performed a bad claim.

## Follow-up acceptance

Use explicit test-side worker enrollment/pending/active/attempt state (or a
deterministic equivalent) to distinguish legitimate pending work from an
unregistered/lost wake or an admitted worker that stops progressing. Keep a
bounded overall deadline and effective stall detection; do not simply enlarge
the stall bound, disable the failing test, or replace the default broad command
with serial execution.

Add deterministic boundary coverage for: registered pending with zero admissions;
never registered; admitted with no further progress; retries returning to the
queue; overall expiry. Preserve existing exact attempt counts, reviewer probes,
fairness, and cancellation checks. Change production only if a separate probe
proves a production defect. This is still a test-focused repair.

## Other observed results and scope

- The other formerly failing test,
  `diagnostic_absent_for_other_states_and_roots`, passed in the broad command.
- The new twelve-root contention test passed in the broad command.
- The new never-admitted test passed with its expected panic.
- The failing named test passed in isolation: 1/1, 7.89s.
- Both unchanged reviewer probes passed in isolation: 2/2, 9.77s.
- Queue tests passed in isolation: 19/19.
- Prior UI/provider/browser evidence belongs to the earlier runtime review.
  Those unchanged workflows were not rerun here; no new 24/24 browser claim.

The source diff is only test helpers, cfg(test) instrumentation, and two handoff
documents. No production, provider, contract or UI changes were requested or
made by the reviewer. Owner data, pilot, OAuth gate, Corporate plan and GitHub
default branch are untouched. No inference, credentials, downloads or push.

Worktree: `E:\Anansi\worktrees\review-claim-recovery-final`.
CARGO_TARGET_DIR=`E:\Anansi\target`; TEMP/TMP=`E:\Anansi\tmp`.
Raw broad and isolated logs accompany this review bundle. Isolated runs used
the test binary produced by the broad command, preserving its feature set.
