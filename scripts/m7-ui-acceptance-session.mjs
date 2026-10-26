// Synthetic fixture/server lifecycle only. UI actions are performed separately.
// Never evaluates, installs, previews, or executes generated source.
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawn } from 'node:child_process';

const target = 'E:/Anansi/build-cache/preview-m7/debug';
const fixture = fs.mkdtempSync('E:/Anansi/tmp/m7-ui-acceptance-');
const storage = path.join(fixture, 'storage');
const completion = path.join(fixture, 'finish.json');
const spec = {
  title: 'Cafe Stock Prototype', summary: 'A synthetic local stock-log prototype.',
  purpose: 'Record cafe bean stock.', intendedUsers: 'Synthetic cafe staff',
  features: ['Add a bean', 'Record stock usage', 'Show a low-stock warning'],
  pages: ['index.html'], nonGoals: ['No accounts', 'No server'],
  dataAndState: 'Browser-local state; never store secrets.',
  acceptanceCriteria: ['Add a bean', 'Record usage'], openQuestions: ['Which units are useful?'],
};
const pkg = {
  plan: { summary: 'Dependency-free static prototype.', steps: ['Create the page', 'Add local behavior'],
    buildNotes: 'No build was performed.', testingNotes: 'Generated source has not been executed.' },
  files: [
    { path: 'index.html', content: '<!doctype html><title>Cafe Stock Prototype</title><h1>INERT SOURCE ONLY</h1><script src="app.js"></script>' },
    { path: 'app.js', content: "console.log('FORGE_INERT_SOURCE_ONLY');" },
    { path: 'README.md', content: '# Cafe Stock Prototype\nSynthetic acceptance source. Not executed, tested, deployed, or licensed by ANANSI. Do not store secrets.' },
  ],
};
const requests = [];
const fake = http.createServer((req, res) => {
  if (req.url !== '/api/chat') { res.writeHead(404); res.end(); return; }
  let body = '';
  req.on('data', chunk => { body += chunk; if (body.length > 100000) req.destroy(); });
  req.on('end', () => {
    const stage = body.includes('product engineer') ? 'source' : 'specification';
    requests.push({ stage, body });
    fs.writeFileSync(path.join(fixture, 'provider-requests.json'), JSON.stringify(requests, null, 2));
    const text = JSON.stringify(stage === 'source' ? pkg : spec);
    res.writeHead(200, { 'content-type': 'application/x-ndjson' });
    res.write(JSON.stringify({ model: 'synthetic-forge', message: { role: 'assistant', content: text }, done: false }) + '\n');
    res.end(JSON.stringify({ model: 'synthetic-forge', message: { role: 'assistant', content: '' }, done: true, prompt_eval_count: 9, eval_count: 42 }) + '\n');
  });
});
await new Promise(resolve => fake.listen(0, '127.0.0.1', resolve));
const fakeUrl = `http://127.0.0.1:${fake.address().port}`;
let api;
let desktop;
try {
  api = spawn(path.join(target, 'anansi-api.exe'), ['--port', '0', '--pairing-root', path.join(fixture, 'pairing')], {
    env: { ...process.env, ANANSI_STORAGE_ROOT: storage }, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'],
  });
  const pairing = await new Promise((resolve, reject) => {
    let output = '';
    const timer = setTimeout(() => reject(Error('Owned API startup timed out.')), 30000);
    api.once('error', reject);
    api.stdout.on('data', chunk => {
      output += chunk;
      const match = output.match(/Pairing file: (.+)\r?\n/);
      if (match) { clearTimeout(timer); resolve(JSON.parse(fs.readFileSync(match[1].trim(), 'utf8'))); }
    });
  });
  async function call(group, operation, input) {
    const response = await fetch(`${pairing.endpoint}/v1/${group}/${operation}`, {
      method: 'POST', headers: { Origin: 'http://127.0.0.1:1420', Authorization: `Bearer ${pairing.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ contractVersion: 1, ...input }),
    });
    if (!response.ok) throw Error(`Fixture operation refused: ${group}/${operation}: ${response.status}`);
    return response.json();
  }
  const created = await call('workspaces', 'create', { name: 'M7 UI Acceptance' });
  const workspaceId = created.workspace.id;
  await call('workspaces', 'settings', { id: workspaceId, reducedMotion: true, onboardingComplete: true });
  const project = await call('memories', 'create', { workspaceId, kind: 'project', draft: { title: 'Cafe Prototype', body: 'Synthetic acceptance project.', privacy: 'private', sourceReferences: [] } });
  const projectId = project.memory.id;
  const concept = { name: 'Cafe Stock Prototype', problem: 'Stock records are inconsistent.', targetCustomer: 'Synthetic cafe staff',
    solution: 'A local stock-log page.', whyUseful: 'Keeps counts visible.', differentiationHypothesis: 'Hypothesis: shared-counter focus.',
    revenueModel: 'Unvalidated hypothesis.', engineeringComplexity: 'low', implementationCost: 'Unvalidated effort estimate.',
    keyRisks: ['Needs user validation'], requiredValidation: 'Observe a real stock workflow.', minimumViableVersion: 'One local page.' };
  await call('artifacts', 'register', { workspaceId, projectId, type: 'idea', title: 'Idea set: Acceptance cafe',
    body: JSON.stringify({ topic: 'Synthetic cafe tools', concepts: Array.from({ length: 5 }, (_, i) => ({ ...concept, name: i ? `Alternative ${i}` : concept.name })) }), sourceReferences: [], privacy: 'private' });
  const configDir = path.join(storage, 'provider-config');
  fs.mkdirSync(configDir, { recursive: true });
  fs.writeFileSync(path.join(configDir, `${workspaceId}.json`), JSON.stringify({
    providers: [{ config: { provider: 'ollama', baseUrl: fakeUrl, credential: null, enabled: true }, locality: 'local', models: [{
      id: 'synthetic-forge', provider: 'ollama', capabilities: { chat: true, streaming: true, vision: false, coding: true, deepReasoning: false, embeddings: false },
      contextWindow: 16384, cost: 'free', speed: 'balanced' }] }], budget: { maxRequests: 20, maxTotalTokens: 500000 },
  }));
  await new Promise(resolve => { api.once('exit', resolve); api.kill(); });
  api = undefined;
  desktop = spawn(path.join(target, 'anansi-desktop.exe'), [], {
    env: { ...process.env, ANANSI_STORAGE_ROOT: storage, ANANSI_VAULT_TEST_PROVIDER: '', ANANSI_PROVIDER_TEST_CREDENTIAL: '', WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '', TAURI_CONFIG: '' },
    windowsHide: false, stdio: 'ignore',
  });
  desktop.once('error', error => { console.error(error.message); });
  console.log(JSON.stringify({ fixture, storage, completion, workspaceId, projectId, desktopPid: desktop.pid, provider: fakeUrl }));
  const deadline = Date.now() + 90 * 60 * 1000;
  while (!fs.existsSync(completion)) {
    if (Date.now() > deadline) throw Error('Acceptance session deadline reached.');
    if (desktop.exitCode !== null) throw Error('Acceptance desktop exited before completion.');
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  console.log('Acceptance session completion marker received; fixture retained.');
} finally {
  if (desktop && desktop.exitCode === null) await new Promise(resolve => { desktop.once('exit', resolve); desktop.kill(); });
  if (api && api.exitCode === null) await new Promise(resolve => { api.once('exit', resolve); api.kill(); });
  fake.closeAllConnections();
  await new Promise(resolve => fake.close(resolve));
}
