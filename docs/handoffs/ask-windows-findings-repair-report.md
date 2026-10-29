# Ask Windows findings repair — report (2026-10-29)

Branch: `muse/ask-windows-findings-repair`, from reviewed
`6e355b02d5aa46f1aa5646b674edb01cd6ec2884` (review branch
`codex/review-ask-scale-hardening`, one review commit over the
delivered hardening tip `825f7561`). The reviewed tip is preserved
untouched; this branch is separate. No cloud calls, credentials,
paid inference, model downloads, pushes, remote CI, master merges,
preview promotion, or generated-code execution in this repair.

## R1 — Windows widen-after-proof gap: closed natively

The reviewer's reproduction
(`content::tests::windows_widen_after_proof_must_fail_closed`)
proved a batch proof authorized a read after the file's ACL was
widened with icacls, because the read-time recheck was Unix-only.
The repair validates current permissions at every proven read on
Windows, in-process, with no per-file process launch:

- New `crates/core/src/windows_security.rs` (Windows only) uses
  the `windows-sys` 0.45.0 crate (already in the lockfile via
  keyring; the lockfile gains one edge, no new package versions).
- For the file and its proven ancestor directories (record,
  workspace, content), it checks exactly the `protect.ps1`
  verify-only invariant: owner SID equals the current process
  user's SID, the DACL is protected (inheritance disabled), at
  least one rule exists, and every rule is (current user, Allow,
  FullControl). Any API failure fails closed; a missing path
  maps to integrity, matching the deleted-file behavior.
- `read_verified` counts one recheck per proven read, covering
  file plus directories, on both platforms. The Unix recheck was
  extended the same way (file mode 0600, directory modes 0700);
  its counter semantics are unchanged (rechecks = N + cited).
- The old unit's Windows arm no longer expects a widened read to
  succeed; the reviewer's reproduction keeps its refusal assertion
  byte-identical and is now a standing regression test instead of
  an ignored probe. The batching speed improvement is untouched:
  batch verification is still one PowerShell process per Ask, and
  the recheck adds only in-process Win32 calls.

Verification available on this Linux machine: the module compiles
and passes strict Clippy (`-D warnings`) for the
`x86_64-pc-windows-msvc` target in an isolated crate (the full
crate's Windows check is blocked here only by the missing MSVC
toolchain — `ring`'s build script needs `lib.exe`, and no mingw
is available in this mirror). The reviewer reruns the explicit
refusal test on Windows; Linux cannot execute Windows ACL
behavior, and no Windows pass is claimed from Linux evidence.

## R2 — Permission accounting: platform-specific expectations

The accounting test asserted `single == batch_paths + 8`, which
is Unix-only: Windows checks batch paths inside the one batch
process, so singles stay at the 8 connection checks regardless
of batch size (review-observed: 8 at 0 and 1 notes, batch 1,
paths 4). The fix changes expectations, not counters:

- Unix: singles == batch paths + 8; batch-path counter ==
  batch paths.
- Windows: singles == 8; batch-path counter == 0.
- Common: batches 1 per Ask (0 for empty), batch paths 2N+2,
  root/database 4/4, no fallbacks, rechecks == N + cited on
  both platforms (the recheck assertion is no longer
  Windows-gated, since R1 makes it true there too).

No counter was inflated to satisfy either platform. The constant
8-connection overhead on an empty Ask (4.4 s observed on
Windows) is unchanged by this repair and remains material; it is
a connection-handling question for a later round, not something
this finding asked to fix.

## R3 — Windows strict Clippy: platform gating, no suppressions

- `PermissionCheckKind::BatchPath` and its match arm are now
  `#[cfg(not(windows))]` (the variant is only constructed by
  the Unix batch loop).
- `record_permission_recheck` is now called by the Windows
  recheck as well, so it is no longer Windows-unused.
- Adversarial constants `REC3`/`REC4` are `#[cfg(unix)]`
  (only the Unix link-rejection test uses them).
- Linux strict Clippy is green; the new Windows module is
  strict-Clippy clean for the Windows target (see R1). The
  reviewer reruns full Windows Clippy; further Windows-only
  findings, if any, surface there.

## R4 — Realistic-corpus binding helper: search all chunks

The realistic helper returned on the first chunk containing a
span, so a wrong-source occurrence followed by a correct-source
occurrence reported false. It now matches the large-scale
helper: it builds a citation index map and returns true if ANY
matching chunk is bound to the declared source. A pure
regression (`binding_helper_searches_all_matching_chunks`)
covers wrong-then-correct, absent-source, only-wrong-source,
and chunk/citation-count-mismatch cases. No corpus question or
gold fact was changed. The review made no claim this affected
the reported 68/68 run; the rerun below confirms it did not.

## A6 — pre-existing benchmark invariant: transparent record

Not changed. The original question ("What is the Harbor
deep-water berth fee?"), corpus, and baseline failure are
retained in the evidence. The distinction this repair records
explicitly: a genuinely empty query (no candidate matches at
all) must return InsufficientEvidence, while a query with
partial matches honestly returns Offline partial context with
the retrieved evidence disclosed. The repaired retrieval's
partial-match behavior is the second case, not a refusal
failure, and no model-refusal claim is made without model
answers. Changing the benchmark's expectation remains an
owner/reviewer decision; this report documents the split so
that decision has the facts.

## Verification (Linux)

- Content unit tests (incl. adversarial): 7/7.
- Batch-proof integration: 4/4.
- Permission accounting: 1/1 (0/1/16/128/1010 notes; rechecks
  == N + cited after the directory extension).
- Realistic corpus: 2/2 (acceptance 68/68, 6/6, 6/6, 80/80,
  80/80, 0 leaks, p95 351.6 ms; plus the new regression).
- Large-scale acceptance: 2/2.
- Strict Clippy, formatting, contracts, TypeScript, docs
  validation, API and web builds: all clean.
- Full Rust core suite: 322 passed, 5 failed — the same 5 as
  the previous round, none caused by this branch (4 live-Ollama
  environment failures; 1 pre-existing benchmark A6, proven on a
  pristine baseline in the previous round and retained unchanged
  here). The run was split by a VM replacement at 23:25 (uptime
  1 minute afterwards): first part 43 binaries ok plus the 2
  failed binaries (284 passed), remainder 8 binaries all ok
  (38 passed, including vault_increment2's 280 s crash-matrix
  test), plus 0 doc tests. Both logs are preserved in the
  delivery evidence; no result is hidden by the split.
- Canonical browser Ask workflow: 1/1.
- Windows-target module check + strict Clippy: clean (see R1);
  full-crate Windows check blocked by missing MSVC toolchain
  here, reviewer step.

## Windows handoff

The reviewer reruns, in order: the explicit refusal test (now
un-ignored), the unchanged probe (speed improvement preserved),
the accounting diagnostic (new platform expectations), strict
all-target Clippy, then full-scale acceptance. Full 1,008-record
Windows runs remain open until those pass. No Windows p95,
native acceptance, preview promotion, model-answer score, token
saving, G1 audit, or CI certification is inferred from this
Linux evidence.
