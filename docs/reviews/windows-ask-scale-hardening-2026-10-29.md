# Windows review of Ask scale hardening — 2026-10-29

## Verdict

Batching materially improves the unchanged Windows probe: one Ask on 16 notes
took **6,910 ms**, versus **27,637 ms** in the pre-batch review. These are single
observations, not a new p95 or a large-corpus speedup guarantee. Seeding took
47,827 ms; the diagnostic completed in 57.48 seconds with the correct fact and
one citation. The original probe source is byte-identical to the reviewed parent.

Windows acceptance is **not complete**: the permission-after-proof reproduction
fails, the accounting test fails on Windows, and strict all-target Clippy fails.
Full 1,008-record Windows runs were not started while these defects remain.
No application code was repaired or preview promoted during this review.

Incoming: `muse/ask-scale-hardening` at
`825f75614656aa0118d924fc5ae1fe51b3a9cf7e`, from `9b9ead8f` and reviewed `8bdcbae1`.
ZIP SHA-256 `0c8b50faeae93305123b4807be777ec0301614f77a7b6334d3c3bb92c75ef60c`;
bundle SHA-256 `0535d42d4312b41082f7514fb23b7130270f8d30939af97e15cbc7c3b5eef486`.
Hashes match; bundle verifies and fetch lands on the exact tip.

## R1 — Permission widening after proof remains readable on Windows

`VerifiedPaths::read_verified` bypasses all permission verification for covered
paths on Windows. The added read-time permission recheck is Unix-only. The
incoming unit `verified_paths_batch_fallback_and_revocation` explicitly expects a
widened Windows file to remain readable through the old proof. That unit passes
on Windows, confirming the behavior rather than establishing refusal.

The review adds an explicit, opt-in expected-success reproduction:
`content::tests::windows_widen_after_proof_must_fail_closed`.
It creates synthetic content, proves the path, widens its ACL using icacls, then
requires the protected read to fail. **Observed result: FAIL**, 3.70 seconds,
because the read succeeds. No private user data is involved.

The test is ignored by default only because it is an intentional open-defect
reproduction. Its explicit failed run is retained and is not counted as passed.
The expected refusal must remain unchanged when repairing the implementation.

This does not assert that every filesystem race was absent before batching.
It demonstrates that the delivered operation-scoped proof authorizes a read
after a relevant permission change, contrary to the preserved-check requirement.
Documenting a residual window does not itself approve that behavior for Windows.

Required repair: validate current owner/DACL and covered directory/file conditions
at the protected read without returning to per-file PowerShell process launches.
Native verification or correctly bound handle-based validation should be evaluated.
Preserve reparse/link refusal, hashes, size limits, privacy and fresh-operation
checks. Do not turn the reproduction into an expected successful read.

## R2 — Permission accounting contains a Linux-only assertion

The unmodified accounting test completes the zero-note sample, then fails at
one note:

| Notes | Ask ms | Batch calls | Batch paths | Single calls | Root / DB |
|---:|---:|---:|---:|---:|---:|
| 0 | 4,376 | 0 | 0 | 8 | 4 / 4 |
| 1 | 5,483 | 1 | 4 | 8 | 4 / 4 |

It asserts `single == batch_paths + 8`, expecting 12 at one note, but Windows
batch paths are checked inside the one batch process and are not counted through
the Rust per-path function. Eight is the correct observed single-call count.
Unix counts its per-path metadata checks differently. Fix the platform-specific
expectation rather than changing the implementation to inflate counters.

This run stopped at the failing assertion. It did NOT measure 16/128/1,010-note
accounting. The source maps the successful clean call path to eight individual
process invocations plus one batch invocation; the counters are not an external
OS process trace. Empty Ask still incurs eight root/catalog checks and took
4.376 seconds in this sample, so constant connection overhead remains material.

## R3 — Strict Clippy is not green on Windows

`clippy -p anansi-core --all-targets --locked -- -D warnings` exits 101 for:

- `PermissionCheckKind::BatchPath`, unused on Windows.
- `record_permission_recheck`, unused on Windows.
- Adversarial constants `REC3` and `REC4`, used only by Unix-gated tests.

Use appropriate platform gating or real uses, not warning suppressions. The
review preserves the failing Windows output; Linux's green result remains Linux
evidence. More findings could appear after these first errors are corrected.

## R4 — New realistic-corpus helper stops at the first phrase occurrence

Static review of `ask_realistic_corpus_acceptance::span_bound_to_source` finds
an early `return` on the first chunk containing a span. If that occurrence is
in another source and a later chunk correctly binds it, the helper returns false
without examining the correct chunk. This can produce false negatives. The
older strengthened large-scale helper correctly searches all matching chunks.
Align the logic and add a pure two-occurrence regression. Do not change questions
or gold facts to avoid collisions. No claim is made that this affected the
reported 68/68 Linux run.

## Checks actually completed

- Incoming checksums, bundle verification, exact fetch, and preserved probe source.
- Unchanged public Windows probe: 1/1 content/budget checks pass; timing above.
- Incoming proof/fallback/revocation unit: 1/1 passes its delivered assertions.
- Content adversarial tests available on Windows: 4/4 pass (failed-batch cleanup,
  outside-set/root fallback, body corruption/missing/oversize, new-revision fallback).
- Source-binding wrong-source regression: 1/1 passes on Windows.
- Explicit Windows permission-widening refusal reproduction: 0/1, failed as above.
- Incoming accounting: 0/1, fails at one note as above.
- Strict all-target Windows Clippy: failed as above.
- Formatting, documentation consistency and Git diff checks run for the review.

Full 80-question/1,008-record Windows acceptance and reparse-specific tests remain
open. No Windows p95, native acceptance, preview promotion, model-answer score,
token saving, G1 audit, CI or generated-execution certification is inferred.
The five reported Linux full-core failures remain recorded as delivered; this
review did not rerun or reclassify them. In particular, preserve the original A6
question and evidence when reconciling its partial-match versus empty expectation.

## Reproduction and handoff

Set TEMP/TMP and CARGO_TARGET_DIR to E:; all tests use synthetic temporary roots.

```powershell
node scripts/cargo.mjs test -p anansi-core --locked --test ask_windows_scale_probe -- --test-threads=1 --nocapture
node scripts/cargo.mjs test -p anansi-core --locked --test ask_permission_accounting -- --test-threads=1 --nocapture
node scripts/cargo.mjs test -p anansi-core --locked --lib content::tests::windows_widen_after_proof_must_fail_closed -- --ignored --exact --nocapture
node scripts/cargo.mjs test -p anansi-core --locked --lib content::adversarial_tests:: -- --test-threads=1 --nocapture
node scripts/cargo.mjs test -p anansi-core --locked --test ask_large_scale_acceptance wrong_source_phrase_cannot_satisfy_binding -- --exact --nocapture
node scripts/cargo.mjs clippy -p anansi-core --all-targets --locked -- -D warnings
```

Read-only incoming package:
`E:/Anansi/reviews/ask-scale-hardening-2026-10-29`.
Raw Windows evidence:
`E:/Anansi/benchmarks/ask-scale-hardening-windows-2026-10-29`.
Review branch: `codex/review-ask-scale-hardening`, preserving `825f7561`.
Only an opt-in test reproduction and review documentation are added. No runtime
behavior, owner data, external credentials, master, previews, remote CI or audit
monitor were changed; no cloud calls or generated-software execution.
