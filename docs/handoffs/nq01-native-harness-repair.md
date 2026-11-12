# NQ-01: native qualification harness assertion repair

Bounded harness-only repair. No application-code
changes, no rebuild, no installer/native approval
claimed. The built application's recorded source and
hashes are preserved (see "Preserved build identity").

## Finding

Windows build review `7662162` (source `6f216e3`,
clean) ran `scripts/native-candidate-acceptance.mjs`
unchanged. The script exited 0 and printed all seven
checks passed, but check [5/7] printed:

```text
[5/7] Registered artifact ID: undefined
```

`scripts/native-candidate-acceptance.mjs` read
`artRes.artifact.id`, which does not exist on the
artifact response, and asserted nothing about the
result. An independent post-run read-only SQLite
inspection confirmed exactly one `Synthesized Idea
Artifact` row at version 1
(`artifact_id 413c75aa59a09ef9637d3df347049b78`), so
this is an assertion gap in the harness, not an
established application defect. The blanket 7/7
banner must not be used as full native acceptance.

## Contract findings (inspected before code)

`crates/core/src/artifacts.rs`:

- `artifacts_command` `register` returns
  `ArtifactDetails { contractVersion, artifact:
  ArtifactRecord, body }`. `ArtifactRecord`
  serializes camelCase with `artifactId`,
  `projectId`, `type`, `version`, `title`,
  `timestamp`, `sourceReferences`, `generator`,
  `validationStatus`, `privacy`, `contentHash`.
  There is no `id` field and no `workspaceId`
  field on the record. The body is the top-level
  `body` field of the details response, not a
  record field.
