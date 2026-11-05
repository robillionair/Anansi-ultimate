# Release storage contract review — 2026-10-30

Incoming repair f5155b390 + docs640d81cb8, baseline f892d718b. Reviewers' earlier tests unchanged; source changes stay in assigned directories. Positive: actual read-only SQLite integrity checks reject header-only and padded corrupt databases; bounded diagnostics and premature-exit refusals preserved. Cleanup now inventories descendants and retains an unconfirmed lease on enumeration failure; this is an improvement, not a complete native-release certification.

## Remaining WR-01d: generic valid storage is still treated as ANANSI state

Two independent controls fail expected-success assertions:
1. Synthetic root containing only .anansi-root with one byte 'x' => readiness.ready:true, without a database.
2. File named workspaces.sqlite3 containing a valid SQLite database with only CREATE TABLE unrelated(value TEXT) => readiness.ready:true. Persistence also accepts this unrelated database.

The integrity function rejects only when both table count and user_version are zero, so neither an ANANSI schema nor actual application records are required. The prior requested known synthetic value/identity verification across restart is absent; two separate valid database checks do not prove that data survived.

Required bounded completion: define the actual required ANANSI catalog/schema contract for the runtime being tested, including schema version and required tables/columns; require that catalog for application readiness (marker-only state may be reported as initialization-only, never full readiness). Verify the same known synthetic workspace/record identity and value after restart using a trusted read-only or existing API path. Test deleted/changed/replaced data that remains structurally valid SQLite. Do not special-case test names or make another file-size heuristic. Preserve all passing integrity, privacy and cleanup fixes.

New reviewer tests: tests/windows-release/reviewer-storage-contract.test.mjs,0/2 as expected on incoming implementation. Reviewer source unchanged except new tests/docs. Local files available directly; no ZIP transfer required.

## Verification accounting

The first combined invocation ran test files concurrently; reviewer-probes hit an occupied shared test lease and failed before launch (undefined evidence, not a launch regression). No lease was stolen. The serial command uses node --test --test-concurrency=1 and logs separately. Document that command as the canonical shared-lease suite runner; do not weaken occupied-lease refusal. Original diagnostics retained in E:/Anansi/artifacts/release-review/final-repair-tests.log; serial results in final-repair-tests-sequential.log. No installer, application build/promotion, cloud, credentials, pushes or CI. E: synthetic fixtures only.

Final serial result:65/65 passing,71.66s. New storage-contract controls remain0/2. Shared lease absent at completion. Existing suites were frozen before the two new probe files were added to the invocation; do not label the full expanded suite green.
