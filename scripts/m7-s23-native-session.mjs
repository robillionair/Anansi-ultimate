// Owns synthetic fixture/process lifetime. Native UI actions are separate.
// All product HTML/JS below is inert test data and is never executed.
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';

const target = 'E:/Anansi/build-cache/preview-m7-slices2-3/debug';
const fixture = fs.mkdtempSync('E:/Anansi/tmp/m7-s23-native-');
const storage = path.join(fixture, 'storage');
const spec = { title: 'Native Cafe Prototype', summary: 'Synthetic local stock log.',
  purpose: 'Record cafe bean stock.', intendedUsers: 'Synthetic cafe staff',
  features: ['Add a bean', 'Record usage', 'Show a warning'], pages: ['index.html'],
  nonGoals: ['No accounts', 'No server'], dataAndState: 'Local state; never store secrets.',
  acceptanceCriteria: ['Add a bean', 'Record usage'], openQuestions: ['Which units?'] };
const pkg = revised => ({ plan: { summary: 'Dependency-free static prototype.',
  steps: ['Create page', 'Add local behavior'], buildNotes: 'No build performed.',
  testingNotes: 'ANANSI has not executed or tested this source.' }, files: [
  { path: 'index.html', content: '<!doctype html><title>Native Cafe Prototype</title><h1>INERT SOURCE</h1>' },
  { path: 'app.js', content: revised ? "console.log('S23_INERT_V2');" : "console.log('S23_INERT_V1');" },
  { path: 'README.md', content: '# Synthetic prototype\nNot built, tested, executed or deployed. Do not store secrets.' },
  ...(revised ? [{ path: 'revision.txt', content: 'Synthetic revision accepted as inert text.' }] : []),
] });
const draft = { documents: [
  ['architecture.md', 'Model architecture S23: index.html and app.js; unexecuted.'],
  ['usage.md', 'Model usage S23: inspect the source before any manual use.'],
  ['limitations.md', 'Unexecuted synthetic prototype. No build, tests or deployment.'],
  ['security.md', 'No authentication or secrets. This is not a security certification.'],
  ['changes.md', 'The source revision adds revision.txt and changes app.js.'],
].map(([name, content]) => ({ name, content })) };
const requests = [];
const fake = http.createServer((req, res) => {
  if (req.url !== '/api/chat') { res.writeHead(404); res.end(); return; }
  let body = '';
  req.on('data', c => { body += c; if (body.length > 250000) req.destroy(); });
  req.on('end', () => {
    const stage = body.includes('technical writer') ? 'documentation'
      : body.includes('Revision request') ? 'revision'
      : body.includes('product engineer') ? 'source' : 'specification';
    requests.push({ stage, body });
    fs.writeFileSync(path.join(fixture, 'provider-requests.json'), JSON.stringify(requests, null, 2));
    const text = JSON.stringify(stage === 'documentation' ? draft : stage === 'specification' ? spec : pkg(stage === 'revision'));
    res.writeHead(200, { 'content-type': 'application/x-ndjson' });
    res.write(JSON.stringify({ model: 'synthetic-s23', message: { role: 'assistant', content: text }, done: false }) + '\n');
    res.end(JSON.stringify({ model: 'synthetic-s23', message: { role: 'assistant', content: '' }, done: true, prompt_eval_count: 9, eval_count: 42 }) + '\n');
  });
});
await new Promise(resolve => fake.listen(0, '127.0.0.1', resolve));
const fakeUrl = `http://127.0.0.1:${fake.address().port}`;
let api, desktop, pairing, workspaceId, projectId;
const ownedEnv = { ...process.env, ANANSI_STORAGE_ROOT: storage, ANANSI_VAULT_TEST_PROVIDER: '',
  ANANSI_PROVIDER_TEST_CREDENTIAL: '', WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '', TAURI_CONFIG: '' };
