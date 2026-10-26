import fs from 'node:fs';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
const archive = 'C:/Users/robin/Downloads/13ce8e70d6a70c4430d32fc400a1e9f9-v1.tar';
const evidence = 'E:/Anansi/tmp/m7-ui-acceptance-S14T39';
const bytes = fs.readFileSync(archive);
const sha256 = data => crypto.createHash('sha256').update(data).digest('hex');
assert.equal(sha256(bytes), '97c063174ff195bf822b99631833ee15de6523df19c8b2bd546c44976de7db5d');
const entries = new Map();
for (let offset = 0; offset + 512 <= bytes.length;) {
  const header = bytes.subarray(offset, offset + 512);
  if (header.every(byte => byte === 0)) break;
  const name = header.subarray(0, 100).toString('utf8').replace(/\0.*$/s, '');
  const octal = (a, b) => parseInt(header.subarray(a, b).toString('ascii').replace(/\0.*$/s, '').trim(), 8);
  const size = octal(124, 136);
  assert.equal(String.fromCharCode(header[156]), '0');
  assert.equal(octal(100, 108), 0o600);
  assert(!entries.has(name));
  assert(!name.includes('..') && !name.includes('\\') && !name.startsWith('/'));
  entries.set(name, bytes.subarray(offset + 512, offset + 512 + size));
  offset += 512 + Math.ceil(size / 512) * 512;
}
assert.deepEqual([...entries.keys()], ['ANANSI-PACKAGE.json', 'index.html', 'app.js', 'README.md']);
const manifest = JSON.parse(entries.get('ANANSI-PACKAGE.json').toString('utf8'));
assert.equal(manifest.format, 'anansi-source-package-export');
assert.equal(manifest.artifactId, '13ce8e70d6a70c4430d32fc400a1e9f9');
assert.equal(manifest.specVersion, 1);
assert.equal(manifest.specArtifactId, 'd81aa84fd2fd7a7d1d9430dc419c89f5');
for (const file of manifest.files) {
  assert.equal(entries.get(file.path).length, file.bytes);
  assert.equal(sha256(entries.get(file.path)), file.sha256);
}
assert(entries.get('app.js').toString('utf8').includes('FORGE_INERT_SOURCE_ONLY'));
fs.copyFileSync(archive, `${evidence}/native-export.tar`);
const result = { passed: true, sha256: sha256(bytes), bytes: bytes.length,
  entries: [...entries.keys()], perFileHashesVerified: true, executed: false };
fs.writeFileSync(`${evidence}/native-export-verification.json`, JSON.stringify(result, null, 2));
console.log(JSON.stringify(result));
