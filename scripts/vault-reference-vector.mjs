// Independent Node/OpenSSL KDF fixture, never imports or invokes RustCrypto.
// All bytes are public synthetic data. Not a primitive audit.
import { argon2Sync } from 'node:crypto';
import fs from 'node:fs';
import assert from 'node:assert/strict';
const file = new URL('../crates/core/tests/fixtures/vault-argon2-reference.json', import.meta.url);
if (process.argv.slice(2).some(a => !['--write', '--check'].includes(a))) throw Error('Use --write or --check');
const message = Buffer.alloc(32, 1), nonce = Buffer.alloc(16, 2);
const derive = (memory, parallelism, extra = {}) => argon2Sync('argon2id', {
  message, nonce, parallelism, tagLength: 32, memory, passes: 3, ...extra,
}).toString('hex');
assert.equal(derive(32, 4, { secret: Buffer.alloc(8, 3), associatedData: Buffer.alloc(12, 4) }),
  '0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659', 'Node reference must match RFC9106 §5.3');
const fixture = {
  provenance: { generator: 'Node built-in crypto.argon2Sync; independent of RustCrypto',
    node: process.version, openssl: process.versions.openssl,
    documentation: 'https://nodejs.org/api/crypto.html#cryptoargon2syncalgorithm-parameters',
    rfcCheck: 'RFC9106 §5.3 p=4 passed before generating production-profile vectors',
    scope: 'Synthetic deterministic interoperability fixture, not third-party security review' },
  algorithm: 'argon2id', version: 19, memoryKib: 65536, iterations: 3, outputBytes: 32,
  passwordHex: message.toString('hex'), saltHex: nonce.toString('hex'),
  p4: derive(65536, 4), p1: derive(65536, 1),
};
assert.notEqual(fixture.p4, fixture.p1);
if (process.argv.includes('--write')) {
  fs.mkdirSync(new URL('../crates/core/tests/fixtures/', import.meta.url), { recursive: true });
  fs.writeFileSync(file, JSON.stringify(fixture, null, 2) + '\n');
} else {
  const saved = JSON.parse(fs.readFileSync(file, 'utf8'));
  for (const field of ['algorithm', 'version', 'memoryKib', 'iterations', 'outputBytes', 'passwordHex', 'saltHex', 'p4', 'p1']) assert.equal(saved[field], fixture[field], field);
}
console.log('PASS: independent RFC check and 64-MiB p=4/p=1 reference vectors');
