// Reads an exported archive as bytes only. Never extracts or executes source.
import fs from 'node:fs';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
const archive = process.argv[2];
assert(archive, 'Supply the native UI export archive path');
assert(fs.statSync(archive).size <= 2 * 1024 * 1024, 'Unexpected archive size');
const bytes = fs.readFileSync(archive);
const hash = b => crypto.createHash('sha256').update(b).digest('hex');
const entries = new Map();
const folded = new Set();
for (let offset = 0; offset + 512 <= bytes.length;) {
  const h = bytes.subarray(offset, offset + 512);
  if (h.every(b => b === 0)) break;
  const text = (a, b) => h.subarray(a, b).toString('utf8').replace(/\0.*$/s, '');
  const octal = (a, b) => parseInt(text(a, b).trim(), 8);
  assert.equal(text(257, 262), 'ustar');
  assert.equal(h[156], '0'.charCodeAt(0));
  assert.equal(octal(100, 108), 0o600);
  const checksum = h.reduce((sum, value, i) => sum + (i >= 148 && i < 156 ? 32 : value), 0);
  assert.equal(octal(148, 156), checksum);
  const prefix = text(345, 500);
  const name = (prefix ? prefix + '/' : '') + text(0, 100);
  assert(name && !name.startsWith('/') && !name.includes('\\') && !name.split('/').includes('..'));
  assert(!folded.has(name.toLowerCase()), 'Duplicate archive member');
  const size = octal(124, 136);
  assert(Number.isFinite(size) && size >= 0 && offset + 512 + size <= bytes.length);
  entries.set(name, bytes.subarray(offset + 512, offset + 512 + size));
  folded.add(name.toLowerCase());
  offset += 512 + Math.ceil(size / 512) * 512;
}
assert(entries.has('ANANSI-DELIVERABLE.json'));
const manifest = JSON.parse(entries.get('ANANSI-DELIVERABLE.json').toString('utf8'));
assert.equal(manifest.format, 'anansi-deliverable-export');
assert.equal(manifest.formatVersion, 1);
assert.deepEqual(manifest.status, { executed: false, built: false, tested: false, published: false, productionReady: false });
assert.equal(manifest.packageVersion, 2);
const expected = new Set(['ANANSI-DELIVERABLE.json']);
for (const file of manifest.files) {
  assert(!expected.has(file.path)); expected.add(file.path);
  const data = entries.get(file.path); assert(data, 'Missing source file');
  assert.equal(data.length, file.bytes); assert.equal(hash(data), file.sha256);
}
for (const doc of manifest.documents) {
  const name = `deliverable-docs/${doc.name}`;
  assert(!expected.has(name)); expected.add(name);
  const data = entries.get(name); assert(data, 'Missing documentation file');
  assert.equal(data.length, doc.bytes); assert.equal(hash(data), doc.sha256);
}
assert.equal(entries.size, expected.size);
assert(entries.get('app.js').toString('utf8').includes('S23_INERT_V2'));
assert(entries.has('revision.txt'));
console.log(JSON.stringify({ archive, sha256: hash(bytes), deliverableArtifactId: manifest.deliverableArtifactId,
  deliverableVersion: manifest.deliverableVersion, packageArtifactId: manifest.packageArtifactId,
  packageVersion: manifest.packageVersion, specArtifactId: manifest.specArtifactId,
  paths: [...entries.keys()], generatedCodeExecuted: false }, null, 2));
