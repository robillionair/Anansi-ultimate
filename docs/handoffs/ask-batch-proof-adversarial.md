# Batch-proof adversarial acceptance (frozen before new tests)

Baseline: `muse/ask-scale-hardening` at the WP1 commit (categorized
instrumentation + accounting diagnostic). The proof under test is
`content::VerifiedPaths` as used by one public Ask (Ask builds one
proof set, shared by rebuild and compilation).

## Claimed trust boundary (to be confirmed or repaired)

- Proofs are point-in-time for one operation: created fresh per Ask
  (or per rebuild when called without proofs), dropped at its end,
  never stored, never shared across operations, never a permanent
  positive cache. A permission change therefore takes effect on the
  next operation at the latest.
- At read time, every proven read re-validates links (all path
  components and the file) and the body hash against the database
  value, plus existence, `MAX_BODY`, and UTF-8. These read-time
  checks close content-substitution and path-swap races.
- Permission state itself is proven at batch time, not re-checked at
  read time. Whether that residual window is acceptable is exactly
  what the tests below must determine. If a test shows a read
  succeeding on a file whose permissions were widened after proving,
  that is a real race: describe it, repair the boundary where a
  bounded repair exists (a cheap read-time recheck where the
  platform allows one without a process spawn), and keep the
  residual explicit where no such repair exists. Merely adding a
  test around the existing implementation is insufficient.

## Cases (each must pass, or be classified)

1. Path outside the verified set: a properly permissioned outside
   file reads through the full fallback verification; an overbroad
   outside file fails. Proofs must never authorize it.
2. Another workspace or storage root: proofs for workspace A (or
   root 1) do not authorize workspace B (or root 2); the same
   fallback/fail rules as case 1 apply.
3. No reuse across operations: an Ask that succeeded, followed by
   widening a file, followed by another Ask, must fail the second
   Ask; repair restores success. Each Ask builds fresh proofs.
4. Files or directories replaced after verification: a replaced
   body fails the hash check; a file or directory replaced by a
   link fails link rejection.
5. Links/reparse points where supported: Linux symlinks (file and
   directory) are tested. Windows reparse points cannot be tested
   on Linux and are classified unsupported-on-Linux, not fabricated;
   the same `reject_links` code path (including the Windows
   reparse-attribute check) is the reviewer's step.
6. Permission changes after verification: widening after proving
   (race case above); tightening after proving must fail a fresh
   verification; repair restores verification.
7. Missing, oversized, and hash-altered bodies: each fails
   integrity, never a silent partial read.
8. Records becoming Sealed or otherwise ineligible: sealing or
   archiving a previously cited record excludes it from the next
   Ask; a revision reads the new revision through fallback with the
   new hash.
9. Empty and partially initialized workspaces: zero notes yields
   InsufficientEvidence; a workspace whose content directory was
   removed yields an error, never a silent empty answer; a missing
   workspace yields an error, never InsufficientEvidence.
10. Cancellation and cleanup during preparation: a failed batch
    leaves no trace (verify-only: it repairs nothing, creates
    nothing, retains no proofs). Batch verification itself is a
    single synchronous step with no cancellation hook (bounded by
    200,000 paths); cancellation lives at the Ask-stream level,
    which existing streaming tests cover. This boundary is
    documented, not tested as if it were cancellable.

## Classification

Every case ends in one of: verified (test passes on Linux),
unsupported-on-Linux (Windows-only, reviewer step), or unresolved
race (described with impact and repair status). No Windows evidence
is fabricated on Linux. At most two repair/retest cycles per
concrete defect.
