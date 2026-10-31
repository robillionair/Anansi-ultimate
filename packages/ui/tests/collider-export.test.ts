// M8 §27 export bundle, node side: deterministic
// ZIP structure, CRC-32, manifest consistency, and
// the fail-closed hash check.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { buildZip, buildExperimentExport, crc32 } from '../src/colliderExport';
import { fixtureDoc } from './collider-artifacts-fixture';

function sha256(data: Uint8Array): Promise<string> {
  return Promise.resolve(createHash('sha256').update(data).digest('hex'));
}

interface ZipEntry {
  name: string;
  data: Uint8Array;
  crc: number;
}

// Minimal STORE-only ZIP reader for the test: local
// headers are sequential, so entries parse in order.
function readZip(bytes: Uint8Array): ZipEntry[] {
  const entries: ZipEntry[] = [];
  let offset = 0;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  while (view.getUint32(offset, true) === 0x04034b50) {
    const crc = view.getUint32(offset + 14, true);
    const size = view.getUint32(offset + 18, true);
    const nameLen = view.getUint16(offset + 26, true);
    const extraLen = view.getUint16(offset + 28, true);
    const nameStart = offset + 30 + extraLen;
    const name = new TextDecoder().decode(bytes.subarray(nameStart, nameStart + nameLen));
    const dataStart = nameStart + nameLen;
    entries.push({
      name,
      data: bytes.subarray(dataStart, dataStart + size),
      crc,
    });
    offset = dataStart + size;
  }
  assert.equal(view.getUint32(offset, true), 0x02014b50, 'central directory follows entries');
  return entries;
}

test('crc32 matches the standard check value', () => {
  assert.equal(crc32(new TextEncoder().encode('123456789')), 0xcbf43926);
});

test('zip output is deterministic and structurally sound', () => {
  const files = [
    { name: 'CONCEPT.md', data: new TextEncoder().encode('# Concept\n') },
    { name: 'MANIFEST.json', data: new TextEncoder().encode('{}\n') },
  ];
  const first = buildZip(files);
  const second = buildZip(files);
  assert.deepEqual(first, second);
  const entries = readZip(first);
  assert.equal(entries.length, 2);
  assert.equal(entries[0].name, 'CONCEPT.md');
  assert.equal(new TextDecoder().decode(entries[0].data), '# Concept\n');
  assert.equal(entries[0].crc, crc32(entries[0].data));
});

test('export manifest hashes match the zip entries and the stored hashes', async () => {
  const storedDoc = {
    ...fixtureDoc,
    artifacts: {
      concept: {
        sha256: 'e2cadb381eda099cd0c34ffd318847e5291fdb2b5d3e685502dd88a31299398c',
        bytes: 882,
      },
      architecture: {
        sha256: 'fbd19e41179b140e584a128ff3e2bce470a9735cce141ced119af2573069f007',
        bytes: 628,
      },
      feasibility: {
        sha256: '19aaa9410f39d5fc981307091a7bf4743169a94d1a6c3bd68c550f7bcd156481',
        bytes: 704,
      },
      roadmap: {
        sha256: '9f53524f011d6621d87e77b9c719db5b07bcdd809151c473e31756c4d9b50c22',
        bytes: 589,
      },
      risks: {
        sha256: 'b9e4a8c213d0d7921651a5d8bdc410f6755bba5055fd73e09fef3c13060d3275',
        bytes: 657,
      },
      testPlan: {
        sha256: '9a9726a6d2e41cfcd364ee3aee42a2531a295aa500b8acbd38091d57973516d5',
        bytes: 869,
      },
    },
  };
  const body = JSON.stringify(storedDoc);
  const built = await buildExperimentExport(
    {
      doc: storedDoc as never,
      body,
      artifactId: 'abcdef0123456789abcdef0123456789',
      version: 1,
      timestamp: '2026-10-31T00:00:00Z',
    },
    sha256,
  );
  const entries = readZip(built.bytes);
  assert.deepEqual(
    entries.map(e => e.name),
    [
      'CONCEPT.md',
      'ARCHITECTURE.md',
      'FEASIBILITY.md',
      'ROADMAP.md',
      'RISKS.md',
      'TEST_PLAN.md',
      'experiment.json',
      'MANIFEST.json',
    ],
  );
  const byName = new Map(entries.map(e => [e.name, e]));
  assert.equal(new TextDecoder().decode(byName.get('experiment.json')!.data), body);
  const manifest = JSON.parse(new TextDecoder().decode(byName.get('MANIFEST.json')!.data));
  assert.equal(manifest.experimentArtifactId, 'abcdef0123456789abcdef0123456789');
  assert.equal(manifest.experimentVersion, 1);
  assert.equal(manifest.provider, 'test-provider');
  assert.equal(manifest.model, 'test-model');
  for (const file of manifest.files) {
    const entry = byName.get(file.name);
    assert.ok(entry, file.name);
    assert.equal(file.bytes, entry.data.length, file.name);
    assert.equal(file.sha256, await sha256(entry.data), file.name);
    assert.equal(entry.crc, crc32(entry.data), file.name);
  }
  // Byte-identical rebuild: determinism end to end.
  const rebuilt = await buildExperimentExport(
    { doc: storedDoc as never, body, artifactId: 'abcdef0123456789abcdef0123456789', version: 1, timestamp: '2026-10-31T00:00:00Z' },
    sha256,
  );
  assert.deepEqual(built.bytes, rebuilt.bytes);
});

test('export refuses a derivation that disagrees with stored hashes', async () => {
  const tampered = {
    ...fixtureDoc,
    artifacts: {
      concept: { sha256: '0'.repeat(64), bytes: 882 },
      architecture: {
        sha256: 'fbd19e41179b140e584a128ff3e2bce470a9735cce141ced119af2573069f007',
        bytes: 628,
      },
      feasibility: {
        sha256: '19aaa9410f39d5fc981307091a7bf4743169a94d1a6c3bd68c550f7bcd156481',
        bytes: 704,
      },
      roadmap: {
        sha256: '9f53524f011d6621d87e77b9c719db5b07bcdd809151c473e31756c4d9b50c22',
        bytes: 589,
      },
      risks: {
        sha256: 'b9e4a8c213d0d7921651a5d8bdc410f6755bba5055fd73e09fef3c13060d3275',
        bytes: 657,
      },
      testPlan: {
        sha256: '9a9726a6d2e41cfcd364ee3aee42a2531a295aa500b8acbd38091d57973516d5',
        bytes: 869,
      },
    },
  };
  await assert.rejects(
    buildExperimentExport(
      {
        doc: tampered as never,
        body: JSON.stringify(tampered),
        artifactId: 'abcdef0123456789abcdef0123456789',
        version: 1,
        timestamp: '2026-10-31T00:00:00Z',
      },
      sha256,
    ),
    /does not match the saved experiment/,
  );
});
