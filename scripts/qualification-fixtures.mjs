// Deterministic synthetic fixtures for the Windows installer-qualification
// manual checks (native document picker, concurrent imports). Generation is
// fully deterministic: fixed content, fixed names, sorted manifest, no
// timestamps or randomness. PDF/DOCX fixtures are copied byte-identical
// from tests/fixtures/; TXT/MD/oversized/unsupported/empty inputs are
// generated here because no trackable TXT/MD fixtures exist.
//
//   node scripts/qualification-fixtures.mjs --target <dir>
//   node scripts/qualification-fixtures.mjs --verify <dir>
//
// The target must be an explicit isolated directory (see
// candidate-locations.mjs: E:-only on Windows, outside the repository and
// owner-data roots, absent or empty). --verify regenerates into a temp
// directory and requires the manifests to match byte-for-byte, then
// re-hashes every file in the target against the manifest.
import fs from 'node:fs';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { assertIsolatedDir } from './windows-release/candidate-locations.mjs';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const hash = buffer => createHash('sha256').update(buffer).digest('hex');

const GENERATED = [
  {
    rel: 'notes.txt',
    content: Buffer.from(
      'Field notes - Harbor Ledger (synthetic qualification fixture)\n' +
      'The bramblewick shipment cleared customs on Tuesday.\n' +
      'Three crates of copper fittings, two of rope.\n' +
      'The quokka ledger keeper counts 41 crates, 12 pallets, 1 lighthouse keeper.\n',
      'utf8',
    ),
    expected: 'imported',
  },
  {
    rel: 'notes.md',
    content: Buffer.from(
      '# Harbor Ledger\n\nThe bramblewick shipment cleared customs on Tuesday.\n\n## Manifest\n\n- Three crates of copper fittings\n- Two coils of rope\n\nThe quokka ledger keeper counts 41 crates.\n',
      'utf8',
    ),
    expected: 'imported',
  },
  {
    rel: 'unicode/café-üñïcode-notes.txt',
    content: Buffer.from(
      'Unicode path fixture (synthetic qualification fixture)\nThe bramblewick shipment cleared customs on Tuesday.\n',
      'utf8',
    ),
    expected: 'imported',
  },
  {
    rel: 'oversized.txt',
    content: Buffer.from('0123456789'.repeat(7000), 'utf8'),
    expected: 'refused-oversized',
  },
  {
    rel: 'unsupported.exe',
    content: Buffer.from('MZ' + 'x'.repeat(62), 'utf8'),
    expected: 'refused-unsupported',
  },
  {
    rel: 'empty.txt',
    content: Buffer.alloc(0),
    expected: 'refused-empty',
  },
];

const COPIED = [
  { rel: 'normal-3page.pdf', source: 'tests/fixtures/pdf/normal-3page.pdf', expected: 'imported' },
  { rel: 'normal.docx', source: 'tests/fixtures/docx/normal.docx', expected: 'imported' },
  { rel: 'unicode-headings.docx', source: 'tests/fixtures/docx/unicode-headings.docx', expected: 'imported' },
];

export function buildManifest() {
  const files = {};
  for (const entry of [...GENERATED, ...COPIED].sort((a, b) => (a.rel < b.rel ? -1 : 1))) {
    const content = entry.content ?? fs.readFileSync(path.join(repoRoot, entry.source));
    files[entry.rel] = { sha256: hash(content), bytes: content.length, expected: entry.expected };
  }
  return { schemaVersion: 1, generator: 'scripts/qualification-fixtures.mjs', files };
}

export function generate(targetDir) {
  const target = assertIsolatedDir(targetDir, '--target', { cwd: repoRoot, env: process.env });
  fs.mkdirSync(target, { recursive: true });
  for (const entry of GENERATED) {
    const dest = path.join(target, entry.rel);
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.writeFileSync(dest, entry.content);
  }
  for (const entry of COPIED) {
    fs.copyFileSync(path.join(repoRoot, entry.source), path.join(target, entry.rel));
  }
  const manifest = buildManifest();
  fs.writeFileSync(path.join(target, 'MANIFEST.json'), JSON.stringify(manifest, null, 2) + '\n');
  return { target, manifest };
}

export function verify(targetDir) {
  const manifestPath = path.join(targetDir, 'MANIFEST.json');
  if (!fs.existsSync(manifestPath)) throw Error(`No fixture MANIFEST.json in ${targetDir}; generate it first with --target.`);
  const recorded = fs.readFileSync(manifestPath, 'utf8');
  // Scratch regeneration lives beside the target (same drive), so the
  // isolated-directory guard applies identically on Windows and Linux.
  const scratch = path.join(path.dirname(path.resolve(targetDir)), `.anansi-fixture-verify-${randomUUID()}`);
  try {
    const fresh = generate(path.join(scratch, 'fixtures'));
    const freshManifest = fs.readFileSync(path.join(fresh.target, 'MANIFEST.json'), 'utf8');
    if (freshManifest !== recorded) throw Error('Fixture manifest does not match a fresh deterministic generation.');
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true });
  }
  const manifest = JSON.parse(recorded);
  for (const [rel, info] of Object.entries(manifest.files)) {
    const file = path.join(targetDir, rel);
    if (!fs.existsSync(file)) throw Error(`Fixture missing: ${rel}`);
    const content = fs.readFileSync(file);
    if (content.length !== info.bytes || hash(content) !== info.sha256) {
      throw Error(`Fixture hash/size mismatch: ${rel}`);
    }
  }
  return manifest;
}

function main() {
  const args = process.argv.slice(2);
  const targetIdx = args.indexOf('--target');
  const verifyIdx = args.indexOf('--verify');
  if ((targetIdx === -1) === (verifyIdx === -1)) {
    console.error('Usage: node scripts/qualification-fixtures.mjs (--target <dir> | --verify <dir>)');
    process.exitCode = 1;
    return;
  }
  try {
    if (targetIdx !== -1) {
      const dir = args[targetIdx + 1];
      if (!dir) throw Error('--target requires a directory.');
      const { target, manifest } = generate(dir);
      console.log(`PASS: ${Object.keys(manifest.files).length} qualification fixtures written to ${target} (MANIFEST.json included).`);
    } else {
      const dir = args[verifyIdx + 1];
      if (!dir) throw Error('--verify requires a directory.');
      const manifest = verify(dir);
      console.log(`PASS: ${Object.keys(manifest.files).length} qualification fixtures verified deterministic and intact in ${dir}.`);
    }
  } catch (error) {
    console.error(`FAIL: ${error.message}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
