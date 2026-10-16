// Compatibility entrypoint retained for existing recovery instructions.
import assert from 'node:assert/strict';
import { loadRepository, validate, main } from './validate-docs.mjs';
if (process.argv.includes('--self-test')) {
  const files = loadRepository();
  const mutations = [
    s => s.delete('plans/PHASE-000-foundation.md'),
    s => s.set('docs/SPEC_COVERAGE.md', s.get('docs/SPEC_COVERAGE.md').replace('| 90 |', '| 89 |')),
    s => s.set('docs/SPEC_COVERAGE.md', s.get('docs/SPEC_COVERAGE.md').replace('| M0 |', '| none |')),
    s => s.set('docs/MASTER_SPEC.md', s.get('docs/MASTER_SPEC.md').replace('# 90.', '# 91.')),
    s => s.set('docs/ROADMAP.md', s.get('docs/ROADMAP.md').replace('| M15 —', '| M16 —')),
    s => s.set('README.md', s.get('README.md') + '\n[bad](../outside.md)'),
    s => s.set('docs/REQUIREMENTS_MATRIX.md', s.get('docs/REQUIREMENTS_MATRIX.md').replace('AN-033', 'AN-032')),
  ];
  for (const mutate of mutations) {
    const snapshot = new Map(files);
    mutate(snapshot);
    assert.ok(validate(snapshot).length > 0, 'Invalid fixture accepted');
  }
  console.log('PASS: 7 negative validation fixtures rejected; repository files unchanged');
}
main();
