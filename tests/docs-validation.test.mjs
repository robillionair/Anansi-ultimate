import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { loadRepository, validate, prose, root } from '../scripts/validate-docs.mjs';

const base = loadRepository();
test('actual repository documentation and state validate', () => assert.deepEqual(validate(base), []));
const invalid = [
  ['missing master section', s => s.set('docs/MASTER_SPEC.md',s.get('docs/MASTER_SPEC.md').replace('# 90.','# 91.')), /Master/],
  ['duplicate coverage', s => s.set('docs/SPEC_COVERAGE.md',s.get('docs/SPEC_COVERAGE.md').replace('| 90 |','| 89 |')), /Coverage/],
  ['missing owner', s => s.set('docs/SPEC_COVERAGE.md',s.get('docs/SPEC_COVERAGE.md').replace('| M0 |','| nobody |')), /Coverage/],
  ['missing plan', s => s.delete('plans/PHASE-000-foundation.md'), /Roadmap/],
  ['missing link', s => s.set('README.md','[missing](missing.md)'), /missing.*link/],
  ['traversal', s => s.set('README.md','[escape](%2e%2e/outside.md)'), /out-of-root/],
  ['absolute path', s => s.set('README.md','[escape](C:/secret.md)'), /invalid local path/],
  ['bad encoding', s => s.set('README.md','[bad](%zz)'), /malformed/],
  ['bad anchor', s => s.set('README.md','[heading](docs/RECOVERY.md#missing)'), /missing anchor/],
  ['bad reference', s => s.set('README.md','[label][missing]'), /undefined reference/],
  ['invalid requirement status', s => s.set('docs/REQUIREMENTS_MATRIX.md',s.get('docs/REQUIREMENTS_MATRIX.md').replace('| planned |','| done-ish |')), /requirement status/],
  ['missing requirement', s => s.set('docs/REQUIREMENTS_MATRIX.md',s.get('docs/REQUIREMENTS_MATRIX.md').replace('AN-033','AN-032')), /Requirements/],
  ['invalid JSON', s => s.set('project-state.json','{'), /valid JSON/],
  ['missing state key', s => { const o=JSON.parse(s.get('project-state.json')); delete o.blockers; s.set('project-state.json',JSON.stringify(o)); }, /missing blockers/],
  ['invalid state type', s => { const o=JSON.parse(s.get('project-state.json')); o.evidence='none'; s.set('project-state.json',JSON.stringify(o)); }, /evidence/],
  ['missing evidence', s => { const o=JSON.parse(s.get('project-state.json')); o.evidence=['docs/evidence/missing.md']; s.set('project-state.json',JSON.stringify(o)); }, /missing.*link/],
  ['invalid checkpoint', s => { const o=JSON.parse(s.get('project-state.json')); o.lastVerifiedCommit='nope'; s.set('project-state.json',JSON.stringify(o)); }, /checkpoint/],
  ['invalid milestone', s => { const o=JSON.parse(s.get('project-state.json')); o.activeMilestone='M99'; s.set('project-state.json',JSON.stringify(o)); }, /identity/],
];
for (const [name, mutate, expected] of invalid) test(`rejects ${name}`, () => {
  const snapshot = new Map(base);
  mutate(snapshot);
  assert.match(validate(snapshot).join('\n'), expected);
});

test('fences and inline examples are ignored; real anchors, duplicates, assets and references resolve', () => {
  const snapshot = new Map(base);
  snapshot.set('tests/fixtures/guide.md','# Topic\n# Topic\n# Café & tea\n```md\n[not a link](missing.md)\n~~~~\n```\n`[example](missing.md)`\n[dup](#topic-1)\n[unicode](#café--tea)\n[policy](../../.editorconfig)\n[ref][source]\n[source]: ../../README.md\n');
  assert.deepEqual(validate(snapshot),[]);
});
test('longer fenced blocks require a matching closing fence', () => {
  assert.equal(prose('~~~~js\n[bad](missing)\n~~~\n~~~~\n# Visible'),'\n\n\n\n# Visible');
});
for (const operation of ['build','backup']) test(`${operation} reports unavailable with nonzero status`, () => {
  const result=spawnSync(process.execPath,['scripts/unavailable.mjs',operation],{cwd:root,encoding:'utf8'});
  assert.equal(result.status,1);
  assert.match(result.stderr,/not (available|implemented)/);
});
