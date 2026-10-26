# M7 Slice 1 repair design — specification approval + portable export paths

Date: 2026-10-26 · Bounded repair of M7 Slice 1 (tip `a439bf0`).
Baseline: `a439bf05add27eba9619fd7218191528663caddf` on
`muse/m7-slice1-product-forge`. No new features, no sandbox,
no generated-code execution, no iframe preview, no dependency
installation, no deployment.

## Gap 1 — renewed specification approval was not enforced

Reproduction (owner diagnostic): generate and save
specification v1, revise it through the ordinary artifact
revise operation to v2, call `forge-build-start` with
`specVersion: 2` — generation was accepted with no new
approval action. In Slice 1, saving a specification draft
was treated as approval, so any later version inherited
that treatment. That is the defect: approval was a UI
gesture, not backend state bound to an exact version.

### Approval model (backend-enforced)

- Approval is a first-class, immutable registry record, not
  a client flag:
  - New artifact type `specification-approval`, created
    **only** by the dedicated `forge-spec-approve` operation.
    It is deliberately **not** in `ARTIFACT_TYPES`, so the
    generic manual `artifacts/register` operation refuses it;
    a generic save or revision can never mint one.
  - Body format `anansi-specification-approval` v1:
    `specArtifactId`, `specVersion`, `specContentHash`.
    The row also carries the project id, workspace id and
    the `artifact://<spec>@v<n>` source reference, so the
    approval is bound to the exact workspace, project,
    specification identity, version and content hash.
  - The content hash is the specification version's stored
    `contentHash`, which `artifact_read` re-verifies against
    the stored body before it is ever returned — so the
    approval binds to bytes the registry itself has
    integrity-checked, not to a client-supplied hash.
- Gating in `forge-build-start`, **before any provider
  traffic**: after the pinned specification version is read
  and format-checked, the backend requires a matching
  approval record. No match → `InvalidRequest`, no run is
  created, no provider stream is started. A client-supplied
  boolean is not authority: `ForgeBuildStartRequest` has no
  approval field and denies unknown fields, so an
  `approved: true` payload is refused at parse time.
- Only the version-1 record of an approval artifact
  confers approval. Revising an approval artifact creates
  a later version that the gate ignores: the v1 evidence
  is preserved, and editing an approval record cannot
  transfer approval to a different specification version.
  Approving a new specification version always means a new
  `forge-spec-approve` call, which creates a new approval
  record (idempotent: re-approving an already-approved
  version returns the existing record with
  `alreadyApproved: true`).
- Archived projects refuse approval, like every other
  artifact change (`require_project_writable`).
- Approval is **permission to generate source from that
  specification version**. It is not permission to execute
  generated software: build/test/preview/deployment stay
  Unavailable under §32, and source is still only ever
  inspected as inert text and exported for review.

### UI surface

- Saving a specification draft no longer claims approval:
  the spec panel button is "Save specification draft" and
  its status text says approval is a separate step.
- The build panel, rendered for the viewed specification
  version, queries `forge-spec-approval-status` for that
  exact version and shows one of:
  - "Specification version N: approved — source generation
    is allowed." (with the approval artifact id), or
  - "Specification version N: not approved. Approve this
    version explicitly before generating source. Saving or
    revising a specification does not approve it."
- An "Approve this specification version" button calls
  `forge-spec-approve`; the Generate button is disabled
  while the version is unapproved. If the backend still
  refuses (e.g. the version changed underneath), its error
  is surfaced, not swallowed.

### Restart and backup/restore semantics

- Restart: approvals live in `artifact_versions`, so they
  survive a process restart exactly like specifications do.
  (The in-memory forge run registry is deliberately not
  approval state.)
- Backup/restore: approval rows travel with the backup
  like every other artifact row. Restore remaps the
  approval's `specArtifactId` to the restored specification
  (fail closed when the specification is not in the
  package, like every other pinned reference), and a
  fix-up pass over the prepared `artifact_versions` rows
  rewrites the approval's `specContentHash` to the restored
  specification version's recomputed hash and recomputes
  the approval row's own hash. So an approval transfers
  only to the restored equivalent of the **same**
  specification version — never to a different version,
  and never to a specification outside the package. A
  restored specification version that was never approved in
  the source workspace is still unapproved after restore
  and requires an explicit approval there.

