import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const paths = ['README.md', ...['docs', 'plans'].flatMap(dir =>
  fs.readdirSync(path.join(root, dir)).filter(f => f.endsWith('.md')).map(f => `${dir}/${f}`))];
const files = new Map(paths.map(p => [p, fs.readFileSync(path.join(root, p), 'utf8')]));

function validate(snapshot) {
  const errors = [];
  const spec = snapshot.get('docs/MASTER_SPEC.md') ?? '';
  const sections = [...spec.matchAll(/^# (\d+)\. (.+)$/gm)];
  if (sections.length !== 90 || sections.some((h, i) => +h[1] !== i + 1)) errors.push('Master sections must be exactly 1–90');
  const ledger = snapshot.get('docs/SPEC_COVERAGE.md') ?? '';
  const rows = [...ledger.matchAll(/^\| (\d+) \| ([^|]+) \| ([^|]+) \| planned \|$/gm)];
  if (rows.length !== 90 || rows.some((r, i) => +r[1] !== i + 1 || r[2].trim() !== sections[i]?.[2].trim() || !/M\d/.test(r[3]))) errors.push('Coverage must preserve all 90 titles with owners');
  const roadmap = snapshot.get('docs/ROADMAP.md') ?? '';
  const milestones = [...roadmap.matchAll(/^\| M(\d+) — [^|]+ \| [^|]+ \| \[([^\]]+)\]\(\.\.\/plans\/([^\)]+)\) \|$/gm)];
  if (milestones.length !== 16 || milestones.some((m,i) => +m[1] !== i || m[2] !== m[3] || !snapshot.has(`plans/${m[3]}`))) errors.push('Roadmap must assign M0–M15 existing plans');
  const matrix = snapshot.get('docs/REQUIREMENTS_MATRIX.md') ?? '';
  const ids = [...matrix.matchAll(/^\| AN-(\d+) \|/gm)].map(m => +m[1]);
  if (ids.length !== 33 || ids.some((id,i) => id !== i + 1)) errors.push('Requirements AN-001–033 must be unique and retained');
  for (const [name, text] of snapshot) {
    // Only Markdown links outside fenced code are audited. No current local links use fragments.
    const prose = text.replace(/^(```|~~~)[\s\S]*?^\1[^\n]*$/gm, '');
    for (const match of prose.matchAll(/\[[^\]]+\]\(([^)]+)\)/g)) {
      const target = match[1];
      if (/^(https?:|mailto:|#)/.test(target)) continue;
      const resolved = path.posix.normalize(path.posix.join(path.posix.dirname(name), target));
      if (resolved.startsWith('../') || !snapshot.has(resolved)) errors.push(`${name}: missing or out-of-root link ${target}`);
    }
  }
  return errors;
}

const errors = validate(files);
assert.deepEqual(errors, [], errors.join('\n'));
const hash = execFileSync('git', ['hash-object', 'docs/MASTER_SPEC.md'], { cwd: root, encoding: 'utf8' }).trim();
assert.equal(hash, 'b95eae4a3813fb2cd87c00360fcdd9b2489cedec', 'Restored master specification changed');
if (process.argv.includes('--self-test')) {
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
    assert.ok(validate(snapshot).length > 0, 'Invalid planning fixture was accepted');
  }
  console.log('PASS: 7 negative validation fixtures rejected; repository files unchanged');
}
console.log(`PASS: ${files.size} Markdown files, 90 section owners, 16 milestone plans, 33 requirements, local links and unchanged master hash`);
