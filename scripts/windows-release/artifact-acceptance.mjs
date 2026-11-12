// Assertion helpers for the native candidate acceptance harness.
//
// These are pure functions so the harness's checks can be regression-tested
// on Linux, where the harness itself refuses to run (Windows-only CDP).
// Field names mirror the Rust contract exactly:
// - ArtifactDetails { contractVersion, artifact: ArtifactRecord, body }
//   (crates/core/src/artifacts.rs). The record's identifier is `artifactId`;
//   there is no `id` field and no `workspaceId` field on the record.
//   NQ-01: the harness read `artifact.id`, printed `undefined`, and still
//   reported success. These helpers fail closed on any missing or
//   mismatched field instead.
// - Readiness { contractVersion, applicationName, applicationVersion,
//   apiVersion, coreVersion, operatingSystem, uptimeMs }
//   (crates/core/src/readiness.rs).

// ANANSI id format (crates/core/src/workspace.rs `validate_id`): exactly 32
// lowercase hex characters.
const ANANSI_ID_PATTERN = /^[0-9a-f]{32}$/;

function describe(value) {
  if (value === undefined) return 'undefined';
  if (value === null) return 'null';
  if (typeof value === 'string') return `string of length ${value.length}`;
  if (Array.isArray(value)) return 'array';
  return typeof value;
}

export function assertAnansiId(value, label) {
  if (typeof value !== 'string' || !ANANSI_ID_PATTERN.test(value)) {
    throw Error(
      `${label} must be a 32-character lowercase hex ANANSI id, got ${describe(value)}.`,
    );
  }
  return value;
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw Error(`${label} mismatch: expected ${JSON.stringify(expected)}, got ${describe(actual)}.`);
  }
}

// Assert an ArtifactDetails response matches the expected registration and
// return the real artifact identifier. `expected` is
// { artifactId?, projectId, type, version, title, body }. When
// `expected.artifactId` is given, the response must carry that exact id;
// otherwise the id must still be a valid ANANSI id. The workspace is not
// asserted here: ArtifactRecord carries no workspace field. Workspace
// scoping is asserted by the harness through the workspace-scoped read
// request plus a cross-workspace negative probe.
export function assertArtifactDetails(details, expected, label) {
  if (details === null || typeof details !== 'object' || Array.isArray(details)) {
    throw Error(`${label}: expected an artifact details object, got ${describe(details)}.`);
  }
  const artifact = details.artifact;
  if (artifact === null || typeof artifact !== 'object' || Array.isArray(artifact)) {
    throw Error(`${label}: expected an artifact record object, got ${describe(artifact)}.`);
  }
  const artifactId = assertAnansiId(artifact.artifactId, `${label}: artifactId`);
  if (expected.artifactId !== undefined) {
    assertEqual(artifactId, expected.artifactId, `${label}: artifactId`);
  }
  assertEqual(artifact.projectId, expected.projectId, `${label}: projectId`);
  assertEqual(artifact.type, expected.type, `${label}: type`);
  assertEqual(artifact.version, expected.version, `${label}: version`);
  assertEqual(artifact.title, expected.title, `${label}: title`);
  if (typeof details.body !== 'string' || details.body !== expected.body) {
    throw Error(
      `${label}: body mismatch: expected a string of length ${expected.body.length}, ` +
        `got ${describe(details.body)}.`,
    );
  }
  return artifactId;
}

export function assertReadiness(readiness, label) {
  if (readiness === null || typeof readiness !== 'object' || Array.isArray(readiness)) {
    throw Error(`${label}: expected a readiness object, got ${describe(readiness)}.`);
  }
  assertEqual(readiness.contractVersion, 1, `${label}: contractVersion`);
  assertEqual(readiness.applicationName, 'ANANSI Ultimate', `${label}: applicationName`);
  assertEqual(readiness.operatingSystem, 'windows', `${label}: operatingSystem`);
  if (typeof readiness.applicationVersion !== 'string' || readiness.applicationVersion.length === 0) {
    throw Error(`${label}: applicationVersion must be a non-empty string, got ${describe(readiness.applicationVersion)}.`);
  }
  if (typeof readiness.coreVersion !== 'string' || readiness.coreVersion.length === 0) {
    throw Error(`${label}: coreVersion must be a non-empty string, got ${describe(readiness.coreVersion)}.`);
  }
  assertEqual(readiness.coreVersion, readiness.applicationVersion, `${label}: coreVersion`);
  if (typeof readiness.uptimeMs !== 'number' || !Number.isFinite(readiness.uptimeMs) || readiness.uptimeMs < 0) {
    throw Error(`${label}: uptimeMs must be a finite number >= 0, got ${describe(readiness.uptimeMs)}.`);
  }
  // apiVersion is intentionally unasserted: the desktop command returns
  // None (null) and the field carries no acceptance meaning here.
  return readiness;
}
