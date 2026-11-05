# Windows release tooling — focused repair acceptance — 2026-10-30

Runtime repair6290f9305, docs tip029e9e51f, reviewer baseline628745347. Source changes stay within release-tooling directories. Existing reviewer probe files are unchanged. No application implementation changes by reviewer or integrator.

## Focused acceptance

WR-01 examples now refuse premature desktop exits, header-only/corrupt/non-ANANSI catalogs, marker-only roots and changed/deleted baseline workspace/settings records. Readiness requires workspaces.sqlite3 and checks supported user_version, the expected ANANSI workspace/settings table columns and application identity policy. Read-only SQLite integrity checks remain. Process enumeration failure returns unconfirmed cleanup and retains a blocked lease; the owned descendant inventory and tests are retained. WR-02 default exports exclude raw logs and validate selected structured field types/bounds. WR-03 historical baseline and reproducibility distinctions remain documented.

Independent compatibility control: copied a genuine schema10 catalog previously produced by ANANSI's public API in synthetic test data (E:/Anansi/benchmarks/m8-lineage-repair-windows-2026-10-30/schema10-source-retry/storage/workspaces.sqlite3) into a new E: test root. It passed catalog/readiness and unchanged snapshot persistence with1 workspace. Separate copy: changing its real workspace name refused persistence; deleting workspace/settings records refused persistence. Original source fixture untouched. This is helper-level compatibility/mutation evidence, not a new application launch/restart test.

Serial test command: node --test --test-concurrency=1 tests/windows-release/*.test.mjs tests/docs-validation.test.mjs . Exact result recorded below; do not run shared-lease test files concurrently. Logs: E:/Anansi/artifacts/release-review/storage-repair-tests.log.

## Practical limits and integration boundary

Focused qualification of tooling does not certify a full native release, installation/uninstallation, every historical schema variant or a current application workflow. No fresh desktop build or successful native launch/restart was performed in this review. The release branch still contains its older application baseline; integrate only the assigned release-tooling directories onto the current verified application branch, then run the real build/launch checks there. Do not merge the old application checkout over Muse's newer runtime. Optional expectedWorkspace checks must be supplied in an integrated known-record acceptance; empty startup catalogs alone do not prove record survival.

Installer Sandbox/VM validation, code signing, current-preview promotion, M5/G1/remote-CI/generated-execution gates remain open. Tests used local E: synthetic state only. No owner-data/preview migration, credentials, paid/cloud inference, installer, pushes or CI. Shared lease must be confirmed absent before closing this handoff.

Final independent serial result:74/74 passed,80.42s. Genuine catalog compatibility and changed/deleted controls passed as described. Shared application-test lease absent at completion. Ready for controlled integration of tooling only; no new blocker found within this focused review scope.