## Gap 2 — export paths were not fully portable

Confirmed diagnostics:

- `validate_package_path("CON.txt")` succeeded.
- `validate_package_files` accepted both `assets.js` and
  `assets.js/app.js` in one package.

Windows extractors normalize names that this validator
allowed, so a package that validates on Linux could
collide or fail on Windows. The shared validator is the
single rule for both parsing generated output and
exporting stored packages (`parse_source_package` and
`forge-export` both call `validate_package_files`, and
export validates **before** any tar bytes are built), so
hardening it once hardens both paths.

### Hardened path rules (additive; all prior rules stay)

For every segment of every path:

- Windows device/reserved names are rejected, case-
  insensitively, as the segment stem (the part before the
  first `.`), with or without an extension, in any
  segment: `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`,
  `LPT1`–`LPT9`. So `CON.txt`, `con`, `Aux.js` and
  `foo/CON/bar.html` are all refused.
- Segments must not start or end with a space or a dot:
  `app.js.`, `app.js `, ` app.js` and `foo./bar.html`
  are refused. (Leading dots were already refused as
  hidden files; trailing dots/spaces are the Windows
  normalization hole.)
- File-versus-directory prefix conflicts are refused,
  case-insensitively and regardless of order: a file path
  that is also a directory prefix of another file
  (`assets.js` + `assets.js/app.js`) is refused, and
  directory prefixes that differ only by case (`CSS/…` +
  `css/…`) are refused, because they collapse to one
  directory on a case-insensitive filesystem.
- Unchanged: traversal, absolute/drive/UNC paths,
  backslashes, alternate-data-stream colons, empty/dot
  segments, hidden files, the ASCII segment alphabet,
  the `.html/.css/.js/.md/.txt` extension allowlist,
  exact and case-folded duplicate paths, per-file and
  total size caps, NUL bytes, and the root `index.html` +
  `README.md` requirements.

## Acceptance criteria (before coding)

1. Unapproved build: `forge-build-start` for a saved but
   never-approved specification version is refused with
   `InvalidRequest`, creates no run, and sends no provider
   traffic (synthetic provider wire capture unchanged).
2. Approved v1 builds; generic revise to v2 does not
   confer approval (build v2 refused); explicit
   `forge-spec-approve` for v2 then builds; approval for
   v1 still builds v1 after v2 exists.
3. A build request carrying an `approved` boolean is
   refused (unknown field), and saving/revising a
   specification never creates an approval record.
4. Stale approval: revising an approval artifact's body to
   point at another version does not approve that version
   (build refused); the originally approved version still
   builds (v1 evidence preserved).
5. Restart: approval for v1 persists across a fresh `Core`
   over the same storage root.
6. Backup/restore: an approved v1 transfers to the
   restored v1 (approval status approved, spec reference
   remapped, hash matching the restored spec); an
   unapproved restored version stays unapproved; an
   approval citing a specification outside the package
   fails the restore closed (unit-proven via the remap
   helper, since no public operation can mint such a
   package).
7. Path validator: `CON.txt`, `con`, `PRN.md`, `aux.js`,
   `COM1.html`, `LPT9.txt`, `foo/CON/bar.html`,
   `app.js.`, `app.js `, `foo./bar.html` are all refused;
   `assets.js` + `assets.js/app.js` (either order) and
   `CSS/x.css` + `css/y.css` are refused; previously
   valid paths still pass.
8. Export: a stored source package containing a
   non-portable path is refused by `forge-export` before
   any archive is produced (no tar bytes are returned).
9. No regression: forge suites, node contracts/UI,
   TypeScript, contracts check, docs, Clippy `-D
   warnings`, fmt, full core, providers/API and the Forge
   e2e workflow (updated to exercise the approval step)
   all pass; supplied tests are updated only to drive the
   new approval operation, never weakened.
10. Boundaries unchanged: no execution, no iframe preview,
    no dependency installation, no deployment, no new
    artifact types usable through generic registration,
    no pushes, no master merges, no remote CI.