- `artifacts_command` `read` takes
  `{ contractVersion: 1, workspaceId, artifactId,
  version? }` and returns the same `ArtifactDetails`
  shape. The read is workspace-scoped: both the
  pinned-version query and the latest-row query
  filter on `workspace_id`, and a miss returns
  NotFound ("Artifact version not found in this
  workspace."). Registration itself returns the
  result of a version-1 read, so a fresh
  registration always has version 1.
- Identifiers use the ANANSI id format: exactly 32
  lowercase hex characters
  (`crates/core/src/workspace.rs` `validate_id`;
  artifact ids are 16 random bytes rendered hex).
- Workspace membership therefore cannot be asserted
  from a response field that does not exist. It is
  asserted instead through the workspace-scoped read
  (the request carries the workspace id and the
  storage query filters on it) plus a negative
  probe: reading the same artifact id under a
  second fresh workspace must fail.

`apps/desktop/src-tauri/src/lib.rs` and
`crates/core/src/readiness.rs`:

- A `get_readiness` Tauri command is registered. It
  takes `{ request: { contractVersion: 1 } }` (see
  `packages/contracts/src/client.ts`) and returns
  `{ contractVersion, applicationName, applicationVersion,
  apiVersion, coreVersion, operatingSystem, uptimeMs }`.
  The harness never invoked it, so its [1/7]
  "Startup & Readiness verified." message was
  unconditional. This is repaired in the same
  bounded scope: same assertion-gap class, one
  existing command, no new surface.

## Acceptance criteria (committed before code)

- AC-NQ1: the harness reads the real identifier
  field `artifact.artifactId` and never `.id`. A
  missing or invalid identifier fails check [5/7];
  a regression test proves it.
- AC-NQ2: after registration, the harness reads the
  exact artifact through native IPC
  (`artifact_command`, `read`) and asserts
  `projectId`, `type`, `version`, `title`, and the
  top-level `body` equal the registered values.
  Workspace scoping is asserted by a read of the
  same artifact id under a second fresh workspace,
  which must fail.
- AC-NQ3: after the existing process restart, the
  harness reopens the same artifact id via `read`
  and asserts its identity and content persist
  (same fields as AC-NQ2).
- AC-NQ4: check [1/7] invokes `get_readiness` and
  asserts `contractVersion`, `applicationName`,
  `operatingSystem`, the version fields, and
  `uptimeMs` before printing its pass line.
- AC-NQ5: all other numbered checks, isolation,
  lease handling, teardown, poll bounds, and
  timeouts are unchanged. No assertion is weakened
  and no timeout is increased to hide a failure.
- AC-NQ6: regression tests run on Linux and prove
  that a missing/invalid identifier and each field
  mismatch fail the check. Before/after evidence is
  recorded: the old field access reproduces
  `undefined` with no failure, and the regression
  suite fails against a helper mutated to the old
  field access.
- AC-NQ7: no application-code changes: the diff
  touches only the harness script, its assertion
  helper, its tests, and documentation. The built
  application's recorded source and hashes are
  preserved and restated in this document.
- AC-NQ8: Linux checks are run and their limits
  reported honestly. Native Windows execution
  remains for the Windows reviewer. Delivery is a
  checksummed bundle ZIP with MANIFEST and raw
  logs, scratch-verified, with a credentials sweep.

## Other numbered checks (audit, step 7)

Inspected for the same unconditional-success class:

- [2/7] shell denial: concrete assertion
  (`ALLOWED` fails the run). Left unchanged.
- [3/7]/[4/7]/[5/7]-project/[6/7]-job ids: each id
  is consumed by a later step that fails closed if
  the id is missing or malformed (memory search
  match, artifact registration id validation,
  backup-completion poll). Only the artifact id was
  never consumed downstream, which is exactly the
  NQ-01 gap. Left unchanged; no new failure modes
  introduced.
- [6/7] backup: completion is asserted by the poll.
  Left unchanged, including the poll bound.
- [7/7] screenshot: `page.screenshot` throws on
  failure. Left unchanged.
- Final banner: prints only if no step threw. Left
  unchanged. It still must not be read as full
  native acceptance (manual CredUI, file-dialog,
  installer, and clean-machine items remain
  separate checklist items).

Nothing found here requires separate scope.

## Preserved build identity

Built application source (unchanged):
`6f216e3224e2c855236aac5492f69b467cf8c5d5`, clean at
build. Windows build review:
`766216263a0c5860b35441ca97334a4cf0799e42`.
Recorded hashes from the review package
(`packaging.json` / `release-manifest.json`,
cross-checked, installer not included in the review
package):

- Installer:
  `2b0c1c7980b7cf8c62b1d9b621723276ef943208d988ea579a62494da5408d74`
  (`ANANSI Ultimate_0.1.0_x64-setup.exe`,
  10,547,273 bytes, NotSigned)
- Standalone app:
  `47e1695d95325f574970331f89d514bb797669dc00021ded50646868901b2a7c`
  (`anansi-desktop.exe`, 26,051,072 bytes)
- NSIS payload app:
  `2f6bbd2b0572fca2a8e9d3d9fc616d6667a962b193c72eb7ee5ecd3ff6816580`
- Config:
  `8868aded25eb36066561ceb3a3404b01219dda839b14985a571d4a794dd77c4d`

No application rebuild is needed solely for this
harness repair. The reviewer reruns the harness
against the already-built standalone binary above.

## Windows rerun commands

Fresh isolated storage/UDF directories (the script
refuses non-empty ones), from the repository root at
the delivered tip:

```powershell
$env:ANANSI_CANDIDATE_BINARY = 'E:\Anansi\qualification\artifacts\anansi-desktop.exe'
$env:ANANSI_CANDIDATE_STORAGE_ROOT = 'E:\Anansi\qualification\native-2026-11-13\data-rerun'
$env:ANANSI_CANDIDATE_WEBVIEW_UDF = 'E:\Anansi\qualification\native-2026-11-13\webview-udf-rerun'
$env:ANANSI_CANDIDATE_EXPECTED_SHA256 = '47e1695d95325f574970331f89d514bb797669dc00021ded50646868901b2a7c'
node scripts/native-candidate-acceptance.mjs
```

Expected: exit 0, all 7 checks pass, and the [5/7]
line prints a 32-hex artifact id (never
`undefined`), followed by the readback and
workspace-scope lines and a post-restart artifact
reopen line. Record the full output verbatim. The
regression suite can also be run on Windows:

```powershell
node --test tests/native-artifact-acceptance.test.mjs
```

## Linux limits

The harness itself requires Windows and refuses to
run on Linux (platform guard). Linux evidence covers
the assertion helpers and their regressions, syntax
checks, and the repository test/docs suites. Native
execution, CDP behavior, and the new IPC readback
against the built binary are Windows-only and remain
for the Windows reviewer.
