# Pre-admission repair Windows review — 12 November 2026

Runtime: `606450202a735e565f6721fa78caab38d539704b`.
Parent review: `7cf1068b1fae2ce1410184ad5a3447f6e765cb64`.
Incoming ZIP SHA-256 matched
`6b0adf62dc6f032865024b902c63876568366abdd480640e0a2fb02f34d48edb`.
Bundle verification and exact identity checks passed. Reviewer changes are
documentation only. Production code is unchanged by the incoming repair.

## Verdict

The formerly failing parking-timeout cases pass in the default-concurrency
Windows broad run. Both original reviewer probes pass. Full acceptance remains
changes required because a new integration fixture has an observation race.
There is no clean 414-test Windows acceptance claim.

## Observed results

Command: `node scripts/cargo.mjs test --locked --workspace --lib`.
No other reviewer build/test ran alongside it.

- API: 4/4 passed.
- Core: 345 passed, 1 failed (271.42s).
- The only failure is
  `ingestion::tests::parking_wait_survives_pending_before_first_admission`.
- `deliberate_wake_rearms_exhausted_root` and
  `diagnostic_absent_for_other_states_and_roots` both passed in that broad run.
- Both original reviewer probes passed in that broad run.
- New scripted phase-machine tests and never-registered expected-panic case
  passed in that broad run.
- Cargo stopped after the core failure. The compiled desktop/provider library
  binaries were then run separately: desktop 1/1, providers 63/63 passed.
  Those separate runs are not a clean aggregate Cargo invocation.
- Separate queue/mirror module run: 22/22 (19 queue + 3 mirror tests).

## WR-REC-WITNESS-01 — pre-admission witness relies on setup speed

The panic at ingestion.rs:2471 reports:

```text
wait observed pending before first admission:
[(0, false, true, 2, 1), (77, false, true, 3, 2)]
```

Tuple fields are elapsed milliseconds, pending, active, admissions, attempts.
The observed root was already active and had been admitted twice when sampling
began. It ultimately parked with the asserted exact admissions, attempts and
fault consumption; the later witness assertion failed. This is not an expired
parking bound or demonstrated production failure.

The test creates and wakes thirty roots sequentially, then begins observing
the last one. Its comment assumes fixture setup takes milliseconds and that
the other roots therefore force a pending-before-first-admission observation.
That relative-speed assumption does not hold on this Windows run. More roots
or a delay cannot provide the required synchronization guarantee.

Use an explicit cfg(test), root-scoped pre-admission gate or equivalent
deterministic handshake. Hold the watched root pending, start its observer,
confirm it records pending with zero admissions, then release it. Do not block
while holding the worker queue mutex. Gate cleanup must release on failure,
panic and timeout and preserve bounded completion and other-root concurrency.
Retain the witness assertion, exact attempt counts, both reviewer probes,
scripted stall/overall-bound tests and cancellation coverage. No production
scheduling change is requested or justified by this finding.

## Scope and reproduction

Worktree: `E:\Anansi\worktrees\review-claim-recovery-final`.
CARGO_TARGET_DIR=`E:\Anansi\target`; TEMP/TMP=`E:\Anansi\tmp`.
The separate binaries are the ones built by the broad Cargo command, preserving
its compiled feature set. Raw logs accompany this review bundle.

UI, provider integration, browser and live-model workflows were not rerun for
this test-only change; prior results retain their original checkpoint scope.
No real credentials, inference, model downloads, preview promotion, or push.
The default branch's Corporate planning commit `dff9bb166` and owner pilot
remain untouched.
