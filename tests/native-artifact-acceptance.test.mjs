import test from 'node:test';
import assert from 'node:assert/strict';
import {
  assertAnansiId,
  assertArtifactDetails,
  assertReadiness,
} from '../scripts/windows-release/artifact-acceptance.mjs';

const ARTIFACT_ID = '413c75aa59a09ef9637d3df347049b78';
const PROJECT_ID = 'f6c4f06e9c2c65dd4c2064bf910e9c7d';
const EXPECTED = {
  projectId: PROJECT_ID,
  type: 'idea',
  version: 1,
  title: 'Synthesized Idea Artifact',
  body: 'Verified idea body content\nTags: synthetic, test',
};

// Register/read response shape, exactly as ArtifactDetails serializes:
// top-level body, record fields camelCase, identifier named `artifactId`.
function validDetails() {
  return {
    contractVersion: 1,
    artifact: {
      artifactId: ARTIFACT_ID,
      projectId: PROJECT_ID,
      type: 'idea',
      version: 1,
      title: 'Synthesized Idea Artifact',
      timestamp: '2026-11-13T00:00:00.000Z',
      sourceReferences: [],
      generator: { kind: 'manual' },
      validationStatus: 'unvalidated',
      privacy: 'standard',
      contentHash: 'deadbeef',
    },
    body: 'Verified idea body content\nTags: synthetic, test',
  };
}

test('NQ-01 regression: a response carrying only the old `id` field fails', () => {
  const details = validDetails();
  details.artifact.id = ARTIFACT_ID;
  delete details.artifact.artifactId;
  // This is the exact shape the old harness code accepted silently:
  // `artRes.artifact.id` read a value that was never there, printed
  // `undefined`, and the check still passed.
  assert.equal(details.artifact.id, ARTIFACT_ID);
  assert.throws(
    () => assertArtifactDetails(details, EXPECTED, 'artifact'),
    /artifactId must be a 32-character lowercase hex ANANSI id, got undefined/,
  );
});

test('missing or malformed details fail', () => {
  for (const bad of [undefined, null, 'x', 42, []]) {
    assert.throws(() => assertArtifactDetails(bad, EXPECTED, 'artifact'), /expected an artifact details object/);
  }
  assert.throws(
    () => assertArtifactDetails({ artifact: undefined, body: EXPECTED.body }, EXPECTED, 'artifact'),
    /expected an artifact record object/,
  );
  assert.throws(
    () => assertArtifactDetails({ artifact: null, body: EXPECTED.body }, EXPECTED, 'artifact'),
    /expected an artifact record object/,
  );
});

test('missing or invalid artifactId fails', () => {
  const badIds = [
    undefined,
    null,
    '',
    '413c75aa59a09ef9637d3df347049b7', // 31 chars
    '413c75aa59a09ef9637d3df347049b789', // 33 chars
    '413C75AA59A09EF9637D3DF347049B78', // uppercase
    'g13c75aa59a09ef9637d3df347049b78', // non-hex
    41375, // non-string
    { artifactId: ARTIFACT_ID }, // non-string
  ];
  for (const badId of badIds) {
    const details = validDetails();
    details.artifact.artifactId = badId;
    assert.throws(
      () => assertArtifactDetails(details, EXPECTED, 'artifact'),
      /artifactId must be a 32-character lowercase hex ANANSI id/,
    );
  }
});

test('each field mismatch fails', () => {
  const cases = [
    ['projectId', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'],
    ['type', 'concept'],
    ['version', 2],
    ['version', '1'],
    ['title', 'Different title'],
  ];
  for (const [field, value] of cases) {
    const details = validDetails();
    details.artifact[field] = value;
    assert.throws(
      () => assertArtifactDetails(details, EXPECTED, 'artifact'),
      new RegExp(`${field} mismatch`),
    );
  }
  const bodyMismatch = validDetails();
  bodyMismatch.body = 'Different body';
  assert.throws(
    () => assertArtifactDetails(bodyMismatch, EXPECTED, 'artifact'),
    /body mismatch/,
  );
  const missingBody = validDetails();
  delete missingBody.body;
  assert.throws(
    () => assertArtifactDetails(missingBody, EXPECTED, 'artifact'),
    /body mismatch/,
  );
});

test('exact match passes and returns the artifact id', () => {
  assert.equal(assertArtifactDetails(validDetails(), EXPECTED, 'artifact'), ARTIFACT_ID);
  assert.equal(
    assertArtifactDetails(validDetails(), { artifactId: ARTIFACT_ID, ...EXPECTED }, 'artifact'),
    ARTIFACT_ID,
  );
});

test('expected artifact id mismatch fails', () => {
  assert.throws(
    () =>
      assertArtifactDetails(
        validDetails(),
        { artifactId: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', ...EXPECTED },
        'artifact',
      ),
    /artifactId mismatch/,
  );
});

test('assertAnansiId accepts only the 32 lowercase hex format', () => {
  assert.equal(assertAnansiId(ARTIFACT_ID, 'id'), ARTIFACT_ID);
  for (const bad of [undefined, '', 'abc', 'A'.repeat(32), 'z'.repeat(32), 0]) {
    assert.throws(() => assertAnansiId(bad, 'id'), /must be a 32-character lowercase hex ANANSI id/);
  }
});

function validReadiness() {
  return {
    contractVersion: 1,
    applicationName: 'ANANSI Ultimate',
    applicationVersion: '0.1.0',
    apiVersion: null,
    coreVersion: '0.1.0',
    operatingSystem: 'windows',
    uptimeMs: 1234,
  };
}

test('readiness: valid response passes', () => {
  assert.equal(assertReadiness(validReadiness(), 'readiness').applicationName, 'ANANSI Ultimate');
});

test('readiness: each invalid field fails', () => {
  assert.throws(() => assertReadiness(undefined, 'readiness'), /expected a readiness object/);
  const cases = [
    ['contractVersion', 2],
    ['applicationName', 'ANANSI'],
    ['operatingSystem', 'linux'],
    ['applicationVersion', ''],
    ['coreVersion', ''],
    ['uptimeMs', -1],
    ['uptimeMs', '1234'],
    ['uptimeMs', Number.NaN],
  ];
  for (const [field, value] of cases) {
    const readiness = validReadiness();
    readiness[field] = value;
    assert.throws(() => assertReadiness(readiness, 'readiness'), new RegExp(field));
  }
  const mismatched = validReadiness();
  mismatched.coreVersion = '0.2.0';
  assert.throws(() => assertReadiness(mismatched, 'readiness'), /coreVersion mismatch/);
});
