# Independent Windows release-tooling review — 2026-10-30

Incoming branch codex/antigravity-release-engineering at `9ceb1c5eeb90180ae9e8ce895a09c0e6500b98ae`, baseline `240af4ef58b09e7ada405286f46f927c902f9c52`. Delivery ZIP SHA-256 matches `b78ce9ddb2829dc97e39fefb1a3348fc57d725db888aa36c5ab1fdbf4d0f20d5`. Source diff stays in the three assigned directories; owner worktree clean. Existing automated tests independently pass 24/24.

Verdict: changes required; no integration or production readiness certification.

## WR-01 (P1): launch qualification reports success for failed processes

`launch-acceptance.mjs` unconditionally returns ok:true after collecting evidence. A controlled run of Windows where.exe (no arguments) exits with code2 in both phases, creates zero files and returns persistenceVerified:false, yet qualifies as ok:true. This exercises failed-process handling without running generated software. Preserve failed evidence but return failure for early/nonzero exit, failed readiness/persistence, spawn errors and unsuccessful teardown. Merely surviving a timer or comparing directory entry counts does not verify database integrity, restart re-admission or actual application health. Define the scope honestly and verify synthetic application state/readiness for stronger claims. Track process close/cleanup and confirm exit before releasing the lease.

## WR-02 (P1): diagnostic export includes arbitrary log text

The diagnostic object includes `sanitizedBuildLogExcerpt` containing free-form build log lines filtered by finite regexes. A synthetic ordinary sentence with no credential-like syntax survives `collectDiagnostics`, despite the privacy notice guaranteeing memories/prompts/secret content excluded. This is a denylist, not an allowlist. Default support export should include bounded structured safe fields only, excluding raw excerpts and arbitrary error/manifest/tool output. Do not fix by adding the review canary to regexes or supplying it as extraCanaries. If raw-log opt-in is proposed, define it separately with explicit content review and truthful limitations; no automatic transmission.

## WR-03: stale baseline and overclaims

R1–R4 are historical failures on this older runtime. Current independent lineage acceptance is recorded on c50d6b03 and schema11 runtime06a3f573a. Do not overwrite newer application code from this worktree, or claim its older binary is the current application. Repeatability report also says both five-of-six sections match and all sections identical: reconcile against measured byte/section differences, and read COFF timestamps relative to the PE header offset stored at0x3C. Hash-different executables are not bit-identical reproducible builds. Installer install/uninstall remains untested absent disposable VM evidence.

## Reproduction

`node --test tests/windows-release/reviewer-probes.test.mjs` fails both expected-success assertions (0/2). Original24 tests pass. New tests/docs only in reviewer branch; no production/tool implementation changed. Review isolated on E:, owner previews/data and Antigravity checkout untouched; shared lease released. No builds, cloud calls, credentials, CI, push or preview promotion during this focused review.
