# M6 Slice 1 — design and acceptance criteria (approved scope)

Approved by the owner 2026-10-21: **universal projects and the
versioned artifact registry, without AI generation.**
Base: integrated checkpoint `2098c25`
(`integration/m5-into-m2-m3`). Branch:
`muse/m6-slice1-projects-registry`.

## Requirement mapping

- **MASTER_SPEC §9 (Universal Project Model)**: "All major
  ANANSI tools must work with one project." The existing
  Memory OS already implements that identity: a **project is a
  memory record with `kind = 'project'`**, and other records
  link to it via `projectId` (tasks do today). Slice 1 reuses
  that identity unchanged — no parallel project system, no new
  project IDs. The registry attaches to it.
- **§9 artifact fields** (verbatim): every artifact receives a
  unique ID, project ID, type, version, timestamp, source
  references, generator/model metadata, validation status,
  privacy classification. All nine are first-class columns and
  DTO fields below.
- **PHASE-006 / Milestone 6**: "universal product projects and
  versioned artifact registry" is the slice's scope.
  Opportunity Engine, idea generation and scoring are later
  slices/decisions — **not** in this slice. Project-optional
  artifacts are **not** created (§9: an artifact belongs to a
  project; `projectId` is required and validated).
- **AGENTS.md**: acceptance criteria before coding (this
  document), tests with every behavior, no completion claim
  without a verified workflow.

## Data model (workspace SQLite, schema v8 → v9)

New table `artifact_versions` (one row per immutable version):

| column | notes |
|---|---|
| `artifact_id` | TEXT, random 128-bit hex, stable across versions |
| `project_id` | TEXT, the project memory's ID (validated, below) |
| `version` | INTEGER, starts at 1, +1 per revision; PK (artifact_id, version) |
| `artifact_type` | TEXT, bounded set for manual artifacts (below) |
| `title` | TEXT (per-version, like memory revisions) |
| `body` | TEXT, ≤ 256 KiB (mirrors the memory body cap) |
| `content_hash` | SHA-256 hex of the canonical body bytes (`content::hash`) |
| `source_references` | JSON array of strings, ≤ 32 (mirrors memories) |
| `generator` | JSON object or NULL — `{kind:"manual"}` in this slice; model metadata lands here for later generator slices |
| `validation_status` | `unvalidated` on manual registration (no validator exists yet; never fabricated as "valid") |
| `privacy` | `standard` \| `private` \| `sealed` (see privacy rule) |
| `created_at` | ISO-8601 UTC |

Primary key (workspace_id, artifact_id, version).

Immutability is structural: the code exposes **no update or
delete** for artifact rows. "Revise" inserts a new row.

## Operations (`Core::artifacts_command`, group `artifacts`)

- `register` {workspaceId, projectId, type, title, body,
  sourceReferences?, privacy?} → validates the project (below),
  inserts version 1, returns the full record.
- `revise` {workspaceId, artifactId, expectedVersion, title,
  body, sourceReferences?} → **stale-write rule**: if
  `expectedVersion` ≠ current latest version, the write is
  refused with `RevisionConflict` and nothing is stored (the
  memory-revision rule, applied to artifacts). Inserts
  `expectedVersion + 1`.
- `list` {workspaceId, projectId} → latest-version metadata
  for every artifact of the project (no bodies).
- `versions` {workspaceId, artifactId} → metadata for all
  versions, ascending.
- `read` {workspaceId, artifactId, version?} → one version in
  full (default latest), **after integrity verification**:
  the stored hash is recomputed over the stored body; a
  mismatch is refused with `ContentIntegrity` and the body is
  never returned.

Project validation on register/revise/list: the project ID
must name an existing memory **in the same workspace** with
`kind = 'project'` (unknown → `NotFound`; wrong kind →
`InvalidRequest`). Registering or revising under an
**archived** project is refused (`InvalidRequest`) — restore
the project first (existing memory lifecycle). Project
archive/restore itself stays the existing memory lifecycle;
Slice 1 adds no second lifecycle.

Privacy rule (fail-closed, mirroring the semantic pipeline's
Sealed posture): artifacts may be registered as `standard` or
`private`. `sealed` registration is **refused with
`VaultUnavailable`** in this slice — sealed content belongs
to the vault, the registry has no vault integration yet, and
accepting content that could never be read back would be a
trap. Recorded as an explicit slice boundary, not an
oversight.

Manual artifact types (bounded set, from §9's contents list):
`research`, `source`, `idea`, `concept`, `experiment`,
`prototype`, `generated-file`, `simulation`, `agent-test`,
`decision`, `launch-material`, `feedback`, `note`.

## Surfaces

- Contracts: DTOs in `crates/core/src/artifacts.rs`, registered
  in `crates/contracts` (generated TS + validators, drift
  check). Client: `registerArtifact`, `reviseArtifact`,
  `listArtifacts`, `listArtifactVersions`, `readArtifact`.
- API: `/v1/artifacts/{operation}` (same auth/origin pipeline
  as every group). Desktop: `artifact_command` Tauri command
  mirroring `memory_command`, plus the native client's
  `artifacts/` routing.
- UI: the existing **Projects** screen gains an Artifacts
  section for the open project (register form, artifact list
  with latest version, version history, per-version read-only
  view marked immutable, revise form). No new navigation, no
  new project UI.

## Acceptance criteria (each maps to a named test)

1. **Workflow** (e2e, real browser + API binary): create
   project in the UI → register artifact v1 → revise to v2 →
   open v1 and v2 (both bodies intact, v1 read-only) →
   archive the project (existing lifecycle) → restore it →
   restart the API → reopen: project, artifact and both
   versions present.
2. **Immutability & integrity** (core): no operation mutates
   a stored version (v1 body/hash unchanged after v2 exists);
   a body tampered directly in SQLite makes `read` fail with
   `ContentIntegrity`.
3. **Stale-write conflicts** (core): revise with a non-current
   `expectedVersion` → `RevisionConflict`, nothing stored;
   revise with the current one succeeds.
4. **Workspace isolation** (core + API): an artifact/project
   ID from workspace A is `NotFound` from workspace B; lists
   never cross.
5. **Privacy enforcement** (core): `sealed` registration
   refused `VaultUnavailable`; `private` and `standard`
   round-trip; the privacy value is stored and returned on
   every version.
6. **§9 field completeness** (core): a read record carries
   all nine §9 fields (id, projectId, type, version,
   timestamp, sourceReferences, generator, validationStatus,
   privacy) — asserted field by field.
7. **No regressions**: M2–M5 suites at their merged-tree
   baselines (providers 120, core provider 18, API suites,
   UI/contracts 36+new, tsc, docs, contracts drift), plus the
   r10 browser checkpoint specs still pass.
8. **Recovery evidence**: the restart leg of (1) plus a core
   reopen test (new `Core` over the same root reads all
   versions).

## Out of scope (explicit)

Idea generation, scoring, Opportunity Engine, generator/model
metadata beyond the recorded `generator` field, sealed
artifacts (vault integration), artifact deletion, cross-project
moves, and any change to the memory/project lifecycle itself.
M5's pending checks (Windows native/manual, live providers,
G1 audit, remote CI) remain visible and do not block this
model-free slice.
