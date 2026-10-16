import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export function prose(text) {
  let fence;
  return text.split(/\r?\n/).map(line => {
    const match = line.match(/^ {0,3}(`{3,}|~{3,})(.*)$/);
    if (fence) {
      if (match && match[1][0] === fence[0] && match[1].length >= fence.length && !match[2].trim()) fence = undefined;
      return '';
    }
    if (match) { fence = match[1]; return ''; }
    return line.replace(/(`+).*?\1/g, '');
  }).join('\n');
}

function anchors(text) {
  const used = new Map();
  return new Set([...prose(text).matchAll(/^ {0,3}#{1,6}\s+(.+?)\s*#*$/gm)].map(m => {
    const slug = m[1].toLowerCase().replace(/[^\p{L}\p{N}_\-\s]/gu, '').replace(/\s/g, '-');
    const count = used.get(slug) ?? 0;
    used.set(slug, count + 1);
    return count ? `${slug}-${count}` : slug;
  }));
}

export function validate(files) {
  const errors = [];
  const fail = message => errors.push(message);
  const get = name => files.get(name) ?? '';
  const local = (from, href) => {
    if (/^(https?:|mailto:)/i.test(href)) return;
    let decoded;
    try { decoded = decodeURIComponent(href); } catch { fail(`${from}: malformed link ${href}`); return; }
    const [target, fragment] = decoded.split('#');
    if (/^[a-z]+:|^[/\\]|\\/i.test(target)) { fail(`${from}: invalid local path ${href}`); return; }
    const resolved = target ? path.posix.normalize(path.posix.join(path.posix.dirname(from), target)) : from;
    if (resolved.startsWith('../') || !files.has(resolved)) { fail(`${from}: missing or out-of-root link ${href}`); return; }
    if (fragment && (!resolved.endsWith('.md') || !anchors(get(resolved)).has(fragment))) fail(`${from}: missing anchor ${href}`);
    return resolved;
  };
  const spec = get('docs/MASTER_SPEC.md').replace(/\r\n/g, '\n');
  const blob = Buffer.from(spec);
  const hash = createHash('sha1').update(`blob ${blob.length}\0`).update(blob).digest('hex');
  if (hash !== 'b95eae4a3813fb2cd87c00360fcdd9b2489cedec') fail('Master specification content changed');
  const sections = [...prose(spec).matchAll(/^# (\d+)\. (.+)$/gm)];
  if (sections.length !== 90 || sections.some((h,i) => +h[1] !== i+1)) fail('Master sections must be exactly 1–90');
  const rows = [...get('docs/SPEC_COVERAGE.md').matchAll(/^\| (\d+) \| ([^|]+) \| ([^|]+) \| (\w+) \|\r?$/gm)];
  if (rows.length !== 90 || rows.some((r,i) => +r[1] !== i+1 || r[2].trim() !== sections[i]?.[2].trim() || !/M(?:[0-9]|1[0-5])\b/.test(r[3]) || !['planned','verified','blocked','deferred'].includes(r[4]))) fail('Coverage must retain 90 titled sections with milestone owners and valid statuses');
  const milestones = [...get('docs/ROADMAP.md').matchAll(/^\| M(\d+) — [^|]+ \| [^|]+ \| \[([^\]]+)\]\(\.\.\/plans\/([^\)]+)\) \|\r?$/gm)];
  if (milestones.length !== 16 || milestones.some((m,i) => +m[1] !== i || m[2] !== m[3] || !files.has(`plans/${m[3]}`))) fail('Roadmap must assign M0–M15 existing plans');
  const requirements = [...get('docs/REQUIREMENTS_MATRIX.md').matchAll(/^\| AN-(\d+) \|(.+)\r?$/gm)];
  if (requirements.length !== 33 || requirements.some((r,i) => +r[1] !== i+1)) fail('Requirements AN-001–033 must be retained and unique');
  for (const r of requirements) {
    const cells = r[2].split('|').map(s => s.trim()).filter(Boolean);
    const status = +r[1] <= 21 ? cells[5] : cells[2];
    if (!['provisional','planned','in progress','verified','blocked','deferred'].includes(status)) fail(`AN-${r[1]}: invalid requirement status`);
  }
  for (const [name, text] of files) {
    if (!name.endsWith('.md')) continue;
    const clean = prose(text);
    const references = new Map([...clean.matchAll(/^ {0,3}\[([^\]]+)\]:\s*<?([^\s>]+)>?/gm)].map(m => [m[1].toLowerCase(),m[2]]));
    for (const m of clean.matchAll(/!?\[[^\]\n]*\]\(\s*(?:<([^>]+)>|([^\s)]+))(?:\s+"[^"]*")?\s*\)/g)) local(name, m[1] ?? m[2]);
    for (const m of clean.matchAll(/\[([^\]\n]+)\]\[([^\]\n]*)\]/g)) {
      const key = (m[2] || m[1]).toLowerCase();
      if (!references.has(key)) fail(`${name}: undefined reference ${key}`);
      else local(name,references.get(key));
    }
    for (const href of references.values()) local(name,href);
  }
  try {
    const state = JSON.parse(get('project-state.json'));
    for (const key of ['schemaVersion','activeMilestone','activeTask','plan','implementationStatus','lastVerifiedCommit','evidence','blockers']) if (!(key in state)) fail(`State missing ${key}`);
    if (state.schemaVersion !== 1 || !/^M(?:[0-9]|1[0-5])$/.test(state.activeMilestone) || !new RegExp(`^${state.activeMilestone}\\.\\d+$`).test(state.activeTask) || typeof state.implementationStatus !== 'string' || !state.implementationStatus.trim() || !/^[a-f0-9]{40}$/.test(state.lastVerifiedCommit)) fail('State has invalid identity/status/checkpoint');
    if (typeof state.plan !== 'string' || !state.plan.startsWith('plans/')) fail('State plan must be in plans/');
    else local('project-state.json', state.plan);
    if (!Array.isArray(state.evidence) || !state.evidence.length || state.evidence.some(e => typeof e !== 'string' || !e.startsWith('docs/evidence/'))) fail('State evidence must reference docs/evidence files');
    else state.evidence.forEach(e => local('project-state.json',e));
    if (!Array.isArray(state.blockers) || state.blockers.some(b => typeof b.scope !== 'string' || typeof b.detail !== 'string' || !b.detail.trim())) fail('State blockers must have scope and detail');
  } catch { fail('State must be valid JSON'); }
  return errors;
}

export function loadRepository(directory = root) {
  const names = execFileSync('git', ['ls-files','--cached','--others','--exclude-standard','-z'], { cwd: directory, encoding: 'utf8' }).split('\0').filter(Boolean);
  const files = new Map();
  for (const name of names) {
    const absolute = path.join(directory,name);
    if (!fs.existsSync(absolute) || !fs.lstatSync(absolute).isFile()) continue;
    files.set(name, /\.md$|^project-state\.json$/.test(name) ? fs.readFileSync(absolute,'utf8') : '');
  }
  return files;
}

export function main() {
  const files = loadRepository();
  const errors = validate(files);
  if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
  else console.log(`PASS: ${[...files.keys()].filter(p => p.endsWith('.md')).length} Markdown files, 90 section owners, 16 milestone plans, 33 requirements, local links, project state and preserved master`);
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
