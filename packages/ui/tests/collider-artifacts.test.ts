// M8 §27 artifacts, node side: the TypeScript
// derivation must be byte-identical to the Rust
// derivation for the shared fixture, and the viewer
// files must contain zero innerHTML.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { deriveAllArtifacts, utf8Bytes } from '../src/colliderArtifacts';
import { fixtureDoc } from './collider-artifacts-fixture';

const PINNED: Array<{ file: string; sha256: string; bytes: number }> = [
  {
    file: 'CONCEPT.md',
    sha256: 'e2cadb381eda099cd0c34ffd318847e5291fdb2b5d3e685502dd88a31299398c',
    bytes: 882,
  },
  {
    file: 'ARCHITECTURE.md',
    sha256: 'fbd19e41179b140e584a128ff3e2bce470a9735cce141ced119af2573069f007',
    bytes: 628,
  },
  {
    file: 'FEASIBILITY.md',
    sha256: '19aaa9410f39d5fc981307091a7bf4743169a94d1a6c3bd68c550f7bcd156481',
    bytes: 704,
  },
  {
    file: 'ROADMAP.md',
    sha256: '9f53524f011d6621d87e77b9c719db5b07bcdd809151c473e31756c4d9b50c22',
    bytes: 589,
  },
  {
    file: 'RISKS.md',
    sha256: 'b9e4a8c213d0d7921651a5d8bdc410f6755bba5055fd73e09fef3c13060d3275',
    bytes: 657,
  },
  {
    file: 'TEST_PLAN.md',
    sha256: '9a9726a6d2e41cfcd364ee3aee42a2531a295aa500b8acbd38091d57973516d5',
    bytes: 869,
  },
];

function sha256(text: string): string {
  return createHash('sha256').update(text, 'utf8').digest('hex');
}

test('derivation matches the Rust pinned hashes and byte counts', () => {
  const derived = deriveAllArtifacts(fixtureDoc);
  assert.equal(derived.length, 6);
  for (const pin of PINNED) {
    const found = derived.find(d => d.file === pin.file);
    assert.ok(found, pin.file);
    assert.equal(sha256(found.text), pin.sha256, pin.file);
    assert.equal(utf8Bytes(found.text), pin.bytes, pin.file);
    assert.ok(found.text.endsWith('\n'), pin.file);
    assert.ok(!found.text.endsWith('\n\n'), pin.file);
  }
});

test('derivation is deterministic and states nothing unstated', () => {
  const first = deriveAllArtifacts(fixtureDoc);
  const second = deriveAllArtifacts(fixtureDoc);
  assert.deepEqual(first, second);
  for (const { file, text } of first) {
    assert.ok(text.includes('Not stated in the source record.'), file);
    assert.ok(!text.includes('Severity: high'), file);
    assert.ok(!text.includes('Likelihood: high'), file);
  }
});

test('artifact viewer and export files contain zero innerHTML', () => {
  for (const file of [
    'packages/ui/src/colliderArtifacts.ts',
    'packages/ui/src/colliderMarkdown.tsx',
    'packages/ui/src/ColliderArtifactViewer.tsx',
    'packages/ui/src/ColliderBuildProduct.tsx',
    'packages/ui/src/colliderExport.ts',
    'packages/ui/src/Transfer.tsx',
  ]) {
    const source = readFileSync(file, 'utf8');
    assert.ok(!source.includes('innerHTML'), file);
    assert.ok(!source.includes('dangerouslySetInnerHTML'), file);
  }
});
