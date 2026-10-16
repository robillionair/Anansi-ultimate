import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = p => fs.readFileSync(path.join(root, p), 'utf8');

test('Git ignores representative secrets, private data and generated output', () => {
  const fixtures = [
    '.env', 'apps/web/.env.production', '.npmrc', 'crates/api/credentials.json',
    'private.pem', 'signing.pfx', 'secrets.json', 'local-data/memory.md',
    'workspaces/personal/note.md', 'backups/source.bundle', 'source.bundle',
    'data/memory.sqlite3-wal', 'data/memory.db-shm', 'data/memory.sqlite',
    'node_modules/example/index.js', 'apps/web/node_modules/example/index.js',
    'apps/web/dist/index.html', 'crates/core/target/debug/core.exe',
  ];
  const ignored = execFileSync('git', ['check-ignore', '--no-index', '--stdin'], {
    cwd: root, input: fixtures.join('\n') + '\n', encoding: 'utf8',
  }).trim().split(/\r?\n/);
  assert.deepEqual(ignored, fixtures);
});

test('Git can track source, configuration, safe fixtures and templates', () => {
  const fixtures = [
    'apps/desktop/src/main.rs', 'crates/core/src/lib.rs', 'packages/contracts/src/index.ts',
    'apps/web/.env.example', '.env.development.example', 'tests/fixtures/memory.md',
    'tests/fixtures/synthetic-memory.json', 'tests/fixtures/schema.sql',
    'pnpm-lock.yaml', 'project-state.json', 'scripts/validate-planning.mjs',
    '.github/workflows/ci.yml', 'docs/evidence/M0.md',
  ];
  const result = spawnSync('git', ['check-ignore', '--no-index', '--stdin'], {
    cwd: root, input: fixtures.join('\n') + '\n', encoding: 'utf8',
  });
  assert.equal(result.status, 1, result.stderr || result.stdout);
  assert.equal(result.stdout, '');
});

test('Git attributes distinguish text, Windows command files and binary assets', () => {
  const output = execFileSync('git', ['check-attr', 'text', 'eol', 'diff', '--',
    'apps/web/src/main.tsx', 'scripts/example.cmd', 'assets/icon.png'], { cwd: root, encoding: 'utf8' });
  assert.match(output, /main.tsx: eol: lf/);
  assert.match(output, /example.cmd: eol: crlf/);
  assert.match(output, /icon.png: text: unset/);
  assert.match(output, /icon.png: diff: unset/);
});

test('State and foundation configuration reference real, trackable files', () => {
  const state = JSON.parse(read('project-state.json'));
  assert.equal(state.schemaVersion, 1);
  assert.match(state.activeMilestone, /^M(?:[0-9]|1[0-5])$/);
  assert.ok(state.activeTask.startsWith(state.activeMilestone + '.'));
  assert.match(state.nextTask, /^M(?:[0-9]|1[0-5])\.\d+$/);
  assert.equal(typeof state.milestoneComplete, 'boolean');
  assert.equal(state.productImplementation, 'not started');
  for (const p of [state.plan, ...state.evidence]) assert.ok(fs.existsSync(path.join(root, p)), p);
  execFileSync('git', ['cat-file', '-e', `${state.lastVerifiedCommit}^{commit}`], { cwd: root });
  const pkg = JSON.parse(read('package.json'));
  assert.equal(pkg.packageManager, 'pnpm@11.25.0');
  assert.equal(Object.keys(pkg.dependencies ?? {}).length, 0);
  assert.equal(Object.keys(pkg.devDependencies ?? {}).length, 0);
  assert.match(read('pnpm-lock.yaml'), /lockfileVersion:/);
  assert.match(read('.editorconfig'), /charset = utf-8/);
  assert.match(read('.editorconfig'), /end_of_line = lf/);
});

test('Planning audit resolves the new nested evidence document', () => {
  const output = execFileSync(process.execPath, ['scripts/validate-planning.mjs', '--self-test'], {
    cwd: root, encoding: 'utf8',
  });
  assert.match(output, /PASS: 7 negative validation fixtures rejected/);
  assert.match(output, /PASS: \d+ Markdown files/);
});
