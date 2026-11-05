# Windows release tooling repair2 — independent review

Incoming `0df905a252c789b70a603770e8faed6877427408`; ZIP SHA256 matches `451acf3db6838c1360350639017823a71789f099ddf05084c5fbf8480909fd27`. Original reviewer files unchanged. All35 existing tests independently pass on Windows. Diagnostic type/bounds repairs close the original WR-02/WR-02b examples in this focused scope; early-exit review probes also pass. No application integration or native release certification.

## Remaining WR-01c: catalog recognition is not catalog integrity

The reviewer created a synthetic workspaces.sqlite3 containing only16 bytes: SQLite format3 plus NUL. Both exported helpers return success: readiness.ready:true and persistence.persistent:true. This is not a valid SQLite database and contains no ANANSI schema/state. Two new expected-success tests fail0/2 in reviewer-catalog-integrity.test.mjs. Recognized markers are also optional alternatives (even pairing.json alone can currently qualify), rather than a defined application storage contract.

Required repair: use a real read-only SQLite integrity/schema check or a trusted application readiness protocol against test-owned state, and verify a known synthetic identity/value after restart. Retain marker/header checks as preliminary checks only. Check every required artifact and reject symlinks/reparse points; do not certify readiness based on a lone arbitrary marker. A startup-only mode may instead remain explicitly weaker, but must not label its result verified database integrity or persistence. Do not solve only by raising a file-size threshold: a padded malformed database must still fail.

## Cleanup verification remains incomplete by inspection

getDescendantPids returns[] for a failed/timed-out PowerShell enumeration, conflating unknown with no descendants. ensureProcessTreeTerminated kills the parent before inventory and queries only direct children; deeper descendants may be orphaned if the root exits. Preserve a recursive owned-process inventory while the root is alive, identify processes robustly, and treat enumeration failure as unconfirmed cleanup. Retain the shared lease whenever absence of owned processes cannot be confirmed. This review has not intentionally left an orphan; this is a source-level gap in the claimed guarantee. Add deterministic enumeration-failure, parent-first-exit and grandchild coverage with cleanup of all test-owned processes.

## Scope

New reviewer tests/docs only; no implementation changes. No installer, builds, host launch, cloud, credentials, push, CI or preview promotion. E: synthetic fixtures only. Shared lease absent at completion. Antigravity's original checkout remains untouched; this review uses a separate branch in the review checkout. Keep release tooling separate from current schema11 application integration until acceptance.
