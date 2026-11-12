import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const script = path.join(root, 'scripts', 'qualification-fixtures.mjs');
const run = args => spawnSync(process.execPath, [script, ...args], { cwd: root, encoding: 'utf8' });

function tmpRoot() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-qualification-fixtures-'));
}

test('generation writes the full deterministic set with a hash manifest', () => {
  const base = tmpRoot();
  try {
    const target = path.join(base, 'fixtures');
    const result = run(['--target', target]);
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /PASS: 9 qualification fixtures/);
    const manifest = JSON.parse(fs.readFileSync(path.join(target, 'MANIFEST.json'), 'utf8'));
    assert.equal(manifest.schemaVersion, 1);
    assert.deepEqual(Object.keys(manifest.files), [...Object.keys(manifest.files)].sort());
    for (const [rel, info] of Object.entries(manifest.files)) {
      const content = fs.readFileSync(path.join(target, rel));
      assert.equal(content.length, info.bytes, rel);
      assert.equal(createHash('sha256').update(content).digest('hex'), info.sha256, rel);
    }
    assert.equal(manifest.files['oversized.txt'].bytes, 70000);
    assert.equal(manifest.files['oversized.txt'].expected, 'refused-oversized');
    assert.equal(manifest.files['empty.txt'].bytes, 0);
    assert.equal(manifest.files['unsupported.exe'].expected, 'refused-unsupported');
    assert.equal(manifest.files['unicode/café-üñïcode-notes.txt'].expected, 'imported');
    // Copies are byte-identical to the canonical trackable fixtures.
    const pdfManifest = JSON.parse(fs.readFileSync(path.join(root, 'tests/fixtures/pdf/MANIFEST.json'), 'utf8'));
    const docxManifest = JSON.parse(fs.readFileSync(path.join(root, 'tests/fixtures/docx/MANIFEST.json'), 'utf8'));
    assert.equal(manifest.files['normal-3page.pdf'].sha256, pdfManifest.fixtures['normal-3page.pdf'].sha256);
    assert.equal(manifest.files['normal.docx'].sha256, docxManifest.fixtures['normal.docx'].sha256);
    assert.equal(manifest.files['unicode-headings.docx'].sha256, docxManifest.fixtures['unicode-headings.docx'].sha256);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test('two generations are byte-identical and --verify passes', () => {
  const base = tmpRoot();
  try {
    const first = path.join(base, 'first');
    const second = path.join(base, 'second');
    assert.equal(run(['--target', first]).status, 0);
    assert.equal(run(['--target', second]).status, 0);
    assert.equal(
      fs.readFileSync(path.join(first, 'MANIFEST.json'), 'utf8'),
      fs.readFileSync(path.join(second, 'MANIFEST.json'), 'utf8'),
    );
    const result = run(['--verify', first]);
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /PASS: 9 qualification fixtures verified/);
    assert.equal(fs.readdirSync(base).filter(name => name.startsWith('.anansi-fixture-verify-')).length, 0);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test('--verify fails on tampered, missing, and manifest-less fixtures', () => {
  const base = tmpRoot();
  try {
    const target = path.join(base, 'fixtures');
    assert.equal(run(['--target', target]).status, 0);
    fs.appendFileSync(path.join(target, 'notes.txt'), 'tamper');
    assert.notEqual(run(['--verify', target]).status, 0);
    fs.rmSync(path.join(target, 'notes.txt'));
    assert.notEqual(run(['--verify', target]).status, 0);
    assert.notEqual(run(['--verify', path.join(base, 'absent')]).status, 0);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test('generation refuses non-empty targets, repository overlap, and bad usage', () => {
  const base = tmpRoot();
  try {
    const occupied = path.join(base, 'occupied');
    fs.mkdirSync(occupied);
    fs.writeFileSync(path.join(occupied, 'existing.txt'), 'owner data');
    assert.notEqual(run(['--target', occupied]).status, 0);
    assert.notEqual(run(['--target', path.join(root, 'fixtures-out')]).status, 0);
    assert.notEqual(run([]).status, 0);
    assert.notEqual(run(['--target', path.join(base, 'a'), '--verify', path.join(base, 'b')]).status, 0);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