async function stop(child) {
  if (child && child.exitCode === null) await new Promise(resolve => { child.once('exit', resolve); child.kill(); });
}
async function startApi() {
  api = spawn(path.join(target, 'anansi-api.exe'), ['--port', '0', '--pairing-root', path.join(fixture, 'pairing')],
    { env: ownedEnv, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  pairing = await new Promise((resolve, reject) => {
    let out = '';
    const timer = setTimeout(() => reject(Error('Owned API startup timed out')), 30000);
    api.once('error', reject);
    api.stdout.on('data', c => { out += c; const match = out.match(/Pairing file: (.+)\r?\n/);
      if (match) { clearTimeout(timer); resolve(JSON.parse(fs.readFileSync(match[1].trim(), 'utf8'))); } });
  });
}
async function call(group, operation, input) {
  const r = await fetch(`${pairing.endpoint}/v1/${group}/${operation}`, { method: 'POST',
    headers: { Origin: 'http://127.0.0.1:1420', Authorization: `Bearer ${pairing.token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ contractVersion: 1, ...input }), signal: AbortSignal.timeout(120000) });
  if (!r.ok) throw Error(`Fixture operation ${group}/${operation} refused: ${r.status}`);
  return r.json();
}
function startDesktop() {
  desktop = spawn(path.join(target, 'anansi-desktop.exe'), [], { env: ownedEnv, windowsHide: false, stdio: 'ignore' });
  desktop.once('error', e => console.error(e.message));
  return desktop.pid;
}
async function fixtureRestore() {
  await startApi();
  const backup = await call('jobs', 'backup', { workspaceId, application: false, plaintextConsent: true });
  let job = backup.job;
  const deadline = Date.now() + 180000;
  while (job.state !== 'Completed') {
    if (Date.now() > deadline || job.state === 'Failed') throw Error('Fixture backup did not complete');
    await new Promise(r => setTimeout(r, 500));
    const listed = await call('jobs', 'list', { workspaceId });
    job = listed.jobs.find(j => j.id === job.id);
  }
  const pieces = [];
  for (let offset = 0; offset < job.artifactBytes;) {
    const part = await call('jobs', 'artifact', { workspaceId, jobId: job.id, artifactId: job.artifactId, offset });
    assert(part.bytes.length > 0); pieces.push(Buffer.from(part.bytes)); offset += part.bytes.length;
  }
  const bytes = Buffer.concat(pieces);
  fs.writeFileSync(path.join(fixture, 'native-workspace-backup.tar'), bytes);
  const { createHash } = await import('node:crypto');
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  const landing = (await call('workspaces', 'create', { name: 'Native S23 Landing' })).workspace.id;
  const upload = await call('jobs', 'upload-begin', { workspaceId: landing, bytes: bytes.length, sha256, plaintextConsent: true });
  let imported;
  for (let offset = 0; offset < bytes.length; offset += 65536) {
    const end = Math.min(offset + 65536, bytes.length);
    imported = await call('jobs', 'upload-part', { workspaceId: landing, id: upload.id,
      offset, bytes: Array.from(bytes.subarray(offset, end)), finish: end === bytes.length });
  }
  const ref = { workspaceId: landing, jobId: imported.job.id, artifactId: imported.job.artifactId };
  const preview = await call('jobs', 'preview', ref);
  const restored = await call('jobs', 'restore', { ...ref, name: 'Native S23 Restored', expectedHash: preview.sha256, confirm: true });
  const restoredWorkspaceId = restored.workspaces[0].id;
  assert.notEqual(restoredWorkspaceId, workspaceId);
  const projects = await call('memories', 'list', { workspaceId: restoredWorkspaceId, kind: 'project' });
  const restoredProjectId = projects.memories[0].id;
  const listed = await call('artifacts', 'list', { workspaceId: restoredWorkspaceId, projectId: restoredProjectId });
  const packageId = listed.artifacts.find(a => a.type === 'source-package').artifactId;
  const deliverableId = listed.artifacts.find(a => a.type === 'deliverable').artifactId;
  const source = await call('artifacts', 'read', { workspaceId: restoredWorkspaceId, artifactId: packageId, version: 2 });
  const sourceDoc = JSON.parse(source.body);
  assert.equal(sourceDoc.parentPackageArtifactId, packageId);
  assert(sourceDoc.files.some(f => f.path === 'app.js' && f.content.includes('S23_INERT_V2')));
  const document = await call('artifacts', 'read', { workspaceId: restoredWorkspaceId, artifactId: deliverableId, version: 2 });
  const documentDoc = JSON.parse(document.body);
  assert.equal(documentDoc.packageArtifactId, packageId);
  assert.equal(documentDoc.packageVersion, 2);
  assert.equal(documentDoc.documents.find(d => d.name === 'architecture.md').content, 'Native S23 architecture v2: inert source only.');
  const exported = await call('artifacts', 'forge-deliverable-export', { workspaceId: restoredWorkspaceId, artifactId: deliverableId, version: 2 });
  fs.writeFileSync(path.join(fixture, 'restored-deliverable.tar'), Buffer.from(exported.tarBase64, 'base64'));
  const result = { restoredWorkspaceId, restoredProjectId, packageId, deliverableId, backupSha256: sha256,
    restoredExportSha256: exported.sha256, method: 'fixture API backup/restore; native reopen separately observed' };
  fs.writeFileSync(path.join(fixture, 'restore.result.json'), JSON.stringify(result, null, 2));
  await stop(api); api = undefined;
  return result;
}
try {
  await startApi();
  workspaceId = (await call('workspaces', 'create', { name: 'Native S23 Acceptance' })).workspace.id;
  await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
  projectId = (await call('memories', 'create', { workspaceId, kind: 'project', draft: {
    title: 'Native S23 Project', body: 'Synthetic native acceptance project.', privacy: 'private', sourceReferences: [] } })).memory.id;
  const concept = { name: spec.title, problem: 'Inconsistent stock records.', targetCustomer: 'Synthetic staff', solution: 'A local page.',
    whyUseful: 'Makes counts visible.', differentiationHypothesis: 'Unvalidated hypothesis.', revenueModel: 'Unvalidated.',
    engineeringComplexity: 'low', implementationCost: 'Unknown.', keyRisks: ['Requires validation'], requiredValidation: 'Observe stock workflow.', minimumViableVersion: 'One local page.' };
  await call('artifacts', 'register', { workspaceId, projectId, type: 'idea', title: 'Idea set: Native S23', privacy: 'private',
    sourceReferences: [], body: JSON.stringify({ topic: 'Synthetic cafe tools', concepts: Array.from({ length: 5 }, (_, i) => ({ ...concept, name: i ? `Alternative ${i}` : concept.name })) }) });
  fs.mkdirSync(path.join(storage, 'provider-config'), { recursive: true });
  fs.writeFileSync(path.join(storage, 'provider-config', `${workspaceId}.json`), JSON.stringify({
    providers: [{ config: { provider: 'ollama', baseUrl: fakeUrl, credential: null, enabled: true }, locality: 'local', models: [{
      id: 'synthetic-s23', provider: 'ollama', capabilities: { chat: true, streaming: true, vision: false, coding: true, deepReasoning: false, embeddings: false },
      contextWindow: 16384, cost: 'free', speed: 'balanced' }] }], budget: { maxRequests: 30, maxTotalTokens: 500000 } }));
  await stop(api); api = undefined;
  const desktopPid = startDesktop();
  const info = { fixture, workspaceId, projectId, desktopPid, provider: fakeUrl,
    completion: path.join(fixture, 'finish.json'), restart: path.join(fixture, 'restart.request'), restore: path.join(fixture, 'restore.request') };
  fs.writeFileSync(path.join(fixture, 'session.json'), JSON.stringify(info, null, 2));
  console.log(JSON.stringify(info));
  const handled = new Set();
  const deadline = Date.now() + 90 * 60 * 1000;
  while (!fs.existsSync(info.completion)) {
    if (Date.now() > deadline) throw Error('Native acceptance session timed out');
    for (const operation of ['restart', 'restore']) {
      if (!handled.has(operation) && fs.existsSync(info[operation])) {
        handled.add(operation); await stop(desktop); desktop = undefined;
        if (operation === 'restore') console.log(JSON.stringify(await fixtureRestore()));
        const pid = startDesktop();
        fs.writeFileSync(path.join(fixture, `${operation}.desktop.json`), JSON.stringify({ pid }));
        console.log(JSON.stringify({ operation, desktopPid: pid }));
      }
    }
    if (desktop.exitCode !== null) throw Error('Owned acceptance desktop exited before completion');
    await new Promise(r => setTimeout(r, 500));
  }
} finally {
  await stop(desktop); await stop(api);
  fake.closeAllConnections(); await new Promise(resolve => fake.close(resolve));
}
